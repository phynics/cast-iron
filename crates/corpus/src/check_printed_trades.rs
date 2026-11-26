//! The corpus-check rule for a printed trade sentence that no stored trade states (#540).
//!
//! In an archetype or class option with `features` (corpus format 4), a feature may print
//! "This alters class skills." or "These replace the normal attack roll bonus and AC penalty for
//! charging." The drift check (`drift.rs`) compares every such sentence with the trades whose `by`
//! names the feature, but the reader ([`crate::trade_sentences`]) treats a sentence whose subject
//! names no ability as *bare*: "This alters X" and "These replace X" count as a trade sentence
//! only when X names a stored trade of the entry, because the pronoun may stand for anything. So a
//! trade that was never stored left no trace: the sentence was bare, named no trade, and was
//! dropped. Data pull request 324 (batch 4) held seven of them. This rule closes the gap: a bare
//! trade sentence in a listed feature that names no stored trade of its op is a missing trade.
//!
//! What the rule does not read stays as it is:
//!
//! - a sentence of the opening text, or of a section under a scaffold heading that no listed
//!   feature contains: those trades have no `by` (owner ruling 28 and 13);
//! - a phrase that ends in "as indicated below" and a parenthesized "(see ...)" or other aside
//!   (owner rulings 16, 29 and 40): the reader drops them before this rule sees a sentence, so a
//!   name that only an aside holds needs no trade;
//! - a sentence that names a stored trade: the drift check compares it, so a wrong `by` or a
//!   phrase that the trades cover only in part is drift there;
//! - an anaphoric phrase ("... works like the standard mutagen class feature and replaces that
//!   ability") names nothing itself; it needs a trade of its op with `by` = the feature, of any
//!   name;
//! - the abilities that a companion gains (owner ruling 37) are unlisted `###` sections inside
//!   the companion feature, so they belong to that feature's section.
//!
//! A sentence that states no concrete trade on purpose ("This alters her other class features, as
//! detailed in each relevant class feature") is listed in `trade-sentence-exceptions.toml` in the
//! corpus root (see `docs/corpus-format.md`, "Trade sentence exceptions"). The list only shrinks:
//! a row that excepts no sentence is an error too (`remove the row`).
//!
//! The rule is an error because [`PRINTED_TRADES_ARE_ERRORS`] is true. It was a warning until the
//! pin of corpus v0.1.57, the first tag that holds the data fixes (phynics/cast-iron-data#324 and
//! #329).

use crate::check::{Report, line_of};
use crate::check_features::{bare_unnamed_sentences_in, feature_part, sections};
use crate::load::Corpus;
use crate::trade_sentences::TradeSentence;
use cast_iron_schema::headings::plain_text;
use cast_iron_schema::{Block, EntryKind, RuleEntry};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

/// Whether a printed trade sentence without a stored trade fails the check (true) or only warns
/// (false). True since the pin of corpus v0.1.57, which holds the data fixes of
/// phynics/cast-iron-data#324 and #329 (decision `2026-11-25-printed-trade-sentence-needs-a-trade`).
pub const PRINTED_TRADES_ARE_ERRORS: bool = true;

/// The optional file in the corpus root that lists the sentences allowed to state no trade.
pub const EXCEPTIONS_FILE: &str = "trade-sentence-exceptions.toml";

/// `trade-sentence-exceptions.toml`: one `[[exception]]` row per sentence.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionFile {
    #[serde(default)]
    exception: Vec<toml::Spanned<SentenceException>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SentenceException {
    /// The entry id (`ref/arc-shifter/weretouched`).
    entry: String,
    /// The anchor of the listed feature that prints the sentence.
    feature: String,
    /// The sentence, or a distinctive quote of it: a text that the sentence holds, case and
    /// spacing aside ("her other class features, as detailed in each relevant class feature").
    sentence: String,
    /// Why the sentence states no trade.
    reason: String,
    /// The issue that decides it, as `owner/repo#number`.
    issue: String,
}

/// A row with the line of its first key.
#[derive(Debug)]
struct Row {
    line: usize,
    exception: SentenceException,
}

