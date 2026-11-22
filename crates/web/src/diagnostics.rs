//! Local measurements for a device session (#423): how long the install, the start, the first
//! search and a large page take, and how much storage and memory the app uses.
//!
//! Nothing here leaves the device (rule 1). The app records a few numbers in memory and keeps the
//! last launches in IndexedDB (`meta` key `diagnostics`). Settings, Diagnostics shows them and
//! copies them as plain text for the owner to paste into a report.
//!
//! The model, the summaries and the text are pure and have native tests. The recorder at the end
//! (marks, the persisted store, the page and memory probes) calls the browser.

use crate::fmt;
use serde::{Deserialize, Serialize};

/// Launches kept in storage, oldest dropped first.
pub const MAX_LAUNCHES: usize = 8;
/// Searches kept per launch. The first search always stays.
pub const MAX_SEARCHES: usize = 60;
/// Pages kept per launch.
pub const MAX_PAGES: usize = 12;
/// Search rows that the text lists, newest last.
const LISTED_SEARCHES: usize = 8;

/// One library install on this device, from the first byte to the activation.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallRecord {
    pub content_version: String,
    /// `Date.now()` when the install started.
    pub started_at: f64,
    pub total_ms: f64,
    /// Looking at what the cache already holds (a resumed install finds files here).
    pub scan_ms: f64,
    pub download_ms: f64,
    /// The re-read and re-hash of every file.
    pub verify_ms: f64,
    pub activate_ms: f64,
    pub files: u32,
    /// Files that the scan found good, so the download skipped them.
    pub resumed_files: u32,
    /// The size of the whole library.
    pub bytes: u64,
    /// The bytes that the download had to fetch: the library minus the files already there.
    pub downloaded_bytes: u64,
    /// What `navigator.storage.persist()` answered when the install asked.
    pub persist_granted: bool,
    pub standalone: bool,
}

/// Reading and decoding `search.bin`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexLoad {
    pub read_ms: f64,
    pub decode_ms: f64,
    pub bytes: u64,
}

/// One search, from the moment its screen started the run to the results painted: two animation
/// frames after they were shown. The typing pause before the run
/// ([`crate::views::search::DEBOUNCE_MS`], and the Quick search pause) is not in these numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSample {
    /// The length of the query. The text itself is never kept.
    pub query_chars: u32,
    pub hits: u32,
    /// How long the search waited for the index to load (about zero once it is loaded).
    pub index_wait_ms: f64,
    pub engine_ms: f64,
    /// Until the results were handed to the screen: everything except drawing them.
    pub work_ms: f64,
    /// Until the results were painted.
    pub total_ms: f64,
}

/// Which screen ran a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    /// The full results page.
    Results,
    /// The suggestions of the Quick search layer.
    Quick,
}

/// One page, from the moment its screen opened to the text on screen, and how big it is.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PageSample {
    pub title: String,
    pub load_ms: f64,
    /// Elements in the whole document.
    pub elements: u32,
    /// Characters of text in the whole document.
    pub text_chars: u32,
}

/// What one run of the app (one page load) recorded. Every `_ms` field that is not a duration
/// counts from the start of the page load.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Launch {
    /// Counts up across launches. Set when the launch is first stored.
    pub number: Option<u32>,
    pub started_at: f64,
    pub standalone: bool,
    pub online: bool,
    /// The app code started running.
    pub wasm_start_ms: f64,
    /// Duration: confirming that every library file is in the cache.
    pub check_installed_ms: Option<f64>,
    /// Duration: reading and validating the catalog.
    pub catalog_ms: Option<f64>,
    /// The library was open and the screen could be drawn.
    pub ready_ms: Option<f64>,
    /// The page opened on Home, so Home's time means "start to usable".
    pub first_screen_home: bool,
    /// Home was on screen and had painted.
    pub home_ms: Option<f64>,
    pub index_load: Option<IndexLoad>,
    /// The search index was loaded.
    pub index_ready_ms: Option<f64>,
    pub searches: Vec<SearchSample>,
    /// Quick search suggestion lookups.
    pub quick_searches: Vec<SearchSample>,
    pub pages: Vec<PageSample>,
    pub wasm_bytes: Option<u64>,
    pub heap_used: Option<u64>,
}

/// Everything that is kept between launches.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stored {
    pub launch_count: u32,
    pub install: Option<InstallRecord>,
    /// Oldest first.
    pub launches: Vec<Launch>,
}

/// Puts `item` last. At the cap, drops the oldest item after the first, so the first stays.
pub fn push_keep_first<T>(list: &mut Vec<T>, item: T, cap: usize) {
    list.push(item);
    if list.len() > cap && cap >= 2 {
        list.remove(1);
    }
}

