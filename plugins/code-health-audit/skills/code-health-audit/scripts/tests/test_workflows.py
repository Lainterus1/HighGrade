"""Real CLI regressions. Run with the pinned analyzer interpreter."""
from __future__ import annotations
import importlib.metadata
import json
import os
import py_compile
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from code_health.execution import run_adapter
from code_health.processes import run_bounded

PINS = json.loads((SCRIPTS / 'data/toolchain.json').read_text())["tools"]
POLICY = json.loads((SCRIPTS / 'data/default-policy.json').read_text())
def pins_available():
    try:
        return all(importlib.metadata.version(name) == version for name, version in PINS.items())
    except importlib.metadata.PackageNotFoundError:
        return False


@unittest.skipUnless(pins_available(), 'requires exact pinned analyzer environment')
class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.work = Path(self.temp.name)
        self.root = self.work / 'project with spaces'
        self.root.mkdir()
        self.env = os.environ.copy()
        self.env['PYTHONDONTWRITEBYTECODE'] = '1'
        self.run_cmd(['git', 'init', '-b', 'main'])
        self.write('m.py', 'def f(x):\n    return x + 1\n')
        self.write('test_m.py', 'from m import f\nassert f(1) == 2\n')
        self.run_cmd(['git', 'add', '.'])
        self.run_cmd(['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-m', 'fixture'])

    def write(self, path, value):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(value, encoding='utf-8')

    def run_cmd(self, args, expected=0):
        result = subprocess.run(args, cwd=self.root, env=self.env, capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=45)
        if expected is not None:
            self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        return result

    def capture(self, expected=0):
        output = self.work / 'capture'
        result = self.run_cmd([sys.executable, '-B', str(SCRIPTS / 'capture_coverage.py'), '--root', str(self.root), '--output-dir', str(output), '--script', 'test_m.py'], expected)
        return output, result

    def audit(self, extra=(), expected=0):
        output = self.work / ('report-' + str(len(list(self.work.glob('report-*')))))
        self.run_cmd([sys.executable, '-B', str(SCRIPTS / 'audit.py'), '--root', str(self.root), '--mode', 'full', '--output-dir', str(output), *extra], expected)
        return json.loads((output / 'code-health-audit.json').read_text(encoding='utf-8'))

    def test_observed_capture_and_repeat_audit(self):
        cap, _ = self.capture()
        options = ['--coverage-json', str(cap / 'coverage.json'), '--coverage-provenance', str(cap / 'coverage-provenance.json')]
        first = self.audit(options)
        second = self.audit(options)
        self.assertEqual(first['audit']['verdict'], 'HEALTHY')
        self.assertEqual(first['audit']['content_digest'], second['audit']['content_digest'])

    def test_old_data_reexport_cannot_confirm_changed_sources(self):
        cap, _ = self.capture()
        self.write('m.py', 'def f(x):\n    raise RuntimeError("untested")\n')
        self.run_cmd([sys.executable, '-B', '-m', 'coverage', 'json', '--data-file', str(cap / '.coverage'), '-o', str(cap / 'coverage.json')])
        report = self.audit(['--coverage-json', str(cap / 'coverage.json'), '--coverage-provenance', str(cap / 'coverage-provenance.json')], expected=2)
        self.assertEqual(report['audit']['verdict'], 'INCOMPLETE')
        self.assertFalse(any(m['metric'].endswith('coverage') for m in report['measurements']))

    def test_json_without_provenance_is_incomplete(self):
        cap, _ = self.capture()
        report = self.audit(['--coverage-json', str(cap / 'coverage.json')], expected=2)
        self.assertTrue(any(e['code'] == 'COVERAGE_PROVENANCE_UNVERIFIED' for e in report['errors']))

    def test_failed_test_cannot_create_verified_capture(self):
        self.write('test_m.py', 'raise RuntimeError("failed test")\n')
        cap, _ = self.capture(expected=2)
        self.assertEqual(json.loads((cap / 'coverage-provenance.json').read_text())['status'], 'FAILED')

    def test_stale_bytecode_cannot_attest_different_source(self):
        path = self.root / 'm.py'
        before = path.stat()
        py_compile.compile(str(path), doraise=True)
        self.write('m.py', 'def f(x):\n    return x + 2\n')
        os.utime(path, ns=(before.st_atime_ns, before.st_mtime_ns))
        cap, _ = self.capture(expected=2)
        self.assertEqual(json.loads((cap / 'coverage-provenance.json').read_text())['status'], 'FAILED')

    def test_syntax_error_produces_report_and_final_snapshot(self):
        self.write('m.py', 'def f():\n    return 1 +\n')
        report = self.audit(expected=2)
        self.assertNotEqual(report['audit']['completeness'], 'COMPLETE')
        self.assertTrue(report['audit']['read_only_verified'])
        self.assertTrue(any(e['code'] == 'AST_PARSE_FAILED' for e in report['errors']))

    def test_invalid_policy_produces_failure_report(self):
        path = self.work / 'invalid-policy.json'
        path.write_text('{broken')
        report = self.audit(['--policy', str(path)], expected=2)
        self.assertEqual(report['audit']['verdict'], 'FAILED')

    def test_worker_timeout_returns_failure_without_measurements(self):
        policy = dict(POLICY, analyzer_timeout_seconds=0.00001)
        result = run_adapter('lizard', self.root, ['m.py'], {}, policy, PINS['lizard'])
        self.assertEqual(result['tool']['status'], 'FAILED')
        self.assertEqual(result['measurements'], [])

    def test_timeout_stops_descendant_before_late_write(self):
        started, late = self.work / 'started', self.work / 'late'
        child = 'from pathlib import Path; import time; Path(' + repr(str(started)) + ').touch(); time.sleep(2); Path(' + repr(str(late)) + ').touch()'
        parent = 'import subprocess,sys,time; subprocess.Popen([sys.executable,"-c",' + repr(child) + ']); time.sleep(30)'
        with self.assertRaises(subprocess.TimeoutExpired):
            run_bounded([sys.executable, '-c', parent], timeout=1, capture_output=True)
        self.assertTrue(started.exists(), 'fixture child must have started')
        time.sleep(2)
        self.assertFalse(late.exists(), 'descendant survived timeout')


if __name__ == '__main__':
    unittest.main()
