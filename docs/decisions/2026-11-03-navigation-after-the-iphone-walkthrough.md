---
title: "Navigation after the iPhone walkthrough"
date: 2026-11-03
status: partly-superseded
superseded_by: ["2026-11-03-planned-simplification-after-the-reference-walkthrough", "2026-11-06-search-is-a-header-icon-that-opens-quick-search"]
issues: [181]
prs: []
area: ui
---

# Navigation after the iPhone walkthrough

- Search is a control available from each reading screen, not a primary destination. The search results route remains for URLs, filters, history, and keyboard lookup. Its heading remains available to screen readers without consuming visual space above the bar. Header submission reads the input value at submit time to avoid stale query state. Superseded by the Quick search entry (#181).
- Home shows the six newest saved entries and recent reading. The existing bookmark and collection manager remains reachable from Home, preserving saved URLs and management actions. Books has a separate primary destination so reading a publication does not require going through category browsing.
- A `grapple` query on the published iPhone build initially surfaced equipment and feats. Quick search scopes expose Rules text directly. Rule body headings feed the existing search alias field, so a section such as Grapple can surface its containing Special Attacks entry. The index layout stays at version 1, and content stays at schema 3. This improves discovery without claiming complete prerequisite or character-build logic.
- App-shell and rules-library updates are separate lifecycles. Settings checks both; a waiting shell requires Reload, while a library replacement requires Download update. The shell-update acceptance test changes the service-worker version through a test-server hook, verifies it waits, then confirms activation preserves the active library and Home bookmarks offline. The hook is not shipped with the app.
- The iPhone report records the published five-tab build, not the replacement navigation. Device-driver failures left Home Screen installation, offline standalone relaunch, and the full character workflow unverified on the simulator. Automated local results are recorded separately.
