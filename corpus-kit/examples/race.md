+++
# EXAMPLE: a `race` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A race has no typed fields. Its racial traits are separate `racial_trait` entries whose `parent` is this entry.
# Body: a list of traits.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/race/example-hillfolk"
# The entry kind. The typed `[fields]` below must match it.
kind = "race"
# The title shown to readers.
title = "Hillfolk"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]
# Stable labels.
tags = ["standard_race"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

## Racial Traits {#racial-traits}

Hillfolk are sure-footed and stubborn.

- **Ability Score Modifiers:** +2 Constitution, -2 Dexterity
- **Size:** Medium
