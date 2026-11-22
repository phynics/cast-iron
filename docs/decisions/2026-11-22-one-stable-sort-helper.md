---
title: "One non-generic stable sort for the workspace"
date: 2026-11-22
status: current
superseded_by: []
issues: [170, 426]
prs: []
area: process
---

# One non-generic stable sort for the workspace, 2026-11-22

Carries out the plan of [the WASM size decision](2026-11-21-wasm-built-for-size.md): the `.sort*` calls in the client, the schema and the search engine use one sort that the compiler does not copy per element type.

- **Why.** `slice::sort_by` and its relatives are generic. Each call with a new element type or closure made the compiler copy the whole merge sort: 61 copies, 315 KB of the WASM.
- **What.** The crate `cast-iron-sort` (`crates/sort`) has four functions, `sort`, `sort_by`, `sort_by_key` and `sort_by_cached_key`, that take `&mut [T]`. The sorting is one non-generic merge sort over a permutation of `usize` indices with a `&mut dyn FnMut(usize, usize) -> Ordering` comparator. The generic part is a few lines: it builds the comparator and applies the permutation with `slice::swap`. There is no `unsafe` (the workspace forbids it) and no dependency.
- **Where it lives.** `cast-iron-search` must not depend on `cast-iron-schema`, and `cast-iron-schema` must stay free of the search engine, so the helper is a third crate that both and the client depend on. The only cost is one more workspace member; Cargo, CI and the lockfile know it, no build step is added.
- **Stability.** All four functions are stable. A call that used `sort_unstable*` now keeps the input order of equal elements, which is one of the orders the unstable sort was allowed to give. A call that was stable keeps its order exactly. `cargo xtask content` writes byte-identical files before and after (the compiler uses the schema and search code).
- **Where it is not used.** Tests, examples, `cast-iron-corpus`, `cast-iron-content-compiler` and `xtask` keep `slice::sort*`: the WASM does not link them. A new call in the client, the schema or the search engine uses the helper. `select_nth_unstable_by` and `dedup` calls are not sorts and stay.
- **Result and budget.** The WASM went from 2,783,393 to 2,583,053 bytes (-200,340, -7.2%) on `9c216dca6`. `wasm_bytes` in `sources/size-budget.toml` was reset to 2,720,000 (the measurement plus 5%, rounded up to the next 10,000), as the earlier decision says: the budget shrinks with the file.
