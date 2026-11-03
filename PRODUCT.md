# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Pathfinder First Edition players and GMs. The reference serves quick rules lookup during play and character-building research equally.

## Product Purpose

Find, browse, and understand the legally redistributable PF1e rules without a network connection after installation. `docs/spec.md` remains the product source of truth.

## Operating Context

Users need both category-based discovery and relationships between rules. A separate book view follows the original chapter and section order only, as confirmed on 2026-11-03. It is not a book-filtered alphabetical category list.

## Capabilities and Constraints

- Search and browse races and related racial traits; classes and archetypes; equipment; spells and spell lists; prestige classes; feats and chains; character traits; bestiary; and rules.
- Preserve source attribution, permanent entry IDs, bookmarks, and complete formatted rules text where redistribution is permitted.
- Offline-first static PWA, Rust/WASM and typed rich-content nodes, no backend, third-party runtime requests, remote fonts, telemetry, or raw HTML rendering.
- Research candidate AoNPRD 1e and d20PFSRD datasets before choosing new sources. Public availability or a code license does not establish rights to redistribute the data.
- The existing 16-book snapshot is incomplete relative to the intended corpus. Coverage gaps and exclusions must remain visible.

## Brand Commitments

Cast Iron is an unofficial reference. Do not use the Pathfinder logo or claim official status or compatibility. No visual rebrand was requested in this review.

## Evidence on Hand

`docs/spec.md`, `docs/status.md`, `docs/research/licensing-audit.md`, the pinned source snapshot, compiler reports, and the current Rust/Leptos UI. The current build has 9,112 entries from 16 available books; this is not evidence of complete PF1e coverage.

## Product Principles

- Fast lookup and deep research share the same canonical entries.
- Category browsing and original-order book reading are distinct navigation paths.
- Relationships carry meaning and provenance; unresolved relationships stay explicit.
- Offline operation and verified library activation are product behavior.
