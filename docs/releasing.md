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
   the tag (see [Publish the PSRD library first, from the tag](#publish-the-psrd-library-first-from-the-tag)).
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

## Publish the PSRD library first, from the tag

The upstream import was removed from `main` after the library moved to the pinned
plain-text corpus. The git tag `psrd-last-buildable` (commit
`9800663202f4cb9bcd4fad1af43c5ebf884b4a09`) is the last commit that builds the PSRD
library from PSRD-Data and Pathfinder Data 1e. The first release step publishes the
alias-capable shell on the PSRD library from that tag (design section 6.5); the canonical
library follows from `main` in a second step. Nothing on `main` builds the PSRD library, so
never delete or move the tag.

The commands use two checkouts that sit next to each other. Replace the names if yours
differ, but keep both outside any other repository:

- `../cast-iron` is your checkout of this repository on `main` (the **main checkout**).
- `../cast-iron-psrd` is a new worktree of the tag (the **tag worktree**).

Run each command from the directory in its `cd` line. The tag worktree needs the same tools
as `main`: Rust, Trunk, Node, and Python 3.

1. **Build the canonical library on `main`.** The upgrade check below needs it, and the
   second release step publishes it.

   ```sh
   cd ../cast-iron
   git switch main && git pull --ff-only
   cargo xtask fetch-sources
   cargo xtask content
   ```

   This writes `../cast-iron/generated/content/`. The build fails if an id listed in
   `sources/published-ids.txt` no longer resolves.

2. **Build the PSRD library in a worktree of the tag.**

   ```sh
   cd ../cast-iron
   git fetch origin tag psrd-last-buildable
   git worktree add --detach ../cast-iron-psrd psrd-last-buildable
   cd ../cast-iron-psrd
   cargo xtask fetch-sources
   cargo xtask content --psrd
   ```

   `fetch-sources` at the tag downloads the pinned PSRD-Data and Pathfinder Data 1e archives
   and the corpus into `.cache/sources`. `content --psrd` writes the library
   `vcdd730994d863e50` (9,990 entries) to `generated/psrd/`.

3. **Check that the canonical library resolves every id the PSRD release published.** Run the
   tag's script, because `main` no longer ships it or its legacy-id snapshot, and name the
   main checkout's library by path:

   ```sh
   cd ../cast-iron-psrd
   python3 scripts/check-library-upgrade.py generated/psrd ../cast-iron/generated/content
   ```

   It must exit 0 and report `"missing_ids": 0`, `"unresolvable_ids": []`, `"alias_problems": 0`,
   and `"dangling_references": 0`. The aliases it resolves are the compiled form of
   `aliases.toml` in the corpus. For `main` at `9800663` it reports 9,968 old ids and 0 of each
   problem.

4. **Prepare the release commit from the tag.** The Pages workflow at the tag builds the
   canonical default. Make one commit that builds the PSRD library instead:

   ```sh
   cd ../cast-iron-psrd
   git switch -c release-prep-psrd
   ```

   In `.github/workflows/pages.yml`, change one line in the `Build the site` step:

   ```diff
   -            cargo xtask build-site --public-url /cast-iron/ && exit 0
   +            cargo xtask build-site --psrd --public-url /cast-iron/ && exit 0
   ```

   and update the comment above the step to say that `build-site` compiles the PSRD library
   (`--psrd`). Change nothing else. The dataset cache needs no new key: the step
   `Read the pinned dataset commits` keys `.cache/sources` on
   `psrd-data-<PSRD-Data commit>-pf-<Pathfinder Data 1e commit>`, and `--psrd` reads the same
   two archives that `fetch-sources` downloads (the corpus lands in the same directory and is
   verified by tree hash whatever the cache holds). Commit it:

   ```sh
   git commit -am "Build the Pages site from the PSRD library"
   git push origin release-prep-psrd
   ```

5. **Verify the prepared commit before it goes near `release`.** Pages only deploys from
   `release`, so dispatching `pages.yml` on the prep branch skips both jobs. Verify in the tag
   worktree and with the tag's own full CI instead:

   ```sh
   cd ../cast-iron-psrd
   cargo xtask build-site --psrd --public-url /cast-iron/
   python3 scripts/check-publication.py
   npm --prefix tests/e2e ci
   CAST_IRON_CONTENT=generated/psrd npm --prefix tests/e2e test -- reference search install legal \
     --project=chromium \
     --grep-invert "Publication filter|Publication name box|Equal relevance ranks|creature spell lists use readable"
   gh workflow run ci.yml --repo phynics/cast-iron --ref release-prep-psrd
   ```

   `check-publication.py` must exit 0 and the Chromium run must pass (30 tests). The full
   suite does not run against the PSRD library at the tag: `entry-header.spec.ts` needs an
   entry that several books publish, and the PSRD library has none, so Playwright stops at
   load. The four excluded names are the product-line and tier tests plus one
   creature-formatting test, which expect data the PSRD library lacks. Wait for the
   `ci.yml` run on `release-prep-psrd` to succeed, including its PSRD determinism steps. Record
   the exact prep commit SHA, the library version `vcdd730994d863e50`, and these results in
   the migration issue.

6. **Publish it.** `scripts/promote-release.py` accepts only a commit that is an ancestor of
   `origin/main` and has a successful `ci.yml` run on `main` for that exact SHA. The prep commit is
   neither, so the helper refuses it. The owner chose a one-time fast-forward push of the verified
   prep commit (decided 2026-11-06 in #127; see `docs/decisions.md`). The helper stays unchanged:

   ```sh
   git push origin PREP_SHA:refs/heads/release
   ```

   `release` must be an ancestor of the prep commit, so the push never resets or forces. The
   2026-11-06 release used this push once, for `ae032ba`. Any later PSRD-first release needs a new
   owner decision.

   The push triggers `pages.yml` from the prep commit. Then follow
   [Verify publication](#verify-publication) for the exact SHA, and check the installed shell
   and library as that section describes.

7. **Bring the release history into `main`.** The canonical promotion requires `origin/release`
   to be an ancestor of the promoted `main` commit, which the prep commit is not yet. Integrate
   it with a merge commit that keeps the tree of `main` unchanged, so the `--psrd` workflow edit
   never reaches `main`:

   ```sh
   cd ../cast-iron
   git fetch origin
   git switch -c integrate-psrd-release origin/main
   git merge --no-ff -s ours origin/release -m "Integrate the PSRD release history"
   git diff --stat origin/main HEAD
   git push -u origin integrate-psrd-release
   ```

   `git diff --stat origin/main HEAD` must print nothing. Open a PR to `main` and merge it with
   a merge commit (not squash or rebase). Then start a full CI run on the new `main` head
   (`gh workflow run ci.yml --ref main`) and promote that SHA with
   [Promote a tested commit](#promote-a-tested-commit).

8. **Clean up.**

   ```sh
   cd ../cast-iron
   git worktree remove ../cast-iron-psrd
   ```

   Keep the tag and the `release-prep-psrd` branch until the canonical library is live and
   verified.

## Published ids

`sources/published-ids.txt` lists every id that a published library has used: the 9,968 legacy
PSRD ids and the `ref/` ids of the canonical library. `cargo xtask content` fails when a listed
id is neither an entry nor an `aliases.toml` row whose target is an entry, so an edit in the
corpus cannot silently break a bookmark. A retired alias row does not count.

After a library is published and verified, record its ids:

```sh
cargo xtask content
cargo xtask published-ids
git add sources/published-ids.txt
```

The command adds the ids of `generated/content` (every entry id and every alias `old` id) to
the file and keeps it sorted. Commit the added lines with the release record. The file only
grows: never delete or edit a line. An id that must truly go away needs an `aliases.toml` row
that sends it to a live entry.

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
