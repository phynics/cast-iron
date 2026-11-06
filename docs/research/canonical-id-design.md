# Canonical IDs and the Pf Data derived-record layer

> **Retired import.** This document describes the upstream import that built the library before it moved to the pinned plain-text corpus. The import code is preserved at the git tag `psrd-last-buildable`; the facts below stay as evidence and history, and the paths they name no longer exist on `main`.

Chosen 2026-11-04. This is an implementation contract, not a shipped-behavior report.
Authored at `921e1d1ce2e3439d3bc04ad9e8cfab688cdc0660`; rebased onto current `main`.
Counts must be reconfirmed against the rebased tree before Phase 1 begins.

The project has decided to extend the pinned Pf Data corpus with derived records and to
cut over to canonical IDs derived from that source. This document fixes the namespace,
the derivation rule, the derived-record layer, the legacy alias map, the client
migration, the verification gates, and the order of work.

## 1. What this design does not do

- It does not claim that the 1,879 production IDs without a Pf counterpart are solved.
  1,039 of them have a designed derivation path, 841 need carry-over or a product
  decision, and neither number counts until the totality check passes. Section 4.7
  gives the accounting.
- It does not change `SCHEMA_VERSION` (3) or `INDEX_FORMAT_VERSION` (1). Section 7
  states why, and what would force a bump.
- It does not add a source, a license, or runtime network access. The alias map is a
  verified library file like `catalog.json`.
- It does not decide corpus growth. Records beyond the 9,968 current entries need a
  separate reviewed list; this design only renumbers and fills legacy gaps.

## 2. Evidence baseline

| Fact | Value | Source |
|---|---|---|
| Production entries at the baseline | 9,968 | `docs/status.md`, `vff56b0e6310710c1` |
| PSRD-derived entries | 9,967 | baseline minus the supplement |
| Entries already in the Pf namespace | 1 | `pf-data/rule/magic`, the reviewed supplement |
| Pf Data input records in the pin | 34,530 | `reports/migration-coverage.json`, `input_records` |
| Of those, records carrying a protocol route | 34,495 | `generated/migration/inventory.json`, `records` with a non-empty `protocol` |
| Legacy IDs with a Pf counterpart | 8,088 | unconfirmed estimate, see note below |
| Legacy IDs without a Pf counterpart | 1,879 | 9,967 - 8,088, unconfirmed estimate |
| Pf-derived IDs verified injective | Phase 1 exit criterion | not yet tested in code; see section 8.4 |

> **Note on the counts.** The 8,088 / 1,879 / 342 / 171 / 526 / 355 / 486 figures come
> from an analysis script run outside the repository. They are planning estimates, not
> verified compiler output. Phase 0 must confirm them against the pinned code and commit
> the comparison script and its output as research evidence before these numbers become
> planning facts. Until then, no release note or issue comment may cite them as measured.

> **Note on injectivity.** The claim that `ref/` IDs are injective over the pinned
> corpus is a design requirement, not a shipped property. Phase 1 must add a compiler
> test that derives IDs for every protocol-carrying record and fails on any collision.
> The 34,530 total input records and the 34,495 protocol-carrying subset are distinct
> numbers: publication and auxiliary records have no protocol route and therefore no
> `ref/` ID.

The gap counts are 342 racial traits, 171 afflictions, 526 rules, 355 monsters, and
486 other attribution gaps. They sum to 1,880 against 1,879 distinct legacy IDs, so at
least one legacy ID is claimed by two categories. The compiler must reconcile the set
union and report both numbers; it must not assume the category counts are disjoint.

The 9,968th entry is the supplemental `pf-data/rule/magic`. It also needs a migration
row, because bookmarks can already store that ID even though it was never PSRD-derived.

## 3. Namespace and derivation

### 3.1 The namespace is `ref/`

A canonical ID is:

```text
ref/<family>/<key>
```

`ref` is the reserved first segment. It stands for "reference entry": the canonical
reference layer of this library, independent of which dataset supplied the record.

It was chosen over the alternatives for these reasons:

- It does not name a dataset, a repository, a vendor, or a file format. The current
  `pf-data/` prefix names one upstream that may be replaced, forked, or joined by a
  second source. Renaming a vendor must not renumber bookmarks.
- It does not name a license or edition. `srd` would be wrong for OGL content that is
  not the System Reference Document, and would become wrong again if the corpus grows.
- It is short. Every canonical ID carries it in catalog rows, entry URLs, and search
  documents; three characters keep a shared prefix cheap and readable even if the
  corpus grows toward the 34,495-protocol-record review inventory.
- It is a valid `EntryId` segment and it is disjoint from the legacy namespace. Every
  legacy PSRD ID starts with a book slug such as `core-rulebook/`; no selected book is
  named `Ref`. The totality check asserts that the two namespaces do not intersect.
- A cutover is visible. A reader can tell from the URL whether a bookmark predates the
  canonical corpus, and the alias direction is unambiguous.

`carried` is reserved as the second segment. No route in `routes.json` may use it.

### 3.2 Upstream identity

