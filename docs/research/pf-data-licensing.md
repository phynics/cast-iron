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

The PSRD carry-over library imports exactly one reviewed record from this pin:
`pf-data/rule/magic`, the Core Rulebook page 206 opening passage, which is
ordinary OGC game mechanics under the OGL. Its `approved.json` row records
`rights: "ogl"`, so the compiler emits the canonical `ogl-1.0a` license
reference. The canonical default instead converts the screened Pf Data corpus
and admits a publication only through the reviewed
`sources/canonical/publications.toml` registry. Both builds append the pin's
complete Section 15 list to the legal page and record the dataset. The PSRD
build still excludes every other record from this pin; the canonical build's
scope and legal screen are described below.

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

**Superseded on 2026-11-05:** Paizo Product Identity is now kept under the Community Use Policy, and the section rules below were removed. The screen only strips outbound links. See [decisions](../decisions.md#paizo-product-identity-kept-under-the-community-use-policy-2026-11-05).

The product owner chose fidelity under the Community Use Policy on 2026-11-04.
Proper names inside game mechanics stay as descriptive references. Every Pf Data
entry carries `ogl-1.0a` and `legal#community-use`. The reviewed section rules in
`sources/pathfinder-data-1-e/pi-screen.json` omit deity `Description` lore, unique
creature `Description` biographies, and named NPC stories, and remove outbound
links. Each omission is a coverage row. See
[decisions](../decisions.md#product-identity-screen-for-the-pf-data-corpus-2026-11-04).

## Section 15 notice coverage

`cargo xtask data-legal` (also run by `cargo xtask data-report`) checks every
publication referenced by an in-scope record against four pinned notice
sources, in order:

1. The Pf Data pin's `OPENGAMECONTENT.md` Section 15 list.
2. PSRD-Data's Section 15 list in `sources/psrd-data/legal-notices.json`.
3. d20pfsrd.com's Section 15 statements export in
   `sources/pathfinder-data-1-e/section15/d20pfsrd-section15.csv`.
4. Section 15 lines supplied by the project owner from the owner's copies of the
   books, in `sources/pathfinder-data-1-e/section15/owner-copies.json`.

The d20pfsrd export was supplied verbatim by the project owner on 2026-11-05.
Its provenance file records the source, SHA-256, and 1,244 rows; the compiler
fails the report if the CSV hash or row count changes. d20pfsrd.com distributes
its Pathfinder First Edition Open Game Content under the OGL 1.0a. The export is
evidence for notice text only; it does not authorize new content.

The owner-copy list was supplied on 2026-11-05 from the owner's copies of the
books. Its provenance file records the source, supply date, SHA-256, and three
rows; the compiler fails the report if the list hash or row count changes. Every
notice is verbatim Section 15 text from the named book. The provenance cites the
book only.

The 2026-11-05 review of the pinned snapshot has 34,497 in-scope records
referencing 453 publications. 434 publications match a notice in at least one
source; 8 are reviewed as notice-pending; 11 have no notice and are comics. The
report records the first source that matched (`exact`, `normalized-prefix`, or
`alias` for Pf Data; `psrd`; `d20pfsrd`; `owner-copy`) and every source that
contributed (`notice_sources`). Sources add rather than replace: an already
matched publication keeps any additional distinct notices from a later list.

Within each source, matching uses:

1. Exact normalized title (`PRPG` is not expanded here).
2. A reviewed alias from `sources/pathfinder-data-1-e/notice-aliases.json`,
   whose target must exist in at least one pinned notice source.
3. A deterministic normalizer that removes the known line prefixes
   (`Pathfinder Roleplaying Game`, `Pathfinder RPG`, `Pathfinder Player
   Companion`, `Pathfinder Campaign Setting`, `Pathfinder Module`,
   `Pathfinder Companion`, `Pathfinder Chronicles`, `Pathfinder Adventure
   Path`, `Pathfinder Society`, `GameMastery Module`, `Pathfinder`, `PRPG`),
   volume/scenario words, module codes such as `J1`, a leading article after an
   adventure number, and treats `and` and `&` as the same connector. For the
   d20pfsrd export, the title is the text before its first `©` or `Copyright`
   marker, with trailing sentence punctuation removed. Every distinct notice
   under one title key applies, so the Core Rulebook keeps both its 2009 and
   2010 printing notices. The owner-copy entries are keyed by the publication
   they name and keep their notice text verbatim.

The four sources contribute overlapping coverage. Pf Data matches 400
publications; PSRD matches 17; the d20pfsrd export matches 279; the owner-copy
list matches 3. The export also keeps both distinct *Rival Guide* author lists
and both 2008 *Pathfinder Chronicles Campaign Setting* statements instead of
choosing one.

The following table records the decision for all 56 publications that were
unmatched before the 2026-11-05 review. Thirty-one now match a pinned source for
the first time, three rows were already reviewed aliases, and three are owner
copies. "Records" is the number of in-scope record references to that
publication.

| Publication | Records | Method | Sources | Matched notice title(s) or reason |
|---|---|---|---|---|
| Academy of Secrets | 1 | d20pfsrd | d20pfsrd | `Pathfinder Module: Academy of Secrets` |
| Advanced Race Guide | 659 | psrd | psrd, d20pfsrd | `Pathfinder Roleplaying Game Advanced Race Guide` |
| Armor Master's Handbook | 128 | d20pfsrd | d20pfsrd | `Pathfinder Player Companion: Armor Master’s Handbook` |
| Bestiary 5 | 447 | d20pfsrd | d20pfsrd | `Pathfinder Roleplaying Game Bestiary 5` |
| Bestiary 6 | 311 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Carnival of Tears | 3 | d20pfsrd | d20pfsrd | `GameMastery Module E1: Carnival of Tears` |
| Carrion Crown Player's Guide | 6 | d20pfsrd | d20pfsrd | `Carrion Crown Player’s Guide` |
| Conquest of Bloodsworn Vale | 4 | d20pfsrd | d20pfsrd | `GameMastery Module W1: Conquest Of Bloodsworn Vale` |
| Crown of the Kobold King | 3 | d20pfsrd | d20pfsrd | `GameMastery Module D1: Crown of the Kobold King` |
| Cult of the Ebon Destroyers | 3 | d20pfsrd | d20pfsrd | `Pathfinder Module: Cult of the Ebon Destroyers` |
| Curse of the Crimson Throne Player's Guide | 24 | d20pfsrd | d20pfsrd | `Curse of the Crimson Throne Player’s Guide` |
| Dungeons of Golarion | 10 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Dungeons of Golarion` |
| Faiths of Balance | 51 | d20pfsrd | d20pfsrd | `Pathfinder Player Companion: Faiths of Balance` |
| Faiths of Purity | 42 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Goblins of Golarion | 61 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Guide to Darkmoon Vale | 1 | owner-copy | owner-copy | `Guide to Darkmoon Vale` |
| Halflings of Golarion | 49 | d20pfsrd | d20pfsrd | `Pathfinder Player Companion: Halflings of Golarion` |
| Heart of the Jungle | 18 | d20pfsrd | d20pfsrd | `Pathfinder Chronicles: Heart of the Jungle` |
| Heaven Unleashed | 10 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Heaven Unleashed` |
| Heroes of the Streets | 94 | alias | pf-data | `Pathfinder Player Companion: Heroes of the Street` |
| Hollow's Last Hope | 1 | d20pfsrd | d20pfsrd | `GameMastery Module D0: Hollow’s Last Hope` |
| Horror Realms | 46 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Humans of Golarion | 28 | d20pfsrd | d20pfsrd | `Pathfinder Player Companion: Humans of Golarion` |
| Inner Sea Magic | 75 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Inner Sea Magic` |
| Inner Sea Taverns | 16 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Inner Sea World Guide | 118 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: The Inner Sea World Guide` |
| Inner Sea world Guide | 1 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: The Inner Sea World Guide` |
| Kingmaker Player's Guide | 7 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Magic of Thassilon | 2 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| Occult Mysteries | 54 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Occult Mysteries` |
| Pathfinder #119: Prisoners of the Blight | 23 | alias | pf-data | `Pathfinder Adventure Path #119: Prisoner of the Blight` |
| Pathfinder #3: The Hook Mountain Massacre | 4 | d20pfsrd | d20pfsrd | `Pathfinder 3: The Hook Mountain Massacre` |
| Pathfinder #63: The Asylum Stone | 11 | d20pfsrd | d20pfsrd | `Pathfinder Adventure Path #63: The Asylum Stone` |
| Pathfinder #64: Beyond the Doomsday Door | 15 | owner-copy | owner-copy | `Pathfinder #64: Beyond the Doomsday Door` |
| Pathfinder Campaign Setting | 84 | d20pfsrd | d20pfsrd | `Pathfinder Chronicles Campaign Setting` |
| Pathfinder Goblins #1 | 5 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Goblins #5 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Hollow Mountain #1 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Hollow Mountain #4 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Origins #6 Ezren | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Runescars #1 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Runescars #4 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Society Field Guide | 67 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Pathfinder Society Field Guide` |
| Pathfinder Worldscape #1 | 3 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Worldscape #2 | 5 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Worldscape #3 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Pathfinder Worldscape #4 | 1 | absent | - | Dynamite comic issue; comics are permanently out of scope. |
| Realm of the Fellnight Queen | 2 | alias | pf-data, d20pfsrd | `Pathfinder Module: Realm of the Fellknight Queen` |
| Revenge of the Kobold King | 3 | d20pfsrd | d20pfsrd | `Pathfinder Module D1.5: Revenge of the Kobold King` |
| Rival Guide | 37 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Rival Guide` (two distinct author lists) |
| Serpent's Skull Player's Guide | 10 | pending | - | No notice in any pinned list; reviewed as notice-pending. |
| The Godsmouth Heresy | 1 | d20pfsrd | d20pfsrd | `Pathfinder Module: The Godsmouth Heresy` |
| Tomb of the Iron Medusa | 3 | d20pfsrd | d20pfsrd | `Pathfinder Module: Tomb of the Iron Medusa` |
| Tome of Horrors Complete (PF) | 3 | d20pfsrd | d20pfsrd | `Tome of Horrors Complete` |
| Undead Revisited | 10 | d20pfsrd | d20pfsrd | `Pathfinder Campaign Setting: Undead Revisited` |
| We Be Goblins! | 5 | owner-copy | owner-copy | `We Be Goblins!` |

*Pathfinder Campaign Setting* is the 2008 *Pathfinder Chronicles Campaign
Setting* (Paizo product `v5748btpy84eo`), not a generic registry label. The
d20pfsrd export carries two statements for that book, one with a comma after
the title and one without; both are kept. The 84 in-scope references therefore
have a notice of their own. Four records name only this label (*Blacklight*,
*The Tainted Plague*, *Hardening*, and *Profits of Kalistrade*), and the alias
covers them directly. The other 80 also name at least one other publication,
which may add its own notice.

The 11 comic labels (*Pathfinder Goblins*, *Pathfinder Hollow Mountain*,
*Pathfinder Origins*, *Pathfinder Runescars*, *Pathfinder Worldscape*) are
Dynamite comic issues and are absent from all four lists. Comics are
permanently outside the current policy, but these labels are not in
`out-of-scope.json`; the canonical legal screen excludes their records by the
missing-notice rule and reports them as legal exclusions.

Eight publications have no Section 15 notice in any source: *Bestiary 6*,
*Faiths of Purity*, *Goblins of Golarion*, *Horror Realms*, *Inner Sea Taverns*,
*Kingmaker Player's Guide*, *Magic of Thassilon*, and *Serpent's Skull Player's
Guide*. The project owner decided on 2026-11-05 to ship their records as assumed
Open Game Content and keep notice coverage open in issue #34. They are listed in
`sources/pathfinder-data-1-e/section15/notice-pending.json`. The canonical
library keeps their records with the usual OGL and Community Use references, the
legal page lists them under **Section 15 notice pending**, and the coverage
report carries them under `notice-pending`. A publication may not be both
matched and notice-pending; the build fails when the lists overlap.

The d20pfsrd export has 17 rows with no separable title, such as misspelled
copyright markers (`Copynght`, `Copuright`) and a blog index. They match no
publication and appear under `unparsed_notices` in the report rather than
disappearing.

To resolve an unmatched publication, use one of these reviewed routes:

- Add its complete Section 15 line to the upstream notice, advance the pin, and
  re-run the audit. Do not edit the cached notice or bypass the hash pin.
- Add a complete notice to the pinned d20pfsrd export with the same provenance
  review, and update its SHA-256 and row count. Never paraphrase or repair a
  notice by hand.
- Add the complete Section 15 line from the owner's copy of the book to
  `owner-copies.json`, then update its provenance SHA-256 and row count. Cite
  the book only in the provenance note.
- Record a different permission route with the project owner and extend the
  legal page model before claiming the publication is covered.
- Move the publication to `out-of-scope.json` with a reason, so its records are
  excluded from the cutover instead of shipped without attribution.
- If the notice already exists under a different title, add a reviewed mapping
  to `notice-aliases.json`. Never add a fuzzy or title-similarity alias.

`pf_legal::gate` blocks a production build while any in-scope publication has
neither a notice nor a reviewed notice-pending decision.
`reports/migration-legal.json` also lists the 133 Pf Data notices that match no
in-scope publication. That list is informational: it includes the OGL, the
System Reference Document, *Pf Data 1e*, third-party material, and Paizo books
not present in the registry. The canonical legal screen now rejects only records
whose publications are all comics; every rejected record remains in the coverage
report.

## Publication gate

The experimental inventory and conversion reports stay separate from the site
library. Before cutover, resolve source notices, review the PI screen's coverage,
resolve cross-source identities, and every critical or high conversion finding. Excluded
records must remain visible in coverage. Do not replace a working offline library
with a partly converted or unapproved corpus.
