---
title: "The library is shipped, verified and stored as gzip files (schema 8)"
date: 2026-11-27
status: current
superseded_by: []
issues: [744, 746, 747]
prs: [748]
area: install-offline
---

# The library is shipped, verified and stored as gzip files (schema 8), 2026-11-27

Refs [#746](https://github.com/phynics/cast-iron/issues/746). The owner decided on 2026-11-27 that the
library is shipped and stored precompressed. This entry is the design and the implementation plan. The owner answered
its questions on 2026-11-27 (see "Owner answers"), so it is current.

## Summary

- The compiler gzips every library file except `manifest.json`: flate2 with its pure-Rust backend,
  level 6, a fixed header. The published file is `<name>.gz`.
- The manifest lists, for each file, the stored bytes (`path`, `sha256`, `bytes`: what is downloaded,
  verified and stored, as today) and, under a new optional `gzip` key, the decoded `sha256` and `bytes`.
- `SCHEMA_VERSION` becomes 8. `INDEX_FORMAT_VERSION` stays 2.
- The installer does not change: it downloads, verifies and stores the stored bytes, so resume and the
  progress line now count the real bytes (about 44.5 MB instead of 215 MB).
- The reader decodes in one place, the `read_file` of `library.rs`, with the browser's
  `DecompressionStream("gzip")` through one bridge function. A row without `gzip` is read as stored, so
  installed schema 1 to 7 libraries open as they are.
- `generated/content` holds both layers: the decoded files at their names, for every tool and test that
  reads them, and the `.gz` files. `dist/content` holds only the manifest and the `.gz` files.

## Measurements

All numbers are from the library of corpus v0.1.62 (content `vf68a2bc3fb8e1fbe`, 276 files, schema 7),
measured on 2026-11-27 in a scratch build on the shared host (4 cores, load average 9 to 16 during the
runs, so the times are upper bounds).

**Sizes.** flate2 1.1.11 with miniz_oxide (`rust_backend`), gzip header with mtime 0, no name, OS 255.

| Files | Count | Decoded | Level 6 | Level 9 |
|---|---|---|---|---|
| Entry shards | 248 | 124,275,220 | 21,253,105 (17.1%) | 21,118,048 |
| `search.bin` | 1 | 68,752,634 | 20,176,277 (29.3%) | 19,950,943 |
| Backlink shards | 16 | 5,527,496 | 1,180,104 (21.3%) | 1,168,511 |
| The rest (catalog, legal, coverage, indexes) | 11 | 16,397,827 | 1,937,212 (11.8%) | 1,924,885 |
| **Total** | **276** | **214,953,177** | **44,546,698 (20.7%)** | **44,162,387 (20.5%)** |

This confirms the v0.1.59 figures of the issue (215.3 MB to 44.7 MB). Level 9 saves 0.4 MB (0.9%) and
took 27 s against 15 s for level 6 in one run. Level 6 is the choice. A second compression of every file
gave the same SHA-256 at both levels.

**The manifest.** It is 39,965 bytes today and 14,014 bytes on the wire. With the `.gz` suffix and the
`gzip` object on every row it is about 68,300 bytes, about 26,300 on the wire: 12 KB more for each
update check.

**Decoding in a browser.** Playwright Chromium 156 and WebKit 27.2, headless, on the same host. Each value
is the best of three runs: read a file from the Cache API and copy it into a fresh buffer, as
`content_cache::read` does today.

| Read | Chromium | WebKit |
|---|---|---|
| `search.bin`, stored decoded (today) | 278 ms | 84 ms |
| `search.bin.gz` through `DecompressionStream` | 364 ms | 394 ms |
| `search.bin.gz` through miniz_oxide in WASM | 514 ms | 513 ms |
| Entry shard `spell-000.json` (521 KB), today | 4 ms | 4 ms |
| The same shard, `.gz` through `DecompressionStream` | 5 ms | 12 ms |
| The same shard, `.gz` through miniz_oxide in WASM | 5 ms | 10 ms |

So `search.bin` costs 0.1 to 0.3 s more, once per session. A shard costs a few milliseconds more. A phone
is slower; the iPhone check in the upgrade check below records the real value.

**WASM size of a Rust inflater.** A `cdylib` that copies a buffer is 12,475 bytes after `wasm-opt -Oz`.
The same crate with the streaming inflate of miniz_oxide 0.8.9 is 26,601 bytes: about 14 KB more.
flate2's `GzDecoder`, which also checks the CRC, adds about 50 KB. The shipped WASM is 2,687,119 of
2,720,000 bytes (#744), so 32,881 bytes are left. The `DecompressionStream` route adds one import, as
`sha256Hex` does: well under 1 KB (estimated, not measured).

## 1. Manifest and format

### The manifest row

```json
{"path": "vf68a2bc3fb8e1fbe/entries/spell-000.json.gz",
 "sha256": "<SHA-256 of the gzip bytes>", "bytes": 86614,
 "gzip": {"sha256": "<SHA-256 of the decoded bytes>", "bytes": 521284}}
```

```rust
pub struct ManifestFile {
    /// The URL under `content/` and the Cache API key. Unchanged meaning.
    pub path: String,
    /// SHA-256 and size of the bytes at `path`: what is downloaded, verified and stored.
    pub sha256: String,
    pub bytes: u64,
    /// Present when the stored bytes are one gzip member. Absent: the file is read as stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gzip: Option<Decoded>,
}
pub struct Decoded { pub sha256: String, pub bytes: u64 }
```

- `path`, `sha256` and `bytes` keep their meaning: the bytes at that URL. That is why the installer
  does not change. `total_bytes` stays the sum of `bytes`, so it is now the download size and the size
  on the device.
- One optional object, not two optional fields, so a row cannot have a decoded hash without a size.
- The name of a file to the reader (its *logical path*) is `path` without `.gz` when `gzip` is present,
  and `path` otherwise. The catalog's shard names, `REQUIRED_FILES`, `manifest_has_file` and the
  backlink shard paths all keep using logical paths, so none of them changes.
- `validate_manifest` adds three rules: a `gzip` row's path ends in `.gz` with a non-empty name before it,
  its decoded hash is valid, and no two rows share a logical path. `REQUIRED_FILES` is checked against
  logical paths.
- `manifest.json` itself stays plain JSON. An old shell must be able to read it to refuse it.
- The content version stays the first 16 hex digits of the SHA-256 over the sorted `path:sha256` lines,
  now of the stored files (`.gz` paths, gzip hashes). One version still means one set of stored bytes,
  and the cache name relies on that. A change of the compressor gives a new version, see section 2.
- The raw size is not shown and the progress line does not use it. The decoded size is in the manifest
  for the reader's length check, for `size-budget`, and for the tools that check `generated/content`. The
  decoded hash lets `build-site` and compile recovery check the decoded layer without compressing again,
  and keeps the alias hash stable (section 4).

### Which version moves

`SCHEMA_VERSION` becomes 8, and `READABLE_SCHEMAS` becomes 1 to 8. A schema-7 shell ignores the unknown
`gzip` key. Without the bump it would download the `.gz` files, find that their hashes match, activate the
library, and then fail to parse every file: a broken active library, which rule 7 forbids. A schema-7
shell today would also stop at `REQUIRED_FILES`, because no row is named `catalog.json` any more. But that
guard holds by coincidence, and its message ("does not list the required file") is wrong. With schema 8
the old shell refuses the manifest in `validate_manifest` before it downloads anything, with "Reload the
app to get the latest version", and the active library stays as it is.

`INDEX_FORMAT_VERSION` stays 2. The decoded `search.bin` is byte for byte the one of today. gzip wraps the
file on the wire and in storage; the search crate never sees it.

### Older installed libraries

A schema 1 to 7 manifest has no `gzip` rows. The reader reads such a row as stored, as today. There is no
branch on the schema number: the same function reads both kinds of row. The fixture of
`canonical-cutover.spec.ts` (a schema-3 library without gzip) keeps testing that path.

## 2. Determinism (rule 5)

- **Crate.** flate2 1.x with `default-features = false, features = ["rust_backend"]` (miniz_oxide), a
  dependency of `crates/content-compiler` only. Nothing compresses in WASM.
- **Settings.** `Compression::new(6)`. `GzBuilder::new().mtime(0).operating_system(255)`, with no
  file name, comment or extra field. The measured header is `1f 8b 08 00 00 00 00 00 00 ff`: deflate,
  no flags, mtime 0, XFL 0, OS 255 (unknown). One member per file. The trailer is the CRC-32 and size of
  the decoded bytes.
- **Versions.** `Cargo.lock` pins flate2 and miniz_oxide, so the bytes depend only on the input. An
  update of either crate can change the bytes and so the content version: users then download the same
  library again. A lock update that touches them says so in its pull request.
- **Proof.**
  - The compiler gzips each file once. It writes the decoded file and the `.gz` file into the staging
    directory and hashes both into the manifest. The determinism CI job (`diff -r` of two compiles of
    `generated/`) therefore compares the shipped bytes directly.
  - A unit test gzips a fixed input and compares the SHA-256 of the output with a committed value. A
    dependency update that changes the bytes fails that test.
  - `build-site` copies the `.gz` files and checks each one against the manifest (it does not compress).
- **Time.** Compression is single-threaded today and took 15 to 27 s for the whole library on the loaded
  host. The compiler compresses the files on `std::thread::scope` workers, with no new dependency, and
  `search.bin` (about a third of the bytes) bounds the wall time.

## 3. GitHub Pages

Checked on 2026-11-27 with `curl` against three GitHub Pages sites that serve a committed `.gz` file
(`ropensci.github.io`, `cloudyr.github.io`, `ghrr.github.io`, `/drat/src/contrib/PACKAGES.gz`), over
HTTP/2 and HTTP/1.1, with `Accept-Encoding: gzip, deflate, br`, `identity`, and none:

- `Content-Type: application/gzip`, no `Content-Encoding`, and a `Content-Length` equal to the file
  size, for every `Accept-Encoding`.
- The body is the committed file: its git blob hash equals the one in the repository
  (`e0eb3dec…` for ropensci), and it begins `1f 8b 08 00`. Pages does not gzip it again.
- For comparison, our own release serves `manifest.json`, `catalog.json` and `search.bin` with
  `Content-Encoding: gzip`: Pages compresses text and `application/octet-stream` on the wire, and the
  browser decodes it before `fetch` returns.

So a `fetch` of a `.gz` file returns exactly the stored bytes. `Accept-Encoding` is a forbidden request
header: script cannot set it, and it does not matter here, because Pages sends `.gz` the same way for
every value. The installer needs no special request.

**Residual risk.** Our site is deployed with `actions/deploy-pages`; the three sites deploy from a branch.
Both are served by the same Pages front end, but our own deployment has no `.gz` file yet. If a server
ever sent a `.gz` file with `Content-Encoding: gzip`, the browser would decode it, the hash check would
fail, and the installer would stop with an integrity error after its retries. Nothing is activated, so
the failure is safe (rule 7). The fallback is then a suffix that Pages serves as
`application/octet-stream`: Pages compresses it again on the wire and the browser removes only that outer
layer, so the stored bytes still arrive. That fallback changes the suffix rule in `crates/schema` and
needs a new schema version. The upgrade check confirms the headers on the live site before anyone relies
on them (owner answer 3: no separate probe first).

## 4. Decompression in the client

**Choice: `DecompressionStream("gzip")` through one bridge function.**

```js
// crates/web/js/bridge.js
export function gunzipSupported() { return typeof DecompressionStream === "function"; }
export async function cacheReadGunzip(cache, url) {
  const response = await cache.match(url);
  if (!response) return null;
  return new Response(response.body.pipeThrough(new DecompressionStream("gzip"))).arrayBuffer();
}
```

| | `DecompressionStream` (bridge) | miniz_oxide in WASM |
|---|---|---|
| WASM size | one import, under 1 KB | about 14 KB (43% of the 32,881 bytes left); 50 KB with flate2's CRC check |
| `search.bin` read (Chromium / WebKit) | 364 / 394 ms | 514 / 513 ms |
| WASM memory high-water mark for `search.bin` | 68.8 MB, as today | 88.9 MB: the 20 MB input is copied in, and WASM memory never shrinks |
| CRC-32 check | yes, native | only with flate2's decoder |
| Browsers | Safari 16.4+ (iOS 16.4, March 2023), Chrome 80+, Firefox 113+ | all |
| Cargo change | none | a new dependency (full CI) |

- The web-sys route is possible as well (features `DecompressionStream`, `CompressionFormat`,
  `ReadableStream`, `ReadableWritablePair`, and `Response::new_with_opt_readable_stream`). It needs the
  same browser and a `Cargo.toml` change, for more Rust than the six lines of JavaScript. Rule 8 allows
  the bridge for storage interop.
- **The choke point.** `library.rs` `read_file` is the one function that reads library bytes for the
  reader. Every `read_json`, the shard LRU, the backlink shards, `aliases.json` and `search.bin` go
  through it. It changes from a path to a manifest row: it finds the row by logical path (a missing row
  is `LibraryError::Missing`), then reads it. A row with `gzip` goes through `cacheReadGunzip`, and its
  length must equal `gzip.bytes`; a decode error or a wrong length is `LibraryError::Corrupt` with the
  path. A row without `gzip` reads as today. The installer, `verify_installed` and `check_installed` work
  on stored bytes and never decode.
- **Memory peak.** For `search.bin`: the stream's chunks of the 20 MB input, the decoded 68.8 MB
  `ArrayBuffer` in JavaScript, and the 68.8 MB copy into WASM. That is today's peak plus the chunks in
  flight. The JavaScript buffers are collected after the read. The WASM memory is the same as today.
- **Diagnostics.** `IndexLoad` counts the decoding in `read_ms` and records both the stored and the decoded
  size, so the Diagnostics page shows the real cost on a phone.
- **A browser without `DecompressionStream` (owner, 2026-11-27: add a Rust fallback).** `read_file` decodes
  through `cacheReadGunzip` when `gunzipSupported()` is true, and otherwise reads the stored bytes and
  inflates them in WASM with `miniz_oxide` (about 14 KB of WASM). The fallback checks the decoded length against `gzip.bytes` as a fast fail,
  then the SHA-256 of the decoded bytes against `gzip.sha256`, because a corrupt inflate of the right
  length would pass a length check alone. Whether the browser has `DecompressionStream` is probed once per
  session (a module-level `OnceCell`), not on every read: `read_file` is the per-shard hot path. So a browser older than Safari/iOS 16.4 can still install and read a schema 8 library,
  only slower (section 4 table) and with the higher WASM memory mark. The installer no longer refuses gzip
  rows for lack of `DecompressionStream`. The fallback's size comes out of the WASM budget, which has
  about 33 KB left (#744); slice 2 reports the budget.
- **The alias hash.** `Library::load` keeps the hash of `aliases.json` to decide whether the stored ids need
  the migration again. It uses the decoded hash when the row has one. The hash then stays the same across
  the 7 to 8 upgrade when the alias map does not change, and the migration does not run again for nothing.
  The migration is idempotent, so the other choice would also be safe.

## 5. The installer

Nothing in `install.rs` changes except the refusal above:

- It downloads `content/<path>` (the `.gz` URL) with `cache: no-store`, hashes the bytes it received,
  compares them with `sha256` and `bytes`, and stores them with `cache.put` under the same key. Pages
  sends them without `Content-Encoding`, so the received bytes are the stored bytes.
- **Resume.** The scan reads each cached file and hashes it against the stored hash, as today. A file
  that verifies is skipped. #747 (a paused update that survives a reload, with Resume) calls the same
  `install`, so it stays compatible: its "98 of 277 files" counts files, and its byte counts are now
  stored bytes.
- **Rule 7.** Unchanged: the manifest is stored last, then everything is hashed again, then
  `active_library` is written in one transaction, and only then are the other caches deleted. The old
  library stays active until the new one has been verified.
- **The progress line and sizes.** They count `bytes` of verified files against `total_bytes`, so they show
  the real download (about 44.5 MB). "Download size" (`download-size`) and "Size on this device"
  (`library-size`) both read `total_bytes`, so both become true without a code change. The space check
  asks for the stored bytes that are left. During a 7 to 8 update the device holds the old 215 MB library
  and the new 45 MB one, 260 MB at the peak instead of the 430 MB of a raw update.

## 6. The service worker and the shell

`web/sw.js` returns early for every `content/` path. It never caches or reads library files, and its
comment says so. No other part of the shell reads library files. The only readers are the installer
(stored bytes) and `library.rs` (`read_file`). The service worker does not change.

## 7. Tools and tests

**`generated/` holds both layers.** The staging directory, and so `generated/content`, holds
`<version>/<name>` (decoded) and `<version>/<name>.gz` (stored), with the manifest last. The atomic
publish of one directory stays as it is. This keeps decoded files where every native consumer reads them
today: `loadCorpus` and `loadBacklinks` (e2e), `entries_round_trip`, `published-ids`, `search-golden`,
the `canonical_ranking` and `canonical_correction` tests, and the search examples. The cost is 45 MB more
disk for each worktree and in the CI `generated` artifact. The other choice, compressing in `build-site`,
compresses twice and leaves the shipped bytes out of the determinism diff.

| Tool | Change |
|---|---|
| `crates/schema` | `Decoded`, `ManifestFile::gzip`, `logical_path()`, the validation rules, `SCHEMA_VERSION` 8 |
| `content-compiler` | gzip each file (section 2), manifest rows of section 1, both layers into staging. `is_whole` checks both layers, by hash, without compressing |
| `xtask` `build-site` | `verify_generated_content` checks both layers (stored hash and decoded hash). `copy_dir` is replaced for `content/` by a copy of `manifest.json` and the stored `.gz` files only, so `dist/content` ships no decoded file |
| `xtask` `size-budget` | Reads the manifest. `library_bytes` limits `total_bytes` plus `manifest.json`: what a device downloads and stores. `largest_file_bytes` limits the largest *decoded* file (`search.bin`), because the decoded size sets the memory peak of a read. `wasm_bytes` stays. New values: the measurement plus 5%. This changes the implementation, not only the values: today `size-budget` sums every file under `generated/content` (`crates/xtask/src/main.rs`, `size_budget`), which would count both layers; slice 3 makes it read the manifest instead |
| `scripts/check-publication.py` | Reads files through the manifest: it checks the stored hash, decodes `gzip` rows with Python's `gzip` module and checks the decoded hash, and reads the catalog, legal text and shards by logical path. Today it globs `entries/*.json`, which would find nothing in a `.gz`-only `dist` (it would then report an entry count mismatch, not pass) |
| `tests/e2e/server.mjs` | `.gz` served as `application/gzip` with no `Content-Encoding`, as Pages does |
| `tests/e2e` fixtures | A helper pair that reads a library file by logical path from `dist` (gunzip with `node:zlib` when the row has `gzip`) and writes one back (gzip, new stored and decoded hashes). Every helper in `tests/e2e/tests/helpers.ts` that reads or rewrites library bytes uses it: `buildUpdatedLibrary` (including its `legal.json`), the internal `buildLibraryWithEntryPatch` behind `buildLibraryWithFeatures` and `buildLibraryWithExpandedIn`, `buildLibraryWithoutBooks` and `buildLibraryWithoutBacklinks`. The overlay gzip does not have to equal the compiler's bytes, because the manifest carries the hashes. `buildNewerManifest` and the manifest-only overlays need no change. `canonical-cutover.spec.ts` keeps its uncompressed schema-3 fixture |

**Unchanged tools and the paperwork.**
- `published-ids` keeps working unchanged: it reads the decoded `catalog.json` in `generated/content`,
  which this design keeps.
- Slice 2's new `compressed-library.spec.ts` needs its row in the verify skill's feature map
  (`.agents/skills/verify/SKILL.md`), or `cargo xtask check` fails.
- Slice 3 runs the verify skill's "Corpus and schema changes" steps: `cargo xtask content` twice from an
  empty `generated/` with a tree compare, and `scripts/check-publication.py` on the built `dist/`.
- The release that ships slice 3 adds its `docs/status/` release entry with the upgrade check results, as
  every release does.

## 8. The upgrade check for the release

`docs/releasing.md` gets a schema 7 to 8 check, run before the promotion, in Chromium and WebKit, with the
previous release's `content/` kept outside `dist/` and `generated/` (verify skill, "Corpus and schema
changes"):

1. With the previous shell and its schema-7 library installed: star entries, change the active books,
   save a section address, open an entry.
2. Serve the new build. The old shell's update check (explicit and daily) refuses the schema-8 manifest
   with the reload message. The request log shows `manifest.json` only, and `active_library` stays the
   schema-7 version.
3. Apply the shell update. The new shell opens the installed, uncompressed schema-7 library before any
   library update: an entry, a search, the stars, an offline reload.
4. Start the library update and interrupt it (the `stall` hook, then close the page). On reopen the old
   library is active and readable. With #747 merged, Settings shows the paused update. Resume: the scan
   skips the cached `.gz` files (`resumed_files` above 0 in Diagnostics) and fetches only the rest.
5. After activation: the progress total and "Size on this device" read about 44.5 MB, the old cache is
   gone, and `navigator.storage.estimate()` drops by about 170 MB. Stars, active books, the saved
   address and Continue reading survive. "Verify library" passes. An offline reload works.
6. A corrupted `.gz` file (the `corrupt` hook) stops the install with an integrity error, and nothing is
   activated.
7. On the iPhone simulator (iOS 16.4 or later): install, then record the `search.bin` read time from
   Diagnostics.
8. After the deployment: `curl -sI` a `.gz` file of the live library. It must answer
   `application/gzip`, no `Content-Encoding`, and a length equal to the manifest's `bytes`. Its SHA-256
   must equal the manifest's `sha256`.

## Alternatives rejected

- **Compression at rest only** (download as today, compress on the device with `CompressionStream` before
  `cache.put`). The stored bytes would not be the published bytes, so nothing could verify them against
  the manifest: "Verify library" would have to decode and hash 215 MB. The installer would compress
  215 MB on a phone, which takes seconds of CPU. The progress line would still count decoded bytes,
  because `fetch` returns the body after the browser has decoded it.
- **Show the real size only** (keep the raw library, show the wire size). The device would still hold
  215 MB and need 430 MB during an update, which is the storage problem on iPhone. And the app cannot count
  wire bytes: `fetch` exposes only the decoded body.
- **A Rust inflater in WASM** (miniz_oxide). See section 4: 14 KB of the 33 KB left, 20 MB more WASM
  memory that never comes back, and slower in both browsers. It is the fallback if the owner needs
  browsers older than Safari 16.4.
- **gzip bytes under their own names** (`catalog.json` that holds gzip). Pages would label them
  `application/json` and compress them again on the wire. The bytes would arrive intact, but the names
  would lie to every tool that opens a `.json` file. This is only the fallback of section 3.
- **Brotli or zstd.** Smaller files, but `DecompressionStream` decodes only gzip and deflate in every
  target browser, so they would need an inflater in WASM.
- **One archive for the whole library.** It loses resume for each file and the read of one shard on demand.
- **Level 9.** 0.9% smaller, for up to twice the compression time in the measured runs.
- **`INDEX_FORMAT_VERSION` 3.** The index bytes do not change.

## Implementation plan

Each slice is one pull request against `main` that leaves `main` releasable. The reader must be able to
read gzip rows before the compiler writes them, so the compiler and the schema bump come last. Test scope
follows `AGENTS.md`, "Test scope".

1. **Schema rows and logical paths (no format change).** `Decoded`, `ManifestFile::gzip`,
   `logical_path()`, and the validation rules in `crates/schema`, with unit tests. In `library.rs`,
   `manifest_has_file`, the alias lookup and the alias hash use logical paths and the decoded hash.
   `READABLE_SCHEMAS` stays 1 to 7. Behaviour does not change for today's libraries.
   *Scope:* `cargo xtask check`; `cargo xtask content` (a `crates/schema` change); `cargo xtask build-site
   --public-url /cast-iron/`; one spec, `canonical-cutover.spec.ts`, `--project=chromium` (the feature map row for `library.rs` aliases). No
   full CI.
2. **The reader decodes.** The bridge functions `gunzipSupported` and `cacheReadGunzip`, and the
   `miniz_oxide` fallback for a browser without `DecompressionStream` (owner answer 1; a new dependency, so
   `Cargo.toml` and `Cargo.lock` change). `read_file` reads through the manifest row and checks the decoded
   length. The spec also runs the fallback path once, by hiding `DecompressionStream` in an init script,
   and the PR reports `cargo xtask size-budget`. The `IndexLoad` fields. `server.mjs` serves `.gz` as `application/gzip`. A new spec,
   `compressed-library.spec.ts`, with a feature map row in the verify skill: an overlay gzips every file
   of `dist` (still schema 7, which slice 1 allows to carry gzip rows), then the spec installs it, opens an
   entry, searches, reloads offline, and checks that a corrupted `.gz` stops the install while the old
   library stays active.
   *Scope:* the storage and offline row (`content_cache.rs`, `install.rs`) and the CI row
   (`server.mjs`): `cargo xtask check`, `build-site`, the new spec with `--project=chromium` and
   `--project=webkit`; a full CI run on the branch before merge.
3. **The compiler writes schema 8.** flate2 in `content-compiler` (section 2) with the golden unit test
   and threaded compression. Both layers in staging, `is_whole` on both, the content version over stored
   lines, `SCHEMA_VERSION` 8 and `READABLE_SCHEMAS` 1 to 8. `build-site` and `verify_generated_content`
   (section 7). `size-budget` measures and its new values, with the reasons in
   `sources/size-budget.toml`. `check-publication.py` and its test. The e2e fixture helpers. The "Size
   budget" text of `docs/releasing.md`. Before the PR, grep every spec for byte counts and size copy.
   *Scope:* `cargo xtask check`; `cargo xtask content` twice from an empty `generated/`, then compare
   the trees (`diff -r`); `build-site`; `cargo xtask size-budget`;
   `python3 scripts/test-check-publication.py` and `python3 scripts/check-publication.py`; one spec,
   `install.spec.ts`, with `--project=chromium` and `--project=webkit`. A full CI run on the branch
   before merge (a format version, `Cargo.toml` and `Cargo.lock`).
4. **The upgrade check.** Section 8 goes into `docs/releasing.md` as the schema 7 to 8 check. It is run
   before the promotion that ships slice 3, and the result is a `docs/status/` entry.
   *Scope:* docs only, `cargo xtask check`.

## Owner answers (2026-11-27)

1. **Browsers: add the Rust fallback.** `DecompressionStream` is used where the browser has it. Elsewhere
   `read_file` inflates in WASM with `miniz_oxide` (section 4), so older browsers can still update. The
   WASM cost (about 14 KB) is taken from the budget.
2. **The size shown: the stored size only.** "Download size" and "Size on this device" show the stored
   size (about 45 MB). The decoded size is not shown in Settings. Diagnostics may record it (`IndexLoad`).
3. **No live probe first.** The evidence of section 3 is accepted. Step 8 of the upgrade check confirms
   the live site at the release that ships slice 3. A wrong header fails safely: the hash check refuses
   the install, and the old library stays active.
