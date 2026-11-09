//! The four book presets (#157), which preset the active set matches, Undo, and the first-run
//! default. Everything here is pure, so native tests cover it.
//!
//! A preset is a rule over a book's [`Tier`] and its third-party flag, not a stored list of ids:
//! a book that a later library adds joins the preset its tier says. The stored choice stays the
//! `disabled_books` set, and a preset writes it once through `Library::update_active_books`.

use crate::fmt;
use crate::views::book_tier::Tier;
use cast_iron_schema::SourceInfo;
use std::collections::BTreeSet;

/// One book as the presets see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetBook<'a> {
    pub id: &'a str,
    pub tier: Tier,
    /// True for a book that does not come from Paizo. See [`is_third_party`].
    pub third_party: bool,
}

impl<'a> PresetBook<'a> {
    pub fn of(source: &'a SourceInfo) -> Self {
        Self {
            id: &source.id,
            tier: Tier::of(source),
            third_party: is_third_party(source),
        }
    }
}

/// The books of a catalog, in catalog order.
pub fn books_of(sources: &[SourceInfo]) -> Vec<PresetBook<'_>> {
    sources.iter().map(PresetBook::of).collect()
}

/// The third-party marker (#189), read from the corpus `publications.toml` field through
/// [`SourceInfo::third_party`]. The compiler copies the reviewed marker into `catalog.json`, so
/// the presets, Undo and the first-run default all follow the same flag.
pub fn is_third_party(source: &SourceInfo) -> bool {
    source.third_party
}

/// A preset, in the order the page lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    PaizoRulebooks,
    PaizoRulebooksAndMore,
    PaizoEverything,
    EverythingInclThirdParty,
}

impl Preset {
    pub const ALL: [Preset; 4] = [
        Preset::PaizoRulebooks,
        Preset::PaizoRulebooksAndMore,
        Preset::PaizoEverything,
        Preset::EverythingInclThirdParty,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Preset::PaizoRulebooks => "Paizo Rulebooks",
            Preset::PaizoRulebooksAndMore => "Paizo Rulebooks & More",
            Preset::PaizoEverything => "Paizo Everything",
            Preset::EverythingInclThirdParty => "Everything incl. 3rd party",
        }
    }

    /// True when the preset turns this book on. Only the last preset ever includes a third-party
    /// book.
    pub fn includes(self, book: &PresetBook<'_>) -> bool {
        match self {
            Preset::EverythingInclThirdParty => true,
            _ if book.third_party => false,
            Preset::PaizoRulebooks => book.tier == Tier::Rulebooks,
            Preset::PaizoRulebooksAndMore => matches!(
                book.tier,
                Tier::Rulebooks | Tier::PlayerCompanions | Tier::CampaignSetting
            ),
            Preset::PaizoEverything => true,
        }
    }

    /// The ids of the books this preset turns on.
    pub fn members<'a>(self, books: &[PresetBook<'a>]) -> BTreeSet<&'a str> {
        books
            .iter()
            .filter(|book| self.includes(book))
            .map(|book| book.id)
            .collect()
    }
}

