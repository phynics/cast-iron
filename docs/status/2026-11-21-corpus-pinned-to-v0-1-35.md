---
title: "Corpus pinned to v0.1.35"
date: 2026-11-21
kind: pin
---

# Corpus pinned to v0.1.35, 2026-11-21

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.35`. The annotated tag object
is `61adc3385f8ac9386c0564616d4912749bbd0eaa`; it peels to commit
`c8aa0bc33b88d1df804d4d135fbc43b27576b6ea`, the head of `main` when the tag was made. The tree
hash is `69288c35ada9f343e6b9a864579af6bdd3debb9fda4f1f26dc7fbb318fcec827`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive). The pin ships three groups of
corpus changes on top of v0.1.34 (145 entry files change; no `id` changes and no file moves):

- **Class-option powers resolved from print** (data PRs #152 to #155,
  [#557](https://github.com/phynics/cast-iron/issues/557)): 115 resolutions, 111 of them clear a
  corpus-check warning (4 passed the check already and are now checked against print). Where print
  differs from the corpus, print wins; each PR body lists the changes.
- **The 19 exceptions and three more resolved powers** (data #156). `power-exceptions.toml` holds 19
  rows: 4 druid plane domains, 8 Adventure Path powers and 7 Hellknight `Reckoning` blocks (kept by
  owner decision; decision `2026-11-21-class-option-power-exceptions`). Combat Swipe (Unchained Rogue)
  lost "This talent does not exist."; the poison rows of Poison Touch and Toxic Blood are folded into
  the prose as printed, and the three entries gain `[[sources]]` page rows. The data CI now runs
  `scripts/rewrite-class-option-powers.py --check` (step "Class-option power queue, resolutions and
  exceptions" of `.github/workflows/corpus.yml` at `c8aa0bc`). The decision requires that step for
  this pin, because the app does not compute `span_sha256`.
- **Composite trades settled** (data #157, phynics/cast-iron-data#134): 39 composite trades are
  split into one trade per feature or settled as one feature. Trades 5,905 before, 5,943 now. Ten
  Replaces/Alters lines now read with commas instead of ` and ` by owner decision.

## The power check is an error from this pin

`crates/corpus/src/check_powers.rs` reports a class-option power that fails validation and has no
exception row as an **error**, and so an exception row that matches no power (unused) or whose power
passes (obsolete). The summary line stays a warning. Counts on the pinned corpus:

| | v0.1.34 | v0.1.35 |
|---|---|---|
| Class-option powers outside the canonical form (warnings) | 133 | 0 |
| Power errors | none (not an error yet) | 0 |
| Excepted powers (`power-exceptions.toml`) | no file | 19 of 3,857 |

The last line reads `19 of 3857 class-option powers have an exception in power-exceptions.toml and
no other is outside the canonical form`. Tests in `check_powers.rs`: a new invalid power fails the
check with its path, line, id, name and reason; an unused row and an obsolete row fail it; an excepted
power passes and the summary counts it; a malformed file is one error and the failing powers then
fail as well.

## Checks on this pin

- **Corpus-check result.** `ok`, 0 errors. 7 warnings: 3 mixed third-party and Paizo source warnings
  (the shadow rats), 1 summary of 2,021 book memberships in 695 entries without a `[[sources]]` row
  (#461), 1 line for Roughrider's `Armor training 4`, 1 summary of 299 of 5,943 trades in 148 entries
  that are unresolved (338 in 182 entries before), and the power summary above.
- **Counts.** 31,825 entries, 443 publications (439 books in the corpus check), unchanged. Links:
  147,298 resolved, 0 out of selection, 0 broken. The library schema stays 6 and the search index
  format stays 2.
- **Share slots.** No new publication. `cargo xtask content` passed the book-slot guard and the
  published-id guard, so `cargo xtask published-books` and `published-ids` were not run.
- **AoN coverage.** `cargo xtask check` reported the AoN report stale; `cargo xtask aon-coverage`
  regenerated `docs/research/aon-path-coverage.md`: only the four pin lines changed. The counts stay
  12,484 of 31,825 entries with an AoN path and 426 of 443 publications with a source page.
- **Specs.** No spec in `tests/e2e/tests/` names any of the 145 changed entries, the trades of the 39
  composites, or Fascinate, Memory Palace, Change Shape, Combat Swipe, Poison Touch or Toxic Blood;
  the entries of `class-option-powers.spec.ts` (Abjuration, Flame mystery, Fire domain, Evil Eye,
  Auspicious Mark, Flight, Natural Spell Combat, Divination) are unchanged. No spec changed.
- **Docs.** `README.md`, `LICENSES/README.md` and `docs/research/pf-data-licensing.md` name the
  pinned corpus v0.1.35. `docs/corpus-format.md` ("Class-option powers", "Power exceptions") says
  error. No legal claim changed. `notice_pending` is unchanged.
- **Local verification** (nothing deployed), on `origin/main` at `72f89db83` plus this pin. Content
  version `v52638e804064b300` (was `v24010454fb1d951c`). `build-site --public-url /cast-iron/` passed
  (shell `d769703e8126d398`). `size-budget`: library 224,615,037 of 234,300,000 bytes (95.9%),
  largest file 69,024,366 of 72,300,000 (95.5%), WASM 3,449,043 of 3,460,000 (99.7%); no limit
  raised. The WASM size was 3,408,711 at the v0.1.34 pin; the growth comes from the class-option power renderer on `main` since then (`crates/web/src/powers.rs`, #537), not from this pin, and 10,957 bytes of room remain.
  Spec `class-option-powers.spec.ts` (the one spec of the class-option power feature) on
  `--project=chromium`, one worker: 7 passed. The full suite runs in batch CI on `main`.
