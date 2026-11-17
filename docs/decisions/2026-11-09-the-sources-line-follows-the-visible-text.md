---
title: "The Sources line follows the visible text"
date: 2026-11-09
status: current
superseded_by: []
issues: [357]
prs: [371]
area: ui
---

# The Sources line follows the visible text, 2026-11-09

Refs [#357](https://github.com/phynics/cast-iron/issues/357). This is the rendering step after
[#371](https://github.com/phynics/cast-iron/pull/371). The data migration is
`phynics/cast-iron-data#64`; this PR must merge before that data is tagged or pinned.

- **One muted line at the bottom of the entry's own text.** A typed citation renders as
  `{Publication} p. {page}`, joined with `; ` in printed order. `p. {page}` appears only when the
  citation has a page. The line sits after the body and above the related panels. The formatting
  is the pure `views::sources` helper.
- **Book names link to the book page when the library has one.** `#/browse/book/<id>` is used only
  when the catalog knows the publication and the library has `books.json`. A library without book
  pages keeps plain titles, as its metadata line already does. That library-level test is enough:
  a publication that is not in the catalog keeps its id plain.
- **Unconverted bodies are untouched.** An entry with no rows renders exactly as before. Its
  leading `**Sources:**` paragraph stays in the body, so both the current pin and migrated data
  read correctly. The app never rewrites or removes a body paragraph.
- **A revision's citations follow its text.** `effective_sources` takes the revision's rows when
  it has any, otherwise the base's. `effective_entry` uses the same rule, so a revised page shows
  the citation of the text on screen, while an unconverted revision does not blank a converted
  base page. The original-text disclosure ends with the base's own line. A removed or merged page
  shows the revision's citation under the printed sentence.
- **Add-on sections own their line.** A mythic, changes or Stamina section shows the add-on's
  typed rows at the end of its own section. The old #318 rule ("the changes body already starts
  with its printed `Sources:` line, so print none") no longer holds once the migration removes
  that paragraph: the typed line is printed when rows exist. An unconverted body still carries its
  paragraph, and with no rows nothing is added, so the two cases cannot double-print. A mythic
  version never had a printed citation, so it falls back to its book name when it has no rows.
- **No raw markup or new JavaScript.** The line is built from typed nodes and `Route` links.
