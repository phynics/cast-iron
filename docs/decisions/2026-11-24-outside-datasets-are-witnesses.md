---
title: "Outside datasets are witnesses for the corpus, not sources of text"
date: 2026-11-24
status: current
superseded_by: []
issues: [668]
prs: []
area: corpus
---

# Outside datasets are witnesses for the corpus, not sources of text, 2026-11-24

The owner asked on 2026-11-24 for a check of the corpus against four outside projects, to catch drift,
and then for a plan to keep using the ones that passed. The evidence is in
`docs/research/cross-check-sources.md`.

## Decision

**Three datasets are approved as witnesses.** The community sheets linked from pathfindercommunity.net,
the PSRD data in `Nietsneflow/open-reference`, and the Foundry module `baileymh/pf1e-content`.
`BurcinSayin/rpg_parser` is a scraper with no dataset and is not used.

**A witness finds candidates. It supplies nothing.** No text, value or code from a witness enters the
corpus or the app. A difference becomes a ticket on the corpus tracker. A print check, cited by book
and page, decides each one.

**A dataset that renames Product Identity is not a witness for names.** The three approved datasets
keep the printed names. d20PFSRD replaces them with generic ones, so its titles are never compared
with corpus titles.

**Independence is counted.** The Foundry module's feats and magic items come from the community
sheets, so the two are one witness there. The PSRD data is not independent of the PSRD carry-over
entries.

## Why

- The first comparison found errors that the corpus checks cannot see, because the entry is valid
  and only its values are wrong: one spell with another spell's level and duration, misspelled
  titles, typos in field values.
- The witnesses are wrong often. On spell fields the community sheet lost about three disagreements
  in four. Applying a witness would add more errors than it removes.
- The corpus ships Paizo names under the Community Use Policy. A comparison with a dataset that
  cannot use that policy would report every such name as a difference.
- AGENTS.md rule 12 already limits how entries enter the library. Reading a dataset to find a
  mistake does not copy it; importing from it would need its own licensing review.

## Consequences

- The comparison becomes a research script that anyone can run again (#668, milestone `post-v1`).
- Findings of 2026-11-24: phynics/cast-iron-data#282, #283, #284 and #285.
- A new candidate witness needs the same three checks before use: what it states as its legal
  basis, whether it keeps printed names, and which approved witness it derives from.
