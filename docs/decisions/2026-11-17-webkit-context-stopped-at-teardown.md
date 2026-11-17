---
title: "WebKit's \"Context is stopped\" page error is tolerated only while a page navigates away"
date: 2026-11-17
status: current
superseded_by: []
issues: [121]
prs: [473]
area: process
---

# WebKit's "Context is stopped" page error is tolerated only while a page navigates away, 2026-11-17

**Decision.** The console guard in `tests/e2e/tests/fixtures.ts` does not count one page error: the exact
message `Context is stopped`, on WebKit only, while the main frame navigates away. The window (`TeardownWindow`
in `tests/e2e/tests/teardown-window.ts`) opens when a main-frame navigation request starts. It closes when the
main frame commits the new document (`framenavigated`) and when the tracked request fails or is cancelled. A
newer request or a redirect target replaces the tracked request, and the failure of a superseded request does
not close the window of the newer one. Each tolerated error is recorded in `guard.teardown` and attached to
the test as a `webkit-teardown-error` annotation, with the URL that the page was navigating to. The same
message outside the window, including from the replacement document and from a page that survived a failed
navigation, still fails the test. So does every other page error. `console-guard.spec.ts` (WebKit) and
`teardown-window.spec.ts` check these boundaries.

**This is a containment, not a diagnosis.** The cause is not found, and it stays open in #121.

**What was observed** (Linux, the WebKit that Playwright ships; not an iPhone):

- Four tests of `class-sections.spec.ts` and `revisions.spec.ts:39` failed only through the guard with this
  message, while their own assertions passed. In a local loop (`goto` an entry hash, then `reload()`), the old
  page's error arrived between the navigation request and the new page's `framenavigated`, in every recorded
  timeline.
- The error has an empty stack. No `unhandledrejection` or `error` event reached the page.
- Each of these changes left the error unchanged: a handler on every Cache API, `fetch`, service worker and
  IndexedDB promise that the page creates; a handler on the `idbTxDone` promise; no `requestAnimationFrame`;
  no timers of 250 ms or more; no service worker `postMessage`; no worker registration; dropping every
  promise, timer and animation-frame callback after `pagehide`.
- A reload of the idle app page, after the app has settled, did not raise it. A `cache.match` was pending at
  `pagehide` in every case that did.

**What does not show a cause.** An earlier version of this entry called the error "WebKit-internal" on the
strength of a reload of `manifest.webmanifest`. That evidence was wrong. `web/sw.js` answers every in-scope
navigation with the cached `index.html` (`serveIndex`), so that page was the app, not a document without script,
and the blocked-worker comparison also removed the app. A page that really has no script was tested instead: a
static HTML page of one paragraph, `0` scripts, served with HTTP 200, once under a worker that answers with
`respondWith(fetch(request))` (the response came from the worker, `fromServiceWorker()` true), once under a worker
with no fetch handler, and once with service workers blocked. In each, the page reloaded 8 times with 0 errors. On
that script-free page, 50 pending `cache.match` calls, a pending IndexedDB open and 10 pending same-origin
`fetch` calls at reload, with and without a controlling worker, also raised 0 errors in 8 reloads. So service
worker control, and a pending Cache, IndexedDB or fetch call by themselves, do not produce the error. It needs
something that the app does. Which call, and whether it is an app callback or a WebKit report about app work
that was in flight, is unknown.

**Effect.** No user-visible effect has been observed: after the interrupted navigation, the affected tests'
assertions on the new page passed. That is not proof that no write is lost. A reload can interrupt an
IndexedDB write or an install on any browser, and the install code protects against that independently (it
verifies before it activates, and it activates and clears the pending marker in one awaited transaction).
Linux Playwright WebKit is not an iPhone.

**How to apply.** Do not add retries or app code for this error without a cause. A page error with another
message, or this message when the page is not navigating away, is a finding. Keep #121 open. When the cause is
found, remove the tolerance and this entry's window.
