---
title: "The masthead wordmark is an outlined SVG with a static oil-slick sheen"
date: 2026-11-21
status: current
superseded_by: []
issues: [181, 194]
prs: [551]
area: ui
---

# The masthead wordmark is an outlined SVG with a static oil-slick sheen, 2026-11-21

The owner decided on 2026-11-20 (#194) that the brand text becomes an original SVG "Cast Iron" wordmark with a subtle, static iridescent sheen, like seasoned cast iron. On 2026-11-21 the owner reviewed three variants and chose Rozha One in mixed case.

## Decision

- **Artwork.** "Cast Iron" is outlined from Rozha One Regular (Copyright (c) 2014, Indian Type Foundry, SIL Open Font License 1.1) into one path in `web/icons/wordmark.svg` (3.4 KB, no font reference). The licence text is `LICENSES/OFL-1.1-Rozha-One.txt`. The copyright line declares no Reserved Font Name, and the outlines are artwork and not a font, so modifying them is allowed. The changes: one-unit tracking, and a round 4-unit stroke on the path (200 units to the em) that thickens the hairlines.
- **Variants compared** (390 px and 1280 px, light and dark, in the real masthead styles): Bevan in mixed case with a hanging hole in the capital I, Alfa Slab One in capitals, and Rozha One in mixed case (chosen by the owner). Capitals were the widest (8.2 times as wide as high) and sit close to a capitals logo, which rule 11 says to avoid.
- **Hairlines at 390 px.** Plain Rozha loses its serifs and thin strokes at 20 px. A stroke of 2.4 units kept them; 4 units made them read cleanly at 2x, so 4 won, and the mark is 1.4rem high (22 px). That stays inside the header (`--header-h`, 3.75rem) and the 44 px brand target. A 5.5-unit stroke began to close the counters of the a and the s.
- **Hanging hole dropped.** The skillet-handle hole of the Bevan variant does not survive in Rozha: the capital I's stem is 150 units wide, and after the stroke a hole large enough to read leaves hairline walls. At a size that fits it is a pinprick that looks like a flaw.
- **Rendering.** The SVG is a CSS alpha mask on `span.brand-mark`; the paint is `--ink` under a fixed 100° gradient whose stops mix blue, violet, teal and amber into `--ink` (`color-mix` in oklab, 30 to 34 % tint). One file therefore serves both themes and no hex value is added to the tokens. A browser without `color-mix` keeps plain `--ink`. In forced-colors mode the paint is `CanvasText`.
- **No animation.** The gradient is static, so Reduce Motion needs no handling.
- **Accessible name.** The mark is `aria-hidden`; a `.sr-only` "Cast Iron" inside the brand link is the one name. The header height (`--header-h`) and the 44 px brand and search targets stay.
- **Size.** The mask is an external file (`icons/wordmark.svg`, relative to the stylesheet), copied with the other icons and precached with the shell like them. Nothing is added to the WASM.
- **Idle search (#181).** Checked on current `main`: the header search icon is a bare `--ink-muted` glyph with no border or fill until hover or focus, so it already reads as secondary and needs no change.

## Consequences

- Changing the wordmark means regenerating the path from the font; keep the licence file with it.
- The brand's left edge is the ink's left edge (the viewBox is cropped to the glyphs, stroke included), which keeps the brand aligned with the page title.
