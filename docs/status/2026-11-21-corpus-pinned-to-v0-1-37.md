---
title: "Corpus pinned to v0.1.37"
date: 2026-11-21
kind: pin
---

# Corpus pinned to v0.1.37, 2026-11-21

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.37`. The annotated tag object is
`13ea093cc526cfeea00205e2150e3d00b39f988d`; it peels to commit
`76517ea3cda1169892b8fd32eb297c091d984b9b`, the head of `main` when the tag was made. The tree hash is
`f2bd044af4ec9b83107db88780e70da16afa1938a43144297ed2fe628cc764ea` (`cargo xtask corpus-tree-hash` on the
extracted codeload archive). The pin ships two data pull requests on top of v0.1.36 (24 entry files change;
no `id`, alias or file move). No compiler change is part of this pin.

- **Flavor notes of class-option powers** (data #162, closes data #158,
  [#557](https://github.com/phynics/cast-iron/issues/557)). The earlier import wrote editorial notes such as
  "Text from the barbarian class." into the `Flavor` rows of class-option powers. The power layout
  ([#537](https://github.com/phynics/cast-iron/issues/537)) shows `Flavor` without a label, so they read as
  printed text. 15 placeholder rows are removed: at the place where the book prints the power, print has no
  flavor sentence. 6 cross-reference notes now read as print words them ("As per the battle mystery
  revelation of the same name."). 2 cross-reference notes that already match print stay. Each row is in
  `scripts/class-option-power-resolutions.toml` and was checked against print, except Haunt Channeler of the
  reaper mystery (*Pathfinder* #139 was not among the reviewed books): its note is removed and the power
  stays an exception in `power-exceptions.toml`. The data repository adds `scripts/survey-flavor-notes.py`.
  Placeholder notes outside class options are not part of this pin: 115 in archetype entries, 5 in rule
  entries and 2 in class entries (reported in data #162).
- **The anointing rule of *Arcane Anthology* p. 23** (data #164, closes data #163). *Arcane Anthology* p. 22-23
  prints one passage for all hosts. Five entries now carry an `### Anointings` section with
  `**Sources:** Arcane Anthology pg 23` and the printed wording: the Transmutation school (*Core Rulebook*
  p. 82), the Artifice domain (*Core Rulebook* p. 41, *Divine Anthology* p. 22), the Alchemist class
  (*Advanced Player's Guide* p. 26), the investigator Alchemist Discovery talent (*Advanced Class Guide*
  p. 32) and the Transformation patron (*Advanced Player's Guide* p. 70). The host's own book does not print
  the rule; the entries carried a paraphrase of it. Each host takes `arcane-anthology` last in `source_ids`
  with a `[[sources]]` row for page 23, so the count of memberships without a page row does not change.
  `ref/ability/annointings` follows the printed passage word for word. The data repository adds
  `scripts/survey-later-book-rules.py` and `scripts/later-book-rules-reviewed.tsv` (19 candidates surveyed,
  7 fixed, 12 reviewed and left). Two entries stay open for the owner because print is not confirmed
  (`hkdiscipline/summon-devil` prerequisite and the `sorcbloodline/draconic` Linnorm table); they are in data
  #164.

## Checks on this pin

- **Corpus-check result.** `ok`, 0 errors: `corpus ok: 31825 entries, 443 publications, 443 books (16
  curated), 1 licences`. 7 warnings, the same kinds and counts as at v0.1.36: 3 mixed third-party and Paizo
  source warnings (the shadow rats), 1 summary of 2,021 book memberships in 695 entries without a
  `[[sources]]` row (#461), 1 line for Roughrider's `Armor training 4`, 1 summary of 299 of 5,943 trades in
  148 entries that are unresolved, and the class-option power summary (19 of 3,857 excepted).
- **Counts.** 31,825 entries, 443 publications, 443 books, unchanged. Links: 147,298 resolved, 0 out of
  selection, 0 broken. The library schema stays 6 and the search index format stays 2.
- **Guards.** `cargo xtask content` passed the published-id guard and the book-slot guard (443 slots).
  No new book, so `cargo xtask published-books` and `published-ids` were not run and neither file changes.
  A second compile gave the same content version.
- **Specs.** No spec asserts a changed Flavor text, the five hosts' bodies or an *Arcane Anthology* count.
  `class-option-powers.spec.ts` reads Abjuration, a mystery's Prereq and Gained rows and layout cases, none
  of which this pin changes; `class-card.spec.ts` reads the alchemist's typed spellcasting row, which does not
  change. No spec changed.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` name the pinned
  corpus v0.1.37. No legal claim changed: no source, notice or `legal.toml` row is touched, and
  `notice_pending` is unchanged. The AoN coverage report is unchanged apart from its pin lines.
- **Local verification** (nothing deployed), on `origin/main` at `80d0cf0c9` plus this pin. Content version
  `v28481793019cf75c` (was `v8f589fddba1bb56a`). `build-site --public-url /cast-iron/` passed (shell
  `f03086b6a336dd0b`). `size-budget`: library 224,621,356 of 234,300,000 bytes (95.9%), largest file
  69,024,585 of 72,300,000 (95.5%), WASM 2,780,794 of 3,460,000 (80.4%; 3,449,043 before, the drop is the
  `opt-level = "z"` WASM profile of #565, not this pin); no limit raised. Spec `class-option-powers.spec.ts`
  (the class-option power layout of the verify skill's feature map) on `--project=chromium`, one worker: 7
  passed. The full suite runs in batch CI on `main`.
