+++
# EXAMPLE: a `feat` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# `prerequisite_feats` lists the feats the prerequisites name. The reader derives the prerequisite relations from it.
# Body: a backslash at the end of a line is a line break. Enter alone is a space; a blank line starts a new paragraph.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/feat/example-quick-draw-twice"
# The entry kind. The typed `[fields]` below must match it.
kind = "feat"
# The title shown to readers.
title = "Quick Draw Twice"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "feat"
# Feat types, such as Combat.
feat_types = ["Combat"]
# The prerequisite wording as printed.
prerequisites = "Dex 13, Example Quick Draw."
# Ids of the feats the wording names.
prerequisite_feats = ["ref/feat/example-quick-draw"]
benefit = "You can draw two light weapons as a single free action."
special = "A fighter can select this feat as a bonus feat."
+++

## Quick Draw Twice (Combat)

You are practiced at drawing both blades together.\
**Prerequisites:** Dex 13, [Example Quick Draw](ref/feat/example-quick-draw).\
**Benefit:** You can draw two light weapons as a single free action.\
**Special:** A fighter can select this feat as a bonus feat.
