# Contribute to Cast Iron

Use [GitHub Issues](https://github.com/phynics/cast-iron/issues) to track new
work. Search open and closed issues first. Create or reuse an issue with a
bounded problem, evidence, and acceptance criteria before implementation.
Small corrections can use one short issue. Record discoveries and scope changes
there instead of creating a second task list in docs.

1. Branch from current `origin/main`, for example `fix/12-offline-restart`.
2. Make small commits. Follow [AGENTS.md](AGENTS.md) and the
   [verification guide](.agents/skills/verify/SKILL.md).
3. Open a PR against `main`. Use `Closes #N` only when its acceptance is complete.
   Use `Refs #N` for partial work or checks that remain open.
4. Record local checks and GitHub CI separately. Merge the finished change into
   `main` through its PR. Main is the integration branch.
5. Publish a reviewed range with [explicit release promotion](docs/releasing.md).
   Merging a development PR does not publish Pages.

A closed implementation issue does not prove deployment. Record the release SHA,
Pages run, and live verification in the issue or promotion PR. Leave acceptance
issues open until the required device or corpus evidence exists.

Docs retain design decisions, corpus limits, and verification evidence. Issues
own outstanding work. [UI task history](docs/ui-tasks.md) retains earlier
acceptance context; do not restart its completed tasks. Star scope follows
[decisions](docs/decisions/README.md): manual star transfer was removed, and stars stay
local to the browser profile. Existing install and update requests are
[#8](https://github.com/phynics/cast-iron/issues/8) and
[#9](https://github.com/phynics/cast-iron/issues/9).

## Decisions and the status log

Add a file, not a section. A decision is `docs/decisions/YYYY-MM-DD-<slug>.md`; a corpus pin, a
release or an acceptance record is `docs/status/YYYY-MM-DD-<slug>.md`. Each starts with front matter
(the keys are in the README of its directory). No other file changes, so two pull requests that each add
one do not conflict: the READMEs hold instructions and no list of entries. `cargo xtask decisions-index`
and `cargo xtask status-index` render an index into `target/docs-index/` when you want one.
`cargo xtask check` fails on invalid front matter, a `superseded_by` that names no file, and a relative
inline Markdown link that does not resolve in a root `*.md` file or under `docs/` or `.agents/`.
`docs/status.md` is the one-screen current state; a pin or release PR edits only the lines that its
change makes untrue. Do not edit the text of an old decision; set its `status` and `superseded_by` when a
new one replaces it.

## Merge a pull request

Merge with `scripts/merge-pr.py`, which needs Python 3 and an authenticated GitHub CLI.
Pull requests run only the Rust checks, so the helper reads which CI the changed files
need, using the Test scope table in [AGENTS.md](AGENTS.md):

```sh
python3 scripts/merge-pr.py PR_NUMBER FULL_HEAD_SHA          # preview
python3 scripts/merge-pr.py PR_NUMBER FULL_HEAD_SHA --merge  # merge commit
```

It refuses a PR that is not open, does not target `main`, has a head other than the
given SHA, or is not `CLEAN`. When any changed file needs a full CI run (any file the
table does not name counts), it requires a successful `workflow_dispatch` run of
`ci.yml` on that exact SHA, with these jobs green: Rust checks, Content compiler,
Content determinism, Build the site, every Chromium shard, and Chromium report. Start
one with `gh workflow run ci.yml --ref BRANCH`. WebKit jobs are printed and do not block.
Otherwise the PR-event Rust checks on that SHA must pass. `--merge` runs
`gh pr merge --merge --match-head-commit`, so it never squashes or rebases, never deletes
the branch, and fails if the branch moved. Test it with `python3 scripts/test-merge-pr.py`.

The helper's rows still follow the Test scope table as it stood before the owner's rules of
2026-11-17, which ask for a branch run only for CI configuration, the toolchain, install, storage and
offline code, and a format version. Until the helper follows them, merge a reviewed pull request that
needs only the required `Rust checks` with `gh pr merge --merge --match-head-commit FULL_HEAD_SHA`,
then run one full CI on `main` after the batch.

## Review comments

Start each review comment with the name of the model that wrote it, for example
`Reviewed by Muse Spark 1.3:`. All comments come from one GitHub account, so the model name is the only
record that the reviewer was independent of the implementer.

## Editing rules content

Rules content lives in [phynics/cast-iron-data](https://github.com/phynics/cast-iron-data), not in
this repository. Fix or add entries there by following its `CONTRIBUTING.md`, then bump the pin in
`sources/corpus.toml` and run `cargo xtask fetch-sources` and `cargo xtask content`. The format is
documented in [docs/corpus-format.md](docs/corpus-format.md).

Footnotes are level-six headings in the entry body. A note needs a short marker, a unique
`footnote-` anchor, and a paragraph. Link the reference with a superscript:

```markdown
- [Homunculus](ref/monster/homunculus)^[5](ref/ability/improved-familiars#footnote-h)^

###### ^5^ {#footnote-h}

The master must first create the homunculus.
```

Keep the anchor. Body links name it, and `corpus-check` fails when the heading key is missing.
Do not use Markdown `[^5]` syntax; the reader rejects it. The note is an ordinary heading, so it
also appears in the page's "On this page" list when that list is shown. Place notes after the
passage and keep ordinary section headings for article structure. See
[Footnotes](docs/corpus-format.md#footnotes) for the full rule.