The upstream identity is the record's `path#key` inside the pinned dataset, for
example `json/feat01.json#power_attack`. The public ID is a projection of that
identity:

- `family` is the reviewed protocol token for the file in
  `sources/pathfinder-data-1-e/routes.json` (`feat`, `spell`, `monster`, `rule`,
  `main`, and so on).
- `key` is the record key, slugged with the existing `slug_segment` contract.

The projection deliberately leaves out three things:

- **The publication.** Attribution can change, especially for the 486 attribution
  gaps. Identity must not follow attribution, or a reviewed attribution fix would
  renumber bookmarks.
- **The record title.** Titles are display data and can be corrected.
- **The dataset id.** The library has one pinned corpus and one manifest. If a second
  dataset is ever added, its routes join the same global family registry and the
  injectivity check runs over the merged inventory. Putting a dataset name in the ID
  would make a dataset rename a renumbering.

`family` is a route token, not an `EntryKind`. Some tokens look mechanical (`main`,
`umr`); that is expected. The token is already what `pf-data/{protocol}/{key}` uses,
and Phase 1 must verify that the pair is injective over every protocol-carrying record
(section 2); that check is a Phase 1 exit criterion, not a shipped property.

Path participates in the override key and in provenance, not in the slug. Moving a
record between files that share a family keeps its ID, which preserves the existing
"native identity survives file repartitioning" property. Moving it to a different
family changes the ID; that is an identity change and needs an alias.

### 3.3 Derivation

```text
canonical_id(record):
  identity = "path#key"
  if id_overrides.records[identity] exists:
      return parse(id_overrides.records[identity])     # must start with "ref/"
  family = reviewed_family(record.path)                # routes.json, reviewed
  key = slug_segment(record.key)
  if family is reserved or key is empty:
      fail
  return parse("ref/" + family + "/" + key)
```

Derived records extend the same shape:

```text
native:   ref/<family>/<key>
derived:  ref/<family>/<key>/<child-kind>/<child-slug>[/<child-kind>/<child-slug>...]
carried:  ref/carried/<legacy-id>
```

Examples (illustrative except the first two, which are current entries):

```text
core-rulebook/spells/fireball       -> ref/spell/fireball
core-rulebook/rules/magic           -> ref/main/magic
core-rulebook/races/dwarves/...     -> ref/race/dwarf/racial-trait/stonecunning
bestiary/monsters/example           -> ref/carried/bestiary/monsters/example
```

Two distinct entries share the bare word `magic` and must never be conflated: the
legacy Magic rules chapter `core-rulebook/rules/magic` maps to `ref/main/magic`
(family `main`, its Pf counterpart lives in `main.json`), while the reviewed supplement
`pf-data/rule/magic` (the page 206 opening passage) maps to `ref/rule/magic`
(family `rule`, its native protocol). They are different families and different
canonical IDs. The supplement is not a Pf record, so its `ref/rule/magic` claim must go
through the `Claims` collision path (section 4.7), not rely on the protocol derivation.

Structural disjointness makes injectivity easy to argue:

- Native IDs have exactly two segments after `ref`: `family`, then `key`.
- Derived IDs repeat the parent's `family` and `key`, then add a fixed child kind:
  `racial-trait`, `affliction`, `section`, or `row`, then the child slug.
- Carried IDs have the reserved second segment `carried`.

A native key cannot create a slash (`slug_segment` removes separators), so a native
ID cannot masquerade as a derived one. Two records still collide if they share a
family and key, or if two derived children share a parent, kind, and slug. Either
case is a build failure, not an automatic suffix.

### 3.4 Collisions

The compiler keeps a `Claims` map from canonical ID to identity, as the PSRD adapter
does today. On collision it fails with both identities and points at
`id-overrides.toml`. There is no `-2` suffix:

- A suffix depends on which record the compiler sees first, so a new upstream record
  could silently renumber an old bookmark.
- A suffix hides the fact that the source has two records whose identity the review
  has not separated.

The fix is one reviewed row in `sources/canonical/id-overrides.toml`.

### 3.5 Overrides and upstream renames

`sources/canonical/id-overrides.toml` is the canonical equivalent of
`sources/psrd-data/id-overrides.toml`. It is keyed by the upstream identity and pins
the public ID. It is append-only; a pinned ID is never reused for another record.

```toml
# Identity grammar:
#   native   <path>#<key>
#   derived  <path>#<key>|<child-kind>:<slug>[|<child-kind>:<slug>...]
# Percent-escape %, |, and # inside raw names. Slugs need no escaping.

[records]
"json/feat01.json#power_attack" = "ref/feat/power-attack"
"json/races.json#dwarf|racial-trait:stonecunning" = "ref/race/dwarf/racial-trait/stonecunning"
```

The rename procedure is one row:

1. Upstream renames `json/feat01.json#power_attack` to `#power_attack_melee`.
2. The base derivation would produce `ref/feat/power-attack-melee`.
3. Add `"json/feat01.json#power_attack_melee" = "ref/feat/power-attack"` to
   `[records]`.
4. The old ID stays valid, and the new record adopts it.

Forgetting step 3 is caught by the golden snapshot (section 8.1): the old canonical
ID disappears and the build fails. A rename can therefore never be silent.

