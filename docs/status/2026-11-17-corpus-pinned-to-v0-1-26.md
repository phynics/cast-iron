---
title: "Corpus pinned to v0.1.26"
date: 2026-11-17
kind: pin
---

# Corpus pinned to v0.1.26, 2026-11-17

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.26`. The annotated tag object
is `cbe97431a4b4a219edebb12ea05a08483d461e9e`; it peels to commit
`a4b3ef61952098358e62274be1d0e17da99ea2c8`. The tree hash is
`55180e45146b29cc154a5b0652c23a979bc45de91913eb9e759b666d8ece2476`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive, which `cargo xtask fetch-sources`
verified). This is the corpus of the release candidate ([#441](https://github.com/phynics/cast-iron/issues/441)).
It resolves the expanding-book credits and variant-family membership leaks of
[#459](https://github.com/phynics/cast-iron/issues/459): Summon Monster and Summon Monster I to IX
now carry one `[[expanded_in]]` row for Monster Summoner's Handbook, page 18, `via` the Versatile
Summon Monster feat, and no longer list the handbook in `source_ids`.

- **Counts.** 31,800 entries, 439 publications and 439 books, unchanged from v0.1.25.
  Reachability: 31,800 reachable, 0 unreachable. Search documents 39,008 (was 39,009). The library
  schema stays 6 and the search index format stays 2. The pin adds 10 `expanded_in` rows (it had
  none) and no entry, publication or id.
- **Membership.** The book memberships in the coverage report fall from 39,161 to 38,752 (-409)
  across 35 books; no book gains an entry. The largest drops: Ultimate Intrigue 737 to 691,
  Ultimate Wilderness 839 to 801, Heroes of the High Court 152 to 115, Heroes of the Streets 153
  to 123, Advanced Class Guide 943 to 915, Legacy of the First World 176 to 148, Occult Adventures
  840 to 812, Heroes of the Wild 169 to 144, Blood of Shadows 216 to 195, Legacy of Dragons 155 to
  136. Monster Summoner's Handbook falls from 78 to 68: its 10 credited spells are no longer
  members. The other books lose wrong credits and variant-family leaks.
- **Duplicate credits.** `corpus-check` now refuses two identical `[[expanded_in]]` rows on one
  entry (same publication, page and `via`); two rows that differ in page or `via` pass.
- **Test.** `summon-monster-expansion.spec.ts` proves that, with only Monster Summoner's Handbook
  active, the 10 credited spells are not in search or the S spell list and not in the book's count,
  and that Summon Monster III shows `Expanded in: Monster Summoner's Handbook p. 18` while visible.
  `expanded-in.spec.ts` keeps its overlay for the `via`, inactive and revision states.
- **Local verification** (CI not run, nothing deployed). Content version `vb4ec72f397a03202` (was
  `veada5093fd101aa3`, `main` at `2f1d4ca` with v0.1.25). `cargo xtask content` passed the
  published-id guard (42,019 ids, 0 added) and the book-slot guard (439 books, 0 added);
  `cargo xtask check` and `build-site --public-url /cast-iron/` (shell `cfcc0682cea0d642`) passed.
  `size-budget`: library 223,172,299 of 234,300,000 bytes, largest file 68,686,066 of 72,300,000,
  WASM 3,340,030 of 3,460,000 (no raise). Specs on `--project=chromium`, one worker:
  `summon-monster-expansion` (1 passed), then `expanded-in`, `active-books` and `books` together
  (50 passed, 1 skipped). The full suite runs in CI.
