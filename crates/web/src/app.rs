//! The application shell: boot, theme, service worker, offline state, and the Reader frame.

use crate::install;
use crate::js;
use crate::library::{Library, use_library};
use crate::pwa::InstallCtx;
use crate::route::{self, Route, provide_route, replace, use_route};
use crate::store::{self, ActiveLibrary};
use crate::theme::{self, Theme};
use crate::views::{
    self,
    active_books::ActiveBooks,
    book::{BookView, same_book},
    browse::{Books, BrowseKind, BrowseSource},
    chains::FeatChains,
    entry::Entry,
    header_action::{HeaderAction, InstallGuide},
    home::Home,
    installer::InstallerScreen,
    legal::Legal,
    not_found::NotFound,
    quick_search::{QuickSearch, QuickSearchButton, QuickSearchLayer},
    search::Search,
    settings::Settings,
    spells::{SpellListView, SpellsHub},
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

/// What the app is showing, from boot onward.
#[derive(Clone)]
pub enum Boot {
    Starting,
    /// No usable library. `notice` says why, when a library was expected.
    Install {
        notice: Option<String>,
    },
    Ready(Library),
    BookSelection {
        active: ActiveLibrary,
        message: String,
    },
    /// Something outside the library is broken, such as IndexedDB being unavailable.
    Fatal(String),
}

/// App-wide state. Get it with [`use_app`].
#[derive(Clone, Copy)]
pub struct AppCtx {
    pub boot: RwSignal<Boot>,
    pub theme: RwSignal<Theme>,
    /// A newer app shell is installed and waiting for the user to reload.
    pub shell_update_ready: RwSignal<bool>,
    pub online: RwSignal<bool>,
}

pub fn use_app() -> AppCtx {
    expect_context::<AppCtx>()
}

impl AppCtx {
    /// Opens the library that was just activated and shows the Reader.
    pub fn activate(self, active: ActiveLibrary) {
        spawn_local(async move {
            self.boot.set(match Library::load(&active).await {
                Ok(library) => Boot::Ready(library),
                Err(crate::library::LibraryError::BookSelection(message)) => Boot::BookSelection {
                    active: active.clone(),
                    message,
                },
                Err(error) => Boot::Install {
                    notice: Some(format!(
                        "The library was installed but cannot be opened: {error}"
                    )),
                },
            });
        });
    }
}

/// Decides between installer and Reader. `active_library` is the only source of "ready".
async fn boot_check() -> Boot {
    match store::active_library().await {
        Err(error) => Boot::Fatal(format!(
            "This browser cannot open its local database, so Cast Iron cannot keep your library or bookmarks. {error}"
        )),
        Ok(None) => {
            let interrupted = store::pending_install().await.ok().flatten().is_some();
            Boot::Install {
                notice: interrupted.then(|| {
                    "An earlier download did not finish. Files that were already verified are kept, so it continues where it stopped.".to_string()
                }),
            }
        }
        Ok(Some(active)) => match Library::load(&active).await {
            Ok(library) => Boot::Ready(library),
            Err(crate::library::LibraryError::BookSelection(message)) => Boot::BookSelection {
                active: active.clone(),
                message,
            },
            Err(error) => Boot::Install {
                notice: Some(format!(
                    "The stored library is incomplete ({error}). Download it again to continue."
                )),
            },
        },
    }
}

/// Keeps `/cast-iron/index.html?x#/hash` and `/cast-iron/#/hash` the same document, so that
/// relative `#/...` links never leave the page.
fn normalize_location() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let location = window.location();
    let (Ok(path), Ok(search), Ok(hash), Ok(Some(base))) = (
        location.pathname(),
        location.search(),
        location.hash(),
        document.base_uri(),
    ) else {
        return;
    };
    let base_path = base
        .split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|i| &rest[i..]))
        .unwrap_or("/");
    if (path != base_path || !search.is_empty())
        && let Ok(history) = window.history()
    {
        route::replace_url_keeping_state(&history, &format!("{base_path}{hash}"));
    }
}

