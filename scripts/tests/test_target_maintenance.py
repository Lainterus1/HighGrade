"""Safety checks for the project-owned target layout."""

import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    'target_maintenance', Path(__file__).parents[1] / 'target-maintenance.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class TargetMaintenanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.project = self.root / 'target/highgrade'
        (self.project / 'tmp/run-1').mkdir(parents=True)
        (self.project / 'tmp/run-1/input.json').write_text('temporary')
        (self.project / 'candidate').mkdir()
        (self.project / 'candidate/highgrade.exe').write_bytes(b'candidate')
        (self.root / 'target/unknown.txt').write_text('foreign')
        (self.project / 'tmp/notes.txt').write_text('foreign scratch')

    def test_preview_is_read_only_and_apply_preserves_candidate_and_unknown(self):
        candidate = (self.project / 'candidate/highgrade.exe').read_bytes()
        preview = module.maintain(self.root)
        self.assertEqual(preview['status'], 'preview')
        self.assertEqual(preview['planned'], [])
        self.assertTrue((self.project / 'tmp/run-1/input.json').exists())
        self.assertEqual((self.project / 'tmp/notes.txt').read_text(), 'foreign scratch')
        with patch.object(module, 'active_builds', return_value=[]):
            applied = module.maintain(self.root, apply=True, scratch=('run-1',))
        self.assertEqual(applied['status'], 'applied')
        self.assertEqual(applied['after']['scratch'], ['notes.txt'])
        self.assertEqual((self.project / 'tmp/notes.txt').read_text(), 'foreign scratch')
        self.assertEqual((self.project / 'candidate/highgrade.exe').read_bytes(), candidate)
        self.assertEqual((self.root / 'target/unknown.txt').read_text(), 'foreign')
        self.assertEqual(applied['after']['unknown_root'], ['unknown.txt'])

    def test_active_build_and_recovery_state_refuse_without_changes(self):
        scratch = self.project / 'tmp/run-1/input.json'
        with patch.object(module, 'active_builds', return_value=['cargo']):
            with self.assertRaisesRegex(ValueError, 'Build processes active'):
                module.maintain(self.root, apply=True, scratch=('run-1',))
        self.assertTrue(scratch.exists())
        (self.project / 'candidate.previous').mkdir()
        with patch.object(module, 'active_builds', return_value=[]):
            with self.assertRaisesRegex(ValueError, 'Candidate recovery'):
                module.maintain(self.root, apply=True, scratch=('run-1',))
        self.assertTrue(scratch.exists())

    def test_link_outside_root_refuses_and_preserves_sentinel(self):
        outside = self.root.parent / (self.root.name + '-outside')
        outside.mkdir()
        self.addCleanup(lambda: outside.rmdir())
        sentinel = outside / 'keep.txt'
        sentinel.write_text('preserve')
        self.addCleanup(lambda: sentinel.unlink())
        link = self.project / 'tmp/linked'
        if os.name == 'nt':
            env = dict(os.environ, HG_TEST_LINK=str(link), HG_TEST_OUTSIDE=str(outside))
            subprocess.run(['powershell', '-NoProfile', '-Command',
                            'New-Item -ItemType Junction -Path $env:HG_TEST_LINK -Target $env:HG_TEST_OUTSIDE | Out-Null'],
                           env=env, check=True, capture_output=True)
            self.addCleanup(lambda: os.rmdir(link))
        else:
            link.symlink_to(outside, target_is_directory=True)
            self.addCleanup(lambda: link.unlink())
        with patch.object(module, 'active_builds', return_value=[]):
            with self.assertRaisesRegex(ValueError, 'Linked or unsupported'):
                module.maintain(self.root, apply=True, scratch=('run-1',))
        self.assertEqual(sentinel.read_text(), 'preserve')
        self.assertTrue((self.project / 'tmp/run-1/input.json').exists())

    def test_cache_cleanup_requires_explicit_selection(self):
        calls = []

        def clean(command, **kwargs):
            calls.append((command, kwargs['cwd']))

        with patch.object(module, 'active_builds', return_value=[]), patch.object(module.subprocess, 'run', side_effect=clean):
            module.maintain(self.root, apply=True, caches=('release', 'debug'), scratch=('run-1',))
        self.assertEqual(calls, [(['cargo', 'clean', '--target-dir', str(self.root / 'target'), '--release'], self.root),
                                 (['cargo', 'clean', '--target-dir', str(self.root / 'target'), '--profile', 'test'], self.root)])
        self.assertEqual((self.project / 'candidate/highgrade.exe').read_bytes(), b'candidate')

    def test_scratch_path_escape_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'unsafe scratch selection'):
            module.maintain(self.root, apply=True, scratch=('../candidate',))
        self.assertEqual((self.project / 'candidate/highgrade.exe').read_bytes(), b'candidate')


if __name__ == '__main__':
    unittest.main()
