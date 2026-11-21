---
title: "The WASM is built for size: opt-level z, search kept at s"
date: 2026-11-21
status: current
superseded_by: []
issues: [170, 426]
prs: [565]
area: process
---

# The WASM is built for size: opt-level z, search kept at s, 2026-11-21

Refs [#170](https://github.com/phynics/cast-iron/issues/170) and [#426](https://github.com/phynics/cast-iron/issues/426) (the size budget, see [the size budget decision](2026-11-17-size-budget-and-compile-gates-before-the-write.md)).

- **Why.** The release WASM was 3,449,043 of 3,460,000 bytes (99.7%) on `27e3209`. The file had grown from 3,123,287 bytes on 2026-11-17 (corpus v0.1.23) in four days; the limit had already been raised once (3,280,000 → 3,460,000, `4b448e06d`) and the build again reached 99.7%. The next UI work (the archetype power layout of #170, and others) needs room. Raising the limit each time would make the budget a number to raise, not a guard.
- **Where the bytes were.** A names build (`wasm-opt -Oz -g`) and `twiggy`, grouped by the defining crate: `cast_iron_web` 27%, `core` 17%, `tachys` 16%, `reactive_graph` 14%, `alloc` 5%, data 4%, `serde_json` 4%, `serde` 3%, `cast_iron_schema` 2%, `cast_iron_search` 1%. Functions that name a `cast_iron_web` type, in any crate, are 56%. `core::slice::sort` alone is 315 KB (9%): 61 stable-sort instantiations for about 60 element types. No regex code is linked.
- **The choice.** The WASM was built with the release profile, `opt-level = "s"`, and Trunk then runs `wasm-opt -Oz`, which gains almost nothing on top of `"s"`. Building the WASM with `opt-level = "z"` is the largest single saving by far. Measured in scratch builds with the release pipeline, on a commit before `27e3209` (they land about 4 KB under the shipped file, so the final figures below differ slightly):

  | Build | WASM bytes | gzip -9 |
  |---|---|---|
  | `"s"` (before) | 3,444,628 | 1,299,332 |
  | `"z"` | 2,744,802 | 1,077,007 |
  | `"z"`, `cast-iron-search` at `"s"` | 2,773,221 | 1,092,878 |
  | `"z"` plus a non-generic sort helper, search at `"z"` | 2,594,931 | 1,042,936 |

  Other settings gave nothing worth their risk: `wasm-opt -Oz --converge` saved 4.8 KB, target features `+bulk-memory,+sign-ext,+nontrapping-fptoint` 5.4 KB, and `-O2` or `-O3` instead of `-Oz` made the file 96 to 144 KB larger. `strip`, `panic`, LTO and `codegen-units` were already as small as they go.
- **Search stays at `"s"`.** `[profile.wasm-release.package.cast-iron-search] opt-level = "s"` costs about 28 KB over `"z"`. Under fat LTO the override sets only the search crate's own compile; the final LTO pass over the whole module runs at `"z"`, and the 28 KB shows the override takes effect. The search crate is the one compute-heavy path in WASM. Its build alone, run under Node against the real `search.bin` (39,031 documents, 50 queries, the path the app uses), loaded in 573 to 786 ms at `"s"` and 694 to 743 ms at `"z"`, with a median query of 7.3 to 7.7 ms at `"s"` and 6.6 to 7.5 ms at `"z"`: no measurable difference, so the 28 KB buys safety, not speed we could see. That test built the search crate alone, outside the app's LTO, so it does not measure the shipped mix.
- **Rendering was checked in the browser.** Chromium, the same build pipeline, one build before and one after on the same commit, 10 alternating runs each, medians. Render is the time from setting the route to the last DOM change; scroll is 30 frames in one `requestAnimationFrame` loop; typing is the time from a key to the next two frames of the Quick search list.

  | Path | Before | After |
  |---|---|---|
  | Wizard spell list, all levels (1,935 rows), render | 117.9 ms | 116.4 ms |
  | Browse feats, first group (182 rows), render | 71.6 ms | 73.1 ms |
  | Browse feats, group S (543 rows), render | 91.6 ms | 95.4 ms |
  | Wizard class page, render | 77.3 ms | 79.1 ms |
  | Long rule "Domains", render | 124.1 ms | 123.9 ms |
  | Long rule "Weapons", render | 94.1 ms | 97.0 ms |
  | Quick search typing, median per key | 11.1 ms | 10.8 ms |
  | Worst scroll frame, all six pages | 16.9 to 21.1 ms | 16.9 to 21.0 ms |

  The largest slowdown is 4.1% (feats group S), inside run-to-run noise. A first run that waited on whole frames, with 5 runs, quantised every render time to 16.7 ms steps and showed up to +18%; it was replaced by the measurement above, which does not depend on frames.

  Cold start and install were not measured: the timings start from a running app. At startup the WASM parses the manifest, the alias index and the backlink index with `serde_json`, and that code now runs at `"z"`; the hash checks run in `crypto.subtle`, outside the WASM. The parse cost is expected to stay small next to the network and IndexedDB work, and a smaller file compiles and instantiates faster. The full CI suite exercises install and offline restart.
- **Only the WASM uses it: a named profile.** Setting `opt-level = "z"` in `[profile.release]` also builds the content compiler at `"z"`: compiling the pinned corpus took 200 s at `"s"` and 254 s at `"z"` (user time 168 s and 226 s), about 27% more on every CI run and release. The owner chose `"z"` for the WASM, not for the compiler, so `[profile.release]` is unchanged (`"s"`, LTO, one codegen unit, `panic = "abort"`) and a new `[profile.wasm-release]` inherits it with `opt-level = "z"` and the search override. Trunk uses it through `data-cargo-profile="wasm-release"` on the `rust` link in `web/index.html`. `cargo xtask build-site` runs `trunk build --release`, and so do the CI site job and `pages.yml` (both call `build-site`), so every WASM build takes the profile; the compiler (`cargo run --release`) and the other native tools stay on `release`. With the profile named, Trunk still runs wasm-bindgen and `wasm-opt -Oz` as before, writes the same `dist/*_bg.wasm` (one file, same name), and gives byte-identical output to the first `"z"` build (2,776,461 bytes). The compiler binary is byte-identical to the one built before this change, and the corpus compiled in 166 s (user 159 s). Nothing in the workflows or `xtask` names `target/release`; the Rust cache keys on the whole `target/` directory, so a second profile only adds a cache entry.
- **Plan.** The named profile alone takes the WASM from 3,449,043 to 2,776,461 bytes and does not touch `sources/size-budget.toml`. The next pull request replaces the 67 `.sort*` call sites with one non-generic merge-sort helper (measured: 150 to 190 KB). After it, the limit is reset to the measured size plus about 5%, so the budget is a guard again. No later change raises the limit without a reason (rule of `docs/releasing.md`, "Size budget").
- **Not done, with reasons.** Replacing the internally tagged serde enums would save an estimated 60 to 80 KB but changes the serialized format and needs a `SCHEMA_VERSION` bump. Nightly-only flags would save little (all `Debug` code is about 5 KB). Splitting views into a second WASM file needs a splitter that Trunk 0.22 does not have, or a second module that duplicates `std` and the Leptos runtime and complicates the service worker's precache.
