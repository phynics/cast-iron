+++
# EXAMPLE: a `creature` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# `abilities` is six strings in STR, DEX, CON, INT, WIS, CHA order; `-` means none. `cr` is written as printed (`1/2`).
# Body: the stat block repeats the headline numbers; edit both copies when you change one.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/monster/example-cave-crawler"
# The entry kind. The typed `[fields]` below must match it.
kind = "creature"
# The title shown to readers.
title = "Cave Crawler"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]
# Where the entry sits in the rules outline (breadcrumb).
section = ["Monsters"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "creature"
# Challenge rating as printed.
cr = "1/2"
creature_type = "Vermin"
size = "Small"
alignment = "N"
ac = "13, touch 12, flat-footed 12 (+1 Dex, +1 natural, +1 size)"
hp = "5 (1d8+1)"
fortitude = "+3"
reflex = "+1"
will = "+0"
melee = "bite +1 (1d4)"
# STR, DEX, CON, INT, WIS, CHA.
abilities = ["10", "13", "12", "-", "10", "6"]
+++

::: statblock
CR: 1/2
Size: Small
Type: Vermin
AC: 13, touch 12, flat-footed 12 (+1 Dex, +1 natural, +1 size)
hp: 5 (1d8+1)
Fort: +3
Ref: +1
Will: +0
Melee: bite +1 (1d4)
Ability Scores: Str 10, Dex 13, Con 12, Int -, Wis 10, Cha 6
:::

A pale, many-legged hunter that clings to cave ceilings and drops on passing prey.
