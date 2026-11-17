//! `migrate-trades`: lower the trades of a corpus to format 2.
//!
//! Format 1 held four lists per archetype or class option (`replaces`, `alters`, and the heading
//! anchors `replaces_features`, `alters_features`) that did not line up with each other. This
//! step reads every entry, merges the printed names with what the base classes' headings prove,
//! writes one flat `trades` list, and sets `format = 2` in `corpus.toml`:
//!
//! 1. A trade the format-1 reader already resolved by name keeps its feature, unless two base
//!    classes hold the anchor (format 2 would then need a written `class`, and nothing proves
//!    which one).
//! 2. A trade with no feature goes through [`resolve`](crate::trade_match::resolve): exact name,
//!    token set, ordinal levels, one qualifier. Everything else keeps `feature = None` and gets a
//!    row in the review queue.
//! 3. An entry whose normal part prints `**Replaces:**` or `**Alters:**` lines but holds no trade
//!    for that key gets one unresolved trade per printed name, and then goes through step 2.
//! 4. The anchors of an old list that no trade claims are not guessed at. The review queue lists
//!    them next to the unresolved trades of the same key.
//!
//! The step is idempotent. It never changes a trade that already has a feature (apart from the
//! ambiguity in step 1), so a person's fix from the review queue survives a second run, and a
//! second run on a migrated corpus changes no byte. Entries are visited in id order and no output
//! depends on a hash-map order.
//!
//! The step does not run on a half-migrated tree: it writes the entries first and `corpus.toml`
//! last, and a corpus that holds files of both formats does not load. Run it on a clean
//! checkout, and `git checkout .` to retry after an interruption.

use crate::check::{base_class_ids, heading_keys};
use crate::config::{CORPUS_FORMAT, Config, write_config};
use crate::drift::{printed_replaces_alters, printed_replaces_alters_anywhere};
use crate::entry::write_entry;
use crate::load::Corpus;
use crate::trade_match::{BaseHeading, Rule, Verdict, Why, holders, overlaps, resolve};
use cast_iron_schema::{EntryKind, LegacyFeatureLists, RuleEntry, Trade, TradeOp, TypedFields};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// What became of one trade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fate {
    /// It already had a feature, and the feature stays.
    Kept,
    /// A rule proved a feature for it in this run.
    Resolved(Rule),
    /// No feature. The reason goes to the review queue.
    Unresolved(Why),
}

/// An entry's trades after the step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lowered {
    pub trades: Vec<Trade>,
    /// One per trade.
    pub fates: Vec<Fate>,
    /// Trades made from printed lines of the normal part.
    pub filled: usize,
    /// The anchors of an old list that no trade of the same key claims.
    pub unclaimed: LegacyFeatureLists,
}

/// The headings each class entry holds, by class id.
pub type ClassHeadings = BTreeMap<String, Vec<BaseHeading>>;

/// The headings of every class entry of a corpus.
pub fn class_headings(corpus: &Corpus) -> ClassHeadings {
    corpus
        .entries
        .iter()
        .filter(|(_, stored)| stored.entry.kind == EntryKind::Class)
        .map(|(id, stored)| {
            let headings = heading_keys(&stored.entry)
                .into_iter()
                .map(|(key, text)| BaseHeading {
                    class: id.clone(),
                    key,
                    text,
                })
                .collect();
            (id.clone(), headings)
        })
        .collect()
}

/// The words of a class id: `ref/class/unchained-rogue` gives `unchained` and `rogue`.
fn class_words(classes: &[&str]) -> BTreeSet<String> {
    classes
        .iter()
        .flat_map(|class| class.rsplit('/').next().unwrap_or(class).split('-'))
        .map(str::to_string)
        .collect()
}

