---
title: "The pinned corpus is the source of truth for the canonical library"
date: 2026-11-06
status: partly-superseded
superseded_by: ["2026-11-06-the-upstream-import-is-removed"]
issues: [18, 127]
prs: []
area: corpus
---

# The pinned corpus is the source of truth for the canonical library, 2026-11-06

Refs [#127](https://github.com/phynics/cast-iron/issues/127), [#18](https://github.com/phynics/cast-iron/issues/18). Step 3 of the plan in #127. The format is in `docs/corpus-format.md`.

- **`cargo xtask content` compiles from the corpus.** It reads `phynics/cast-iron-data` through `crates/corpus` (`cast-iron-compile --corpus <dir>`), then runs the stages that derive what the corpus does not store: relations, spell lists, coverage (from the entries, `exclusions.toml`, and `history.toml`), search aliases and facets, backlinks, catalog, search index, and manifest. It reads no upstream dataset and does not need `.cache/sources/psrd-data` or the Pathfinder Data cache.
- **The upstream path stays for one more step.** `cargo xtask content --from-upstream` runs the previous canonical adapter. CI and reviewers use it to prove that the corpus build gives the same bytes. The PSRD build (`--psrd`) is unchanged. A later step removes both upstream paths.
- **The pin is a commit and a tree hash.** `sources/corpus.toml` names the repository, the tag (`v0.1.2`), the commit, and `tree_sha256`: the SHA-256 of one line per regular file, `<sha256 of the file>  <path>\n`, in byte order of the path. `cargo xtask fetch-sources` downloads the archive of the commit, extracts it into a staging directory, and replaces `.cache/sources/cast-iron-data` only when the tree hash matches. `cargo xtask content` re-checks the cached tree before it compiles, so a hand-edited cache fails with the expected and actual hash.
- **Why not the archive hash.** GitHub does not promise that a generated archive stays byte-identical, and the gzip bytes already differ between the tag URL and the commit URL of the same tree. The tree hash covers the files, which is what the build reads. A symbolic link or other special file in the archive is an error.
- **To bump the pin:** tag the new corpus in `cast-iron-data`, set `tag` and `commit` in `sources/corpus.toml`, extract the archive of that commit, set `tree_sha256` to `cargo xtask corpus-tree-hash <extracted dir>`, then run `cargo xtask fetch-sources` and `cargo xtask content`.
- **Provenance is optional.** `validate_library` no longer requires a `provenance` row, because a new hand-written entry has none. The serialized shape does not change: `provenance` stays a required array in `RuleEntry`, and an entry without provenance writes `[]`. A client that reads the old library reads the new one, so `SCHEMA_VERSION` stays. The entry page hides the Provenance section when the list is empty.
- **Reports differ, content does not.** The corpus build has no upstream survey, deduplication counts, or link-pass report, so `generated/reports/validation.json` and `coverage.md` carry a single corpus note instead, and no `link-pass.json` is written. Everything under `generated/content/` is byte-identical to the upstream build.
