---
title: "The AoN link opens where the platform decides; the app keeps a plain target=_blank anchor"
date: 2026-11-19
status: current
superseded_by: []
issues: [514]
prs: []
area: ui
---

# The AoN link opens where the platform decides; the app keeps a plain target=_blank anchor, 2026-11-19

The owner reported that **View on Archives of Nethys** opens "in place" in the installed Android app. This entry records what the platforms do, why the link stays as it is, and what a reader can do.

## What each platform does

- **Android, installed (Chrome WebAPK).** An off-scope `target="_blank"` link opens in a **Chrome Custom Tab** laid over the app, not in a new browser tab and not in the app window. web.dev says so: "On Android, links with `target="_blank"` open in a Chrome Custom Tab" ([Add a web app manifest, scope](https://web.dev/articles/add-manifest)). Chromium routes `_blank` links and `window.open()` off-origin navigations from an installed web app to a Custom Tab in a separate Android task, "which mimics creating a new tab/window in the browser" ([Chromium CL 2898373002, bug 709889](https://codereview.chromium.org/2898373002), 2017; the review does not say "WebAPK" and the behaviour was not re-checked on a device here). To a reader the Custom Tab, which has a toolbar with a close button and covers the whole screen, looks like the page "opened in place". Closing it returns to the app on the same entry. A Chromium tracker issue titled "Links from PWA always open in custom tab mode on Android. Create way to open in browser" (issues.chromium.org/issues/40745354) asks for a way out of this; it needs a sign-in to read, so its status is not recorded here.
- **Android, in a browser tab.** A normal new tab.
- **iOS, installed (Home Screen web app).** Since iOS 12.2 an off-scope link opens in an in-app browser inside the app, with a Done button; with `target="_blank"` the app context is restored when it closes. No attribute, script or URI scheme was found that opens Safari ([Firtman, iOS 12.2](https://firt.dev/ios-12.2/), 2019; later iOS versions were not verified here).
- **Desktop.** A new browser tab (the Playwright test asserts a separate page).
- MDN states the general rule: off-scope navigations "are not blocked by browsers and are allowed to be opened in a new top-level browsing context", and browsers add UI such as the URL bar ([`scope`](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/scope)).

## The app does not make it worse

No app code handles a click on this link. The app has no document-level click or anchor listener (`crates/web/src` has only element-level handlers), the router listens to `hashchange` only, and `web/sw.js` returns without handling any request whose origin is not its own, so an off-origin navigation never reaches it. `tests/e2e/tests/aon-links.spec.ts` ("Tapping the link opens a separate page…") taps the link in Chromium and asserts that a second page opens at the stored AoN URL with no `window.opener`, that the app window keeps its address and entry, and that the only request to the reference site is the tapped one.

## Options and the decision

| Option | Android | iOS | Desktop | Verdict |
|---|---|---|---|---|
| Keep `<a target="_blank" rel="noopener noreferrer">` | Custom Tab in its own task (documented) | In-app browser, app restored on close | New tab | **Keep.** Plain markup: no JavaScript, no request before the tap (rules 1 and 8), no change to any platform's native handling. |
| `window.open(url, "_blank", "noopener")` | Same Custom Tab: Chromium routes `window.open` and `_blank` together (CL above) | No way to Safari found | New tab | No gain, and it adds JavaScript to the bridge. Rejected. |
| `intent:` URL to force the browser app | An `intent:` URL asks the system to start an activity. Whether that reaches a full browser window, or comes back to Chrome or the fallback URL when Chrome is the default browser, is not documented in the sources read and could not be tested. Needs UA detection in JS. | Not supported | Not supported | Rejected: unverifiable, platform-sniffing, and it leaves the app with a dead link where `intent:` is not understood. |
| Manifest changes (`scope`, `display`) | Widening `scope` to `https://aonprd.com/` is not possible: scope must be on the manifest's own origin. A browser `display` mode would drop the installed-app experience. | n/a | n/a | Rejected. |
| An extra "Copy link" action | Lets a reader paste into the browser of choice | Same | Same | Not done here; it is new UI for a platform choice that the Custom Tab already handles well. Open a separate issue if the owner wants it. |

## What a reader can do

On Android, the Custom Tab's toolbar menu has **Open in Chrome** (or the default browser); closing the tab returns to the same entry. On iOS, the in-app browser has a Done button and the app returns to the same entry. The link text and its screen-reader suffix, "(opens in a new tab)", stay as they are.

## Residual uncertainty

No Android or iOS device was available, so the Custom Tab and in-app browser behaviour comes from the sources above, the oldest of which is from 2017, and from the owner's report, which matches it. If a device test finds that the installed app's own window navigates, that would be a Chrome bug and a new issue; the Chromium test above shows the app itself does not do it.