/// Lowers the trades of `entry`. `None` when the entry is not an archetype or a class option.
/// `classes` are the ids of its base classes.
pub fn lower_entry(
    entry: &RuleEntry,
    classes: &[&str],
    headings: &ClassHeadings,
    format_1: bool,
) -> Option<Lowered> {
    let (existing, legacy) = match &entry.fields {
        TypedFields::Archetype(fields) => (&fields.trades, &fields.legacy_features),
        TypedFields::ClassOption(fields) => (&fields.trades, &fields.legacy_features),
        _ => return None,
    };
    let mut trades: Vec<Trade> = Vec::new();
    let (printed_replaces, printed_alters) = printed_replaces_alters(entry);
    let has = |op: TradeOp| existing.iter().any(|trade| trade.op == op);
    let mut filled = 0;
    if !has(TradeOp::Replaces) {
        filled += printed_replaces.len();
        trades.extend(
            printed_replaces
                .iter()
                .map(|name| Trade::unresolved(TradeOp::Replaces, name.as_str())),
        );
    }
    trades.extend(existing.iter().cloned());
    if !has(TradeOp::Alters) {
        filled += printed_alters.len();
        trades.extend(
            printed_alters
                .iter()
                .map(|name| Trade::unresolved(TradeOp::Alters, name.as_str())),
        );
    }

    let base: Vec<BaseHeading> = classes
        .iter()
        .filter_map(|class| headings.get(*class))
        .flatten()
        .cloned()
        .collect();
    let held = holders(&base);
    let words = class_words(classes);
    let mut fates = Vec::with_capacity(trades.len());
    for trade in &mut trades {
        let fate = match &trade.feature {
            // In format 1 every feature comes from the reader, which links a name to an anchor of
            // its list by slug alone. That proves less than the resolver does (`Trap sense +3`
            // and the anchor `trap-sense-3`, which may be a collision suffix), so the resolver
            // judges it again: the same feature stays, a better proof replaces it, and no proof
            // leaves the trade unresolved. A feature written in format 2 is never judged.
            Some(feature) if format_1 && feature.class.is_none() => {
                if classes.is_empty() {
                    trade.feature = None;
                    Fate::Unresolved(Why::NoBaseClass)
                } else {
                    match resolve(&trade.printed, &base, &words) {
                        Verdict::Resolved(proved, _) if &proved == feature => Fate::Kept,
                        Verdict::Resolved(proved, rule) => {
                            trade.feature = Some(proved);
                            Fate::Resolved(rule)
                        }
                        Verdict::Unresolved(why) => {
                            trade.feature = None;
                            Fate::Unresolved(why)
                        }
                    }
                }
            }
            Some(feature) => {
                let several = feature.class.is_none()
                    && held
                        .get(feature.anchor.as_str())
                        .is_some_and(|holding| holding.len() > 1);
                if several {
                    let classes = held[feature.anchor.as_str()]
                        .iter()
                        .map(|class| class.to_string())
                        .collect();
                    trade.feature = None;
                    Fate::Unresolved(Why::AmbiguousClass(classes))
                } else {
                    Fate::Kept
                }
            }
            None if classes.is_empty() => Fate::Unresolved(Why::NoBaseClass),
            None => match resolve(&trade.printed, &base, &words) {
                Verdict::Resolved(feature, rule) => {
                    trade.feature = Some(feature);
                    Fate::Resolved(rule)
                }
                Verdict::Unresolved(why) => Fate::Unresolved(why),
            },
        };
        fates.push(fate);
    }

    // Two trades of one key that name the same feature at the same levels and power cannot both
    // be the proven reading (a book that prints `Smite evil` twice, or two overlapping lists of
    // bonus feats). The later one stays unresolved, so the entry passes the overlap check. A
    // feature that a person wrote in format 2 is never touched; the check reports it.
    let may_demote = |fate: &Fate| match fate {
        Fate::Resolved(_) => true,
        Fate::Kept => format_1,
        Fate::Unresolved(_) => false,
    };
    for later in 1..trades.len() {
        for earlier in 0..later {
            let (Some(a), Some(b)) = (&trades[earlier].feature, &trades[later].feature) else {
                continue;
            };
            if trades[earlier].op != trades[later].op || !overlaps(a, b) {
                continue;
            }
            let anchor = a.anchor.clone();
            let victim = if may_demote(&fates[later]) {
                later
            } else if may_demote(&fates[earlier]) {
                earlier
            } else {
                continue;
            };
            trades[victim].feature = None;
            fates[victim] = Fate::Unresolved(Why::SameFeatureTwice(anchor));
            if victim == later {
                break;
            }
        }
    }

    let unclaimed = |op: TradeOp, old: &[String]| -> Vec<String> {
        let claimed: BTreeSet<&str> = trades
            .iter()
            .filter(|trade| trade.op == op)
            .filter_map(|trade| trade.feature.as_ref())
            .map(|feature| feature.anchor.as_str())
            .collect();
        old.iter()
            .filter(|anchor| !claimed.contains(anchor.as_str()))
            .cloned()
            .collect()
    };
    let unclaimed = LegacyFeatureLists {
        replaces: unclaimed(TradeOp::Replaces, &legacy.replaces),
        alters: unclaimed(TradeOp::Alters, &legacy.alters),
    };
    Some(Lowered {
        trades,
        fates,
        filled,
        unclaimed,
    })
}

/// One row of the review queue: a trade that has no feature, or a printed line that has no trade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRow {
    pub id: String,
    pub kind: &'static str,
    /// 1-based position among the entry's trades; 0 for a line that has no trade.
    pub index: usize,
    pub op: &'static str,
    pub printed: String,
    pub reason: &'static str,
    /// Headings the reason names, as `class#anchor` or class ids.
    pub candidates: Vec<String>,
    /// Anchors of an old list that no trade claims, for the same key.
    pub legacy: Vec<String>,
}

/// The counts and the review queue of one run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TradesReport {
    /// Archetypes and class options that hold trades after the step.
    pub entries: usize,
    /// Entry files written (changed, or converted from format 1).
    pub rewritten: usize,
    pub trades: usize,
    /// Trades that had a feature before the step and keep it.
    pub kept: usize,
    /// Trades resolved by this run, by rule.
    pub resolved: BTreeMap<Rule, usize>,
    pub unresolved: usize,
    /// Unresolved trades by reason.
    pub reasons: BTreeMap<&'static str, usize>,
    pub archetypes: usize,
    /// Archetypes with trades whose every trade has a feature.
    pub archetypes_resolved: usize,
    pub class_options: usize,
    pub class_options_resolved: usize,
    /// Entries, and trades, made from printed lines.
    pub filled_entries: usize,
    pub filled_trades: usize,
    /// Class options that print lines only below the normal part (variant subdomains and the
    /// like). They have no trade, because a trade there would disagree with the drift check.
    pub variant_only: usize,
    pub review: Vec<ReviewRow>,
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

