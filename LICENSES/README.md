# Licenses

This repository combines code, game content, and upstream notices under
different terms. No single license covers everything. Each part is covered by
the license listed below.

| Part | License | Where |
|---|---|---|
| Code written for Cast Iron | MIT | Root [LICENSE](../LICENSE), copy in [MIT.txt](MIT.txt), and `license = "MIT"` in `Cargo.toml` |
| Open Game Content in the generated rules library (both datasets below) | OGL 1.0a | [OGL-1.0a.txt](OGL-1.0a.txt), plus the app's Legal page |
| Section 15 copyright notices | OGL 1.0a Section 15 | `sources/psrd-data/legal-notices.json`, appended Pf Data notices at build time, shown in the app |
| Paizo material used under Paizo's policy | Paizo Community Use Policy | Notice in the app and in the README; not a file in this repo |

## MIT (code)

The Rust crates, the web shell (`web/`), the build tooling (`crates/xtask`),
and the tests are under the MIT license. The root `LICENSE` is canonical;
[MIT.txt](MIT.txt) is an identical copy kept beside the other license texts.

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
the sources that contributed Open Game Content. PSRD-Data notices are kept in
`sources/psrd-data/legal-notices.json`. The compiler appends the complete
Section 15 list from the pinned Pf Data source's `OPENGAMECONTENT.md` for the
imported record. The app shows the combined notices on its Legal and
attribution page.

## PSRD-Data (dataset)

The bulk of the generated library is compiled from
[PSRD-Data](https://github.com/devonjones/PSRD-Data), pinned in
`sources/psrd-data/source.toml`. PSRD-Data has no license of its own: its
repository has no license file and no license field. The licensing position of
the data is recorded in `docs/research/licensing-audit.md`. Its rules content
is used under OGL 1.0a plus Paizo's Community Use Policy for permitted
descriptive references, as reviewed there.

## Pf Data 1e (supplemental dataset)

One reviewed record is imported from
[jasontankapps/pathfinder-data-1-e](https://github.com/jasontankapps/pathfinder-data-1-e),
pinned in `sources/pathfinder-data-1-e/source.json`. That repository declares
its terms in its README and license files at the pin:

- `json/` is Open Game Content under OGL 1.0a (see its `OPENGAMECONTENT.md`,
  which carries the OGL text, Section 15 credits, and Paizo's Product Identity
  exclusion). The imported passage is ordinary OGC game mechanics used under
  this grant. See `docs/research/pf-data-licensing.md`.
- Non-OGC application content in that repository is copyright 2024 Jason Tamez
  and offered under GPL-3.0-or-later (see its `GNULICENSE.md`). Cast Iron does
  not copy its renderer or other application code, so this grant does not
  extend to Cast Iron's MIT-licensed code.
- That repository also carries `ORCLICENSE.md` for its Pathfinder Second
  Edition material. ORC-licensed PF2 records and notices are explicitly
  out of scope, excluded before selection in
  `sources/pathfinder-data-1-e/out-of-scope.json`, and never enter the library
  or the OGL legal page.

GitHub reporting no recognized repository license does not override these
in-file declarations. Upstream GPL and ORC texts are hashed in the source pin
so a changed notice fails the audit; they are not redistributed here.

## Paizo Community Use Policy

Paizo material is used under the Paizo Community Use Policy. The policy
requires the notice that the README and the app's Legal and attribution page
show. Cast Iron is unofficial and is not endorsed by Paizo. The policy is a
permission with conditions (free access, unofficial status, contact
information), not a file shipped in this repository. The re-verified review is
in `docs/research/licensing-audit.md`.
