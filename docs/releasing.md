# Publish a release

`main` integrates development. `release` contains the selected publication
history. `.github/workflows/pages.yml` builds and publishes pushes to `release`.
Both jobs require `refs/heads/release`, including manual dispatch. CI still
checks pushes and pull requests on all branches.

Pages uses **GitHub Actions**, not branch-folder publishing. The `github-pages`
environment must allow only the branch `release`, with no matching tag rule.
The Pages API can retain `source.branch: main` as legacy metadata while
`build_type` is `workflow`. That field does not select Actions artifacts.
Keep `build_type: workflow`; changing to branch-folder publishing would publish
repository source rather than the Trunk distribution. See
[GitHub's Pages API](https://docs.github.com/en/rest/pages/pages#update-information-about-a-github-pages-site)
and [environment rules](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments).

## Promote a tested commit

Use Git, Python 3, and an authenticated GitHub CLI with Actions read access and
permission to push `release`. Run the commands from the repository root.

1. Choose a full commit SHA on `main` with a successful full **CI** run for
   that exact commit. The full run happens once a day and on demand; if the
   chosen commit has none, start one while `main` still points at it with
   `gh workflow run ci.yml --ref main`. Review all commits and changes since `release`, including changes
   from other issues. A green feature-branch run is not the main integration run.
   If the range includes the canonical default switch, confirm the alias-capable
   shell from the previous PSRD release is already live and verified. An old shell
   cannot resolve canonical bookmarks; publish and verify it before this promotion
   (design section 6.5). The PSRD library no longer builds on `main`; build it from
   the tag (see [Build the PSRD library from the tag](#build-the-psrd-library-from-the-tag)).
2. Preview the promotion from the repository root:

   ```sh
   python3 scripts/promote-release.py FULL_MAIN_SHA
   ```

   The helper fetches both branches, checks ancestry and main CI, and prints the
   range and diff summary. It does not change your checkout. Inspect the full
   diff with `git diff origin/release FULL_MAIN_SHA`.
3. Publish the reviewed range:

   ```sh
   python3 scripts/promote-release.py FULL_MAIN_SHA --push
   ```

   This uses a normal fast-forward push. It never creates, resets, or force-pushes
   `release`. A missing branch, divergent history, missing CI, or denied push
   stops promotion. Do not bypass these failures with a reset or force push.
4. Link the range, issues, and exact SHA in the migration issue or a release
   record. For a GitHub review page, open a promotion PR with base `release` and
   head `main`, inspect its exact range, then use the helper at the reviewed SHA.
   Publish with the helper rather than the promotion PR's Merge button. Merge,
   squash, and rebase operations can create release-only commits that prevent
   the next fast-forward promotion. If existing release history diverges from
   main, integrate that history into main through a merge-commit PR, rerun main
   CI, and review the combined range before retrying. Never rewrite release
   history or change repository rules just to use the helper.

There is no automatic promotion from `main`. To rebuild the current release
without moving it:

```sh
gh workflow run pages.yml --repo phynics/cast-iron --ref release
```

Dispatch on `main`, a tag, or another branch skips both jobs once the guarded
workflow is installed. The environment restriction also blocks the old main
workflow during migration. The Pages concurrency group remains `pages` with
cancellation disabled, so deployments are serialized. A queued older release
can finish first; verify the final successful run's SHA, not just any green run.

## Build the PSRD library from the tag

The upstream import was removed from `main` after the library moved to the pinned
plain-text corpus. The git tag `psrd-last-buildable` (commit
`9800663202f4cb9bcd4fad1af43c5ebf884b4a09`) is the last commit that builds the PSRD
library from PSRD-Data and Pathfinder Data 1e. The first release step builds from it:
the alias-capable shell on the PSRD library, followed by the canonical library.
Nothing on `main` builds the PSRD library, so do not delete or move the tag.

1. Check out the tag in a separate worktree. Use a path outside this repository:

   ```sh
   git fetch origin tag psrd-last-buildable
   git worktree add --detach WORKTREE_DIR psrd-last-buildable
   cd WORKTREE_DIR
   ```

2. Build the PSRD library and the site from the tag. These are the exact commands the
   tag's own README and `AGENTS.md` document:

   ```sh
   cargo xtask fetch-sources
   cargo xtask content --psrd
   cargo xtask build-site --psrd --public-url /cast-iron/
   ```

   `fetch-sources` downloads the pinned PSRD-Data and Pathfinder Data 1e archives and
   the corpus. `content --psrd` writes the library to `generated/psrd/`, and
   `build-site --psrd` packages it into `dist/`. Run the browser suite against it with
   `CAST_IRON_CONTENT=generated/psrd npm --prefix tests/e2e test -- ...`.

3. Check that the canonical library built on `main` resolves every id that the PSRD
   release published. Build the canonical library on `main` first
   (`cargo xtask fetch-sources && cargo xtask content`), then run the tag's check
   from the tag worktree. `main` no longer ships `scripts/check-library-upgrade.py`
   or its legacy-id snapshot; the tag does:

   ```sh
   python3 scripts/check-library-upgrade.py generated/psrd MAIN_CHECKOUT/generated/content
   ```

   It must exit 0 with no missing, unresolvable, or dangling ids. The aliases it
   resolves are the compiled form of `aliases.toml` in the corpus.

4. Publishing the PSRD build needs a release commit whose Pages workflow builds with
   `--psrd`. The workflow on `main` builds the corpus library only. Prepare that
   commit from the tag's tree, link it in the migration issue, and promote it with
   the steps above before the canonical promotion.

Remove the worktree after the release step (`git worktree remove WORKTREE_DIR`).

## Verify publication

1. Read the **Deploy to GitHub Pages** run and its deployment status. Confirm
   `headBranch: release`, the expected SHA, and a successful deploy job:

   ```sh
   gh run list --repo phynics/cast-iron --workflow pages.yml --branch release --limit 5
   gh api repos/phynics/cast-iron/pages
   gh api repos/phynics/cast-iron/environments/github-pages/deployment-branch-policies
   ```

2. Open `https://phynics.github.io/cast-iron/` in a fresh browser profile. Check
   asset loading under `/cast-iron/`, install the library, open a known entry,
   then go offline and reload. Check navigation and search, console errors,
   and third-party requests. Use the [verification guide](../.agents/skills/verify/SKILL.md)
   for affected acceptance tests.
3. Check an existing installed profile separately. In Settings, check for an
   app update and reload when offered. Library download/update is a separate
   action. Verify stars, active books, and offline reading survive both.
   For corpus/schema changes, retain the previous library outside build output
   and complete the upgrade checks before promotion.
4. Update `docs/status.md` with local, CI, deployment, and live results separately.
   Record exact app SHA and content version. A successful YAML check or deployment
   alone does not verify existing offline clients or physical devices.

## Recover a failed release

If build or deploy fails, the last successful Pages deployment remains the
baseline. Read the failed job output. For a transient failure, rerun that failed
run only if its SHA is still the intended release. Otherwise dispatch the current
`release` with the command above. Verify the resulting deployment SHA.

For a shipped regression, prefer a forward fix. To revert a change while keeping
history and the release guard:

1. Create a fix branch from current `origin/main` and link the regression issue.
2. Run `git revert BAD_COMMIT_SHA` for each identified change, newest first.
   For a merge commit, inspect its parents and use `git revert -m 1 MERGE_SHA`
   only when parent 1 is the intended baseline. Resolve conflicts and inspect
   the complete diff. Keep the release workflow, deployment policy, and format
   compatibility fixes. Do not restore the entire repository to an old tree.
3. Run `cargo xtask check` and the affected built-site/browser checks. For content
   changes, recompile before building and verify upgrade compatibility.
4. Open and merge a fix PR to `main`, wait for its exact main CI, then promote
   that SHA with the helper. Review the entire release range: other integrated
   changes may also be included. Verify fresh and existing profiles again.

Do not reset `release` backward or redeploy a historical main workflow as a
rollback shortcut. An older app may not understand a newer installed library,
search index, or saved-data format. Reverting bytes does not undo IndexedDB or
Cache API state, and offline devices may keep the previous shell until they
reconnect and accept Reload. Preserve readers for already-published formats or
ship a forward compatibility fix. Clearing site data deletes local stars and
libraries and is not a routine recovery step.

## Bootstrap and finish this migration

On 2026-11-03, `release` was created at the successfully published main commit
`d48eaa3c8dce901b80a777600ae09238647c41cd`. The existing environment branch rule
was changed from `main` to `release`; no reviewer or timer gate was added.
Pages remains in Actions mode. The live artifact was not replaced by that setup.

The guarded workflow is initially a PR against `main`. After it merges and main
CI succeeds, promote its full SHA using the steps above. The first release push
installs and runs the new workflow. Until then the bootstrap release has the old
workflow: pushes to it do not deploy, and dispatch on `release` can rebuild the
baseline. Main publication is already blocked by the environment rule. Record
first release CI/deployment and live checks in
[#10](https://github.com/phynics/cast-iron/issues/10) before calling rollout complete.
