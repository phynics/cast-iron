---
title: "The class-option group is a search facet"
date: 2026-11-22
status: current
superseded_by: []
issues: [578, 594]
prs: []
area: search
---

# The class-option group is a search facet, 2026-11-22

Refs [#578](https://github.com/phynics/cast-iron/issues/578) and the survey
[#594](https://github.com/phynics/cast-iron/issues/594). The owner accepted #578 on 2026-11-22.

- **The compiler emits `group`.** A class option carries the slug of its `option-groups.toml`
  group as the facet `group`. It is additive: the index layout is unchanged, so
  `INDEX_FORMAT_VERSION` stays 2 and `SCHEMA_VERSION` does not change (precedent:
  `2026-11-10-race-groups`). The facet reaches a reader with the next library update.
- **A group is a scope like any facet.** Quick search, the results page and the class-option list
  read `group` from the index, as they read every other facet. This replaces the rule of
  `2026-11-08-class-options-in-the-app` that groups come from `option-groups.json` and that a
  search scope drops the group. `option-groups.json` stays for labels (a slug such as
  `arcane-discovery` does not print as `Arcane discoveries`), for the class page's `Class options`
  links and for the `{Group singular}` of a row.
- **Old libraries keep today's behaviour.** An installed library compiled before this change has
  no `group` field in its index. The client detects that by the absence of the field in the facet
  catalog, not by a version check: the scope then drops the group and the list filters through
  `option-groups.json`. An address that names the group (a link from a newer library) opens the
  results page without it: the page replaces its address once the index shows it has no `group`
  field.
- **A group's label comes from the library.** The label is registered when the library reads
  `option-groups.json`, so a scope pill built from a route reads like the menu. Before it is
  registered the slug reads as capitalised words.
- **A group's label names the kind.** The scope of a group reads `Search Rage powers`, without
  the noun `class options`, as an item category (`Search Rods`) does.
