+++
# EXAMPLE: a `haunt` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A haunt: a lingering spirit hazard that is listed apart from traps. No typed fields; the
# challenge rating, the hit points and the other rows sit in the body, as the book prints them.
# Body: the description, the numbers as bold rows, then `### Effect` and `### Destruction`.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/haunt/example-cold-spot"
# The entry kind. The typed `[fields]` below must match it.
kind = "haunt"
# The title shown to readers.
title = "Cold Spot"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

A chill gathers where a traveler once froze to death.

**Cold Spot**

**Challenge rating:** 1

**XP** 400\
NE haunt (10-ft. square)\
**Caster Level** 1st

**Notice:** Perception DC 15 (to see the breath of a figure that is not there)

**Hit points:** 6

**Trigger:** proximity

**Reset:** 1 day

### Effect

The air turns bitter. Each creature in the area takes 1d6 points of cold damage (Fortitude DC 11
halves).

### Destruction

The haunt is destroyed when someone lays the traveler's bones in consecrated ground.
