//! The acceptance fixtures of `docs/decisions/2026-11-20-class-option-power-layout.md`, one test
//! per row of its "Fixtures" table, against the shared classifier and validator.
//!
//! Each fixture is the minimal block structure of the real corpus text at the pinned corpus
//! (`phynics/cast-iron-data` v0.1.32, `c265325e`): the same rows in the same order, with the effect text
//! shortened. The table's `#` column is the 1-based `occurrence` of a candidate.
//!
//! This PR holds the classifier and the validator, not the rewrite. For a `rewritten` fixture the
//! test asserts that the original run is a candidate that fails validation with the reasons that
//! the rewrite resolves, and that the run in the rewrite's target form passes. For a `queued`
//! fixture it asserts the candidate and the reasons that validation gives. A reason that only the
//! rewrite knows (a capital in an upgrade, a seeded print item) is noted where it applies.
//!
//! Three assertions of the fixtures table are the rewrite's and belong to the corpus repository
//! (phynics/cast-iron-data#88), not to this validator: Foretell's seed (a print item that no
//! block structure shows), Forewarned's lowercase rule (the first word of an upgrade on the
//! lowercase list) and S3's second pass (rerunning the rewrite on its own output changes
//! nothing). The tests for them show only what the validator can: the shape of the run, that a
//! capital is no validation rule, and that a canonical run passes.

use cast_iron_schema::power::{
    ActionGroupProblem, ActionMapping, Candidate, EFFECT_LABELS, PowerProblem, action_effect_group,
    candidates, map_action_value, one_power_page, power_heading_level, validate_power,
};
use cast_iron_schema::{Block, Inline, ParagraphStyle, Table};

// --- A tiny builder: `**` toggles bold. ---------------------------------------------------------

fn para(text: &str) -> Block {
    let content = text
        .split("**")
        .enumerate()
        .filter(|(_, part)| !part.is_empty())
        .map(|(index, part)| {
            if index % 2 == 1 {
                Inline::Strong {
                    c: vec![Inline::text(part)],
                }
            } else {
                Inline::text(part)
            }
        })
        .collect();
    Block::Paragraph {
        style: ParagraphStyle::Normal,
        content,
    }
}

fn heading(level: u8, text: &str) -> Block {
    Block::Heading {
        level,
        anchor: None,
        content: vec![Inline::text(text)],
    }
}

fn table() -> Block {
    Block::Table(Table::default())
}

/// The candidate of `body` called `name`, at `occurrence` (from 1).
fn candidate(body: &[Block], name: &str, occurrence: usize) -> Candidate {
    candidates(body)
        .into_iter()
        .find(|candidate| candidate.name == name && candidate.occurrence == occurrence)
        .unwrap_or_else(|| panic!("no candidate `{name}` #{occurrence}"))
}

/// The reasons that validation gives for a candidate, sorted, or none when it passes.
fn reasons(body: &[Block], name: &str, occurrence: usize) -> Vec<&'static str> {
    let found = candidate(body, name, occurrence);
    match validate_power(found.blocks(body)) {
        Ok(_) => Vec::new(),
        Err(problems) => {
            let mut slugs: Vec<&'static str> = problems.iter().map(PowerProblem::reason).collect();
            slugs.sort_unstable();
            slugs.dedup();
            slugs
        }
    }
}

fn assert_canonical(body: &[Block], name: &str, occurrence: usize) {
    let found = candidate(body, name, occurrence);
    if let Err(problems) = validate_power(found.blocks(body)) {
        panic!("`{name}` #{occurrence} should be canonical: {problems:?}");
    }
}

fn assert_fails(body: &[Block], name: &str, occurrence: usize, expected: &[&str]) {
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(
        reasons(body, name, occurrence),
        expected,
        "`{name}` #{occurrence}"
    );
}

fn only_candidate(body: &[Block]) -> Candidate {
    let mut found = candidates(body);
    assert_eq!(found.len(), 1, "{found:?}");
    found.remove(0)
}

// --- The fixtures. ------------------------------------------------------------------------------

