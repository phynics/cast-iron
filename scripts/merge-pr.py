#!/usr/bin/env python3
"""Preview or perform a merge commit of a PR, only with the CI its files need.

Without --merge this only previews. With --merge it runs
`gh pr merge --merge --match-head-commit`: never squash, never rebase, never
delete the branch.
"""
import argparse
import fnmatch
import json
import re
import subprocess

REPO = 'phynics/cast-iron'
ORIGINS = (
    'git@github.com:phynics/cast-iron.git',
    'https://github.com/phynics/cast-iron.git',
)

# The Test scope table of AGENTS.md. The first row that matches a file wins, so a
# specific row precedes the general row that would also match. Each row is
# (name, needs_full_ci, patterns). A pattern holding "/" matches the path from the
# repository root (fnmatch, where * also crosses "/"); a pattern without "/" matches
# the file name anywhere. A directory pattern ends in "/".
ROWS = (
    ('service worker, installer, storage, offline', True,
     ('web/sw.js', 'crates/web/src/install.rs', 'crates/web/src/content_cache.rs',
      'crates/web/src/store.rs')),
    ('CI and build configuration', True,
     ('.github/', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'Trunk.toml',
      'tests/e2e/package*.json', 'playwright.config.ts', 'server.mjs')),
    ('release and merge scripts', False,
     ('scripts/promote-release.py', 'scripts/test-promote-release.py',
      'scripts/merge-pr.py', 'scripts/test-merge-pr.py')),
    ('publication screen', True, ('scripts/check-publication.py',)),
    ('UI', True, ('crates/web/', 'web/', 'tests/e2e/')),
    ('corpus pin', True, ('sources/corpus.toml',)),
    ('Rust outside crates/web', True, ('crates/',)),
    # AGENTS.md lists docs/, AGENTS.md, DESIGN.md, .agents/skills/ and the research
    # scripts scripts/*-nested.py. Root Markdown files (README.md, CONTRIBUTING.md,
    # PRODUCT.md) are documents too.
    ('docs only', False,
     ('docs/', 'AGENTS.md', 'DESIGN.md', '.agents/skills/', 'scripts/*-nested.py', '/*.md')),
)
# A file that no row names is not known to be safe, so it needs the full run.
UNCLASSIFIED = 'unclassified (no row names it)'

# Job names of ci.yml. A name matches a job that equals it or starts with it and a
# space, so "Chromium shard" matches "Chromium shard 2/3 (required)".
REQUIRED_JOBS = (
    'Rust checks', 'Content compiler', 'Content determinism', 'Build the site',
    'Chromium shard', 'Chromium report',
)
PR_JOBS = ('Rust checks',)
INFORMATIONAL_PREFIX = 'WebKit'


def matches(path, pattern):
    if pattern.endswith('/'):
        return path.startswith(pattern)
    if pattern.startswith('/'):
        return '/' not in path and fnmatch.fnmatchcase(path, pattern[1:])
    if '/' in pattern:
        return fnmatch.fnmatchcase(path, pattern)
    return fnmatch.fnmatchcase(path.rsplit('/', 1)[-1], pattern)


def classify(paths):
    """Return (rows, needs_full): the names of the matched rows, in table order, and
    whether any file falls in a row that needs a full CI run."""
    found = {}
    for path in paths:
        for name, needs_full, patterns in ROWS:
            if any(matches(path, p) for p in patterns):
                found[name] = needs_full
                break
        else:
            found[UNCLASSIFIED] = True
    order = [name for name, _, _ in ROWS] + [UNCLASSIFIED]
    rows = [name for name in order if name in found]
    return rows, any(found.values())


def job_matches(name, wanted):
    return name == wanted or name.startswith(wanted + ' ')


def evaluate_jobs(jobs, wanted):
    """Return (problems, results). `results` maps each required name to the
    conclusions of its jobs; `problems` lists every missing or unsuccessful one."""
    problems = []
    results = {}
    for want in wanted:
        found = [j for j in jobs if job_matches(j.get('name', ''), want)]
        results[want] = [(j['name'], j.get('conclusion') or j.get('status') or 'unknown')
                         for j in found]
        if not found:
            problems.append(f'job "{want}" is missing from the run')
        for job in found:
            if job.get('conclusion') != 'success':
                state = job.get('conclusion') or job.get('status') or 'unknown'
                problems.append(f'job "{job["name"]}" is {state}, not success')
    return problems, results


def informational(jobs):
    return [(j['name'], j.get('conclusion') or j.get('status') or 'unknown')
            for j in jobs if j.get('name', '').startswith(INFORMATIONAL_PREFIX)]


