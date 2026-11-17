---
title: "Class pages: phone-friendly progression"
date: 2026-11-15
status: current
superseded_by: []
issues: [170]
prs: []
area: ui
---

# Class pages: phone-friendly progression, 2026-11-15

Refs [#170](https://github.com/phynics/cast-iron/issues/170), part C (the section switcher, summary card, archetype list, class options and collapsed references are other parts). On `#/entry/ref/class/wizard` the progression table was 16 columns and 1,334 px wide at 390 px: Level, BAB and Fort were visible, nothing was pinned.

- **The split is a render-time transform of the typed table, not a compiler change.** `views/progression.rs` reads the header cells. A progression is a table with exactly one header row that starts `Level`, includes `Base Attack Bonus`, has no spans and one cell per column in every row. Its spell columns are the headers `0`, `1st`…`9th` (any ordinal) and `Spells Per Day` (the one column of a prestige class); every other column stays in the core table. `SCHEMA_VERSION` does not change, and the compiler output is byte-identical. The corpus holds 190 such tables (49 classes, 115 prestige classes, 26 rule copies such as `ref/rule/class-wizard/wizard`); 104 have spell columns. A table that does not match (the Spells known tables, the Archetype matrix, a rule's `Level / Damage` table) renders as before.
- **Level is in both tables.** It is the row key. The first cell of each body row becomes a row header (`th scope="row"`) in both, because the corpus stores it as a plain cell.
- **Extra class columns stay in the core table** (Monk's Flurry of Blows, Unarmed Damage, AC Bonus and Fast Movement; Warpriest's Sacred Weapon Damage; Brawler's Unarmed Damage). They are class features, not slots. Monk therefore still scrolls sideways at 390 px, with Level pinned; every other class's core columns fit. Moving them behind a second disclosure was rejected: a reader of Monk wants Unarmed Damage beside the level.
- **Fit.** A slash break after each iterative attack bonus (`<wbr>`, a typed node) lets Base Attack Bonus narrow. The cell padding, header type and Special width are in the review-round entries below, which supersede the first values (4 px padding, no font shrink, about 110 px).
- **Pinned Level.** `position: sticky; left: 0` on the first cell of every row, with an opaque background in both themes (the striped rows use `color-mix(surface-2 45%, surface)` over an opaque base, not a translucent tint). The header cell of the other columns is a tint so that the scroll cues show through it (75 % in round 1; round 3 lowered it to 60 %).
- **Scroll affordance.** The scroll-shadow technique on the wrap's background: a `--shadow-edge` gradient fixed to the scroll box and a solid `--surface` cover of the same size that moves with the content (`background-attachment: local`), which hides the shadow while the content has not moved from that edge. A table that does not overflow shows no cue. CSS only, no scroll-driven animation (Firefox lacks it). The round-3 entry below adds the pinned-edge cue the same way.
- **Spells per day** is a native `details.progression-spells` after the core table: closed by default, a 44 px summary, a chevron that turns, the slot table inside. The slot table is 13 px with 3 px cell padding, which fits all ten levels at 390 px for the checked pages (Wizard, Arcanist, Bard, Magus, Psychic, Eldritch Knight, Adept); a wider table still scrolls with Level pinned. Opening it is not remembered. A non-caster has no disclosure.
- **Checked pages** (light and dark, 390 and 1280 px): Wizard, Fighter, Arcanist (hybrid), Bard, Magus (6-level casters), Kineticist (no table columns for slots), Psychic (occult), Monk, Eldritch Knight (prestige, `Spells Per Day`), Adept (NPC).
- **Not done here.** The optional "tap a level for a card" of the issue; it needs a design decision of its own.


### Part C review round, 2026-11-15

- **Pinned edge (superseded by round 3, which makes it scroll-dependent).** A 2 px `--ink-muted` line plus an 8 px shadow, drawn with `::before` and `::after` on the sticky cell. A `box-shadow` on the cell did not paint in Chromium captures of this collapsed-border table, so the first round's edge was invisible. The strip of the previous column that shows between Level and the next grid line while scrolling stays; the edge now makes it read as a clipped column.
- **Type size.** Cell padding 3 px, header type 13 px, and the 13 px slot table apply below 30 rem only. From 30 rem up the slot table shares the core table's size and padding.
- **Special and BAB.** At 390 px Special is 96 px (it was about 85). Each part of an iterative attack bonus is an `.atk` span, shown one per line below 30 rem when there are three or more (`+16/`, `+11/`, `+6/`, `+1`; round 3 keeps two-part bonuses on one line), so the breaks are even and never inside a value. Rows with a long bonus are taller; Fighter's level 20 row is 87 px.
- **Labels.** A prestige class's single `Spells Per Day` column sits under the summary `Spellcasting`, so the disclosure does not repeat the column header. The region names are `Spells per day by level` and `Spellcasting by level`, so a screen reader does not hear the summary twice. The corpus caption "Class Features and Spells Per Day" is corpus text and stays.
- **Header matching.** A bare number is a spell level only for `0`–`9`; an ordinal (`1st`…) matches at any size. An unrecognised header in a progression stays in the core table, and a table that is not progression-shaped renders whole; both are unit tests.
- **Closed disclosure** keeps the accent-coloured chevron row already used by `.backlink-more`; no new summary style.

### Part C review round 3, 2026-11-15

- **The pinned edge shows only when the table scrolls.** Round 2 drew the 2 px edge and its shadow as pseudo-elements on the sticky cell, so they showed on every table, including those that fit. They are now two more layers of the wrap's background, with the same cover trick as the right-edge shadow: a 2 px `--ink-muted` line and a short shadow at `--pin-w` (3 rem, the width of the pinned column, set on the cell as well), fixed to the scroll box, under a solid cover that moves with the content. The line and the shadow are behind the cells, so the tinted cells (the striped rows at 45 %, the header at 60 %) soften them; the pinned cells are opaque and carry none. A table that fits shows the ordinary 1 px grid line and nothing else. `mobile.spec.ts` checks this by screenshot comparison: the wrap is captured as it is and again with a plain surface background, and the two must be identical for Wizard (core and slots, at 390 and 1280 px), Fighter, in light and dark; Monk must differ at both ends of its scroll.
- **Clearance.** The first column after Level has 8 px of left padding, so its text sits clear of the edge. The shadow is 6 px beyond the 2 px line.
- **Two-part bonuses stay on one line.** Only a bonus of three or more parts stacks one per line below 30 rem (`.atk-stack`); `+6/+1` is two `.atk` spans with a break hint, so it wraps only when the column is too narrow.
- **Cross-browser proof (round 4).** CI runs `mobile.spec.ts` only on mobile-chromium, so the scroll cues were proved by hand in WebKit and Firefox with a one-off script (not committed), using the same with/without-background comparison. WebKit ran the real app (Wizard core and slots, Monk at scroll 0, 90 and end, at 390 and 1280 px, light and dark): a fitting table differs by 0 pixels, Monk at 390 px by 11,898 to 38,203. Firefox cannot install the library in this local setup (the existing Firefox specs fail the same way, and Firefox is not in CI), so it ran the shipped `styles.css` on a static page with the same markup (a fitting table and a Monk-like table): 0 pixels for the fitting table, 10,747 to 36,815 for the scrolling one. Neither browser mispainted the `local` cover. The test compares with a small pixel allowance (50) instead of byte equality.
