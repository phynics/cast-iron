---
title: "The drift check ends the normal part by publication, not by a body Sources line"
date: 2026-11-23
status: current
superseded_by: []
issues: [623]
prs: []
area: corpus
---

# The drift check ends the normal part by publication, not by a body Sources line, 2026-11-23

The drift check compares a feat, archetype or class option with its normal part: the body before
another source's version of the entry starts. It used to find that end only through a `**Sources:**`
paragraph (a `###` heading with one directly below it, or a second bare one). The citation work of
phynics/cast-iron-data#170 moves those paragraphs into `[[sources]]` rows, so the marker goes away.
The data kept 54 paragraphs only to hold the end in place (phynics/cast-iron-data#194).

## What "normal part" meant

The body up to the first `###` heading that is followed by a `**Sources:**` paragraph, or up to the
second `**Sources:**` paragraph, whichever comes first. The first such paragraph opened the part.

## What it means now

The body up to the first block that starts another publication's version:

- **A `**Sources:**` paragraph that is not the first block.** The first block names the entry's own
  citation. A later one names another publication. A `###` heading directly above it starts the
  block at the heading, as before.
- **A `###` heading, when the entry prints in more than one publication** (`source_ids`; every
  `[[sources]]` row names one of them). The rows cite the page of each printing, so a body line is
  redundant, and a second publication's `###` section needs no paragraph to mark it.

An entry of one publication has no other source. Its `###` headings organize the normal part and
stay in the comparison, as in the entries `ref/feat/virtuous-creed`, `ref/feat/draconic-heritage`
and the spirits whose typed trades name lines below a `###` heading.

Example, a domain of the Core Rulebook and the Advanced Player's Guide:

```markdown
**Replaces:** Detect evil      <- normal part: compared with the typed trades
### Alchemy Subdomain          <- ends the normal part (two publications)
**Replaces:** Smite evil       <- subdomain line: not compared
```

Stat blocks, titled or not, are not paragraphs. They neither end the part nor mark it. Creature,
item and spell rows still come from the first stat block that holds the label.

## Why not read the rows alone

A `[[sources]]` row carries a publication and a page, not a heading, so no row says which `###`
section belongs to it. And 1,501 memberships in 384 entries still have no row (#461), so counting
rows would miss entries that print in a second book. `source_ids` is the membership, and each row
is a subset of it.

## Effect on the pinned corpus

On v0.1.40 the drift stays at 0. With every paragraph that sits under a `###` heading removed
from the 59 entries that have one (a scratch copy of what the data change leaves), the old rule
reports 58 disagreements and this rule reports 0. In 36 multi-publication entries the normal part
now ends at an unmarked first `###` heading and the lines below it are no longer compared; none of
them disagrees with its typed trades. The data repository can drop the 54 marker paragraphs.
