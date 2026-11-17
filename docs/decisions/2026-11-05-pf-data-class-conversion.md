---
title: "Pf Data class conversion"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: corpus
---

# Pf Data class conversion, 2026-11-05

- **Every class and prestige-class description record now converts.** The full import moves from 27,853 to 28,496 converted records and from 2,173 to 1,579 findings; the remaining findings are the separately tracked non-class causes. No class or prestige-class record is excluded, and the derived class-archetype rules no longer skip because their host is missing.
- **Class spellcasting summaries are content, stated in our own words.** `::spellAb` and `::cantrips` carry only flags in the OGC JSON (`type`, `stat`, `caster`, `spontaneous`, `trade53`, ...). Upstream's sentences for them live in its GPL renderer (`prebuild/block/spellAb.js`), so porting that template logic or text would break AGENTS.md rule 12. The converter renders the same mechanics as short labelled facts (spell list, key ability, casting, spells known, spell swap, notes). Dropping them as layout would lose the spellcasting rules of every class. `::spellAb` with no braces is valid.
- **Generated bonus macros expand from their documented grammar.** `@B-3y-ninja-x` and its variants become readable prose ("This bonus is equal to one-third of your ninja level"). The converter decodes the upstream grammar but does not copy the renderer. A malformed macro still fails the record.
- **Source-extension attributes on `::ab` carry content.** Leading `x`, `y`, and `z` keys are alternate orders, penalties, patrons, and similar fields. They render under their unprefixed label instead of being rejected; the visible values are preserved.
- **Duplicate presentation metadata is collapsed, not accepted for content.** A repeated `icon`, `order`, `l`, or ability-use flag keeps the last value because upstream uses it for layout and ordering. A duplicate `type` keeps the first value because the later copy is a data defect. Two competing `ability`, `passive`, or other content values still fail.
- **`:::elephant` is a presentation wrapper.** It marks optional Elephant in the Room rules; its contents and source marker are converted, and the wrapper is recorded as `optional-rules-container` coverage. The same pass renders `::list` visible names and `::prereq` prerequisites without the upstream link and list markers.
- **A derived section carries the footnote definitions it references.** `pf_rules` extracts only the section body, so a `[^N]` reference whose `[^N]:` definition sits after the section used to become a dangling `#N` link. The derived entry now appends the definitions it uses, scoped to the marks in that section, so the link resolves inside the derived entry. Schema 3 and search format 1 are unchanged.
