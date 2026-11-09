//! The "Report a problem" link.
//!
//! One control sits in the footer. An entry route opens the `phynics/cast-iron-data` issue
//! form with the entry id; every other route opens the app issue form added in this repository
//! with the route and the app and library versions. The URLs are built here so that the
//! percent-encoding is native-tested, and the control itself is a plain anchor: nothing is
//! fetched until the reader clicks.

use crate::route::{Route, percent_encode};
use cast_iron_schema::EntryId;

/// The content issue form. Its identity field is `entry`; the other fields describe the
/// problem and stay empty so the reader fills them in. A later form change can add fields
/// without changing the route this link opens.
const DATA_FORM: &str = "https://github.com/phynics/cast-iron-data/issues/new";

/// The app issue form in this repository, `.github/ISSUE_TEMPLATE/app-problem.yml`.
const APP_FORM: &str = "https://github.com/phynics/cast-iron/issues/new";

/// The longest encoded route that goes into the URL. The route comes from the address bar and
/// can be arbitrarily long; capping it keeps the prefilled URL well under GitHub's 8 KB limit.
const MAX_ROUTE_BYTES: usize = 4096;

/// The footer control's target and accessible name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportLink {
    pub url: String,
    pub label: String,
}

/// The URL for an entry problem.
pub fn entry_url(id: &EntryId) -> String {
    form_url(
        DATA_FORM,
        &[("template", "data-problem.yml"), ("entry", id.as_str())],
    )
}

/// The URL for an app problem on any other page.
pub fn app_url(
    route: &str,
    app_version: &str,
    content_version: &str,
    schema_version: u32,
) -> String {
    let schema_version = schema_version.to_string();
    let route = truncate_encoded(route, MAX_ROUTE_BYTES);
    form_url(
        APP_FORM,
        &[
            ("template", "app-problem.yml"),
            ("route", route),
            ("app-version", app_version),
            ("content-version", content_version),
            ("schema-version", &schema_version),
        ],
    )
}

/// The link for the page that is open. `entry_title` names an entry page in the accessible
/// name; it never enters the URL.
pub fn for_route(
    route: &Route,
    entry_title: Option<&str>,
    app_version: &str,
    content_version: &str,
    schema_version: u32,
) -> ReportLink {
    if let Route::Entry { id, .. } = route {
        let label = match entry_title.filter(|title| !title.is_empty()) {
            Some(title) => format!("Report a problem with {title} on GitHub (opens in a new tab)"),
            None => "Report a problem with this entry on GitHub (opens in a new tab)".to_string(),
        };
        return ReportLink {
            url: entry_url(id),
            label,
        };
    }
    ReportLink {
        url: app_url(&route.href(), app_version, content_version, schema_version),
        label: "Report a problem on GitHub (opens in a new tab)".to_string(),
    }
}

/// `base?key=value&...`, skipping empty values. Every key and value is percent-encoded, so a
/// value's `&`, `#`, quotes, and non-ASCII characters cannot change the query's shape.
fn form_url(base: &str, fields: &[(&str, &str)]) -> String {
    let mut url = String::from(base);
    let mut separator = '?';
    for (key, value) in fields {
        if value.is_empty() {
            continue;
        }
        url.push(separator);
        separator = '&';
        url.push_str(&percent_encode(key));
        url.push('=');
        url.push_str(&percent_encode(value));
    }
    url
}

