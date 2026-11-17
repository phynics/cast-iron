---
title: "One file per decision and per status entry"
date: 2026-11-17
status: current
superseded_by: []
issues: [462]
prs: []
area: process
---

# One file per decision and per status entry

Closes [#462](https://github.com/phynics/cast-iron/issues/462). It carries out M3 of the 2026-11-15
architecture review.

- **Decisions.** `docs/decisions.md` held 148 sections in one file, with no index, a mixed order, a
  duplicate heading and no superseded markers, and nearly every pull request appended to its end, so two
  open pull requests always conflicted there. Each section is now `docs/decisions/YYYY-MM-DD-<slug>.md`.
  The text is unchanged, except for the relative links that moving a file broke and one renamed duplicate
  heading (the second `Install button`, a second copy of the first). Front matter holds `title`, `date`,
  `status`, `superseded_by`, `issues`, `prs` and `area`. Dates come from the heading; for the sections
  without one, from the commit that added the heading.
- **Superseded entries.** A status other than `current` rests on an explicit statement in a later entry
  (the 2026-11-17 membership entry lists what it replaces; the 2026-11-16 owner decisions, the
  2026-11-03 simplification, and the "Superseded" notes inside entries) or on a known replacement. Where
  the text did not settle it, the entry stays `current`. The text of a replaced entry is not edited.
- **Index.** `cargo xtask decisions-index` writes `docs/decisions/README.md`, grouped by area. `cargo xtask
  check` fails when the index is stale, when front matter is invalid, and when a relative link in an entry
  does not resolve. `docs/decisions.md` stays as a short pointer, because entries and older notes cite it.
- **Status.** `docs/status.md` holds only the current state. Its 45 sections (pins, releases and
  acceptance records) are `docs/status/YYYY-MM-DD-<slug>.md`, one file each, with a generated index
  (`cargo xtask status-index`). One file per entry, not one `docs/status-log.md`: a single log would be the
  shared file every pin pull request edits, which is the conflict this change removes.
- **Merges.** The two indexes are generated files that two pull requests each change. `.gitattributes` marks
  them `merge=union`, so Git keeps the rows of both sides, and `cargo xtask check` fails until the
  index is regenerated, if the union leaves it out of order.
- **How to add one.** Add a file; run `cargo xtask decisions-index` (or `status-index`). The migration
  script `scripts/split-docs.py` stays in the repository: its `verify` command proves that no text was lost.