/// `ref/discovery/elixir-of-life`: canonical; a one-power page.
#[test]
fn elixir_of_life_is_canonical_with_one_continuation_paragraph() {
    let body = vec![
        para("**Elixir of Life**"),
        para("**Ability:** Once per day, the alchemist can brew an elixir of life."),
        para("Alternatively, the alchemist himself may drink the elixir of life."),
        para("**Prereq:** 16th-level alchemist"),
    ];
    assert_canonical(&body, "Elixir of Life", 1);
    let power = one_power_page("Elixir of Life", &body).expect("a one-power page");
    assert_eq!(power.effect.label, "Ability");
    assert_eq!(power.effect.continuation.len(), 1);
    assert!(power.prereq.is_some());
    assert_eq!(power.gained, None);
}

/// `ref/arcaneschool/abjuration`, Resistance (Ex): rewritten (upgrades move after the effect).
#[test]
fn abjuration_resistance_has_old_upgrade_labels_and_the_rewrite_target_passes() {
    let body = vec![
        para("**Resistance (Ex)**"),
        para("**At 11th Level:** This resistance increases to 10."),
        para("**At 20th Level:** This resistance changes to immunity to the chosen energy type."),
        para("**Gained:** At 1st Level"),
        para("**Passive:** You gain resistance 5 to an energy type of your choice."),
    ];
    assert_fails(&body, "Resistance (Ex)", 1, &["legacy-label"]);

    let rewritten = vec![
        para("**Resistance (Ex)**"),
        para("**Gained:** At 1st Level"),
        para("**Passive:** You gain resistance 5 to an energy type of your choice."),
        para("**At 11th level,** this resistance increases to 10."),
        para("**At 20th level,** this resistance changes to immunity to the chosen energy type."),
    ];
    assert_canonical(&rewritten, "Resistance (Ex)", 1);
    let found = candidate(&rewritten, "Resistance (Ex)", 1);
    let power = validate_power(found.blocks(&rewritten)).unwrap();
    assert_eq!(power.gained, Some(1));
    let levels: Vec<u8> = power.upgrades.iter().map(|upgrade| upgrade.level).collect();
    assert_eq!(levels, [11, 20]);
}

/// `ref/arcaneschool/abjuration`, Counterspell Mastery: queued (two effect rows).
#[test]
fn abjuration_counterspell_mastery_has_two_effect_rows() {
    let body = vec![
        para("**Counterspell Mastery**"),
        para("**Ability:** You gain Improved Counterspell as a bonus feat."),
        para("**Immediate:** You may attempt to counterspell an opponent's spell."),
        para("**Gained:** At 6th Level"),
        para("**Usage:** 1 time/day + 1 per 4 wizard levels beyond 6th"),
    ];
    let slugs = reasons(&body, "Counterspell Mastery", 1);
    assert!(slugs.contains(&"multiple-effect-rows"), "{slugs:?}");
}

/// The whole Abjuration body: the valid powers pass, the others fail, and the headings of the
/// powers follow the preceding top-level heading.
#[test]
fn abjuration_body_has_the_powers_the_decision_lists() {
    let body = vec![
        para("The abjurer uses magic against itself."),
        para("**Protective Ward (Su)**"),
        para("**Gained:** At 1st Level"),
        para("**Standard:** As a standard action, you can create a field of protective magic."),
        para("**Usage:** 3 + Intelligence modifier times/day"),
        para("**Energy Absorption (Su)**"),
        para("**Gained:** At 6th Level"),
        para(
            "**Passive:** You gain an amount of energy absorption equal to 3 × your wizard level.",
        ),
        heading(3, "Banishment School"),
        para("**Sources:** Advanced Player's Guide pg 144"),
        para("**Replacement Powers:** The following school powers replace the others."),
        para("**Unstable Bonds (Su)**"),
        para(
            "**Ability:** As a melee touch attack, you can cause a summoned creature to become shaken.",
        ),
        para("**Gained:** At 1st Level"),
        para("**Usage:** 3 + Intelligence modifier times/day"),
    ];
    let found = candidates(&body);
    let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Protective Ward (Su)",
            "Energy Absorption (Su)",
            "Unstable Bonds (Su)"
        ]
    );
    assert_canonical(&body, "Protective Ward (Su)", 1);
    assert_canonical(&body, "Energy Absorption (Su)", 1);
    assert_fails(&body, "Unstable Bonds (Su)", 1, &["out-of-order"]);
    // Before `### Banishment School` a power is `h2`; after it, one level below, so `h4`.
    assert_eq!(power_heading_level(&body, found[0].start), 2);
    assert_eq!(power_heading_level(&body, found[1].start), 2);
    assert_eq!(power_heading_level(&body, found[2].start), 4);
}

