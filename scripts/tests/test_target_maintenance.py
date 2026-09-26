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
        resolved_root = self.root.resolve(strict=True)
        target_dir = str(resolved_root / 'target')
        self.assertEqual(calls, [(['cargo', 'clean', '--target-dir', target_dir, '--release'], resolved_root),
                                 (['cargo', 'clean', '--target-dir', target_dir, '--profile', 'test'], resolved_root)])
        self.assertEqual((self.project / 'candidate/highgrade.exe').read_bytes(), b'candidate')

    def test_scratch_path_escape_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'unsafe scratch selection'):
            module.maintain(self.root, apply=True, scratch=('../candidate',))
        self.assertEqual((self.project / 'candidate/highgrade.exe').read_bytes(), b'candidate')

    def test_selected_preview_and_real_build_cache_cleanup(self):
        (self.root / 'Cargo.toml').write_text('[package]\nname="cleanup_fixture"\nversion="0.1.0"\nedition="2021"\n')
        (self.root / 'src').mkdir()
        (self.root / 'src/lib.rs').write_text('pub fn value() -> u8 { 1 }\n')
        cache = self.project / 'build-cache'
        subprocess.run(['cargo', 'build', '--offline', '--release', '--target-dir', str(cache)],
                       cwd=self.root, check=True, capture_output=True)
        (cache / 'foreign.txt').write_text('outside Cargo release output')
        reports = self.project / 'reports'
        reports.mkdir()
        (reports / 'old.log').write_text('obsolete')
        (reports / 'keep.log').write_text('keep')
        before = {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob('*') if p.is_file()}
        result = subprocess.run(['python', str(Path(module.__file__).resolve()), '--root', str(self.root),
                                 '--cache', 'build-cache', '--scratch', 'run-1', '--report', 'old.log'],
                                check=True, capture_output=True, text=True)
        import json
        preview = json.loads(result.stdout)
        self.assertEqual(preview['status'], 'preview')
        self.assertEqual(before, {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob('*') if p.is_file()})
        with patch.object(module, 'active_builds', return_value=[]):
            applied = module.maintain(self.root, apply=True, caches=('build-cache',), scratch=('run-1',), reports=('old.log',))
        self.assertEqual(applied['planned'], preview['planned'])
        self.assertEqual(applied['cargo_commands'], preview['cargo_commands'])
        self.assertFalse((cache / 'release').exists())
        self.assertFalse((reports / 'old.log').exists())
        self.assertEqual((reports / 'keep.log').read_text(), 'keep')
        self.assertEqual((cache / 'foreign.txt').read_text(), 'outside Cargo release output')
        self.assertEqual((self.project / 'candidate/highgrade.exe').read_bytes(), b'candidate')

    def test_invalid_mixed_selection_refuses_before_deleting_scratch(self):
        for args in [{'caches': ('unknown',)}, {'reports': ('../candidate/highgrade.exe',)}, {'reports': ('missing.log',)}]:
            with self.subTest(args=args), patch.object(module, 'active_builds', return_value=[]):
                with self.assertRaises(ValueError):
                    module.maintain(self.root, apply=True, scratch=('run-1',), **args)
            self.assertTrue((self.project / 'tmp/run-1/input.json').exists())
        (self.project / 'work').mkdir()
        (self.project / 'work/build.lock').write_text('running')
        with patch.object(module, 'active_builds', return_value=[]):
            with self.assertRaisesRegex(ValueError, 'build work'):
                module.maintain(self.root, apply=True, scratch=('run-1',))
        self.assertTrue((self.project / 'tmp/run-1/input.json').exists())

    def test_cache_file_refuses_before_removing_selected_scratch(self):
        for relative in ['build-cache', 'build-cache/release']:
            with self.subTest(relative=relative):
                path = self.project / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('foreign file')
                with patch.object(module, 'active_builds', return_value=[]):
                    with self.assertRaisesRegex(ValueError, 'not a directory'):
                        module.maintain(self.root, apply=True, caches=('build-cache',), scratch=('run-1',))
                self.assertTrue((self.project / 'tmp/run-1/input.json').exists())
                self.assertEqual(path.read_text(), 'foreign file')
                path.unlink()


if __name__ == '__main__':
    unittest.main()
