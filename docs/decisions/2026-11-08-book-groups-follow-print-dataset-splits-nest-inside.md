---
title: "Book groups follow print; dataset splits nest inside"
date: 2026-11-08
status: current
superseded_by: []
issues: [13]
prs: []
area: content
---

# Book groups follow print; dataset splits nest inside, 2026-11-08

Refs [#13](https://github.com/phynics/cast-iron/issues/13). Owner decisions recorded in the issue
on 2026-11-08. The corpus edit is `phynics/cast-iron-data`; this app repository records the rule.

- **Top-level groups are the printed chapters and appendices.** A book's `chapters` list follows
  the printed contents page for title, order, and grouping. A dataset chapter that print does not
  have does not stay at the top level.
- **A dataset split nests inside its printed chapter.** Base Classes and Core Classes stay inside
  Classes (Advanced Player's Guide), Story Feats inside Character Background (Ultimate Campaign),
  Class Archetypes inside Classes (Ultimate Combat), and New Familiars inside Mastering Magic
  (Ultimate Magic). Each keeps its dataset title and its entries.
- **A dataset hub entry opens the first printed section it links to.** Where the dataset has one
  aggregate entry that print splits or renames, the aggregate becomes the opening entry of the
  first printed chapter it links to, and the linked sections follow as its children or as sibling
  chapters. This keeps every entry in the book tree while the printed title governs the
  top-level label: the Core Rulebook Glossary opens Special Abilities, the Bestiary
  Miscellaneous Monster Rules opens Monsters as PCs, the Advanced Race Guide Age, Height & Weight
  opens Age, and Ultimate Magic Updates becomes Appendix.
- **A printed chapter with no entries gets no group.** It is named in
  `docs/research/book-order-evidence.md` instead, so an empty group never suggests the content
  exists. This covers the Core Rulebook's Inspiring Reading and Game Aids, the Advanced Race
  Guide's Spells appendix, the Bestiary's Appendices 3, 7 and 9-12, and the Game Mastery Guide
  Appendix.
- **Nested placement is a separate check.** Matching the top-level groups does not verify the
  sections inside a chapter. Each book's nested placement gets its own issue, Core Rulebook first.
