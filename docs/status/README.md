# Status log

One pin, release or acceptance record is one file here, named `YYYY-MM-DD-<slug>.md`. [Current
state](../status.md) holds what is true now; this directory holds how it got that way.

This page has no list of entries, on purpose: a pin PR adds a file and edits no shared file. Browse the
directory, or render the index:

```sh
cargo xtask status-index    # writes target/docs-index/status.md (not committed)
```

## Add an entry

Add a file with front matter, then a `# ` heading. Do not edit an older entry.

```markdown
---
title: "Corpus pinned to v0.1.26"
date: 2026-11-18
kind: pin
---

# Corpus pinned to v0.1.26, 2026-11-18
```

`kind` is `pin`, `release` or `record`. `date` equals the date in the file name. Keep local verification,
CI and deployment results apart. `cargo xtask check` validates the front matter and the links.