def check_pr(pr, sha):
    """Return the reasons to refuse this PR before CI is looked at."""
    reasons = []
    if pr.get('baseRefName') != 'main':
        reasons.append(f'base is {pr.get("baseRefName")!r}, not main')
    if pr.get('state') != 'OPEN':
        reasons.append(f'PR is {pr.get("state")}, not OPEN')
    if pr.get('headRefOid') != sha:
        reasons.append(f'head is {pr.get("headRefOid")}, not the given {sha}; '
                       'the branch moved or the SHA is wrong')
    if pr.get('mergeStateStatus') != 'CLEAN':
        reasons.append(f'mergeStateStatus is {pr.get("mergeStateStatus")}, not CLEAN')
    return reasons


def pick_run(runs, sha, event, jobs_of):
    """Newest completed successful `event` run of `sha` whose required jobs passed.
    Return (run, jobs, problems). `run` is None when no run qualifies; `problems`
    then says why for each run that was considered."""
    wanted = REQUIRED_JOBS if event == 'workflow_dispatch' else PR_JOBS
    candidates = sorted((r for r in runs if r.get('headSha') == sha and r.get('event') == event),
                        key=lambda r: r['databaseId'], reverse=True)
    problems = []
    for run in candidates:
        if run.get('status') != 'completed':
            problems.append(f'{run["url"]}: {run.get("status")}, not completed')
        elif run.get('conclusion') != 'success':
            problems.append(f'{run["url"]}: run concluded {run.get("conclusion")}')
        else:
            jobs = jobs_of(run)
            bad, _ = evaluate_jobs(jobs, wanted)
            if not bad:
                return run, jobs, []
            problems.extend(f'{run["url"]}: {p}' for p in bad)
    if not candidates:
        problems.append(f'no {event} run of ci.yml found for {sha}')
    return None, [], problems


def gh(*args):
    return subprocess.check_output(('gh',) + args, text=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('pr', type=int, help='pull request number')
    parser.add_argument('sha', help='full 40-character head SHA of the PR')
    parser.add_argument('--merge', action='store_true',
                        help='merge with a merge commit after every check passes')
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.sha):
        parser.error('sha must be a full lowercase 40-character SHA')
    origin = subprocess.check_output(['git', 'remote', 'get-url', '--push', 'origin'],
                                     text=True).strip()
    if origin not in ORIGINS:
        parser.error(f'origin must push to {REPO}')

    pr = json.loads(gh('pr', 'view', str(args.pr), '--repo', REPO, '--json',
                       'headRefOid,headRefName,mergeStateStatus,baseRefName,state'))
    # The files come from the REST API: it pages past 100 files and names the old
    # path of a rename, which must count as changed too.
    listing = [json.loads(line) for line in
               gh('api', '--paginate', f'repos/{REPO}/pulls/{args.pr}/files',
                  '--jq', '.[] | tojson').splitlines() if line]
    paths = sorted({p for f in listing for p in (f['filename'], f.get('previous_filename'))
                    if p})
    print(f'PR #{args.pr} {pr["headRefName"]} at {pr["headRefOid"]}: {len(paths)} files', flush=True)
    refusals = check_pr(pr, args.sha)
    if refusals:
        raise SystemExit('Refused: ' + '; '.join(refusals))

    rows, needs_full = classify(paths)
    print('Rows matched:', '; '.join(rows) or '(no files)', flush=True)
    event = 'workflow_dispatch' if needs_full else 'pull_request'
    print('CI required:', 'full run (workflow_dispatch of ci.yml) on this head' if needs_full
          else 'PR-event "Rust checks" on this head', flush=True)

    runs = json.loads(gh('run', 'list', '--repo', REPO, '--workflow', 'ci.yml',
                         '--commit', args.sha, '--limit', '100',
                         '--json', 'databaseId,headSha,event,status,conclusion,url'))

    def jobs_of(run):
        return json.loads(gh('run', 'view', str(run['databaseId']), '--repo', REPO,
                             '--json', 'jobs'))['jobs']

    run, jobs, problems = pick_run(runs, args.sha, event, jobs_of)
    if run is None:
        hint = ('; start one with: gh workflow run ci.yml --ref ' + pr['headRefName']
                if needs_full else '')
        raise SystemExit('Refused: ' + '; '.join(problems) + hint)
    print('Run:', run['url'], flush=True)
    wanted = REQUIRED_JOBS if needs_full else PR_JOBS
    _, results = evaluate_jobs(jobs, wanted)
    for want in wanted:
        for name, state in results[want]:
            print(f'  {name}: {state}', flush=True)
    for name, state in informational(jobs):
        print(f'  {name}: {state} (informational)', flush=True)

    if not args.merge:
        print('Mergeable. Preview only; repeat with --merge to merge.', flush=True)
        return
    # --merge only: no squash, no rebase, and no --delete-branch. The head match
    # makes GitHub refuse the merge if the branch moved after this check.
    subprocess.run(['gh', 'pr', 'merge', str(args.pr), '--repo', REPO, '--merge',
                    '--match-head-commit', args.sha], check=True)


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f'Merge helper stopped: {error}') from error