#[component]
pub fn App() -> impl IntoView {
    normalize_location();
    provide_route();
    let online = web_sys::window().is_none_or(|w| w.navigator().on_line());
    let app = AppCtx {
        boot: RwSignal::new(Boot::Starting),
        theme: RwSignal::new(theme::mirrored()),
        shell_update_ready: RwSignal::new(false),
        online: RwSignal::new(online),
    };
    provide_context(app);
    provide_context(InstallCtx::start());

    let online_handle = window_event_listener(leptos::ev::online, move |_| app.online.set(true));
    let offline_handle = window_event_listener(leptos::ev::offline, move |_| app.online.set(false));
    on_cleanup(move || {
        online_handle.remove();
        offline_handle.remove();
    });

    spawn_local(async move {
        app.theme.set(theme::load().await);
    });
    spawn_local(async move {
        let result = js::register_service_worker(move || app.shell_update_ready.set(true)).await;
        match result {
            Ok(true) => {}
            Ok(false) => leptos::logging::warn!("service workers are not supported here"),
            Err(error) => leptos::logging::warn!("service worker registration failed: {error}"),
        }
    });
    spawn_local(async move { app.boot.set(boot_check().await) });

    view! {
        <a class="skip-link" href="#main" on:click=skip_to_main>"Skip to content"</a>
        {move || match app.boot.get() {
            Boot::Starting => view! {
                <div class="app app-loading" role="status">"Loading Cast Iron..."</div>
            }.into_any(),
            Boot::Install { notice } => view! { <InstallerScreen notice /> }.into_any(),
            Boot::Ready(library) => view! { <Reader library /> }.into_any(),
            Boot::BookSelection { active, message } => view! { <BookSelectionRecovery active message /> }.into_any(),
            Boot::Fatal(message) => view! {
                <main id="main" class="app install-main">
                    <h1>"Cast Iron cannot start"</h1>
                    <p class="error" role="alert">{message}</p>
                </main>
            }.into_any(),
        }}
    }
}

#[component]
fn BookSelectionRecovery(active: ActiveLibrary, message: String) -> impl IntoView {
    let app = use_app();
    let busy = RwSignal::new(false);
    let failure = RwSignal::new(None::<String>);
    let retry = active.clone();
    view! {
        <main id="main" class="app install-main">
            <h1>"Book choices could not be opened"</h1>
            <p class="error" role="alert">{message}</p>
            <p>"Your downloaded library and stars are kept. You can retry, or reset the choices to make every book active."</p>
            <div class="actions">
                <button class="btn" disabled=move || busy.get() on:click=move |_| {
                    busy.set(true);
                    app.activate(retry.clone());
                }>"Retry"</button>
                <button class="btn" disabled=move || busy.get() on:click=move |_| {
                    if busy.get_untracked() { return; }
                    busy.set(true);
                    let active = active.clone();
                    spawn_local(async move {
                        match store::set_disabled_books(&Default::default()).await {
                            Ok(()) => app.activate(active),
                            Err(error) => { failure.set(Some(error.to_string())); busy.set(false); }
                        }
                    });
                }>"Reset active books"</button>
            </div>
            {move || failure.get().map(|error| view! { <p class="error" role="alert">{error}</p> })}
        </main>
    }
}

fn skip_to_main(event: leptos::ev::MouseEvent) {
    // A plain `#main` link would replace the route hash.
    event.prevent_default();
    focus_main();
}

fn focus_main() {
    if let Some(main) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("main"))
        .and_then(|m| m.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = main.focus_with_options(&options);
    }
}

