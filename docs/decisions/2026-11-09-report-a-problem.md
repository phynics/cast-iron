---
title: "Report a problem"
date: 2026-11-09
status: current
superseded_by: []
issues: [140]
prs: []
area: ui
---

# Report a problem, 2026-11-09

Closes [#140](https://github.com/phynics/cast-iron/issues/140). Owner decisions recorded in the issue on 2026-11-09.

- **One footer control, routed by the page.** The old "Send feedback or report a problem" link is replaced by one "Report a problem" link at the bottom of every Reader page. An entry route opens the cast-iron-data "Problem in an entry" form with `template=data-problem.yml&entry=<ref id>`; every other route opens the app form in this repository with `template=app-problem.yml&route=…&app-version=…&content-version=…&schema-version=…`. There are not two report buttons, and "Source on GitHub" stays as a separate source link.
- **Only fields the form has are prefilled.** `data-problem.yml` currently carries `entry` as its only identity field, so the entry link sends that field and nothing else. Its title, kind, route and versions are not invented as query parameters the form does not define. The app form added in `.github/ISSUE_TEMPLATE/app-problem.yml` has fields for the route, app version, library `content_version`, `schema_version` and "What's wrong?"; the link fills all but the reader's description.
- **The URL is native Rust.** `crates/web/src/report.rs` builds both forms and percent-encodes every key and value, so `&`, `#`, quotes and non-ASCII text cannot change the query. Entry body text never enters the URL. The encoded route is capped at 4,096 bytes so the prefilled URL stays well under GitHub's 8 KB limit.
- **The app version is the shell version.** Settings and the link read the same service-worker `SHELL_VERSION`; the link asks the controlling worker once it claims the page and omits the field until it answers. The version is display data only, like the route and library versions.
- **Nothing is fetched before the click.** The control is a plain `target="_blank" rel="noopener noreferrer"` anchor. Offline behavior needs no special case and no hint: the device browser cannot load GitHub, and the app itself makes no request.
- **The control is a 44 px text action, not a pill.** It sits in the existing footer link line, names the entry in its accessible name on an entry page, and says "opens in a new tab". The wording names no publisher and implies no official status.