/// `ref/aspect/giant-wasp`, Minor Form: rewritten (two legacy upgrades become leads).
#[test]
fn giant_wasp_minor_form_has_legacy_upgrades_and_the_rewrite_target_passes() {
    let body = vec![
        para("**Minor Form**"),
        para("**Passive:** You gain a +4 bonus on Will saves against mind-affecting affects."),
        para("At 8th level: The bonus increases to +6."),
        para("At 15th level: The bonus increases to +8."),
        para("**Major Form**"),
        para("**Ability:** Your shape changes to that of a giant wasp."),
    ];
    assert_fails(&body, "Minor Form", 1, &["legacy-upgrade"]);

    let rewritten = vec![
        para("**Minor Form**"),
        para("**Passive:** You gain a +4 bonus on Will saves against mind-affecting affects."),
        para("**At 8th level,** the bonus increases to +6."),
        para("**At 15th level,** the bonus increases to +8."),
    ];
    assert_canonical(&rewritten, "Minor Form", 1);
}

/// `ref/deed/utility-shot`, Utility Shot (Ex): queued (three empty sub-option rows). The span runs
/// from the name to the end of the body.
#[test]
fn utility_shot_spans_to_the_end_of_the_body_and_has_empty_rows() {
    let body = vec![
        para("**Utility Shot (Ex)**"),
        para(
            "**Ability:** If the gunslinger has at least 1 grit point, she can perform all of the following utility shots.",
        ),
        para("**Prereq:** 3rd-level gunslinger"),
        para("**Blast Lock:**"),
        para("**Ability:** The gunslinger makes an attack roll against a lock."),
        para("The lock gains a bonus to its AC against this attack based on its quality."),
        para("**Scoot Unattended Object:**"),
        para("**Ability:** The gunslinger makes an attack roll against a Tiny or smaller object."),
        para("**Stop Bleeding:**"),
        para(
            "**Ability:** The gunslinger makes a firearm attack and presses the hot barrel against herself.",
        ),
    ];
    let found = only_candidate(&body);
    assert_eq!((found.start, found.end), (0, body.len()));
    let found = candidate(&body, "Utility Shot (Ex)", 1);
    let problems = validate_power(found.blocks(&body)).unwrap_err();
    let empty: Vec<&str> = problems
        .iter()
        .filter_map(|problem| match problem {
            PowerProblem::EmptyRow { label } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        empty,
        ["Blast Lock", "Scoot Unattended Object", "Stop Bleeding"]
    );
    assert!(problems.contains(&PowerProblem::MultipleEffectRows));
    // The empty rows are rows, never names: they start no candidate of their own.
    assert_eq!(candidates(&body).len(), 1);
}

/// `ref/blrgbloodline/fey`, One with Nature (Su): queued (`Move-Equivalent Action` is not in the
/// map).
#[test]
fn one_with_nature_has_an_unmapped_action_value() {
    let body = vec![
        para("**One with Nature (Su)**"),
        para("**Action:** Move-Equivalent Action"),
        para("**Gained:** At 16th Level"),
        para("**Usage:** Three times a day."),
        para("**Usage:** Three times a day."),
        para("You can transport yourself from one tree to another tree."),
        para("You gain these abilities even while not bloodraging."),
    ];
    assert_fails(
        &body,
        "One with Nature (Su)",
        1,
        &[
            "legacy-label",
            "no-effect-row",
            "second-singleton-row",
            "stray-paragraph",
        ],
    );
    let Block::Paragraph { content, .. } = &body[1] else {
        unreachable!()
    };
    assert_eq!(map_action_value(&content[1..]), ActionMapping::Unmapped);
}

/// `ref/hex/coven`, Coven (Ex): queued (numbered effect label `Ability2`).
#[test]
fn coven_has_a_numbered_effect_label() {
    let body = vec![
        para("**Coven (Ex)**"),
        para("**Ability:** The witch counts as a hag for the purpose of joining a hag's coven."),
        para(
            "**Ability2:** In addition, whenever the witch with this hex is within 30 feet of another witch with this hex, she can use the aid another action.",
        ),
    ];
    // A numbered label is its own reason, not a second effect row.
    assert_fails(&body, "Coven (Ex)", 1, &["numbered-effect-label"]);
}

/// `ref/mystery/flame`, Firestorm (Su): rewritten (`Prereq` moves after `Usage`; the identical
/// second `Usage` row goes).
#[test]
fn firestorm_has_prereq_before_the_effect_and_the_rewrite_target_passes() {
    let body = vec![
        para("**Firestorm (Su)**"),
        para("**Flavor:** You can cause fire to erupt around you."),
        para("**Prereq:** 11th-level oracle"),
        para("**Standard:** You can create one 10-foot cube of fire per oracle level."),
        para("**Usage:** Once per day."),
        para("**Usage:** Once per day."),
    ];
    assert_fails(
        &body,
        "Firestorm (Su)",
        1,
        &["out-of-order", "second-singleton-row"],
    );

    let rewritten = vec![
        para("**Firestorm (Su)**"),
        para("**Flavor:** You can cause fire to erupt around you."),
        para("**Standard:** You can create one 10-foot cube of fire per oracle level."),
        para("**Usage:** Once per day."),
        para("**Prereq:** 11th-level oracle"),
    ];
    assert_canonical(&rewritten, "Firestorm (Su)", 1);
}

/// `ref/mystery/reaper`, Haunt Channeler (Su) #1: canonical; #2: queued (`Ability2` and two
/// effect rows; the occurrence tells it from the first).
#[test]
fn the_two_haunt_channelers_differ_by_occurrence() {
    let body = vec![
        para("**Haunt Channeler (Su)**"),
        para("**Ability:** You gain the medium's haunt channeler class feature."),
        para("**Prereq:** 5th-level oracle"),
        para("**Haunt Channeler (Su)**"),
        para("**Ability:** Your touch funnels a portion of the haunt into yourself."),
        para(
            "**Ability2:** Additionally, you can forgo your saving throw against a haunt's effects.",
        ),
        para("**Flavor:** Text from the medium class ability."),
        para("**Passive:** You can always act on the surprise round against a haunt."),
    ];
    assert_canonical(&body, "Haunt Channeler (Su)", 1);
    assert_fails(
        &body,
        "Haunt Channeler (Su)",
        2,
        &[
            "numbered-effect-label",
            "multiple-effect-rows",
            "out-of-order",
        ],
    );
    let second = candidate(&body, "Haunt Channeler (Su)", 2);
    assert_eq!(second.start, 3);
}

/// `ref/arcaneschool/divination`, Foretell (Su): queued. The rewrite queues it as a seeded print
/// item; this PR can show that the run is a candidate with a legacy `Action` row whose effect text
/// is a settled group, so only the seed keeps the rewrite from mapping it.
#[test]
fn foretell_is_a_candidate_with_a_legacy_action_row() {
    let body = vec![
        para("**Foretell (Su)**"),
        para("**Action:** Standard Action"),
        para("**Gained:** At 8th Level"),
        para("**Usage:** 1 time/day per wizard level"),
        para("You can utter a prediction of the immediate future."),
        para("If you choose to aid, you and your allies gain a +2 luck bonus."),
        para("If you choose to hinder, your enemies take a -2 penalty on those rolls instead."),
        heading(3, "Prophecy School"),
    ];
    assert_fails(
        &body,
        "Foretell (Su)",
        1,
        &["legacy-label", "no-effect-row", "stray-paragraph"],
    );
    let found = only_candidate(&body);
    assert_eq!((found.start, found.end), (0, 7));
    let Block::Paragraph { content, .. } = &body[1] else {
        unreachable!()
    };
    assert_eq!(
        map_action_value(&content[1..]),
        ActionMapping::Effect("Standard")
    );
    assert_eq!(action_effect_group(found.blocks(&body), 1), Ok(4..7));
}

/// `ref/arcaneschool/divination`, Forewarned (Su): queued. The old upgrade label fails validation;
/// the capital `Anytime` is a rule of the rewrite's lowercase list and not of validation, so a
/// canonical run keeps its wording.
#[test]
fn forewarned_has_an_old_upgrade_label_and_validation_keeps_a_capital() {
    let body = vec![
        para("**Forewarned (Su)**"),
        para(
            "**At 20th Level:** Anytime you roll initiative, assume the roll resulted in a natural 20.",
        ),
        para("**Gained:** At 1st Level"),
        para("**Passive:** You can always act in the surprise round."),
    ];
    assert_fails(&body, "Forewarned (Su)", 1, &["legacy-label"]);

    let capital = vec![
        para("**Forewarned (Su)**"),
        para("**Gained:** At 1st Level"),
        para("**Passive:** You can always act in the surprise round."),
        para(
            "**At 20th level,** Anytime you roll initiative, assume the roll resulted in a natural 20.",
        ),
    ];
    assert_canonical(&capital, "Forewarned (Su)", 1);
}

/// `ref/ragepower/auspicious-mark`, Auspicious Mark (Su): rewritten (`Flavor`, effect, `Usage`,
/// then `Special`; the identical second `Usage` row goes). A one-power page that keeps `Flavor`.
#[test]
fn auspicious_mark_is_reordered_by_the_rewrite_and_stays_a_one_power_page() {
    let body = vec![
        para("**Auspicious Mark (Su)**"),
        para("**Flavor:** The barbarian has been marked by the spirits."),
        para(
            "**Special:** She can call on the auspicious mark after seeing the result of the d20 roll.",
        ),
        para(
            "**Swift:** By spending 2 rounds of rage, the barbarian can call upon the spirits' favor.",
        ),
        para("**Usage:** Once per rage"),
        para("**Usage:** Once per rage"),
    ];
    assert_fails(
        &body,
        "Auspicious Mark (Su)",
        1,
        &["out-of-order", "second-singleton-row"],
    );
    assert!(one_power_page("Auspicious Mark", &body).is_none());

    let rewritten = vec![
        para("**Auspicious Mark (Su)**"),
        para("**Flavor:** The barbarian has been marked by the spirits."),
        para(
            "**Swift:** By spending 2 rounds of rage, the barbarian can call upon the spirits' favor.",
        ),
        para("**Usage:** Once per rage"),
        para(
            "**Special:** She can call on the auspicious mark after seeing the result of the d20 roll.",
        ),
    ];
    let power = one_power_page("Auspicious Mark", &rewritten).expect("a one-power page");
    assert!(power.flavor.is_some());
    assert_eq!(power.effect.label, "Swift");
    assert_eq!(power.other.len(), 1);
    assert_eq!(power.other[0].label, "Special");
}

/// `ref/hex/evil-eye`, Evil Eye (Su): rewritten (`Flavor` moves before the effect; the upgrade
/// becomes a lead). A one-power page.
#[test]
fn evil_eye_is_reordered_by_the_rewrite_and_stays_a_one_power_page() {
    let body = vec![
        para("**Evil Eye (Su)**"),
        para("**Ability:** The target takes a -2 penalty on one of the following."),
        para("**Flavor:** The witch can cause doubt to creep into the mind of a foe."),
        para("**At 8th Level:** The penalty increases to -4."),
        para(
            "**Special:** This hex lasts for a number of rounds equal to 3 + the witch's Intelligence modifier.",
        ),
    ];
    assert_fails(&body, "Evil Eye (Su)", 1, &["legacy-label", "out-of-order"]);

    let rewritten = vec![
        para("**Evil Eye (Su)**"),
        para("**Flavor:** The witch can cause doubt to creep into the mind of a foe."),
        para("**Ability:** The target takes a -2 penalty on one of the following."),
        para("**At 8th level,** the penalty increases to -4."),
        para(
            "**Special:** This hex lasts for a number of rounds equal to 3 + the witch's Intelligence modifier.",
        ),
    ];
    let power = one_power_page("Evil Eye", &rewritten).expect("a one-power page");
    assert_eq!(power.upgrades.len(), 1);
}

/// `ref/hex/flight`, Flight (Su): queued (no effect row; the lowest upgrade can become the base).
#[test]
fn flight_has_no_effect_row_and_gets_the_suggestion() {
    let body = vec![
        para("**Flight (Su)**"),
        para("**Flavor:** The witch grows lighter as she gains power."),
        para("**At 1st Level:** The witch can use feather fall at will."),
        para("**At 3rd Level:** She can cast levitate once per day."),
        para("**At 5th Level:** She can fly, as per the spell."),
        para("**Special:** This hex only affects the witch."),
    ];
    assert_fails(&body, "Flight (Su)", 1, &["legacy-label", "no-effect-row"]);
    let found = candidate(&body, "Flight (Su)", 1);
    let problems = validate_power(found.blocks(&body)).unwrap_err();
    assert!(problems.contains(&PowerProblem::NoEffectRow {
        suggest_lowest_upgrade: true
    }));
}

/// `ref/domain/fire`, Fire Resistance (Ex): queued (no effect row; the lowest upgrade can become
/// the base). The upgrades are out of order in the source, which is the rewrite's to sort.
#[test]
fn fire_resistance_has_no_effect_row_and_gets_the_suggestion() {
    let body = vec![
        para("**Fire Resistance (Ex)**"),
        para("**At 12th Level:** Your resistance increases to 20."),
        para("**At 20th Level:** You gain immunity to fire."),
        para("**At 6th Level:** You gain resist fire 10."),
        para("**Domain Spells**"),
        para("**Spells:** Burning hands (1st)"),
    ];
    assert_fails(
        &body,
        "Fire Resistance (Ex)",
        1,
        &["legacy-label", "no-effect-row"],
    );
    let found = candidate(&body, "Fire Resistance (Ex)", 1);
    assert_eq!(found.end, 4);
    let problems = validate_power(found.blocks(&body)).unwrap_err();
    assert!(problems.contains(&PowerProblem::NoEffectRow {
        suggest_lowest_upgrade: true
    }));
}

/// `ref/hex/blight`, Blight (Su): queued (the `Action` paragraph group would cross a `Special`
/// row). The affliction after it holds no power marker and is no candidate.
#[test]
fn blight_action_group_crosses_a_special_row() {
    let body = vec![
        para("**Blight (Su)**"),
        para("**Action:** Ability"),
        para("**Special:** Both types of curse can be removed with a remove curse."),
        para("The witch can curse an animal, plant creature, or plot of land."),
        para("If it's used on a plot of land, the land begins to wither the following day."),
        para("Blighting a creature is a **standard action** that requires a melee touch attack."),
        para("**Blight Hex**"),
        para("**Type:** Curse"),
        para("**Save:** Will negates"),
        para("**Frequency:** 1/day"),
        para("**Effect:** Constitution damage 1"),
    ];
    let found = only_candidate(&body);
    assert_eq!((found.start, found.end), (0, 6));
    assert_fails(
        &body,
        "Blight (Su)",
        1,
        &["legacy-label", "no-effect-row", "stray-paragraph"],
    );
    let Block::Paragraph { content, .. } = &body[1] else {
        unreachable!()
    };
    assert_eq!(
        map_action_value(&content[1..]),
        ActionMapping::Effect("Ability")
    );
    assert_eq!(
        action_effect_group(found.blocks(&body), 1),
        Err(ActionGroupProblem::Crosses {
            what: "the row `Special:`".into()
        })
    );
}

/// `ref/arcana/accurate-strike`, Accurate Strike (Ex): rewritten (the name loses its colon). A
/// one-power page. The legacy name is a row that no candidate starts from; only the rewrite
/// treats it as a name.
#[test]
fn accurate_strike_legacy_name_is_an_empty_row_and_the_rewrite_target_passes() {
    let body = vec![
        para("**Accurate Strike (Ex):**"),
        para("**Prereq:** 9th-level magus"),
        para("**Swift:** The magus can expend 2 points from his arcane pool."),
    ];
    assert!(candidates(&body).is_empty());
    // Validating the span directly cannot accept it either: no power with an empty name.
    assert_eq!(
        validate_power(&body),
        Err(vec![
            PowerProblem::NotAName,
            PowerProblem::OutOfOrder {
                label: "Swift".into()
            }
        ])
    );
    let with_effect_first = [&body[..1], &body[2..]].concat();
    assert_eq!(
        validate_power(&with_effect_first),
        Err(vec![PowerProblem::NotAName])
    );

    let rewritten = vec![
        para("**Accurate Strike (Ex)**"),
        para("**Swift:** The magus can expend 2 points from his arcane pool."),
        para("**Prereq:** 9th-level magus"),
    ];
    assert_canonical(&rewritten, "Accurate Strike (Ex)", 1);
    let power = one_power_page("Accurate Strike", &rewritten).expect("a one-power page");
    assert_eq!(power.effect.label, "Swift");
}

/// `ref/talent/black-market-connections`, Black Market Connection (Ex): rewritten. The name
/// differs from the title `Black Market Connections`, so the head stays: it is no one-power page.
#[test]
fn black_market_connection_is_not_a_one_power_page_for_its_title() {
    let body = vec![
        para("**Black Market Connection (Ex)**"),
        para("**Action:** Ability"),
        para("A rogue with this talent gains better access to magic items."),
        para(
            "With a successful Diplomacy check, the rogue can treat the settlement as two sizes larger.",
        ),
        para("With a successful check, the rogue can also sell stolen items on the black market."),
        table(),
    ];
    assert_fails(
        &body,
        "Black Market Connection (Ex)",
        1,
        &["legacy-label", "no-effect-row", "stray-paragraph"],
    );
    let found = only_candidate(&body);
    assert_eq!((found.start, found.end), (0, 5), "the table is a boundary");
    assert_eq!(action_effect_group(found.blocks(&body), 1), Ok(2..5));

    let rewritten = vec![
        para("**Black Market Connection (Ex)**"),
        para("**Ability:** A rogue with this talent gains better access to magic items."),
        para(
            "With a successful Diplomacy check, the rogue can treat the settlement as two sizes larger.",
        ),
        para("With a successful check, the rogue can also sell stolen items on the black market."),
        table(),
    ];
    assert_canonical(&rewritten, "Black Market Connection (Ex)", 1);
    let found = candidate(&rewritten, "Black Market Connection (Ex)", 1);
    let power = validate_power(found.blocks(&rewritten)).unwrap();
    assert_eq!(power.effect.continuation.len(), 2);
    assert!(one_power_page("Black Market Connections", &rewritten).is_none());
    assert!(one_power_page("Black Market Connection", &rewritten).is_some());
}

/// Synthetic S1: `Fullround` and `Full-round` are two effect rows; alias mapping never merges
/// rows.
#[test]
fn s1_an_alias_and_its_canonical_label_are_two_effect_rows() {
    let body = vec![
        para("**Rending Shot**"),
        para("**Fullround:** You make a shot."),
        para("**Full-round:** You make a shot."),
    ];
    assert_fails(
        &body,
        "Rending Shot",
        1,
        &["legacy-label", "multiple-effect-rows"],
    );
}

/// Synthetic S2: an empty `Usage` row is a row, never a name.
#[test]
fn s2_an_empty_row_fails_and_is_never_a_name() {
    let body = vec![
        para("**Quick Step**"),
        para("**Ability:** You step quickly."),
        para("**Usage:**"),
    ];
    assert_fails(&body, "Quick Step", 1, &["empty-row"]);
    assert_eq!(candidates(&body).len(), 1);
}

/// Synthetic S3: a canonical run is canonical, so the rewrite has nothing to change in it.
#[test]
fn s3_a_canonical_run_passes_with_every_part() {
    let body = vec![
        para("**Resistance (Ex)**"),
        para("**Gained:** At 1st Level"),
        para("**Flavor:** The wizard resists energy."),
        para("**Standard:** You gain resistance 5."),
        para("The resistance can be changed each day."),
        para("**Usage:** 3 + Intelligence modifier times/day"),
        para("**At 11th level,** this resistance increases to 10."),
        para("**At 20th level,** this resistance changes to immunity."),
        para("**Prereq:** 3rd-level wizard"),
        para("**Replaces:** Arcane bond"),
        para("**Special:** Rows of any other label keep their source order."),
        para("**Penalty:** The other row."),
    ];
    assert_canonical(&body, "Resistance (Ex)", 1);
    let found = candidate(&body, "Resistance (Ex)", 1);
    let power = validate_power(found.blocks(&body)).unwrap();
    assert_eq!(power.gained, Some(1));
    assert_eq!(power.effect.label, "Standard");
    assert_eq!(power.effect.continuation.len(), 1);
    assert!(power.flavor.is_some() && power.usage.is_some());
    assert!(power.prereq.is_some() && power.replaces.is_some());
    let other: Vec<&str> = power.other.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(other, ["Special", "Penalty"]);
}

/// Every canonical effect label is accepted in the effect position.
#[test]
fn every_canonical_effect_label_passes() {
    for label in EFFECT_LABELS {
        let body = vec![
            para("**A Power**"),
            para(&format!("**{label}:** It works.")),
        ];
        assert_canonical(&body, "A Power", 1);
    }
}
