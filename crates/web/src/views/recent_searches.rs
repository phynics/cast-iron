//! Recent searches: the queries that the reader submitted, with the scope each one ran in, kept
//! in the `meta` store under `recent_searches`, next to the recently viewed entries. They stay
//! on the device.
//!
//! A search is its text plus an optional [`Scope`]: `grapple` in `Feats` is not `grapple`
//! everywhere. Tapping a recent search restores both. Lists written before scopes existed hold
//! plain strings; they are read as searches with no scope.

use crate::route::Route;
use crate::store;
use crate::views::scope::Scope;
use serde::{Deserialize, Serialize};

pub const KEY: &str = store::KEY_RECENT_SEARCHES;
pub const MAX_RECENT_SEARCHES: usize = 10;

/// One remembered search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentSearch {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
}

/// What the store may hold: the current object, or the plain string of an older version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum Stored {
    Search(RecentSearch),
    Plain(String),
}

impl RecentSearch {
    pub fn new(text: &str, scope: Option<Scope>) -> Self {
        Self {
            text: normalize(text),
            scope,
        }
    }

    /// The muted label next to the text: `Feats`, `Combat feats`. None for a search over
    /// everything.
    pub fn scope_label(&self) -> Option<String> {
        self.scope.as_ref().map(Scope::label)
    }

    /// The results page for this search, in the scope it ran in.
    pub fn route(&self) -> Route {
        match &self.scope {
            Some(scope) => scope.route(self.text.clone()),
            None => Route::search(self.text.clone()),
        }
    }

    /// What names the search to a screen reader: `grapple in Feats`.
    pub fn name(&self) -> String {
        match self.scope_label() {
            Some(label) => format!("{} in {label}", self.text),
            None => self.text.clone(),
        }
    }
}

/// The text that is stored and compared: trimmed, with each run of whitespace made one space.
pub fn normalize(query: &str) -> String {
    query.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The same scope: the same kind and the same facet values, without regard to case.
fn same_scope(a: &Option<Scope>, b: &Option<Scope>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.kind == b.kind
                && a.facets.len() == b.facets.len()
                && a.facets.iter().all(|(field, value)| {
                    b.facets.iter().any(|(other_field, other_value)| {
                        field.eq_ignore_ascii_case(other_field)
                            && value.eq_ignore_ascii_case(other_value)
                    })
                })
        }
        _ => false,
    }
}

/// Two searches are the same when their text matches without regard to case and they ran in the
/// same scope. `grapple` in Feats and `grapple` everywhere are two entries.
fn same(a: &RecentSearch, b: &RecentSearch) -> bool {
    a.text.to_lowercase() == b.text.to_lowercase() && same_scope(&a.scope, &b.scope)
}

/// Puts the search first, drops an older copy of it (case does not matter; the newest spelling
/// stays), and keeps at most [`MAX_RECENT_SEARCHES`]. An empty query changes nothing.
pub fn push_search(list: &mut Vec<RecentSearch>, query: &str, scope: Option<Scope>) {
    let search = RecentSearch::new(query, scope);
    if search.text.is_empty() {
        return;
    }
    list.retain(|item| !same(item, &search));
    list.insert(0, search);
    list.truncate(MAX_RECENT_SEARCHES);
}

/// Removes one search, however it was capitalized. The same text in another scope stays.
pub fn remove_search(list: &mut Vec<RecentSearch>, search: &RecentSearch) {
    list.retain(|item| !same(item, search));
}

/// Reads what the store holds. A list written by an older version has plain strings; a list
/// from a future version could be longer or hold empty rows.
fn from_stored(stored: Vec<Stored>) -> Vec<RecentSearch> {
    let mut list: Vec<RecentSearch> = Vec::new();
    for item in stored {
        let search = match item {
            Stored::Plain(text) => RecentSearch::new(&text, None),
            Stored::Search(search) => RecentSearch::new(&search.text, search.scope),
        };
        if !search.text.is_empty() && !list.iter().any(|earlier| same(earlier, &search)) {
            list.push(search);
        }
    }
    list.truncate(MAX_RECENT_SEARCHES);
    list
}

