---
title: "Canonical legal screen and alias retirements"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: licensing
---

# Canonical legal screen and alias retirements, 2026-11-05

- **Section 15 is the union, and coverage is judged against it.** The canonical `legal.json` carries PSRD's Section 15 notices, every distinct Pf Data notice that matched an in-scope publication (all printings), and both dataset pins. A publication counts as covered when either list matches it. This matters: Pf Data's own notice file omits the Advanced Race Guide, but PSRD's list covers it, so its 659 records stay under the PSRD notice. Only publications with no notice in either list drive exclusion.
- **A record needs one covered publication.** Records whose publications all lack a Section 15 notice are rejected and listed in `coverage.json` with the publication slugs and the reason; a record with at least one covered publication stays, and `source_ids` keeps only the covered publications. On the pinned corpus this rejects 1,292 records, dominated by Bestiary 5/6 and setting books absent from both lists.
- **References to a rejected record stay readable.** A kept record may link to a rejected one. Flattening such a link to its visible text (or dropping an unresolvable parent, relation, or feat prerequisite) is the only outcome compatible with both rules: the host has a valid notice and must stay, and the target cannot be redistributed. The count is reported under `legal-reference-settlement`; nothing disappears silently. The same pass is why `validate_library` reports no dangling link, parent, or relation.
- **The alias map never points outside the installed library.** `aliases.json` is generated in process from the reviewed map and the same catalog the alias gate uses. A row whose canonical target is rejected by the legal screen is emitted under `retired` with reason `rights`; a row whose planned target the converter did not emit is emitted with `unrepresentable` and a note. On the pin this retires 174 unrepresentable rows and no rights rows. A target that is missing for any other reason still fails the build.
- **Package the alias file, do not append it.** `aliases.json` is part of the library's file set before the version hash is computed, so `content_version` covers it and an interrupted publication cannot leave a manifest without its map. This replaces the earlier `publish_extra_file` path for the canonical library; the PSRD default still has no alias file.
- **Coverage carries every accounting class.** `coverage.json` includes the PI screen totals, converter coverage and exclusions, carry-over counts and per-book rows, legal-reference settlement, alias retirements, and the out-of-scope inventory rows. `coverage.md` renders the same rows for review.
