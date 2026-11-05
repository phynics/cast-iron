//! Installing the app: when the header offers it, how launches are counted, and how the
//! browser's install capability is detected.
//!
//! The decisions are pure functions with native tests. The browser reads stay in small
//! functions at the bottom of the file. The browser's `beforeinstallprompt` event is held by
//! `js/bridge.js`, because it can fire before the app starts.

use crate::js;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// `localStorage` key: how many separate browser sessions have opened the app.
pub const LAUNCHES_KEY: &str = "cast-iron.launches";
/// `sessionStorage` key: this session was already counted. A reload must not count again.
pub const LAUNCH_COUNTED_KEY: &str = "cast-iron.launch-counted";
/// The header offers installing through this launch. The next launch hides it.
pub const HEADER_LAUNCHES: u32 = 5;

/// How this browser can install the app, if at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCapability {
    /// The browser gave us an install prompt that we can open.
    Native,
    /// iOS Safari: no prompt exists, so we explain Share, then Add to Home Screen.
    IosGuide,
    /// Nothing we can do from the page.
    None,
}

/// Everything the header decision depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallFacts {
    /// The app already runs as an installed app.
    pub standalone: bool,
    /// The launch count after this session was counted. `None` when storage is unavailable.
    pub launches: Option<u32>,
    pub capability: InstallCapability,
}

/// Whether the header shows the Install button. The button must never do nothing, so it needs
/// a capability. It is a hint for new users, so it disappears after the first few launches.
pub fn show_header_install(facts: InstallFacts) -> bool {
    !facts.standalone
        && facts.capability != InstallCapability::None
        && facts.launches.is_some_and(|n| n <= HEADER_LAUNCHES)
}

/// Whether Settings offers an Install button. The launch count does not matter there.
pub fn can_offer_install(standalone: bool, capability: InstallCapability) -> bool {
    !standalone && capability != InstallCapability::None
}

/// Safari on an iPhone or iPad, the browsers that install through Share, then Add to Home
/// Screen. Other iOS browsers carry their own marker in the user agent. iPadOS 13 and later
/// reports itself as a Mac, so a Mac with a touch screen counts as an iPad.
pub fn is_ios_safari(user_agent: &str, platform: &str, max_touch_points: u32) -> bool {
    let ios = ["iPhone", "iPad", "iPod"]
        .iter()
        .any(|d| user_agent.contains(d))
        || (platform == "MacIntel" && max_touch_points > 1);
    let other_browser = ["CriOS", "FxiOS", "EdgiOS", "OPiOS", "OPT/", "GSA/"]
        .iter()
        .any(|m| user_agent.contains(m));
    ios && user_agent.contains("Safari") && !other_browser
}

/// A string store that can fail, which is what `Storage` does in private browsing.
pub trait KeyValue {
    fn get(&self, key: &str) -> Result<Option<String>, StorageUnavailable>;
    fn set(&self, key: &str, value: &str) -> Result<(), StorageUnavailable>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageUnavailable;

/// Counts this session once. Returns the launch count, or `None` when either store fails.
/// `session` remembers that this session was counted, so reloads and route changes do not
/// count again; `local` keeps the total across sessions.
pub fn record_launch(local: &impl KeyValue, session: &impl KeyValue) -> Option<u32> {
    let stored = local
        .get(LAUNCHES_KEY)
        .ok()?
        .and_then(|text| text.parse::<u32>().ok())
        .unwrap_or(0);
    if session.get(LAUNCH_COUNTED_KEY).ok()?.is_some() {
        return Some(stored);
    }
    let next = stored.saturating_add(1);
    local.set(LAUNCHES_KEY, &next.to_string()).ok()?;
    session.set(LAUNCH_COUNTED_KEY, "1").ok()?;
    Some(next)
}

impl KeyValue for web_sys::Storage {
    fn get(&self, key: &str) -> Result<Option<String>, StorageUnavailable> {
        self.get_item(key).map_err(|_| StorageUnavailable)
    }
    fn set(&self, key: &str, value: &str) -> Result<(), StorageUnavailable> {
        self.set_item(key, value).map_err(|_| StorageUnavailable)
    }
}

/// Counts this session in the browser's storage.
fn record_launch_in_browser() -> Option<u32> {
    let window = web_sys::window()?;
    let local = window.local_storage().ok()??;
    let session = window.session_storage().ok()??;
    record_launch(&local, &session)
}

/// True when the app runs as an installed app: the display modes of an installed web app, or
/// iOS Safari's own `navigator.standalone`.
fn detect_standalone() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let by_media = ["standalone", "fullscreen", "minimal-ui"]
        .iter()
        .any(|mode| {
            window
                .match_media(&format!("(display-mode: {mode})"))
                .ok()
                .flatten()
                .is_some_and(|query| query.matches())
        });
    let by_navigator = js_sys::Reflect::get(&window.navigator(), &"standalone".into())
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    by_media || by_navigator
}

