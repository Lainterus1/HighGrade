"""Safety checks for the project-owned target layout."""

import importlib.util
from contextlib import contextmanager, redirect_stdout
import io
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    'target_maintenance', Path(__file__).parents[1] / 'target-maintenance.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def stat_fixture(info, *, without=(), **overrides):
    # Preserve platform metadata, including Windows reparse-point attributes.
    fields = {name: getattr(info, name) for name in dir(info)
              if name.startswith('st_') and name not in without}
    fields.update(overrides)
    return SimpleNamespace(**fields)


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

    def test_inventory_deduplicates_hardlinks_and_reports_each_area(self):
        debug = self.root / 'target/debug'
        debug.mkdir()
        payload = debug / 'binary'
        payload.write_bytes(b'x' * (2 * 1024 * 1024))
        os.link(payload, debug / 'binary-alias')
        reports = self.project / 'reports'
        reports.mkdir()
        (reports / 'result.log').write_bytes(b'y' * (1024 * 1024))
        os.link(payload, reports / 'cross-area-alias')

        report = module.inventory(self.root)

        self.assertEqual(report['mib'], 3.0)
        self.assertEqual(report['logical_mib'], 7.0)
        self.assertEqual(report['hardlink_duplicates_mib'], 4.0)
        self.assertEqual(report['areas']['debug']['mib'], 2.0)
        self.assertEqual(report['areas']['debug']['logical_mib'], 4.0)
        self.assertEqual(report['areas']['highgrade/reports']['mib'], 3.0)
        self.assertEqual(report['areas']['highgrade/reports']['logical_mib'], 3.0)
        if hasattr(payload.stat(), 'st_blocks'):
            # Physical allocation includes the directories, without charging
            # either Cargo's aliases or a cross-area hardlink a second time.
            paths = [self.root / 'target', *(self.root / 'target').rglob('*')]
            unique = {(p.stat().st_dev, p.stat().st_ino): p.stat().st_blocks * 512
                      for p in paths}
            self.assertEqual(report['physical_mib'], round(sum(unique.values()) / 1024 / 1024, 2))
        self.assertEqual(payload.read_bytes(), b'x' * (2 * 1024 * 1024))

    def test_inventory_without_allocated_size_reports_unknown_not_estimate(self):
        stat_path = module.os.stat

        def portable_stat(path, *args, **kwargs):
            # Windows exposes no st_blocks. Keep every other platform field.
            return stat_fixture(stat_path(path, *args, **kwargs), without=('st_blocks',))

        with patch.object(module.os, 'stat', portable_stat):
            report = module.inventory(self.root)
        self.assertIsNone(report['physical_mib'])
        self.assertIsNone(report['areas']['highgrade/tmp']['physical_mib'])
        self.assertEqual(report['mib'], 0.0)

    def test_inventory_does_not_deduplicate_unavailable_inode_ids(self):
        debug = self.root / 'target/debug'
        debug.mkdir()
        for name in ('one', 'two'):
            (debug / name).write_bytes(b'x' * (1024 * 1024))
        stat_path = module.os.stat

        def unidentified_stat(path, *args, **kwargs):
            return stat_fixture(stat_path(path, *args, **kwargs), st_ino=0)

        with patch.object(module.os, 'stat', unidentified_stat):
            report = module.inventory(self.root)
        self.assertEqual(report['mib'], 2.0)
        self.assertEqual(report['logical_mib'], 2.0)
        self.assertEqual(report['hardlink_duplicates_mib'], 0.0)

    def test_windows_direntry_zero_identity_uses_real_path_stat(self):
        debug = self.root / 'target/debug'
        debug.mkdir()
        payload = debug / 'binary'
        payload.write_bytes(b'x' * (2 * 1024 * 1024))
        os.link(payload, debug / 'alias')
        scan = module.os.scandir

        class WindowsEntry:
            def __init__(self, entry):
                self.entry = entry

            def __getattr__(self, name):
                return getattr(self.entry, name)

            def stat(self, **kwargs):
                # Windows DirEntry.stat has zero st_ino/st_dev/st_nlink even
                # when os.stat can provide the true filesystem identity.
                return stat_fixture(self.entry.stat(**kwargs), st_ino=0, st_dev=0, st_nlink=0)

        @contextmanager
        def windows_scan(directory):
            with scan(directory) as entries:
                yield (WindowsEntry(entry) for entry in entries)

        with patch.object(module.os, 'scandir', windows_scan):
            report = module.inventory(self.root)
        self.assertEqual(report['mib'], 2.0)
        self.assertEqual(report['logical_mib'], 4.0)
        self.assertEqual(report['hardlink_duplicates_mib'], 2.0)
        self.assertEqual(report['areas']['debug']['mib'], 2.0)

    def test_missing_target_has_empty_measurements(self):
        report = module.inventory(self.root / 'absent')
        self.assertEqual(report['mib'], 0.0)
        self.assertEqual(report['logical_mib'], 0.0)
        self.assertEqual(report['physical_mib'], 0.0)
        self.assertEqual(report['hardlink_duplicates_mib'], 0.0)
        self.assertEqual(report['areas'], {})

    def test_budget_check_counts_hardlinked_payload_once(self):
        root = self.root / 'budget-fixture'
        debug = root / 'target/debug'
        debug.mkdir(parents=True)
        payload = debug / 'binary'
        payload.write_bytes(b'x' * (600 * 1024))
        os.link(payload, debug / 'alias')
        output = io.StringIO()
        with patch.object(module, 'BUDGET_MIB', 1), \
                patch('sys.argv', ['target-maintenance.py', '--root', str(root), '--check']), \
                redirect_stdout(output):
            module.main()
        import json
        report = json.loads(output.getvalue())
        self.assertLess(report['before']['mib'], 1)
        self.assertGreater(report['before']['logical_mib'], 1)

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
