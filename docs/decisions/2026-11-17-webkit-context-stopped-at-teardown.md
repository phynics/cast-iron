---
title: "WebKit's \"Context is stopped\" page error at teardown is not counted"
date: 2026-11-17
status: current
superseded_by: []
issues: [121]
prs: []
area: process
---

# WebKit's "Context is stopped" page error at teardown is not counted, 2026-11-17

**Decision.** The console guard in `tests/e2e/tests/fixtures.ts` ignores one page error: the message
`Context is stopped`, on WebKit only, and only while the page navigates away (from the main frame's
navigation request until the next `load`). The guard counts the ignored errors in `guard.teardown`. The same
message outside that window still fails the test, and so does every other page error.

**Why.** The error has no app cause. It was the recurring WebKit failure of #121: `class-sections.spec.ts`
(four tests, which `goto` an entry and `reload` it) and `revisions.spec.ts:39`, each failing only through the
guard while its own assertions passed. The evidence, from the WebKit that Playwright ships:

- The error has an empty stack, and no `unhandledrejection` or `error` event reaches the page for it.
- Wrapping every Cache API, `fetch`, service worker and IndexedDB promise with a handler did not remove it.
  Marking the `Tx` completion promise in `bridge.js` as handled did not remove it either. Disabling
  `requestAnimationFrame`, timers, the worker's `postMessage` and the worker registration did not remove it.
- A plain same-origin document with no app script (`manifest.webmanifest`) raised it on 8 of 8 reloads
  while a service worker controlled it, and on 0 of 8 in a context with service workers blocked. The
  worker (`web/sw.js`) never messages a page. So WebKit raises the error when it tears down a document that a
  service worker controls. No app callback runs after its page is gone.
- The page timeline shows the error between the navigation request and the new page's `framenavigated`,
  always during the old document's teardown, never during the new one.

It is harmless: the document is already being replaced, and each affected test's own assertions on the new
page passed.

**How to apply.** Do not add retries or app code for this error. A WebKit page error with another message,
or this message after the page has loaded, is a finding: investigate it. A Chromium run is unchanged.
