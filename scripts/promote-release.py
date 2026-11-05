#!/usr/bin/env python3
"""Preview an explicit main commit for release; --push performs a fast-forward push."""
import argparse
import json
import re
import subprocess


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def ancestor(older, newer):
    subprocess.run(['git', 'merge-base', '--is-ancestor', older, newer], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('commit', help='full 40-character commit SHA from main')
    parser.add_argument('--push', action='store_true', help='publish the displayed range')
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.commit):
        parser.error('commit must be a full lowercase SHA, not a moving branch name')
    if run('git', 'remote', 'get-url', '--push', 'origin') not in (
        'git@github.com:phynics/cast-iron.git',
        'https://github.com/phynics/cast-iron.git',
    ):
        parser.error('origin must push to phynics/cast-iron')
    # Explicit refspecs also refresh tracking refs in clones with narrow fetch settings.
    run('git', 'fetch', 'origin',
        'refs/heads/main:refs/remotes/origin/main',
        'refs/heads/release:refs/remotes/origin/release')
    ancestor(args.commit, 'origin/main')
    ancestor('origin/release', args.commit)
    runs = json.loads(run('gh', 'run', 'list', '--repo', 'phynics/cast-iron',
                         '--workflow', 'ci.yml', '--branch', 'main',
                         '--commit', args.commit, '--limit', '100',
                         '--json', 'headSha,conclusion,url'))
    passed = [r for r in runs if r['headSha'] == args.commit and r['conclusion'] == 'success']
    if not passed:
        parser.error('no successful main CI run for this exact commit; start one with: gh workflow run ci.yml --ref main')
    print('CI:', passed[0]['url'], flush=True)
    print('Current release:', run('git', 'rev-parse', 'origin/release'), flush=True)
    print('Candidate:', args.commit, flush=True)
    print(run('git', 'log', '--oneline', f'origin/release..{args.commit}'), flush=True)
    print(run('git', 'diff', '--stat', 'origin/release', args.commit), flush=True)
    if args.push:
        # No force, reset, checkout, or branch creation. Git rejects non-fast-forwards.
        subprocess.run(['git', 'push', 'origin', f'{args.commit}:refs/heads/release'], check=True)
    else:
        print('Preview only. Review the range, then repeat with --push.')


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f'Promotion stopped: {error}') from error
