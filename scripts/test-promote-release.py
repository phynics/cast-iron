#!/usr/bin/env python3
"""Exercise promotion failures without writing remote refs."""
from contextlib import redirect_stderr
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('promote_release', Path(__file__).with_name('promote-release.py'))
promote = importlib.util.module_from_spec(spec)
spec.loader.exec_module(promote)
SHA = 'd48eaa3c8dce901b80a777600ae09238647c41cd'


class PromotionTests(unittest.TestCase):
    def invoke(self, *, push=False, conclusion='success', failed_ancestor=False,
               remote='git@github.com:phynics/cast-iron.git', sha=SHA, ci_sha=SHA):
        def output(*args):
            if args[:4] == ('git', 'remote', 'get-url', '--push'):
                return remote
            if args[:3] == ('gh', 'run', 'list'):
                return json.dumps([{'headSha': ci_sha, 'conclusion': conclusion, 'url': 'CI URL'}])
            return ''

        def execute(args, **kwargs):
            if failed_ancestor and args[:3] == ['git', 'merge-base', '--is-ancestor']:
                raise subprocess.CalledProcessError(1, args)
            return subprocess.CompletedProcess(args, 0)

        argv = ['promote-release.py', sha] + (['--push'] if push else [])
        with patch('sys.argv', argv), patch.object(promote, 'run', side_effect=output), \
                patch.object(promote.subprocess, 'run', side_effect=execute) as commands, \
                patch('builtins.print'), redirect_stderr(io.StringIO()):
            try:
                promote.main()
            finally:
                self.commands = [call.args[0] for call in commands.call_args_list]

    def assert_no_push(self):
        self.assertFalse(any(c[:2] == ['git', 'push'] for c in self.commands))

    def test_preview_never_pushes(self):
        self.invoke()
        self.assert_no_push()

    def test_push_uses_exact_sha_without_force(self):
        self.invoke(push=True)
        self.assertEqual(self.commands[-1], ['git', 'push', 'origin', f'{SHA}:refs/heads/release'])
        self.assertEqual(self.commands[:2], [
            ['git', 'merge-base', '--is-ancestor', SHA, 'origin/main'],
            ['git', 'merge-base', '--is-ancestor', 'origin/release', SHA],
        ])

    def test_non_ancestor_stops_before_push(self):
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke(push=True, failed_ancestor=True)
        self.assert_no_push()

    def test_failed_or_pending_ci_stops_before_push(self):
        for conclusion in ('failure', ''):
            with self.subTest(conclusion=conclusion), self.assertRaises(SystemExit):
                self.invoke(push=True, conclusion=conclusion)
            self.assert_no_push()

    def test_ci_must_match_exact_commit(self):
        with self.assertRaises(SystemExit):
            self.invoke(push=True, ci_sha='a' * 40)
        self.assert_no_push()

    def test_wrong_remote_stops_before_push(self):
        with self.assertRaises(SystemExit):
            self.invoke(push=True, remote='git@github.com:other/repo.git')
        self.assert_no_push()

    def test_moving_ref_stops_before_push(self):
        with self.assertRaises(SystemExit):
            self.invoke(push=True, sha='main')
        self.assert_no_push()


if __name__ == '__main__':
    unittest.main()
