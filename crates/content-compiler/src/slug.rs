//! Identifier slugs. The rule is part of the stable-id contract: changing it changes ids.

use unicode_normalization::UnicodeNormalization;

const URL_SCHEME: &str = "pfsrd://";

/// Lowercase, ASCII-fold, drop apostrophes, and turn every other non-alphanumeric run into `-`.
pub fn slug_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_dash = false;
    for ch in input.nfkd() {
        let folded: &str = match ch {
            'æ' | 'Æ' => "ae",
            'œ' | 'Œ' => "oe",
            'ß' => "ss",
            'ø' | 'Ø' => "o",
            'đ' | 'Đ' => "d",
            'ł' | 'Ł' => "l",
            _ => "",
        };
        let mut buf = [0u8; 4];
        let piece: &str = if folded.is_empty() {
            ch.encode_utf8(&mut buf)
        } else {
            folded
        };
        for c in piece.chars() {
            if matches!(c, '\'' | '\u{2019}' | '\u{2018}' | '`') {
                continue;
            }
            if c.is_ascii_alphanumeric() {
                if pending_dash && !out.is_empty() {
                    out.push('-');
                }
                pending_dash = false;
                out.push(c.to_ascii_lowercase());
            } else if c.is_ascii() || !is_combining(c) {
                pending_dash = true;
            }
        }
    }
    out
}

fn is_combining(c: char) -> bool {
    ('\u{0300}'..='\u{036f}').contains(&c)
}

/// Slugifies each `/` segment and drops segments that become empty.
pub fn slug_path(path: &str) -> String {
    path.split('/')
        .map(slug_segment)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// The path part of a `pfsrd://` url.
pub fn url_path(url: &str) -> Option<&str> {
    url.strip_prefix(URL_SCHEME)
}

/// The id derived from a `pfsrd://` url, or `None` when the url has no usable path.
pub fn url_slug(url: &str) -> Option<String> {
    let slug = slug_path(url_path(url)?);
    (!slug.is_empty()).then_some(slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_the_documented_example() {
        assert_eq!(
            url_slug("pfsrd://Core Rulebook/Spells/Fireball").as_deref(),
            Some("core-rulebook/spells/fireball")
        );
    }

    #[test]
    fn drops_apostrophes_and_folds_ascii() {
        assert_eq!(slug_segment("Will-o'-Wisp"), "will-o-wisp");
        assert_eq!(
            slug_segment("Advanced Player\u{2019}s Guide"),
            "advanced-players-guide"
        );
        assert_eq!(slug_segment("Café Ærie"), "cafe-aerie");
    }

    #[test]
    fn collapses_runs_and_trims_segments() {
        assert_eq!(slug_segment("  Light; Steel...  "), "light-steel");
        assert_eq!(slug_path("A  B/ -C- //D"), "a-b/c/d");
        assert_eq!(slug_segment("..."), "");
    }

    #[test]
    fn rejects_urls_without_scheme_or_path() {
        assert_eq!(url_slug("http://x/y"), None);
        assert_eq!(url_slug("pfsrd://"), None);
        assert_eq!(url_slug("pfsrd:///.../"), None);
    }

    #[test]
    fn slugs_are_valid_entry_ids() {
        let slug = url_slug("pfsrd://Bestiary/Monsters/Will-o'-Wisp").unwrap();
        assert!(cast_iron_schema::EntryId::parse(slug).is_ok());
    }
}
