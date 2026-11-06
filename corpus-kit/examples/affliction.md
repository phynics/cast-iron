+++
# EXAMPLE: a `affliction` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A disease, curse, or poison. No typed fields; the numbers sit in the stat block.
# Body: the stat block, then the description.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/affliction/example-marsh-fever"
# The entry kind. The typed `[fields]` below must match it.
kind = "affliction"
# The title shown to readers.
title = "Marsh Fever"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

::: statblock
Type: disease, injury
Save: Fortitude DC 14
Onset: 1 day
Frequency: 1/day
Effect: 1d3 Con damage
Cure: 2 consecutive saves
:::

A fever that spreads through still water.
