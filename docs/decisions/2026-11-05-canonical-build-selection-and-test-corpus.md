---
title: "Canonical build selection and test corpus"
date: 2026-11-05
status: partly-superseded
superseded_by: ["2026-11-05-canonical-library-is-the-default-build"]
issues: []
prs: []
area: corpus
---

# Canonical build selection and test corpus, 2026-11-05

- **The build target is a flag, not a new default.** `cargo xtask build-site` still packages PSRD-Data from `generated/content`. `--canonical` packages `generated/canonical`, the output of `cargo xtask content --canonical`, into the same `dist/`; both accept `--public-url`. The production default does not move until the cutover itself is accepted. (Superseded by the default switch on 2026-11-05.)
- **Tests read the corpus that was built.** `CAST_IRON_CONTENT` selects the generated directory the Playwright expectations load from, and defaults to `generated/content`. `loadCorpus` fails when `dist/content/manifest.json` and that directory carry different content versions, so a canonical build cannot be checked against PSRD expectations (or the reverse) by accident.
- **Specs derive their targets from the corpus.** Entry ids, titles, counts, shard names, publication ids, and chapter paths come from the compiled catalog and `books.json`, never from one dataset's literal route. A book-structure test follows the tree the library actually publishes, including the canonical page-order and Unpaged shape. An entry that several books place is attributed to its first placement in `books.json` order, which is what the reader uses for active-book filtering.
- **Canonical facets come from typed fields.** The PSRD builder captured search facets while parsing upstream HTML. The canonical build now derives school, descriptor, class, spell level, feat type, creature type, CR, size, alignment, item category, slot, class type, trait category, and archetype class from `TypedFields` and the typed archetype edge, so the school, level, CR, and similar filters work on the canonical library. Race groups stay absent: the pin records a race's group only in upstream HTML, and no canonical field carries it.
- **Publication chip totals are one scan, not one per chip.** Search renders a chip for every publication, and each chip rebuilt the whole discovery set to show its active-book total. On the canonical library (397 publications, 29,515 entries) that blocked the search page for about 48 seconds before the index loaded. A memo now computes every source total in one pass per selection revision, and each chip switches to the search's `FilterCounts` when a result arrives. The same page reaches results in about a second offline.
- **`tests/e2e/measure-library.mjs` records local acceptance numbers for one build.** It serves `dist/`, installs it in fresh Chromium and WebKit profiles, and reports install time, storage estimate, first-entry render time, and search latency over 50 queries. The output is local evidence for `docs/status.md`; it is not a CI gate.
