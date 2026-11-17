---
title: "Books sorting, contents tree, and publication metadata"
date: 2026-11-05
status: partly-superseded
superseded_by: ["2026-11-06-a-book-row-opens-the-book-page", "2026-11-07-books-tab-and-book-page"]
issues: []
prs: []
area: ui
---

# Books sorting, contents tree, and publication metadata, 2026-11-05

- **Books has three URL-owned orders.** `#/books` keeps library order, `?sort=category` groups by publication category, and `?sort=date` groups by publication year newest first. The sort is a hash query, so Back and Forward restore it and every state is bookmarkable. Sorting does not change the per-book Active control or the catalog totals.
- **Category and date are reviewed metadata, not guesses.** The pinned Pf Data registry has only a title and a product URL; `catalog.json`, `books.json`, and `publications.toml` carry no product line or release date. `sources/canonical/publication-metadata.toml` records both from the Archives of Nethys source catalogue, whose product-line pages group titles and show an Estimated Release Date. `scripts/extract-publication-metadata.py` regenerates the file from saved pages; the committed file is the reviewed compiler input. The fields are optional additions to `SourceInfo`, skipped when empty, so `SCHEMA_VERSION` stays 3 and a catalog or reader from before the change still works.
- **Unknown is visible.** A publication with no reviewed category or date is never dropped: category order puts `Unknown category` last and date order puts `Unknown date` last. The row keeps its Active control and contents tree. A build with no reviewed file at all (the compiler fixture corpus) still succeeds and every publication is Unknown; a malformed file still fails the build.
- **Book expansion is the real contents tree.** *(Superseded on 2026-11-06: the Books list no longer expands; see "A book row opens the book page".)* The Books directory reuses `books.json` instead of a flat chapter list. Its root is the `Read {book}` link, chapters and sections are native `<details>` disclosures, and entries are links. Only an open branch builds its children, so a two-thousand-entry book stays cheap; the book page and the directory share the same order and labels.
