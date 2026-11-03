# Licenses

This repository contains code, generated content, and upstream notices under different terms. Each part is covered by the license listed below.

| Part | License | Where |
|---|---|---|
| Code written for Cast Iron | MIT | [MIT.txt](MIT.txt), and `license = "MIT"` in `Cargo.toml` |
| Open Game Content in the generated rules library | OGL 1.0a | [OGL-1.0a.txt](OGL-1.0a.txt) |
| Section 15 copyright notices | Shown in the app, from the upstream source | `sources/psrd-data/legal-notices.json` |
| Paizo material used under Paizo's policy | Paizo Community Use Policy | Notice in the app and in the README |

## MIT (code)

The Rust crates, the web shell (`web/`), the build tooling (`crates/xtask`), and the tests are under the MIT license. See [MIT.txt](MIT.txt).

## OGL 1.0a (Open Game Content)

The rules text in the generated library is derived from Paizo's Pathfinder Reference Document. The Open Game Content in it is licensed under the Open Game License, version 1.0a. [OGL-1.0a.txt](OGL-1.0a.txt) is the full license text. OGL 1.0a Section 10 requires a copy of the license with each distribution, so the app also shows this text on its Legal and attribution page.

OGL 1.0a covers only Open Game Content. Product Identity is excluded from the grant, and the app shows the Product Identity declaration. The project does not ship artwork or other Product Identity (`AGENTS.md`, rule 10). See `docs/research/licensing-audit.md` for the open questions.

## Section 15 notices

OGL 1.0a Section 15 requires the copyright notices of the sources that contributed Open Game Content. These notices are kept in `sources/psrd-data/legal-notices.json`. The app shows them on its Legal and attribution page.

## PSRD-Data (dataset)

The generated library is compiled from [PSRD-Data](https://github.com/devonjones/PSRD-Data), pinned in `sources/psrd-data/source.toml`. PSRD-Data has no license of its own: its repository has no license file and no license field. The licensing position of the data is recorded in `docs/research/licensing-audit.md`.

## Paizo Community Use Policy

Paizo material is used under the Paizo Community Use Policy. The policy requires the notice that the README and the app's Legal and attribution page show. Cast Iron is unofficial and is not endorsed by Paizo.