/// The frame around every screen once a library is ready.
#[component]
fn Reader(library: Library) -> impl IntoView {
    let updates = crate::updates::UpdateCtx::provide(library.active().clone());
    provide_context(library);
    provide_context(updates);
    let app = use_app();
    let current = use_route();
    let quick = QuickSearch::provide();

    // After each route change: scroll to the anchor or the top, and move focus to <main>
    // so that screen readers announce the new page. Search edits its own route while the user
    // types, so a change from one search route to another leaves scroll and focus alone.
    Effect::new(move |previous: Option<Route>| {
        let route = current.get();
        let first = previous.is_none();
        // Toggling a chapter edits the chapter in the URL of the book that is on screen, which
        // changes nothing the reader should be moved for.
        let within_search = matches!(
            (&previous, &route),
            (Some(Route::Search { .. }), Route::Search { .. })
        ) || previous.as_ref().is_some_and(|p| same_book(p, &route));
        // The Search screen focuses its own box, so the frame leaves focus alone there.
        let to_search = matches!(route, Route::Search { .. });
        if let Some(title) = static_title(&route) {
            views::util::set_title(title);
        }
        let next = route.clone();
        // Back and Forward to an entry that was left earlier return to its scroll offset.
        let restore = if within_search {
            None
        } else {
            route::arrive_at_entry()
        };
        request_animation_frame(move || {
            if within_search {
                return;
            }
            match (route.anchor(), restore) {
                (Some(anchor), _) => {
                    route::scroll_to_anchor(&anchor);
                }
                (None, Some(y)) => route::scroll_to_offset(y),
                (None, None) => route::scroll_to_top(),
            }
            if !first && !to_search {
                focus_main();
            }
        });
        next
    });

    views::search::install_slash_shortcut(quick);
    window_event_listener(leptos::ev::scroll, |_| route::remember_scroll());

    view! {
        <div class="app">
            <header class="app-header">
                <a class="brand" href=Route::Home.href()>"Cast Iron"</a>
                <nav aria-label="Primary">
                    <NavLink label="Home" route=Route::Home />
                    <NavLink label="Books" route=Route::Books />
                    <NavLink label="Settings" route=Route::Settings />
                </nav>
                <QuickSearchButton />
                <HeaderAction />
                <Show when=move || !app.online.get()>
                    <span class="badge" title="No connection. Everything here still works.">"Offline"</span>
                </Show>
            </header>
            <InstallGuide />
            <QuickSearchLayer />
            <main id="main" tabindex="-1">
                <RouteView />
            </main>
            <footer class="app-footer">
                <p>
                    "Cast Iron is an unofficial fan project. It is not published, endorsed, or approved by Paizo Inc. "
                    <a href=Route::Legal.href()>"Legal and attribution"</a>
                </p>
                <p class="footer-links">
                    <a href="https://github.com/phynics/cast-iron" rel="noopener noreferrer">"Source on GitHub"</a>
                    " · "
                    <a href="https://github.com/phynics/cast-iron/issues" rel="noopener noreferrer">"Send feedback or report a problem"</a>
                    <span class="muted">" (needs a connection)"</span>
                </p>
            </footer>
        </div>
    }
}

/// True when `link` is the section that `current` belongs to.
fn in_section(link: &Route, current: &Route) -> bool {
    matches!(
        (link, current),
        (
            Route::Home,
            Route::Home
                | Route::BrowseKind { .. }
                | Route::BrowseSource(_)
                | Route::Spells
                | Route::SpellList { .. }
                | Route::FeatChains,
        ) | (
            Route::Books,
            Route::Books | Route::BooksActive | Route::Book { .. },
        ) | (Route::Settings, Route::Settings | Route::Legal)
    )
}

#[component]
fn NavLink(label: &'static str, route: Route) -> impl IntoView {
    let current = use_route();
    let href = route.href();
    view! {
        <a
            class="nav-link"
            href=href
            aria-current=move || in_section(&route, &current.get()).then_some("page")
        >
            <span class="nav-icon" aria-hidden="true">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                    {match label {
                        "Home" => view! { <path d="M3 10.5 12 3l9 7.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 19.5z"/><path d="M9 21v-7h6v7"/> }.into_any(),
                        "Books" => view! { <path d="M12 6c-2-2-5-2.5-9-2v15c4-.5 7 0 9 2m0-15c2-2 5-2.5 9-2v15c-4-.5-7 0-9 2m0-15v15"/> }.into_any(),
                        _ => view! { <circle cx="12" cy="12" r="3"/><path d="M19.4 13.5a7.7 7.7 0 0 0 0-3l2-1.5-2-3.5-2.4 1a8 8 0 0 0-2.6-1.5L14 2h-4l-.4 3a8 8 0 0 0-2.6 1.5l-2.4-1-2 3.5 2 1.5a7.7 7.7 0 0 0 0 3l-2 1.5 2 3.5 2.4-1a8 8 0 0 0 2.6 1.5l.4 3h4l.4-3a8 8 0 0 0 2.6-1.5l2.4 1 2-3.5z"/> }.into_any(),
                    }}
                </svg>
            </span>
            <span>{label}</span>
        </a>
    }
}

