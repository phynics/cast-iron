---
title: "One GameMastery Guide book record"
date: 2026-11-09
status: current
superseded_by: []
issues: [359, 362]
prs: []
area: content
---

# One GameMastery Guide book record, 2026-11-09

Refs [#362](https://github.com/phynics/cast-iron/issues/362), part of [#359](https://github.com/phynics/cast-iron/issues/359).

- **The two book trees are one publication.** The corpus exported the printed-chapter tree under `gamemastery-guide` and a dataset-topic tree under the publication id `game-mastery-guide`. `publications.toml` has one row, `game-mastery-guide`, and every GameMastery Guide entry names it. Decision [#423](#canonical-publication-registry-2026-11-05) keeps the PSRD source id, so the printed tree moves to `game-mastery-guide` and the second tree is dropped. The split came from the import, not from the book: `history.toml` records both source ids in its frozen book-order results.
- **Only the second tree's 111 records needed a new place.** The printed tree already placed the other 614. Of the 111, 81 are NPC stat blocks, 22 are drugs, hazards, and insanities, and 8 are rules. None carries a GameMastery Guide page line, so `scripts/gmg-nested.py` places each at the end of the printed section its dataset group names. The script is idempotent: `--check` accepts the merged tree.
- **The old book id still resolves in addresses and in stored state.** `crates/web/src/book_source.rs` holds one retired-book table. `crates/web/src/route.rs` uses it for `#/browse/book/<source>`, chapter pages, the legacy `#/browse/source/<source>` address, and an entry's `?book=` context; the canonical address is `#/browse/book/game-mastery-guide`. The same table migrates the two stores that also hold a book id. `store::resolve_continue_points` maps `continue_points.source`, and keeps the newest point when both old ids have one. `disabled_books` is rewritten once, when the installed library no longer has the retired tree: the merged book is disabled only when the reader had disabled the printed tree (`gamemastery-guide`), alone or together with the topic tree; a disable of the topic tree alone is dropped because the printed tree stayed active. A marker (`book_source_migration`) keeps that rewrite from running again and undoing a later choice. The map is one retired-book row, not a general alias file: `BookIndex` has no alias field, so the client is the only layer that can resolve the id without a format bump. Unlike an entry id, a book id is not a bookmark key, but it does persist in these two stores, so the client must migrate it there as well as in URLs.
- **No entry id changed.** The published-id guard still resolves every id, so the merge moves only book nodes.
