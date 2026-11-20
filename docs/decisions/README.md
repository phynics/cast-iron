# Decisions

One decision is one file here, named `YYYY-MM-DD-<slug>.md`. `docs/spec.md` stays the product
specification; these entries refine it. `docs/decisions.md` is a pointer to this page.

This page has no list of entries, on purpose. A list in a committed file would be a file that every pull
request that adds a decision edits, and two such pull requests conflict. Browse the directory, search it
(`grep -l 'area: search' docs/decisions/2*.md`), or render the index:

```sh
cargo xtask decisions-index    # writes target/docs-index/decisions.md, grouped by area (not committed)
```

## Add a decision

Add a file. Do not edit another entry's text. Start it with front matter, then a `# ` heading:

```markdown
---
title: "Short name of the decision"
date: 2026-11-17
status: current
superseded_by: []
issues: [462]
prs: []
area: process
---

# Short name of the decision, 2026-11-17
```

- `title`: a JSON string. `date`: `YYYY-MM-DD`, equal to the date in the file name.
- `status`: `proposed`, `current`, `superseded` or `partly-superseded`. A `proposed` entry waits for the
  owner's approval; its pull request sets `current` when the owner approves. A decision that a later one
  replaced keeps its text; set `status` and list the file names (without `.md`) of the replacing entries in
  `superseded_by`. `superseded_by` is empty exactly when the status is `proposed` or `current`.
- `issues` and `prs`: ascending numbers of this repository, each once.
- `area`: `corpus`, `content`, `search`, `ui`, `install-offline`, `releases`, `process` or `licensing`.

`cargo xtask check` fails on invalid front matter, a `superseded_by` that names no file, a README that lists
an entry, and a relative Markdown link (file or heading) that does not resolve. The link check reads the
inline links `[text](target)` of the root `*.md` files and of every Markdown file under `docs/` and
`.agents/`; it does not read reference-style links, autolinks, HTML or other directories.