The same table pins one side of a slug collision and stabilizes a derived child whose
display name is edited. That is the exact protection
`sources/psrd-data/id-overrides.toml` gives PSRD IDs against upstream URL changes.

### 3.6 Family registry

`routes.json` is already a pinned, reviewed file with 399 entries. It becomes the
family registry, with two additions:

- A build check that every route is a valid `EntryId` segment and is not reserved.
- A review note in `docs/data-migration.md` that a route change is an identity
  change: it needs `[records]` rows for every affected record, or it will fail the
  golden check.

Changing `routes.json` without the rows is a build failure, not a corpus edit.

## 4. Derived-record layer

### 4.1 Common contract

Every derived record is a normal schema-3 `RuleEntry`. No new kind, field, or block
type is added. The rules are:

- The record has a canonical ID from section 3 and a `parent` when the source has a
  containing record.
- `provenance` names the pinned dataset, the upstream file, and a derived identity
  string. The record string is
  `"<path>#<key>|<child-kind>:<slug>"`, so a later reader can reproduce it.
- `source_ids` and `license_refs` are inherited from the parent record. A derived
  record does not invent attribution and does not mix the Pf Data license with the
  PSRD carry-over license.
- A `Relation::Part` (or the existing relation kind) is added only when the current
  corpus already models that edge. The derived layer must not invent graph edges.
- Every rejected derivation goes to `coverage.json` with the identity and the reason.
  A rejected derivation never aliases the legacy ID to the parent; the legacy ID goes
  to the retired list instead.

### 4.2 Racial traits (342 legacy IDs)

**Input shape.** A race record in a `race`-family file. The record body is a Pf Data
Markdown description. Standard traits appear under a racial-traits heading; alternate
traits appear under an alternate-traits heading, usually as `###` sections or a bold
lead-in followed by prose and an ability-score line.

**Transformation.** Split the body into trait sections. For each section:

- title = heading or bold lead-in;
- body = the section's prose, converted by the existing Markdown-to-typed-node path;
- optional ability-score line = `RacialTraitFields` description;
- `alternate` = whether the section is under the alternate heading;
- `parent` = the race's canonical ID;
- `Relation::Part` to the race with label `alternate` when alternate.

The legacy `racial_trait` record supplies the expected name and parent for the alias
row; it does not supply body text.

**Key rule.**
`ref/<race-family>/<race-key>/racial-trait/<trait-slug>`.
`alternate` is not part of identity, so moving a trait between standard and alternate
sections does not renumber it.

**Failure mode.** No parseable heading; a duplicate trait name within one race; an
empty slug; unsupported directives inside the trait text; or a parent race that was
itself carried over. Each is reported per trait. The legacy ID is retired as
`unrepresentable` rather than aliased to the race, because that would send a bookmark
to the wrong article.

### 4.3 Afflictions (171 legacy IDs)

**Input shape.** Inline stat-block lines inside a creature or rule record, for example
a bold name with `(disease)` and labelled lines for save, onset, frequency, effect,
and cure. The parent record owns the surrounding prose.

**Transformation.** Parse name, type, and labelled lines into `AfflictionFields`.
Emit `EntryKind::Affliction` with the remaining prose as body and the parent set to
the containing record. Do not duplicate the inline copy in the parent body: replace it
with an entry link, as the PSRD compiler does for nested typed copies.

**Key rule.**
`ref/<parent-family>/<parent-key>/affliction/<name-slug>`.

**Failure mode.** Unrecognized inline syntax, two afflictions in one paragraph, a
missing name, or a labelled field that cannot be typed. The parser never guesses. A
recognized affliction whose name collides with another derivation under the same
parent also fails and needs an override.

### 4.4 Rules promotions (526 legacy IDs)

**Input shape.** Nested sections (`##`/`###` headings) and table rows inside a
`rule`/`main` record that PSRD publishes as separate entries.

**Transformation.** Promote a section to a `Rule` entry when, and only when, a legacy
ID claims it. The promoted entry gets the section body, a parent link to the containing
record, and a heading plus entry link left in the parent. A table row is promoted only
when it has a name cell and rules-bearing text; its body is the row's cells rendered as
a definition list, not raw HTML. The allowlist is derived from the legacy inventory, so
the cutover cannot inflate the corpus with thousands of invented headings.

**Key rule.**

```text
section: ref/<family>/<parent-key>/section/<heading-slug>
row:     ref/<family>/<parent-key>/row/<row-slug>
nested:  ref/<family>/<parent-key>/section/<heading-slug>/row/<row-slug>
```

A row directly under the record uses the second form; a row inside a promoted section
uses the third. The deeper form exists so that a row keeps its ancestry, but it is not
required when there is no promoted section.

**Failure mode.** Duplicate headings under one parent; empty headings; rows with merged
cells or no name; a promoted section that duplicates a native record. A native record
wins; the legacy ID aliases to the native ID instead. Anything else is reported and
retired, not guessed.

### 4.5 Carried-over monsters (355 legacy IDs)

