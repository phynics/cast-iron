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
[decisions](docs/decisions.md): manual star transfer was removed, and stars stay
local to the browser profile. Existing install and update requests are
[#8](https://github.com/phynics/cast-iron/issues/8) and
[#9](https://github.com/phynics/cast-iron/issues/9).

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
