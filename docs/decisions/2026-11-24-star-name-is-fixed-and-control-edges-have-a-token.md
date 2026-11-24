---
title: "The star keeps a fixed name, and control edges have their own token"
date: 2026-11-24
status: current
superseded_by: []
issues: [12]
prs: []
area: ui
---

# The star keeps a fixed name, and control edges have their own token, 2026-11-24

Refs [#12](https://github.com/phynics/cast-iron/issues/12). The owner decided both on 2026-11-24, on the accessibility audit (L4, L5).

- **Star name.** The star button's accessible name is `Star {title}` in every state. It no longer changes to `Unstar {title}`. `aria-pressed` states whether the entry is starred, so a screen reader hears one name and a toggle state, not a new name and the state together. Home's stars are pressed (`aria-pressed="true"`) and remove the star when pressed.
- **Control edge token.** `--control-edge` (light `#857a6d`, dark `#857a6e`) is at least 3:1 against `--bg`, `--surface`, `--surface-2` and `--stat-bg` in both themes. A control whose border is its visible edge uses it; `--line` stays for dividers and cards (1.44:1 light, 1.94:1 dark). `.btn` and text fields already used `--ink-muted`.
