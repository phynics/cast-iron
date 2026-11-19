---
title: "The class-sections spec cold-loads an address instead of racing the page's own rewrite with a reload"
date: 2026-11-19
status: current
superseded_by: []
issues: [121]
prs: []
area: process
---

# The class-sections spec cold-loads an address instead of racing the page's own rewrite with a reload, 2026-11-19

**Decision.** `tests/e2e/tests/class-sections.spec.ts`, "An address that names a section the page lacks drops
it and opens at the top", no longer does `goto("#/entry/…?s=…")` followed at once by `reload()`. It sets the
address with `history.replaceState` (which fires no `hashchange`, so the open page does not react) and then
reloads. The guard, the teardown window and the app are unchanged.

**Cause of this failure** (run 44492577097, `class-sections.spec.ts:303`, WebKit; reproduced locally, 2 of 6
and 3 of 6 runs of that one test before the change). A timeline of navigation requests, `framenavigated`,
`load` and page errors showed this order in every failure:

1. `goto(hash)` changes the hash of the open page. Its `framenavigated` is a same-document event.
2. `reload()` starts at once, 7 ms later.
3. The open page, one tick behind, drops the section it lacks with `replaceState` (`entry.rs`, the effect that
   calls `route::replace`). That fires a second same-document `framenavigated` on the old document, after the
   reload's navigation request and response.
4. The guard's `TeardownWindow` closes on any main-frame `framenavigated`, so this event closed the window
   before the new document committed.
5. WebKit then reported `Context is stopped` for the old document's teardown. It arrived outside the window
   and was counted. The new document's own commit followed.

So the window closed on a same-document event of the old document, and the test reloaded an address that the
page was still rewriting. With the page allowed to settle before the reload, 8 of 8 runs passed. The reload
also loaded either address, so the old test did not reliably test a cold load of the unknown section.

**What this does not show.** It does not say which call in the old document raises `Context is stopped`; that
is still open (see [the entry of 2026-11-17](2026-11-17-webkit-context-stopped-at-teardown.md)). It shows
that this spec's failure came from the test's race and from the window closing on a same-document event, not
from a change in the app. Leaving a settled page for `about:blank` instead raised the error three times in
one run, because that navigation has no request that opens the window; it is not a quiet way to leave.

**How to apply.** A spec that needs a cold load of an address sets it with `replaceState` and reloads, once the
page is settled. Do not `goto` a hash and reload straight after: the page may still rewrite the hash. The
2026-11-17 rule stands: no retries and no app code for this error without a cause.
