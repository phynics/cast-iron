---
title: "Corpus pinned to v0.1.36 (corpus format 3)"
date: 2026-11-21
kind: pin
---

# Corpus pinned to v0.1.36, 2026-11-21

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.36`. The annotated tag object is
`fca7c8d4dfd56be55ffce1207f009d0dead04322`; it peels to commit
`8344c87b2093a6fcfe69e2fff4a4a923074d4deb`, the head of `main` when the tag was made. The tree hash is
`e2bd17276e53c4d5058c35573b62d09d522820e21f2cafc1e603757979e28995` (`cargo xtask corpus-tree-hash` on the
extracted codeload archive). The pin ships two data pull requests on top of v0.1.35. The app reads and
derives format 3 since [#478](https://github.com/phynics/cast-iron/pull/478); no compiler change is part of
this pin.

- **Corpus format 3** (data #159, [#425](https://github.com/phynics/cast-iron/issues/425), decision
  `2026-11-17-page-order-books-are-derived-corpus-format-3`). `migrate-layout` cut `books.toml` (4.35 MB)
  into 16 curated `books/<publication-id>.toml` files (1.76 MB), removed `books.toml` and set
  `format = 3`. The other 423 page-order books are not stored: the compiler derives them from each entry's
  `[[sources]]` pages. With the 4 publications that had no book before, 427 books are derived and 16 are
  curated, 443 in all. No entry file, `id`, alias or `[[sources]]` row changes.
- **Apply scripts** (data #160). `membership-apply.py` and `expansion-credits-apply.py` edit the curated
  `books/<publication-id>.toml` (or nothing, for a derived book); the one-shot scripts of earlier
  migrations are marked historical.

## Differences against the stored books

Data #159 compared the stored books with the derived ones (its `scripts/books-format-3-equivalence.py`).
291 of 443 books have no difference; the other 152 differ in these classes, each reviewed:

| Class | Count | Books |
|---|---:|---:|
| Entry was `Unpaged`, now `Page order` (its `[[sources]]` row carries a page the old list did not use) | 258 | 15 |
| Variant ordered by its own page instead of after the host's page | 31 pairs | 19 |
| Tie on one page, ordered by kind, title, id | 155 pairs | 71 |
| Order inside `Unpaged` | 77 pairs | 53 |
| Entry now placed (61 `ref/domain/*`, 2 Bestiary 3 rot grubs in Pathfinder #25) | 63 | 18 |
| Book new to the library (`paizo-blog-ultimate-cantrips`, `paizo-blog-golarion-day-subdomains`, `paizo-blog-the-gauntlet-will-be-ours`, `hodgepodge-from-the-grand-lodge`) | 14 placements | 4 |
| Book title | 15 | 15 |
| Node title (76 The Elephant in the Room revisions lose `(The Elephant in the Room)`; 3 People of the Stars telescopes read `Telescope (×10)`) | 79 | 2 |
| Chapter shape (`Unpaged` gone in Blood for Blood, The Frozen Stars, Into the Shattered Continent, Seers of the Drowned City) | 4 | 4 |
| Nesting (The Elephant in the Room `Ranger` grouping row) | 1 | 1 |

No curated override is lost and no entry leaves a book. 26 entries that had no placement in any book now
have one (31,799 to 31,825 placed). Every file of the library is identical to the v0.1.35 build except
`books.json`, `search.bin` and `manifest.json` (data #159). `Game Mastery Guide` (curated) moves from
position 123 to 7 in the book order.

### The 15 derived titles

14 follow the format 3 decision; a slug or `Pathfinder #125: …` becomes the publication title. The 15th is
not in the decision: `magic-of-thassilon`, `Magic of Thassilon` becomes `More Magic of Thassilon`, the
publication title after the 2026-11-18 publication correction. The new title sorts the book later, past
Monster Summoners Handbook (data #159).

| Book | Was | Now |
|---|---|---|
| `carrion-crown-players-guide` | `carrion-crown-players-guide` | Carrion Crown Player's Guide |
| `council-of-thieves-players-guide` | `council-of-thieves-players-guide` | Council of Thieves Player's Guide |
| `giantslayer-players-guide` | `giantslayer-players-guide` | Giantslayer Player's Guide |
| `hells-rebels-players-guide` | `hells-rebels-players-guide` | Hell's Rebels Player's Guide |
| `ironfang-invasion-players-guide` | `ironfang-invasion-players-guide` | Ironfang Invasion Player's Guide |
| `kingmaker-players-guide` | `kingmaker-players-guide` | Kingmaker Player's Guide |
| `magic-of-thassilon` | Magic of Thassilon | More Magic of Thassilon |
| `mummys-mask-players-guide` | `mummys-mask-players-guide` | Mummy's Mask Player's Guide |
| `pathfinder-125-tower-of-the-drowned-dead` | Pathfinder #125: Tower of the Drowned Dead | Pathfinder #125 Tower of the Drowned Dead |
| `return-of-the-runelords-players-guide` | `return-of-the-runelords-players-guide` | Return of the Runelords Players Guide |
| `rise-of-the-runelords-anniversary-edition-players-guide` | `rise-of-the-runelords-anniversary-edition-players-guide` | Rise of the Runelords Anniversary Edition Player's Guide |
| `serpents-skull-players-guide` | `serpents-skull-players-guide` | Serpent's Skull Player's Guide |
| `strange-aeons-players-guide` | `strange-aeons-players-guide` | Strange Aeons Player's Guide |
| `tyrants-grasp-players-guide` | `tyrants-grasp-players-guide` | Tyrant's Grasp Player's Guide |
| `wrath-of-the-righteous-players-guide` | `wrath-of-the-righteous-players-guide` | Wrath of the Righteous Player's Guide |

(Compiled here from the v0.1.35 and v0.1.36 corpora with the same compiler: `books.json` titles differ in
exactly these 15 of 439 and 443 books.)

## One legacy address re-pointed

Format 3 drops the stored `Ranger` grouping row in The Elephant in the Room, so its legacy address
`?a=toc-1-1` (stable anchor `row-h-unpaged--h-ranger`) fell back to the chapter, the one row of
`sources/legacy-sections.json` that lost its row. By owner decision of 2026-11-21 the table now maps
`toc-1-1` to `row-ref-rule-class-ranger-ranger-teitr`, the Ranger revision row that `toc-1-1-1` already
named. It is the only change to the table (one value), recorded in decision
`2026-11-21-legacy-ranger-address-opens-the-revision-row`. A native test
(`the_old_ranger_grouping_address_of_the_elephant_in_the_room_opens_the_ranger_revision_row`,
`crates/content-compiler/src/legacy_sections.rs`) proves that both `toc-1-1` and `toc-1-1-1` resolve to the
Ranger revision row on the pinned tree.

## Checks on this pin

- **Corpus-check result.** `ok`, 0 errors: `corpus ok: 31825 entries, 443 publications, 443 books (16
  curated), 1 licences`. 7 warnings, the same kinds as at v0.1.35: 3 mixed third-party and Paizo source
  warnings (the shadow rats), 1 summary of 2,021 book memberships in 695 entries without a `[[sources]]`
  row (#461), 1 line for Roughrider's `Armor training 4`, 1 summary of 299 of 5,943 trades in 148 entries
  that are unresolved, and the class-option power summary (19 of 3,857 excepted).
- **Counts.** 31,825 entries, 443 publications, 443 books, unchanged; 31,825 placed (31,799 before).
  Links: 147,298 resolved, 0 out of selection, 0 broken. The library schema stays 6 and the search index
  format stays 2.
- **Routes** (`cargo xtask content` compile notes). Legacy section addresses: 6,794 of 6,795 still open their
  row, the other 1 opens its chapter (the same as v0.1.33; it was 6,793 before the re-point). Legacy Contents
  addresses (`?c=`): 412 of 413 chapter rows still open their chapter, the same as on v0.1.35 (compiled
  here for the comparison). Published ids: the published-id guard passed, so every id of
  `sources/published-ids.txt` resolves. Share slots: the book-slot guard passed with the 443 slots of
  `sources/published-books.txt`; the 4 new books already had theirs, so `cargo xtask published-books` and
  `published-ids` were not run and neither file changes.
- **AoN coverage.** The pin lines of `docs/research/aon-path-coverage.md` changed; the counts stay 12,484 of
  31,825 entries with an AoN path and 426 of 443 publications with a source page.
- **Specs.** No spec asserts a changed title or order. `changes.spec.ts` and `revisions.spec.ts` name the
  entry titles `Unchained Monk (The Elephant in the Room)` and the book title, which this pin does not
  change (only the Contents row titles of the book change); `books.spec.ts` reads publication titles and the
  corpus; `search.spec.ts` computes the `Unpaged` chapter of Blood of the Beast from the compiled books.
  No spec changed.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` name the pinned
  corpus v0.1.36. No legal claim changed. `notice_pending` is unchanged.
- **Local verification** (nothing deployed), on `origin/main` at `27e3209f5` plus this pin. Content version
  `v8f589fddba1bb56a` (was `v52638e804064b300`). `build-site --public-url /cast-iron/` passed (shell
  `d769703e8126d398`). `size-budget`: library 224,620,596 of 234,300,000 bytes (95.9%), largest file
  69,024,540 of 72,300,000 (95.5%), WASM 3,449,043 of 3,460,000 (99.7%), the WASM size of v0.1.35; no limit
  raised. Spec `search.spec.ts --grep "book section"` (the book Contents and section feature of the verify
  skill's feature map) on `--project=chromium`, one worker: 4 passed. The full suite runs in batch CI on
  `main`.