/// Gives a new launch its number and stores or replaces it. A launch that has a number replaces
/// the stored launch of that number. Only the newest [`MAX_LAUNCHES`] stay.
pub fn store_launch(stored: &mut Stored, launch: &mut Launch) {
    let number = *launch.number.get_or_insert_with(|| {
        stored.launch_count += 1;
        stored.launch_count
    });
    stored.launch_count = stored.launch_count.max(number);
    match stored
        .launches
        .iter_mut()
        .find(|l| l.number == Some(number))
    {
        Some(slot) => *slot = launch.clone(),
        None => stored.launches.push(launch.clone()),
    }
    let extra = stored.launches.len().saturating_sub(MAX_LAUNCHES);
    stored.launches.drain(..extra);
}

/// Nearest-rank percentile of `values`, which it sorts. `None` for no values.
pub fn percentile(values: &mut [f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    cast_iron_sort::sort_by(values, f64::total_cmp);
    let rank = (p * values.len() as f64).ceil() as usize;
    Some(values[rank.clamp(1, values.len()) - 1])
}

/// `4.2 ms`, `38 ms`, `1.4 s`, `2 min 5 s`.
pub fn duration(ms: f64) -> String {
    if !ms.is_finite() || ms < 0.0 {
        return "unknown".to_string();
    }
    if ms < 10.0 {
        format!("{ms:.1} ms")
    } else if ms < 1000.0 {
        format!("{ms:.0} ms")
    } else if ms < 60_000.0 {
        format!("{:.1} s", ms / 1000.0)
    } else {
        let seconds = (ms / 1000.0).round() as u64;
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

/// The searches after the first, summed up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchSummary {
    pub count: usize,
    pub first: SearchSample,
    /// Over the searches after the first. `None` when there are none.
    pub rest: Option<RestSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RestSummary {
    pub count: usize,
    pub median_total: f64,
    pub p95_total: f64,
    pub slowest_total: f64,
    pub median_engine: f64,
}

pub fn summarize_searches(samples: &[SearchSample]) -> Option<SearchSummary> {
    let (first, rest) = samples.split_first()?;
    let rest_summary = (!rest.is_empty()).then(|| {
        let mut totals: Vec<f64> = rest.iter().map(|s| s.total_ms).collect();
        let mut engines: Vec<f64> = rest.iter().map(|s| s.engine_ms).collect();
        RestSummary {
            count: rest.len(),
            median_total: percentile(&mut totals, 0.5).unwrap_or_default(),
            p95_total: percentile(&mut totals, 0.95).unwrap_or_default(),
            slowest_total: totals.last().copied().unwrap_or_default(),
            median_engine: percentile(&mut engines, 0.5).unwrap_or_default(),
        }
    });
    Some(SearchSummary {
        count: samples.len(),
        first: *first,
        rest: rest_summary,
    })
}

// ---- the text -------------------------------------------------------------------------------

/// What the browser says about storage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StorageFacts {
    pub usage: Option<u64>,
    pub quota: Option<u64>,
    pub persisted: Option<bool>,
}

/// Memory the page can see. Safari offers neither heap number.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MemoryFacts {
    /// The size of the WebAssembly linear memory allocated so far: a high-water mark that never
    /// shrinks. Not live objects, and not the memory of the page or the process.
    pub wasm_bytes: Option<u64>,
    pub heap_used: Option<u64>,
    pub heap_total: Option<u64>,
    pub heap_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LibraryFacts {
    pub label: String,
    pub content_version: String,
    pub schema_version: u32,
    pub entries: u64,
    pub files: usize,
    pub bytes: u64,
}

/// Inputs of the report. The browser fills it in; the text is a pure function of it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub made_at: String,
    pub app: String,
    pub library: LibraryFacts,
    pub device: String,
    pub online_now: bool,
    pub storage: StorageFacts,
    pub memory: MemoryFacts,
    pub install: Option<InstallRecord>,
    pub launch: Launch,
    /// Other launches, newest first.
    pub earlier: Vec<Launch>,
    /// The typing pause before every results search, in ms. It is not in the search numbers.
    pub typing_pause_ms: u32,
    /// The same for a Quick search lookup.
    pub quick_pause_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub title: &'static str,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub sections: Vec<Section>,
}

impl Report {
    /// The plain text that Copy puts on the clipboard.
    pub fn text(&self) -> String {
        let mut out = String::from("Cast Iron diagnostics\n");
        for section in &self.sections {
            out.push('\n');
            out.push_str(section.title);
            out.push('\n');
            for line in &section.lines {
                out.push_str("  ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }
}

fn shown_as(standalone: bool) -> &'static str {
    if standalone {
        "Home Screen app"
    } else {
        "browser tab"
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn bytes_or(value: Option<u64>, missing: &str) -> String {
    value.map_or_else(|| missing.to_string(), fmt::bytes)
}

fn ms_or(value: Option<f64>, missing: &str) -> String {
    value.map_or_else(|| missing.to_string(), duration)
}

fn search_line(sample: &SearchSample, unit: &str) -> String {
    format!(
        "{} characters, {} {unit}: painted after {} (work {}, engine {})",
        sample.query_chars,
        fmt::count(sample.hits),
        duration(sample.total_ms),
        duration(sample.work_ms),
        duration(sample.engine_ms)
    )
}

/// The lines of one search section. `unit` names what a hit is, `pause_ms` is the wait before a run.
fn search_lines(samples: &[SearchSample], unit: &str, pause_ms: u32) -> Vec<String> {
    let Some(summary) = summarize_searches(samples) else {
        return vec!["none yet".to_string()];
    };
    let mut lines = vec![
        format!(
            "{} {}. A run starts {} after the last key. Times end when the {unit} are painted.",
            summary.count,
            if summary.count == 1 {
                "search"
            } else {
                "searches"
            },
            duration(f64::from(pause_ms))
        ),
        format!(
            "first: {} (waited {} for the index)",
            search_line(&summary.first, unit),
            duration(summary.first.index_wait_ms)
        ),
    ];
    if let Some(rest) = summary.rest {
        lines.push(format!(
            "after the first ({}): median {}, 95th percentile {}, slowest {}, engine median {}",
            rest.count,
            duration(rest.median_total),
            duration(rest.p95_total),
            duration(rest.slowest_total),
            duration(rest.median_engine)
        ));
    }
    let from = samples.len().saturating_sub(LISTED_SEARCHES);
    lines.push(format!("last {}:", samples.len() - from));
    lines.extend(
        samples[from..]
            .iter()
            .map(|s| format!("- {}", search_line(s, unit))),
    );
    lines
}

pub fn report(snapshot: &Snapshot) -> Report {
    let launch = &snapshot.launch;
    let mut sections = Vec::new();

    sections.push(Section {
        title: "Device and library",
        lines: vec![
            format!("made: {}", snapshot.made_at),
            format!("app: {}", snapshot.app),
            format!(
                "library: {}, content format {}, {} entries, {} files, {}",
                snapshot.library.label,
                snapshot.library.schema_version,
                fmt::count(snapshot.library.entries),
                fmt::count(snapshot.library.files),
                fmt::bytes(snapshot.library.bytes)
            ),
            format!("library id: {}", snapshot.library.content_version),
            format!("device: {}", snapshot.device),
            format!(
                "shown as: {}; online now: {}",
                shown_as(launch.standalone),
                yes_no(snapshot.online_now)
            ),
        ],
    });

    let storage = &snapshot.storage;
    let memory = &snapshot.memory;
    sections.push(Section {
        title: "Storage and memory",
        lines: vec![
            format!(
                "storage used: {} of {}",
                bytes_or(storage.usage, "not reported"),
                bytes_or(storage.quota, "not reported")
            ),
            format!(
                "persistent storage: {}",
                match storage.persisted {
                    Some(true) => "granted",
                    Some(false) => "not granted",
                    None => "not reported",
                }
            ),
            format!(
                "WebAssembly memory allocated (high-water mark): {}",
                bytes_or(memory.wasm_bytes, "not available")
            ),
            match memory.heap_used {
                Some(used) => format!(
                    "JavaScript heap: {} used of {} (limit {})",
                    fmt::bytes(used),
                    bytes_or(memory.heap_total, "unknown"),
                    bytes_or(memory.heap_limit, "unknown")
                ),
                None => "JavaScript heap: not offered by this browser".to_string(),
            },
        ],
    });

    sections.push(Section {
        title: "Install (the last one on this device)",
        lines: match &snapshot.install {
            None => vec!["none recorded since this feature arrived".to_string()],
            Some(install) => {
                // The rate counts the bytes that were fetched, never the files that were already
                // there, so a resumed install does not read as a fast one.
                let downloaded = if install.downloaded_bytes > 0 && install.download_ms > 0.0 {
                    let per_second =
                        install.downloaded_bytes as f64 / (install.download_ms / 1000.0);
                    format!(
                        "downloaded: {}, about {}/s",
                        fmt::bytes(install.downloaded_bytes),
                        fmt::bytes(per_second as u64)
                    )
                } else {
                    "downloaded: nothing, every file was already there".to_string()
                };
                vec![
                    format!("library id: {}", install.content_version),
                    format!("shown as: {}", shown_as(install.standalone)),
                    format!("total: {}", duration(install.total_ms)),
                    format!(
                        "scan {}, download {}, verify {}, activate {}",
                        duration(install.scan_ms),
                        duration(install.download_ms),
                        duration(install.verify_ms),
                        duration(install.activate_ms)
                    ),
                    format!(
                        "files: {} ({} already there), {}",
                        fmt::count(install.files),
                        fmt::count(install.resumed_files),
                        fmt::bytes(install.bytes)
                    ),
                    downloaded,
                    format!(
                        "persistent storage asked for: {}",
                        if install.persist_granted {
                            "granted"
                        } else {
                            "not granted"
                        }
                    ),
                ]
            }
        },
    });

    let mut lines = vec![
        format!(
            "launch: {}, shown as {}, online at start: {}",
            launch
                .number
                .map_or_else(|| "not stored yet".to_string(), |n| format!("#{n}")),
            shown_as(launch.standalone),
            yes_no(launch.online)
        ),
        format!(
            "app code running: {} after the page started",
            duration(launch.wasm_start_ms)
        ),
        format!(
            "library files checked: {}; catalog read: {}",
            ms_or(launch.check_installed_ms, "not measured"),
            ms_or(launch.catalog_ms, "not measured")
        ),
        format!("library ready: {}", ms_or(launch.ready_ms, "not recorded")),
    ];
    lines.push(if launch.first_screen_home {
        format!(
            "Home on screen, ready to use: {}",
            ms_or(launch.home_ms, "not yet")
        )
    } else {
        "Home time: not recorded, because this launch did not open on Home".to_string()
    });
    lines.push(match launch.index_load {
        Some(load) => format!(
            "search index: read {}, decode {}, {}; ready at {}",
            duration(load.read_ms),
            duration(load.decode_ms),
            fmt::bytes(load.bytes),
            ms_or(launch.index_ready_ms, "not recorded")
        ),
        None => "search index: not loaded in this launch".to_string(),
    });
    sections.push(Section {
        title: "This launch (times count from the start of the page)",
        lines,
    });

    sections.push(Section {
        title: "Full results searches in this launch",
        lines: search_lines(&launch.searches, "results", snapshot.typing_pause_ms),
    });

    sections.push(Section {
        title: "Quick search suggestions in this launch",
        lines: search_lines(
            &launch.quick_searches,
            "suggestions",
            snapshot.quick_pause_ms,
        ),
    });

    sections.push(Section {
        title: "Pages opened in this launch",
        lines: if launch.pages.is_empty() {
            vec![
                "none yet. Only pages opened after this screen was first opened in this launch are measured."
                    .to_string(),
            ]
        } else {
            launch
                .pages
                .iter()
                .map(|page| {
                    format!(
                        "{}: on screen after {}, {} elements, {} characters",
                        page.title,
                        duration(page.load_ms),
                        fmt::count(page.elements),
                        fmt::count(page.text_chars)
                    )
                })
                .collect()
        },
    });

    sections.push(Section {
        title: "Earlier launches (newest first)",
        lines: if snapshot.earlier.is_empty() {
            vec!["none".to_string()]
        } else {
            snapshot
                .earlier
                .iter()
                .map(|earlier| {
                    format!(
                        "#{}: {}, online at start {}, Home {}, {} searches",
                        earlier
                            .number
                            .map_or_else(|| "?".to_string(), |n| n.to_string()),
                        shown_as(earlier.standalone),
                        yes_no(earlier.online),
                        if earlier.first_screen_home {
                            ms_or(earlier.home_ms, "not reached")
                        } else {
                            "not measured".to_string()
                        },
                        earlier.searches.len()
                    )
                })
                .collect()
        },
    });

    Report { sections }
}

// ---- the recorder ---------------------------------------------------------------------------
// Everything below calls the browser and is not run natively.

use crate::js;
use crate::route::{self, Route};
use crate::store;
use futures::lock::Mutex;
use js_sys::Reflect;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

/// The `meta` key of [`Stored`].
pub const KEY: &str = store::KEY_DIAGNOSTICS;

thread_local! {
    static CURRENT: RefCell<Launch> = RefCell::new(Launch::default());
    static PAGE_PROBE: Cell<bool> = const { Cell::new(false) };
    static LAST_INSTALL: RefCell<Option<InstallRecord>> = const { RefCell::new(None) };
    static LOCK: Rc<Mutex<()>> = Rc::new(Mutex::new(()));
}

/// Milliseconds since the page load began (`performance.now()`).
pub fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or_else(js::now_ms, |p| p.now())
}

fn with_launch(change: impl FnOnce(&mut Launch)) {
    CURRENT.with(|current| change(&mut current.borrow_mut()));
}

/// Starts this launch. Call it first, from the root component.
pub fn start() {
    let online = web_sys::window().is_none_or(|w| w.navigator().on_line());
    let first_screen_home = Route::parse(&route::current_hash()) == Route::Home;
    with_launch(|launch| {
        *launch = Launch {
            started_at: js::now_ms(),
            standalone: crate::pwa::detect_standalone(),
            online,
            wasm_start_ms: now(),
            first_screen_home,
            ..Launch::default()
        };
    });
}

pub fn current() -> Launch {
    CURRENT.with(|current| current.borrow().clone())
}

/// Records the first value of a launch's duration. A later library load (after an update) does
/// not replace what the start measured.
pub fn mark_library_check(ms: f64) {
    with_launch(|l| {
        l.check_installed_ms.get_or_insert(ms);
    });
}

pub fn mark_catalog(ms: f64) {
    with_launch(|l| {
        l.catalog_ms.get_or_insert(ms);
    });
}

/// The library opened at boot and the reader can be drawn.
pub fn mark_ready() {
    let at = now();
    with_launch(|l| {
        l.ready_ms.get_or_insert(at);
    });
}

/// Home has painted. Only the first Home of a launch that opened on Home counts.
pub fn mark_home() {
    let at = now();
    let mut first = false;
    with_launch(|l| {
        if l.first_screen_home && l.home_ms.is_none() {
            l.home_ms = Some(at);
            first = true;
        }
    });
    if first {
        spawn_persist();
    }
}

pub fn mark_index_load(load: IndexLoad) {
    with_launch(|l| {
        l.index_load.get_or_insert(load);
    });
}

pub fn mark_index_ready() {
    let at = now();
    let mut first = false;
    with_launch(|l| {
        if l.index_ready_ms.is_none() {
            l.index_ready_ms = Some(at);
            first = true;
        }
    });
    if first {
        spawn_persist();
    }
}

/// Records a search at the moment its results are painted: two animation frames after the screen
/// was handed them. `began` is the `now()` at which the run started, and `current` says whether
/// the run is still the newest one, so a search that was replaced never reports. A run whose
/// frame came while the page was hidden is dropped too.
pub fn record_search_painted(
    kind: SearchKind,
    mut sample: SearchSample,
    began: f64,
    current: impl Fn() -> bool + 'static,
) {
    sample.work_ms = now() - began;
    leptos::prelude::request_animation_frame(move || {
        leptos::prelude::request_animation_frame(move || {
            // A page that was hidden when the frame came would count the wait in its time.
            let visible = web_sys::window()
                .and_then(|w| w.document())
                .is_none_or(|d| d.visibility_state() == web_sys::VisibilityState::Visible);
            if !visible || !current() {
                return;
            }
            sample.total_ms = now() - began;
            with_launch(|l| {
                let list = match kind {
                    SearchKind::Results => &mut l.searches,
                    SearchKind::Quick => &mut l.quick_searches,
                };
                push_keep_first(list, sample, MAX_SEARCHES);
            });
        });
    });
}

pub fn record_install(record: InstallRecord) {
    LAST_INSTALL.with(|last| *last.borrow_mut() = Some(record));
    spawn_persist();
}

/// Turns the page-size probe on for the rest of this launch. The Diagnostics screen calls it when
/// it opens, so an ordinary reader never pays for counting a page's elements and text.
pub fn start_page_probe() {
    PAGE_PROBE.with(|on| on.set(true));
}

/// Measures the open page after it has painted: two animation frames after `opened`. Does
/// nothing until [`start_page_probe`] has run in this launch.
pub fn sample_page(title: String, opened: f64) {
    if !PAGE_PROBE.with(Cell::get) {
        return;
    }
    leptos::prelude::request_animation_frame(move || {
        leptos::prelude::request_animation_frame(move || {
            let load_ms = now() - opened;
            let (elements, text_chars) = page_size();
            with_launch(|l| {
                let page = PageSample {
                    title,
                    load_ms,
                    elements,
                    text_chars,
                };
                l.pages.push(page);
                let extra = l.pages.len().saturating_sub(MAX_PAGES);
                l.pages.drain(..extra);
            });
        });
    });
}

/// Elements and text characters of the whole document.
fn page_size() -> (u32, u32) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return (0, 0);
    };
    let elements = document.get_elements_by_tag_name("*").length();
    let text_chars = document
        .body()
        .and_then(|body| body.text_content())
        .map_or(0, |text| text.chars().count() as u32);
    (elements, text_chars)
}

/// The size of the WebAssembly linear memory allocated so far. It never shrinks, so this is a
/// high-water mark of what the app asked for, not live data and not the process total.
pub fn wasm_memory_bytes() -> Option<u64> {
    let memory: js_sys::WebAssembly::Memory = wasm_bindgen::memory().dyn_into().ok()?;
    let buffer: js_sys::ArrayBuffer = memory.buffer().dyn_into().ok()?;
    Some(u64::from(buffer.byte_length()))
}

/// `performance.memory`, which Chromium offers and Safari does not.
pub fn js_heap() -> MemoryFacts {
    let mut facts = MemoryFacts {
        wasm_bytes: wasm_memory_bytes(),
        ..MemoryFacts::default()
    };
    let memory = web_sys::window()
        .and_then(|w| w.performance())
        .and_then(|p| Reflect::get(&p, &JsValue::from_str("memory")).ok())
        .filter(|m| m.is_object());
    if let Some(memory) = memory {
        let field = |name: &str| {
            Reflect::get(&memory, &JsValue::from_str(name))
                .ok()
                .and_then(|v| v.as_f64())
                .map(|v| v as u64)
        };
        facts.heap_used = field("usedJSHeapSize");
        facts.heap_total = field("totalJSHeapSize");
        facts.heap_limit = field("jsHeapSizeLimit");
    }
    facts
}

pub fn spawn_persist() {
    leptos::task::spawn_local(persist());
}

/// Reads what earlier launches stored.
pub async fn load() -> Stored {
    store::get_meta::<Stored>(KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Stores this launch (and the last install), after any earlier write of this tab has finished.
/// The read, the change and the write are one IndexedDB transaction, so a second tab cannot take
/// the same launch number or overwrite this write. A failure is ignored: measuring must never
/// break the app.
pub async fn persist() {
    let lock = LOCK.with(Rc::clone);
    let _guard = lock.lock().await;
    let mut launch = current();
    let memory = js_heap();
    launch.wasm_bytes = memory.wasm_bytes;
    launch.heap_used = memory.heap_used;
    let install = LAST_INSTALL.with(|last| last.borrow().clone());
    let stored = store::update_meta::<Stored, ()>(KEY, |stored| {
        let mut stored = stored.unwrap_or_default();
        store_launch(&mut stored, &mut launch);
        if install.is_some() {
            stored.install = install;
        }
        (stored, ())
    })
    .await;
    if stored.is_ok()
        && let Some(number) = launch.number
    {
        with_launch(|l| {
            l.number.get_or_insert(number);
        });
    }
}

/// Removes this record and nothing else: the stored launches and install, and what this launch
/// has measured so far. How the launch began (its time, display mode and first screen) stays.
pub async fn clear() {
    let lock = LOCK.with(Rc::clone);
    let _guard = lock.lock().await;
    let _ = store::remove_meta(KEY).await;
    LAST_INSTALL.with(|last| *last.borrow_mut() = None);
    with_launch(|l| {
        *l = Launch {
            started_at: l.started_at,
            standalone: l.standalone,
            online: l.online,
            wasm_start_ms: l.wasm_start_ms,
            first_screen_home: l.first_screen_home,
            ..Launch::default()
        };
    });
}

/// The launches other than the current one, newest first.
pub fn earlier_launches(stored: &Stored, current: &Launch) -> Vec<Launch> {
    stored
        .launches
        .iter()
        .rev()
        .filter(|l| l.number.is_none() || l.number != current.number)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(chars: u32, total: f64) -> SearchSample {
        SearchSample {
            query_chars: chars,
            hits: 10,
            index_wait_ms: 0.0,
            engine_ms: total / 4.0,
            work_ms: total * 0.75,
            total_ms: total,
        }
    }

    #[test]
    fn percentile_is_nearest_rank() {
        let mut values: Vec<f64> = (1..=10).map(f64::from).collect();
        assert_eq!(percentile(&mut values, 0.5), Some(5.0));
        assert_eq!(percentile(&mut values, 0.95), Some(10.0));
        assert_eq!(percentile(&mut values, 0.0), Some(1.0));
        assert_eq!(percentile(&mut [7.0], 0.95), Some(7.0));
        assert_eq!(percentile(&mut [], 0.5), None);
        let mut unsorted = vec![9.0, 1.0, 5.0];
        assert_eq!(percentile(&mut unsorted, 0.5), Some(5.0));
    }

    #[test]
    fn durations_read_in_the_unit_that_fits() {
        assert_eq!(duration(4.24), "4.2 ms");
        assert_eq!(duration(38.4), "38 ms");
        assert_eq!(duration(999.4), "999 ms");
        assert_eq!(duration(1400.0), "1.4 s");
        assert_eq!(duration(59_949.0), "59.9 s");
        assert_eq!(duration(125_000.0), "2 min 5 s");
        assert_eq!(duration(f64::NAN), "unknown");
        assert_eq!(duration(-1.0), "unknown");
    }

    #[test]
    fn the_first_search_is_kept_when_the_list_is_full() {
        let mut list = Vec::new();
        for i in 0..10u32 {
            push_keep_first(&mut list, i, 4);
        }
        assert_eq!(list, vec![0, 7, 8, 9]);
    }

    #[test]
    fn search_summary_separates_the_first_search() {
        assert_eq!(summarize_searches(&[]), None);
        let only = summarize_searches(&[search(3, 200.0)]).unwrap();
        assert_eq!(only.count, 1);
        assert_eq!(only.first.total_ms, 200.0);
        assert!(only.rest.is_none());

        let mut samples = vec![search(3, 200.0)];
        samples.extend((1..=10).map(|i| search(5, f64::from(i * 10))));
        let summary = summarize_searches(&samples).unwrap();
        let rest = summary.rest.unwrap();
        assert_eq!(summary.count, 11);
        assert_eq!(rest.count, 10);
        assert_eq!(rest.median_total, 50.0);
        assert_eq!(rest.p95_total, 100.0);
        assert_eq!(rest.slowest_total, 100.0);
        assert_eq!(rest.median_engine, 12.5);
    }

    #[test]
    fn a_launch_gets_the_next_number_and_replaces_itself() {
        let mut stored = Stored::default();
        let mut first = Launch::default();
        store_launch(&mut stored, &mut first);
        assert_eq!(first.number, Some(1));
        first.home_ms = Some(900.0);
        store_launch(&mut stored, &mut first);
        assert_eq!(stored.launches.len(), 1);
        assert_eq!(stored.launches[0].home_ms, Some(900.0));
        let mut second = Launch::default();
        store_launch(&mut stored, &mut second);
        assert_eq!(second.number, Some(2));
        assert_eq!(stored.launch_count, 2);
        let earlier = earlier_launches(&stored, &second);
        assert_eq!(earlier.len(), 1);
        assert_eq!(earlier[0].number, Some(1));
    }

    #[test]
    fn only_the_newest_launches_stay() {
        let mut stored = Stored::default();
        for _ in 0..(MAX_LAUNCHES + 3) {
            store_launch(&mut stored, &mut Launch::default());
        }
        assert_eq!(stored.launches.len(), MAX_LAUNCHES);
        assert_eq!(stored.launches[0].number, Some(4));
        assert_eq!(
            stored.launches.last().and_then(|l| l.number),
            Some(MAX_LAUNCHES as u32 + 3)
        );
    }

    #[test]
    fn a_store_from_an_older_shape_still_reads() {
        let stored: Stored =
            serde_json::from_str(r#"{"launch_count":2,"launches":[{"number":2,"online":true}]}"#)
                .unwrap();
        assert_eq!(stored.launch_count, 2);
        assert_eq!(stored.launches[0].number, Some(2));
        assert!(stored.launches[0].searches.is_empty());
        let empty: Stored = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Stored::default());
    }

    fn full_snapshot() -> Snapshot {
        let mut samples = vec![SearchSample {
            query_chars: 7,
            hits: 312,
            index_wait_ms: 0.0,
            engine_ms: 8.0,
            work_ms: 130.0,
            total_ms: 160.0,
        }];
        samples.extend((1..=3).map(|i| search(5, f64::from(i * 20))));
        Snapshot {
            made_at: "2026-11-17T10:22:31.000Z".into(),
            app: "64ecde68 (2026-11-16)".into(),
            library: LibraryFacts {
                label: "v0.1.23 (v65d35c59…)".into(),
                content_version: "v65d35c593e7620bd".into(),
                schema_version: 4,
                entries: 31_800,
                files: 31_113,
                bytes: 217_000_000,
            },
            device: "Mozilla/5.0 (iPhone)".into(),
            online_now: false,
            storage: StorageFacts {
                usage: Some(209_000_000),
                quota: Some(11_000_000_000),
                persisted: Some(false),
            },
            memory: MemoryFacts {
                wasm_bytes: Some(100_000_000),
                ..MemoryFacts::default()
            },
            install: Some(InstallRecord {
                content_version: "v65d35c593e7620bd".into(),
                total_ms: 192_000.0,
                scan_ms: 400.0,
                download_ms: 160_000.0,
                verify_ms: 25_000.0,
                activate_ms: 40.0,
                files: 31_113,
                resumed_files: 0,
                bytes: 217_000_000,
                downloaded_bytes: 217_000_000,
                persist_granted: true,
                standalone: true,
                ..InstallRecord::default()
            }),
            launch: Launch {
                number: Some(3),
                standalone: true,
                online: false,
                wasm_start_ms: 412.0,
                check_installed_ms: Some(1200.0),
                catalog_ms: Some(310.0),
                ready_ms: Some(2100.0),
                first_screen_home: true,
                home_ms: Some(2200.0),
                index_load: Some(IndexLoad {
                    read_ms: 180.0,
                    decode_ms: 520.0,
                    bytes: 40_000_000,
                }),
                index_ready_ms: Some(3000.0),
                searches: samples,
                quick_searches: vec![search(4, 30.0), search(6, 50.0)],
                pages: vec![PageSample {
                    title: "Wizard".into(),
                    load_ms: 420.0,
                    elements: 18_340,
                    text_chars: 212_000,
                }],
                ..Launch::default()
            },
            earlier: vec![Launch {
                number: Some(2),
                standalone: true,
                online: true,
                first_screen_home: true,
                home_ms: Some(2500.0),
                ..Launch::default()
            }],
            typing_pause_ms: 80,
            quick_pause_ms: 100,
        }
    }

    #[test]
    fn the_text_holds_every_measurement() {
        let text = report(&full_snapshot()).text();
        assert!(text.starts_with("Cast Iron diagnostics\n"));
        for expected in [
            "made: 2026-11-17T10:22:31.000Z",
            "library: v0.1.23 (v65d35c59…), content format 4, 31,800 entries, 31,113 files, 207 MB",
            "shown as: Home Screen app; online now: no",
            "storage used: 199 MB of 10.2 GB",
            "persistent storage: not granted",
            "WebAssembly memory allocated (high-water mark): 95.4 MB",
            "JavaScript heap: not offered by this browser",
            "total: 3 min 12 s",
            "scan 400 ms, download 2 min 40 s, verify 25.0 s, activate 40 ms",
            "files: 31,113 (0 already there), 207 MB",
            "downloaded: 207 MB, about 1.3 MB/s",
            "persistent storage asked for: granted",
            "launch: #3, shown as Home Screen app, online at start: no",
            "app code running: 412 ms after the page started",
            "library files checked: 1.2 s; catalog read: 310 ms",
            "library ready: 2.1 s",
            "Home on screen, ready to use: 2.2 s",
            "search index: read 180 ms, decode 520 ms, 38.1 MB; ready at 3.0 s",
            "4 searches. A run starts 80 ms after the last key. Times end when the results are painted.",
            "first: 7 characters, 312 results: painted after 160 ms (work 130 ms, engine 8.0 ms) (waited 0.0 ms for the index)",
            "2 searches. A run starts 100 ms after the last key. Times end when the suggestions are painted.",
            "first: 4 characters, 10 suggestions: painted after 30 ms (work 22 ms, engine 7.5 ms)",
            "after the first (3): median 40 ms, 95th percentile 60 ms, slowest 60 ms, engine median 10 ms",
            "Wizard: on screen after 420 ms, 18,340 elements, 212,000 characters",
            "#2: Home Screen app, online at start yes, Home 2.5 s, 0 searches",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    #[test]
    fn the_text_says_what_is_missing_instead_of_inventing_it() {
        let snapshot = Snapshot {
            launch: Launch {
                first_screen_home: false,
                ..Launch::default()
            },
            ..Snapshot::default()
        };
        let text = report(&snapshot).text();
        for expected in [
            "none recorded since this feature arrived",
            "launch: not stored yet",
            "Home time: not recorded, because this launch did not open on Home",
            "search index: not loaded in this launch",
            "storage used: not reported of not reported",
            "persistent storage: not reported",
            "JavaScript heap: not offered by this browser",
            "none yet",
            "none",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in\n{text}");
        }
    }

    #[test]
    fn the_download_rate_counts_only_what_was_downloaded() {
        let mut snapshot = full_snapshot();
        let install = snapshot.install.as_mut().unwrap();
        // Half of the library was already there: the rate is over the other half.
        install.resumed_files = 15_000;
        install.downloaded_bytes = 104_857_600;
        install.download_ms = 100_000.0;
        let text = report(&snapshot).text();
        assert!(
            text.contains("downloaded: 100 MB, about 1.0 MB/s"),
            "{text}"
        );
        assert!(!text.contains("1.3 MB/s"));
        // Everything was already there: no rate at all.
        let install = snapshot.install.as_mut().unwrap();
        install.resumed_files = install.files;
        install.downloaded_bytes = 0;
        install.download_ms = 40.0;
        let text = report(&snapshot).text();
        assert!(text.contains("downloaded: nothing, every file was already there"));
        assert!(!text.contains("/s\n"));
    }

    #[test]
    fn one_search_is_not_plural() {
        let lines = search_lines(&[search(3, 20.0)], "results", 80);
        assert!(
            lines[0].starts_with("1 search. A run starts 80 ms"),
            "{lines:?}"
        );
    }

    #[test]
    fn a_chromium_heap_is_listed() {
        let mut snapshot = full_snapshot();
        snapshot.memory.heap_used = Some(5_242_880);
        snapshot.memory.heap_total = Some(10_485_760);
        snapshot.memory.heap_limit = Some(4_294_967_296);
        let text = report(&snapshot).text();
        assert!(text.contains("JavaScript heap: 5.0 MB used of 10.0 MB (limit 4.0 GB)"));
    }

    #[test]
    fn the_text_never_holds_a_query() {
        // A sample has no field for the query text, so no report can carry it.
        let text = report(&full_snapshot()).text();
        assert!(!text.contains("grapple"));
    }
}
