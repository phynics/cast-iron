---
title: "Pf Data migration coverage exceptions"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Pf Data migration coverage exceptions, 2026-11-04

- **Explained omissions are coverage, not findings.** `migration-test.json` and `migration-full.json` separate `findings` from `coverage`. A finding still means a converter defect or a reference that does not resolve anywhere in the pinned inventory. Coverage records a construct the converter deliberately omits, with a stable category, the exact detail, a reason, and the affected record. Nothing is deleted: the Markdown report lists every coverage row and a per-category tally.
- **Presentation metadata is not body content.** Jump-list flags (`jl`), ability order (`order`), layout flags (`head`, `next`, `sub`, `indent`), `::aside` and `::fakeFootnotes` wrappers, and `@I-` icons are recorded under `jump-list-metadata`, `ability-order-metadata`, `ability-structure-metadata`, `layout-container`, and `presentation-icon`. Container contents and surrounding prose still render.
- **Outside-sample references stay visible text.** A link whose target exists in the pinned inventory but not in the bounded selection is recorded as `reference-outside-sample`; the link text is kept and a full import resolves the target. A target missing from the inventory is still a high-severity `unresolved reference` finding.
- **An untitled stat directive must not emit `****`.** `::mdefense` and other unbracketed directives now start with an empty body instead of an empty bold heading. Markdown parsed the old `****` as a thematic break, which the HTML pass then dropped as an `<hr>`. The three observed `hr` findings were this bug, not missing content.
- **Fixture `references` checks only fail on real reference defects.** Outside-sample targets no longer fail the mandatory fixtures; unresolved targets still do.
