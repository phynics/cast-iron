---
title: "WebKit stops a Cache read on navigation"
date: 2026-11-08
status: current
superseded_by: []
issues: [121]
prs: []
area: install-offline
---

# WebKit stops a Cache read on navigation, 2026-11-08

Refs [#121](https://github.com/phynics/cast-iron/issues/121).

- **A stopped Cache API read is a page error in WebKit.** Home warms the search index in the
  background (a `Cache.match` plus `arrayBuffer` of the whole index). When the document is
  reloaded or left while that read is in flight, WebKit stops its context and reports
  `pageerror: Context is stopped` (from `DOMCacheEngine`). No JavaScript handler receives the
  rejection, so the app cannot catch it and the promise cannot be cancelled. Chromium reports
  nothing. Home already marks the end of the read with `.home-page[data-index-ready=true]`, and
  `installLibrary` waited on it before this change; this change only extends that wait to the
  offline recent-searches test. It is a signal from the app, not a timeout or a retry.
- **Recently viewed is one app-wide signal.** The visit is recorded by the entry screen and the
  list is shown by Home. The `recents` signal in `AppCtx` is the single source for both, loaded
  once at startup (`recent::load_into`) and updated after the store write
  (`recent::record`). A Home that reads the store before that write lands renders the row when
  the signal updates, instead of showing an empty "Recently viewed" until a reload. The stored
  list stays the durable source; `merge_stored` keeps a visit that arrived during the startup
  read from being overwritten by that read's older answer.
