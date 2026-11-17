---
title: "Product Identity screen for the Pf Data corpus"
date: 2026-11-04
status: partly-superseded
superseded_by: ["2026-11-05-paizo-product-identity-kept-under-the-community-use-policy"]
issues: []
prs: []
area: licensing
---

# Product Identity screen for the Pf Data corpus, 2026-11-04

- **Names stay, descriptive PI prose goes.** Pf Data is book text, not Paizo's genericized PRD. About 16% of its description records name setting Product Identity, including most core class and race pages, mainly in archetype flavor lines. The product owner chose fidelity under Paizo's Community Use Policy. Proper names (deities, places, organizations, characters) inside game mechanics stay as descriptive references. This replaces the zero-names standard that the PSRD baseline met only because PRD text was already genericized. Descriptive Product Identity prose is still omitted, as the licensing audit requires.
- **Every Pf Data entry carries the Community Use notice.** Imported and derived Pf Data entries carry `ogl-1.0a` and `legal#community-use`. A lexicon cannot reliably tell which records name PI, so routing every Pf entry through the notice is the safe default; it claims nothing beyond the notice the app already shows. `scripts/check-publication.py` now accepts named material only on entries that carry the notice path, and it fails on an outbound address in displayed text.
- **The screen is reviewed section rules in `sources/pathfinder-data-1-e/pi-screen.json`.** It runs once in the inventory, after hash verification and the directive count, so conversion, rule extraction, derived records, approved records, and the alias catalog all read screened text. A missing or malformed screen fails the build. Its hash is part of the inventory `input_digest`. The initial rules omit the deity `Description` section (463 records), the unique creature `Description` section (164), and named NPC stories (190 `###` sections in the NPC files, matching the PSRD `omit_npc_stories` rule). Alignment, domains, obediences, boons, stat blocks, tactics, and special abilities stay.
- **Outbound links are removed.** 483 `[text](http…)` links, mostly the `Pathfinder Wiki` line on deity pages, are dropped; inline link text is kept. The schema cannot express an external link, and the reference must stay self-contained (AGENTS.md rule 1).
- **Every omission is coverage.** Omissions appear as `product-identity-section` and `external-link` rows in the migration coverage, and their totals appear in `reports/migration-coverage.json` under `pi_screen`. A new PI rule is a reviewed edit to the screen file, not a converter special case.
