# Pf Data 1e source review

Reviewed snapshot: `jasontankapps/pathfinder-data-1-e` commit
`08ea71f24375908d0190c822fd1fd080797667a9`. This review establishes source-level
evidence. It does not classify every passage in the corpus for publication.

## Primary source

The [README at the pin](https://github.com/jasontankapps/pathfinder-data-1-e/blob/08ea71f24375908d0190c822fd1fd080797667a9/README.md)
identifies OGC in `json/` and separates non-OGC application content under GPL-3.0-or-later.
The [OGC notice](https://github.com/jasontankapps/pathfinder-data-1-e/blob/08ea71f24375908d0190c822fd1fd080797667a9/OPENGAMECONTENT.md)
contains OGL 1.0a, Section 15 credits, and Paizo's Product Identity exclusion.
GitHub's absent recognized license metadata does not override these declarations.

The OGC grant supports converting designated game mechanics into Cast Iron's
canonical format. It does not authorize arbitrary setting prose, artwork, or
other excluded Product Identity. The existing [licensing audit](licensing-audit.md)
explains the project's use of Paizo's Community Use Policy and its limits.
Cast Iron must preserve complete applicable notices and screen imported text.
Upstream renderer code is not being copied. `routes.json` contains factual
file-to-protocol associations extracted without executing that renderer.

Production imports exactly one reviewed record from this pin:
`pf-data/rule/magic`, the Core Rulebook page 206 opening passage, which is
ordinary OGC game mechanics under the OGL. Its `approved.json` row records
`rights: "ogl"`, so the compiler emits the canonical `ogl-1.0a` license
reference. The compiler appends the pin's complete Section 15 list to the legal
page and records the dataset. Every other record remains excluded from the
library. This does not expand the rights review to the rest of the corpus.

## Per-record rights basis

Each row in `sources/pathfinder-data-1-e/approved.json` must carry a required
`rights` value. The adapter accepts four values and rejects every other value:

- `ogl`: import as Open Game Content with the `ogl-1.0a` license reference.
- `cup`: import with both `ogl-1.0a` and the Community Use notice path on the
  legal page, `legal#community-use`.
- `orc`: refuse. The current legal page has no ORC notice path, so ORC text must
  not be presented under the OGL.
- `excluded`: do not import. The record stays visible in coverage with the
  reviewed reason from its `reason` field.

The rights value is a separate gate from `out-of-scope.json`. A record that
names an allowed import route still needs its publication, Product Identity, and
notice review; the field records the basis, not a blanket clearance.

The source registry includes `Monster Core`, and upstream has a separate
`ORCLICENSE.md` for that publication. It is not cleared by the OGL notice.
Keep its content outside the OGL publication path until independently reviewed.

The pinned OGL notice includes Adventure Path #144, Druma, Profit and Prophecy,
Chronicle of Legends, Cradle of Night, and Planar Adventures. Direct text searches
did not find Spiral of Bones or Hodgepodge from the Grand Lodge in that notice.
The corresponding mandatory records exist, but publication requires checking the
applicable source credits and permission route. A successful experimental parse
does not resolve this licensing question. The blog permission route and comic
game-content notices need separate evidence.

## Supplementary sources

No supplementary records have been imported from these sources.

| Repository and reviewed revision | Evidence and limit |
|---|---|
| `baileymh/pf1e-content`, `baf5232c5dc16af99d49ae1bf57ead6473b46bbb` | GPL repository metadata, OGL and legal notices, and Foundry content packs. A code license does not establish record-level game-content provenance or permission for PI. |
| `baileymh/statblock-library`, `877283f251d6e53418b8a20573b2b075c93bf954` | GPL repository metadata, OGL and legal notices, and adventure/NPC statblocks. Source attribution and PI screening remain necessary. |
| `foundryvtt_pathfinder1e/foundryvtt-pathfinder1`, `ad38c8aa2a7bbe282b288ae232aa445756ff53b9` | README separates GPL software from OGL game content. The inspected OGL file does not supply a complete book-specific Paizo notice chain for arbitrary packed records. |
| `devonjones/PSRD-Data`, `76ea4784604145ba3dea40eb56f83bb904759467` | Existing reviewed fallback and permanent-ID baseline. Its scope and limitations remain in the original licensing audit. |

Primary evidence for the supplementary reviews is each repository's README,
`OGL.txt`, and `Legal.txt` at the named revision. Repository descriptions and
advertised statblock counts do not prove coverage or redistribution permission.

## Out-of-scope PF2 material

Issue [#34](https://github.com/phynics/cast-iron/issues/34) tracks the unresolved
rights decisions. The pinned dataset is not purely PF1e OGL material:

- `json/sources.json` registers two PF2 books: `monster_core` ("Monster Core")
  and `bestiary_3_2nd_edition` ("Bestiary 3 (2nd Edition)").
- Eight records are tagged with them: `json/monster_families.json`
  (`alghollthu`, `dark_folk`, `dinosaur`, `drake`), and
  `json/monster_families2.json` (`genie`, `naga`, `troll`, `eurypterid`).
- `json/main.json#ogc` is a legal notice containing both OGL and ORC text.
- `json/equipment_weapons2.json#gnome_flick_mace` has no PF1e description; its
  imported prose is taken from PF2 *Player Core* page 279.
- The dataset carries `ORCLICENSE.md` and `GNULICENSE.md` separately from the
  OGL notice.

`sources/pathfinder-data-1-e/out-of-scope.json` excludes both publications, the
legal notice, and the untagged PF2 description before any selection. The inventory
still reports them, and the migration report lists them as out-of-scope input. No
PF2 record or ORC notice enters the PF1e library or the OGL legal page. The source
pin hashes the ORC and GNU license files so a changed notice cannot pass unnoticed.

Records whose only publications are comics or Paizo Blog posts are also out of
scope. Comics are excluded permanently; blog-only records are deferred to issue
#36. A record that also appears in an in-scope PF1e book stays, so shared PF1e
material is not lost.

The filter is clean for known contamination. Residual risk is untagged PF2 prose
that does not name its edition; the audit found no other instance, but a bulk
cutover still needs per-record review. `json/faiths4.json#grandmother_spider`
mentions PF2 Anadi in a footnote but is PF1e *Faiths of Golarion* content, so it is
not excluded.

## Product Identity screen

The product owner chose fidelity under the Community Use Policy on 2026-11-04.
Proper names inside game mechanics stay as descriptive references. Every Pf Data
entry carries `ogl-1.0a` and `legal#community-use`. The reviewed section rules in
`sources/pathfinder-data-1-e/pi-screen.json` omit deity `Description` lore, unique
creature `Description` biographies, and named NPC stories, and remove outbound
links. Each omission is a coverage row. See
[decisions](../decisions.md#product-identity-screen-for-the-pf-data-corpus-2026-11-04).

## Section 15 notice coverage

`cargo xtask data-legal` (also run by `cargo xtask data-report`) checks every
publication referenced by an in-scope record against the pinned
`OPENGAMECONTENT.md` Section 15 list. It also writes
`reports/migration-legal.json` and adds `legal` counts to
`reports/migration-coverage.json`. The report is deterministic and sorted. This
tool is evidence only: it does not change the production legal page until the
canonical Pf Data library is built.

The 2026-11-04 snapshot has 34,497 in-scope records referencing 453
publications. 397 publications match a Section 15 entry; 56 do not. Matching
uses, in order:

1. Exact normalized title (`PRPG` is not expanded here).
2. A reviewed alias from `sources/pathfinder-data-1-e/notice-aliases.json`,
   whose target must exist in the pinned notice.
3. A deterministic normalizer that removes the known line prefixes
   (`Pathfinder Roleplaying Game`, `Pathfinder RPG`, `Pathfinder Player
   Companion`, `Pathfinder Campaign Setting`, `Pathfinder Module`,
   `Pathfinder Companion`, `Pathfinder Chronicles`, `Pathfinder Adventure
   Path`, `Pathfinder Society`, `GameMastery Module`, `Pathfinder`, `PRPG`),
   volume/scenario words, module codes such as `J1`, a leading article after an
   adventure number, and treats `and` and `&` as the same connector. Every
   distinct notice under one title key applies, so the Core Rulebook keeps both
   its 2009 and 2010 printing notices.

Exact and normalized matching accounted for 387 publications (44 exact, 343
normalized-prefix). Ten publications needed a reviewed alias: the three *Book of
the Damned* volumes, *GameMastery Condition Cards*, both Osirion titles,
*Pathfinder #91* (notice says "Hill", source says "Hills"), *Pathfinder #139*
("Road" versus "Roads"), *Pathfinder Society S03-10* (a title typo), and *Rivers
into Darkness* (notice says "River Into Darkness").

The 56 unmatched publications have in-scope records but no matching Section 15
entry in the pin:

| Publication | Records |
|---|---|
| Academy of Secrets | 1 |
| Advanced Race Guide | 659 |
| Armor Master's Handbook | 128 |
| Bestiary 5 | 447 |
| Bestiary 6 | 311 |
| Carnival of Tears | 3 |
| Carrion Crown Player's Guide | 6 |
| Conquest of Bloodsworn Vale | 4 |
| Crown of the Kobold King | 3 |
| Cult of the Ebon Destroyers | 3 |
| Curse of the Crimson Throne Player's Guide | 24 |
| Dungeons of Golarion | 10 |
| Faiths of Balance | 51 |
| Faiths of Purity | 42 |
| Goblins of Golarion | 61 |
| Guide to Darkmoon Vale | 1 |
| Halflings of Golarion | 49 |
| Heart of the Jungle | 18 |
| Heaven Unleashed | 10 |
| Heroes of the Streets | 94 |
| Hollow's Last Hope | 1 |
| Horror Realms | 46 |
| Humans of Golarion | 28 |
| Inner Sea Magic | 75 |
| Inner Sea Taverns | 16 |
| Inner Sea World Guide | 118 |
| Inner Sea world Guide | 1 |
| Kingmaker Player's Guide | 7 |
| Magic of Thassilon | 2 |
| Occult Mysteries | 54 |
| Pathfinder #119: Prisoners of the Blight | 23 |
| Pathfinder #3: The Hook Mountain Massacre | 4 |
| Pathfinder #63: The Asylum Stone | 11 |
| Pathfinder #64: Beyond the Doomsday Door | 15 |
| Pathfinder Campaign Setting | 84 |
| Pathfinder Goblins #1 | 5 |
| Pathfinder Goblins #5 | 1 |
| Pathfinder Hollow Mountain #1 | 1 |
| Pathfinder Hollow Mountain #4 | 1 |
| Pathfinder Origins #6 Ezren | 1 |
| Pathfinder Runescars #1 | 1 |
| Pathfinder Runescars #4 | 1 |
| Pathfinder Society Field Guide | 67 |
| Pathfinder Worldscape #1 | 3 |
| Pathfinder Worldscape #2 | 5 |
| Pathfinder Worldscape #3 | 1 |
| Pathfinder Worldscape #4 | 1 |
| Realm of the Fellnight Queen | 2 |
| Revenge of the Kobold King | 3 |
| Rival Guide | 37 |
| Serpent's Skull Player's Guide | 10 |
| The Godsmouth Heresy | 1 |
| Tomb of the Iron Medusa | 3 |
| Tome of Horrors Complete (PF) | 3 |
| Undead Revisited | 10 |
| We Be Goblins! | 5 |

`Pathfinder Campaign Setting` is a generic registry label, not a book title, and
has no Section 15 entry of its own. The comics and *Magic of Thassilon* are also
outside the current policy's explicit exclusions, even though they are not
eligible content. The report lists six additional publication labels excluded
by `out-of-scope.json` before matching; they are retained under
`excluded_publications` rather than dropped.

To resolve an unmatched publication, use one of these reviewed routes:

- Add its complete Section 15 line to the upstream notice, advance the pin, and
  re-run the audit. Do not edit the cached notice or bypass the hash pin.
- Record a different permission route with the project owner and extend the
  legal page model before claiming the publication is covered.
- Move the publication to `out-of-scope.json` with a reason, so its records are
  excluded from the cutover instead of shipped without attribution.
- If the notice already exists under a different title, add a reviewed mapping
  to `notice-aliases.json`. Never add a fuzzy or title-similarity alias.

`pf_legal::gate` blocks a production build while any in-scope publication lacks
a notice. `reports/migration-legal.json` also lists the 136 notices that match
no in-scope publication. That list is informational: it includes the OGL, the
System Reference Document, *Pf Data 1e*, third-party material, and Paizo books
not present in the registry.

## Publication gate

The experimental inventory and conversion reports stay separate from the site
library. Before cutover, resolve source notices, review the PI screen's coverage,
resolve cross-source identities, and every critical or high conversion finding. Excluded
records must remain visible in coverage. Do not replace a working offline library
with a partly converted or unapproved corpus.
