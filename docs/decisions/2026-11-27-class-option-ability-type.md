---
title: "The ability tag of a single-power class option is a typed field"
date: 2026-11-27
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# The ability tag of a single-power class option is a typed field, 2026-11-27

A single-power class option (about 1,597 entries, such as Agony, Advanced Player's Guide p. 67) has no
body head that repeats its title (owner ruling Q1 of #540). The printed ability tag, `(Ex)`, `(Su)` or
`(Sp)`, was on that head. Owner ruling 63 keeps it, at entry level, so that the page header shows
"Agony (Su)". The app thread chose how to store it.

## Decision

- **(b) A typed field**, `ability_type`, on `kind = "class_option"` entries: `[fields] ability_type = "Su"`.
  The value is the printed text inside the parentheses, as printed (`Ex or Sp`, `Su, Sp`). The format is
  in `docs/corpus-format.md` ("Ability type of a class option"). `corpus-check` accepts the value only when
  the tag reader of the schema (`power::split_ability_tag`) would read it as a tag, and refuses the key on
  any other kind.
- **(a) A tag in the title** (`title = "Agony (Su)"`) is rejected. The title is the sort key, the search
  text, the link text and the string of every list row. A tag in it puts "(Su)" into each of those: Agony
  would sort and match as "Agony (Su)", every link to it would read "Agony (Su)", and 1,597 titles would
  differ from the names that the book's other pages use. Existing rows that show a tag (the Options
  section) would have to strip it again. Search, sort and links stay clean with a field.
- **The key is additive.** `SCHEMA_VERSION` stays 7: `ClassOptionFields::ability_type` is `Option<String>`
  with `skip_serializing_if`, so a library without a typed option is byte-identical, and no type of the
  schema crate denies unknown fields, so an older client ignores the key. The corpus format number stays
  at 4.
- **The page header** shows the value after the title, in parentheses, in the muted italic of the tag of a
  feature head (`2026-11-27-features-in-print-order-as-heads`): `Agony (Su)`. The parentheses belong to the
  display; the stored value has none. The accessible name of the `h1` includes it ("Agony (Su)"), as it
  includes the printed name of a feature head with its tag, because the tag is part of what the book
  prints and a reader who hears the name should hear the type. The title of the entry, the star's name,
  the browser title and the search results do not carry it.
- **The Options section** of a class page shows the tag of each row from the field when the entry has it,
  and from the body otherwise.
- **No search alias** is added: the title stays clean and the tag is not searchable text.
