---
title: "List-page template and scoped Quick search"
date: 2026-11-06
status: partly-superseded
superseded_by: ["2026-11-23-entry-breadcrumbs-and-tabs"]
issues: [193]
prs: [1, 2]
area: search
---

# List-page template and scoped Quick search, 2026-11-06

Refs [#193](https://github.com/phynics/cast-iron/issues/193), PR 1 of 5. A design council (Muse, Gemini, Opus) specified the list pages; the owner's decisions below replace the parts the council wrote before Quick search existed.

- **A kind page is a title, a count line, the filters, a letter bar and rows.** The breadcrumb, the lede ("Feats by type…") and the `Search {kind}` link are gone from kind pages, and so is the `Home` crumb on Feat chains and the spell pages. Each stays reachable: Home through the tab bar and the brand; the blurb through the category cards on Home; the search through the header icon, which is now scoped. The filter area (the lens pills) is unchanged until PR 2, and the letter bar now follows it. The "All, A to Z" heading stays for screen readers only.
- **The count line names the kind.** `3583 feats`, `1 feat`, `40 prestige classes`. The owner prefers the kind word to "entries". The line no longer says "in your active books"; the numbers are still totals for the active books.
- **The breadcrumb rule.** A breadcrumb lists only the pages between a tab destination and the current page, never Home, Books or Settings, and none appears directly under a destination. Entry pages and a book's own pages (the book list and the reader) keep their `Home` and `Books` crumbs for now: the Books pages belong to another change (#154), and entry crumbs move in a later PR of #193.
- **Owner decision: the scope is a removable pill inside Quick search.** The council wanted a header link shaped like a field that navigates to the results, and "a scope is never a token". The owner chose Quick search itself, with a scope that is obvious and easy to remove. On a kind page the icon opens the layer with `Search feats` as the field's name and placeholder, and a pill (`Feats` with a cross, accessible name `Remove scope: Feats`, 44 px) under the field. Removing it searches everything and returns focus to the field. The council's other rules stay: a scope is derived from the open page at that moment and never stored, and the header never holds a second control.
- **One scope type serves both places** (`views/scope.rs`): the kind, optionally with facet values, plus the labels, the engine filters and the results route. Quick search takes it from the page (`Scope::of_page`); the results page takes it from the route (`Scope::of_search`, exactly one kind). Suggestions use its filters. Enter, "See all results" and a recent search open `#/search` with the same kind (and facets) applied.
- **The results page shows the pill too.** Above the existing category row, for any search limited to exactly one kind. Removing it clears the kind and the facets, keeps the text, and moves focus to "All entries" (which becomes pressed) so a phone's keyboard does not rise. The category row stays; the pill adds the one-tap removal and covers kinds the row does not offer (traits, items).
- **The results field is not focused on arrival.** The council asked that arriving from a list's header link focus the field regardless of pointer type. Quick search now does the typing, so a reader arrives with text and results already; focusing then would raise a phone's keyboard over the results. The existing rule (fine pointers only) stays.
- **Only kind pages scope the search for now.** Feat chains, the spell hub and spell lists keep an unscoped Quick search; they take a scope when their filters arrive (PRs 2 to 5). A facet in the scope is supported by the type and the labels (`Search Combat feats`), and nothing sets one until PR 2.
- **Forward definitions in DESIGN.md.** The Filter menu and `.page-link` roles are written down now, marked "(arrives in PR 2)", so PR 2 builds to a settled form.
- **Digits are grouped app-wide** (own commit in the same PR, so it can be reverted alone): `3,583`, not `3583`. A count of four digits or more is easier to read with a separator. The app has no locale setting, so the separator is always a comma. One helper, `fmt::count`, writes every count the reader sees: category and letter counts, the kind count line, filter and chip counts, search result counts, spell, chain and backlink counts, the library size in Settings and the installer, and the coverage figures on the legal page. Book views and the Books directory keep their ungrouped counts for now, because another change owns them (#154). This is the orchestrator's recommendation, recorded in #193 as open.
