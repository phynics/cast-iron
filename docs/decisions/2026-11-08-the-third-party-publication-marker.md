---
title: "The third-party publication marker"
date: 2026-11-08
status: current
superseded_by: []
issues: [189]
prs: []
area: content
---

# The third-party publication marker, 2026-11-08

Refs [#189](https://github.com/phynics/cast-iron/issues/189). App half only: the corpus marks the
books in a follow-up, so the pinned corpus has no flagged publication yet and the browser test
waits for that pin.

- **One optional corpus field.** `publications.toml` accepts `third_party = true`; absent means a
  Paizo book, and the canonical writer omits it when false. The field sits after
  `publication_date` and before `suffix` in the struct order that the reader, writer and
  formatter share.
- **The `corpus-check` rule is the contradiction only.** `third_party = true` beside a copyright
  line that names Paizo, in any case, is rejected. A missing marker is not rejected when the
  copyright omits Paizo: several Paizo books still have a pending Section 15 notice (#34), so
  absence is not evidence of a third-party publisher. The rule can be tightened once every
  notice is settled.
- **No schema bump.** `SourceInfo.third_party` is an optional additive boolean, serialized only
  when true. Schema 4 is unreleased, so the field costs no `SCHEMA_VERSION` change. The compiler
  copies it from `publications.toml` into `catalog.json`, and output stays deterministic.
- **One tier, last.** `Tier::ThirdParty` follows every Paizo tier in `Tier::ALL`, wins over the
  reviewed category, starts collapsed, and is a disclosure in the search Publication filter.
  `book_presets::is_third_party` now reads the flag, so only `Everything incl. 3rd party`
  includes a marked book; the other three presets, the first-run default and Undo follow from the
  same rule.
- **The search rank is the one exception to the tier order.** `Tier::Other` keeps the search
  crate's neutral rank, `u8::MAX`, so a third-party rank after it cannot be expressed. Third
  party uses 4, after `Adventures` and before neutral; the rank only breaks ties between equally
  relevant hits, and every group still lists `Third party` last. A page whose only results are in
  one collapsed tier opens that tier and spends the budget on it (#142), now for third-party
  books as well as adventures, so it never renders a page of closed headings.
- **Grouping follows the marker.** Books and Manage books build their groups from
  `BookListEntry`, which now carries the flag, so a marked book shows under `Third party` even
  when its reviewed category is a Paizo one. The arrival rule below leaves the group collapsed
  and empty until the reader turns a book on.
- **An arrival starts off.** The app stores `known_third_party`, the set of marked books the
  profile has seen. A marked book that is not in that set is added to `disabled_books` on load;
  the first library that carries the marker therefore turns every third-party book off, and a
  later update that adds one does the same. A book the reader turned on stays in the known set,
  so it stays on. The set only grows: a library that drops a marked book does not forget the
  choice, and a later re-add is not an arrival. This is bookkeeping, not a second copy of the
  choice: `disabled_books` remains the only stored selection. The settle runs inside
  `Library::load` before the `Library` handle exists, so `apply_active_plan`'s in-flight guard
  has no concurrent control write to serialise; the comment in `library.rs` records that and the
  two-tab limit.
- **The first marked library shows a notice once.** When the settle turns a marked book off on
  an existing profile (no remembered set, nothing seeded), it queues `third_party_notice =
  "pending"`. Home shows a dismissible notice in the standard `.notice` style, "Third-party books
  are now off by default. Turn them on in Manage books.", links to `#/books/manage`, and retires
  the flag to `"shown"` so a reload does not repeat it. A fresh install is seeded silently and
  never queues it. The browser test waits for a corpus with a marked publication.
