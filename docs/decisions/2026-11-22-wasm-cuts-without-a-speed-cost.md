---
title: "WASM cuts without a speed cost: collections built by insert, effects through one helper"
date: 2026-11-22
status: current
superseded_by: []
issues: [426, 579, 584, 587, 592]
prs: []
area: process
---

# WASM cuts without a speed cost: collections built by insert, effects through one helper, 2026-11-22

Refines [the WASM size decision](2026-11-21-wasm-built-for-size.md) and [the sort helper decision](2026-11-22-one-stable-sort-helper.md). Refs [#426](https://github.com/phynics/cast-iron/issues/426) (the size budget). The room is for [#579](https://github.com/phynics/cast-iron/issues/579), [#584](https://github.com/phynics/cast-iron/issues/584), [#587](https://github.com/phynics/cast-iron/issues/587) and [#592](https://github.com/phynics/cast-iron/issues/592).

- **Why.** The WASM was 2,687,865 of 2,720,000 bytes (98.8%) on `51dc43bdf`, and four UI tickets still need room. The owner's rule is to shrink the file, not to raise the limit.
- **Where the bytes are.** A names build (`wasm-opt -Oz -g` on the wasm-bindgen output) and `twiggy top`, on `f9968ce82` (2,679,334 bytes). By defining crate: `cast_iron_web` 958 KB, `tachys` 426 KB, `core` 276 KB, `reactive_graph` 169 KB, data 148 KB, `alloc` 146 KB, `serde_json` 100 KB, `cast_iron_schema` 98 KB, `serde` 55 KB, `leptos` 41 KB, `cast_iron_search` 32 KB. By kind of function, over all crates:

  | Group | Bytes | Functions |
  |---|---|---|
  | Reactive view closures (`Render::build` of a closure; the view code itself) | 415,665 | 141 |
  | `AnyView` glue: `into_any` build, rebuild, mount, unmount, per view type | about 285,000 | 243 types |
  | serde deserialization (all of it) | 263,719 | |
  | Drop glue | 116,566 | 2,336 |
  | BTree code (all of it, with the sorts below) | 118,020 | |
  | `tachys` `Render::build` and `rebuild` of static trees | 119,433 | 641 |
  | `core::slice::sort` still linked | 63,123 | 13 copies |
  | `RenderEffect`, `Effect` and `Memo` tasks, per closure | 86,643 | 144 |
  | Loader futures (`local_resource`) | 53,944 | 23 |

  `panic = "abort"`, fat LTO, one codegen unit and `wasm-opt -Oz` are already set. `cargo tree -d` for the WASM target lists only build-time duplicates (`syn`, `convert_case`) and `thiserror` 1 and 2, which add no shared code. No regex or Unicode table is large (Unicode code is 7 KB). Float formatting is 17 KB and stays: replacing the five `{:.1}` call sites with integer code saved nothing, because other code links the same formatter.
- **Cut 1: `BTreeMap` and `BTreeSet` values are built by insert.** `collect()` into a `BTreeMap` or `BTreeSet`, and `BTreeMap::from`, first sort the input with the generic `slice::sort_by`. That is why 13 copies of the std merge sort were still in the WASM after the sort helper removed every direct call. `cast_iron_sort::map_of` and `cast_iron_sort::set_of` call `extend`, which inserts one item at a time and needs no sort. A later pair replaces an earlier pair with the same key, as with `collect()`. 31 call sites in the client, the schema and the search engine changed. No function of `core::slice::sort::stable` is left in the WASM. **Rule:** non-test code in `crates/web`, `crates/schema` and `crates/search` builds these collections with `map_of`, `set_of`, `insert` or `extend`, not with `collect()` or `From`. Tests and the native-only crates keep `collect()`. To check a build: `twiggy top` on `target/wasm32-unknown-unknown/wasm-release/cast-iron-web.wasm` must list no `sort::stable` function.
- **Cut 2: unit effects go through `views::util::effect`.** `Effect::new` is generic over its closure, so each call site compiled its own copy of the effect's task. The helper boxes the closure; the 21 effects that return nothing share one copy. An effect that reads its previous value keeps `Effect::new` (8 remain).
- **Result.** Built with `cargo xtask build-site --public-url /cast-iron/`, measured with `cargo xtask size-budget`:

  | Build | WASM bytes | Change |
  |---|---|---|
  | `f9968ce82` | 2,679,334 | |
  | with cut 2 | 2,667,477 | -11,857 |
  | with cuts 1 and 2 | 2,595,691 | -71,786 |
  | main `51dc43bdf` (the Legal page merged) | 2,687,865 | |
  | `51dc43bdf` with cuts 1 and 2 | 2,604,221 | -83,644 (-3.1%) |

- **Behaviour and speed.** `cargo xtask content` writes a byte-identical library before and after (the manifest has the same SHA-256; the compiler uses the schema and search code). Rendering was measured in Chromium with the method of the WASM size decision (the time from setting the route to the last DOM change, 10 alternating runs, medians), main `51dc43bdf` against the same commit with both cuts. The page markup is identical on every path.

  | Path | Before | After |
  |---|---|---|
  | Wizard spell list, all levels (1,935 rows) | 118.2 ms | 111.9 ms |
  | Browse feats, first group | 73.2 ms | 70.6 ms |
  | Browse feats, group S | 87.6 ms | 86.4 ms |
  | Wizard class page | 72.7 ms | 72.6 ms |
  | Rule "Domains" | 114.4 ms | 115.0 ms |
  | Rule "Weapons" | 48.8 ms | 48.1 ms |
  | Search results for "fire" | 358.3 ms | 319.7 ms |
  | Books | 12.9 ms | 13.5 ms |
  | Quick search typing, key to second frame | 13.3 ms | 13.3 ms |

  The differences are inside run-to-run noise. Building a map by insert costs more comparisons than the bulk build on sorted input; no path shows it.
- **The budget line stays.** 2,604,221 bytes plus 5% is 2,734,432, above the current 2,720,000, so lowering `wasm_bytes` would take room from the four tickets. The line is unchanged and the build is at 95.7% of it, with 115,779 bytes free.
- **Measured and not done: Leptos `--cfg erase_components`.** One line in `.cargo/config.toml` (`[target.wasm32-unknown-unknown] rustflags = ["--cfg", "erase_components"]`) makes `tachys` store the children of every element as type-erased views. It needs no source change and the build passes. On `f9968ce82` the WASM went from 2,679,334 to 2,216,546 bytes (-462,788, -17.3%); on top of the two cuts above it is 2,134,071 bytes. The cost: every element child gets a marker comment node, and rendering slows down.

  | Path | Before | With the flag | With the flag, CPU slowed 4 times |
  |---|---|---|---|
  | Wizard spell list, all levels | 117.9 ms | 157.9 ms (+34%) | 325.8 → 456.8 ms (+40%) |
  | Browse feats, group S | 92.6 ms | 101.1 ms (+9%) | 246.6 → 276.5 ms (+12%) |
  | Wizard class page | 74.5 ms | 82.8 ms (+11%) | 195.5 → 215.5 ms (+10%) |
  | Rule "Domains" | 127.6 ms | 122.8 ms (-4%) | 323.9 → 356.9 ms (+10%) |
  | Books | 12.5 ms | 14.8 ms (+18%) | 24.1 → 29.2 ms (+21%) |

  The Wizard spell list holds 12,692 elements either way, and 1,295 comment nodes before against 17,790 with the flag. This trades speed on every page for size, so it is the owner's decision, and this entry does not take it. The flag cannot be applied to part of the app: it changes the element types inside `tachys`.
- **Not done, with reasons.** The serde cost is in the generated `Deserialize` code of the schema types and in the internally tagged enums; the saving needs a `SCHEMA_VERSION` bump (the earlier decision says the same). A view wrapper that replaces the DOM instead of patching it would drop most of the 127 KB of `AnyView` rebuild code, but it changes what a page keeps across an update (focus, open `<details>`, scroll inside a panel) and needs a per-view review. A boxed helper for `Memo::new` shares code only between memos of one value type; the estimate is under 10 KB.
