---
title: "Paizo Blog source bases under the Community Use Policy"
date: 2026-11-18
status: current
superseded_by: []
issues: [36]
prs: []
area: licensing
---

# Paizo Blog source bases under the Community Use Policy, 2026-11-18

The project owner confirmed in issue #36 that Paizo Blog posts are covered under Paizo's Community Use Policy (CUP) for this project. This decision does not authorize reproducing artwork. The Community Use notice stays on the Legal page.

## Decision

A `[[publication_licenses]]` row in the corpus `legal.toml` records a reviewed legal basis for one publication: its id, its exact title, and the license ids it relies on (`ogl-1.0a`, `legal#community-use`, or both).

- A publication with no row keeps the legacy basis: OGL, so it needs a Section 15 notice in its `copyright` or a `notice_pending` title.
- A row that does not name `ogl-1.0a` is a CUP-only source. It needs attribution in its `copyright` and no Section 15 notice. Never invent one.
- A row that names both bases still needs the OGL notice or a pending title.
- `corpus-check` rejects a row for an unknown publication, a title that differs from `publications.toml`, an empty list, an unknown license id, and two rows for one publication.

The field is optional and additive. `LegalNotices` has no `deny_unknown_fields`, a library without rows serializes byte-for-byte as before (`skip_serializing_if`), and older readers ignore the key. It is the same pattern as `notice_pending`. `SCHEMA_VERSION` and the corpus format do not change (AGENTS rule 4). A schema unit test pins the unchanged serialization.

The app shows the basis in two places. The Legal page lists each source that names the Community Use Policy, marked "Community Use Policy only" or "Open Game License and Community Use Policy", and no longer lists a CUP-only source as a pending OGL publication. An entry footer follows the entry's own `license_refs`.

## Source-by-source outcome

| Source | Outcome | Basis |
|---|---|---|
| Ultimate Cantrips (blog) | Six spells added: Breeze, Drench, Jolt, Penumbra, Root, Scoop. | CUP only. Credit: Jason Bulmahn. Illustration not reproduced. |
| The Gauntlet Will Be Ours! and the Primitive Human PDF | Primitive Human added as a race. Rare Races gains the membership. | The PDF carries the OGL and its own Section 15 notice (author Jason Bulmahn), so OGL; the post is also CUP. |
| Golarion Day: Other Gods and New Subdomains | Memberships added to Scalykind and Void. The blog-only Death and Repose adjustments are recorded as exclusions, because those domains are not corpus entries yet. | CUP only. Credit: James Jacobs. |
| Return of the Sable Company | No membership. The shipped record is the later Inner Sea Combat revision. The entry text keeps the blog as a credit. An exclusion row records this. | Not imported. |
| More Magic of Thassilon (web supplement) | Corrected in place under the same id. Category "Web supplement". Credits: Brian Cortijo and Owen K. C. Stephens, with Stephen S. Greer, Tim Hitchcock, Michael Kortes, Nicolas Logue and Greg A. Vaughan. | CUP only. The PDF has no OGL page. Sadist's Lash keeps its Rise of the Runelords Anniversary Edition p. 425 membership. |
| Hodgepodge from the Grand Lodge! | Thought Thief (Arcane Trickster) added, under the existing publication. | Entry is CUP only. The publication keeps its pending OGL notice for its other entries. |
