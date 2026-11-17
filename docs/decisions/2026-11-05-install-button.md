---
title: "Install button"
date: 2026-11-05
status: current
superseded_by: []
issues: []
prs: []
area: install-offline
---

# Install button, 2026-11-05

- **The header shows Install only when installing is possible.** All three must hold: the app is not running as an installed app, the launch count is 5 or less, and either the browser gave us a `beforeinstallprompt` event or the browser is iOS Safari. A button that does nothing is worse than none. The rule is the pure function `pwa::show_header_install`, with native tests. "Installed" means a `standalone`, `fullscreen` or `minimal-ui` display mode, or `navigator.standalone === true`.
- **A launch is a browser session.** `localStorage` key `cast-iron.launches` holds the count. A `sessionStorage` key `cast-iron.launch-counted` makes the app count once per session, so reloads and route changes do not count. A new tab or a restarted browser does. If either store throws (private mode), the count is unknown and the header button stays hidden; the app does not fail. An unreadable stored value restarts the count at 1. The count is not tied to the library and is not synced anywhere.
- **The browser event is held by `bridge.js`.** Chromium fires `beforeinstallprompt` once and may do so before the WASM app starts. `js/bridge.js` loads first, keeps the event, and tells the app through a `cast-iron:install-change` window event. The event works once, so the bridge clears it when the prompt opens. After the user accepts or dismisses the prompt, Install disappears from the header and Settings falls back to the browser-menu hint, until the browser fires the event again. The page does not try to open the prompt a second time.
- **iOS Safari has no prompt, so the button opens guidance.** `pwa::is_ios_safari` matches iPhone, iPad and iPod user agents, plus a touch-screen Mac (iPadOS 13 and later reports a Mac), and excludes iOS browsers that carry their own marker (Chrome, Firefox, Edge, Opera, Google app). The guidance is a native `<dialog>` opened with `showModal()`: Share, then Add to Home Screen, then Add. Esc and Done close it, and focus returns to the control that opened it. The icons are self-drawn strokes.
- **Settings keeps the option for good.** Under App, Settings shows the same Install action in a browser regardless of the launch count, an "Installed" badge when running as an installed app, or a hint to use the browser menu when neither a prompt nor iOS guidance exists. It does not claim that installing protects stored data; the storage limits in the spec still apply.
- **One header slot.** `views::header_action` renders at most one quiet action in the header, selected from an enum. Install is the only member today. When an app update action joins, it takes priority over Install.