**Input shape.** The existing production `RuleEntry` and its PSRD provenance. Pf Data
has no record, or only an aggregate that loses distinctions the legacy entry keeps.

**Transformation.** Re-emit the already-typed entry under a canonical ID. Body, fields,
relations, source ids, and license refs are unchanged. Provenance keeps the PSRD
dataset and URL, and the coverage report records `carried-over: no Pf Data record`.
The record is a first-class canonical record, not a hidden compatibility shim; it
renders, searches, and links normally.

**Key rule.** `ref/carried/<legacy-id>`. The legacy ID is already a valid slug path,
so this is deterministic and injective by construction.

**Failure mode.** The legacy ID collides with a canonical ID (must not happen; the
totality check asserts it); the content does not validate under schema 3; the
publication mapping is missing; or the rights review rejects it. Then it is retired
with a reason and a note, and it appears in coverage.

An aggregate is many:1 only after a text-preservation check. If the Pf aggregate drops
distinct stat-block text, carry the individual legacy records instead. Do not point
several bookmarks at one record that says less than they did.

### 4.6 Other attribution gaps (486 legacy IDs)

**Input shape.** A legacy ID whose Pf counterpart exists by title and kind but is
attributed to a different publication, or whose correspondence is not proven.

**Transformation.** Per-record decision, in this order:

1. If body evidence proves the same record, map it with `record = "<path>#<key>"`.
   The alias is direct; the entry's attribution becomes the Pf attribution. That
   attribution change is a product decision and is recorded in the map note.
2. If the content is the same but the attribution cannot be accepted, carry the
   legacy record with the 4.5 mechanism.
3. If the content is out of scope or rights-restricted, retire it with a reason.

**Key rule.** The native rule, the carried rule, or retirement; no fourth mechanism.

**Failure mode.** A title-only match is forbidden. The audit that proposes these rows
must show body evidence; a row without evidence stays `no_source` and blocks the
release decision list. Do not silently prefer the Pf attribution.

### 4.7 Gap accounting

All counts in this table are unconfirmed estimates from an analysis script that is not
yet committed. Phase 0 must confirm them and commit the script and output. See the note
in section 2.

| Legacy category | Count | Status in this design | Canonical origin |
|---|---:|---|---|
| Pf counterpart, native | 8,088 | direct alias | `ref/<family>/<key>` |
| Supplemental entry via Claims | 1 | remap through the Claims path, not a Pf record | `ref/rule/magic` (the opening passage, not the Magic article) |
| Racial traits nested in race bodies | 342 | derivable, subject to parser acceptance | `ref/race/.../racial-trait/...` |
| Afflictions inline in stat blocks | 171 | derivable, subject to parser acceptance | `.../affliction/...` |
| Rules nested as sections or rows | 526 | derivable, allowlisted | `.../section/...` or `.../row/...` |
| Missing or aggregated monsters | 355 | carry-over; many:1 only after text check | `ref/carried/...` |
| Other attribution gaps | 486 | evidence, then direct or carry-over | native or `ref/carried/...` |
| **Reconciled total** | **9,968** | 8,088 direct + 1,879 distinct gaps + 1 supplemental | |

The raw category rows sum to 9,969, not 9,968, because one gap ID is counted by two
categories (1,880 vs 1,879) and the supplemental row is outside the 9,967 PSRD count.
The reconciled total is the union, and the build report prints both numbers.

The supplemental remap `pf-data/rule/magic` → `ref/rule/magic` must go through the same
`Claims` collision path as every derived record (section 3.4). It is not a Pf record, so
it cannot rely on the protocol derivation to avoid a collision: a Pf `rule`-family
record with key `magic` would otherwise claim `ref/rule/magic` silently. Route the
supplemental through the claim map, or pin it with an explicit row in
`sources/canonical/id-overrides.toml`.

Derivable by this design: 342 + 171 + 526 = 1,039, minus any recorded overlap.
Carry-over or product decision: 355 + 486 = 841, minus any of the 486 that become
direct after evidence. The category counts sum to 1,880 against 1,879 distinct IDs;
the compiler reports the union, not the sum.

This is the honest limit: 1,039 legacy IDs have a construction path; 841 have no
construction path in this design and need evidence, carry-over, or a product decision.
No sentence in the release notes may say the 1,879 gaps are closed until the totality
check shows every one of the 9,968 IDs in exactly one alias or retirement row.

## 5. Legacy alias map

### 5.1 Artifacts

| Artifact | Committed | Purpose |
|---|---|---|
| `sources/canonical/legacy-ids.json` | yes, frozen | The 9,968 production IDs with title, kind, and source. Immutable; the totality oracle. |
| `sources/canonical/id-map.toml` | yes, append-only | Reviewed disposition of every legacy ID. The reviewable alias golden. |
| `generated/content/<version>/aliases.json` | no | Runtime map, generated deterministically from `id-map.toml` and the canonical catalog. |
| `sources/canonical/golden-ids.json` | yes | Sorted canonical catalog snapshot for review (section 8.1). |
| `sources/canonical/new-records.toml` | when needed | Reviewed canonical entries with no legacy counterpart, so corpus growth is never implicit. |
| `sources/canonical/title-exceptions.json` | yes | Reviewed title corrections and overrides (section 8.6). |

