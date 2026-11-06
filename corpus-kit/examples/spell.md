+++
# EXAMPLE: a `spell` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# Typed spell fields feed search and the school/level facets.
# Body: the stat block repeats them for the reader; `corpus-check` warns when the two copies disagree. In a stat block row, `[` and `]` are written `\[` and `\]`.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/spell/example-spark-needle"
# The entry kind. The typed `[fields]` below must match it.
kind = "spell"
# The title shown to readers.
title = "Spark Needle"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]
# Where the entry sits in the rules outline (breadcrumb).
section = ["Spells", "Spell Descriptions"]
# Stable labels.
tags = ["fire"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "spell"
# Required.
school = "evocation"
descriptors = ["fire"]
# One row per class and level.
levels = [
  { class = "Sorcerer", level = 1 },
  { class = "Wizard", level = 1 },
]
casting_time = "1 standard action"
components = "V, S"
range = "close (25 ft. + 5 ft./2 levels)"
# Target, Effect, or Area rows as (label, value) pairs.
effects = [
  { label = "Target", value = "one creature" },
]
duration = "instantaneous"
saving_throw = "Reflex half"
spell_resistance = "yes"
+++

::: statblock
title = "Spell statistics"
School: evocation \[fire\]
Level: 1: Sorcerer, Wizard
Casting time: 1 standard action
Components: V, S
Range: close (25 ft. + 5 ft./2 levels)
Duration: instantaneous
Saving throw: Reflex half
Spell resistance: yes
Target: one creature
:::

A needle of sparks leaps from your finger. The target takes **1d6** fire damage per two caster
levels (maximum 5d6).
