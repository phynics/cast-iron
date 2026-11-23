---
title: "v1 scope: owner decisions of 2026-11-23"
date: 2026-11-23
status: current
superseded_by: []
issues: [12, 134, 183, 254, 264, 423, 447, 461, 514, 540, 618, 637, 646, 647, 648, 650, 651]
prs: []
area: process
---

# v1 scope: owner decisions of 2026-11-23

The owner wants to close the open issues, declare v1, and then work on testing and UI/UX. On 2026-11-23
the owner decided the open questions that stood in the way.

**The rule.** An open issue with the `post-v1` milestone does not block v1. Every other open issue of
`phynics/cast-iron` and `phynics/cast-iron-data` is v1 work.

## In v1

- **Haunts are a new kind** ([#618](https://github.com/phynics/cast-iron/issues/618)): `EntryKind::Haunt`,
  with `SCHEMA_VERSION` 7, a full CI run before merge and the upgrade check of
  [releasing.md](../releasing.md). Rejected: `kind = "trap"` with a type value, because haunts must not
  show in the Traps list and every list, count and search scope would then need a special case.
- **Browse lists order same-name entries as search does**
  ([#637](https://github.com/phynics/cast-iron/issues/637)): product-line rank, newest printing, title, id
  ([search prefers the latest printing](2026-11-23-search-prefers-the-latest-printing.md)). Books
  sometimes disagree on a rule that they print under one name: every entry stays, and the latest one is
  elevated. Linking the printings to each other is post-v1 (below).
- **Archetype features follow the printed prose**
  ([#540](https://github.com/phynics/cast-iron/issues/540)). The owner dislikes the partitioning of class
  features into rows: the row layout of the class options is not extended to archetype pages as the
  rendered form. Structure stays at the data level, if it is kept at all, and corpus-format extensions help
  the app on printed prose. The survey of #540 proposes the extension, and the owner approves it before a
  rewrite. Whether the class-option layout changes the same way is not decided. This is the last large
  corpus rewrite, and the UI/UX work after v1 then runs on final data.
- **The print queues finish, with no cut line:** wrong book memberships
  ([#461](https://github.com/phynics/cast-iron/issues/461)) and missing citation pages
  ([cast-iron-data#215](https://github.com/phynics/cast-iron-data/issues/215)).
- **Update later on 2026-11-23:** the owner moved the 9 leftover creature key rows
  ([#254](https://github.com/phynics/cast-iron/issues/254)) to `post-v1`. They cite Adventure Path volumes
  (#33, #34, #48, #49, #70, #89 and #142) that are not available for a print check, so by the rule above
  they no longer block v1. They are tracked in phynics/cast-iron-data#166 and #215.
- **Device checks run on simulators.** An iOS Simulator and an Android Emulator, driven through the T3 Code
  device tools, replace the physical-device session for
  [#423](https://github.com/phynics/cast-iron/issues/423),
  [#514](https://github.com/phynics/cast-iron/issues/514) and the device part of
  [#12](https://github.com/phynics/cast-iron/issues/12). Results are recorded as simulator evidence: their
  timings, memory and storage eviction are not numbers of a physical iPhone. On 2026-11-23 the build host
  had no simulator; the iOS Simulator needs a macOS host with Xcode, and the Android Emulator needs the
  Android SDK.
- **Licensing:** see
  [two standing limitations accepted for v1](2026-11-23-licensing-limitations-accepted-for-v1.md).

## Post-v1

- **Format work:** compressed storage and the other library-size options
  ([#264](https://github.com/phynics/cast-iron/issues/264)), a smaller `lists.json`
  ([#183](https://github.com/phynics/cast-iron/issues/183)) and the later corpus format items
  ([#134](https://github.com/phynics/cast-iron/issues/134)). v1 ships with the library stored
  uncompressed.
- **The three last usability candidates of [#447](https://github.com/phynics/cast-iron/issues/447)**, which
  is closed: spell-list controls on phones ([#646](https://github.com/phynics/cast-iron/issues/646)), large
  feat chains ([#647](https://github.com/phynics/cast-iron/issues/647)) and effective-source clarity
  ([#648](https://github.com/phynics/cast-iron/issues/648)).
- **Entry pages link the other printings of a rule**
  ([#650](https://github.com/phynics/cast-iron/issues/650)), the linking part of the #637 intention.
- **The `Wide` spell row density is renamed `Expanded`**
  ([#651](https://github.com/phynics/cast-iron/issues/651)).
- **Leftover creature key rows** ([#254](https://github.com/phynics/cast-iron/issues/254)): the 9 rows stay
  until their prints are available (see the update under In v1).

## Status at the end of 2026-11-23

- [#637](https://github.com/phynics/cast-iron/issues/637) closed with #654. The feat-chain cap fix of
  [#647](https://github.com/phynics/cast-iron/issues/647) merged as #653; folding and the deep link are
  post-v1.
