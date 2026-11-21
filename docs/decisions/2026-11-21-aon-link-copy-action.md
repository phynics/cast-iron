---
title: "The AoN link stays a plain anchor and gains a Copy link button"
date: 2026-11-21
status: current
superseded_by: []
issues: [514]
prs: []
area: ui
---

# The AoN link stays a plain anchor and gains a Copy link button, 2026-11-21

Issue #514: the owner reports that **View on Archives of Nethys** "opens in place" in the installed Android app. [2026-11-19-aon-link-opens-where-the-platform-decides](2026-11-19-aon-link-opens-where-the-platform-decides.md) found that the platform, not the app, decides where the page opens, and kept the anchor. This entry adds the primary sources that settle the other options, and one small action for the reader who wants a real browser.

## What each platform does

| Platform | Where the tapped link opens | Source |
|---|---|---|
| Desktop browser | A new tab. | Tested: `tests/e2e/tests/aon-links.spec.ts`, "Tapping the link opens a separate page". |
| Desktop installed app | The system browser, for a `_blank` link. | web.dev: the `_blank` route "works only on desktop PWAs" ([Window management, Force a browser's navigation](https://web.dev/learn/pwa/windows)). Not re-tested here. |
| Android, installed (WebAPK) | An in-app browser over the app (a Chrome Custom Tab), also for `target="_blank"`. The app's own page is not navigated, so closing the tab returns to the same entry. | web.dev: "On Android, links with `target="_blank"` open in a Chrome Custom Tab" ([Add a web app manifest](https://web.dev/add-manifest/)); "Opening new windows on Android creates a new in-app browser for the URL" and "On mobile devices, there's no option to open a browser with a URL" ([Window management](https://web.dev/learn/pwa/windows)). |
| iOS, Home Screen web app | Safari View Controller: "links outside the scope will open in Safari View Controller" ([WWDC23, What's new in web apps](https://developer.apple.com/videos/play/wwdc2023/10120/)); web.dev adds that its storage is isolated from the PWA ([Window management](https://web.dev/learn/pwa/windows)). | As cited. |

To a reader, the Custom Tab covers the whole screen, so it can look like the page opened "in place". Both mobile in-app browsers can be closed: [Window management](https://web.dev/learn/pwa/windows) says "Users can close the browser or go back" and "your PWA is waiting in the background", so the app returns on the same entry.

## The app does not intercept the tap

`aon_line` renders a plain anchor with `target="_blank"` and `rel="noopener noreferrer"`. The app has no document-level click handler, the router listens to `hashchange` only, and `web/sw.js` returns without handling any request whose origin is not its own. The e2e test taps the link in Chromium and checks that a separate page opens at the stored URL with no `window.opener`, that the app keeps its address and entry, and that the tap is the only request to the reference site.

## Options

| Option | Verdict |
|---|---|
| Keep the anchor | **Keep.** Plain markup: no JavaScript, no request before the tap (rules 1 and 8). It follows each platform's own rule for an external link. |
| `window.open(url, "_blank", "noopener")` from a click handler | **Rejected.** On Android it opens the same in-app browser as the anchor ("Opening new windows on Android creates a new in-app browser", [Window management](https://web.dev/learn/pwa/windows)). On iOS the same page says "When you call `open()` within a PWA window on iOS or iPadOS, it returns `null` and doesn't open a window", so the tap could do nothing. It adds JavaScript for a result that is the same or worse. |
| Android `intent:` URL to force the browser | **Rejected.** None of the sources read says that an installed app may start a browser this way, or that it would not return to Chrome or to the fallback URL. It needs user-agent detection in JavaScript, and it gives a dead link where `intent:` is not understood. Not tested: no device. |
| Manifest `scope` or `display` | **Rejected.** `scope` limits URLs of the manifest's own origin and cannot cover `aonprd.com`. `display: minimal-ui` or `browser` would trade away the installed-app experience for every page. |
| A **Copy link** button | **Added.** Both mobile platforms put the page in their own in-app browser and give the page no way to choose another. Copying the URL is the route expected to work on every platform: the reader pastes it into any browser. It was tested in Chromium; `writeText` in an installed Android app and an iOS Home Screen app awaits the device check below. |

## The change

`aon_line` adds a `Copy link` button after the link, in the same muted Metadata role, with the same 44 px hit area. It calls the existing `js::copy_text` (`navigator.clipboard.writeText`, by reflection: no new bridge code and no new `web-sys` feature) and writes `Link copied.` to a polite status line, or `Could not copy. Press and hold the link to copy it.` when the clipboard refuses. The button copies the stored URL and makes no request, so rule 1 holds; the anchor is unchanged, so the 2026-11-19 behaviour holds. The wording names the reference site only and does not imply official status (rule 11).

## What we could not verify

No Android or iOS device was available. The Custom Tab and Safari View Controller behaviour comes from the cited documentation, not from a device test. The `Copy link` button was tested in Chromium only; a device check should confirm that `navigator.clipboard.writeText` succeeds from a tap in an installed WebAPK and in an iOS Home Screen web app (it needs a secure context and a user gesture, both present). If it does not, the status line gives the long-press fallback on the link.