`legacy-ids.json` is exported once from the last PSRD build. It is never regenerated;
a later change to it is a deliberate, reviewed edit with a reason, because it is the
only complete list of IDs that real bookmarks may hold.

### 5.2 `id-map.toml`

One array-of-tables entry per legacy ID, sorted by `old`:

```toml
[[map]]
old = "core-rulebook/rules/magic"
record = "json/main.json#magic"

[[map]]
old = "core-rulebook/races/dwarves/dwarf-racial-traits/stonecunning"
derived = "json/races.json#dwarf|racial-trait:stonecunning"

[[map]]
old = "bestiary/monsters/example"
target = "ref/monster/example"
note = "aggregate keeps both stat blocks; text check passed"

[[map]]
old = "bestiary/monsters/example-variant"
target = "ref/monster/example"
note = "same aggregate as the previous row"

[[map]]
old = "core-rulebook/comics/example"
retired = "rights"
note = "Product Identity; #34"
```

Exactly one of `record`, `derived`, `target`, or `retired` is present. Unknown fields
and duplicate `old` values fail the build. `record` and `derived` are resolved through
the canonical derivation at build time; `target` is an explicit canonical ID and must
exist in the catalog.

The `record` and `derived` values above are illustrative. The reviewed map is the
source of truth; the audit proposes rows and a person accepts them.

### 5.3 `aliases.json`

```json
{
  "format": 1,
  "legacy": {
    "count": 9968,
    "sha256": "<sha256 of legacy-ids.json>"
  },
  "entries": [
    { "old": "core-rulebook/rules/magic", "target": "ref/main/magic", "basis": "direct" },
    { "old": "core-rulebook/races/dwarves/dwarf-racial-traits/stonecunning", "target": "ref/race/dwarf/racial-trait/stonecunning", "basis": "derived" },
    { "old": "bestiary/monsters/example-variant", "target": "ref/monster/example", "basis": "shared" }
  ],
  "retired": [
    { "old": "core-rulebook/comics/example", "reason": "rights", "note": "Product Identity; #34" }
  ]
}
```

Rules:

- `entries` and `retired` are sorted by `old`; no map iteration, no timestamps.
- `format` is independent of `SCHEMA_VERSION` and `INDEX_FORMAT_VERSION`.
- `basis` is one of `direct`, `derived`, `carried`, `shared`, `renamed`. The client
  does not branch on it; it is for review and for the Settings/coverage counts.
- `retired.reason` is one of `rights`, `out_of_scope`, `no_source`,
  `unrepresentable`.
- A target must exist in the same library's catalog. A target must not itself appear
  as an alias `old`; chains are forbidden so resolution is one step.
- A legacy ID appears in exactly one of `entries` and `retired`.
- `legacy.sha256` binds the file to the frozen snapshot.

### 5.4 1:1 and many:1

1:1 is the normal case: one legacy ID, one canonical target.

Many:1 is explicit. Several legacy IDs may name the same target when the reviewed
disposition says they are the same article or one aggregate. Every such row is
`target` with a note; the generator marks it `shared` when another row already names
the target. Two bookmarks that collapse onto one target are merged by the client rule
in section 6.3; no bookmark is lost and none points at a different article.

Many:1 is not a way to make a gap disappear. A shared target needs either identical
content or a reviewed aggregate whose text check passed.

### 5.5 Retirement

A legacy ID that is not covered by a canonical record is retired, not dropped:

- `rights`: removed for licensing or Product Identity.
- `out_of_scope`: publication, record type, or edition outside the PF1e OGL scope.
- `no_source`: no canonical record and no approved carry-over. This is a blocking
  product decision, not a silent exclusion.
- `unrepresentable`: the content cannot be expressed in schema 3 without an
  incompatible change. The alternative is a schema bump, which this design rejects.

Retired IDs stay in the map forever and are never reassigned. The bookmarks view shows
the saved title with "not in this library"; the stored row is not deleted. The release
report lists retirement counts by reason.

### 5.6 Generation and review

1. `cargo xtask canonical-audit` proposes rows from the legacy snapshot and the Pf
   inventory: exact title/kind/source matches first, then body-evidence candidates,
   then unresolved. It reproduces the 8,088 direct baseline before anyone accepts a
   map.
2. Reviewers accept or change every proposed row in `id-map.toml`; ambiguous rows are
   never accepted automatically.
3. The content build resolves `record` and `derived` rows through the canonical
   derivation, validates `target` rows, and writes `aliases.json`.
4. The audit is not part of the ordinary build. Only the committed map is. That keeps
   fuzzy matching out of release output and makes alias changes diffable.

## 6. Client resolution and bookmark migration

### 6.1 Loading

`crates/schema` gains `ALIASES_FILE`, `AliasIndex`, `AliasEntry`, and
`RetirementReason`. Existing serialized structs do not change.

