+++
# EXAMPLE: a `rule` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A plain rules entry. `[fields] kind = "none"` because no typed fields apply.
# Body: `##` headings carry `{#anchor}` when something links to them; a list, a pipe table, a quote, and a link to another entry.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/rule/example-ambush"
# The entry kind. The typed `[fields]` below must match it.
kind = "rule"
# The title shown to readers.
title = "Ambush"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]
# Where the entry sits in the rules outline (breadcrumb).
section = ["Combat", "Special Actions"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "none"
+++

## Ambush {#ambush}

A hidden creature that attacks from cover surprises its target. The target is flat-footed against
the first attack, as the rules in [Stealth](ref/skill/example-stealth) describe.

### Choosing a Hiding Place {#choosing-a-hiding-place}

- The place must give cover or concealment.
- The place must lie within **30 feet** of the target.
- A creature that can see you cannot be ambushed.

|Distance|Check|
| --- | --- |
|10 feet|DC 10|
|30 feet|DC 15|

> **Table note:** a creature with blindsense ignores concealment from darkness.
