+++
# EXAMPLE: a `item` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# Typed item fields. `slot = "none"` means no body slot.
# Body: a stat block with the price, weight, aura, and caster level rows.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/item/example-lantern-of-warning"
# The entry kind. The typed `[fields]` below must match it.
kind = "item"
# The title shown to readers.
title = "Lantern of Warning"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "item"
category = "wondrous item"
price = "1,200 gp"
weight = "2 lbs."
slot = "none"
aura = "faint divination"
caster_level = "3rd"
+++

::: statblock
Aura: faint divination
CL: 3rd
Slot: none
Price: 1,200 gp
Weight: 2 lbs.
:::

This hooded lantern burns blue when a hostile creature comes within 60 feet.
