# PF1e data migration

Tracking issue: [#18](https://github.com/phynics/cast-iron/issues/18).
This document records the migration design and observed limitations. Acceptance
and outstanding work remain in the issue. PSRD-Data remains the primary production
source. One explicitly reviewed primary-source record is now imported through the
bounded adapter; the rest stays in coverage.

The migration branch was rebased onto `0111ea7` after PR #7 and the subsequent
removal of star transfer in PR #28. PR #7's runs `41732375845` and `41732382340`
both passed Rust checks, deterministic content generation, Chromium, and WebKit.
Those runs verify the PR #7 snapshot, not this migration. Its mapping evidence is
bound to the old corpus by hashes and must be regenerated after content changes.

## Current architecture

`cargo xtask fetch-sources` retrieves PSRD-Data commit
`76ea4784604145ba3dea40eb56f83bb904759467`. The native compiler selects reviewed
JSON files, applies `content-policy.toml`, converts HTML into typed nodes, assigns
permanent URL-derived IDs, resolves links, and builds schema-3 content shards.
It generates the catalog, legal notices, relationships, spell lists, book index,
and format-1 native/WASM search index. The last recorded production baseline is
9,967 entries from 16 publications. See `docs/status.md` for its verification scope.

`crates/web/src/library.rs` reads these files from the active Cache API library.
The installer verifies every manifest hash before activation. IndexedDB stores
bookmarks and preferences independently. `web/sw.js` caches the application shell.
No source repository or external service participates at runtime.

## Target architecture and usage

The primary-source adapter belongs in the native compiler. It returns the shared
`RuleEntry` model, never upstream JSON or raw HTML to the client. Import diagnostics
and the original source records stay in build artifacts outside the distribution.
The existing shard, catalog, search, and installation paths remain the integration
contract. A second browser renderer or runtime database would duplicate those paths.

The planned command interface is `cargo xtask data-fetch`, `data-audit`,
`data-import`, `data-validate`, `data-report`, and `data-test`. Publication requires
a separate successful production build and upgrade acceptance. An audit succeeding
does not mean an import, licensing review, or cutover passed.

The adapter separates source validation, record expansion, typed Markdown and
directive conversion, mechanical extraction, identity resolution, and emission.
Only validated canonical records may enter production packaging. Unknown directives,
unresolved compilation inputs, and unreviewed rights must appear in diagnostics.

## Field mapping

| Source data | Canonical data |
|---|---|
| File and record key | Immutable namespaced identity, explicit equivalence map for existing IDs |
| `name` or `title` | `RuleEntry.title` |
| Record family | `EntryKind` and `TypedFields` |
| `sources` and `SOURCE` markers | Publication registry, `source_ids`, page attribution in body |
| Description Markdown and directives | Typed `Block` and `Inline` nodes |
| Spell, class, monster, and item directives | Mechanical fields and labelled body statistics |
| Encoded links and Markdown links | Resolved `EntryReference` targets, reported misses |
| Categories, tags, parent topics | Tags and hierarchy after explicit mapping |
| File, record key, snapshot | `Provenance` and manifest dataset pin |
| OGC notices and rights decisions | Legal notices and publication gate |

Do not equate entries by name alone. Existing PSRD IDs must retain their meaning.
Source-native keys can distinguish same-name records. `copyof`, `redirect`,
`compileFrom`, external table definitions, and addenda require explicit expansion
before an entry can be considered complete.

## Source pins and rights

The primary pin is `jasontankapps/pathfinder-data-1-e` commit
`08ea71f24375908d0190c822fd1fd080797667a9`. `sources/pathfinder-data-1-e/source.json`
records the archive hash and individual input hashes. Its README explicitly places
OGC in `json/` and separates non-OGC GPL code. Its OGL notice excludes Product
Identity. The existing PSRD audit does not clear every passage in this new source.
Do not copy upstream renderer code or artwork. Preserve all applicable Section 15
notices before distributing imported OGC. See [the source-specific audit](research/pf-data-licensing.md)
and the measured migration reports.

Supplementary sources require independent pins and rights decisions. They are not
automatically imported. AoN source indexes are a benchmark, not a bulk export API
or a redistribution grant. Count a publication registry separately from publications
with imported records.

## Reviewed supplemental records

`sources/pathfinder-data-1-e/approved.json` is the only path from the primary
source into production. Each record names an exact file and key, an expected name
and publication, a permanent native-protocol ID, an optional parent, a review
reason, a required `rights` basis, and the reviewed topic targets. The adapter
rejects a mismatched identity, an unapproved field, an unknown link target, a
duplicate ID, or an unknown parent. All remaining inventory records appear in
`coverage.json` with a reason. The initial selection is `pf-data/rule/magic`,
"Magic and Spellcasting", the missing Core Rulebook page 206 opening passage
under the existing Magic article.

The `rights` field accepts exactly four values:

| Value | Meaning |
|---|---|
| `ogl` | Import under the Open Game License. The entry carries the canonical `ogl-1.0a` license reference. |
| `cup` | Import under the OGL plus Paizo's Community Use Policy. The entry carries `ogl-1.0a` and the legal-page Community Use notice path, `legal#community-use`. |
| `orc` | Refused. The current legal page has no ORC notice path, so an ORC record cannot be routed through the OGL page. |
| `excluded` | Do not import. The record appears in coverage with the reviewed reason instead of becoming an entry. |

An omitted or unknown `rights` value fails the selection, as does an `orc`
record. A `cup` record is always emitted with the reviewed legal-page notice path.

`sources/psrd-data/source.toml` points at this selection in its `[supplement]`
table. The compile therefore appends the reviewed entry after the ordinary build,
adds its links and nested placement, appends the dataset pin and Section 15
notices, and folds the full inventory exclusions into coverage. The PSRD selection
digest still gates its own inputs; the coverage report's `input_digest` also covers
the primary inventory digest.

`sources/pathfinder-data-1-e/out-of-scope.json` names publications and records
that are outside the PF1e OGL scope. The inventory still counts them, the migration
report lists them, and selection skips them. The initial policy excludes the PF2
`Monster Core` and `Bestiary 3 (2nd Edition)` publications, their eight tagged
records, the `json/main.json#ogc` legal notice that carries ORC text, and the
untagged PF2 *Player Core* description in `gnome_flick_mace`. The pin also hashes
the ORC and GNU license files so a changed notice fails the source audit.

`cargo xtask data-fetch` verifies the archive hash and the individual input hashes
before activation. The migration commands remain isolated: `data-report` writes
`reports/migration-coverage.json` and the representative test files, while full
corpus diagnostics stay under ignored `reports/migration-full.*`. A blocked gate
still exits nonzero for `import`, `validate`, and `test`; it cannot alter
`generated/content`.

## Legacy alias map

The canonical cutover renumbers public entries under `ref/`, so every id in the
frozen `sources/canonical/legacy-ids.json` snapshot needs a reviewed disposition in
`sources/canonical/id-map.toml`. `cargo xtask data-alias` resolves each row through
the canonical derivation over the pinned Pf Data corpus and writes
`generated/content/<version>/aliases.json`. It fails on an unaccounted legacy id, a
target the corpus cannot produce, a chain, a many:1 merge that is not an explicit
`[shared]` row, or a `[map]` row whose legacy title and source book do not fit the
canonical record. The fit gate is what rejects a row that shares only a slug with an
unrelated record; a retired row from that gate says so in its note.

The map is append-only. A route change or an upstream rename is an identity change:
add the affected `[map]` rows instead of editing an already published target. The
generated file is build output and is not committed. The client does not read it yet;
schema version 3 and search format 1 remain unchanged. See the Phase D decision in
[decisions](decisions.md#legacy-to-canonical-alias-map-2026-11-04) and
[the canonical id design](research/canonical-id-design.md).

## Migration and test sequence

1. Verify pinned source files and inventory every input, including non-content files.
2. Import a bounded sample covering all requested families and late publications.
3. Serialize and re-read canonical records. Build and query the real local index.
4. Inspect representative typed rendering and follow resolved links.
5. Report rejected records, unsupported syntax, missing metadata, and rights gaps.
6. Fix critical and high findings with source-specific regression fixtures.
7. Expand to all eligible records and compare counts against the reviewed baseline.
8. Build the production library twice and compare every byte.
9. Verify old IDs, browser update behavior, offline search, navigation, and persistence.
10. Make the new pipeline default only after these gates pass.

## Validation and cutover requirements

`cargo xtask check` is mandatory. A changed corpus also requires explicit
`cargo xtask content` before `cargo xtask build-site`, the affected real-distribution
e2e tests, `scripts/check-publication.py`, and `scripts/check-library-upgrade.py`.
Save the previous library outside `generated/` and `dist/`. Install it in a browser,
save an affected bookmark, activate the replacement, and reload offline. Check
interrupted updates against the active library. Record local, CI, and Pages evidence
separately. Do not automatically promote `main` to `release`.

## Rollback and recovery

Until acceptance passes, retain the current production pipeline and its IDs.
Keep experimental artifacts under `generated/migration`, outside site packaging.
A failed experiment must not alter `generated/content` or the active browser cache.
After eventual cutover, rebuild a reviewed prior source/compiler revision for a new
content version if recovery is needed. Follow `docs/releasing.md` for publication.
Do not reset release history or delete user storage to recover content.
