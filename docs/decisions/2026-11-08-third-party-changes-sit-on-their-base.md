---
title: "Third-party changes sit on their base"
date: 2026-11-08
status: partly-superseded
superseded_by: ["2026-11-09-the-sources-line-follows-the-visible-text"]
issues: [246, 250]
prs: []
area: content
---

# Third-party changes sit on their base, 2026-11-08

Refs [#250](https://github.com/phynics/cast-iron/issues/250), [#246](https://github.com/phynics/cast-iron/issues/246). The `changes` half of slice 3; corpus v0.1.14 holds the data.

- **34 `changes` add-ons in corpus v0.1.14.** Each is a kind `rule` entry `ref/<base>/teitr` from The Elephant in the Room: Feat Taxes in Pathfinder, with one `addon`/`changes` edge to its base. The bases are 3 classes, 23 sorcerer bloodlines (`class_option`), 5 items and 3 rules. The book is third-party, so it is off by default.
- **The base page stacks each eligible change.** One `section.rel-changes` per add-on, expanded like a mythic version: an `h2` `Changes in {book}`, the add-on's title as an `h3` link, then its body with headings demoted and no stat block. The section does not print its own `Sources` line, because the compiled body already begins with the add-on's printed `Sources:` paragraph; printing both would name the book twice inside one section. The heading names the book rather than reading only the book title, so a reader sees that this is a change set and not a second copy of the base.
- **Only the add-on's book gates the section.** An inactive base never hides it, the same rule as a mythic version. With The Elephant off, the base page renders nothing and the add-on is not a search result.
- **The add-on's own page** ends its metadata line with `Changes {book} makes to {base}`, linking the base, plus a muted `Inactive` when the base's book is off. `addon_base_phrase` now takes the add-on's book because only this label puts it in the sentence.
- **Search and lists reuse the `addon` label path.** A `changes` row reads `Changes · {book}` through `addon_name`, with no new row type or lookup.
- **The relation index loads for the kinds a change can attach to.** Class, feat and spell pages already did. Rule, item and class-option pages now load it too, which is 459 KB in corpus v0.1.14; every other kind still skips the file.
- **Chains ignore them.** The existing `addon` exclusion covers `changes`; `Leads to` and the chain marker read prerequisites only.

Refs #250, #246.
