---
title: "Settings has seven sections, and the update panel says each thing once"
date: 2026-11-23
status: current
superseded_by: []
issues: [584]
prs: []
area: ui
---

# Settings has seven sections, and the update panel says each thing once, 2026-11-23

Refs [#584](https://github.com/phynics/cast-iron/issues/584) and [#594](https://github.com/phynics/cast-iron/issues/594). The page is described in `DESIGN.md` under Settings.

- **Updates no longer moves.** It is the first of seven sections in a fixed order (Updates, Appearance, Books, Library, Storage, Install, Help), with or without a waiting update. The old rule moved the section to the top while something waited, which shifted the page under the reader and needed a focus repair. The header action still opens Settings with `?updates` and focuses the section. This replaces the placement rule of [the daily update check](2026-11-06-the-daily-update-check-is-opt-out-and-same-origin.md) and the section grouping of [the corpus tag decision](2026-11-07-settings-names-the-corpus-tag-and-the-app-release-date.md); the opt-out, the same-origin check and the version formats of those entries stand.
- **Maintainer words leave the page.** `Corpus` becomes `Rules library`, `App` becomes `App version` under Help, and the `Content format` fact is only in Diagnostics (its report already carries `content format N`). No `schema`, `manifest` or content hash shows outside the version values.
- **The update row says the size once and the reassurance once.** The row is a facts row `Rules library` / `{tag} · {size}` and the installer panel's single button and note. In update mode the panel shows only the offer, progress and failures; the facts block and the iPhone paragraph belong to the first install.
- **The first-run installer says `Library version`** with the corpus tag (`fmt::corpus_label`), not the raw content hash.
- **Every test id stays.** `library-version`, `shell-version`, `app-install`, `verify-status`, `update-status` and `download-size` keep their meaning.
