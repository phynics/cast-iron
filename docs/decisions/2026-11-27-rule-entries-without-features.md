---
title: "Feature-shaped rule entries get printed prose and headings, but no features key"
date: 2026-11-27
status: current
superseded_by: []
issues: [540]
prs: []
area: corpus
---

# Feature-shaped rule entries get printed prose and headings, but no features key, 2026-11-27

On 2026-11-27 the owner answered question 5 of #540 with option A, accept for v1. 33 rule entries
are shaped like class features but are filed as rules: 16 `ref/ability/*` lists, 11 Core Rulebook
class chapters, and 6 medium spirits. They keep their printed prose and headed sections. They get no
`features` in `[fields]`, so their heads show no `Level N` and they get no feature search aliases.

The corpus reader and checker are unchanged. `features` stays limited to `archetype`, `class_option`
and `class` (`kind_has_features` in `crates/corpus/src/features.rs`). Adding `features` to rule entries
can be revisited after v1, if a reader needs the level or the search aliases of one of them.
