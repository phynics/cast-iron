# Design

Cast Iron keeps one warm, paper-like identity: the palette, type and tokens in `web/styles.css` (`:root`). This file records the rules that later UI work must follow. Product context lives in `PRODUCT.md`.

## Chip roles

A small control can mean four different things. Each role has one form, so a reader can tell what a thing does from its shape. Do not invent a fifth form, and do not borrow one role's form for another.

| Role | Use it for | Form | Class | Never |
|---|---|---|---|---|
| Metadata | What the entry is: kind, book | One muted text line. Links are underlined text, separated by `·`. More than one extra book opens from a `+N more` disclosure. | `.entry-meta`, `.meta-link`, `.meta-more` | Pills, borders, a row of boxes |
| Filter | A choice that narrows a list: search kinds, books and facets; browse facets | The only pill: `border-radius: 999px`. A selected filter is filled with `--accent` and shows a check mark. The count stays. | `.chip` (checkbox in `search.rs`), `.lens-chip` (link), `.scope-choice` | A pill for anything else |
| Navigation | A jump to another place: recently viewed, shortcuts, spell-list links, search suggestions | A rectangle with the card radius (`--radius`, 6 px) or a plain link row | `.jump` in `.jump-list`, `.rel-chips li`, `.rel-list` | A pill, a filled selected state |
| Label | A short fact about the item next to it: "or", "repeats", "not in this library" | Small muted text with no border, no background and no pointer cursor. Use `.tag-warn` for a warning (error color, heavier weight). | `.tag`, `.tag-warn` | A border, a link, anything that looks clickable |

Rules that apply to every role:

- Interactive parts keep a 44 px target (`--touch`). A text line that stays compact reaches 44 px through a larger hit area (`.meta-link::after`), not through padding.
- Keyboard focus uses the global `:focus-visible` ring (3 px `--focus`). Do not remove it.
- Selected state never relies on color alone: filters add a check mark. Navigation has no selected state.
- Use `aria-pressed` for a toggle that keeps its place (the star), and `aria-current` for a link to the current page.

## Icon toggles

The star is an icon-only 44 by 44 button (`.star-btn`), in the title row on an entry and at the end of each starred row on Home. It keeps its accessible name (`Star {title}` or `Unstar {title}`) through `aria-label`. A starred entry shows a filled icon in `--accent`. Do not add visible text next to it.

## Tokens

Colors: `--bg`, `--surface`, `--surface-2`, `--ink`, `--ink-muted`, `--line`, `--accent`, `--accent-ink`, `--focus`, `--error`. Type: `--font-text`, `--font-display`, `--step--1` to `--step-3`. Space: `--space-1` to `--space-6`. Shape: `--radius` (6 px) for rectangles; 999 px only for filters. Touch size: `--touch`.

The stat block keeps its left accent rule; it is part of the identity.
