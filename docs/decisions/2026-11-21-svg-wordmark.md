---
title: "The masthead wordmark is an outlined SVG with a static oil-slick sheen"
date: 2026-11-21
status: current
superseded_by: []
issues: [181, 194]
prs: []
area: ui
---

# The masthead wordmark is an outlined SVG with a static oil-slick sheen, 2026-11-21

The owner decided on 2026-11-20 (#194) that the brand text becomes an original SVG "Cast Iron" wordmark with a subtle, static iridescent sheen, like seasoned cast iron.

## Decision

- **Artwork.** "Cast Iron" is outlined from Bevan Regular (Copyright 2016 The Bevan Project Authors, SIL Open Font License 1.1) into one path in `web/icons/wordmark.svg` (3.2 KB, no font reference). The licence text is `LICENSES/OFL-1.1-Bevan.txt`. Bevan has no Reserved Font Name. The outlines are artwork and not a font, and the licence allows that use. Two changes make it ours: tight hand-set tracking, and a round hanging hole cut in the capital I, which reads as the hole in a skillet handle.
- **Variants compared** (390 px and 1280 px, light and dark, in the real masthead styles): Bevan in mixed case with the hole; Alfa Slab One in capitals; Rozha One in mixed case. Capitals read as the strongest "iron" but are 8.2 times as wide as high, shout in a 390 px header and sit close to the capitals of the publisher's own logo, which rule 11 says to avoid. Rozha's hairlines break up at 20 px. Bevan in mixed case holds its weight at that size and looks like a foundry specimen, so it won.
- **Rendering.** The SVG is a CSS alpha mask on `span.brand-mark`; the paint is `--ink` under a fixed 100° gradient whose stops mix blue, violet, teal and amber into `--ink` (`color-mix` in oklab, 30 to 34 % tint). One file therefore serves both themes and no hex value is added to the tokens. A browser without `color-mix` keeps plain `--ink`. In forced-colors mode the paint is `CanvasText`.
- **No animation.** The gradient is static, so Reduce Motion needs no handling.
- **Accessible name.** The mark is `aria-hidden`; a `.sr-only` "Cast Iron" inside the brand link is the one name. The header height (`--header-h`) and the 44 px brand and search targets stay.
- **Size.** The mask is an external file (`icons/wordmark.svg`, relative to the stylesheet), copied with the other icons and precached with the shell like them. Nothing is added to the WASM.
- **Idle search (#181).** Checked on current `main`: the header search icon is a bare `--ink-muted` glyph with no border or fill until hover or focus, so it already reads as secondary and needs no change.

## Consequences

- Changing the wordmark means regenerating the path from the font; keep the licence file with it.
- The brand's left edge is the ink's left edge (the viewBox is cropped to the glyphs), which keeps the brand aligned with the page title.
