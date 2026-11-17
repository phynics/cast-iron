# Supplemental conditional-view frames for #448

The [40-frame conditional-view archive](https://github.com/phynics/cast-iron/tree/42e9a57eab1dad150fe899f356f23d24e96a8132) names two states that its frames do not show: the iOS Settings install row, and the combined-update reload row at phone width. This directory holds six frames that scroll each subject into view, plus the record of every check in `manifest.json`.

## Frames

All six are viewport shots of the same state as the archive frame they supplement, with the named subject scrolled to the middle of the viewport. Bounds are CSS pixels from the `scroll_to` record.

| File | Viewport | Theme | Subject bounds |
|---|---|---|---|
| `combined-updates-settings-reload-row-phone-light.png` | 390×844 | light | 366×44 at y=436 |
| `combined-updates-settings-reload-row-phone-dark.png` | 390×844 | dark | 366×44 at y=436 |
| `ios-guide-settings-install-row-scrolled-phone-light.png` | 390×844 | light | 366×106 at y=468 |
| `ios-guide-settings-install-row-scrolled-phone-dark.png` | 390×844 | dark | 366×106 at y=468 |
| `ios-guide-settings-install-row-scrolled-desktop-light.png` | 1280×900 | light | 656×81 at y=627 |
| `ios-guide-settings-install-row-scrolled-desktop-dark.png` | 1280×900 | dark | 656×81 at y=627 |

- `settings-install-row-scrolled` (WebKit): the App section with "Install Cast Iron as an app for full-screen reading and an icon on your device." and its "Install app" button. The sentence wraps to two lines on the phone and one on the desktop.
- `settings-reload-row` (Chromium, phone only): the end of the library offer ("Download update"), then the app reload row "App update ready. [Reload]", then Automatic checks. This is the combined state at phone width; the desktop frames in the archive already show both rows.

The scrolled frames crop the panel above the subject by design. The `combined-updates` phone frames show the app's own "Last checked: 18 Nov 2026" because the run crossed local midnight; that date is not a fixture.

## Provenance

- **App build:** commit `8d45b9d6b0621b9c8912191f2f8183ba11932716`, shell `ea714651ab69553d`, content `veada5093fd101aa3`, schema 6, corpus `v0.1.25`, 31800 entries, `source_dirty` false. The build was reproduced from a checkout of `8d45b9d6b` with the pinned corpus `v0.1.25` (`cargo xtask fetch-sources`, `cargo xtask content`, `cargo xtask build-site --public-url /cast-iron/`). The shell and content versions equal the archive manifest's, so these frames are of the same build as the 40.
- **Fixture:** `tests/e2e/capture-conditional.mjs` and `tests/e2e/capture-lib.mjs` from branch `t3/ios-install-row-captures` at `11ff4cbf0d4ec35a4b71c09d032e504b9726fc16`, which is PR #486 head `0eaa51b185ae8d375612b4cc7243aaead47befed` plus the scrolled-capture change. The fixture is test-only; it is not part of `dist` and was not in `8d45b9d6b`.
- **Harness:** Playwright `1.64.1`, Node `v24.21.0`, `linux-x64`, WebKit 27.2 (engine simulation), Chromium 156.0.8078.4.
- **Evidence class:** the iOS frames are WebKit engine simulation with an iPhone Safari user agent, mobile viewport and touch emulation, as in the archive. They are not iOS Safari and not a device; physical-device acceptance stays with [#423](https://github.com/phynics/cast-iron/issues/423).

## Method

```
node tests/e2e/capture-conditional.mjs <out> --only=ios-guide,combined-updates
```

Before each scrolled shot the script scrolls the subject into the middle of the viewport and requires all of the following, then saves the frame:

- the element's box is fully inside the viewport;
- its top is below the sticky header;
- it is rendered (`visibility`, `display`, `opacity`);
- `elementFromPoint` at its top centre returns the element or a child;
- its visible `innerText` contains the expected strings.

A failed check ends the capture instead of saving a frame. `manifest.json` records the measured bounds, viewport, scroll offset, text and each check for all six. Every frame also passed the frame probe: the masthead is in the saved pixels.

The same run also captured the 20 states that the archive already holds. 13 files are byte-identical to `42e9a57eab1dad150fe899f356f23d24e96a8132`; the other 7 differ only in small regions (the header Install button focus/hover ring behind the modal in the four `guide-dialog` frames, one 9x12 glyph on the two desktop `settings-install-row` frames, an 11-pixel sliver on `combined-updates-settings-updates-desktop-light`), with no text or layout change. Those 20 files are not duplicated here; the archive keeps them.

## Known artifact

The four WebKit frames record the known console artifact of the archive manifest: `Refused to apply a stylesheet because its hash, its nonce, or 'unsafe-inline' does not appear in the style-src directive of the Content Security Policy.` Playwright injects the stylesheet into persistent WebKit contexts and the page CSP refuses it. No frame shows an effect.

## Verify

```
sha256sum -c SHA256SUMS
```

All files are also addressed by blob in git at the commit that added them, so the archive zip is not the only copy.
