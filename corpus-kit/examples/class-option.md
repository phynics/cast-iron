+++
# EXAMPLE: a `class_option` entry. Comments like this one are allowed in this example only;
# `cast-iron-corpus fmt` removes them from a real entry file.
# A class option is picked in place of a class feature, such as an arcane discovery.
# `group` names a slug in `option-groups.toml`; the group states its label and classes.
# There is no `provenance` (frozen import history: leave it out of new entries) and no
# `license_refs` (the default in corpus.toml applies).
#
# The permanent id. It never changes, because bookmarks store it. The file path is the id without `ref/`.
id = "ref/arcanedisc/example-discovery"
# The entry kind. The typed `[fields]` below must match it.
kind = "class_option"
# The title shown to readers.
title = "Example Discovery"
# Publication ids, first = primary. Each one must be listed in publications.toml.
source_ids = ["example-book"]

# Typed fields: what search, facets, and the entry page read. They must match the kind.
[fields]
kind = "class_option"
# A slug from option-groups.toml.
group = "arcane-discovery"
+++

A wizard can choose this discovery in place of an arcane discovery.
