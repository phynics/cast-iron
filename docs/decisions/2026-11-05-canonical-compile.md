---
title: "Canonical compile"
date: 2026-11-05
status: partly-superseded
superseded_by: ["2026-11-05-canonical-library-is-the-default-build", "2026-11-06-the-upstream-import-is-removed"]
issues: []
prs: []
area: corpus
---

# Canonical compile, 2026-11-05

- **One builder, one packager, two sources.** `pipeline::build_canonical` runs the screened Pf Data inventory through `canonical::convert_corpus`, applies the legal screen, builds typed relations, carries over the PSRD entries named by `id-map.toml`, merges by id, and builds books, lists, legal, search aliases, coverage, and the alias file. `pipeline::package` is unchanged: the canonical library is a schema-3 shard/catalog/search/manifest set like the PSRD one.
- **`cargo xtask content --canonical` writes `generated/canonical/`.** The default `cargo xtask content` still writes `generated/content/` from PSRD-Data and remains byte-identical to `origin/main`. Canonical reports go to `generated/reports/canonical/`. (Superseded by the default switch on 2026-11-05: canonical now owns `generated/content/` and PSRD is `--psrd`.)
- **Books use page attribution; carry-over uses its PSRD book.** `books::build_canonical` orders Pf entries by their `‹SOURCE book/page›` line. Carried entries have no page, so they join their PSRD book's Unpaged section with the PSRD display title. Entries with no page remain visible under Unpaged and in `book_order.unplaced`; neither is a drop.
