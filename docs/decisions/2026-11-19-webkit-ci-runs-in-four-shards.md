---
title: "WebKit CI runs in four shards with a 20 minute limit"
date: 2026-11-19
status: current
superseded_by: []
issues: []
prs: []
area: process
---

# WebKit CI runs in four shards with a 20 minute limit, 2026-11-19

The informational WebKit jobs ran three shards under `timeout-minutes: 15`. WebKit shard 1/3 reached that limit on runs 44539453564 (main 3919b89) and 44555076541 (PR #508). GitHub cancelled it, which made the whole run `cancelled`, and `scripts/promote-release.py` refuses a run that is not `success`. `continue-on-error` does not cover a cancellation.

- **Cause.** Two things, both visible in the shard logs. (1) The suite grew: WebKit test time went from 1800 s over 358 tests (run 44508434885, main fc00215) to 2300 s over 372 tests (run 44555076541). `books-share.spec.ts` alone went from 56 s to 201 s, and Playwright puts whole files in a shard by test count, so shard 1 took the new files and went from 8.5 to about 14.5 minutes. (2) `books-share.spec.ts` called `page.goto("#/books/manage")` while the page was already on that URL. Chromium treats that as a no-op. WebKit reloads the document, which costs a whole start-up and drops the in-memory Undo, so "An Undo that was clicked while another change was saving does nothing" waited out a 15 s `expect` timeout and failed in WebKit only. The spec now goes through `visit`, which skips the navigation when the hash already matches. The console guard (`fixtures.ts`) adds no wait: it only listens to events.
- **Four shards, 20 minutes.** `--shard=i/4` brings the per-shard test time to about 10 minutes. The limit is a guard against a hang, so it now equals the Chromium jobs' 20 minutes. The shard count is the one the earlier decision (`2026-11-05-ci-browser-shards-and-early-cache-saves`) said to raise only when it lowers the longest shard; it does here, because no single WebKit file takes more than about 225 s.
- **No retries.** A WebKit failure stays a failure; the shards stay informational.
