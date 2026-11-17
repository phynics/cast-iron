---
title: "Plain-text corpus, step 1"
date: 2026-11-06
kind: record
---

# Plain-text corpus, step 1, 2026-11-06

`crates/corpus` writes and reads the format in `docs/corpus-format.md`.
`cargo xtask corpus-export` turns the compiled canonical library into text;
`corpus-check` validates it; `corpus-roundtrip` compares it with the compiled
library. The build is not switched, so this adds tooling and no library change.

Local verification on 2026-11-06 after the review changes (`v6aa587a686a3caf0`,
31,106 entries, 149,047 body links):

| Check | Result |
|---|---|
| `cargo xtask content` twice | Byte-identical: `v6aa587a686a3caf0`, content tree hash `2c6005d60af70d6c2c09d5106cd5303d434ab049310d6c13cc706586166b50f3` |
| Change against the previous library (`vd14efc1479965461`) | 4 entries: adjacent text nodes merged in 3, one explicit heading anchor (`sec-ghast` in the Ghoul entry); `coverage.json`, `books.json`, `relations.json` byte-identical |
| `cargo xtask corpus-export` twice | Byte-identical, tree hash `4564c25541e4f9d7facb7a1f9707a20ae15127dfd1d211aa84ba404f4ce176e9` |
| Export size and count | 31,113 files, 82.61 MB (sum of file bytes) |
| `cargo xtask corpus-fmt --check` | 0 of 31,113 files unformatted |
| `cargo xtask corpus-check` | 0 errors; 80 drift warnings (creature 1, feat 79) |
| `cargo xtask corpus-roundtrip` | 0 mismatches, exact comparison; coverage rebuilt from the corpus equals `coverage.json` |
| PSRD `--psrd` | Unchanged: `vcdd730994d863e50`, 9,990 entries |
| `check-publication.py generated/content` | Exit 0 |
| `check-library-upgrade.py generated/psrd generated/content` | Exit 0: 0 missing, 0 unresolvable, 0 dangling |
| `cargo xtask data-report` | Exit 0; tracked `reports/` files unchanged |
| `cargo xtask check` | Passed (format, Python tests, native and WASM clippy, workspace tests, WASM build) |

Tree hash: SHA-256 over the sorted `file-sha256  relative-path` lines.

Earlier status, 2026-11-05. The repository is public. The latest observed successful
[main CI](https://github.com/phynics/cast-iron/actions/runs/41691686691) and
[Pages deployment](https://github.com/phynics/cast-iron/actions/runs/41691686716)
are for `d48eaa3c8dce901b80a777600ae09238647c41cd`. The release migration has
not replaced that deployment or repeated its live browser acceptance.
Earlier published iPhone Simulator observations remain in
[the journey report](../research/iphone-journeys.md).