fn detect_ios_safari() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    is_ios_safari(
        &navigator.user_agent().unwrap_or_default(),
        &navigator.platform().unwrap_or_default(),
        navigator.max_touch_points().max(0) as u32,
    )
}

/// App-wide install state. Get it with [`use_install`].
#[derive(Clone, Copy)]
pub struct InstallCtx {
    standalone: bool,
    ios_safari: bool,
    launches: Option<u32>,
    /// The browser holds an install prompt that we can open.
    native: RwSignal<bool>,
    /// The iOS guidance dialog is open.
    pub guide_open: RwSignal<bool>,
    /// The control that opened the dialog. Focus goes back to it when the dialog closes.
    opener: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
}

pub fn use_install() -> InstallCtx {
    expect_context::<InstallCtx>()
}

impl InstallCtx {
    /// Reads the browser once, counts this session, and follows the install prompt.
    /// Call it from the root component.
    pub fn start() -> Self {
        let native = RwSignal::new(js::install_available());
        let handle = window_event_listener_untyped(js::INSTALL_CHANGE_EVENT, move |_| {
            native.set(js::install_available());
        });
        on_cleanup(move || handle.remove());
        Self {
            standalone: detect_standalone(),
            ios_safari: detect_ios_safari(),
            launches: record_launch_in_browser(),
            native,
            guide_open: RwSignal::new(false),
            opener: StoredValue::new_local(None),
        }
    }

    pub fn standalone(&self) -> bool {
        self.standalone
    }

    pub fn capability(&self) -> InstallCapability {
        if self.native.get() {
            InstallCapability::Native
        } else if self.ios_safari {
            InstallCapability::IosGuide
        } else {
            InstallCapability::None
        }
    }

    pub fn show_in_header(&self) -> bool {
        show_header_install(InstallFacts {
            standalone: self.standalone,
            launches: self.launches,
            capability: self.capability(),
        })
    }

    pub fn offer_in_settings(&self) -> bool {
        can_offer_install(self.standalone, self.capability())
    }

    /// Opens the browser's install prompt, or the iOS guidance where no prompt exists.
    /// `opener` is the control that was used; the guidance gives focus back to it.
    pub fn request(&self, opener: Option<web_sys::HtmlElement>) {
        match self.capability() {
            InstallCapability::Native => spawn_local(async {
                // The outcome does not matter here: the event is single-use, so the bridge
                // clears it and tells us through the change event.
                let _ = js::prompt_install().await;
            }),
            InstallCapability::IosGuide => {
                self.opener.set_value(opener);
                self.guide_open.set(true);
            }
            InstallCapability::None => {}
        }
    }
}

impl InstallCtx {
    /// The control that opened the guidance, once.
    pub fn take_opener(&self) -> Option<web_sys::HtmlElement> {
        self.opener.try_update_value(Option::take).flatten()
    }
}