/// The 1-based line that byte `offset` of `text` is on.
fn line_at(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

/// Whether `issue` reads `owner/repo#number`.
fn is_issue_reference(issue: &str) -> bool {
    let Some((repo, number)) = issue.split_once('#') else {
        return false;
    };
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let word = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
    };
    word(owner)
        && word(name)
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

/// Lower case, with runs of whitespace as one space, for comparing a quote with a sentence.
fn squash(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Reads the exceptions file of the corpus in `dir`. A missing file holds no exception.
fn read_exceptions(dir: &Path) -> Result<Vec<Row>, String> {
    let path = dir.join(EXCEPTIONS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{EXCEPTIONS_FILE}:1: cannot read file: {error}")),
    };
    parse_exceptions(&text)
}

fn parse_exceptions(text: &str) -> Result<Vec<Row>, String> {
    let file: ExceptionFile = toml::from_str(text).map_err(|error| {
        let line = error.span().map_or(1, |span| line_at(text, span.start));
        format!("{EXCEPTIONS_FILE}:{line}: {}", error.message())
    })?;
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for spanned in file.exception {
        let line = line_at(text, spanned.span().start);
        let exception = spanned.into_inner();
        let at = |message: &str| {
            format!(
                "{EXCEPTIONS_FILE}:{line}: exception for `{}` in the feature `{}`: {message}",
                exception.entry, exception.feature
            )
        };
        if squash(&exception.sentence).is_empty() {
            return Err(at("`sentence` needs a text"));
        }
        if exception.reason.trim().is_empty() {
            return Err(at("`reason` needs a text"));
        }
        if !is_issue_reference(&exception.issue) {
            return Err(at("`issue` reads `owner/repo#number`"));
        }
        if !seen.insert((
            exception.entry.clone(),
            exception.feature.clone(),
            squash(&exception.sentence),
        )) {
            return Err(at("a second exception for the same sentence"));
        }
        rows.push(Row { line, exception });
    }
    Ok(rows)
}

/// A printed trade sentence with no stored trade.
#[derive(Debug, PartialEq, Eq)]
struct Gap {
    /// The anchor of the feature that prints the sentence.
    feature: String,
    sentence: TradeSentence,
}

impl Gap {
    /// The sentence as the reader read it: "This alters class skills".
    fn printed(&self) -> String {
        format!("This {} {}", self.sentence.op.slug(), self.sentence.phrase)
    }

    /// Whether the quote of an exception row names this sentence.
    fn is_quoted_by(&self, quote: &str) -> bool {
        let quote = squash(quote);
        [self.printed(), self.sentence.phrase.clone()]
            .iter()
            .any(|text| squash(text).contains(&quote))
    }
}

/// True when `phrase` is only a demonstrative ("that ability", "those abilities", "it"): the name
/// it stands for sits earlier in the paragraph ("works like the standard mutagen class feature
/// and replaces that ability").
fn is_anaphoric(phrase: &str) -> bool {
    matches!(
        crate::trade_sentences::name_words(phrase)
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()[..],
        ["that" | "those" | "this" | "these" | "it" | "them"]
    )
}

/// The printed trade sentences of the listed features of `entry` that no stored trade states.
fn gaps(entry: &RuleEntry) -> Vec<Gap> {
    if !matches!(entry.kind, EntryKind::Archetype | EntryKind::ClassOption) {
        return Vec::new();
    }
    let features = entry.fields.features();
    let found = sections(feature_part(entry), &entry.body);
    let mut out = Vec::new();
    for (index, feature) in features.iter().enumerate() {
        if features[..index]
            .iter()
            .any(|other| other.anchor == feature.anchor)
        {
            continue;
        }
        let Some(section) = found.iter().find(|section| section.key == feature.anchor) else {
            continue;
        };
        let head = match section.heading {
            Block::Heading { content, .. } => Some(plain_text(content)),
            _ => None,
        };
        let trades = entry.fields.trades();
        for sentence in bare_unnamed_sentences_in(section.blocks, trades, head.as_deref()) {
            // "... and replaces that ability": the phrase points back to a name printed earlier
            // in the paragraph, so it names no trade itself. It needs a trade of its op that the
            // feature states, whatever the name.
            if is_anaphoric(&sentence.phrase)
                && trades
                    .iter()
                    .any(|trade| trade.op == sentence.op && trade.by.contains(&feature.anchor))
            {
                continue;
            }
            let gap = Gap {
                feature: feature.anchor.clone(),
                sentence,
            };
            if !out.contains(&gap) {
                out.push(gap);
            }
        }
    }
    out
}

pub(crate) fn check_printed_trades(corpus: &Corpus, report: &mut Report) {
    match read_exceptions(&corpus.dir) {
        Ok(rows) => check_sentences(corpus, &rows, report),
        Err(error) => {
            report.problems.push(error);
            check_sentences(corpus, &[], report);
        }
    }
}

fn check_sentences(corpus: &Corpus, rows: &[Row], report: &mut Report) {
    let mut used = vec![false; rows.len()];
    let mut messages = Vec::new();
    for stored in corpus.entries.values() {
        let entry = &stored.entry;
        for gap in gaps(entry) {
            let excepted = rows.iter().enumerate().filter(|(_, row)| {
                row.exception.entry == entry.id.as_str()
                    && row.exception.feature == gap.feature
                    && gap.is_quoted_by(&row.exception.sentence)
            });
            let mut any = false;
            for (index, _) in excepted {
                used[index] = true;
                any = true;
            }
            if any {
                continue;
            }
            let at = format!(
                "{}:{}",
                stored.path.display(),
                line_of(&stored.text, &format!("{{#{}}}", gap.feature))
            );
            messages.push(format!(
                "{at}: [trades] the feature `{}` prints \"{}.\" but the entry has no `{}` trade named \"{}\" with `by` = [\"{}\"]; store `{{ {} = \"{}\", by = [\"{}\"] }}`, or list the sentence in `{EXCEPTIONS_FILE}` when it states no trade",
                gap.feature,
                gap.printed(),
                gap.sentence.op.slug(),
                gap.sentence.phrase,
                gap.feature,
                gap.sentence.op.slug(),
                gap.sentence.phrase,
                gap.feature
            ));
        }
    }
    for (row, used) in rows.iter().zip(&used) {
        if *used {
            continue;
        }
        let why = match corpus.entries.get(&row.exception.entry) {
            None => "the entry does not exist".to_string(),
            Some(stored)
                if !stored
                    .entry
                    .fields
                    .features()
                    .iter()
                    .any(|feature| feature.anchor == row.exception.feature) =>
            {
                "the entry lists no such feature".to_string()
            }
            Some(_) => {
                "the feature prints no such trade sentence without a stored trade".to_string()
            }
        };
        messages.push(format!(
            "{EXCEPTIONS_FILE}:{}: [trades] exception for `{}` in the feature `{}`: {why}; remove the row",
            row.line, row.exception.entry, row.exception.feature
        ));
    }
    let target = if PRINTED_TRADES_ARE_ERRORS {
        &mut report.problems
    } else {
        &mut report.warnings
    };
    target.extend(messages);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus_in_format, heading, paragraph, rule};
    use cast_iron_schema::{ArchetypeFields, FeatureFact, Inline, Trade, TradeOp, TypedFields};

    fn text(text: &str) -> Block {
        paragraph(vec![Inline::text(text)])
    }

    fn archetype(
        body: Vec<Block>,
        features: &[&str],
        trades: Vec<Trade>,
    ) -> cast_iron_schema::RuleEntry {
        let mut entry = rule("ref/arc-hunter/roof-runner", body);
        entry.kind = EntryKind::Archetype;
        entry.fields = TypedFields::Archetype(ArchetypeFields {
            features: features
                .iter()
                .map(|anchor| FeatureFact {
                    anchor: (*anchor).into(),
                    level: None,
                })
                .collect(),
            trades,
            ..Default::default()
        });
        entry
    }

    fn trade(op: TradeOp, printed: &str, by: &[&str]) -> Trade {
        Trade {
            by: by.iter().map(|anchor| anchor.to_string()).collect(),
            ..Trade::unresolved(op, printed)
        }
    }

    fn found(entry: cast_iron_schema::RuleEntry) -> Vec<String> {
        gaps(&entry).iter().map(Gap::printed).collect()
    }

    fn body(sentence: &str) -> Vec<Block> {
        vec![
            text("Roof runners lurk in the shadows."),
            heading(Some("skilled"), "Skilled (Ex)"),
            text("A roof runner adds Acrobatics to her list of class skills."),
            text(sentence),
            heading(Some("natural-leaper"), "Natural Leaper (Ex)"),
            text("At 2nd level, a roof runner jumps. This ability replaces track."),
        ]
    }

    fn track() -> Trade {
        trade(TradeOp::Replaces, "Track", &["natural-leaper"])
    }

    #[test]
    fn a_bare_sentence_without_a_trade_is_a_gap() {
        let entry = archetype(
            body("This alters the hunter's class skills."),
            &["skilled", "natural-leaper"],
            vec![track()],
        );
        assert_eq!(found(entry), ["This alters the hunter's class skills"]);
    }

    #[test]
    fn a_bare_modifies_sentence_is_a_gap() {
        // Owner ruling 55 of 2026-11-26 (#540): `modifies` is an `alters` verb.
        for sentence in [
            "This modifies the hunter's class skills.",
            "These modify the hunter's class skills.",
        ] {
            let entry = archetype(
                body(sentence),
                &["skilled", "natural-leaper"],
                vec![track()],
            );
            let gaps = found(entry);
            assert_eq!(gaps.len(), 1, "{sentence}: {gaps:?}");
            assert!(gaps[0].ends_with("the hunter's class skills"), "{gaps:?}");
            assert!(gaps[0].contains("alters"), "{gaps:?}");
        }
        // A stored `alters` trade of the feature states it.
        let entry = archetype(
            body("This modifies the hunter's class skills."),
            &["skilled", "natural-leaper"],
            vec![
                trade(TradeOp::Alters, "Class skills", &["skilled"]),
                track(),
            ],
        );
        assert!(found(entry).is_empty());
    }

    #[test]
    fn a_plural_bare_sentence_is_a_gap() {
        let entry = archetype(
            body("These replace the normal attack roll bonus and AC penalty for charging."),
            &["skilled", "natural-leaper"],
            vec![track()],
        );
        assert_eq!(
            found(entry),
            ["This replaces the normal attack roll bonus and AC penalty for charging"]
        );
    }

    #[test]
    fn a_stored_trade_of_the_feature_states_the_sentence() {
        let entry = archetype(
            body("This alters the hunter's class skills."),
            &["skilled", "natural-leaper"],
            vec![
                trade(TradeOp::Alters, "Class skills", &["skilled"]),
                track(),
            ],
        );
        assert!(found(entry).is_empty());
    }

    #[test]
    fn a_wrong_op_is_a_gap() {
        let entry = archetype(
            body("This alters the hunter's class skills."),
            &["skilled", "natural-leaper"],
            vec![
                trade(TradeOp::Replaces, "Class skills", &["skilled"]),
                track(),
            ],
        );
        assert_eq!(found(entry), ["This alters the hunter's class skills"]);
    }

    #[test]
    fn a_wrong_by_is_drift_not_a_gap() {
        // The sentence names a stored trade, so the drift check reads it: no second report here.
        let entry = archetype(
            body("This alters the hunter's class skills."),
            &["skilled", "natural-leaper"],
            vec![
                trade(TradeOp::Alters, "Class skills", &["natural-leaper"]),
                track(),
            ],
        );
        assert!(found(entry.clone()).is_empty());
        let corpus = corpus_in_format(4, vec![entry.clone()]);
        let stored = corpus.entries.values().next().unwrap();
        assert!(!crate::drift::find_drift(stored).is_empty());
    }

    #[test]
    fn an_anaphoric_phrase_needs_a_trade_of_its_op_by_the_feature() {
        let sentence = "This otherwise works like the standard hunter's skills class feature and replaces that ability.";
        let stored = |op, by: &str| {
            archetype(
                body(sentence),
                &["skilled", "natural-leaper"],
                vec![trade(op, "Skills", &[by]), track()],
            )
        };
        assert!(found(stored(TradeOp::Replaces, "skilled")).is_empty());
        assert_eq!(
            found(stored(TradeOp::Alters, "skilled")),
            ["This replaces that ability"]
        );
        assert_eq!(
            found(stored(TradeOp::Replaces, "natural-leaper")),
            ["This replaces that ability"]
        );
        // A bare pronoun is anaphoric too.
        let bare = |by: &str| {
            archetype(
                body(
                    "This otherwise works like the standard hunter's skills class feature and replaces it.",
                ),
                &["skilled", "natural-leaper"],
                vec![trade(TradeOp::Replaces, "Skills", &[by]), track()],
            )
        };
        assert!(found(bare("skilled")).is_empty());
        assert_eq!(found(bare("natural-leaper")).len(), 1);
    }

    #[test]
    fn a_cross_reference_is_no_gap() {
        for sentence in [
            "This alters hunter's bond and replaces specific hunter abilities as indicated below.",
            "This alters hunter tactics (see the Hunter Tactics section below).",
        ] {
            let entry = archetype(
                body(sentence),
                &["skilled", "natural-leaper"],
                vec![
                    trade(TradeOp::Alters, "Hunter's bond", &["skilled"]),
                    trade(TradeOp::Alters, "Hunter tactics", &["skilled"]),
                    track(),
                ],
            );
            assert!(found(entry).is_empty(), "{sentence}");
        }
        // With no trade stored at all, the cross-referenced phrase still asks for none: the
        // reader drops it (ruling 40), so the check cannot pass by naming it.
        let entry = archetype(
            body("This replaces specific hunter abilities as indicated below."),
            &["skilled", "natural-leaper"],
            vec![track()],
        );
        assert!(found(entry).is_empty());
    }

    #[test]
    fn a_sentence_of_the_opening_text_is_no_gap() {
        // Owner ruling 28: the trade stated before the first feature has no `by`, and this rule
        // reads listed features only.
        let entry = archetype(
            vec![
                text("This alters the hunter's class skills."),
                heading(Some("natural-leaper"), "Natural Leaper (Ex)"),
                text("At 2nd level, a roof runner jumps. This ability replaces track."),
            ],
            &["natural-leaper"],
            vec![track()],
        );
        assert!(found(entry).is_empty());
    }

    #[test]
    fn a_sentence_under_an_unlisted_subheading_belongs_to_the_feature() {
        // Owner ruling 37: what a companion gains sits in `###` sections under the feature.
        let entry = archetype(
            vec![
                heading(Some("shadow-companion"), "Shadow Companion (Su)"),
                text("A shadow caller gains a shadow."),
                Block::Heading {
                    level: 3,
                    anchor: None,
                    content: vec![Inline::text("Following Abilities")],
                },
                text("This replaces the normal shadow trick."),
            ],
            &["shadow-companion"],
            vec![],
        );
        assert_eq!(found(entry), ["This replaces the normal shadow trick"]);
    }

    #[test]
    fn other_kinds_are_not_read() {
        let mut entry = archetype(
            body("This alters the hunter's class skills."),
            &["skilled", "natural-leaper"],
            vec![],
        );
        entry.kind = EntryKind::Rule;
        assert!(found(entry).is_empty());
    }

    fn corpus() -> Corpus {
        let entry = archetype(
            body(
                "This alters her other class features, as detailed in each relevant class feature.",
            ),
            &["skilled", "natural-leaper"],
            vec![track()],
        );
        corpus_in_format(4, vec![entry])
    }

    const ROW: &str = r#"
[[exception]]
entry = "ref/arc-hunter/roof-runner"
feature = "skilled"
sentence = "as detailed in each relevant class feature"
reason = "A cross-reference to the features that follow."
issue = "phynics/cast-iron#540"
"#;

    fn messages(exceptions: &str) -> Vec<String> {
        let rows = parse_exceptions(exceptions).unwrap();
        let mut report = Report::default();
        check_sentences(&corpus(), &rows, &mut report);
        let mut all = report.problems;
        all.extend(report.warnings);
        all
    }

    #[test]
    fn a_listed_sentence_is_no_gap() {
        assert!(messages(ROW).is_empty());
    }

    #[test]
    fn an_unlisted_sentence_names_entry_feature_sentence_and_trade() {
        let all = messages("");
        assert_eq!(all.len(), 1, "{all:?}");
        let message = &all[0];
        assert!(message.contains("`skilled`"), "{message}");
        assert!(
            message.contains("This alters her other class features"),
            "{message}"
        );
        assert!(
            message.contains("alters = \"her other class features"),
            "{message}"
        );
        assert!(message.contains("by = [\"skilled\"]"), "{message}");
    }

    #[test]
    fn a_row_that_excepts_nothing_is_stale() {
        let all = messages(&ROW.replace("skilled", "natural-leaper"));
        assert_eq!(all.len(), 2, "{all:?}");
        assert!(all.iter().any(|m| m.contains("remove the row")), "{all:?}");
    }

    #[test]
    fn a_bad_row_is_an_error() {
        for (bad, why) in [
            (
                ROW.replace("phynics/cast-iron#540", "540"),
                "owner/repo#number",
            ),
            (
                ROW.replace("A cross-reference to the features that follow.", " "),
                "`reason`",
            ),
            (
                ROW.replace("as detailed in each relevant class feature", " "),
                "`sentence`",
            ),
            (format!("{ROW}{ROW}"), "second exception"),
            (format!("{ROW}\nextra = 1"), "unknown field"),
        ] {
            let error = parse_exceptions(&bad).unwrap_err();
            assert!(error.contains(why), "{error}");
        }
    }
}
