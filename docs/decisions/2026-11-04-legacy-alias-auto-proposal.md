---
title: "Legacy alias auto-proposal"
date: 2026-11-04
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Legacy alias auto-proposal, 2026-11-04

- **The proposal tool shares the gate, not a parallel matcher.** `cargo xtask data-alias-propose` rebuilds the same canonical catalog as `cargo xtask data-alias` and matches every `no_source` row through `alias::title_fit`, the function the gate itself calls. A proposed row therefore cannot pass the proposer and fail generation. The command writes `generated/migration/id-map.proposed.toml` and `generated/migration/alias-proposals.json`; `--apply` rewrites the committed map. A second run finds no `no_source` rows and produces the same bytes.
- **Matching requires kind, source book, and title evidence together.** The legacy kind must equal the canonical kind; the legacy source must appear in the record's publication set; and the titles must be equal after case, punctuation, and spacing normalization, or differ only by a documented spelling. Comma inversion accepts `Dragon, Red` for `Red Dragon` and drops the PSRD `Primal`/`Imperial` dragon-category word. A trailing family/category parenthetical accepts `Olethrodaemon` for `Olethrodaemon (Daemon)` only for creatures, traits, feats, and afflictions. For items, a parenthetical must be a measure or package (`Chain (10 ft.)`, `Ale (gallon)`), or the `weapon`/`armor` slot label selected by the path tie-break; `Lock (average)`, `Tent (large)`, and `Smart Armor (compact form)` are variants, not matches. A slug is never evidence by itself.
- **Tie-breaks are limited and deterministic.** A native record beats its derived duplicate. A racial trait whose parent race is named by the legacy path wins. A `weapon-special-abilities` or `armor-special-abilities` path picks the candidate whose parenthetical or id names that slot. Everything else stays retired with `ambiguous: <candidates>`, and a proposal that would merge onto an existing non-shared `[map]` target is skipped so the 94 already published rows stay byte-identical.
- **The first proposal pass maps 7,658 more ids.** The map moves from 94 mapped (1 direct, 93 derived, 0 shared) and 9,874 `no_source` rows to 7,752 mapped (5,735 direct, 115 derived, 1,902 shared) and 2,216 retired, all `no_source`. Of the 7,658 proposals, 5,756 are `[map]` and 1,902 are `[shared]`; 17 ties remain ambiguous, 2,195 rows have no matching counterpart, and 4 merge onto an existing published target and stay retired. Remaining gaps are dominated by rules (512), items (418), creatures (327), and racial traits (275); the full per-kind breakdown is in `docs/status.md`. This supersedes the earlier "current state is deliberately incomplete" count, not the rule that an unmatched id stays explicit.
