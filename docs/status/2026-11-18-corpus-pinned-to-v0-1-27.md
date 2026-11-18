---
title: "Corpus pinned to v0.1.27"
date: 2026-11-18
kind: pin
---

# Corpus pinned to v0.1.27, 2026-11-18

`sources/corpus.toml` now pins `phynics/cast-iron-data` tag `v0.1.27`. The annotated tag object
is `61effa1fb3cdb757830fbeb97acfbf34c3c8a158`; it peels to commit
`65f396735d6c7a685cc89782cc5b113c854bf76f`. The tree hash is
`8d779094786c75acd978d231754d9f965f55b64f122c9493338cef8ec2848499`
(`cargo xtask corpus-tree-hash` on the extracted codeload archive, which `cargo xtask fetch-sources`
verified). The pin ships the corpus changes #100, #101, #102, #104 and #105:

- **Deadeye Devotee** ([#483](https://github.com/phynics/cast-iron/issues/483)).
  `ref/pclass/deadeye-devotee-arcane-archer` cites only Hodgepodge from the Grand Lodge! and has
  the archetype relation to Arcane Archer. It is no longer a Core Rulebook member.
- **Paizo Blog sources under the Community Use Policy**
  ([#36](https://github.com/phynics/cast-iron/issues/36), decision
  [2026-11-18](../decisions/2026-11-18-paizo-blog-source-bases-under-cup.md)). Eight records that
  an earlier pin excluded now ship: Thought Thief (Arcane Trickster); the spells Breeze, Drench,
  Jolt, Penumbra, Root and Scoop (Ultimate Cantrips); and the race Primitive Human (The Gauntlet
  Will Be Ours!). The corpus adds three exclusion rows: the Sable Company Marine membership and the
  blog-only Death and Repose domain adjustments.

- **Counts.** 31,808 entries (was 31,800), 443 publications and 443 books (were 439 and 439).
  Reachability: 31,808 reachable, 0 unreachable. Links: 147,789 resolved, 0 out of selection, 0
  broken. The library schema stays 6 and the search index format stays 2. By kind: archetype
  1,298 to 1,299, race 75 to 76, spell 3,082 to 3,088. The compiler writes a chapter tree for 439
  books; the four new web supplements (below) list their entries without one.
- **New publications.** Hodgepodge from the Grand Lodge! (share slot 439), Golarion Day: Other
  Gods and New Subdomains (440), Ultimate Cantrips (441) and The Gauntlet Will Be Ours! (442). The
  slots were already reserved in `sources/published-books.txt`; `cargo xtask published-books`
  added 0 lines, and `cargo xtask check-book-slots --base origin/main` reported all 443 slots
  kept.
- **Membership.** The book memberships in the coverage report rise from 38,752 to 38,763 (+11):
  Hodgepodge 2, Golarion Day 2 (Scalykind and Void), Ultimate Cantrips 6, The Gauntlet Will Be
  Ours! 2 (Primitive Human and its Rare Races membership), and Core Rulebook 2,984 to 2,983 (-1,
  Deadeye Devotee). No other book changes. No archetype is a Core Rulebook member.
- **Legal.** `legal.json` carries a `publication_licenses` row for each of the four new
  publications and for More Magic of Thassilon. Ultimate Cantrips, Golarion Day and More Magic of
  Thassilon are Community Use only and are not listed in `notice_pending`; Hodgepodge and The
  Gauntlet Will Be Ours! name both bases. Primitive Human has its Section 15 notice. `notice_pending`
  lists Hodgepodge, Kingmaker Player's Guide and Serpent's Skull Player's Guide.
- **AoN coverage.** `cargo xtask aon-coverage` regenerated `docs/research/aon-path-coverage.{md,tsv}`:
  12,467 of 31,808 entries with an AoN path, 426 of 443 publications with a source page.
- **Test.** `books.spec.ts` gains "The Core Rulebook has no archetype members, in the corpus or on
  its book page (#483)". It expects the literal 0 and an empty list of members, not a count read
  from the corpus.
- **Publication screen.** `scripts/check-publication.py` required `ogl-1.0a` on every entry, so it
  failed on the nine entries that the decision makes Community Use only (Deadeye Devotee, Thought
  Thief, The Gluttonous Tome and the six Ultimate Cantrips spells). It now accepts an entry without
  `ogl-1.0a` only when the entry carries `legal#community-use` and every source it cites has a
  `publication_licenses` row that names Community Use. Any other entry without `ogl-1.0a` still
  fails.
- **Local verification** (CI not run, nothing deployed), on `origin/main` at `02490f40a`
  plus this pin. Content version `v6bd1da0aee54f66d` (was `vb4ec72f397a03202`; compiling the
  v0.1.26 corpus again reproduced that version). `cargo xtask content` passed the published-id
  guard and the book-slot guard; `cargo xtask check` (with `check-book-slots` and
  `aon-coverage --check`) passed; `build-site --public-url /cast-iron/` passed (shell
  `05add9e832fb869b`, `generated/` and `dist/` manifests identical). `size-budget`: library
  223,213,634 of 234,300,000 bytes, largest file 68,701,524 of 72,300,000, WASM 3,348,303 of
  3,460,000 (no raise). `scripts/check-publication.py` on `dist/content`: 31,808 entries, 0 issues.
  Spec on `--project=chromium`, one worker: `books.spec.ts`, 32 passed. The full suite runs in CI.