/// The presets worth showing for a library, in order. A preset with no books is left out (a
/// library without Rulebooks would otherwise offer "nothing"), and so is a preset whose set
/// equals an earlier one's: two rows that do the same thing would only confuse.
pub fn offered(books: &[PresetBook<'_>]) -> Vec<Preset> {
    let mut seen: Vec<BTreeSet<&str>> = Vec::new();
    let mut presets = Vec::new();
    for preset in Preset::ALL {
        let members = preset.members(books);
        if members.is_empty() || seen.contains(&members) {
            continue;
        }
        seen.push(members);
        presets.push(preset);
    }
    presets
}

/// The offered preset whose books are exactly the active ones, or `None` ("Custom").
pub fn matching(books: &[PresetBook<'_>], is_active: impl Fn(&str) -> bool) -> Option<Preset> {
    let active: BTreeSet<&str> = books
        .iter()
        .map(|book| book.id)
        .filter(|id| is_active(id))
        .collect();
    offered(books)
        .into_iter()
        .find(|preset| preset.members(books) == active)
}

/// How many books a preset turns on.
pub fn size(preset: Preset, books: &[PresetBook<'_>]) -> usize {
    preset.members(books).len()
}

/// The count line of Books and Manage books: `28 of 439 books active`. `preset` is the name or
/// `Custom` on Books and `None` on Manage books, where the checked preset row says it. No active
/// book reads `No books active`, with no preset name.
pub fn count_line(active: usize, total: usize, preset: Option<&str>) -> String {
    if active == 0 {
        return "No books active".to_string();
    }
    let noun = if total == 1 { "book" } else { "books" };
    let mut line = format!(
        "{} of {} {noun} active",
        fmt::count(active),
        fmt::count(total)
    );
    if let Some(preset) = preset {
        line.push_str(" · ");
        line.push_str(preset);
    }
    line
}

/// The name that Books shows beside the count: the matching preset, or `Custom`.
pub fn name_or_custom(preset: Option<Preset>) -> &'static str {
    preset.map_or("Custom", Preset::label)
}

/// The books a preset changes: those it turns on that are off, and those it turns off that are on.
/// Handing it to `Library::update_active_books` writes the preset once. The inverse is Undo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Change {
    pub activate: Vec<String>,
    pub deactivate: Vec<String>,
}

impl Change {
    pub fn is_empty(&self) -> bool {
        self.activate.is_empty() && self.deactivate.is_empty()
    }

    /// The change that puts every touched book back as it was.
    pub fn inverse(&self) -> Change {
        Change {
            activate: self.deactivate.clone(),
            deactivate: self.activate.clone(),
        }
    }
}

/// What applying `preset` changes, given which books are active now.
pub fn plan(preset: Preset, books: &[PresetBook<'_>], is_active: impl Fn(&str) -> bool) -> Change {
    let mut change = Change::default();
    for book in books {
        match (preset.includes(book), is_active(book.id)) {
            (true, false) => change.activate.push(book.id.to_string()),
            (false, true) => change.deactivate.push(book.id.to_string()),
            _ => {}
        }
    }
    change
}

// ---- the first-run default ------------------------------------------------------------------

/// The meta key that remembers whether the first-run default is still to be applied.
pub const DEFAULTS_KEY: &str = "book_defaults";
/// Written once, when a library is activated and the profile never had one: this install is new.
pub const DEFAULTS_PENDING: &str = "seed";
/// Written once the default has been applied, or found not to apply.
pub const DEFAULTS_SETTLED: &str = "done";

/// What `Library::load` does about the first-run default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultsAction {
    /// An existing install, or one already settled. The stored choice is left as it is.
    Nothing,
    /// Mark the default settled and write no choice.
    Settle,
    /// Write this disabled set and mark the default settled.
    Seed(BTreeSet<String>),
}

/// The disabled set of a new install. With Rulebooks, every book outside `Paizo Rulebooks`.
/// Without Rulebooks (the retired PSRD carry-over has none), only the third-party books, so the
/// Paizo books stay active and a marked book never starts on. `None` when that set is empty, so
/// a Paizo-only library with no Rulebooks keeps every book.
pub fn fresh_install_disabled(books: &[PresetBook<'_>]) -> Option<BTreeSet<String>> {
    let rulebooks = Preset::PaizoRulebooks;
    let has_rulebooks = !rulebooks.members(books).is_empty();
    let disabled: BTreeSet<String> = books
        .iter()
        .filter(|book| {
            if has_rulebooks {
                !rulebooks.includes(book)
            } else {
                book.third_party
            }
        })
        .map(|book| book.id.to_string())
        .collect();
    (!disabled.is_empty()).then_some(disabled)
}

/// The third-party books of a catalog. The first-run default and the arrival rule use it.
pub fn third_party_ids(books: &[PresetBook<'_>]) -> BTreeSet<String> {
    books
        .iter()
        .filter(|book| book.third_party)
        .map(|book| book.id.to_string())
        .collect()
}

/// The third-party books that a library update added since `known`, and that therefore start
/// disabled. `known` is the set the profile saw last time. With no remembered set, every
/// third-party book counts as new: the profile has never seen a marked book, and #189 makes
/// third-party books opt-in even on an existing install. The caller stores the current set
/// after this, so the next arrival is caught.
pub fn newly_third_party(
    known: Option<&BTreeSet<String>>,
    books: &[PresetBook<'_>],
) -> BTreeSet<String> {
    let current = third_party_ids(books);
    match known {
        Some(known) => current.difference(known).cloned().collect(),
        None => current,
    }
}

/// The remembered set after a load: every third-party book the profile saw before, plus every
/// one the catalog holds now. A book that a later library drops stays remembered, so the
/// reader's choice survives a downgrade and a later re-add.
pub fn remembered_third_party(
    known: Option<&BTreeSet<String>>,
    books: &[PresetBook<'_>],
) -> BTreeSet<String> {
    let mut seen = known.cloned().unwrap_or_default();
    seen.extend(third_party_ids(books));
    seen
}

/// Decides the first-run default. Only a profile that was new when its first library was
/// activated carries the `DEFAULTS_PENDING` marker (`store::activate_library` writes it). A
/// profile from before this change has no marker, so it keeps its active set whether or not it
/// ever changed it: the owner's rule is that nothing a reader relies on disappears on update. A
/// stored choice also wins over the marker, so the default never overwrites what a reader chose.
pub fn defaults_action(
    marker: Option<&str>,
    stored_choice_exists: bool,
    books: &[PresetBook<'_>],
) -> DefaultsAction {
    if marker != Some(DEFAULTS_PENDING) {
        return DefaultsAction::Nothing;
    }
    if stored_choice_exists {
        return DefaultsAction::Settle;
    }
    match fresh_install_disabled(books) {
        Some(disabled) => DefaultsAction::Seed(disabled),
        None => DefaultsAction::Settle,
    }
}

/// The book-selection outcome of one library load: the disabled set to keep, the remembered
/// third-party set to store, whether this load settles the first-run default, and whether it
/// should queue the one-time third-party notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledBooks {
    pub disabled: BTreeSet<String>,
    pub known_third_party: BTreeSet<String>,
    /// This load applied the first-run default or cleared its marker.
    pub settle_marker: bool,
    /// This load turned off third-party books that an existing profile had never seen.
    pub notice: bool,
}

/// Whether an existing profile should see the one-time third-party notice. True on the first
/// load that turns off third-party books on a profile that had never seen one; a fresh install
/// is seeded silently, and a profile that already remembers a third-party book has seen it.
///
/// An empty remembered set counts as "never seen": the release before the marker shipped (app
/// `2840faf`, library v0.1.13) stored an empty set, because that library marked no book.
pub fn should_show_third_party_notice(
    known: Option<&BTreeSet<String>>,
    arrived: &BTreeSet<String>,
    seeded: bool,
) -> bool {
    known.is_none_or(BTreeSet::is_empty) && !arrived.is_empty() && !seeded
}

/// The pure settle behind `Library::load`: the first-run default, the third-party arrival rule,
/// and the notice decision. The store writes follow from this value, so the sequence is native-
/// tested without IndexedDB.
pub fn settle_books(
    marker: Option<&str>,
    stored: Option<&BTreeSet<String>>,
    known: Option<&BTreeSet<String>>,
    books: &[PresetBook<'_>],
) -> SettledBooks {
    let action = defaults_action(marker, stored.is_some(), books);
    let seeded = matches!(action, DefaultsAction::Seed(_));
    let arrived = newly_third_party(known, books);
    let notice = should_show_third_party_notice(known, &arrived, seeded);
    let mut disabled = match action {
        DefaultsAction::Nothing | DefaultsAction::Settle => stored.cloned().unwrap_or_default(),
        DefaultsAction::Seed(disabled) => disabled,
    };
    disabled.extend(arrived);
    SettledBooks {
        disabled,
        known_third_party: remembered_third_party(known, books),
        settle_marker: marker == Some(DEFAULTS_PENDING),
        notice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn book(id: &str, tier: Tier) -> PresetBook<'_> {
        PresetBook {
            id,
            tier,
            third_party: false,
        }
    }

    fn third_party(id: &str) -> PresetBook<'_> {
        PresetBook {
            id,
            tier: Tier::Other,
            third_party: true,
        }
    }

    /// One book per tier, plus a second Rulebook and a third-party book.
    fn shelf() -> Vec<PresetBook<'static>> {
        vec![
            book("core", Tier::Rulebooks),
            book("apg", Tier::Rulebooks),
            book("companion", Tier::PlayerCompanions),
            book("setting", Tier::CampaignSetting),
            book("path", Tier::Adventures),
            book("misc", Tier::Other),
            third_party("indie"),
        ]
    }

    fn ids(set: BTreeSet<&str>) -> Vec<&str> {
        set.into_iter().collect()
    }

    #[test]
    fn each_preset_has_its_own_book_set() {
        let books = shelf();
        assert_eq!(ids(Preset::PaizoRulebooks.members(&books)), ["apg", "core"]);
        assert_eq!(
            ids(Preset::PaizoRulebooksAndMore.members(&books)),
            ["apg", "companion", "core", "setting"]
        );
        assert_eq!(
            ids(Preset::PaizoEverything.members(&books)),
            ["apg", "companion", "core", "misc", "path", "setting"]
        );
        assert_eq!(
            ids(Preset::EverythingInclThirdParty.members(&books)),
            [
                "apg",
                "companion",
                "core",
                "indie",
                "misc",
                "path",
                "setting"
            ]
        );
    }

    #[test]
    fn only_the_last_preset_turns_third_party_books_on() {
        let books = shelf();
        for preset in Preset::ALL {
            let includes = preset.members(&books).contains("indie");
            assert_eq!(
                includes,
                preset == Preset::EverythingInclThirdParty,
                "{}",
                preset.label()
            );
        }
    }

    #[test]
    fn presets_use_the_owner_names_in_order() {
        let labels: Vec<_> = Preset::ALL.iter().map(|preset| preset.label()).collect();
        assert_eq!(
            labels,
            [
                "Paizo Rulebooks",
                "Paizo Rulebooks & More",
                "Paizo Everything",
                "Everything incl. 3rd party"
            ]
        );
    }

    #[test]
    fn the_third_party_preset_is_hidden_while_no_book_is_marked() {
        // No marker yet (`is_third_party` is false for every source): the fourth preset equals
        // the third and is not offered.
        let books: Vec<_> = shelf().into_iter().filter(|b| !b.third_party).collect();
        assert_eq!(
            offered(&books),
            [
                Preset::PaizoRulebooks,
                Preset::PaizoRulebooksAndMore,
                Preset::PaizoEverything
            ]
        );
        // With a marked book it appears.
        assert_eq!(offered(&shelf()), Preset::ALL);
    }

    #[test]
    fn a_preset_that_repeats_another_or_is_empty_is_not_offered() {
        // Only Rulebooks and Adventures: "& More" repeats "Paizo Rulebooks".
        let books = [
            book("core", Tier::Rulebooks),
            book("path", Tier::Adventures),
        ];
        assert_eq!(
            offered(&books),
            [Preset::PaizoRulebooks, Preset::PaizoEverything]
        );
        // No Rulebooks: "Paizo Rulebooks" is empty and "& More" too; only Everything stays.
        let books = [book("path", Tier::Adventures), book("misc", Tier::Other)];
        assert_eq!(offered(&books), [Preset::PaizoEverything]);
        assert!(offered(&[]).is_empty());
    }

    #[test]
    fn the_matching_preset_is_named_and_anything_else_is_custom() {
        let books = shelf();
        let active_in = |set: &'static [&'static str]| move |id: &str| set.contains(&id);
        assert_eq!(
            matching(&books, active_in(&["core", "apg"])),
            Some(Preset::PaizoRulebooks)
        );
        assert_eq!(
            matching(&books, active_in(&["core", "apg", "companion", "setting"])),
            Some(Preset::PaizoRulebooksAndMore)
        );
        assert_eq!(
            matching(
                &books,
                active_in(&["core", "apg", "companion", "setting", "path", "misc"])
            ),
            Some(Preset::PaizoEverything)
        );
        assert_eq!(
            matching(&books, |_| true),
            Some(Preset::EverythingInclThirdParty)
        );
        assert_eq!(matching(&books, active_in(&["core"])), None);
        assert_eq!(matching(&books, |_| false), None);
        assert_eq!(name_or_custom(None), "Custom");
        assert_eq!(
            name_or_custom(Some(Preset::PaizoEverything)),
            "Paizo Everything"
        );
    }

    #[test]
    fn a_hidden_duplicate_matches_as_the_preset_that_is_shown() {
        // Without a marked book, every book active is Paizo Everything, never the hidden fourth.
        let books: Vec<_> = shelf().into_iter().filter(|b| !b.third_party).collect();
        assert_eq!(matching(&books, |_| true), Some(Preset::PaizoEverything));
    }

    #[test]
    fn the_count_line_names_the_preset_only_when_asked() {
        assert_eq!(
            count_line(28, 439, Some("Paizo Rulebooks")),
            "28 of 439 books active · Paizo Rulebooks"
        );
        assert_eq!(
            count_line(1234, 1500, Some("Custom")),
            "1,234 of 1,500 books active · Custom"
        );
        assert_eq!(count_line(412, 439, None), "412 of 439 books active");
        assert_eq!(count_line(1, 1, None), "1 of 1 book active");
        assert_eq!(count_line(0, 439, Some("Custom")), "No books active");
        assert_eq!(count_line(0, 0, None), "No books active");
    }

    #[test]
    fn a_plan_changes_only_the_books_that_differ() {
        let books = shelf();
        let active = |id: &str| id != "apg" && id != "path";
        let change = plan(Preset::PaizoRulebooks, &books, active);
        assert_eq!(change.activate, ["apg"]);
        assert_eq!(change.deactivate, ["companion", "setting", "misc", "indie"]);
        // A preset that already matches changes nothing.
        let on = |id: &str| Preset::PaizoRulebooks.members(&books).contains(id);
        assert!(plan(Preset::PaizoRulebooks, &books, on).is_empty());
    }

    #[test]
    fn undo_restores_the_previous_set_exactly() {
        let books = shelf();
        // A custom starting set.
        let mut active: BTreeSet<String> = ["core", "path", "indie"].map(String::from).into();
        let before = active.clone();
        let apply = |active: &mut BTreeSet<String>, change: &Change| {
            for id in &change.deactivate {
                active.remove(id);
            }
            for id in &change.activate {
                active.insert(id.clone());
            }
        };
        for preset in Preset::ALL {
            let change = plan(preset, &books, |id| active.contains(id));
            apply(&mut active, &change);
            let want: BTreeSet<String> = preset
                .members(&books)
                .into_iter()
                .map(String::from)
                .collect();
            assert_eq!(active, want, "{}", preset.label());
            apply(&mut active, &change.inverse());
            assert_eq!(active, before, "undo after {}", preset.label());
        }
    }

    #[test]
    fn a_new_install_starts_on_paizo_rulebooks() {
        let books = shelf();
        let disabled = fresh_install_disabled(&books).expect("the shelf has Rulebooks");
        let active: Vec<_> = books
            .iter()
            .map(|b| b.id)
            .filter(|id| !disabled.contains(*id))
            .collect();
        assert_eq!(active, ["core", "apg"]);
        // The default is the Paizo Rulebooks preset, third-party books included in the disabled set.
        assert!(disabled.contains("indie"));
        // A library with no Rulebooks keeps its Paizo books active, but still leaves third-party
        // books off.
        let no_rulebooks = [book("path", Tier::Adventures), third_party("indie")];
        let disabled = fresh_install_disabled(&no_rulebooks).expect("a marked book");
        assert_eq!(
            ids(disabled.iter().map(String::as_str).collect()),
            ["indie"]
        );
        // A Paizo-only library with no Rulebooks keeps every book active.
        assert_eq!(
            fresh_install_disabled(&[book("path", Tier::Adventures)]),
            None
        );
    }

    #[test]
    fn size_counts_the_marked_book_only_in_the_last_preset() {
        let books = shelf();
        assert_eq!(size(Preset::PaizoRulebooks, &books), 2);
        assert_eq!(size(Preset::PaizoRulebooksAndMore, &books), 4);
        assert_eq!(size(Preset::PaizoEverything, &books), 6);
        assert_eq!(size(Preset::EverythingInclThirdParty, &books), 7);
    }

    #[test]
    fn only_a_new_install_gets_the_default() {
        let books = shelf();
        let seeded = fresh_install_disabled(&books).unwrap();
        // New install: marker set, nothing stored.
        assert_eq!(
            defaults_action(Some(DEFAULTS_PENDING), false, &books),
            DefaultsAction::Seed(seeded)
        );
        // Existing install that never touched its books: no marker, nothing stored. Untouched.
        assert_eq!(
            defaults_action(None, false, &books),
            DefaultsAction::Nothing
        );
        // Existing install with a stored choice: untouched.
        assert_eq!(defaults_action(None, true, &books), DefaultsAction::Nothing);
        // Already settled.
        assert_eq!(
            defaults_action(Some(DEFAULTS_SETTLED), false, &books),
            DefaultsAction::Nothing
        );
        // Marker and a stored choice together: the choice wins, the marker is cleared.
        assert_eq!(
            defaults_action(Some(DEFAULTS_PENDING), true, &books),
            DefaultsAction::Settle
        );
        // A new install of a library with no Rulebooks: settle without writing.
        assert_eq!(
            defaults_action(
                Some(DEFAULTS_PENDING),
                false,
                &[book("path", Tier::Adventures)]
            ),
            DefaultsAction::Settle
        );
        // With a marked book, even a Rulebooks-less library seeds the third-party book off.
        assert_eq!(
            defaults_action(
                Some(DEFAULTS_PENDING),
                false,
                &[book("path", Tier::Adventures), third_party("indie")]
            ),
            DefaultsAction::Seed(["indie".to_string()].into_iter().collect())
        );
    }

    #[test]
    fn settling_seeds_settles_and_leaves_an_existing_choice_alone() {
        let books = shelf();
        let marked: BTreeSet<String> = ["indie".to_string()].into_iter().collect();

        // Seed: a new install, no stored choice, no remembered marked book. Every marked book
        // is off, and the notice stays quiet.
        let seed = settle_books(Some(DEFAULTS_PENDING), None, None, &books);
        assert!(seed.settle_marker);
        assert!(seed.disabled.contains("indie"));
        assert_eq!(seed.known_third_party, marked);
        assert!(!seed.notice, "a fresh install shows no notice");

        // A second load of the seeded profile changes nothing and shows nothing.
        let second = settle_books(
            Some(DEFAULTS_SETTLED),
            Some(&seed.disabled),
            Some(&seed.known_third_party),
            &books,
        );
        assert!(!second.settle_marker);
        assert_eq!(second.disabled, seed.disabled);
        assert!(!second.notice);

        // Settle: an existing profile with a stored choice and a pending marker. The marked
        // book arrives, is turned off, and queues the one-time notice.
        let stored: BTreeSet<String> = ["path".to_string()].into_iter().collect();
        let settle = settle_books(Some(DEFAULTS_PENDING), Some(&stored), None, &books);
        assert!(settle.settle_marker);
        assert!(settle.disabled.contains("indie"));
        assert!(settle.disabled.contains("path"), "the choice is kept");
        assert_eq!(settle.known_third_party, marked);
        assert!(settle.notice);

        // Nothing: an existing profile that never carried the marker. Same arrival, same
        // notice, and no marker write.
        let nothing = settle_books(None, None, None, &books);
        assert!(!nothing.settle_marker);
        assert!(nothing.disabled.contains("indie"));
        assert!(nothing.notice);

        // A second load of that profile remembers the book and does nothing.
        let again = settle_books(
            None,
            Some(&nothing.disabled),
            Some(&nothing.known_third_party),
            &books,
        );
        assert_eq!(again.disabled, nothing.disabled);
        assert_eq!(again.known_third_party, nothing.known_third_party);
        assert!(!again.notice);
    }

    #[test]
    fn a_profile_that_remembered_an_empty_third_party_set_sees_the_notice() {
        // The release before the marker stored an empty remembered set: its library marked no
        // book. The first marked library must still turn the book off and show the notice once.
        let books = shelf();
        let empty = BTreeSet::new();
        let stored: BTreeSet<String> = ["path".to_string()].into_iter().collect();
        let first = settle_books(None, Some(&stored), Some(&empty), &books);
        assert!(first.disabled.contains("indie"));
        assert!(first.disabled.contains("path"), "the choice is kept");
        assert!(first.notice);
        let again = settle_books(
            None,
            Some(&first.disabled),
            Some(&first.known_third_party),
            &books,
        );
        assert!(!again.notice, "the notice shows once");
    }

    #[test]
    fn a_library_update_that_adds_a_third_party_book_keeps_it_off() {
        let first = shelf();
        let known = third_party_ids(&first);
        assert_eq!(ids(known.iter().map(String::as_str).collect()), ["indie"]);
        // The same catalog plus a new third-party book.
        let mut next = shelf();
        next.push(third_party("indie2"));
        let arrived = newly_third_party(Some(&known), &next);
        assert_eq!(
            ids(arrived.iter().map(String::as_str).collect()),
            ["indie2"]
        );
        // The arrival joins the stored disabled set, so the reader keeps every other choice.
        let mut disabled = fresh_install_disabled(&first).expect("Rulebooks");
        // The reader had turned the first third-party book on.
        disabled.remove("indie");
        assert!(!disabled.contains("indie2"));
        disabled.extend(arrived.clone());
        assert!(disabled.contains("indie2"));
        assert!(!disabled.contains("indie"), "an enabled choice stays on");
        // A later load remembers the arrival and adds nothing.
        let current = third_party_ids(&next);
        assert!(newly_third_party(Some(&current), &next).is_empty());
    }

    #[test]
    fn the_first_marked_library_turns_every_third_party_book_off() {
        // No remembered set: the profile has never seen a marked book, so each one is new.
        let arrived = newly_third_party(None, &shelf());
        assert_eq!(ids(arrived.iter().map(String::as_str).collect()), ["indie"]);
        // A profile whose library had no third-party book only records the empty set.
        let paizo: Vec<_> = shelf().into_iter().filter(|b| !b.third_party).collect();
        assert!(newly_third_party(None, &paizo).is_empty());
    }

    #[test]
    fn the_remembered_set_only_grows() {
        let shelf = shelf();
        let known = third_party_ids(&shelf);
        assert_eq!(
            ids(remembered_third_party(None, &shelf)
                .iter()
                .map(String::as_str)
                .collect()),
            ["indie"]
        );
        // A library without the book keeps it remembered, so a later re-add is not "new".
        let paizo: Vec<_> = shelf.iter().copied().filter(|b| !b.third_party).collect();
        let remembered = remembered_third_party(Some(&known), &paizo);
        assert_eq!(
            ids(remembered.iter().map(String::as_str).collect()),
            ["indie"]
        );
        assert!(newly_third_party(Some(&remembered), &shelf).is_empty());
    }

    #[test]
    fn books_of_reads_the_tier_from_the_category() {
        let sources = vec![
            SourceInfo {
                id: "core".into(),
                title: "Core".into(),
                entry_count: 1,
                copyright: None,
                category: Some("RPG".into()),
                publication_date: None,
                series_kind: None,
                series: None,
                series_number: None,
                season: None,
                third_party: false,
            },
            SourceInfo {
                id: "indie".into(),
                title: "Indie".into(),
                entry_count: 1,
                copyright: None,
                category: Some("RPG".into()),
                publication_date: None,
                series_kind: None,
                series: None,
                series_number: None,
                season: None,
                third_party: true,
            },
        ];
        let books = books_of(&sources);
        assert_eq!(books[0].tier, Tier::Rulebooks);
        assert!(!books[0].third_party);
        // The marker wins over the category: a third-party book is Third party, not Rulebooks.
        assert_eq!(books[1].tier, Tier::ThirdParty);
        assert!(books[1].third_party);
    }
}
