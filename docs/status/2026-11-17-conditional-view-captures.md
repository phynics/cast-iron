---
title: "Conditional-view captures and the masthead discrepancy"
date: 2026-11-17
kind: record
---

# Conditional-view captures and the masthead discrepancy, 2026-11-17

Refs [#448](https://github.com/phynics/cast-iron/issues/448), part of [#443](https://github.com/phynics/cast-iron/issues/443). This is local evidence from a headless run on one Linux host. CI and deployment did not take part. It is screenshot evidence, not a device, accessibility or upgrade-integrity result: the physical iPhone work is [#423](https://github.com/phynics/cast-iron/issues/423), and the format 5 upgrade check is [#441](https://github.com/phynics/cast-iron/issues/441).

## What was captured

Build under test: commit `8d45b9d6b0621b9c8912191f2f8183ba11932716` with a clean source tree, shell version `ea714651ab69553d`, content `veada5093fd101aa3` (schema 6), corpus tag `v0.1.25`, `dist/` from `cargo xtask build-site --public-url /cast-iron/`. Chromium 156.0.8078.4 and WebKit 27.2 from Playwright 1.64.1.

| Group | States | Engine | Per state |
|---|---|---|---|
| `storage-failure` | `cannot-start` | Chromium | phone 390×844 and desktop 1280×900, light and dark |
| `shell-update` | `reading-header`, `settings-updates` | Chromium | same four |
| `library-update` | `reading-header`, `settings-updates` | Chromium | same four |
| `combined-updates` | `reading-header`, `settings-updates` | Chromium | same four |
| `ios-guide` | `header-install-button`, `guide-dialog`, `settings-install-row` | WebKit, iPhone Safari user agent | same four |

That is 40 PNGs. The `ios-guide` frames are **engine simulation**: WebKit with an iPhone Safari user agent, touch and mobile emulation. They are not iOS Safari, not a device and have no Home Screen; the manifest repeats this on every one of those captures. Every capture records commit, shell and content version, corpus tag, engine and version, viewport, theme as applied, the settled state (ready selector, fonts loaded, no running animation) and the masthead probe. The manifest is [448-conditional-views.json](../research/capture-evidence/448-conditional-views.json); each group's reproduction steps are in its `groups` list.

Reproduce with the scripts, not by hand:

```sh
cargo xtask build-site --public-url /cast-iron/
node tests/e2e/capture-conditional.mjs <out-dir> [--only=storage-failure,shell-update,library-update,combined-updates,ios-guide]
```

Fixtures, all through the test server's hooks and a page init script, so nothing in the app changed:

- **Storage failure.** An init script replaces `IDBFactory.prototype.open` with a function that throws `SecurityError`. The app stops at `Boot::Fatal` before the reader starts.
- **App-shell update.** `POST /__test/shell` makes the server serve `sw.js` with a different `SHELL_VERSION`; Settings → Check now installs it and waits.
- **Library update.** `POST /__test/overlay` serves a copy of the manifest that names a newer content version (`vcapturenext0000000`, synthetic). Check now offers it; it is never downloaded. The size shown (213 MB) is the real library's.
- **Combined.** Both of the above.

## Observations for the assessment (not scores)

The council scores these views; these notes only record what the frames show, so the scoring can start from them.

- **Storage failure.** A bare page: heading and one alert, no masthead, no Home link, no retry or other action, and the browser's raw error text (`local storage error: SecurityError: The operation is insecure.`) is part of the sentence. The rest of the page is empty at both sizes.
- **App update.** On a reading page the header action is an icon with a pip on the phone and "Update ready" on desktop; Settings → Updates shows "App update ready." with Reload. The reading column does not move.
- **Library update.** Settings → Updates states the newer version and size, the "stars and book choices are kept" line, a facts box, the "On iPhone?" paragraph, a second "Your stars stay" line and Download update. The two reassurance lines say nearly the same thing.
- **Combined.** The header action reads "Update ready"; the Updates section stacks the library offer above the app Reload row.
- **iOS guide.** The three steps fit at both sizes. On the phone the dialog sits at the bottom with Done focused; on desktop emulation it is a small centred modal over a dimmed page.

## The masthead discrepancy

The audit saw Chromium frames with no top masthead although the measurement before the shot said the header was at y=0 and 61 px tall. [diagnose-masthead.mjs](../../tests/e2e/diagnose-masthead.mjs) takes the same pages (search results and the Fighter class page) at scroll 0 and 1500, in three capture modes, on phone and desktop, and records scroll position, focus, header bounds and computed style, the element at the header's centre, and what the saved frame shows. Each probed PNG is the exact bytes written to disk (the saved file is read back and probed, and its SHA-256 is in the row; an element frame of a long page is probed from its buffer and not saved, and its row says `frame_saved: false`). The masthead must be there as pixels (header background, an unbroken bottom border, ink on the brand), not only in the DOM. Every row also records engine and version, theme and the settled state. The diagnosis manifest's `build.commit` is the PR commit at the time of the run; `dist/` was built from `8d45b9d6b` and nothing under `crates/`, `web/` or `sources/` differs between them (`source_dirty` is false). Data: [448-masthead-diagnosis.json](../research/capture-evidence/448-masthead-diagnosis.json).

| Capture mode | Scroll 0 | Scroll above 0 |
|---|---|---|
| Viewport shot | masthead in 4 of 4 | masthead in 4 of 4 |
| Full-page shot | masthead in 4 of 4 | masthead in 0 of 4 |
| Element shot of `<main>` | 0 of 4 (never contains it) | 0 of 4 |

In every one of the 24 rows `getBoundingClientRect()` says y=0, height 61, `position: sticky`, and the hit test at the header's centre lands on the header. Focus was on `<main>`, the search input or `<body>` and made no difference. So the DOM measurement is right and the frame can still lack the masthead.

**Cause: a capture artifact, not a navigation defect.** The header is `position: sticky; top: 0`. A viewport shot always carries it. A full-page shot of a page that is scrolled does not put the sticky header at the top of the frame. On desktop it lands at the scroll offset (y=1500 for a scroll of 1500). Under phone emulation it lands elsewhere and moves between runs (frame rows 236, 2921 and 2340 were seen, and in one frame it was outside the first 4000 rows). The top of the image then shows page content where the masthead should be. An element shot of `<main>` excludes the header by construction. A reader never sees any of this: in the browser the viewport always carries the header. No runtime evidence supports a product defect, so none is filed. I cannot see the audit's capture script, so this shows the mechanism that reproduces the symptom, not which mode the audit used.

**Fix in the capture tooling.** `capture-ui.mjs` and `capture-conditional.mjs` take viewport shots by default, settle first and read every saved frame back. `shoot()` in `capture-lib.mjs` scrolls to the top before a full-page shot. A frame without the masthead prints a warning, and the conditional script exits 1. In this run the 44 frames of `capture-ui.mjs` ([448-ui-states.json](../research/capture-evidence/448-ui-states.json)) and the 40 conditional frames all passed; the four storage-failure frames and the four guide-dialog frames are marked `masthead_expected: false` with the reason (no masthead on that screen; the modal backdrop dims it).

## Stale fixtures refreshed

`capture-ui.mjs` assumed things that no longer hold:

- Inline per-book disclosures on Books. The page now lists priority tiers; the script opens every tier and series, then opens Core Rulebook from its row and captures the book page.
- A duplicate "Anatomist" trait. The source-variants fixture now comes from the compiled catalog: the first title, in a fixed kind and title order, that two books publish **and** that the browse list renders as one disclosure (a title that the list merges into one row, such as an Elephant revision pair, does not qualify). In this run it was the creature "Acolyte". The letter page is opened with `?l=`.
- The Rules text wait expected every row to start with "Rule ·". The scope also lists Stamina and Mythic add-on rows, and the first row can be a publication group with no anchor. The waits now check the `kind=rule` URL and the first row.

## Where the images are

The images are not committed. Attaching them to a comment on #448 is still pending; until then they exist only as local generated files. The JSON manifests above are in the repository. See [the decision](../decisions/2026-11-17-capture-evidence-manifests-in-the-repo-images-on-the-issue.md).
