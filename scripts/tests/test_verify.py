"""Regression coverage for console-independent verification reports."""
import importlib.util
import io
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / 'verify.py'
spec = importlib.util.spec_from_file_location('verify', MODULE)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class VerifyOutputTests(unittest.TestCase):
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