`crates/web/src/library.rs` reads `aliases.json` in `Library::load` when the manifest
lists it. An older library has no file and gets an empty index. The map is small
(about 1 MB for 10,000 rows) next to a 62 MB library; it loads eagerly so that
`resolve` is synchronous and the bookmarks view cannot race an async read. If a
measurement shows more than 20 ms on a mid-range phone, it moves behind the existing
lazy pattern and the callers become async; that is an implementation detail, not a
format change.

A corrupt alias file fails the load like a corrupt `legal.json`, because the installer
verified its hash. An absent file is normal.

### 6.2 Resolution

```rust
pub enum Resolution {
    Canonical(EntryId),
    Aliased { canonical: EntryId, legacy: EntryId },
    Retired { legacy: EntryId, reason: RetirementReason },
    Unknown(EntryId),
}

impl Library {
    pub fn resolve(&self, id: &EntryId) -> Resolution;      // one step, no chains
    pub async fn entry_resolved(&self, id: &EntryId) -> Result<RuleEntry, LibraryError>;
}
```

`stub` and `entry` keep their strict canonical contract. Resolution is explicit at the
call sites so that no internal caller starts depending on legacy IDs.

Caller behavior:

- **Entry route.** `#/entry/<old>` resolves. On `Aliased`, replace the route with the
  canonical ID, keeping the anchor, so the address bar updates without a Back entry.
  On `Retired`, show the standard "not in this library" empty state with the reason
  category. Never redirect to a guessed article.
- **Bookmarks view.** Resolve every stored row. Deduplicate by canonical target for
  display; an aliased row shows the target's current title, while a retired or unknown
  row shows `title_snapshot` and the existing "not in this library" tag.
- **Recent view.** The same resolution on read; rewrite the stored list on the next
  push.
- **Search and browse.** They only emit canonical IDs from the catalog and index.
- **Star toggle.** Pressed means "some stored row resolves to this canonical ID".
  Unstar deletes every row that resolves to it, so a not-yet-migrated legacy row
  cannot resurrect.

### 6.3 IndexedDB migration

No `DB_VERSION` bump. The object stores and key paths do not change; this is a data
rewrite, not a schema change. A version bump would force an upgrade path with nothing
to create.

Add `store::migrate_entry_ids(index, aliases_sha256)`:

1. Begin one read-write transaction over `bookmarks` and `meta`.
2. Read all bookmarks. If `meta["id_migration"].aliases_sha256` already equals the
   current value, commit and stop.
3. Resolve each row. Group aliased rows by canonical target.
4. Merge each group:
   - `entry_id` = target;
   - `added_at` = minimum in the group;
   - `collection_ids` = sorted union;
   - `kind` = the target's catalog kind when it exists, else the earliest row's kind;
     ties on `added_at` break on the lexicographically smallest kind, so the result
     does not depend on read order;
   - `title_snapshot` = the earliest row's snapshot; ties break on the
     lexicographically smallest title, so the result does not depend on read order.
5. Put one row per target and delete every original key in the group.
6. Rewrite `meta["recent"]` with the same resolution and dedupe, newest first.
7. Write `meta["id_migration"] = { "aliases_sha256": "..." }` in the same transaction.

Retired and unknown rows are left untouched. They already render from their snapshot.

The marker is the alias file hash, not the content version, because the map is
append-only: a new alias can make a previously retired bookmark resolvable. The
transaction is idempotent; a failure before commit leaves every row as it was, and the
next load retries.

If IndexedDB refuses the transaction (quota, private mode, interrupted upgrade), the
library still loads. `resolve` makes every read correct without the rewrite, the UI
shows a non-blocking "saved items will be converted when storage is available" notice
in Settings, and the next load retries. This is intentionally different from
`disabled_books`, whose read failure blocks the load because there is no correct
fallback.

For a very large bookmark set, chunking is allowed only if each chunk is idempotent and
the marker is written last; the merge rule above is idempotent, so a retry after a
partial commit produces the same rows.

### 6.4 Import and export

If the star-transfer path is present, imported rows go through the same resolution and
merge before `put`. An old export from another device therefore imports into canonical
IDs. A retired row is kept with its snapshot and reported in the import summary, not
silently dropped.

### 6.5 Shell and library release order

The app shell and the library update independently. An old shell reading a canonical
library can open entries from the catalog, but it cannot resolve old bookmarks; the
rows show "not in this library" until the shell updates, and the new shell then
migrates them. The new shell reading an old library is fully compatible.

Therefore the alias-capable shell must be published and verified before the canonical
library. The library is only fetched when the user asks, so the exposure is bounded.
This sequencing is a release requirement, not an implementation detail.

## 7. Versioning

`SCHEMA_VERSION` stays 3 and `INDEX_FORMAT_VERSION` stays 1.

- No struct in `crates/schema` changes shape. `RuleEntry`, `EntryStub`, `Catalog`,
  `RelationIndex`, `BookIndex`, `Provenance`, and `Bookmark` are untouched. Only the
  string values of IDs change. The schema version describes the serialized shape, not
  which values a slug function produces.
- `aliases.json` is a new optional manifest file. Old clients download and hash it
  because the manifest lists it, but they never open it; new clients treat absence as
  an empty map. This is the same optional-file mechanism as `relations.json`,
  `lists.json`, and `books.json`.
