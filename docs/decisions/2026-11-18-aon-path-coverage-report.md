---
title: "AoN path coverage reports the corpus fact, not a guessed cause"
date: 2026-11-18
status: current
superseded_by: []
issues: [197]
prs: []
area: content
---

# AoN path coverage reports the corpus fact, not a guessed cause, 2026-11-18

Refs [#197](https://github.com/phynics/cast-iron/issues/197).

- **One published list, regenerated from the pin.** [`docs/research/aon-path-coverage.md`](../research/aon-path-coverage.md) holds the provenance, the per-kind matched/missing/total counts, and the publications without a source page. Its companion [`aon-path-coverage.tsv`](../research/aon-path-coverage.tsv) lists every canonical entry without an `aon` path as `id`, `kind`, `title`, sorted by kind then id. The ids are the permanent public ids, so a row stays identifiable after a later harvest adds a link.
- **Absence is the fact, not a cause.** The corpus stores only paths that a curation-time harvest verified against an AoN bulk listing. It stores no negative disposition. A row therefore means "this entry has no stored path"; it does not say whether the harvest found no page, found several, or never covered the kind. The report does not attach those causes to individual rows, because the corpus holds no evidence for one.
- **The harvest report is kind-level evidence.** [cast-iron-data #67](https://github.com/phynics/cast-iron-data/pull/67) separates matched, ambiguous and unmatched per kind on its own revision. It is cited for the kind-level explanation (rules, afflictions, traps, skills and racial traits had no bulk display-page listing; item listings were category indexes) and not reused as per-entry labels. On corpus v0.1.26, the stored-path count equals that report's matched count for every kind. Between the harvest and v0.1.26, the race total fell from 83 to 75 (nine missing paths to one), and the rule total rose from 6,237 to 6,238.
- **Reproduction reads the corpus, never AoN.** `cargo xtask aon-coverage` loads `.cache/sources/cast-iron-data`, which `cargo xtask fetch-sources` verifies against the tree hash in `sources/corpus.toml`. It writes no timestamp and sorts by kind then id, so two runs on one pin are byte-identical. `--check` writes nothing and fails when the committed files differ. Run it in the pull request that changes the corpus pin.
- **The command checks before it writes.** It rejects a duplicate id, an unknown kind, an empty title, and a title that would break the TSV. It requires every kind's matched plus missing to equal its total, the list's ids to equal the set of entries without a path, and the list's row count and per-kind counts to match the tables. The report's SHA-256 is the digest of the list it describes.
- **No link is guessed.** No row is a candidate, a near match or a base page. Nothing fetches AoN, no corpus entry is edited, and the app still renders a link only where the corpus stores a path. A later harvest that adds a path removes the entry from the next list; a later corpus edit that removes an entry removes its row.
