---
title: "An option group names the class feature that grants it with an explicit anchor"
date: 2026-11-27
status: current
superseded_by: []
issues: [540, 742, 754]
prs: []
area: corpus
---

# An option group names the class feature that grants it with an explicit anchor, 2026-11-27

The Options section of a class page (`2026-11-27-class-options-section`) puts a link row such as
`Inquisitions · N ›` under the class feature that grants a choice from a group. It found that feature
by printed name: the group's label or singular, compared with the headings of the class. 12 of 54
groups match no heading. For three of them (from the Advanced Player's Guide classes) no printed
heading names the group at all: the Inquisitor prints `Domain`, and "inquisition" is only in its first
paragraph; the Summoner and the Unchained Summoner print `Eidolon`, with no head that says
"evolution". Name matching cannot find these, and a looser rule links the wrong feature as often as
the right one.

## Decision

- **An optional `feature` key on a group of `option-groups.toml`** holds the explicit `{#anchor}` of
  the class heading that grants the choice: `feature = "domain"` on `inquisition`. The format is in
  `docs/corpus-format.md` ("`option-groups.toml`").
- **It wins over name matching.** A group with `feature` is placed only by that anchor. The class page
  does not fall back to the label when the page has no heading with the anchor: it shows no row, and
  the chip in the Options section still works. A group without the key keeps name matching, so every
  existing row stays where it is.
- **It names one anchor for every class in `classes`.** `corpus-check` fails when a class that the
  group lists has no heading with exactly that explicit anchor (a heading that only gets its key from
  its text does not count), so a checked corpus never reaches a class page without the heading. A
  group whose classes need different anchors (Summoner and Unchained Summoner print the same
  `eidolon`, so they fit; a group shared by classes with different heads would not) needs a later,
  additive change, such as a per-class table. It is not built before a group needs it.
- **The key is additive.** `SCHEMA_VERSION` stays 7: `OptionGroup::feature` is `Option<String>` with
  `skip_serializing_if` and `serde(default)`, so a library whose corpus sets no key has a byte-identical
  `option-groups.json`, and no type of the schema crate denies unknown fields, so an older client
  ignores the key. The corpus format number stays at 4. The writer writes nothing for an unset key, so
  `corpus-fmt` leaves an existing file byte-identical.
- **The row goes after the text under the heading**, before the next heading of the same or a higher
  level, as a name-matched heading's row does. The heading may be anywhere in the class body; it does
  not have to sit inside the Features section.
- **No group sets the key yet.** The data thread sets it for `inquisition`, `summoner-evolution`,
  `unchained-summoner-evolution` and the other groups as their classes convert.
