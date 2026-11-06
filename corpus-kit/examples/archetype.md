+++
# EXAMPLE: a `archetype` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# An archetype. The `archetype` relation names its class; `ref/arc-<class>/...` is the id family for archetypes.
# Body: a list of alternate class features.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/arc-wayfarer/example-pathfinder"
# The entry kind. The typed `[fields]` below must match it.
kind = "archetype"
# The title shown to readers.
title = "Pathfinder"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]
# Typed edges to other entries. A feat prerequisite is NOT written here: it is derived from `prerequisite_feats`.
relations = [
  { kind = "archetype", target = "ref/class/example-wayfarer" },
]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

A pathfinder trades a wayfarer's weather sense for a talent in leading others.

## Alternate Class Features {#alternate-class-features}

- **Trail Guide:** A pathfinder replaces the wayfarer's first ability with this one.
