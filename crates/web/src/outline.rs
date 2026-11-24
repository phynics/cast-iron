//! The heading levels of an entry body (#12, finding L3).
//!
//! The corpus stores the levels of the printed text: a class page is an `h1` title followed by
//! `###` sections, a creature the same. The page already has its own `h1`, so a body that starts
//! at `h3` skips a level (WCAG 1.3.1). This module maps the stored levels of one body to the
//! levels of the elements that render them: the shallowest heading of the body becomes `h2`, the
//! rest keep their distance from it, and no heading is more than one level below the heading
//! before it. The look of a heading follows the stored level, not the element (`data-printed-level`
//! in `web/styles.css`), so the page reads as it did.
//!
//! The functions are pure, so they have native tests.

/// The deepest heading level HTML has.
const DEEPEST: u8 = 6;

/// The level of the first heading of a body: the page's own `h1` stands above it.
const FIRST: u8 = 2;

/// How one heading renders: the element it is, and the level whose look it keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// The level of the element (`2` is an `h2`).
    pub element: u8,
    /// The level the corpus stored, from 2 to 6 (a stored `0` or `1` has always rendered as `h2`).
    pub printed: u8,
}

/// The stored level, as it has always rendered: no lower than `h2` and no higher than `h6`.
pub fn printed(stored: u8) -> u8 {
    stored.clamp(FIRST, DEEPEST)
}

/// The levels of the headings of one body, given their stored levels in document order.
///
/// The shallowest stored level maps to `h2`, and every other level moves by the same amount, so
/// the nesting is the one the corpus wrote. A heading never sits more than one level below the
/// one before it, so a gap in the corpus (`###` then `#####`) closes up and a first heading that
/// is not the shallowest still starts at `h2`.
pub fn levels(stored: &[u8]) -> Vec<Level> {
    let all: Vec<(u8, bool)> = stored.iter().map(|level| (*level, true)).collect();
    levels_among(&all)
}

/// [`levels`] for a body whose outline also holds headings that the renderer makes, such as the
/// name of a class-option power. Each item is its level and whether it counts when the
/// shallowest level is found: a power name takes its level from the heading before it, so it
/// follows the shift and never sets it.
pub fn levels_among(items: &[(u8, bool)]) -> Vec<Level> {
    let Some(shallowest) = items
        .iter()
        .filter(|(_, sets)| *sets)
        .map(|(level, _)| printed(*level))
        .min()
    else {
        return items
            .iter()
            .map(|(level, _)| Level {
                element: printed(*level),
                printed: printed(*level),
            })
            .collect();
    };
    let shift = shallowest - FIRST;
    let mut previous = FIRST - 1;
    items
        .iter()
        .map(|(level, _)| {
            let printed = printed(*level);
            let element = (printed - shift).min(previous + 1).clamp(FIRST, DEEPEST);
            previous = element;
            Level { element, printed }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elements(stored: &[u8]) -> Vec<u8> {
        levels(stored).iter().map(|level| level.element).collect()
    }

    #[test]
    fn a_body_with_no_heading_has_no_levels() {
        assert!(levels(&[]).is_empty());
    }

    #[test]
    fn the_shallowest_heading_becomes_h2_and_the_rest_keep_their_distance() {
        assert_eq!(elements(&[3, 4, 4, 5, 3]), [2, 3, 3, 4, 2]);
        assert_eq!(elements(&[4, 5, 6]), [2, 3, 4]);
    }

    #[test]
    fn a_body_that_starts_at_h2_is_unchanged() {
        assert_eq!(elements(&[2, 3, 4, 3, 2]), [2, 3, 4, 3, 2]);
    }

    #[test]
    fn a_stored_h1_or_h0_renders_as_h2_as_it_always_did() {
        assert_eq!(elements(&[1, 3]), [2, 3]);
        assert_eq!(elements(&[0, 2, 3]), [2, 2, 3]);
        assert_eq!(levels(&[1])[0].printed, 2);
    }

    #[test]
    fn the_printed_level_is_the_stored_one() {
        let printed: Vec<u8> = levels(&[3, 4, 6]).iter().map(|l| l.printed).collect();
        assert_eq!(printed, [3, 4, 6]);
    }

    #[test]
    fn a_gap_closes_up() {
        assert_eq!(elements(&[3, 5, 6]), [2, 3, 4]);
        assert_eq!(elements(&[3, 4, 6, 3, 6]), [2, 3, 4, 2, 3]);
    }

    #[test]
    fn a_first_heading_below_the_shallowest_starts_at_h2() {
        assert_eq!(elements(&[4, 3, 4]), [2, 2, 3]);
    }

    #[test]
    fn no_level_is_lost_below_h6() {
        assert_eq!(elements(&[2, 3, 4, 5, 6, 6]), [2, 3, 4, 5, 6, 6]);
        assert!(elements(&[2, 3, 4, 5, 6]).iter().all(|level| *level <= 6));
    }

    #[test]
    fn a_power_name_follows_the_shift_and_never_sets_it() {
        // A power before the first heading is `h2`; the `###` sections after it still start the
        // shift, and a power under one is a level below it.
        let items = [(2, false), (3, true), (4, false), (3, true)];
        let elements: Vec<u8> = levels_among(&items).iter().map(|l| l.element).collect();
        assert_eq!(elements, [2, 2, 3, 2]);
    }

    #[test]
    fn no_outline_skips_a_level() {
        for stored in [
            vec![3, 5, 3, 6, 4],
            vec![6, 2, 6],
            vec![4, 4, 6, 6, 5, 3],
            vec![1, 5, 1, 4],
        ] {
            let mut previous = 1;
            for level in elements(&stored) {
                assert!(level <= previous + 1, "{stored:?}");
                previous = level;
            }
        }
    }
}
