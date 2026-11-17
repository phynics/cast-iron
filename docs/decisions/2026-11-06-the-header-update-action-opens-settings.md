---
title: "The header update action opens Settings"
date: 2026-11-06
status: current
superseded_by: []
issues: [205]
prs: []
area: install-offline
---

# The header update action opens Settings, 2026-11-06

Refs [#205](https://github.com/phynics/cast-iron/issues/205).

- **The header never reloads by surprise.** The Update action used to call `applyShellUpdate()` from the one tap, which reloaded the app mid-lookup with nothing on screen saying so. It now opens Settings on its Updates section, where **Reload** applies the app update and **Download** offers the library. Update still outranks Install in the one header slot.
- **The label says what is ready, the accessible name says where it goes.** The visible label is "Update ready" (the icon and pip alone below 48 rem), and the accessible name is "App update ready. Open Settings". A button that names its destination is the rule from here on.
- **The destination is `#/settings?section=updates`.** Settings has no route per section, so the query names the part of the screen to land on. `Route::parse` still returns `Route::Settings`, so links, `aria-current`, Back and the route signal are unchanged; `route::wants_updates_section` reads the query. It is shareable and survives a reload, and the destination is a normal history entry.
- **Focus moves to the section.** The Updates section carries `id="set-updates"` and `tabindex="-1"`. After a route change the Reader's effect scrolls to the top and focuses `<main>` one animation frame later, so Settings waits two frames before it scrolls the section into view and focuses it. When Settings is already the current screen the route does not change and no rebuild happens, so the action scrolls and focuses directly.
- **Reload keeps its own row and its own behaviour.** `js::apply_shell_update()` is reached only from that button.
