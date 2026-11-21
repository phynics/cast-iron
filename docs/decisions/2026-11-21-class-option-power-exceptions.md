---
title: "Class-option powers: the exceptions file that corpus-check honours"
date: 2026-11-21
status: current
superseded_by: []
issues: [537, 557]
prs: []
area: corpus
---

# Class-option powers: the exceptions file that corpus-check honours, 2026-11-21

Refs [#557](https://github.com/phynics/cast-iron/issues/557) and
[#537](https://github.com/phynics/cast-iron/issues/537). Hand-off step 3 of
[2026-11-20-class-option-power-layout](2026-11-20-class-option-power-layout.md) makes the class-option
power check an error at the pin of the rewritten corpus, "except the ones still queued in #88's resolution
file". The corpus repository has no file that the check can read for that: its queue is a tool file
(`scripts/class-option-power-queue.tsv`) that lists every power that still needs a decision and holds no
reason, book or page. This entry adds the smallest file that does.

## Decision

- **`power-exceptions.toml` in the corpus root**, optional, read only by `corpus-check` (`crates/corpus`,
  `check_powers.rs`). One `[[exception]]` row per power: `id`, `name`, `occurrence`, `reason`, `print`, and
  an optional `span_sha256`. The format is in `docs/corpus-format.md`, "Power exceptions".
- **What it means.** A power that print cannot settle stays in a non-canonical form on purpose. It needs a
  reason and the book title and page that was checked (or why no page exists). It is never a silent drop.
- **What the check does.** A failing power that a row names is not reported and is counted in the last line.
  A row that matches no class-option power, or a power that now passes, is a warning (`remove the
  exception`), so the file cannot go stale unnoticed. A malformed file is an error: it is data that the check
  itself reads.
- **The hash.** The corpus repository's script keeps `span_sha256` honest (the exception applies only to
  the exact original span, as a resolution does). The check does not read it: the span text rule is the
  script's, and the check would need a second implementation of it.
- **Not a format change.** No entry, no field and no reader changes. The compiler and the library ignore the
  file. `corpus.toml` keeps its `format`, and `SCHEMA_VERSION` stays.
- **Still a warning.** This entry changes no severity. The pin pull request that takes the resolved
  corpus turns the check into an error, and that is the change that decides whether any exception row
  remains.

## Why not the queue file

The queue lists a power that waits for a hand decision, and its rows leave it as the powers are resolved.
An exception is the opposite: a power that was looked at and stays. Mixing the two would make the queue
file's empty state ("nothing left to decide") mean "nothing is excepted", and the app would have to parse a
tool file in `scripts/`. A separate file in the corpus root follows `aliases.toml` and `option-groups.toml`,
the other optional corpus files.

## Open case

The seven `Reckoning` blocks of the Hellknight orders are stat-block lines in print, not powers, and the
paragraph after each describes the order. They are excepted until the owner decides whether `Reckoning`
joins `NON_POWER_LABELS` (decision 2026-11-21-class-option-power-owner-cases, case 5); that is a one-line
change with a line in a decision, and it removes seven rows.
