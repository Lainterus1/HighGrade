"""Exercise the real freshness guard with isolated native-report fixtures."""
import importlib.util
import json
import os
import shutil
from pathlib import Path
import subprocess
import sys
import time
import tempfile
import unittest
import uuid
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / 'scripts/source-scenarios.mjs'
REPORTS = Path('target/nextest/highgrade')
XML = '<testsuites tests="1" skipped="0" failures="0" errors="0"><testsuite tests="1" skipped="0" failures="0" errors="0"><testcase name="smoke"/></testsuite></testsuites>'
module_spec = importlib.util.spec_from_file_location('verify_freshness', ROOT / 'scripts/verify.py')
verify = importlib.util.module_from_spec(module_spec)
module_spec.loader.exec_module(verify)


class SourceFreshnessTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='highgrade-freshness-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.write('specs/catalog.json', '{}')
        self.write('specs/requirements/HG-FIXTURE-R1.json', json.dumps({
            'id': 'HG-FIXTURE-R1', 'title': 'Test', 'statement': 'Keep actual evidence',
            'scenarios': [{'id': 'HG-FIXTURE-S1', 'given': 'Input', 'when': 'Test',
                           'then': 'Result', 'verification': 'Native report'}]}))
        self.write('specs/changes/HG-FIXTURE/spec.json', json.dumps({
            'archived': True, 'title': 'Metadata', 'operations': [], 'checks': []}))
        self.write('tests/smoke.rs', '// highgrade: HG-FIXTURE-S1\n#[test]\nfn smoke() {}\n')
        for name in ['src/main.rs', 'kit/rules.md', 'bundle/rules.md',
                     'tests/fixtures/p0p2-bundle/input.txt', 'build.rs', 'Cargo.toml',
                     'Cargo.lock', 'rust-toolchain.toml', '.config/nextest.toml', '.gitattributes']:
            self.write(name, 'initial\n')
        for name in ['scripts/source-scenarios.mjs', 'scripts/verify.py']:
            self.write(name, (ROOT / name).read_text(encoding='utf-8'))
        # Only the CLI semantic digest is stubbed; source discovery, lifecycle,
        # hashing, report validation and all filesystem operations are real.
        self.stub = self.write('stub-cli.mjs', '''import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
childProcess.spawnSync = () => ({ status: 0, stdout: JSON.stringify({ measurements: [{ trace_sha256: 'fixture-digest' }] }) });
syncBuiltinESMExports();
''')

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding='utf-8')
        return path

    def run_step(self, mode, success=True):
        if mode == 'capture':
            self.attempt_id = uuid.uuid4().hex
        args = [self.attempt_id] if mode in ('capture', 'complete') else []
        result = subprocess.run(['node', '--import', self.stub.as_uri(), str(CHECKER), mode, str(self.root), *args],
                                cwd=ROOT, capture_output=True, text=True, encoding='utf-8', timeout=15)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        return json.loads(result.stdout if success else result.stderr)

    def reports(self, xml=XML):
        self.write(REPORTS / 'list.json', json.dumps({'rust-suites': {'fixture': {'testcases': {'smoke': {}}}}}))
        self.write(REPORTS / 'junit.xml', xml)

    def write_trace(self):
        data = json.loads((self.root / REPORTS / 'run.json').read_text())
        provenance = {field: data[field] for field in ['tool', 'report', 'report_sha256', 'command', 'scope', 'captured_at']}
        return self.write(REPORTS / 'trace.json', json.dumps({'operation': 'trace', 'findings': [], 'measurements': [
            {'scenario_id': 'HG-FIXTURE-S1', 'status': 'passed'},
            {'scenarios_total': 1, 'automatic_passed': 1, 'outside_automatic_trace': 0,
             'test_cases_reported': 1, 'linked_test_cases': 1, 'test_cases_executed': 1,
             'test_cases_skipped': 0, 'test_cases_unknown': 0, 'test_cases_missing': 0, 'unlinked_test_cases': 0}, provenance]}))

    def with_hook(self, mode, code):
        hook = self.write('final-gate-hook.mjs', code)
        return subprocess.run(['node', '--import', hook.as_uri(), '--import', self.stub.as_uri(),
                               str(CHECKER), mode, str(self.root)], cwd=ROOT,
                              capture_output=True, text=True, encoding='utf-8', timeout=15)

    def capture_hook_action(self):
        return f"""const xml = read({json.dumps(str(self.root / REPORTS / 'junit.xml'))}), inventory = read({json.dumps(str(self.root / REPORTS / 'list.json'))});
const next = spawn(process.execPath, [{json.dumps(str(CHECKER))}, 'capture', {json.dumps(str(self.root))}, '{uuid.uuid4().hex}'], {{ encoding: 'utf8' }});
if (next.status !== 0) throw new Error(next.stderr);
fs.writeFileSync({json.dumps(str(self.root / REPORTS / 'junit.xml'))}, xml);
fs.writeFileSync({json.dumps(str(self.root / REPORTS / 'list.json'))}, inventory);
"""

    def passed_attempt(self):
        self.run_step('capture')
        self.reports()
        self.run_step('complete')
        return self.run_step('prepare')

    def test_same_report_and_equivalent_spec_metadata_are_reusable(self):
        self.passed_attempt()
        self.assertFalse(self.run_step('prepare')['native_report_reused'])
        path = self.root / 'specs/changes/HG-FIXTURE/spec.json'
        value = json.loads(path.read_text())
        value['title'] += ' updated metadata'
        path.write_text(json.dumps(value))
        self.assertEqual(self.run_step('prepare')['native_report_reason'], 'equivalent_specification')
        self.write_trace()
        self.run_step('verify')

    def test_changed_inputs_and_deleted_files_rejected_without_timestamp_drift(self):
        self.passed_attempt()
        for name in ['src/main.rs', 'kit/rules.md', 'bundle/rules.md',
                     'tests/fixtures/p0p2-bundle/input.txt', 'scripts/source-scenarios.mjs', 'scripts/verify.py']:
            with self.subTest(name=name):
                path = self.root / name
                original, stat = path.read_bytes(), path.stat()
                path.write_bytes(original + b'changed\n')
                os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
                self.assertIn('inputs changed', self.run_step('prepare', False)['message'])
                path.unlink()
                self.run_step('prepare', False)
                path.write_bytes(original)
        self.run_step('prepare')
        self.write('src/new.rs', 'new input')
        self.assertIn('inputs changed', self.run_step('prepare', False)['message'])

    def test_relocated_completed_attempt_keeps_original_capture_and_rejects_drift(self):
        self.passed_attempt()
        original = json.loads((self.root / REPORTS / 'run.json').read_text())
        with tempfile.TemporaryDirectory(prefix='highgrade-relocated-') as moved:
            destination = Path(moved) / 'checkout'
            shutil.move(str(self.root), destination)
            self.root = destination
            self.stub = destination / 'stub-cli.mjs'
            self.run_step('prepare')
            relocated = json.loads((self.root / REPORTS / 'run.json').read_text())
            self.assertEqual(relocated['capture_root'], original['capture_root'])
            self.assertEqual(relocated['captured_at'], original['captured_at'])
            self.write_trace()
            self.run_step('verify')
            for name in ['src/main.rs', REPORTS / 'junit.xml', REPORTS / 'list.json']:
                with self.subTest(name=name):
                    path = self.root / name
                    content, stat = path.read_bytes(), path.stat()
                    path.write_bytes(content + b'changed')
                    os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
                    self.run_step('prepare', False)
                    path.write_bytes(content)
            self.run_step('prepare')

    def test_older_attempt_cannot_complete_a_newer_capture(self):
        self.run_step('capture')
        old_id = self.attempt_id
        self.write('src/main.rs', 'new input that failed a newer test')
        self.run_step('capture')
        self.reports()  # An older passing runner can overwrite the shared JUnit.
        self.attempt_id = old_id
        self.assertIn('superseded', self.run_step('complete', False)['message'])
        self.assertIn('incomplete', self.run_step('prepare', False)['message'])

    def test_capture_between_complete_read_and_write_cannot_inherit_success(self):
        self.run_step('capture')
        old_id, new_id = self.attempt_id, uuid.uuid4().hex
        self.reports()
        hook = self.write('interleave.mjs', f"""import fs from 'node:fs';
import childProcess from 'node:child_process';
import {{ syncBuiltinESMExports }} from 'node:module';
const rename = fs.renameSync, spawn = childProcess.spawnSync;
fs.renameSync = (from, to) => {{
  if (to === {json.dumps(str(self.root / REPORTS / 'completion.json'))}) {{
    const next = spawn(process.execPath, [{json.dumps(str(CHECKER))}, 'capture', {json.dumps(str(self.root))}, '{new_id}'], {{ encoding: 'utf8' }});
    if (next.status !== 0) throw new Error(next.stderr);
    fs.writeFileSync({json.dumps(str(self.root / REPORTS / 'junit.xml'))}, {json.dumps(XML)});
    fs.writeFileSync({json.dumps(str(self.root / REPORTS / 'list.json'))}, {json.dumps(json.dumps({'rust-suites': {'fixture': {'testcases': {'smoke': {}}}}}))});
  }}
  return rename(from, to);
}};
syncBuiltinESMExports();
""")
        result = subprocess.run(['node', '--import', hook.as_uri(), str(CHECKER), 'complete', str(self.root), old_id],
                                capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads((self.root / REPORTS / 'attempt.json').read_text())['attempt_id'], new_id)
        self.assertEqual(json.loads((self.root / REPORTS / 'completion.json').read_text())['attempt_id'], old_id)
        self.assertIn('superseded', self.run_step('prepare', False)['message'])

    def test_running_attempt_cannot_complete_after_relocation(self):
        self.run_step('capture')
        self.reports()
        with tempfile.TemporaryDirectory(prefix='highgrade-relocated-') as moved:
            destination = Path(moved) / 'checkout'
            shutil.move(str(self.root), destination)
            self.root = destination
            self.stub = destination / 'stub-cli.mjs'
            self.assertIn('incomplete', self.run_step('complete', False)['message'])
            self.assertIn('incomplete', self.run_step('prepare', False)['message'])

    def test_first_run_drift_rejected_and_new_successful_run_is_accepted(self):
        self.run_step('capture')
        self.write('src/main.rs', 'changed during run\n')
        self.reports()
        self.assertIn('inputs changed', self.run_step('complete', False)['message'])
        self.assertIn('incomplete', self.run_step('prepare', False)['message'])
        self.passed_attempt()

    def test_semantic_change_rejected_even_with_restored_timestamp(self):
        self.passed_attempt()
        path = self.root / 'specs/requirements/HG-FIXTURE-R1.json'
        value, stat = json.loads(path.read_text()), path.stat()
        value['scenarios'][0]['then'] = 'Different requirement'
        path.write_text(json.dumps(value))
        os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        self.assertIn('inputs changed', self.run_step('prepare', False)['message'])

    def test_failed_partial_and_interrupted_attempts_cannot_inherit_success(self):
        self.passed_attempt()
        archive = self.write('docs/evidence/saved.xml', 'archived evidence')
        self.run_step('capture')
        for name in ['list.json', 'junit.xml', 'run.json', 'trace.json', 'completion.json']:
            self.assertFalse((self.root / REPORTS / name).exists())
        self.assertEqual(archive.read_text(), 'archived evidence')
        self.assertIn('incomplete', self.run_step('prepare', False)['message'])
        # No completed marker is written by a failed or interrupted runner.
        self.reports(XML.replace('failures="0"', 'failures="1"'))
        self.assertIn('not a complete pass', self.run_step('complete', False)['message'])
        self.assertIn('incomplete', self.run_step('prepare', False)['message'])
        self.reports('<testsuites>')
        self.run_step('complete', False)
        self.assertEqual(json.loads((self.root / REPORTS / 'attempt.json').read_text())['state'], 'running')
        self.passed_attempt()

    def test_capture_parse_error_invalidates_prior_success(self):
        self.passed_attempt()
        self.write('specs/catalog.json', 'invalid json')
        self.run_step('capture', False)
        self.assertFalse((self.root / REPORTS / 'run.json').exists())
        self.write('specs/catalog.json', '{}')
        self.assertIn('incomplete', self.run_step('prepare', False)['message'])

    def test_report_inventory_and_missing_snapshot_are_rejected(self):
        self.passed_attempt()
        for name in ['junit.xml', 'list.json']:
            path = self.root / REPORTS / name
            original = path.read_text()
            path.write_text(original + '\n')
            self.assertIn('changed after completion', self.run_step('prepare', False)['message'])
            path.write_text(original)
        (self.root / REPORTS / 'attempt.json').unlink()
        self.assertIn('attempt is missing', self.run_step('prepare', False)['message'])

    def test_superseded_prepare_cannot_publish_current_evidence(self):
        self.passed_attempt()
        old_trace = self.write_trace().read_text()
        result = self.with_hook('prepare', f"""import fs from 'node:fs';
import childProcess from 'node:child_process';
import {{ syncBuiltinESMExports }} from 'node:module';
const rename = fs.renameSync, read = fs.readFileSync, spawn = childProcess.spawnSync;
fs.renameSync = (from, to) => {{
  if (to === {json.dumps(str(self.root / REPORTS / 'run.json'))}) {{ {self.capture_hook_action()} }}
  return rename(from, to);
}};
syncBuiltinESMExports();
""")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        published = json.loads((self.root / REPORTS / 'run.json').read_text())
        self.assertIn((REPORTS / 'attempt.json').as_posix(), published['source_hashes'])
        self.assertIn((REPORTS / 'completion.json').as_posix(), published['source_hashes'])
        self.write(REPORTS / 'trace.json', old_trace)
        self.assertIn('incomplete', self.run_step('verify', False)['message'])

    def test_final_gate_rejects_capture_and_source_drift_while_reading_trace(self):
        for mutation in ['capture', 'source']:
            with self.subTest(mutation=mutation):
                self.passed_attempt()
                self.write_trace()
                source_path = json.dumps(str(self.root / 'src/main.rs'))
                action = self.capture_hook_action() if mutation == 'capture' else f"""const before = fs.statSync({source_path});
fs.writeFileSync({source_path}, 'changed while reading trace');
fs.utimesSync({source_path}, before.atime, before.mtime);"""
                result = self.with_hook('verify', f"""import fs from 'node:fs';
import childProcess from 'node:child_process';
import {{ syncBuiltinESMExports }} from 'node:module';
const read = fs.readFileSync, spawn = childProcess.spawnSync;
let changed = false;
fs.readFileSync = (file, ...args) => {{
  const result = read(file, ...args);
  if (!changed && file === {json.dumps(str(self.root / REPORTS / 'trace.json'))}) {{
    changed = true; {action}
  }}
  return result;
}};
syncBuiltinESMExports();
""")
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn('incomplete' if mutation == 'capture' else 'inputs changed', result.stderr)

    def test_final_identity_check_rejects_capture_after_source_freshness_reads(self):
        self.passed_attempt()
        self.write_trace()
        paths = [str(self.root / REPORTS / name) for name in ['junit.xml', 'list.json', 'run.json', 'trace.json']]
        hook = self.write('last-gate-hook.mjs', f"""import fs from 'node:fs';
import childProcess from 'node:child_process';
import {{ syncBuiltinESMExports }} from 'node:module';
const spawn = childProcess.spawnSync;
let calls = 0;
childProcess.spawnSync = () => {{
  if (++calls === 3) {{
    const files = {json.dumps(paths)}.map((file) => [file, fs.readFileSync(file)]);
    const next = spawn(process.execPath, [{json.dumps(str(CHECKER))}, 'capture', {json.dumps(str(self.root))}, '{uuid.uuid4().hex}'], {{ encoding: 'utf8' }});
    if (next.status !== 0) throw new Error(next.stderr);
    for (const [file, bytes] of files) fs.writeFileSync(file, bytes);
  }}
  return {{ status: 0, stdout: JSON.stringify({{ measurements: [{{ trace_sha256: 'fixture-digest' }}] }}) }};
}};
syncBuiltinESMExports();
""")
        result = subprocess.run(['node', '--import', hook.as_uri(), str(CHECKER), 'verify', str(self.root)],
                                cwd=ROOT, capture_output=True, text=True, encoding='utf-8', timeout=15)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse((self.root / REPORTS / 'completion.json').exists())

    def test_final_gate_rejects_unbound_record_and_mismatched_trace_provenance(self):
        self.passed_attempt()
        trace_path = self.write_trace()
        original_trace = trace_path.read_text()
        self.run_step('verify')
        for field in ['tool', 'report_sha256', 'command', 'captured_at']:
            with self.subTest(field=field):
                trace = json.loads(original_trace)
                trace['measurements'][-1][field] = 'different provenance'
                trace_path.write_text(json.dumps(trace))
                self.assertIn('does not match', self.run_step('verify', False)['message'])
        trace_path.write_text(original_trace)
        record = json.loads((self.root / REPORTS / 'run.json').read_text())
        del record['source_hashes'][(REPORTS / 'attempt.json').as_posix()]
        self.write(REPORTS / 'run.json', json.dumps(record))
        self.assertIn('changed or superseded', self.run_step('verify', False)['message'])

    def test_real_failed_and_interrupted_processes_leave_attempt_incomplete(self):
        wrapper = self.write('wrapper.py', f"""import importlib.util, pathlib, sys
spec=importlib.util.spec_from_file_location('verify', {str(ROOT / 'scripts/verify.py')!r})
verify=importlib.util.module_from_spec(spec); spec.loader.exec_module(verify)
verify.ROOT=pathlib.Path({str(self.root)!r}); verify.REPORTS=verify.ROOT/'target/nextest/highgrade'
original=verify.run
def run(args, output=None, codes=(0,)):
    if args[0]=='node': args[1]={str(CHECKER)!r}
    if args[0]=='cargo': args=[sys.executable, 'runner.py', args[2]]
    return original(args, output, codes)
verify.run=run
sys.argv=['verify.py','tests']; verify.main()
""")
        self.write('runner.py', f"""import json, pathlib, sys, time
if sys.argv[1]=='list':
    print(json.dumps({{'rust-suites':{{'fixture':{{'testcases':{{'smoke':{{}}}}}}}}}}))
else:
    pathlib.Path('ready').write_text('running')
    while not pathlib.Path('finish').exists(): time.sleep(.01)
    pathlib.Path('target/nextest/highgrade/junit.xml').write_text({XML!r})
    pathlib.Path('finished').write_text('done')
    sys.exit(1)
""")
        for interrupted in [False, True]:
            with self.subTest(interrupted=interrupted):
                self.passed_attempt()
                for name in ['ready', 'finish', 'finished']:
                    (self.root / name).unlink(missing_ok=True)
                child = subprocess.Popen([sys.executable, str(wrapper)], cwd=self.root,
                                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    deadline = time.monotonic() + 10
                    while not (self.root / 'ready').exists() and child.poll() is None and time.monotonic() < deadline:
                        time.sleep(.01)
                    self.assertTrue((self.root / 'ready').exists(), 'runner did not reach execution')
                    if interrupted:
                        child.terminate()
                        child.wait(timeout=5)
                    self.write('finish', 'finish and exit')
                    child.communicate(timeout=5)
                    self.assertNotEqual(child.returncode, 0)
                    deadline = time.monotonic() + 5
                    while not (self.root / 'finished').exists() and time.monotonic() < deadline:
                        time.sleep(.01)
                    self.assertTrue((self.root / 'finished').exists(), 'fixture child did not finish')
                    self.assertIn('incomplete', self.run_step('prepare', False)['message'])
                finally:
                    self.write('finish', 'cleanup fixture child')
                    if child.poll() is None:
                        child.kill()
                    child.communicate(timeout=5)

    def test_verify_orchestration_completes_only_after_success(self):
        with patch('sys.argv', ['verify.py', 'tests']), patch.object(verify, 'run') as run:
            verify.main()
        self.assertEqual([call.args[0][2] for call in run.call_args_list], ['capture', 'list', 'run', 'complete'])
        for error in [SystemExit(1), KeyboardInterrupt(), subprocess.TimeoutExpired('cargo', 1)]:
            with self.subTest(error=type(error).__name__), patch('sys.argv', ['verify.py', 'tests']), \
                    patch.object(verify, 'run', side_effect=[None, None, error]) as run:
                with self.assertRaises(type(error)):
                    verify.main()
                self.assertEqual(len(run.call_args_list), 3)


if __name__ == '__main__':
    unittest.main()
