---
title: "Development and release migration"
date: 2026-11-03
kind: record
---

# Development and release migration

[GitHub Issues](https://github.com/phynics/cast-iron/issues) is the tracking
source. [#10](https://github.com/phynics/cast-iron/issues/10) tracks this migration.
`release` was created at the published `d48eaa3` commit without changing main or
existing history. Readback confirms Pages uses Actions and the `github-pages`
environment permits only branch `release`. Its old `source.branch: main` API
field is legacy metadata, not the Actions publication selector.

The guarded workflow and promotion helper are prepared in
[PR #16](https://github.com/phynics/cast-iron/pull/16) against main. Main publication is already blocked by the environment policy. The workflow
must merge and be explicitly promoted before the first release-branch deployment.
Local workflow validation passes with actionlint 1.7.12. Eight promotion safety
checks pass, and a real preview against `d48eaa3` confirms the successful main
CI and unchanged release baseline. A real non-main promotion attempt was
rejected by the ancestry check before any push.
[Wrong-ref dispatch 41725677628](https://github.com/phynics/cast-iron/actions/runs/41725677628)
ran on this feature branch and skipped both build and deploy jobs. This verifies
the workflow guard on GitHub without publishing. Local `cargo xtask check`
passes, including formatting, native and WASM clippy, workspace tests, and the
WASM build. No new distribution or browser suite was run for this workflow/docs
change. Final-head GitHub CI is separate and remains visible on the PR.

No release-branch deployment or live migration verification is claimed. See
[release operations](../releasing.md) for exact completion and recovery steps.

### Canonical id alias map (Phase D)

[PR #49](https://github.com/phynics/cast-iron/pull/49) on `data/pf-derived-rules`
added the Phase D alias map; the proposal pass below extends it.
`sources/canonical/id-map.toml` gives every one of the 9,968 frozen legacy ids one
disposition. The current map has 9,968 mapped (5,735 direct, 115 derived, 2,216
carried, 1,902 shared) and none retired; the pre-proposal state was 94 mapped
(1 direct, 93 derived), none shared, and 9,874 retired. `cargo xtask data-alias`
rebuilds the canonical catalog from the pinned Pf Data corpus and the PSRD
snapshot, runs totality, injectivity, target, direction, hash, and semantic-fit
gates, and writes `generated/content/<version>/aliases.json`. The carried rows
come from the all-unmatched product decision: each re-emits the PSRD entry at
`ref/carried/<legacy-id>`, and the gate adds only the carried entries the run
actually produced.

`cargo xtask data-alias-propose` proposes the dispositions through the same
`title_fit` function the gate uses. It requires equal kind, a source book in the
record's publication set, and equal normalized titles or a documented comma
inversion, family/category parenthetical, or item measure/package parenthetical. It
does not match on a slug. Ties resolve only by native-over-derived, a racial-trait
parent race named in the legacy path, or a weapon/armor special-ability path; other
ties stay retired with an `ambiguous: <candidates>` note. The 94 already published
rows are unchanged, and four proposals that would merge onto them stay retired.

Carried rows by kind: rules 512, items 418, creatures 327, racial traits 275,
afflictions 214, spells 179, feats 165, traps 58, races 36, skills 26, archetypes
4, traits 2. The carry-over reports 434 possible duplicates with candidate ids
for later review; none of them changes a mapping. It rewrites 7,238 references
(6,201 body, 438 parent, 439 relation, 160 feat prerequisite) and flattens none.

Local verification on 2026-11-05: `cargo xtask check` passes, two
`cargo xtask data-alias` runs produced a byte-identical alias file
(`5dd41777f712bfd14fa4a084d8b6b77f7b65b38874c838cfb2a592322c4bfe9c`, content
`va174cce225939e2e`), and two `cargo xtask data-report` runs produced the same
`reports/migration-coverage.json`. This is local evidence only; the PR's GitHub
checks are separate and are not claimed here.

Phase 5 makes the map part of the library. `cargo xtask data-alias` adds
`aliases.json` to `manifest.json` and recomputes the content version deterministically.
`Library::resolve` reads the map eagerly and resolves one step (catalog, alias, then
retirement). On the first load of a library that lists a map, an idempotent IndexedDB
transaction rewrites bookmark and recent ids to their canonical targets, merges
many:1 rows, and records `meta["id_migration"].aliases_sha256`; retired and unknown
rows keep their snapshots. Schema version 3 and search format 1 are unchanged.

Local verification on 2026-11-04: `cargo test -p cast-iron-web` (104 lib tests),
`cargo test -p cast-iron-content-compiler`, and `cargo xtask check` pass.
`cargo xtask data-alias` produced a manifest-listed alias file
(`ve730cbbe80014fec`, 94 mapped and 9,874 retired rows). `check-library-upgrade.py`
compared the pre-alias and post-alias library: 9,968 ids, zero missing, zero title
changes, zero incorrect class categories, zero dangling references. The new
`canonical-cutover.spec.ts` fixture test passes in Chromium and WebKit. This is local
evidence only; the production cutover is not live and the deployed library still uses
PSRD ids.

Review follow-up on [PR #51](https://github.com/phynics/cast-iron/pull/51) fixes the
findings above against the same design: the merged-kind tie-break uses the serialized
slug, the star contract resolves stored rows through the alias map (pressed, add, unstar,
and collection membership), the bookmarks and Home lists collapse one saved item to one
row, a refused conversion shows the Settings notice, an unreadable migration marker reads
as absent, catalog membership trusts the sorted catalog, a retired entry route shows its
reason category, and `AliasIndex::validate` enforces the one-step and one-disposition
rules. Local verification on 2026-11-04: `cargo test -p cast-iron-web` passes with 113
lib tests, `cargo test -p cast-iron-content-compiler` passes with 162 lib and 20
integration tests, and `cargo xtask check` passes (format, native and WASM clippy,
workspace tests, WASM build). No `dist/` build or Playwright rerun was made for this
follow-up, and no production cutover is claimed.
