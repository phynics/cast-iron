# Licenses

This repository combines code, game content, and upstream notices under
different terms. No single license covers everything. Each part is covered by
the license listed below.

| Part | License | Where |
|---|---|---|
| Code written for Cast Iron | MIT | Root [LICENSE](../LICENSE), copy in [MIT.txt](MIT.txt), and `license = "MIT"` in `Cargo.toml` |
| Open Game Content in the generated rules library (compiled from the corpus below) | OGL 1.0a | [OGL-1.0a.txt](OGL-1.0a.txt), plus the app's Legal page |
| Section 15 copyright notices | OGL 1.0a Section 15 | `legal.toml` in the corpus, shown in the app |
| Outlined Rozha One letterforms in the wordmark (`web/icons/wordmark.svg`) | SIL OFL 1.1 | [OFL-1.1-Rozha-One.txt](OFL-1.1-Rozha-One.txt) |
| Paizo material used under Paizo's policy | Paizo Community Use Policy | Notice in the app and in the README; not a file in this repo |

## MIT (code)

The Rust crates, the web shell (`web/`), the build tooling (`crates/xtask`),
and the tests are under the MIT license. The root `LICENSE` is canonical and
carries a scope note stating it covers code only; [MIT.txt](MIT.txt) is the
bare license text without that note.

## SIL OFL 1.1 (wordmark letterforms)

The "Cast Iron" wordmark in `web/icons/wordmark.svg` is outlined from the font
Rozha One (Copyright (c) 2014, Indian Type Foundry), which is licensed under
the SIL Open Font License, version 1.1. [OFL-1.1-Rozha-One.txt](OFL-1.1-Rozha-One.txt)
is the licence text with that notice. The copyright line declares no Reserved
Font Name, so a modified outline may carry the name of the font's source. The app
ships the outlines as artwork, not the font: there is no font file and no remote
font. The wordmark's shapes are modified (tracking, and a 4-unit round stroke on
the path that thickens the hairlines).

## OGL 1.0a (Open Game Content)

The rules text in the generated library is Open Game Content licensed under
the Open Game License, version 1.0a. [OGL-1.0a.txt](OGL-1.0a.txt) is the full
license text. OGL 1.0a Section 10 requires a copy of the license with each
distribution, so the app also shows this text on its Legal and attribution page.

OGL 1.0a covers only Open Game Content. Product Identity is excluded from the
grant, and the app shows the Product Identity declaration. The project does not
ship artwork or other Product Identity (`AGENTS.md`, rule 10). See
`docs/research/licensing-audit.md` for the open questions.

## Section 15 notices

OGL 1.0a Section 6 and Section 15 require reproducing the copyright notices of
the sources that contributed Open Game Content. The corpus carries them in its
`legal.toml`, and the compiler copies them into the library's legal data, which
the app shows on its Legal and attribution page. The corpus repository also
holds the complete Section 15 list in its own `LICENSES/OGL-1.0a.md`, and names
the publications whose notice is still pending.

## The corpus (rules content)

The rules text is compiled from the plain-text corpus in
[phynics/cast-iron-data](https://github.com/phynics/cast-iron-data), pinned by
commit and tree hash in `sources/corpus.toml`. `cargo xtask fetch-sources`
downloads it and verifies the hash; nothing else supplies rules content. The
corpus repository carries its own licensing files: `LICENSE.md` (the table of
parts), `LICENSES/MIT.txt` for its format and tooling files,
`LICENSES/OGL-1.0a.md` for the Open Game Content, and
`LICENSES/community-use.md` for Paizo's Community Use Policy.

The corpus was written once from two public datasets, recorded in its
`legal.toml` and in each entry's frozen `provenance`:

- [PSRD-Data](https://github.com/devonjones/PSRD-Data). It has no license of
  its own; its content derives from Paizo's Pathfinder Reference Document, used
  under OGL 1.0a plus Paizo's Community Use Policy for permitted descriptive
  references.
- [jasontankapps/pathfinder-data-1-e](https://github.com/jasontankapps/pathfinder-data-1-e).
  Its `json/` directory contains Open Game Content under OGL 1.0a; the
  dataset's Open Game Content notice excludes Product Identity. The non-OGC
  application content of that repository is under GPL-3.0-or-later; Cast Iron
  never copied its renderer or other application code. The status of the
  dataset's editor's notes is unresolved: its notices do not designate them
  individually and support two readings (Open Game Content of the dataset, or
  GPL application content). The corpus keep test ([owner decision of
  2026-11-20](../docs/decisions/2026-11-20-printed-footnotes-stay.md)): a note stays when Archives of Nethys or the Paizo PRD carries it,
  or when the printed book carries it; a note that only the dataset's editor
  added (a pointer, commentary, a compiled list) and a note of unclear origin
  go. A match on AoN, in the PRD or in print is evidence of provenance, not a
  license. The OGL 1.0a covers wording that is identifiable as the book's Open Game
  Content, with Section 15 attribution. For a note kept because the printed
  book carries it, the basis is the same as for the entry text around it (the
  book's Open Game Content and its Section 15 notice), for wording identifiable
  as that content. A kept paraphrase has no basis beyond that, so its exact
  wording falls under the unresolved question, like any original wording by the
  dataset's editor or by AoN. The footnote audit (merged in cast-iron #526)
  found notes that only the editor added; the pinned corpus v0.1.41 removes
  them (data issue #117, PR #123, in v0.1.31). Its `LICENSE.md` names the Pathfinder Data
  notes that stay and their basis (data PR #119) and no longer says that none of
  the dataset's application content was copied.
  `docs/research/editorial-footnotes-audit.md` records the
  evidence. Its ORC-licensed Pathfinder Second Edition material was out of
  scope and is not in the corpus.

Cast Iron no longer downloads or reads either dataset. The import code, the
pins, and the source selection are preserved in the git tag
`psrd-last-buildable`. The licensing reviews remain in
`docs/research/licensing-audit.md` and `docs/research/pf-data-licensing.md`;
they describe that retired import.

## Paizo Community Use Policy

Paizo material is used under the Paizo Community Use Policy. The policy
requires the notice that the README and the app's Legal and attribution page
show. Cast Iron is unofficial and is not endorsed by Paizo. The policy is a
permission with conditions (free access, unofficial status, contact
information), not a file shipped in this repository. The re-verified review is
in `docs/research/licensing-audit.md`.
