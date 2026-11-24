# Archives of Nethys path coverage

Every canonical entry of the pinned corpus that has no verified Archives of Nethys path, with per-kind totals. This report is generated from the corpus alone and fetches no page.

## Pinned corpus

| | |
|---|---|
| Corpus | [`phynics/cast-iron-data`](https://github.com/phynics/cast-iron-data/tree/4aa07208a30471d3eefa41c4ce61538df30b47e7) |
| Tag | `v0.1.45` |
| Commit | `4aa07208a30471d3eefa41c4ce61538df30b47e7` |
| Tree hash | `769ac89f1480e0834037f31b14c088148e6d1e4de403db4f2ce69d8d8aa1e6da` |
| Entries | 31,672 |
| Publications | 443 |

## Totals by kind

`Matched` counts entries whose corpus file stores an `aon` path; `Missing` counts entries without one. `corpus-check` accepts only a relative `*Display.aspx` path on `aonprd.com`, so a stored path is the verified path the app renders.

| Kind | Matched | Missing | Total |
|---|---:|---:|---:|
| affliction | 0 | 462 | 462 |
| archetype | 1,246 | 53 | 1,299 |
| class | 49 | 0 | 49 |
| class_option | 30 | 2,789 | 2,819 |
| creature | 2,193 | 1,469 | 3,662 |
| feat | 3,442 | 215 | 3,657 |
| haunt | 0 | 45 | 45 |
| item | 439 | 7,658 | 8,097 |
| prestige_class | 115 | 0 | 115 |
| race | 74 | 2 | 76 |
| racial_trait | 0 | 342 | 342 |
| rule | 0 | 5,981 | 5,981 |
| skill | 0 | 44 | 44 |
| spell | 2,999 | 30 | 3,029 |
| trait | 1,896 | 39 | 1,935 |
| trap | 0 | 60 | 60 |
| **Total** | **12,483** | **19,189** | **31,672** |

## Publications

| | Count |
|---|---:|
| With a source page | 426 |
| Without a source page | 17 |
| Total | 443 |

### Publications without a source page

| Id | Title |
|---|---|
| `artifacts-and-legends` | Artifacts and Legends |
| `curse-of-the-crimson-throne` | Curse of the Crimson Throne |
| `gods-and-magic` | Gods and Magic |
| `hodgepodge-from-the-grand-lodge` | Hodgepodge from the Grand Lodge! |
| `magic-of-thassilon` | More Magic of Thassilon |
| `paizo-blog-golarion-day-subdomains` | Golarion Day: Other Gods and New Subdomains |
| `paizo-blog-the-gauntlet-will-be-ours` | The Gauntlet Will Be Ours! |
| `paizo-blog-ultimate-cantrips` | Ultimate Cantrips |
| `pathfinder-125-tower-of-the-drowned-dead` | Pathfinder #125 Tower of the Drowned Dead |
| `pathfinder-139-the-dead-road` | Pathfinder #139: The Dead Road |
| `pathfinder-society-s03-10-the-immortal-connundrum` | Pathfinder Society S03-10 The Immortal Connundrum |
| `plunder-and-peril` | Plunder and Peril |
| `quests-and-campaigns` | Quests and Campaigns |
| `rivers-into-darkness` | Rivers into Darkness |
| `skull-and-shackles-players-guide` | Skull and Shackles Player's Guide |
| `the-elephant-in-the-room-feat-taxes-in-pathfinder` | The Elephant in the Room: Feat Taxes in Pathfinder |
| `tome-of-horrors-complete-pf` | Tome of Horrors Complete (PF) |

## The per-entry list

[`aon-path-coverage.tsv`](aon-path-coverage.tsv) holds one row per missing entry, sorted by kind then id: `id`, `kind`, and `title`, with a header row. 19,189 data rows.

SHA-256 of the list: `8410207c4773d9c455dc77d6a04bd97c8c32bc2b999e5423056a6c0c6ec5729a`

## What a missing row means

A missing row is a canonical entry whose corpus file has no `aon` field. That is the whole fact the corpus records. The corpus stores only paths that a curation-time harvest verified against an AoN bulk listing; it stores no negative disposition. A missing row therefore does not say whether the harvest found no page (unmatched), found several pages (ambiguous), or never covered the kind. The report does not attach those causes to rows.

The harvest's own report is [cast-iron-data #67](https://github.com/phynics/cast-iron-data/pull/67). It separates matched, ambiguous and unmatched per kind, and it is the evidence for the cause at kind level. It ran on an earlier corpus revision, so its totals are not repeated here: every entry that the harvest left without a link and that is still in the corpus is in this list, and the per-kind counts above are the current corpus's. In that harvest, rules, afflictions, traps, skills and racial traits had no AoN bulk display-page listing, and item listings were category indexes, so those kinds stayed without a link rather than being guessed.

Paths are not guessed. No row here is a candidate, a near match or a base page; the app renders a link only when the corpus stores an `aon` path.

## Rerun

```sh
cargo xtask fetch-sources          # download and tree-hash-check the pinned corpus
cargo xtask aon-coverage           # rewrite this report and the list
cargo xtask aon-coverage --check   # fail when the committed files differ
```

The command reads `.cache/sources/cast-iron-data` only, sorts by kind then id, and writes no timestamp, so two runs on one corpus pin produce byte-identical files. Before writing, it checks that every entry appears once, that each kind's matched plus missing equals its total, that the list's ids are exactly the ids of the entries without a path, and that the list's row count and per-kind counts agree with the tables. Regenerate this report in the pull request that changes the corpus pin.