pub async fn load() -> Vec<RecentSearch> {
    from_stored(
        store::get_meta::<Vec<Stored>>(KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default(),
    )
}

async fn save(list: &[RecentSearch]) {
    let stored: Vec<Stored> = list.iter().cloned().map(Stored::Search).collect();
    let _ = store::set_meta(KEY, &stored).await;
}

/// Adds a submitted query with the scope it ran in and returns the new list.
pub async fn record(query: &str, scope: Option<Scope>) -> Vec<RecentSearch> {
    let mut list = load().await;
    push_search(&mut list, query, scope);
    save(&list).await;
    list
}

/// Removes one search and returns the new list.
pub async fn remove(search: &RecentSearch) -> Vec<RecentSearch> {
    let mut list = load().await;
    remove_search(&mut list, search);
    save(&list).await;
    list
}

pub async fn clear() {
    save(&[]).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::EntryKind;

    fn feats() -> Option<Scope> {
        Some(Scope {
            kind: EntryKind::Feat,
            facets: Vec::new(),
        })
    }

    fn combat_feats() -> Option<Scope> {
        Some(Scope {
            kind: EntryKind::Feat,
            facets: vec![("feat_type".into(), "Combat".into())],
        })
    }

    fn texts(list: &[RecentSearch]) -> Vec<&str> {
        list.iter().map(|item| item.text.as_str()).collect()
    }

    fn push_all(list: &mut Vec<RecentSearch>, queries: &[&str]) {
        for query in queries {
            push_search(list, query, None);
        }
    }

    #[test]
    fn newest_first() {
        let mut list = Vec::new();
        push_all(&mut list, &["fireball", "grapple", "dragon"]);
        assert_eq!(texts(&list), ["dragon", "grapple", "fireball"]);
    }

    #[test]
    fn a_repeated_query_moves_to_the_front_once() {
        let mut list = Vec::new();
        push_all(&mut list, &["fireball", "grapple", "dragon", "fireball"]);
        assert_eq!(texts(&list), ["fireball", "dragon", "grapple"]);
    }

    #[test]
    fn case_and_spacing_do_not_make_a_second_entry() {
        let mut list = Vec::new();
        push_all(&mut list, &["Cure Light", "  cure   light "]);
        assert_eq!(texts(&list), ["cure light"]);
    }

    #[test]
    fn the_same_text_in_another_scope_is_another_entry() {
        let mut list = Vec::new();
        push_search(&mut list, "grapple", None);
        push_search(&mut list, "grapple", feats());
        push_search(&mut list, "grapple", combat_feats());
        assert_eq!(list.len(), 3);
        assert_eq!(
            list.iter()
                .map(RecentSearch::scope_label)
                .collect::<Vec<_>>(),
            [Some("Combat feats".into()), Some("Feats".into()), None]
        );
        // A repeat in the same scope moves that entry to the front and leaves the others.
        push_search(&mut list, "GRAPPLE", feats());
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].scope_label().as_deref(), Some("Feats"));
        assert_eq!(list[0].text, "GRAPPLE");
        // The facet value compares without regard to case.
        let mut lower = combat_feats().unwrap();
        lower.facets[0].1 = "combat".into();
        push_search(&mut list, "grapple", Some(lower));
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].scope_label().as_deref(), Some("Combat feats"));
    }

    #[test]
    fn the_list_is_capped_and_drops_the_oldest() {
        let mut list = Vec::new();
        for n in 0..25 {
            push_search(&mut list, &format!("query {n}"), feats());
        }
        assert_eq!(list.len(), MAX_RECENT_SEARCHES);
        assert_eq!(list[0].text, "query 24");
        assert_eq!(list[MAX_RECENT_SEARCHES - 1].text, "query 15");
        // A repeat at the cap does not push anything else out.
        push_search(&mut list, "query 20", feats());
        assert_eq!(list.len(), MAX_RECENT_SEARCHES);
        assert_eq!(list[0].text, "query 20");
        assert_eq!(list[MAX_RECENT_SEARCHES - 1].text, "query 15");
        // The same text in another scope is new, and does push the oldest out.
        push_search(&mut list, "query 20", None);
        assert_eq!(list.len(), MAX_RECENT_SEARCHES);
        assert_eq!(list[MAX_RECENT_SEARCHES - 1].text, "query 16");
    }

    #[test]
    fn an_empty_query_is_ignored() {
        let mut list = vec![RecentSearch::new("fireball", None)];
        push_search(&mut list, "", None);
        push_search(&mut list, "   ", feats());
        assert_eq!(texts(&list), ["fireball"]);
    }

    #[test]
    fn remove_takes_out_one_search_and_keeps_the_order() {
        let mut list = Vec::new();
        push_search(&mut list, "a", None);
        push_search(&mut list, "b", feats());
        push_search(&mut list, "b", None);
        push_search(&mut list, "c", None);
        // Same text, other scope: stays.
        remove_search(&mut list, &RecentSearch::new("B", feats()));
        assert_eq!(texts(&list), ["c", "b", "a"]);
        assert_eq!(list[1].scope, None);
        remove_search(&mut list, &RecentSearch::new("missing", None));
        assert_eq!(texts(&list), ["c", "b", "a"]);
    }

    #[test]
    fn a_list_from_before_scopes_reads_as_unscoped_searches() {
        let stored: Vec<Stored> =
            serde_json::from_str(r#"["fireball", "  Cure   Light ", "", "fireball"]"#).unwrap();
        let list = from_stored(stored);
        assert_eq!(texts(&list), ["fireball", "Cure Light"]);
        assert!(list.iter().all(|item| item.scope.is_none()));
        assert_eq!(list[0].route(), Route::search("fireball"));
    }

    #[test]
    fn a_mixed_list_keeps_each_scope_and_a_search_round_trips_through_the_store_format() {
        let stored: Vec<Stored> = serde_json::from_str(
            r#"["fireball",
                {"text": "grapple", "scope": {"kind": "feat", "facets": [["feat_type", "Combat"]]}},
                {"text": "dragon"},
                {"text": "  ", "scope": {"kind": "feat", "facets": []}}]"#,
        )
        .unwrap();
        let list = from_stored(stored);
        assert_eq!(texts(&list), ["fireball", "grapple", "dragon"]);
        assert_eq!(list[1].scope, combat_feats());
        assert_eq!(list[2].scope, None);
        // What is saved reads back the same, and an unscoped search writes no scope key.
        let saved: Vec<Stored> = list.iter().cloned().map(Stored::Search).collect();
        let json = serde_json::to_string(&saved).unwrap();
        assert!(!json.contains("null"), "{json}");
        let back = from_stored(serde_json::from_str(&json).unwrap());
        assert_eq!(back, list);
    }

    #[test]
    fn tapping_a_recent_search_opens_it_in_its_scope() {
        let search = RecentSearch::new("grapple", combat_feats());
        assert_eq!(
            search.route().href(),
            "#/search?q=grapple&kind=feat&facet=feat_type:Combat"
        );
        assert_eq!(search.name(), "grapple in Combat feats");
        let everywhere = RecentSearch::new("grapple", None);
        assert_eq!(everywhere.route().href(), "#/search?q=grapple");
        assert_eq!(everywhere.name(), "grapple");
    }

    #[test]
    fn a_list_longer_than_the_cap_is_cut() {
        let stored: Vec<Stored> = (0..30).map(|n| Stored::Plain(format!("q{n}"))).collect();
        let list = from_stored(stored);
        assert_eq!(list.len(), MAX_RECENT_SEARCHES);
        assert_eq!(list[0].text, "q0");
    }
}
