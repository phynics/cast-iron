# Library expansion design

Historical design for the schema-2 expansion from the two-book POC to 16 books. Counts and migration results below describe that stage. For schema 3 and the current UI, see [status](../status.md) and [decisions](../decisions.md).

## Problem and traced data flow

`source.toml` selects typed directories and complete rules trees. `select` hashes the selected input, `pipeline` validates upstream nodes, `build` assigns permanent URL-derived IDs and replaces nested typed copies with links, then the compiler emits schema nodes, catalog, coverage, and a string-based search index. The client reads the active cache and derives browse groups and filters from `EntryKind`. At the start of this design, installer validation accepted only the current schema version.

## Usage and shape

Users browse Classes and Prestige classes separately or select either search category. Both retain class fields. An old bookmarked Arcane Archer URL continues to open the same entry.

Candidate A keeps explicit, reviewable source lists in `source.toml` and normalizes `class` plus subtype into `EntryKind::PrestigeClass` at the compiler boundary. Candidate B discovers all upstream nodes and derives categories and books dynamically from their metadata.

## Synthesis decision

Choose A. The source configuration remains the publication boundary, unsupported record families remain visible in coverage, and schema matches stay exhaustive. Adapt B's discovery only into a rerunnable configuration generator that inventories the pinned snapshot. Automatic runtime discovery would expose unreviewed types and source metadata errors to the client.

`EntryKind::PrestigeClass` uses `ClassFields`, serializes as `prestige_class`, and routes under that category. Existing IDs remain based on upstream URLs. Schema 2 is required because schema-1 clients cannot parse the new variant. The new client accepts schema 1 as well as schema 2, retaining access to the active library while an update downloads. Search format stays unchanged because it already stores category strings.

Select all supported typed directories and complete top-level rules trees from reviewed books. Non-core record families remain rendered within their parent rules trees where available. Do not select every per-section JSON copy, which would duplicate the same trees. Preserve existing POC rule-child IDs when adding additional trees. Report malformed records and removed material with reasons.

## Tradeoffs and verification

Explicit configuration is longer than automatic discovery but makes the shipped scope reviewable. A single index remains until full-corpus measurements justify partitioning. Compare two compiler runs, verify the old ID set, test schema-1 reads and schema-2 updates, and exercise distinct class categories, new book browsing, offline install, integrity, and search in the built app. Review newly encountered licensing notices and named material before publication.

## Implementation reassessment

An independent check of selection and migration was adopted: explicitly support schema versions 1 and 2, preserve the old ID set, identify source placeholders, and state unsupported-family coverage honestly. The alternative UI-only subtype grouping was rejected because it would duplicate category derivation across catalog, browse, and search.

The expanded compile found optional null item attributes, a class encoded as a generic section, named NPC stories, and a construct with both creature and item representations. These are handled at compiler boundaries with explicit source decisions and coverage evidence. The resulting library has 9,112 entries and 19 prestige classes. All 1,937 old IDs and titles survive. The actual published schema-1 library was installed in the new app, bookmarked, and upgraded to schema 2 with the bookmark and offline reading retained. Query and index-load measurements support keeping one index on the tested desktop; real-phone memory limits remain unverified.