/// Focuses `element` again after a dialog closes, when it is still in the page.
pub fn restore_focus(element: Option<web_sys::HtmlElement>) {
    if let Some(element) = element
        && element.is_connected()
    {
        let _ = element.focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Memory {
        values: RefCell<HashMap<String, String>>,
        broken: bool,
    }

    impl Memory {
        fn broken() -> Self {
            Self {
                broken: true,
                ..Self::default()
            }
        }
        fn with(key: &str, value: &str) -> Self {
            let memory = Self::default();
            memory.values.borrow_mut().insert(key.into(), value.into());
            memory
        }
    }

    impl KeyValue for Memory {
        fn get(&self, key: &str) -> Result<Option<String>, StorageUnavailable> {
            if self.broken {
                return Err(StorageUnavailable);
            }
            Ok(self.values.borrow().get(key).cloned())
        }
        fn set(&self, key: &str, value: &str) -> Result<(), StorageUnavailable> {
            if self.broken {
                return Err(StorageUnavailable);
            }
            self.values.borrow_mut().insert(key.into(), value.into());
            Ok(())
        }
    }

    fn facts(
        standalone: bool,
        launches: Option<u32>,
        capability: InstallCapability,
    ) -> InstallFacts {
        InstallFacts {
            standalone,
            launches,
            capability,
        }
    }

    #[test]
    fn header_button_follows_the_show_rule() {
        use InstallCapability::*;
        assert!(show_header_install(facts(false, Some(1), Native)));
        assert!(show_header_install(facts(false, Some(5), Native)));
        assert!(show_header_install(facts(false, Some(5), IosGuide)));
        assert!(!show_header_install(facts(false, Some(6), Native)));
        assert!(!show_header_install(facts(false, Some(60), IosGuide)));
        // An installed app never offers installing, however fresh the count.
        assert!(!show_header_install(facts(true, Some(1), Native)));
        assert!(!show_header_install(facts(true, Some(1), IosGuide)));
        // A button without a way to install would do nothing.
        assert!(!show_header_install(facts(false, Some(1), None)));
        // Without storage the count is unknown, so the header stays quiet.
        assert!(!show_header_install(facts(false, Option::None, Native)));
    }

    #[test]
    fn settings_offer_ignores_the_count() {
        use InstallCapability::*;
        assert!(can_offer_install(false, Native));
        assert!(can_offer_install(false, IosGuide));
        assert!(!can_offer_install(false, None));
        assert!(!can_offer_install(true, Native));
    }

    #[test]
    fn detects_ios_safari_only() {
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1";
        let ipad_as_mac = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";
        let chrome_ios = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/123.0 Mobile/15E148 Safari/604.1";
        let firefox_ios = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/124.0 Mobile/15E148 Safari/605.1.15";
        assert!(is_ios_safari(iphone, "iPhone", 5));
        assert!(is_ios_safari(ipad_as_mac, "MacIntel", 5));
        assert!(!is_ios_safari(ipad_as_mac, "MacIntel", 0), "a real Mac");
        assert!(!is_ios_safari(chrome_ios, "iPhone", 5));
        assert!(!is_ios_safari(firefox_ios, "iPhone", 5));
        assert!(!is_ios_safari(
            "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/120 Mobile Safari/537.36",
            "Linux armv8l",
            5
        ));
    }

    #[test]
    fn a_session_counts_once() {
        let local = Memory::default();
        let session = Memory::default();
        assert_eq!(record_launch(&local, &session), Some(1));
        // A reload or route change in the same session keeps the count.
        assert_eq!(record_launch(&local, &session), Some(1));
        assert_eq!(record_launch(&local, &session), Some(1));
        // A new session counts again, and the stored count survives.
        let next_session = Memory::default();
        assert_eq!(record_launch(&local, &next_session), Some(2));
    }

    #[test]
    fn six_sessions_hide_the_header_button() {
        let local = Memory::default();
        let shown: Vec<bool> = (0..6)
            .map(|_| {
                let launches = record_launch(&local, &Memory::default());
                show_header_install(facts(false, launches, InstallCapability::Native))
            })
            .collect();
        assert_eq!(shown, [true, true, true, true, true, false]);
    }

    #[test]
    fn unreadable_counts_start_again_and_broken_storage_gives_none() {
        assert_eq!(
            record_launch(&Memory::with(LAUNCHES_KEY, "many"), &Memory::default()),
            Some(1)
        );
        assert_eq!(
            record_launch(
                &Memory::with(LAUNCHES_KEY, "u32 overflow"),
                &Memory::default()
            ),
            Some(1)
        );
        assert_eq!(record_launch(&Memory::broken(), &Memory::default()), None);
        assert_eq!(record_launch(&Memory::default(), &Memory::broken()), None);
        assert_eq!(
            record_launch(
                &Memory::with(LAUNCHES_KEY, "4294967295"),
                &Memory::default()
            ),
            Some(u32::MAX)
        );
    }
}
