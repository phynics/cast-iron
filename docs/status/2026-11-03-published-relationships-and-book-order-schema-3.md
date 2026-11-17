---
title: "Published relationships and book order, schema 3"
date: 2026-11-03
kind: record
---

# Published relationships and book order, schema 3

Content version `vdb8e844bfc068446`, schema 3. Verified locally: `cargo xtask check` passes; two `cargo xtask content` runs give the same tree hash; `scripts/check-library-upgrade.py` finds all 9,112 earlier ids and titles with no kind change and no dangling reference in the three new files; the full Chromium, WebKit, and mobile browser suite passes against the built site (47 tests). The five relationship/navigation tests also passed with the browser offline in Chromium and WebKit (10 checks). Desktop and mobile screenshots in both themes were inspected; the screenshot run reported no console errors or horizontal overflow. Data findings: `docs/research/relationship-data.md`. Decisions: `docs/decisions.md`.

| Measure | Value |
|---|---|
| Entries | 9,967 (was 9,112): 218 archetypes, 342 racial traits, 295 traits |
| Relations | 1,531 edges: 342 `part`, 218 `archetype`, 971 `prerequisite` |
| Feat prerequisites | 1,002 feats with wording; 597 name a feat; 969 of 1,298 name-like requirements resolve (74.7%) |
| Spell lists | 23 class names, 8,010 distinct pairs (8,504 upstream); 8 names have no class entry: Arcanist, Bloodrager, Elementalist Wizard, Hunter, Investigator, Shaman, Skald, Warpriest |
| Book view | 16 books, 9,967 entries placed, 0 unplaced; Core Rulebook main chapter order verified against the publisher PDF, other outlines and nested placement remain drafts |
| Download | 61,827,011 bytes raw (was 58,487,268, +5.7%); largest shard unchanged at 524,249 bytes |
| New files | `relations.json` 173,675 (14,491 gzip); `lists.json` 128,922 (28,157); `books.json` 1,067,959 |

The client reads schemas 1, 2, and 3. It now uses the new files for racial traits, archetypes, feat links and chains, class spell lists, book contents, and previous/next book navigation. Category browsing has dedicated shortcuts and facets; source filters remain part of the search URL. Missing schema-3 files in an older library offer an update path. Book contents display source evidence and review warnings.

AoN source pages group database records rather than chapters. Legacy PRD section links are captured in a reproducible 18-menu, 167-link inventory, including sections for books absent from the current corpus. Details: `docs/research/aon-book-sections.md`, `docs/research/legacy-prd-sections.json`, and `docs/research/book-order-evidence.md`.
