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
