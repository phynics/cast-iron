---
title: "Owner decisions from the architecture review"
date: 2026-11-16
status: current
superseded_by: []
issues: [394]
prs: []
area: process
---

# Owner decisions from the architecture review, 2026-11-16

These are the owner's answers to the 2026-11-15 architecture review. Each decision has a one-line reason.

- **An entry is visible while any book in its `source_ids` is active.** Placement in `books.toml` controls presentation only. When [#394](https://github.com/phynics/cast-iron/issues/394) lands, this rule supersedes "Discovery snapshots eligible IDs using book placement" in "Reference simplification implementation". Reason: an entry that several books print must not vanish when one of them is turned off.
- **Breaking format changes reach `release` together, as "format 5".** Each breaking change still bumps `SCHEMA_VERSION` or `INDEX_FORMAT_VERSION` on `main`, as AGENTS.md rule 4 requires. Only promotion to `release` waits. "Format 5" means the first release that carries those bumps. This entry does not fix the version numbers. Reason: the published app then changes format once, so users download the library once.
- **The audience until 2027-01-11 is the owner and their table.** Public polish, contributor docs, and bandwidth work have lower priority. Reason: the owner chose this audience; it sets what is worth doing first.
- **Performance is measured on an iPhone.** Desktop Chromium with throttling stands in for Android, but it is not device evidence for Android memory or speed. Reason: an iPhone is the phone available for measurement.
- **Collections and star export are dropped.** This decision supersedes FR-08 and FR-14 in `docs/spec.md`, which stays unedited. The retirements are recorded in "Planned simplification after the reference walkthrough, 2026-11-03", "Reference simplification implementation", and the star export removal in "Rule target evidence and new-source preparation". A later PR deletes the dead code. Reason: `PRODUCT.md` and the 2026-11-03 simplification entries already retired them; the remaining code is unmounted.
- **Effective entries from The Elephant in the Room are a core feature.** The book stays off by default, as "Book presets and third-party books, 2026-11-06" and "Third-party revisions replace their original in place, 2026-11-09" decide. Reason: the owner chose to keep investing in them rather than freeze them as an experiment.
- **There is no Product Identity fallback build for now.** The risk stays accepted, as in "Paizo Product Identity kept under the Community Use Policy, 2026-11-05". Reason: a second build costs real work, and the owner has already accepted the risk.
- **The compiler derives page-order books from the pages in `[[sources]]`.** Only the core books stay hand-curated in `books.toml`. Reason: a derived order cannot drift from the page citations that already exist in the corpus.
- **Outside corpus contributors are not a goal.** Corpus pipeline work is limited to two items: fix the checker version mismatch, and split `books.toml`. Reason: with no outside contributors, a contributor-friendly pipeline has no reader.
- **Each review comment names the model that wrote it.** The rule is in the review section of `CONTRIBUTING.md`. Reason: all comments come from one GitHub account, so the independence of implementer and reviewer models cannot be audited without it.
- **A ruleset on `main` requires the `Rust checks` PR check, and a merge helper script refuses a UI PR without a green full CI run on its head SHA.** Both are follow-up work. Reason: the full CI run is the only check that covers the browser suite, and nothing enforces it today.
