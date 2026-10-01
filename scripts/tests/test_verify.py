"""Regression coverage for console-independent verification reports."""
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / 'verify.py'
spec = importlib.util.spec_from_file_location('verify', MODULE)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class VerifyOutputTests(unittest.TestCase):
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
