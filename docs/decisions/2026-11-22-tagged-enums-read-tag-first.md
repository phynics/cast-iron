---
title: "The tagged enums are read tag first; the format and SCHEMA_VERSION stay"
date: 2026-11-22
status: current
superseded_by: []
issues: [426, 579, 584, 587, 592]
prs: [622]
area: content
---

# The tagged enums are read tag first; the format and SCHEMA_VERSION stay, 2026-11-22

Refines [the WASM size decision](2026-11-21-wasm-built-for-size.md), which left the internally tagged enums alone because a change of their serialized form needs a `SCHEMA_VERSION` bump. Follows [the cuts without a speed cost](2026-11-22-wasm-cuts-without-a-speed-cost.md). Refs [#426](https://github.com/phynics/cast-iron/issues/426). The room is for [#579](https://github.com/phynics/cast-iron/issues/579), [#584](https://github.com/phynics/cast-iron/issues/584), [#587](https://github.com/phynics/cast-iron/issues/587) and [#592](https://github.com/phynics/cast-iron/issues/592).

- **Where the cost was.** `Block`, `Inline` and `TypedFields` are internally tagged (`{"t":"heading",...}`, `{"kind":"spell",...}`). The derived `Deserialize` of such an enum accepts the tag at any position. To do that it reads the whole value into serde's private `Content` tree, finds the tag, and then reads the variant from the tree. Every type inside these enums (tables, stat rows, the nine field structs and their parts) was compiled twice: once for the JSON deserializer and once for the tree. The tree code and those second copies were 79 KB.
- **The choice: change the reader, not the format.** The compiler writes these enums with serde, and serde writes the tag first, always. So the reader can require that. `crates/schema/src/tagged.rs` implements `Deserialize` for the three enums by hand: it reads the first key, which must be the tag, reads the variant name, and gives the rest of the same JSON object to the variant. Nothing is buffered, and no type is compiled for a second deserializer. `Serialize` stays derived.
- **No `SCHEMA_VERSION` bump.** The serialized form is unchanged, so rule 4 does not ask for one. `cargo xtask content` writes a byte-identical library (the manifest has the same SHA-256 before and after), and the library size does not change. The owner's approval of 2026-11-22 covered a format change with a bump; this reaches the estimated saving without either.
- **Older libraries and installed libraries.** The client reads schemas 1 to 6 as before (`READABLE_SCHEMAS` is unchanged). Each of them was written by the compiler through serde, with the tag first, so each is read as before. An installed library keeps working across the shell update; nothing is downloaded or migrated, and rule 7 is not involved.
- **What is stricter.** A JSON object of one of these enums whose tag is not the first key is now an error (``the key `t` must come first``); the client reports the shard as corrupt, as it does for any shard it cannot read. No published library has such an object, and the hash check refuses a file that was edited after publication. A tool that writes library JSON without serde, such as a test fixture, must put `t` or `kind` first. The fixtures in `tests/e2e` do.
- **What is the same.** An unknown key is ignored, a key with a default may be left out, an unknown variant and a missing field are errors. The form with the enum as a JSON array, which the derived reader also accepted and nothing ever wrote, is no longer read.
- **Keeping the two in step.** Each struct variant of `Block` and `Inline` has a private body struct in `tagged.rs` with the same fields. Building the variant from the body does not compile when a field is missing on either side. The tests hold one value per variant behind an exhaustive `match`, so a new variant does not compile until it is added there, and they read back what `Serialize` wrote for every variant. They also pin the written form of schema 6 for a few values.
- **Rule for new code.** A new internally tagged enum in the schema gets its reader in `tagged.rs`, not `#[derive(Deserialize)]`. `#[serde(untagged)]` and `#[serde(flatten)]` use the same `Content` tree and are not used in `crates/schema` for the same reason.
- **Result.** `cargo xtask build-site --public-url /cast-iron/`, then `cargo xtask size-budget`, on the branch of the earlier cuts (`0bea8cfd1`): 2,604,221 to 2,525,278 bytes (-78,943, -3.0%). `wasm_bytes` stays 2,720,000; the build is at 92.8% of it.
