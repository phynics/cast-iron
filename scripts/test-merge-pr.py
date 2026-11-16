#!/usr/bin/env python3
"""Exercise the merge helper's file classification and CI evaluation with canned data."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('merge_pr', Path(__file__).with_name('merge-pr.py'))
merge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(merge)
SHA = 'a' * 40
OTHER = 'b' * 40


def job(name, conclusion='success', status='completed'):
    return {'name': name, 'conclusion': conclusion, 'status': status}


def full_jobs(**overrides):
    names = ['Rust checks', 'Content compiler', 'Content determinism (NFR-05)',
             'Build the site', 'Chromium shard 1/3 (required)',
             'Chromium shard 2/3 (required)', 'Chromium shard 3/3 (required)',
             'Chromium report', 'WebKit shard 1/3 (informational)', 'WebKit report']
    return [job(n, **overrides.get(n, {})) for n in names]


def run(id_, event='workflow_dispatch', sha=SHA, status='completed', conclusion='success'):
    return {'databaseId': id_, 'headSha': sha, 'event': event, 'status': status,
            'conclusion': conclusion, 'url': f'https://example.test/runs/{id_}'}


class ClassifyTests(unittest.TestCase):
    def test_docs_only_needs_no_full_run(self):
        rows, full = merge.classify(['docs/decisions.md', 'AGENTS.md', 'DESIGN.md',
                                     '.agents/skills/verify/SKILL.md',
                                     'scripts/gmg-nested.py', 'CONTRIBUTING.md'])
        self.assertEqual(rows, ['docs only'])
        self.assertFalse(full)

    def test_merge_and_release_scripts_need_no_full_run(self):
        rows, full = merge.classify(['scripts/merge-pr.py', 'scripts/test-merge-pr.py',
                                     'scripts/promote-release.py', 'docs/releasing.md'])
        self.assertEqual(rows, ['release and merge scripts', 'docs only'])
        self.assertFalse(full)

    def test_publication_screen_needs_full_run(self):
        self.assertEqual(merge.classify(['scripts/check-publication.py']),
                         (['publication screen'], True))

    def test_ui_files_need_full_run(self):
        for path in ('crates/web/src/lib.rs', 'web/index.html', 'web/css/app.css',
                     'tests/e2e/tests/journeys.spec.ts'):
            with self.subTest(path=path):
                self.assertEqual(merge.classify([path]), (['UI'], True))

    def test_service_worker_row_wins_over_ui(self):
        for path in ('web/sw.js', 'crates/web/src/install.rs',
                     'crates/web/src/content_cache.rs', 'crates/web/src/store.rs'):
            with self.subTest(path=path):
                self.assertEqual(merge.classify([path]),
                                 (['service worker, installer, storage, offline'], True))

    def test_build_configuration_wins_over_ui_and_rust(self):
        for path in ('.github/workflows/ci.yml', 'Cargo.toml', 'Cargo.lock',
                     'crates/web/Cargo.toml', 'rust-toolchain.toml', 'Trunk.toml',
                     'tests/e2e/package.json', 'tests/e2e/package-lock.json',
                     'tests/e2e/playwright.config.ts', 'tests/e2e/server.mjs'):
            with self.subTest(path=path):
                self.assertEqual(merge.classify([path]), (['CI and build configuration'], True))

    def test_rust_and_corpus_pin(self):
        self.assertEqual(merge.classify(['crates/search/src/lib.rs']),
                         (['Rust outside crates/web'], True))
        self.assertEqual(merge.classify(['sources/corpus.toml']), (['corpus pin'], True))

    def test_unknown_file_needs_full_run(self):
        self.assertEqual(merge.classify(['sources/published-ids.txt']),
                         (['unclassified (no row names it)'], True))

    def test_one_full_file_makes_the_pr_full(self):
        rows, full = merge.classify(['docs/status.md', 'crates/web/src/app.rs'])
        self.assertEqual(rows, ['UI', 'docs only'])
        self.assertTrue(full)

    def test_nested_markdown_is_not_a_root_document(self):
        self.assertEqual(merge.classify(['crates/web/README.md'])[1], True)

    def test_agents_and_design_are_anchored_to_the_root(self):
        self.assertEqual(merge.classify(['AGENTS.md', 'DESIGN.md']), (['docs only'], False))
        for path in ('crates/foo/AGENTS.md', 'crates/web/DESIGN.md', 'web/AGENTS.md'):
            with self.subTest(path=path):
                self.assertTrue(merge.classify([path])[1])

    def test_no_files(self):
        self.assertEqual(merge.classify([]), ([], False))


class JobTests(unittest.TestCase):
    def test_all_green(self):
        problems, results = merge.evaluate_jobs(full_jobs(), merge.REQUIRED_JOBS)
        self.assertEqual(problems, [])
        self.assertEqual([len(results[f'Chromium shard {n}/3']) for n in (1, 2, 3)], [1, 1, 1])
        self.assertEqual(results['Content determinism'],
                         [('Content determinism (NFR-05)', 'success')])

    def test_failed_shard(self):
        jobs = full_jobs(**{'Chromium shard 2/3 (required)': {'conclusion': 'failure'}})
        problems, _ = merge.evaluate_jobs(jobs, merge.REQUIRED_JOBS)
        self.assertEqual(problems, ['job "Chromium shard 2/3 (required)" is failure, not success'])

    def test_skipped_and_running_jobs_fail(self):
        jobs = full_jobs(**{'Build the site': {'conclusion': 'skipped'},
                            'Chromium report': {'conclusion': None, 'status': 'in_progress'}})
        problems, _ = merge.evaluate_jobs(jobs, merge.REQUIRED_JOBS)
        self.assertEqual(len(problems), 2)
        self.assertIn('in_progress', problems[1])

    def test_missing_job(self):
        jobs = [j for j in full_jobs() if j['name'] != 'Build the site']
        problems, _ = merge.evaluate_jobs(jobs, merge.REQUIRED_JOBS)
        self.assertEqual(problems, ['job "Build the site" is missing from the run'])

    def test_each_chromium_shard_is_required(self):
        for missing in (1, 2, 3):
            with self.subTest(missing=missing):
                jobs = [j for j in full_jobs()
                        if not j['name'].startswith(f'Chromium shard {missing}/3')]
                problems, _ = merge.evaluate_jobs(jobs, merge.REQUIRED_JOBS)
                self.assertEqual(problems,
                                 [f'job "Chromium shard {missing}/3" is missing from the run'])

    def test_no_chromium_shard_at_all(self):
        jobs = [j for j in full_jobs() if not j['name'].startswith('Chromium shard')]
        problems, _ = merge.evaluate_jobs(jobs, merge.REQUIRED_JOBS)
        self.assertEqual(len(problems), 3)

    def test_run_with_two_of_three_shards_is_refused(self):
        jobs = [j for j in full_jobs() if not j['name'].startswith('Chromium shard 3/3')]
        picked, _, problems = merge.pick_run([run(1)], SHA, 'workflow_dispatch', lambda r: jobs)
        self.assertIsNone(picked)
        self.assertIn('Chromium shard 3/3', problems[0])

    def test_name_prefix_needs_a_word_boundary(self):
        jobs = [job('Content compiler extra'), job('Content compilerX')]
        problems, _ = merge.evaluate_jobs(jobs, ('Content compiler',))
        self.assertEqual(problems, [])
        problems, _ = merge.evaluate_jobs([job('Content compilerX')], ('Content compiler',))
        self.assertEqual(len(problems), 1)

    def test_webkit_is_informational(self):
        jobs = full_jobs(**{'WebKit shard 1/3 (informational)': {'conclusion': 'failure'}})
        problems, _ = merge.evaluate_jobs(jobs, merge.REQUIRED_JOBS)
        self.assertEqual(problems, [])
        self.assertIn(('WebKit shard 1/3 (informational)', 'failure'), merge.informational(jobs))


class PullRequestTests(unittest.TestCase):
    def pr(self, **overrides):
        base = {'baseRefName': 'main', 'state': 'OPEN', 'headRefOid': SHA,
                'mergeStateStatus': 'CLEAN', 'headRefName': 'topic'}
        base.update(overrides)
        return base

    def test_clean_open_pr(self):
        self.assertEqual(merge.check_pr(self.pr(), SHA), [])

    def test_each_refusal(self):
        for overrides, text in (
                ({'baseRefName': 'release'}, 'not main'),
                ({'state': 'MERGED'}, 'not OPEN'),
                ({'headRefOid': OTHER}, 'not the given'),
                ({'mergeStateStatus': 'BLOCKED'}, 'not CLEAN'),
                ({'mergeStateStatus': 'BEHIND'}, 'not CLEAN')):
            with self.subTest(overrides=overrides):
                reasons = merge.check_pr(self.pr(**overrides), SHA)
                self.assertEqual(len(reasons), 1)
                self.assertIn(text, reasons[0])


class RunTests(unittest.TestCase):
    def pick(self, runs, event='workflow_dispatch', jobs=None):
        return merge.pick_run(runs, SHA, event, lambda r: jobs if jobs is not None else full_jobs())

    def test_dispatch_run_accepted(self):
        picked, jobs, problems = self.pick([run(1)])
        self.assertEqual(picked['databaseId'], 1)
        self.assertEqual(problems, [])
        self.assertTrue(jobs)

    def test_pull_request_run_does_not_satisfy_a_full_requirement(self):
        picked, _, problems = self.pick([run(1, event='pull_request')])
        self.assertIsNone(picked)
        self.assertIn('no workflow_dispatch run', problems[0])

    def test_run_of_another_commit_is_ignored(self):
        picked, _, problems = self.pick([run(1, sha=OTHER)])
        self.assertIsNone(picked)
        self.assertEqual(len(problems), 1)

    def test_in_progress_and_failed_runs_refused(self):
        picked, _, problems = self.pick([run(2, status='in_progress', conclusion=''),
                                         run(1, conclusion='failure')])
        self.assertIsNone(picked)
        self.assertIn('in_progress', problems[0])
        self.assertIn('concluded failure', problems[1])

    def test_green_run_with_a_bad_job_is_refused(self):
        jobs = full_jobs(**{'Content determinism (NFR-05)': {'conclusion': 'skipped'}})
        picked, _, problems = self.pick([run(1)], jobs=jobs)
        self.assertIsNone(picked)
        self.assertIn('Content determinism', problems[0])

    def test_newest_qualifying_run_wins(self):
        picked, _, _ = self.pick([run(1), run(3, conclusion='failure'), run(2)])
        self.assertEqual(picked['databaseId'], 2)

    def test_pr_event_needs_only_rust_checks(self):
        picked, _, problems = self.pick([run(1, event='pull_request')], event='pull_request',
                                        jobs=[job('Rust checks')])
        self.assertEqual(picked['databaseId'], 1)
        self.assertEqual(problems, [])

    def test_pr_event_with_failed_rust_checks_refused(self):
        picked, _, problems = self.pick([run(1, event='pull_request')], event='pull_request',
                                        jobs=[job('Rust checks', conclusion='failure')])
        self.assertIsNone(picked)
        self.assertIn('Rust checks', problems[0])


if __name__ == '__main__':
    unittest.main()