/// The longest prefix of `value` whose percent-encoded form is at most `max` bytes, never
/// splitting a UTF-8 sequence or an encoded character.
fn truncate_encoded(value: &str, max: usize) -> &str {
    let mut used = 0;
    let mut buffer = [0u8; 4];
    for (at, character) in value.char_indices() {
        let encoded = percent_encode(character.encode_utf8(&mut buffer));
        if used + encoded.len() > max {
            return &value[..at];
        }
        used += encoded.len();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> EntryId {
        EntryId::parse(value).unwrap()
    }

    /// Decode the query of `url` into its pairs, as GitHub reads it.
    fn query(url: &str) -> Vec<(String, String)> {
        let query = url.split_once('?').expect("a query").1;
        query
            .split('&')
            .map(|pair| {
                let (key, value) = pair.split_once('=').expect("a key and a value");
                (
                    percent_decode(key).expect("decoded key"),
                    percent_decode(value).expect("decoded value"),
                )
            })
            .collect()
    }

    fn percent_decode(value: &str) -> Option<String> {
        crate::route::percent_decode(value)
    }

    fn field<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
        pairs
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn an_entry_url_opens_the_data_form_with_the_ref_id() {
        let url = entry_url(&id("ref/spell/fog-cloud"));
        assert_eq!(
            url,
            "https://github.com/phynics/cast-iron-data/issues/new?template=data-problem.yml&entry=ref%2Fspell%2Ffog-cloud"
        );
        let pairs = query(&url);
        assert_eq!(field(&pairs, "template"), Some("data-problem.yml"));
        assert_eq!(field(&pairs, "entry"), Some("ref/spell/fog-cloud"));
        assert_eq!(pairs.len(), 2, "entry identity is the only data field");
    }

    #[test]
    fn an_app_url_carries_the_route_and_versions() {
        let url = app_url("#/settings", "shell1234", "v0000000000000001", 4);
        let pairs = query(&url);
        assert_eq!(field(&pairs, "template"), Some("app-problem.yml"));
        assert_eq!(field(&pairs, "route"), Some("#/settings"));
        assert_eq!(field(&pairs, "app-version"), Some("shell1234"));
        assert_eq!(field(&pairs, "content-version"), Some("v0000000000000001"));
        assert_eq!(field(&pairs, "schema-version"), Some("4"));
        // The route is a fragment, so its '#' must arrive encoded.
        assert!(url.contains("route=%23%2Fsettings"));
    }

    #[test]
    fn every_separator_and_non_ascii_character_is_encoded() {
        let url = app_url(
            "#/search?q=a&b=\"c#d\"&name=Zoltan Üllő",
            "shell&1",
            "v1#2",
            4,
        );
        let pairs = query(&url);
        assert_eq!(
            field(&pairs, "route"),
            Some("#/search?q=a&b=\"c#d\"&name=Zoltan Üllő")
        );
        assert_eq!(field(&pairs, "app-version"), Some("shell&1"));
        assert_eq!(field(&pairs, "content-version"), Some("v1#2"));
        assert_eq!(pairs.len(), 5, "a value's '&' cannot add a field");
        // No raw '#', '&', or quote from a value reaches the query.
        let values = url.split_once('?').unwrap().1;
        assert!(!values.contains('#'));
        assert!(!values.contains('"'));
    }

    #[test]
    fn a_long_route_is_capped_and_stays_well_under_the_limit() {
        let long = format!("#/search?q={}", "ü".repeat(10_000));
        let url = app_url(&long, "shell", "v1", 4);
        assert!(url.len() < 8_192, "{} bytes", url.len());
        let pairs = query(&url);
        let route = field(&pairs, "route").unwrap();
        assert!(route.len() <= MAX_ROUTE_BYTES, "{} bytes", route.len());
        assert!(route.starts_with("#/search?q="));
    }

    #[test]
    fn an_entry_title_never_enters_the_url() {
        let title = format!("Fog Cloud & \"Friends\" #{}", "ü".repeat(4_000));
        let link = for_route(
            &Route::entry(id("ref/spell/fog-cloud")),
            Some(&title),
            "shell",
            "v1",
            4,
        );
        assert!(!link.url.contains("ü"));
        assert!(link.url.len() < 8_192);
        assert_eq!(
            link.label,
            format!("Report a problem with {title} on GitHub (opens in a new tab)")
        );
    }

    #[test]
    fn routes_without_an_entry_use_the_app_form_and_its_label() {
        for route in [Route::Home, Route::Settings, Route::Legal] {
            let link = for_route(&route, None, "shell", "v1", 4);
            assert_eq!(
                link.label,
                "Report a problem on GitHub (opens in a new tab)"
            );
            assert!(link.url.starts_with(APP_FORM));
            let href = route.href();
            let pairs = query(&link.url);
            assert_eq!(field(&pairs, "route"), Some(href.as_str()));
        }
    }

    #[test]
    fn an_empty_version_leaves_its_field_out() {
        let url = app_url("#/", "", "", 4);
        let pairs = query(&url);
        assert!(field(&pairs, "app-version").is_none());
        assert!(field(&pairs, "content-version").is_none());
        assert_eq!(field(&pairs, "schema-version"), Some("4"));
    }
}
