+++
# EXAMPLE: a `prestige_class` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A prestige class uses the same `[fields] kind = "class"` as a base class; only the entry `kind` differs.
# Body: requirements and features.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/class/example-road-warden"
# The entry kind. The typed `[fields]` below must match it.
kind = "prestige_class"
# The title shown to readers.
title = "Road Warden"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "class"
class_type = "prestige"
hit_die = "d10"
skill_ranks = "4 + Int modifier per level"
class_skills = "Intimidate, Perception"
+++

**Requirements:** base attack bonus +5, 5 ranks in Perception.
