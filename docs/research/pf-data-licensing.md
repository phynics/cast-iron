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
ordinary OGC game mechanics under the OGL. The compiler appends the pin's complete
Section 15 list to the legal page and records the dataset. Every other record
remains excluded from the library. This does not expand the rights review to the
rest of the corpus.

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

- `json/sources.json` registers `monster_core`, a PF2 book.
- Seven monster-family records are tagged `Monster Core`:
  `json/monster_families.json` (`alghollthu`, `dark_folk`, `dinosaur`, `drake`)
  and `json/monster_families2.json` (`genie`, `naga`, `troll`).
- `json/main.json#ogc` is a legal notice containing both OGL and ORC text.
- The dataset carries `ORCLICENSE.md` and `GNULICENSE.md` separately from the
  OGL notice.

`sources/pathfinder-data-1-e/out-of-scope.json` excludes the publication and the
legal notice before any selection. The inventory still reports them, and the
migration report lists them as out-of-scope input. No Monster Core record or ORC
notice enters the PF1e library or the OGL legal page. The source pin hashes the
ORC and GNU license files so a changed notice cannot pass unnoticed.

## Publication gate

The experimental inventory and conversion reports stay separate from the site
library. Before cutover, resolve source notices, record-level PI exclusions,
cross-source identities, and every critical or high conversion finding. Excluded
records must remain visible in coverage. Do not replace a working offline library
with a partly converted or unapproved corpus.
