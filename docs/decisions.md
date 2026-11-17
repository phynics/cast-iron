# Decisions

Decisions are one file each in [`docs/decisions/`](decisions/README.md). The
[README](decisions/README.md) holds the rules. It lists no entry, because a committed list would be a file
that every pull request that adds a decision edits. `cargo xtask decisions-index` renders the index (every
decision by area, with its date, status and issues) into `target/docs-index/decisions.md`.

To record a decision, add `docs/decisions/YYYY-MM-DD-<slug>.md` with the front matter that the README
describes. Do not edit another entry's text. When a decision
replaces an earlier one, set the earlier entry's `status` and `superseded_by` and leave its text alone.

This page stays so that older links and references (`docs/decisions.md, "Title"`) find the right
place. Look the title up in the index.