impl TradesReport {
    /// The review queue as TSV: a header, then one row per line, in entry id and trade order.
    pub fn review_tsv(&self) -> String {
        let mut out = String::from(
            "id\tkind\ttrade\top\tprinted\treason\tcandidates\tunclaimed_old_anchors\n",
        );
        let clean = |text: &str| text.replace(['\t', '\n', '\r'], " ");
        for row in &self.review {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                row.id,
                row.kind,
                row.index,
                row.op,
                clean(&row.printed),
                row.reason,
                clean(&row.candidates.join(" ")),
                clean(&row.legacy.join(" ")),
            ));
        }
        out
    }

    /// A summary for the person who ran the step.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        let mut line = |text: String| {
            out.push_str(&text);
            out.push('\n');
        };
        line(format!(
            "trades: {} in {} entries ({} files written)",
            self.trades, self.entries, self.rewritten
        ));
        let newly: usize = self.resolved.values().sum();
        line(format!(
            "resolved: {} ({} kept, {} new: {})",
            self.kept + newly,
            self.kept,
            newly,
            Rule::ALL
                .iter()
                .map(|rule| format!(
                    "{} {}",
                    rule.slug(),
                    self.resolved.get(rule).copied().unwrap_or(0)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        line(format!(
            "unresolved: {} ({})",
            self.unresolved,
            self.reasons
                .iter()
                .map(|(reason, count)| format!("{reason} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        line(format!(
            "archetypes fully resolved: {} of {}",
            self.archetypes_resolved, self.archetypes
        ));
        line(format!(
            "class options fully resolved: {} of {}",
            self.class_options_resolved, self.class_options
        ));
        line(format!(
            "filled from printed lines: {} trades in {} entries; {} class options print lines only below the normal part",
            self.filled_trades, self.filled_entries, self.variant_only
        ));
        line(format!("review queue: {} rows", self.review.len()));
        out
    }
}

/// What the step would do to a corpus, without touching the disk. `texts` holds each entry file
/// whose text changes, by relative path.
pub struct Plan {
    pub report: TradesReport,
    pub texts: BTreeMap<std::path::PathBuf, String>,
}

/// Works out the migration of a loaded corpus.
pub fn plan(corpus: &Corpus) -> Plan {
    let headings = class_headings(corpus);
    let target = Config {
        format: CORPUS_FORMAT,
        license_refs: corpus.config.license_refs.clone(),
    };
    let mut report = TradesReport::default();
    let mut texts = BTreeMap::new();
    for (id, stored) in &corpus.entries {
        let classes = base_class_ids(corpus, &stored.entry);
        let Some(lowered) = lower_entry(
            &stored.entry,
            &classes,
            &headings,
            corpus.config.format == 1,
        ) else {
            continue;
        };
        let kind = stored.entry.kind.slug();
        let is_archetype = stored.entry.kind == EntryKind::Archetype;
        let mut entry = stored.entry.clone();
        match &mut entry.fields {
            TypedFields::Archetype(fields) => {
                fields.trades = lowered.trades.clone();
                fields.legacy_features = LegacyFeatureLists::default();
            }
            TypedFields::ClassOption(fields) => {
                fields.trades = lowered.trades.clone();
                fields.legacy_features = LegacyFeatureLists::default();
            }
            _ => {}
        }
        let text = write_entry(&entry, &target);
        if text != stored.text {
            report.rewritten += 1;
            texts.insert(stored.path.clone(), text);
        }
        if lowered.trades.is_empty() {
            // An old anchor list with no printed names has no trade to carry it. The file drops
            // it, so the queue keeps it.
            for (op, anchors) in [
                ("replaces", &lowered.unclaimed.replaces),
                ("alters", &lowered.unclaimed.alters),
            ] {
                if !anchors.is_empty() {
                    report.review.push(ReviewRow {
                        id: id.clone(),
                        kind,
                        index: 0,
                        op,
                        printed: String::new(),
                        reason: "old-anchors-only",
                        candidates: Vec::new(),
                        legacy: anchors.clone(),
                    });
                }
            }
            // A class option may print lines only in a subsection that the drift check does not
            // read. Typing them here would make the entry disagree with its normal part, so they
            // wait in the review queue.
            if !is_archetype {
                let (replaces, alters) = printed_replaces_alters_anywhere(&stored.entry);
                if !replaces.is_empty() || !alters.is_empty() {
                    report.variant_only += 1;
                    for (op, names) in [("replaces", replaces), ("alters", alters)] {
                        for name in names {
                            report.review.push(ReviewRow {
                                id: id.clone(),
                                kind,
                                index: 0,
                                op,
                                printed: name,
                                reason: "variant-only",
                                candidates: Vec::new(),
                                legacy: Vec::new(),
                            });
                        }
                    }
                }
            }
            continue;
        }
        report.entries += 1;
        report.trades += lowered.trades.len();
        if lowered.filled > 0 {
            report.filled_entries += 1;
            report.filled_trades += lowered.filled;
        }
        let mut all_resolved = true;
        for (index, (trade, fate)) in lowered.trades.iter().zip(&lowered.fates).enumerate() {
            match fate {
                Fate::Kept => report.kept += 1,
                Fate::Resolved(rule) => *report.resolved.entry(*rule).or_default() += 1,
                Fate::Unresolved(why) => {
                    all_resolved = false;
                    report.unresolved += 1;
                    *report.reasons.entry(why.slug()).or_default() += 1;
                    let unclaimed = match trade.op {
                        TradeOp::Replaces => &lowered.unclaimed.replaces,
                        TradeOp::Alters => &lowered.unclaimed.alters,
                    };
                    report.review.push(ReviewRow {
                        id: id.clone(),
                        kind,
                        index: index + 1,
                        op: trade.op.slug(),
                        printed: trade.printed.clone(),
                        reason: why.slug(),
                        candidates: sorted_unique(why.candidates().to_vec()),
                        legacy: unclaimed.clone(),
                    });
                }
            }
        }
        if is_archetype {
            report.archetypes += 1;
            report.archetypes_resolved += usize::from(all_resolved);
        } else {
            report.class_options += 1;
            report.class_options_resolved += usize::from(all_resolved);
        }
    }
    Plan { report, texts }
}

/// Migrates the corpus at `dir` to format 2, in place. Returns the counts and the review queue.
pub fn migrate_trades(dir: &Path) -> Result<TradesReport, String> {
    let corpus = Corpus::load(dir).map_err(|errors| {
        for error in &errors {
            eprintln!("{error}");
        }
        format!("could not read the corpus: {} error(s)", errors.len())
    })?;
    let Plan { report, texts } = plan(&corpus);
    for (path, text) in &texts {
        let full = dir.join(path);
        fs::write(&full, text).map_err(|e| format!("cannot write {}: {e}", full.display()))?;
    }
    if corpus.config.format != CORPUS_FORMAT {
        let config = Config {
            format: CORPUS_FORMAT,
            license_refs: corpus.config.license_refs.clone(),
        };
        let path = dir.join("corpus.toml");
        fs::write(&path, write_config(&config))
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{corpus_in_format, heading, paragraph, rule};
    use cast_iron_schema::{
        ArchetypeFields, ClassOptionFields, EntryId, Inline, Relation, RelationKind,
    };

    fn typed(mut entry: RuleEntry, kind: EntryKind, fields: TypedFields) -> RuleEntry {
        entry.kind = kind;
        entry.fields = fields;
        entry
    }

    fn class(id: &str, headings: &[(&str, &str)]) -> RuleEntry {
        typed(
            rule(
                id,
                headings
                    .iter()
                    .map(|(anchor, text)| heading(Some(anchor), text))
                    .collect(),
            ),
            EntryKind::Class,
            TypedFields::Class(Default::default()),
        )
    }

    fn printed(label: &str, text: &str) -> cast_iron_schema::Block {
        paragraph(vec![
            Inline::Strong {
                c: vec![Inline::text(format!("{label}:"))],
            },
            Inline::text(format!(" {text}")),
        ])
    }

    fn archetype(
        id: &str,
        classes: &[&str],
        trades: Vec<Trade>,
        body_lines: &[(&str, &str)],
    ) -> RuleEntry {
        let mut entry = typed(
            rule(
                id,
                body_lines
                    .iter()
                    .map(|(label, text)| printed(label, text))
                    .collect(),
            ),
            EntryKind::Archetype,
            TypedFields::Archetype(ArchetypeFields {
                trades,
                ..Default::default()
            }),
        );
        entry.relations = classes
            .iter()
            .map(|class| Relation {
                kind: RelationKind::Archetype,
                target: EntryId::parse(*class).unwrap(),
                label: None,
            })
            .collect();
        entry
    }

    fn fighter() -> RuleEntry {
        class(
            "ref/class/fighter",
            &[
                ("bravery", "Bravery (Ex)"),
                ("bonus-feats", "Bonus Feats"),
                ("weapon-training", "Weapon Training (Ex)"),
                ("evasion", "Evasion"),
            ],
        )
    }

    fn lower(entry: RuleEntry, classes: Vec<RuleEntry>) -> Lowered {
        let mut all = classes;
        all.push(entry.clone());
        let corpus = corpus_in_format(2, all);
        let ids = base_class_ids(&corpus, &entry)
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        lower_entry(&entry, &ids, &class_headings(&corpus), true).unwrap()
    }

    /// [`lower`] for a corpus that is already format 2: features are the author's.
    fn lower_in_format_2(entry: RuleEntry, classes: Vec<RuleEntry>) -> Lowered {
        let mut all = classes;
        all.push(entry.clone());
        let corpus = corpus_in_format(2, all);
        let ids = base_class_ids(&corpus, &entry)
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        lower_entry(&entry, &ids, &class_headings(&corpus), false).unwrap()
    }

    fn anchors(lowered: &Lowered) -> Vec<Option<&str>> {
        lowered
            .trades
            .iter()
            .map(|trade| trade.feature.as_ref().map(|f| f.anchor.as_str()))
            .collect()
    }

    #[test]
    fn exact_levels_and_qualifier_names_resolve_and_the_rest_stay_unresolved() {
        let trades = vec![
            Trade::unresolved(TradeOp::Replaces, "Bravery (Ex)"),
            Trade::unresolved(TradeOp::Replaces, "The bonus feat gained at 6th level"),
            Trade::unresolved(TradeOp::Replaces, "Weapon training 2"),
            Trade::unresolved(TradeOp::Alters, "Improved evasion"),
        ];
        let lowered = lower(
            archetype("ref/arc-fighter/a", &["ref/class/fighter"], trades, &[]),
            vec![fighter()],
        );
        assert_eq!(
            anchors(&lowered),
            [
                Some("bravery"),
                Some("bonus-feats"),
                Some("weapon-training"),
                None
            ]
        );
        assert_eq!(lowered.trades[1].feature.as_ref().unwrap().levels, [6]);
        assert_eq!(
            lowered.trades[2].feature.as_ref().unwrap().power.as_deref(),
            Some("2")
        );
        assert_eq!(
            lowered.fates,
            [
                Fate::Resolved(Rule::Exact),
                Fate::Resolved(Rule::Levels),
                Fate::Resolved(Rule::Qualifier),
                Fate::Unresolved(Why::NoHeading)
            ]
        );
    }

    #[test]
    fn a_resolved_trade_is_never_changed_by_a_run() {
        // A person's fix: `Bravery` resolved to `evasion`. The step keeps it.
        let mut fixed = Trade::unresolved(TradeOp::Replaces, "Bravery");
        fixed.feature = Some(cast_iron_schema::FeatureRef {
            anchor: "evasion".into(),
            ..Default::default()
        });
        let lowered = lower_in_format_2(
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                vec![fixed.clone()],
                &[],
            ),
            vec![fighter()],
        );
        assert_eq!(lowered.trades, [fixed]);
        assert_eq!(lowered.fates, [Fate::Kept]);
    }

    #[test]
    fn an_anchor_two_base_classes_hold_is_not_resolved_by_name_or_kept() {
        let investigator = class("ref/class/investigator", &[("alchemy", "Alchemy (Su)")]);
        let alchemist = class("ref/class/alchemist", &[("alchemy", "Alchemy (Su)")]);
        let mut named = Trade::unresolved(TradeOp::Alters, "Alchemy");
        let lowered = lower(
            archetype(
                "ref/arc-alchemist/mc",
                &["ref/class/alchemist", "ref/class/investigator"],
                vec![named.clone()],
                &[],
            ),
            vec![alchemist.clone(), investigator.clone()],
        );
        assert_eq!(anchors(&lowered), [None]);
        assert_eq!(
            lowered.fates,
            [Fate::Unresolved(Why::AmbiguousClass(vec![
                "ref/class/alchemist".into(),
                "ref/class/investigator".into()
            ]))]
        );
        // A feature the format 1 reader resolved by name is dropped for the same reason.
        named.feature = Some(cast_iron_schema::FeatureRef {
            anchor: "alchemy".into(),
            ..Default::default()
        });
        let lowered = lower(
            archetype(
                "ref/arc-alchemist/mc",
                &["ref/class/alchemist", "ref/class/investigator"],
                vec![named.clone()],
                &[],
            ),
            vec![alchemist.clone(), investigator.clone()],
        );
        assert_eq!(anchors(&lowered), [None]);
        // With one base class the anchor is unambiguous and resolves.
        let lowered = lower(
            archetype(
                "ref/arc-alchemist/one",
                &["ref/class/alchemist"],
                vec![Trade::unresolved(TradeOp::Alters, "Alchemy")],
                &[],
            ),
            vec![alchemist, investigator],
        );
        assert_eq!(anchors(&lowered), [Some("alchemy")]);
    }

    #[test]
    fn a_second_trade_of_one_key_on_the_same_feature_and_levels_stays_unresolved() {
        // The book prints `Bravery` twice, and two lists of bonus feats overlap at level 6.
        let lowered = lower(
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                vec![
                    Trade::unresolved(TradeOp::Replaces, "Bravery"),
                    Trade::unresolved(TradeOp::Replaces, "Bravery (Ex)"),
                    Trade::unresolved(TradeOp::Replaces, "Bonus feats at 2nd and 6th levels"),
                    Trade::unresolved(TradeOp::Replaces, "Bonus feats at 6th and 10th levels"),
                    Trade::unresolved(TradeOp::Replaces, "Bonus feats at 12th level"),
                ],
                &[],
            ),
            vec![fighter()],
        );
        assert_eq!(
            anchors(&lowered),
            [
                Some("bravery"),
                None,
                Some("bonus-feats"),
                None,
                Some("bonus-feats")
            ]
        );
        assert_eq!(
            lowered.fates[1],
            Fate::Unresolved(Why::SameFeatureTwice("bravery".into()))
        );
        assert_eq!(
            lowered.fates[3].clone(),
            Fate::Unresolved(Why::SameFeatureTwice("bonus-feats".into()))
        );
    }

    #[test]
    fn replacing_one_instance_and_altering_the_feature_both_stay_resolved() {
        let lowered = lower(
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                vec![
                    Trade::unresolved(TradeOp::Replaces, "Bonus feat gained at 6th level"),
                    Trade::unresolved(TradeOp::Alters, "Bonus feats"),
                ],
                &[],
            ),
            vec![fighter()],
        );
        assert_eq!(
            anchors(&lowered),
            [Some("bonus-feats"), Some("bonus-feats")]
        );
    }

    #[test]
    fn a_format_2_feature_that_a_person_wrote_is_never_demoted() {
        let kept = |printed: &str| {
            let mut trade = Trade::unresolved(TradeOp::Replaces, printed);
            trade.feature = Some(cast_iron_schema::FeatureRef {
                anchor: "bravery".into(),
                ..Default::default()
            });
            trade
        };
        let entry = archetype(
            "ref/arc-fighter/a",
            &["ref/class/fighter"],
            vec![kept("Bravery"), kept("Bravery (Ex)")],
            &[],
        );
        let corpus = corpus_in_format(2, vec![fighter(), entry.clone()]);
        let headings = class_headings(&corpus);
        let in_format_2 = lower_entry(&entry, &["ref/class/fighter"], &headings, false).unwrap();
        assert_eq!(in_format_2.fates, [Fate::Kept, Fate::Kept]);
        // In format 1 the reader's resolution is the migration's to settle.
        let in_format_1 = lower_entry(&entry, &["ref/class/fighter"], &headings, true).unwrap();
        assert_eq!(anchors(&in_format_1), [Some("bravery"), None]);
        // A new resolution gives way to a kept one that is later in the list.
        let entry = archetype(
            "ref/arc-fighter/a",
            &["ref/class/fighter"],
            vec![
                Trade::unresolved(TradeOp::Replaces, "Bravery (Ex)"),
                kept("Bravery"),
            ],
            &[],
        );
        let lowered = lower_entry(&entry, &["ref/class/fighter"], &headings, false).unwrap();
        assert_eq!(anchors(&lowered), [None, Some("bravery")]);
    }

    #[test]
    fn a_prefix_name_stays_unresolved_and_keeps_its_neighbour_resolved() {
        let lowered = lower(
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                vec![
                    Trade::unresolved(TradeOp::Replaces, "Improved evasion"),
                    Trade::unresolved(TradeOp::Replaces, "Evasion"),
                    Trade::unresolved(TradeOp::Replaces, "Bravery, 2nd favored enemy"),
                ],
                &[],
            ),
            vec![fighter()],
        );
        assert_eq!(anchors(&lowered), [None, Some("evasion"), None]);
    }

    #[test]
    fn printed_lines_with_no_trade_become_unresolved_trades_and_then_resolve() {
        let lowered = lower(
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                vec![],
                &[("Replaces", "Bravery, Spellcasting"), ("Alters", "Evasion")],
            ),
            vec![fighter()],
        );
        assert_eq!(lowered.filled, 3);
        assert_eq!(
            lowered
                .trades
                .iter()
                .map(|trade| (trade.op.slug(), trade.printed.as_str()))
                .collect::<Vec<_>>(),
            [
                ("replaces", "Bravery"),
                ("replaces", "Spellcasting"),
                ("alters", "Evasion")
            ]
        );
        assert_eq!(anchors(&lowered), [Some("bravery"), None, Some("evasion")]);
    }

    #[test]
    fn a_key_with_trades_is_not_filled_from_its_printed_line() {
        let lowered = lower(
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                vec![Trade::unresolved(TradeOp::Replaces, "Bravery")],
                &[("Replaces", "Evasion")],
            ),
            vec![fighter()],
        );
        assert_eq!(lowered.filled, 0);
        assert_eq!(lowered.trades.len(), 1);
    }

    #[test]
    fn a_trade_with_no_base_class_is_unresolved_for_that_reason() {
        let lowered = lower(
            archetype(
                "ref/arc-fighter/a",
                &[],
                vec![Trade::unresolved(TradeOp::Replaces, "Bravery")],
                &[],
            ),
            vec![],
        );
        assert_eq!(lowered.fates, [Fate::Unresolved(Why::NoBaseClass)]);
    }

    #[test]
    fn old_anchors_that_no_trade_claims_go_to_the_review_queue() {
        let mut entry = archetype(
            "ref/arc-fighter/a",
            &["ref/class/fighter"],
            vec![Trade::unresolved(
                TradeOp::Replaces,
                "Bravery, 2nd favored enemy",
            )],
            &[],
        );
        if let TypedFields::Archetype(fields) = &mut entry.fields {
            fields.legacy_features.replaces = vec!["bravery".into()];
        }
        let lowered = lower(entry, vec![fighter()]);
        assert_eq!(lowered.unclaimed.replaces, ["bravery"]);
    }

    #[test]
    fn a_class_option_prints_lines_below_the_normal_part_for_the_queue() {
        let mut option = typed(
            rule("ref/domain/air", vec![]),
            EntryKind::ClassOption,
            TypedFields::ClassOption(ClassOptionFields {
                group: "domain".into(),
                ..Default::default()
            }),
        );
        option.body = vec![
            paragraph(vec![Inline::text("Granted powers.")]),
            cast_iron_schema::Block::Heading {
                level: 3,
                anchor: None,
                content: vec![Inline::text("Cloud Subdomain")],
            },
            printed("Sources", "Advanced Player's Guide pg 88"),
            printed(
                "Replaces",
                "The electricity resistance power of the Air domain",
            ),
        ];
        let mut c = corpus_in_format(1, vec![option]);
        c.option_groups.groups.insert(
            "domain".into(),
            crate::OptionGroupDef {
                label: "Domains".into(),
                singular: "Domain".into(),
                classes: vec![],
            },
        );
        let planned = plan(&c);
        assert_eq!(planned.report.variant_only, 1);
        assert_eq!(planned.report.filled_trades, 0);
        assert_eq!(planned.report.review[0].reason, "variant-only");
        assert!(planned.texts.is_empty());
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cast-iron-migrate-trades-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Writes `source` to `dir` the way a corpus repository holds it.
    fn write_corpus(dir: &Path, source: &Corpus) {
        use crate::entry::entry_path;
        use crate::files::{write_books, write_exclusions, write_legal};
        for stored in source.entries.values() {
            let path = dir.join(entry_path(&stored.entry.id));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, write_entry(&stored.entry, &source.config)).unwrap();
        }
        fs::write(dir.join("corpus.toml"), write_config(&source.config)).unwrap();
        fs::write(
            dir.join("publications.toml"),
            "[[publication]]\nid = \"core-rulebook\"\ntitle = \"Core Rulebook\"\ncopyright = \"Core Rulebook, (c) Paizo Inc.\"\n",
        )
        .unwrap();
        fs::write(dir.join("books.toml"), write_books(&source.books)).unwrap();
        fs::write(dir.join("legal.toml"), write_legal(&source.legal)).unwrap();
        fs::write(
            dir.join("exclusions.toml"),
            write_exclusions(&source.exclusions),
        )
        .unwrap();
    }

    fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut files = Vec::new();
        crate::load::list_files(dir, &mut files).unwrap();
        files
            .into_iter()
            .map(|path| {
                (
                    path.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn a_format_1_corpus_migrates_to_format_2_and_a_second_run_changes_nothing() {
        let mut bravery = Trade::unresolved(TradeOp::Replaces, "Bravery");
        bravery.feature = Some(cast_iron_schema::FeatureRef {
            anchor: "bravery".into(),
            ..Default::default()
        });
        let trades = vec![
            bravery,
            Trade::unresolved(TradeOp::Replaces, "The bonus feat gained at 6th level"),
            Trade::unresolved(TradeOp::Alters, "Improved evasion"),
        ];
        let entries = vec![
            fighter(),
            archetype(
                "ref/arc-fighter/a",
                &["ref/class/fighter"],
                trades,
                &[
                    ("Replaces", "Bravery, The bonus feat gained at 6th level"),
                    ("Alters", "Improved evasion"),
                ],
            ),
        ];
        let source = corpus_in_format(1, entries);
        let dir = temp_dir("twice");
        write_corpus(&dir, &source);
        let before = tree(&dir);
        assert!(
            String::from_utf8_lossy(&before["entries/arc-fighter/a.md"])
                .contains("replaces = [\"Bravery\", \"The bonus feat gained at 6th level\"]")
        );

        let first = migrate_trades(&dir).unwrap();
        assert_eq!(first.rewritten, 1);
        assert_eq!(first.trades, 3);
        assert_eq!(first.kept, 1);
        assert_eq!(first.resolved.get(&Rule::Levels), Some(&1));
        assert_eq!(first.unresolved, 1);
        let after = tree(&dir);
        assert_eq!(
            String::from_utf8_lossy(&after["corpus.toml"]),
            "format = 2\n\n[defaults]\nlicense_refs = [\"ogl-1.0a\", \"legal#community-use\"]\n"
        );
        let text = String::from_utf8_lossy(&after["entries/arc-fighter/a.md"]).to_string();
        assert!(
            text.contains(
                "trades = [\n  { replaces = \"Bravery\", feature = \"bravery\" },\n  { replaces = \"The bonus feat gained at 6th level\", feature = \"bonus-feats\", levels = [6] },\n  { alters = \"Improved evasion\" },\n]"
            ),
            "{text}"
        );

        // The migrated corpus loads, checks, and is canonical.
        let loaded = Corpus::load(&dir).unwrap();
        assert_eq!(loaded.config.format, 2);
        let report = crate::check(&loaded);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(
            crate::format_corpus(&dir, false)
                .unwrap()
                .changes
                .is_empty()
        );

        // A second run changes no byte and finds the same trades.
        let second = migrate_trades(&dir).unwrap();
        assert_eq!(tree(&dir), after);
        assert_eq!(second.rewritten, 0);
        assert_eq!(second.kept, 2);
        assert!(second.resolved.is_empty());
        assert_eq!(second.unresolved, 1);
        assert_eq!(first.review.len(), second.review.len());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_anchor_held_under_another_text_stays_unresolved_and_a_second_run_keeps_it_so() {
        // Two classes hold `alchemy`, with different heading texts. `Alchemies` token-matches
        // only the first text, and the migration must not write `alchemy` without a `class`.
        let alchemist = class("ref/class/alchemist", &[("alchemy", "Alchemy (Su)")]);
        let investigator = class(
            "ref/class/investigator",
            &[("alchemy", "Alchemy and Extracts")],
        );
        let entry = archetype(
            "ref/arc-alchemist/mc",
            &["ref/class/alchemist", "ref/class/investigator"],
            vec![
                Trade::unresolved(TradeOp::Alters, "Alchemies"),
                Trade::unresolved(TradeOp::Alters, "Extracts"),
            ],
            &[("Alters", "Alchemies, Extracts")],
        );
        let source = corpus_in_format(1, vec![alchemist, investigator, entry]);
        let dir = temp_dir("cross-class");
        write_corpus(&dir, &source);
        let first = migrate_trades(&dir).unwrap();
        assert_eq!(first.resolved.values().sum::<usize>(), 0);
        assert_eq!(first.unresolved, 2);
        assert_eq!(first.reasons.get("ambiguous-class"), Some(&1));
        let after = tree(&dir);
        let text = String::from_utf8_lossy(&after["entries/arc-alchemist/mc.md"]).to_string();
        assert!(!text.contains("feature ="), "{text}");
        // The migrated corpus checks clean, and a second run changes nothing and drops nothing.
        let loaded = Corpus::load(&dir).unwrap();
        let report = crate::check(&loaded);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let second = migrate_trades(&dir).unwrap();
        assert_eq!(second.rewritten, 0);
        assert_eq!(second.kept, 0);
        assert_eq!(second.unresolved, 2);
        assert_eq!(tree(&dir), after);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A format 1 corpus whose archetype names `Trap sense +3` with the anchor `trap-sense-3`,
    /// as the reader links them by slug. Returns the migrated entry text and the report.
    fn trap_sense_corpus(name: &str, headings: &[(&str, &str)]) -> (String, TradesReport) {
        let rogue = class("ref/class/rogue", headings);
        let mut trade = Trade::unresolved(TradeOp::Replaces, "Trap sense +3");
        trade.feature = Some(cast_iron_schema::FeatureRef {
            anchor: "trap-sense-3".into(),
            ..Default::default()
        });
        let entry = archetype(
            "ref/arc-rogue/a",
            &["ref/class/rogue"],
            vec![trade],
            &[("Replaces", "Trap sense +3")],
        );
        let dir = temp_dir(name);
        write_corpus(&dir, &corpus_in_format(1, vec![rogue, entry]));
        let before = fs::read_to_string(dir.join("entries/arc-rogue/a.md")).unwrap();
        assert!(
            before.contains("replaces = [\"Trap sense +3\"]")
                && before.contains("replaces_features = [\"trap-sense-3\"]"),
            "{before}"
        );
        // The reader links them.
        let loaded = Corpus::load(&dir).unwrap();
        let read = loaded.entries["ref/arc-rogue/a"].entry.fields.trades();
        assert_eq!(read[0].feature.as_ref().unwrap().anchor, "trap-sense-3");
        let report = migrate_trades(&dir).unwrap();
        let after = tree(&dir);
        let text = String::from_utf8_lossy(&after["entries/arc-rogue/a.md"]).to_string();
        // A second run changes nothing.
        let second = migrate_trades(&dir).unwrap();
        assert_eq!(tree(&dir), after);
        assert_eq!(second.rewritten, 0);
        assert_eq!(second.unresolved, report.unresolved);
        let _ = fs::remove_dir_all(&dir);
        (text, report)
    }

    #[test]
    fn a_format_1_numeric_anchor_that_may_be_a_suffix_is_judged_again() {
        // Two `Trap Sense` headings: the second got `trap-sense-3`. The name is the +3 step, which
        // neither heading proves, so the reader's link does not survive.
        let (text, report) = trap_sense_corpus(
            "trap-twin",
            &[
                ("trap-sense", "Trap Sense (Ex)"),
                ("trap-sense-3", "Trap Sense (Ex)"),
            ],
        );
        assert!(!text.contains("feature ="), "{text}");
        assert_eq!(report.kept, 0);
        assert_eq!(report.unresolved, 1);
        assert_eq!(report.reasons.get("ambiguous-heading"), Some(&1));
    }

    #[test]
    fn a_format_1_numeric_anchor_with_one_heading_becomes_a_qualifier() {
        let (text, report) = trap_sense_corpus("trap-one", &[("trap-sense-3", "Trap Sense (Ex)")]);
        assert!(
            text.contains(
                "{ replaces = \"Trap sense +3\", feature = \"trap-sense-3\", power = \"+3\" }"
            ),
            "{text}"
        );
        assert_eq!(report.kept, 0);
        assert_eq!(report.resolved.get(&Rule::Qualifier), Some(&1));
    }

    #[test]
    fn the_generic_migrate_command_runs_the_same_step() {
        let source = corpus_in_format(
            1,
            vec![
                fighter(),
                archetype(
                    "ref/arc-fighter/a",
                    &["ref/class/fighter"],
                    vec![Trade::unresolved(TradeOp::Replaces, "Bravery")],
                    &[("Replaces", "Bravery")],
                ),
            ],
        );
        let dir = temp_dir("generic");
        write_corpus(&dir, &source);
        let message = crate::migrate(&dir).unwrap();
        assert!(message.contains("from format 1 to 2"), "{message}");
        let loaded = Corpus::load(&dir).unwrap();
        assert_eq!(loaded.config.format, 2);
        let trades = loaded.entries["ref/arc-fighter/a"].entry.fields.trades();
        assert_eq!(trades[0].feature.as_ref().unwrap().anchor, "bravery");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_review_tsv_has_a_header_and_one_clean_row_per_unresolved_trade() {
        let report = TradesReport {
            review: vec![ReviewRow {
                id: "ref/arc-fighter/a".into(),
                kind: "archetype",
                index: 2,
                op: "replaces",
                printed: "Bravery,\tsecond".into(),
                reason: "no-heading",
                candidates: vec!["x#a".into(), "x#b".into()],
                legacy: vec!["bravery".into()],
            }],
            ..Default::default()
        };
        assert_eq!(
            report.review_tsv(),
            "id\tkind\ttrade\top\tprinted\treason\tcandidates\tunclaimed_old_anchors\nref/arc-fighter/a\tarchetype\t2\treplaces\tBravery, second\tno-heading\tx#a x#b\tbravery\n"
        );
    }
}
