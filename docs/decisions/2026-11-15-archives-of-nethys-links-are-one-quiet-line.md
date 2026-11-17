---
title: "Archives of Nethys links are one quiet line"
date: 2026-11-15
status: current
superseded_by: []
issues: [197]
prs: []
area: ui
---

# Archives of Nethys links are one quiet line, 2026-11-15

Refs [#197](https://github.com/phynics/cast-iron/issues/197). This is the app step after the
optional stored paths (`aon` on an entry and on a publication), the harvest, and the corpus pin.

- **One link per record, and no link without a stored path.** The entry page renders it from
  `RuleEntry.aon`, the book page from `SourceInfo.aon`. A record with no field renders nothing,
  so an unmatched or ambiguous record simply has no link. The app never builds or guesses a URL.
- **Placement is the Metadata role, quieter than the citation it follows.** On an entry the link
  is its own muted line directly under the Sources line, at the end of the entry's own text
  (`.aon-line`, `.meta-link`). It keeps the line's regular weight and muted colour, because the
  underline and the icon already say "link"; `.meta-link` supplies the 44 px target through its
  `::after`, so the visible line stays compact. Because that target reaches 11 px above the text,
  the citation block keeps `space-3` below it when the link follows, so the target cannot cover
  the last book link. On a book page the link sits in the colophon, above the copyright notices.
  Both use the same `views::sources::aon_line` component, so the form cannot drift.
- **The wording never implies official status.** The label is `View on Archives of Nethys`. The
  app is not official and does not claim compatibility (AGENTS.md rule 11).
- **It opens in the device browser, after the tap.** `target="_blank"` with
  `rel="noopener noreferrer"`, an `aria-hidden` external-link icon, and a screen-reader-only
  ` (opens in a new tab)`. Nothing is fetched while the page renders, so offline reading makes no
  request and the guard sees no third-party host.
- **The stored path is prefixed, never re-encoded.** The corpus stores it percent-encoded, so
  `aon_href` puts `https://aonprd.com/` in front and stops; encoding again would turn `%20` into
  `%2520`.
- **Both closing lines own a grid row on a wide entry.** The wide entry is a grid with named
  areas, and an unnamed child was auto-placed into the first free cell: the typed Sources line
  appeared above the body, and the new link landed in the panel column. `.entry > .sources-line`
  and `.entry > .aon-line` now have the `sources` and `aon` rows, in that order, between the body
  and the related panels. Nothing else about the wide layout changes. The revision notice, the
  removed-text sentence, the revision collision list, and the original-text disclosure had the
  same problem (they landed after the footer), so they own the `revision`, `sentence`,
  `collision`, and `original` rows between the header and the contents; a removed revision's
  second `.rich` needs its own row rather than the body's. The disclosure also keeps a `space-3`
  block gap below it, because its summary no longer touches the body's first line once it sits in
  the content column.
- **A revised page follows its visible text.** `effective_entry` already prefers the revision's
  own `aon` and keeps the base's when the revision has none, so the link and the citations agree.