- Search encoding is unchanged. Document IDs are strings either way; only their values
  change. `INDEX_FORMAT_VERSION` describes the binary layout.
- `Bookmark` is unchanged, so the IndexedDB record shape is unchanged.

A bump becomes necessary if any of these are attempted:

- embedding a `legacy_ids` field in `catalog.json` or `EntryStub`;
- changing an entry's ID to a structure instead of a string;
- changing the search document or posting encoding;
- changing the `Bookmark` record shape.

This design forbids all four for the cutover. If a future alias format cannot be read
additively, bump its `format` and `SCHEMA_VERSION` together so the existing activation
gate rejects the library before it becomes active.

## 8. Verification

### 8.1 Golden snapshot

`sources/canonical/golden-ids.json` is a sorted snapshot of every canonical entry:
`id`, `kind`, `title`, `source`, `parent`, and `origin`
(`native`, `derived:<kind>`, `carried`). It is generated from the compiled corpus and
committed. A canonical check regenerates it and fails on any difference; `cargo xtask
check` runs it against the fixture corpus and the CI content job runs it against the
real corpus. A change needs a reviewed diff in the commit, which is where an
accidental renumbering or a silent disappearance becomes visible.

Alias changes have their own review surface: `id-map.toml`. A generator test asserts
that `aliases.json` is byte-identical across runs and matches the map.

### 8.2 Totality

Using `legacy-ids.json` as the oracle:

- every legacy ID appears exactly once in `entries` or `retired`;
- no alias `old` is duplicated;
- `entries.len() + retired.len() == legacy.count == 9,968`;
- the per-category counts are printed: direct, derived, carried, shared, each
  retirement reason;
- the set union of the 1,879 gap categories is reported next to the 1,880 sum, and
  any difference is explained in the report.

The release fails if any legacy ID is unaccounted for.

### 8.3 Target existence and direction

- Every `entries[].target` exists in the catalog.
- No target appears as an `old`.
- No legacy ID equals a canonical ID (the namespaces are disjoint).
- `target` rows marked `shared` name a target that at least two rows use; an
  accidental one-to-many row is a review error.
- Every canonical entry is either the target of at least one legacy row or listed in
  `sources/canonical/new-records.toml` with a reason. This prevents the cutover from
  quietly adding or dropping records.

### 8.4 Injectivity

Unit tests over the real pinned inventory, skipped when the dataset is absent:

- the derivation returns a distinct ID for each of the 34,495 protocol-carrying input identities;
- derived children do not collide with native records or with each other;
- carried IDs are unique and start with `ref/carried/`;
- the catalog has no duplicate IDs after compilation.

The test also proves the structural argument in section 3.3: native IDs have two
segments after `ref`, derived IDs repeat the parent and add a fixed child kind, and
carried IDs use the reserved segment.

### 8.5 Byte-identical builds

Run `cargo xtask content` twice and compare the tree hash, as CI already does. The
alias generator must:

- iterate `BTreeMap`/sorted vectors, never hash maps;
- serialize with a fixed key order and a trailing newline;
- include no timestamp, build path, host name, or environment value;
- derive no ID from a counter or from discovery order.

A rename, a new route, or a collision must fail the build rather than change an ID
under the same inputs.

### 8.6 `scripts/check-library-upgrade.py`

The script currently fails when an old ID is missing. Under the cutover that is the
expected state, so it must learn to resolve:

1. Load `aliases.json` from the new library.
2. For each old ID, accept it if it is present directly, resolves through one alias to
   a present entry, or is in `retired`.
3. Fail on a missing alias target, a chain, an old ID in both lists, or a legacy ID in
   neither.
4. Compare titles of direct and aliased entries with the frozen snapshot. Legitimate
   title corrections are listed in `sources/canonical/title-exceptions.json` with a
   reason; the script fails on any other change.
5. Report counts by basis and retirement reason, plus `changed_kinds` as it does today.
6. Run twice against the same libraries to prove idempotence.

The legacy snapshot makes the check meaningful even when the previous library is no
longer on disk.

### 8.7 Real browser upgrade test

Add `tests/e2e/tests/canonical-cutover.spec.ts` against a real `dist/` build:

1. Install a predecessor library that uses legacy IDs. The test fixture generator
   writes a small schema-3 library with one entry from each category: a direct match,
   a derived racial trait, a derived affliction, a promoted rule, a carried monster, a
   many:1 pair, and a retired ID.
2. Star all of them. Star the same target twice through two legacy IDs to exercise the
   merge.
3. Overlay the canonical library and install it through the normal update flow.
4. Assert that every star opens the intended canonical entry; the many:1 pair shows
   once; the retired row shows its saved title with "not in this library"; the star
   toggle is pressed on each canonical page.
5. Open the old URL directly (`#/entry/<legacy-id>`) and assert the address bar becomes
   the canonical URL and the article renders.
6. Reload with the network off and repeat the checks. Assert no console error and no
   third-party request.
7. Interrupt the update once and confirm the old library and its stars remain usable.

