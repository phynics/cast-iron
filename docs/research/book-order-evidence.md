# Printed book order evidence

> **Retired import.** The evidence below still governs `books.toml` in the corpus. The `book-order.toml` and upstream order file it names belonged to the retired import (git tag `psrd-last-buildable`) and no longer exist on `main`.

This record distinguishes publisher evidence from the order present in the PSRD dataset. A rule-tree sequence is useful for grouping records, but does not establish printed chapter order. Only chapter titles and order are recorded here; no book artwork or PDF text is reproduced.

## Core Rulebook

Paizo's [Pathfinder Roleplaying Game Preview #1](https://cdn.paizo.com/blog/pathfinder-roleplaying-game-preview-1), dated May 13, 2009, links a [ZIP containing the contents PDF](https://cdn.paizo.com/download/pathfinder/PZO1110-TOC.zip), `PZO1110-TOC.pdf`. The PDF was downloaded, rendered, and visually checked on 2026-11-03. It verifies the following main chapter sequence. The PDF and its artwork are not included in Cast Iron.

| Chapter | Title | First page |
|---|---|---|
| 1 | Getting Started | 8 |
| 2 | Races | 20 |
| 3 | Classes | 30 |
| 4 | Skills | 86 |
| 5 | Feats | 112 |
| 6 | Equipment | 140 |
| 7 | Additional Rules | 166 |
| 8 | Combat | 176 |
| 9 | Magic | 206 |
| 10 | Spells | 224 |
| 11 | Prestige Classes | 374 |
| 12 | Gamemastering | 396 |
| 13 | Environment | 410 |
| 14 | Creating NPCs | 448 |
| 15 | Magic Items | 458 |

This sequence agrees with the first 15 groups in `book-order.toml`. The remaining `Glossary` group is an upstream aggregate, not a printed chapter title. The PDF lists appendices for Special Abilities, Conditions, Inspiring Reading, and Game Aids, followed by the license, character sheet, and index. The current dataset also places some Creating NPCs text inside Gamemastering and the prestige-class introduction inside Classes. Main chapter order is verified; nested placement and appendix fidelity remain incomplete.

Paizo's [product page for the first-edition Core Rulebook](https://paizo.com/products/btpy88yj?Pathfinder-Roleplaying-Game-Core-Rulebook=) is the corresponding first-edition product listing. It does not provide an accessible first-edition contents listing in the material reviewed. Do not treat the current second-edition Core Rulebook page or its preview as evidence for this book.

## Advanced Player's Guide

Paizo's [first-edition Advanced Player's Guide product page](https://store.paizo.com/pathfinder-roleplaying-game-advanced-player-s-guide/) establishes product identity, edition, release date, and broad included categories, but gives no table of contents. The [Paizo PDF listing](https://paizo.com/products/btpy8fo1/discuss%26page%3D7?Pathfinder-Roleplaying-Game-Advanced-Players-Guide=) was also reviewed; its publicly accessible product text likewise does not expose a contents sequence. No primary-source preview containing the printed contents page was located during this review. Therefore the existing proposed chapter order and labels remain provisional upstream grouping, not verified printed order.

## Other books in the curated list

No complete publisher contents listing was located in this review for Advanced Race Guide, Bestiary, Bestiary 2, Bestiary 3, Bestiary 4, Game Mastery Guide, Monster Codex, Mythic Adventures, NPC Codex, Technology Guide, Ultimate Campaign, Ultimate Combat, Ultimate Equipment, or Ultimate Magic. These sequences must remain explicitly provisional until a publisher contents page or legitimate publisher-hosted preview is checked. Paizo blog previews and product pages exist for some books, but listings of included subjects or a monster alphabetical index do not by themselves establish chapter sequence.

## Page attribution in Pf Data

Pf Data records carry attribution in their description lines, such as `‹SOURCE Inner Sea Gods/12›`, or as a `source="Inner Sea Gods/12"` attribute on a stat directive. The page is publisher evidence for that record in that publication: it is where the record starts in print. It is stronger than a website menu or dataset order, and it is the evidence the canonical book index uses.

It is not chapter evidence. A page number does not name a chapter, prove that two records are in the same chapter, or order records that share a page. A record may name several publications, each with its own page. A range or list such as `12-14` or `12, 14` keeps its first page. A SOURCE line with no page, or a record with no SOURCE line, gives no page evidence; the canonical index puts that entry in an explicit Unpaged section and reports it. A derived child inherits its host's attribution unless its own extracted lines carry a SOURCE line. These limits are why the canonical index starts page-ordered rather than claiming printed chapters.

## Curation rule

Keep match rules and record placement independent from evidence status. When a publisher source verifies only a subset, record precisely that subset and retain a review note for the rest. Never infer printed sequence from SQLite nested-set order, PSRD JSON arrays, alphabetical naming, a third-party review, or a product-description bullet list. The current `book-order.toml` is thus a provisional browsing outline pending source-by-source verification; it must not be described as complete printed chapter fidelity.

## Canonical library

The canonical library (the default `cargo xtask content`) keeps this curated chapter list and order for the same sixteen publications. It does not add, remove, reorder, or rename a PSRD chapter. Instead it re-addresses each placed entry through the reviewed legacy alias map, so the reviewed tree decides the chapter and the canonical id decides the entry. A shared canonical target appears once in a book, at the first placement; a legacy id whose canonical target is not installed is dropped and listed in coverage.

Canonical entries that the PSRD tree does not place for a publication (new Pf Data content) go into one final chapter, labelled `New entries (page order)`. That chapter is ordered only by the page in the entry's source line and is not evidence that the entries belong at the end of the printed book. Publications outside the sixteen PSRD books keep the page-ordered `Page order` and `Unpaged` lists. This mechanism does not upgrade any provisional chapter sequence: the Core Rulebook remains the only publisher-verified order recorded above, and the other fifteen remain provisional.

