+++
# EXAMPLE: a `skill` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A skill has no typed fields; its numbers sit in the stat block.
# Body: the stat block, then the description.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/skill/example-stealth"
# The entry kind. The typed `[fields]` below must match it.
kind = "skill"
# The title shown to readers.
title = "Stealth"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

::: statblock
Key Ability: Dex
Trained Only: no
Armor Check Penalty: yes
:::

You are skilled at avoiding notice.