/// Titles for screens that do not set their own.
fn static_title(route: &Route) -> Option<&'static str> {
    match route {
        Route::Settings => Some("Settings"),
        Route::Legal => Some("Legal and attribution"),
        Route::NotFound(_) => Some("Page not found"),
        _ => None,
    }
}

#[component]
fn RouteView() -> impl IntoView {
    let current = use_route();
    // The Search screen reads and edits its own route, so edits must not rebuild it.
    // A book edits its own chapter in the URL when a chapter opens, and must not be rebuilt for it.
    let library = use_library();
    let has_books = library.has_books();
    let revision = library.selection_revision();
    if has_books {
        // The address follows the page: `#/browse/source/<id>` becomes the book's own address,
        // so the Books tab is the one highlighted (the old route belongs to Home) and the
        // history holds one entry for the page, not a redirect to step back through.
        Effect::new(move |_| {
            if let Route::BrowseSource(id) = current.get() {
                replace(&Route::Book {
                    source: id,
                    chapter: None,
                });
            }
        });
    }
    let page = Memo::new(move |_| {
        let route = match current.get() {
            Route::Search { .. } => Route::search(""),
            Route::Book { source, .. } => Route::Book {
                source,
                chapter: None,
            },
            // An old link to a publication is its book page when the library has book order.
            Route::BrowseSource(id) if has_books => Route::Book {
                source: id,
                chapter: None,
            },
            other => other,
        };
        // Keep Books mounted so toggles preserve disclosure, focus, and scroll state.
        // Other screens rebuild their discovery snapshot after a saved selection change.
        // Active books and a book page write the selection themselves, so they stay mounted too:
        // a rebuild after each toggle would drop focus and the scroll position.
        let selection = if matches!(
            route,
            Route::Books | Route::BooksActive | Route::Book { .. }
        ) {
            0
        } else {
            revision.get()
        };
        (route, selection)
    });
    move || match page.get().0 {
        Route::Home => view! { <Home /> }.into_any(),
        Route::Search { .. } => view! { <Search /> }.into_any(),
        Route::BrowseKind {
            kind,
            letter,
            facets,
            source,
        } => view! { <BrowseKind kind letter=letter facets=facets source=source /> }.into_any(),
        Route::BrowseSource(id) => view! { <BrowseSource id /> }.into_any(),
        Route::Books => view! { <Books /> }.into_any(),
        Route::BooksActive => view! { <ActiveBooks /> }.into_any(),
        Route::Book { source, .. } => view! { <BookView source /> }.into_any(),
        Route::Spells => view! { <SpellsHub /> }.into_any(),
        Route::SpellList { class, level } => view! { <SpellListView class level /> }.into_any(),
        Route::FeatChains => view! { <FeatChains /> }.into_any(),
        Route::Entry { id, anchor } => view! { <Entry id anchor /> }.into_any(),
        Route::Bookmarks | Route::Collection(_) => view! { <Home /> }.into_any(),
        Route::Settings => view! { <Settings /> }.into_any(),
        Route::Legal => view! { <Legal /> }.into_any(),
        Route::NotFound(hash) => view! { <NotFound hash /> }.into_any(),
    }
}

/// Used by Settings to leave no doubt which version of the check ran.
pub use install::UpdateCheck;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_highlights_the_containing_section() {
        assert!(in_section(
            &Route::Home,
            &Route::browse_kind(cast_iron_schema::EntryKind::Spell)
        ));
        assert!(in_section(
            &Route::Home,
            &Route::BrowseSource("core-rulebook".into())
        ));
        assert!(in_section(&Route::Settings, &Route::Legal));
        for inside in [
            Route::Spells,
            Route::SpellList {
                class: "Wizard".into(),
                level: Some(3),
            },
            Route::FeatChains,
        ] {
            assert!(in_section(&Route::Home, &inside), "{inside:?}");
        }
        assert!(in_section(
            &Route::Books,
            &Route::Book {
                source: "core-rulebook".into(),
                chapter: Some(vec![1])
            }
        ));
        assert!(!in_section(&Route::Home, &Route::Books));
        assert!(in_section(&Route::Home, &Route::Home));
        assert!(!in_section(&Route::Home, &Route::Settings));
        assert!(!in_section(&Route::Home, &Route::search("fire")));
    }
}