The same spec must keep the existing `update.spec.ts` behavior, including
`#/entry/core-rulebook/spells/fireball` after the update. That old URL is the simplest
end-to-end proof that a legacy bookmark survives.

The full 9,968-entry upgrade is a release rehearsal, not the Playwright fixture: save
the previous library outside `generated/` and `dist/`, install it, star representative
entries in every category, activate the canonical build, and repeat offline. Follow the
`verify` skill for the corpus and schema checks, and record local, CI, and Pages
results separately.

### 8.8 Release gates

- `cargo xtask check` passes.
- Two content builds are byte-identical.
- Totality, target, direction, and injectivity checks pass on the real corpus.
- `check-library-upgrade.py` reports zero missing, zero dangling, and zero unexpected
  title changes.
- The cutover spec passes in Chromium and WebKit on the built distribution.
- The alias-capable shell is deployed before the canonical library.
- The previous library is retained for rollback.

## 9. Phased plan

| Phase | Work | Depends on | Rough effort |
|---|---|---|---:|
| 0 | Freeze `legacy-ids.json` from the last PSRD build; confirm 9,968 / 9,967 / 8,088 / 1,879; reconcile the 1,880-vs-1,879 overlap | none | 2-3 d |
| 1 | Canonical derivation module (`ref/`, family registry, overrides, claims), unit tests, injectivity over 34,495 protocol-carrying records | 0 | 3-4 d |
| 2a | Racial-trait derivation, fixtures, coverage | 1 | 2-3 d |
| 2b | Affliction derivation, fixtures, coverage | 1 | 2-3 d |
| 2c | Rule section and row promotion, allowlist, fixtures, coverage | 1 | 4-6 d |
| 3 | Carry-over pipeline for 355 monsters and unresolved rows; provenance and license separation | 1 | 3-5 d |
| 4 | `canonical-audit` proposal; review `id-map.toml`; generator for `aliases.json`; many:1 rules | 0, 2a-2c, 3 | 6-10 d |
| 5 | Schema alias types; `Library::resolve`; route redirect; bookmark/recent migration; dedupe and unstar | 4 format frozen (code can start earlier) | 4-5 d |
| 6 | Golden snapshots; upgrade-script extension; fixture pair; Playwright cutover spec | 4, 5 | 3-4 d |
| 7 | Full-corpus rehearsal: two builds, real browser update, offline reload, rollback, `docs/status.md` | 6 | 2-3 d |
| 8 | Publish alias-capable shell, then canonical library; monitor | 7 | 1-2 d |

Total: roughly 32-48 engineer-days. Phases 2a-2c and 3 run in parallel after phase 1.
Phase 5 can start once the alias format in section 5 is frozen, in parallel with the
phase 4 review. Phase 4 is the long pole because every one of the 8,088 direct rows and
every one of the 1,879 dispositions is reviewed evidence, not a heuristic at build
time.

## 10. Risks, limitations, and open decisions

- **The gap is not closed.** 1,039 legacy IDs have a designed derivation path, 841 need
  carry-over or a product decision. The design's success criterion is totality, not a
  claim that every gap is derivable.
- **Parsers can fail.** The 342/171/526 numbers are structural opportunities, not
  accepted records. Each rejected derivation becomes a retirement row and must be
  reviewed.
- **Aggregates can lose text.** A many:1 monster row needs a text-preservation check;
  otherwise carry the individual entries.
- **Attribution changes.** The 486 attribution rows may change a book's source while
  keeping the article. That is a product decision; the ID stays stable either way
  because publication is not part of identity.
- **Derived names are content-derived.** A trait or row rename changes the natural
  slug. The golden snapshot turns that into a build failure, and the override keeps the
  old ID. Reviewers must not "fix" it by accepting a new hash.
- **Old shell plus new library.** Old bookmarks show as missing until the shell
  updates. The release order in section 6.5 is the mitigation; the stored rows survive.
- **Map growth and review cost.** The alias file is append-only and will grow with each
  later rename. That is the price of permanent bookmarks. The alternative, a short
  compatibility window, breaks saved state and is rejected.
- **Open product decisions.** Carry versus drop for the 355 monsters; accept or reject
  attribution for the 486; user-facing copy and Settings reporting for retired rows;
  whether carried records are labelled as such in coverage.
- **Licensing.** Derived records inherit the Pf record's publication and license refs;
  carried records keep PSRD provenance and license refs. The legal page must keep both
  dataset pins and both sets of notices. This design does not merge licenses and does
  not add a source.

## 11. References

- `docs/spec.md`, identifier strategy.
- `docs/data-migration.md`, migration and cutover requirements.
- `docs/decisions.md`, schema-3 and bookmark decisions.
- `docs/status.md`, current counts and verification scope.
- `crates/content-compiler/src/pf_data.rs`, the existing native-ID derivation.
- `crates/content-compiler/src/build.rs`, `id_for` and the collision claim map.
- `sources/psrd-data/id-overrides.toml` and `crates/content-compiler/src/config.rs`,
  the protection this design mirrors.
- `scripts/check-library-upgrade.py` and `.agents/skills/verify/SKILL.md`, the upgrade
  checks this design extends.
