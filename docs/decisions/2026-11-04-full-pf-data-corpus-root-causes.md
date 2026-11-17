---
title: "Full Pf Data corpus root causes"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Full Pf Data corpus root causes, 2026-11-04

- **A link to a selected record that failed conversion is not a second dangling reference.** The full run now marks selected native keys without a converted entry and records each such link as `reference-to-failed-record` coverage, naming the target. The failed target keeps its own high conversion finding, so the report shows one root cause instead of tens of thousands of link findings. Ordering is not the cause: the target map is populated from every record that converted before the final pass, and the remaining misses were failed targets. External URLs and non-entry app or source shorthands are `external-link` and `non-entry-link` coverage with the visible text kept.
- **Markdown footnote definitions are typed like `::mfn`.** `[^D]: Domain spell` becomes a level-6 heading with anchor `footnote-d` plus the note as a paragraph. `[^D]` references resolve to that anchor, and pulldown's definition `<div>` is no longer an `unwrapped_tag:div` finding. This covers the `#O`, `#D`, `#S`, and numeric footnote anchors.
- **Container directives do not drop text.** `:::block` and `:::group` are layout wrappers recorded as `layout-container`; a block title becomes a heading after the upstream `&L&`/`&FN&` markers are removed. `:::stamina` renders a Stamina heading. `:::mythic` renders its `title` as a heading and treats `jl` as jump-list metadata. Their closing `:::` is not content.
- **Encoded list fields are split, not rejected.** `::minfo` subtype and sense fields (`subs`, `othersubs`, `augment`, `sen`, `senSpell`) split on `~`. `::mdefense` keeps its hit-point encoding and now splits encoded `def` and `weak` lists into labeled prose. Unknown encodings on other fields still fail.
- **Addenda codewords are validated, not expanded.** The pinned OGC `json/` stores only codewords such as `sneakattack`; the reminder text exists only in upstream application code (`src/pages/*.tsx`), which is GPL and not Open Game Content. Copying it would break AGENTS.md rule 12. The converter accepts the 16 observed codewords, records each as `addendum-reminder-omitted` coverage (301 rows in the full run), and turns an unknown codeword into a conversion failure. The record body still converts.
- **The migration exits on the full corpus with high findings below 2,000 while schema 3 and search format 1 stay unchanged.** Representative mode keeps 0 findings and 6/6 fixtures. Production content still comes from PSRD-Data and is not changed by these converter fixes.
