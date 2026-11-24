---
title: "Corpus model proposals approved; kinds and field ownership stay a direction"
date: 2026-11-24
status: current
superseded_by: []
issues: [134, 670, 671, 672, 673]
prs: [669]
area: corpus
---

# Corpus model proposals approved; kinds and field ownership stay a direction, 2026-11-24

The owner approved the seven proposals of `docs/research/corpus-model-review.md` on 2026-11-24, and
asked that proposals 6 and 7 stay loose and flexible.

## Decision

**Proposals 1 to 5 are work to do**, each with a ticket:

1. One spelling per value in creature size and alignment, hit die, spell school and subschools, with
   a closed-vocabulary check (phynics/cast-iron-data#287).
2. Typed creature fields for the NPC entries (phynics/cast-iron-data#289).
3. Typed fields for afflictions and traps (#670).
4. Fixed keys for weapon and armor statistics (#671), after 3.
5. An optional numeric price beside the printed price (#672).

**Proposals 6 and 7 are a direction, not a plan.** The data should state the family of the typed
things now filed as `rule` (#673), creature hit points should be structured, and each value should
have one owner between the typed field and the body (#134). No design, order or date is fixed for
them. Whether a family becomes a kind, a group or a tag is open, and the list of families can
change.

## Why

- Proposals 1 to 5 add or correct data inside the present model. None needs an id, a kind or the
  corpus format to change, so each can land alone.
- Proposals 6 and 7 break serialized content. The haunt kind (#618, landed as schema 7 in #662 with
  its data in corpus v0.1.43) showed what a new kind costs, and they are cheaper as one format batch
  than as two. Fixing a design now would be a guess.

## Consequences

- Proposals 3 to 6 are in the `post-v1` milestone and do not block v1.
- Work on 6 or 7 starts with a short design note.
