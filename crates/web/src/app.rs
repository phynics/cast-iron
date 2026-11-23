//! The application shell: boot, theme, service worker, offline state, and the Reader frame.

use crate::density::{self, Density};
use crate::focus_guard::{FocusTicket, Verdict};
use crate::install;
use crate::js;
use crate::library::{Library, use_library};
use crate::pwa::InstallCtx;
use crate::report;
use crate::route::{self, Route, provide_route, replace, use_route};
use crate::store::{self, ActiveLibrary};
use crate::theme::{self, Theme};
use crate::views::{
    self,
    active_books::ActiveBooks,
    book::{BookView, ChapterView, same_book},
    browse::{Books, BrowseKind, BrowseSource, FeatBrowseKind},
    chains::FeatChains,
    diagnostics::Diagnostics,
    entry::Entry,
    header_action::{HeaderAction, InstallGuide},
    home::Home,
    installer::InstallerScreen,
    legal::Legal,
    not_found::NotFound,
    quick::Quick,
    quick_search::{QuickSearch, QuickSearchButton, QuickSearchLayer},
    recent::{self, Recent},
    search::Search,
    settings::Settings,
    spells::{self, SpellListView, SpellsHub},
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
    /// The reader's spell-row density. Both the spell lists and Settings read it.
    pub density: RwSignal<Density>,
    /// Recently viewed entries. The entry screen records a visit here; Quick search shows them.
    pub recents: RwSignal<Vec<Recent>>,
    /// A newer app shell is installed and waiting for the user to reload.
    pub shell_update_ready: RwSignal<bool>,
    /// The shell version that the controlling service worker reports, once it answers. The
    /// report link reads it; Settings asks the same worker for the same value.
    pub app_version: RwSignal<Option<String>>,
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
    crate::diagnostics::start();
    normalize_location();
    provide_route();
    let online = web_sys::window().is_none_or(|w| w.navigator().on_line());
    let app = AppCtx {
        boot: RwSignal::new(Boot::Starting),
        theme: RwSignal::new(theme::mirrored()),
        density: RwSignal::new(Density::default()),
        recents: RwSignal::new(Vec::new()),
        shell_update_ready: RwSignal::new(false),
        app_version: RwSignal::new(None),
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
        app.density.set(density::load().await);
    });
    spawn_local(async move {
        recent::load_into(app.recents).await;
    });
    spawn_local(async move {
        let result = js::register_service_worker(move || app.shell_update_ready.set(true)).await;
        match result {
            Ok(true) => {}
            Ok(false) => leptos::logging::warn!("service workers are not supported here"),
            Err(error) => leptos::logging::warn!("service worker registration failed: {error}"),
        }
    });
    // The first load has no controlling worker yet, so the report link and Settings wait for
    // the claim. No request leaves the origin: this only asks the worker for its version.
    spawn_local(async move {
        for _ in 0..20 {
            if let Some(info) = js::shell_info().await {
                app.app_version.set(Some(info.version));
                return;
            }
            js::sleep(250).await;
        }
    });
    spawn_local(async move {
        let boot = boot_check().await;
        if matches!(boot, Boot::Ready(_)) {
            crate::diagnostics::mark_ready();
        }
        app.boot.set(boot);
    });

    view! {
        <a class="skip-link" href="#main" on:click=skip_to_main>"Skip to content"</a>
        {move || match app.boot.get() {
            Boot::Starting => view! {
                <div class="app app-loading" role="status">"Loading Cast Iron…"</div>
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

/// What the book-choice recovery screen says, in the words of a player at the table. The
/// engine's own text stays out of it (it sits under the technical-details disclosure), and so
/// does every word the reader would have to look up.
pub const RECOVERY_HEADING: &str = "Your book choices can't be read";
pub const RECOVERY_WHAT: &str = "Cast Iron couldn't read which books you have turned on. Your downloaded library and your stars are safe.";
pub const RECOVERY_NEXT: &str = "Try again. If that doesn't help, reset your book choices: every book turns on, except any third-party books you haven't seen before, which stay off. You can change this in Manage books.";
pub const RECOVERY_RESET_FAILED: &str =
    "Your book choices could not be reset, so nothing was changed. Try again.";

#[component]
fn BookSelectionRecovery(active: ActiveLibrary, message: String) -> impl IntoView {
    let app = use_app();
    let busy = RwSignal::new(false);
    let failure = RwSignal::new(false);
    let retry = active.clone();
    view! {
        <main id="main" class="app install-main recovery-main">
            <h1>{RECOVERY_HEADING}</h1>
            <p class="error" role="alert">{RECOVERY_WHAT}</p>
            <p>{RECOVERY_NEXT}</p>
            <div class="actions">
                <button class="btn btn-primary" disabled=move || busy.get() on:click=move |_| {
                    busy.set(true);
                    app.activate(retry.clone());
                }>"Try again"</button>
                <button class="btn" disabled=move || busy.get() on:click=move |_| {
                    if busy.get_untracked() { return; }
                    busy.set(true);
                    failure.set(false);
                    let active = active.clone();
                    spawn_local(async move {
                        match store::set_disabled_books(&Default::default()).await {
                            Ok(()) => app.activate(active),
                            Err(_) => { failure.set(true); busy.set(false); }
                        }
                    });
                }>"Reset book choices"</button>
            </div>
            {move || failure.get().then(|| view! { <p class="error" role="alert">{RECOVERY_RESET_FAILED}</p> })}
            {views::util::recovery_details(message)}
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
        ) || previous.as_ref().is_some_and(|p| same_book(p, &route))
            // Choosing a school edits the schools in the URL of the spell list that is on screen.
            || previous.as_ref().is_some_and(|p| spells::same_spell_list(p, &route))
            // Typing in the Find a book field of Books or Manage books edits `?q=` of the page.
            || previous.as_ref().is_some_and(|p| views::book_find::same_page(p, &route))
            // So does choosing an archetype filter on the class page that is on screen.
            || previous.as_ref().is_some_and(|p| p.only_arch_differs(&route));
        // A jump to another section of the entry page that is on screen, also from an `?a=`
        // address. The page scrolls, and focus stays on the switcher (or moves to the section's
        // heading), not on <main>.
        let section_jump = previous.as_ref().is_some_and(|p| p.is_section_jump(&route));
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
        // A newer navigation cancels this frame (the route signal bumps the generation when it
        // changes), and a control the reader focused since keeps focus: the frame then scrolls
        // and leaves focus alone (#419).
        let ticket = FocusTicket::take();
        request_animation_frame(move || {
            let verdict = ticket.verdict();
            if within_search || verdict == Verdict::Stale {
                return;
            }
            let moves_focus = verdict == Verdict::Proceed;
            match (route.anchor(), route.section(), restore) {
                (Some(anchor), _, _) => {
                    route::scroll_to_anchor(&anchor);
                }
                // The page is not in the document yet on a first load: it jumps itself when it
                // has rendered (`EntryView`). The jump moves focus to the section's heading,
                // unless the reader has focused a control since.
                (None, Some(section), _) => {
                    route::scroll_to_section_with(section, moves_focus);
                }
                (None, None, Some(y)) => route::scroll_to_offset(y),
                (None, None, None) => route::scroll_to_top(),
            }
            if moves_focus && !first && !to_search && !section_jump {
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
                <a class="brand" href=Route::Home.href()>
                    <span class="brand-mark" aria-hidden="true"></span>
                    <span class="sr-only">"Cast Iron"</span>
                </a>
                <nav aria-label="Primary">
                    <NavLink label="Home" route=Route::Home />
                    <NavLink label="Books" route=Route::books() />
                    <NavLink label="Table" route=Route::Quick />
                    <NavLink label="Settings" route=Route::Settings />
                </nav>
                <QuickSearchButton />
                <HeaderAction />
                <Show when=move || !app.online.get()>
                    <span class="offline-mark">
                        {views::icons::offline()}
                        <span class="offline-text">"Offline"</span>
                        <span class="sr-only">". Everything here still works."</span>
                    </span>
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
                    <ReportProblem />
                </p>
            </footer>
        </div>
    }
}

/// The one footer control. An entry routes to the content issue form; every other page
/// routes to the app form. It is a plain anchor, so GitHub is not contacted until the click.
#[component]
fn ReportProblem() -> impl IntoView {
    let route = use_route();
    let library = use_library();
    let app = use_app();
    let link = Memo::new(move |_| {
        let route = route.get();
        let title = match &route {
            Route::Entry { id, .. } => library.stub(id).map(|stub| stub.title.clone()),
            _ => None,
        };
        let active = library.active();
        report::for_route(
            &route,
            title.as_deref(),
            app.app_version.get().as_deref().unwrap_or_default(),
            &active.content_version,
            active.schema_version,
        )
    });
    view! {
        <a
            class="report-problem"
            href=move || link.get().url
            target="_blank"
            rel="noopener noreferrer"
            aria-label=move || link.get().label
        >
            "Report a problem"
        </a>
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
            Route::Books { .. },
            Route::Books { .. }
                | Route::BooksSet(_)
                | Route::BookRemoved(_)
                | Route::BooksManage { .. }
                | Route::Book { .. }
                | Route::BookChapter { .. },
        ) | (Route::Quick, Route::Quick)
            | (
                Route::Settings,
                Route::Settings | Route::Diagnostics | Route::Legal,
            )
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
            <span class="nav-icon" aria-hidden="true">{views::icons::nav(label)}</span>
            <span>{label}</span>
        </a>
    }
}

/// Titles for screens that do not set their own.
fn static_title(route: &Route) -> Option<&'static str> {
    match route {
        Route::Settings => Some("Settings"),
        Route::Diagnostics => Some("Diagnostics"),
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
        crate::views::util::effect(move || {
            if let Route::BrowseSource(id) = current.get() {
                replace(&Route::Book {
                    source: id,
                    chapter: None,
                });
            }
        });
    }
    let page = Memo::new(move |previous: Option<&(Route, u64)>| {
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
            // A share link is Manage books with a question on top. Clearing the code after the
            // reader answers must not rebuild the page, which would drop the Undo line it just
            // wrote.
            Route::BooksSet(_) | Route::BooksManage { .. } => Route::books_manage(),
            // The find text of Books and Manage books is the page's own state: it reads the text
            // from the route and must not be rebuilt for a keystroke, which would drop the focus
            // in the field. The removed-book line is the same page, so a search clears it without
            // a rebuild.
            Route::Books { .. } | Route::BookRemoved(_) => Route::books(),
            // The schools of a spell list are the list's own state: it reads them from the route
            // and must not be rebuilt for a choice, which would close its open menu.
            Route::SpellList { class, level, .. } => Route::spell_list(class, level),
            // The section of a class page is a place on the page that is already on screen: a
            // jump scrolls it and must not rebuild it, which would drop open disclosures and the
            // focus on the switcher. The page reads its section from the route itself. A stop
            // chosen from an `?a=` address clears the anchor; the page that shows that entry
            // keeps its key, so the clear is not a rebuild either.
            Route::Entry {
                id, anchor, book, ..
            } => {
                let kept = previous.and_then(|(shown, _)| match shown {
                    Route::Entry {
                        id: shown_id,
                        anchor: shown_anchor,
                        book: shown_book,
                        ..
                    } if anchor.is_none() && *shown_id == id && *shown_book == book => {
                        Some(shown_anchor.clone())
                    }
                    _ => None,
                });
                Route::Entry {
                    id,
                    anchor: kept.unwrap_or(anchor),
                    book,
                    section: None,
                    arch: Default::default(),
                }
            }
            other => other,
        };
        // Keep Books mounted so toggles preserve disclosure, focus, and scroll state.
        // Other screens rebuild their discovery snapshot after a saved selection change.
        // Manage books and a book page write the selection themselves, so they stay mounted too:
        // a rebuild after each toggle would drop focus and the scroll position.
        let selection = if matches!(
            route,
            Route::Books { .. }
                | Route::BooksSet(_)
                | Route::BookRemoved(_)
                | Route::BooksManage { .. }
                | Route::Book { .. }
                | Route::BookChapter { .. }
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
        } => match kind {
            // A revision hides or replaces feat rows, so the feat list waits for the graph.
            cast_iron_schema::EntryKind::Feat => {
                view! { <FeatBrowseKind letter=letter facets=facets source=source /> }.into_any()
            }
            _ => view! { <BrowseKind kind letter=letter facets=facets source=source /> }.into_any(),
        },
        Route::BrowseSource(id) => view! { <BrowseSource id /> }.into_any(),
        Route::Books { .. } | Route::BookRemoved(_) => view! { <Books /> }.into_any(),
        Route::BooksManage { .. } | Route::BooksSet(_) => view! { <ActiveBooks /> }.into_any(),
        Route::Book { source, .. } => view! { <BookView source /> }.into_any(),
        Route::BookChapter {
            source,
            chapter,
            anchor,
        } => view! { <ChapterView source chapter anchor=anchor /> }.into_any(),
        Route::Spells => view! { <SpellsHub /> }.into_any(),
        Route::SpellList { class, level, .. } => view! { <SpellListView class level /> }.into_any(),
        Route::FeatChains => view! { <FeatChains /> }.into_any(),
        Route::Entry {
            id, anchor, book, ..
        } => view! { <Entry id anchor book /> }.into_any(),
        Route::Quick => view! { <Quick /> }.into_any(),
        Route::Settings => view! { <Settings /> }.into_any(),
        Route::Diagnostics => view! { <Diagnostics /> }.into_any(),
        Route::Legal => view! { <Legal /> }.into_any(),
        Route::NotFound(_) => view! { <NotFound /> }.into_any(),
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
        // A book-filtered kind list is still Home, not Books.
        assert!(in_section(
            &Route::Home,
            &Route::browse_kind(cast_iron_schema::EntryKind::Spell)
                .with_source(Some("core-rulebook".into()))
        ));
        assert!(!in_section(
            &Route::books(),
            &Route::browse_kind(cast_iron_schema::EntryKind::Spell)
                .with_source(Some("core-rulebook".into()))
        ));
        assert!(in_section(
            &Route::Home,
            &Route::BrowseSource("core-rulebook".into())
        ));
        assert!(in_section(&Route::Settings, &Route::Legal));
        assert!(in_section(&Route::Settings, &Route::Diagnostics));
        for inside in [
            Route::Spells,
            Route::SpellList {
                class: "Wizard".into(),
                level: Some(3),
                schools: Vec::new(),
                name: String::new(),
            },
            Route::FeatChains,
        ] {
            assert!(in_section(&Route::Home, &inside), "{inside:?}");
        }
        assert!(in_section(
            &Route::books(),
            &Route::Book {
                source: "core-rulebook".into(),
                chapter: Some(cast_iron_schema::books::ContentsRow::Position(vec![1]))
            }
        ));
        assert!(in_section(
            &Route::books(),
            &Route::BookChapter {
                source: "core-rulebook".into(),
                chapter: "combat".into(),
                anchor: None
            }
        ));
        assert!(!in_section(&Route::Home, &Route::books()));
        assert!(in_section(&Route::Home, &Route::Home));
        assert!(!in_section(&Route::Home, &Route::Settings));
        assert!(!in_section(&Route::Home, &Route::search("fire")));
        // Table (#437) is one page and highlights only itself.
        assert!(in_section(&Route::Quick, &Route::Quick));
        for other in [
            Route::Home,
            Route::books(),
            Route::Settings,
            Route::Diagnostics,
            Route::Legal,
            Route::search("fire"),
            Route::entry(cast_iron_schema::EntryId::parse("ref/rule/bleed").unwrap()),
        ] {
            assert!(!in_section(&Route::Quick, &other), "{other:?}");
        }
        for link in [Route::Home, Route::books(), Route::Settings] {
            assert!(!in_section(&link, &Route::Quick), "{link:?}");
        }
    }
}
