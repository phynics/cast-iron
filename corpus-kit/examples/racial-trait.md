+++
# EXAMPLE: a `racial_trait` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A racial trait. `parent` names the race and a `part` relation says the same. `alternate = true` marks an alternate racial trait.
# Body: the trait text.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/racial-trait/example-hillfolk/stonefoot"
# The entry kind. The typed `[fields]` below must match it.
kind = "racial_trait"
# The title shown to readers.
title = "Stonefoot"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]
# The containing entry. It must exist.
parent = "ref/race/example-hillfolk"
# Typed edges to other entries. A feat prerequisite is NOT written here: it is derived from `prerequisite_feats`.
relations = [
  { kind = "part", target = "ref/race/example-hillfolk" },
]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "racial_trait"
# Required boolean.
alternate = false
+++

Hillfolk gain a +2 bonus on Acrobatics checks made on uneven ground.
