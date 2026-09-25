"""Real Git/Cargo lifecycle checks; all files belong to isolated temporary repos."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('build_release', Path(__file__).parents[1] / 'build-release.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseBuildTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'src').mkdir()
        (self.root / 'kit').mkdir()
        (self.root / 'Cargo.toml').write_text('[package]\nname="highgrade"\nversion="0.0.0"\nedition="2021"\n')
        (self.root / 'src/main.rs').write_text('fn main() { println!("committed"); }')
        (self.root / 'kit/version.txt').write_text('committed kit')
        self.run_command('cargo', 'generate-lockfile')
        self.run_command('git', 'init', '-q')
        self.run_command('git', 'config', 'user.email', 'test@example.invalid')
        self.run_command('git', 'config', 'user.name', 'Local test')
        self.run_command('git', 'add', 'Cargo.toml', 'Cargo.lock', 'src', 'kit')
        self.run_command('git', 'commit', '-qm', 'fixture')
        self.sha = self.run_command('git', 'rev-parse', 'HEAD').strip()

    def run_command(self, *args):
        return subprocess.check_output(args, cwd=self.root, stderr=subprocess.STDOUT, text=True)

    def assert_no_temporary_sources(self):
        self.assertEqual(list((self.root / 'target/highgrade/work').iterdir()), [])

    def test_exact_revision_repeat_and_failed_build_preserve_candidate(self):
        (self.root / 'src/main.rs').write_text('invalid dirty source')
        (self.root / 'kit/version.txt').write_text('dirty kit')
        first = module.build(self.root, self.sha)
        second = module.build(self.root, self.sha)
        self.assertEqual(first['files'].keys(), second['files'].keys())
        self.assertEqual(first['files']['kit/version.txt'], second['files']['kit/version.txt'])
        self.assertEqual(first['source_sha'], self.sha)
        self.assertEqual(first['cargo_target_dir'], str(self.root / 'target/highgrade/build-cache'))
        candidate = self.root / 'target/highgrade/candidate'
        self.assertEqual((candidate / 'kit/version.txt').read_text(), 'committed kit')
        executable = candidate / ('highgrade.exe' if os.name == 'nt' else 'highgrade')
        self.assertEqual(self.run_command(str(executable)).strip(), 'committed')
        self.assert_no_temporary_sources()
        before = module.files(candidate)
        self.run_command('git', 'add', 'src/main.rs')
        self.run_command('git', 'commit', '-qm', 'broken source')
        with self.assertRaises(subprocess.CalledProcessError):
            module.build(self.root, 'HEAD')
        self.assertEqual(module.files(candidate), before)
        self.assertIn('FAILED:', (self.root / 'target/highgrade/reports/release-build.log').read_text())
        self.assert_no_temporary_sources()

    def test_foreign_candidate_data_is_preserved(self):
        module.build(self.root, self.sha)
        candidate = self.root / 'target/highgrade/candidate'
        foreign = candidate / 'notes.txt'
        foreign.write_text('keep me')
        with self.assertRaises(ValueError):
            module.build(self.root, self.sha)
        self.assertEqual(foreign.read_text(), 'keep me')
        foreign.unlink()
        (candidate / 'empty-foreign-directory').mkdir()
        with self.assertRaises(ValueError):
            module.build(self.root, self.sha)
        self.assertTrue((candidate / 'empty-foreign-directory').is_dir())
        self.assert_no_temporary_sources()

    def test_linked_target_is_refused(self):
        self.check_linked_path('target')

    def test_nested_cache_junction_is_refused_before_cargo_writes(self):
        (self.root / 'target/highgrade').mkdir(parents=True)
        self.check_linked_path('target/highgrade/build-cache')

    def test_double_rename_failure_keeps_recovery_candidate(self):
        module.build(self.root, self.sha)
        candidate = self.root / 'target/highgrade/candidate'
        before = module.files(candidate)
        rename = Path.rename

        def fail_replacement(path, destination):
            if ((path.name == 'candidate' and path.parent != self.root / 'target/highgrade')
                    or path.name == 'candidate.previous'):
                raise OSError('injected rename failure')
            return rename(path, destination)

        with patch.object(Path, 'rename', fail_replacement):
            with self.assertRaises(OSError):
                module.build(self.root, self.sha)
        recovery = self.root / 'target/highgrade/candidate.previous'
        self.assertEqual(module.files(recovery), before)
        with self.assertRaises(ValueError):
            module.build(self.root, self.sha)
        self.assertEqual(module.files(recovery), before)
        self.assert_no_temporary_sources()

    def test_developer_release_does_not_replace_candidate_artifact(self):
        original = subprocess.run

        def run_with_competing_build(args, **kwargs):
            result = original(args, **kwargs)
            if args[:2] == ['cargo', 'build']:
                environment = os.environ.copy()
                environment['CARGO_TARGET_DIR'] = str(self.root / 'target')
                original(['cargo', 'build', '--release', '--locked'], cwd=self.root,
                         env=environment, check=True, capture_output=True)
            return result

        (self.root / 'src/main.rs').write_text('fn main() { println!("different developer build"); }')
        with patch.object(subprocess, 'run', run_with_competing_build):
            module.build(self.root, self.sha)
        executable = self.root / 'target/highgrade/candidate' / ('highgrade.exe' if os.name == 'nt' else 'highgrade')
        self.assertEqual(self.run_command(str(executable)).strip(), 'committed')

    def test_target_override_uses_artifact_from_current_build(self):
        module.build(self.root, self.sha)
        (self.root / 'src/main.rs').write_text('fn main() { println!("new commit"); }')
        self.run_command('git', 'add', 'src/main.rs')
        self.run_command('git', 'commit', '-qm', 'second source')
        second_sha = self.run_command('git', 'rev-parse', 'HEAD').strip()
        host = next(line.split(': ', 1)[1] for line in self.run_command('rustc', '-vV').splitlines()
                    if line.startswith('host: '))
        with patch.dict(os.environ, {'CARGO_BUILD_TARGET': host}):
            record = module.build(self.root, second_sha)
        executable = self.root / 'target/highgrade/candidate' / ('highgrade.exe' if os.name == 'nt' else 'highgrade')
        self.assertEqual(record['source_sha'], second_sha)
        self.assertEqual(self.run_command(str(executable)).strip(), 'new commit')
        self.assert_no_temporary_sources()

    def check_linked_path(self, relative):
        outside = self.root / 'outside'
        outside.mkdir()
        marker = outside / 'keep.txt'
        marker.write_text('preserve')
        link = self.root / relative
        if os.name == 'nt':
            environment = dict(os.environ, HG_TEST_LINK=str(link), HG_TEST_OUTSIDE=str(outside))
            subprocess.run(['powershell', '-NoProfile', '-Command',
                            'New-Item -ItemType Junction -Path $env:HG_TEST_LINK -Target $env:HG_TEST_OUTSIDE | Out-Null'],
                           env=environment, check=True, capture_output=True)
        else:
            link.symlink_to(outside, target_is_directory=True)
        try:
            with self.assertRaises(ValueError):
                module.build(self.root, self.sha)
            self.assertEqual(marker.read_text(), 'preserve')
            self.assertEqual(list(outside.iterdir()), [marker])
        finally:
            if os.name == 'nt':
                os.rmdir(link)
            else:
                link.unlink()


if __name__ == '__main__':
    unittest.main()
