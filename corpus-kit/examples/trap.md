+++
# EXAMPLE: a `trap` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A trap. No typed fields; the numbers sit in the stat block.
# Body: the stat block, then the description.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/trap/example-falling-net"
# The entry kind. The typed `[fields]` below must match it.
kind = "trap"
# The title shown to readers.
title = "Falling Net"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

::: statblock
Type: mechanical
Perception: DC 20
Disable Device: DC 20
Trigger: location
Reset: manual
Effect: net falls (Reflex DC 14 avoids)
CR: 1
:::

A weighted net hangs from the ceiling of the passage.
