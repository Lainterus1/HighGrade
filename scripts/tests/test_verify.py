"""Regression coverage for console-independent verification reports."""
import importlib.util
import io
import json
import os
from pathlib import Path
from types import SimpleNamespace
import unittest
import tempfile
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / 'verify.py'
spec = importlib.util.spec_from_file_location('verify', MODULE)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class VerifyOutputTests(unittest.TestCase):
    def test_default_test_command_enters_the_owned_run_wrapper(self):
        with patch('sys.argv', ['verify.py', 'tests']), \
                patch.dict(os.environ, {}, clear=True), \
                patch.object(verify, 'storage_preflight'), \
                patch.object(verify, 'run') as execute:
            verify.main()
        command = execute.call_args.args[0]
        self.assertIn('evidence_store.py', str(command[1]))
        self.assertEqual(command[2:6], ['run', '--name', 'rust', '--'])
        self.assertEqual(str(command[-1]), 'tests')

    def test_managed_test_attempt_keeps_native_outputs_even_on_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            owned = root / 'target/highgrade/runs/rust-fixture'; owned.mkdir(parents=True)
            (owned/'run.json').write_text(json.dumps({'schema':1,'state':'active','owner_pid':os.getpid(),'run_token':'fixture'}))
            reports = root / 'target/nextest/highgrade'; reports.mkdir(parents=True)
            def execute(args, output=None, codes=(0,)):
                if 'capture' in args:
                    (reports/'attempt.json').write_text('{"state":"running"}')
                if output:
                    output.write_text('{"rust-suites":{}}')
                if args[:3] == ['cargo','nextest','run']:
                    (reports/'junit.xml').write_text('<failure>real diagnostic fixture</failure>')
                    raise SystemExit(7)
            with patch('sys.argv', ['verify.py','tests']), \
                    patch.object(verify,'ROOT',root), patch.object(verify,'REPORTS',reports), \
                    patch.dict(os.environ, {'HIGHGRADE_RUN_DIR':str(owned),'HIGHGRADE_RUN_TOKEN':'fixture'}), \
                    patch.object(verify,'storage_preflight'), patch.object(verify,'run',side_effect=execute):
                with self.assertRaises(SystemExit) as error: verify.main()
            self.assertEqual(7,error.exception.code)
            for name in ['attempt.json','list.json','junit.xml']:
                self.assertEqual((reports/name).read_bytes(),(owned/'nextest'/name).read_bytes())
            self.assertFalse((owned/'nextest/completion.json').exists())

    def test_foreign_or_finished_report_folder_refuses_before_running_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            foreign = root / 'docs/evidence'; foreign.mkdir(parents=True)
            finished = root / 'target/highgrade/runs/rust-fixture'; finished.mkdir(parents=True)
            (finished/'run.json').write_text(json.dumps({'schema':1,'state':'sealed','owner_pid':None}))
            for folder in [foreign,finished]:
                with self.subTest(folder=folder), patch('sys.argv',['verify.py','tests']), \
                        patch.object(verify,'ROOT',root), patch.dict(os.environ,{'HIGHGRADE_RUN_DIR':str(folder)}), \
                        patch.object(verify,'storage_preflight'), patch.object(verify,'run') as execute:
                    with self.assertRaises(ValueError): verify.main()
                    execute.assert_not_called()

    def test_foreign_active_context_cannot_replace_another_runs_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            foreign = root / 'target/highgrade/runs/foreign-active'
            (foreign/'nextest').mkdir(parents=True)
            (foreign/'run.json').write_text(json.dumps({
                'schema':1, 'state':'active', 'owner_pid':os.getpid(), 'run_token':'other-run'}))
            report = foreign/'nextest/junit.xml'; report.write_text('preserve active report')
            reports = root/'target/nextest/highgrade'; reports.mkdir(parents=True)
            (reports/'junit.xml').write_text('unrelated report')
            for token in ['', 'wrong-run']:
                with self.subTest(token=token), patch('sys.argv',['verify.py','tests']), \
                        patch.object(verify,'ROOT',root), patch.object(verify,'REPORTS',reports), \
                        patch.dict(os.environ,{'HIGHGRADE_RUN_DIR':str(foreign),'HIGHGRADE_RUN_TOKEN':token}), \
                        patch.object(verify,'storage_preflight'), patch.object(verify,'run') as execute:
                    with self.assertRaisesRegex(ValueError, 'context'): verify.main()
                    execute.assert_not_called()
                self.assertEqual('preserve active report', report.read_text())

    def test_low_space_refuses_before_capture_or_report_replacement(self):
        metadata = SimpleNamespace(stdout=json.dumps({'target_directory': str(verify.ROOT / 'target')}))
        with patch('sys.argv', ['verify.py', 'tests']), \
                patch.object(verify.subprocess, 'run', return_value=metadata), \
                patch.object(verify.shutil, 'disk_usage', return_value=SimpleNamespace(free=1)), \
                patch.object(verify, 'run') as execute, \
                patch('sys.stdout', io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, 'Insufficient build headroom'):
                verify.main()
        execute.assert_not_called()

    def test_headroom_uses_cargo_target_and_accepts_exact_boundary(self):
        target = verify.ROOT.parent / 'configured-target-not-created' / 'nested'
        metadata = SimpleNamespace(stdout=json.dumps({'target_directory': str(target)}))
        with patch.object(verify.subprocess, 'run', return_value=metadata) as cargo, \
                patch.object(verify.shutil, 'disk_usage',
                             return_value=SimpleNamespace(free=verify.MIN_BUILD_FREE_BYTES)) as disk, \
                patch('sys.stdout', io.StringIO()):
            result = verify.storage_preflight()
        self.assertEqual([item['destination'] for item in result['storage_preflight']],
                         [str(verify.ROOT), str(target)])
        self.assertEqual(disk.call_args_list[-1].args[0], verify.ROOT.parent)
        self.assertEqual(cargo.call_args.args[0],
                         ['cargo', 'metadata', '--locked', '--offline', '--no-deps', '--format-version', '1'])

    def test_low_configured_target_refuses_even_when_project_has_space(self):
        target = verify.ROOT.parent / 'configured-target-not-created'
        metadata = SimpleNamespace(stdout=json.dumps({'target_directory': str(target)}))
        with patch.object(verify.subprocess, 'run', return_value=metadata), \
                patch.object(verify.shutil, 'disk_usage', side_effect=[
                    SimpleNamespace(free=verify.MIN_BUILD_FREE_BYTES * 2),
                    SimpleNamespace(free=verify.MIN_BUILD_FREE_BYTES - 1)]), \
                patch('sys.stdout', io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, 'Insufficient build headroom'):
                verify.storage_preflight()

    def test_inspect_report_round_trips_on_windows_legacy_console(self):
        result = {'status': 'passed', 'findings': [],
                  'measurements': [{'path': 'проект/данные', 'note': '✓ 😀'}]}
        stream = io.BytesIO()
        stdout = io.TextIOWrapper(stream, encoding='cp1252', errors='strict')
        with patch('sys.argv', ['verify.py', 'inspect']), \
                patch.object(verify, 'report', return_value=result), \
                patch('sys.stdout', stdout):
            verify.main()
            stdout.flush()
        self.assertEqual(json.loads(stream.getvalue().decode('cp1252')), result)


if __name__ == '__main__':
    unittest.main()
