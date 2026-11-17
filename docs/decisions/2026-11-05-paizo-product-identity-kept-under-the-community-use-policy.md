---
title: "Paizo Product Identity kept under the Community Use Policy"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: licensing
---

# Paizo Product Identity kept under the Community Use Policy, 2026-11-05

- **The owner keeps Paizo Product Identity.** Deity descriptions, unique character biographies, named NPC stories, suggested race names, deity-named items, religion traits, and setting commentary now ship. This replaces the omission rules of the Pf Data PI screen (2026-11-04) and the PI entries of `sources/psrd-data/content-policy.toml`. Rules text still rests on the OGL. Names and setting material rest on Paizo's Community Use Policy, which is free, non-commercial, unofficial, and carries the required notice.
- **Every entry carries the Community Use notice path.** Pf Data entries already did. PSRD-derived entries, including carried ones, now carry `ogl-1.0a` and `legal#community-use`. `scripts/check-publication.py` keeps rejecting named material on an entry without that path, and keeps rejecting outbound addresses.
- **Accepted risk.** The licensing audit reads the CUP as permitting descriptive reference to Product Identity, with informal FAQ guidance against republishing significant descriptive text. The owner accepts that risk for this free, non-commercial reference; Paizo can revoke CUP permission at any time. Artwork, logos, trade dress, and any claim of official status stay excluded (AGENTS.md rules 10 and 11).
- **The screen machinery stays.** `pi-screen.json` keeps removing outbound links (rule 1) and can hold a reviewed omission rule if one is needed later. Omissions remain coverage rows.
