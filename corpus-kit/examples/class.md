+++
# EXAMPLE: a `class` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A base class. `[fields] kind = "class"` is shared by `class` and `prestige_class`.
# Body: headings per class feature.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/class/example-wayfarer"
# The entry kind. The typed `[fields]` below must match it.
kind = "class"
# The title shown to readers.
title = "Wayfarer"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "class"
class_type = "base"
hit_die = "d8"
skill_ranks = "6 + Int modifier per level"
class_skills = "Climb, Perception, Survival"
+++

## Class Features {#class-features}

A wayfarer knows the roads, the weather, and the quiet ways around trouble.
