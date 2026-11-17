---
title: "Full Pf Data corpus second root-cause pass"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Full Pf Data corpus second root-cause pass, 2026-11-05

- **An aggregate page without a publication is coverage, not a broken record.** 161 index, navigation, and disambiguation records carry a title but no `sources` entry, and no subtopic supplies one. They cannot be attributed to a Section 15 notice, so they are excluded with the stable category `unattributed-aggregate-record`. The links they point to are still covered as individual records.
- **`::trait` metadata is decoded, not rejected.** Six tildes start a paragraph or table, three tildes separate prose paragraphs, and three tildes inside a table become a row break when the next non-space character is `|`. `subs` and `surs` encode a label and body, so the label is used as the field label instead of the old guessed "Subtypes" and "Sources". `singleBreaks` is accepted as presentation metadata. The encoded values become typed paragraphs and tables.
- **Stat-block directives render their own facts.** `::spelllist`, `::room`, `::team`, `::trap`, `::haunt`, `::drug`, and `::lesserOrders` now emit labeled typed prose. The labels come from the documented attribute names and the existing PSRD-Data representation of the same records, not from upstream renderer wording. A domain spell list keeps its level order and annotations, and a room or team keeps Earnings, Benefit, Create, Time, Size, and Upgrade.
- **Presentation wrappers stay wrappers.** `:::blockquote` applies a display class. It is recorded under `layout-container`; its contents render in source order. The already-supported `:::elephant` path is unchanged.
- **Class-skill and header metadata are presentation.** `::cskill` accepts `noAlter`, `preHL`, `preTitle`, `postTitle`, and `icon`; its gained and lost lists and its pre/post prose still render. `::mh` accepts `jl` as jump-list metadata. These flags add no body prose.
- **Extra ability fields keep readable labels.** The `x*` ability fields that reach `::ab` render as prose labels such as Penalty, Description, Order, Gift, Reckoning, and Available patron themes instead of a mechanical key name.
- **A broad search term proves retrieval, not rank.** The full-run smoke check still requires the record in the first 1,000 hits, then retries with the whole result window before reporting a finding. A term such as "sources" is indexed even when thousands of entries share it; ranking has its own rule tests.
- **The full corpus now has 523 high findings, below the 800 target.** On the current canonical main, converted records rose from 28,496 to 29,024, findings fell from 1,579 to 858, and high findings fell from 1,244 to 523. Excluded records fell from 1,489 to 961. The deliberate `::aff block has no name` rejection remains 332 medium findings. Schema 3 and search format 1 are unchanged.
- **The default canonical library gains the newly converted records.** It moves from 29,515 entries at `vd7596846128e349b` to 30,000 entries at `v46b91fe4b0c71e28`, a net gain of 485 entries. The PSRD carry-over library is still built on demand with `cargo xtask content --psrd` and remains 9,968 entries at `vdb43853600b8ed8b`.
- **Evidence is local and reproducible.** `cargo xtask check` passes; `cargo xtask data-report` reports 0 findings and 6/6 fixtures; two full `data-import` runs and two canonical `cargo xtask content` runs are byte-identical; `scripts/check-library-upgrade.py generated/psrd generated/content` exits 0 with no missing ids, changed titles, wrong class categories, or dangling references.
