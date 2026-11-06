+++
# EXAMPLE: a `trait` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A character trait. `category` is required; `detail` is optional.
# Body: the trait text.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/trait/example-trailwise"
# The entry kind. The typed `[fields]` below must match it.
kind = "trait"
# The title shown to readers.
title = "Trailwise"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "trait"
# Required.
category = "Regional"
detail = "Borderlands"
+++

You grew up tracking game. You gain a +1 trait bonus on Survival checks, and Survival is always a
class skill for you.
