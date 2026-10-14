from __future__ import annotations
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from types import SimpleNamespace
from contextlib import redirect_stdout
import io
SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from code_health.validation import load_policy, schema_for, validate, validate_report
from code_health.git_scope import discover_scope, snapshot_repository, compare_snapshots, acceptance_is_clean, ACCEPTANCE_PATH
from code_health.core import build_ast_index, make_measurement, sha256_text, source_kind
from code_health.acceptance import apply_acceptance, _parse_registry
from code_health.adapters import run_pylint
from test_workflows import pins_available
import test_workflows

POLICY = load_policy()


class ContractTests(unittest.TestCase):
    def test_failure_report_uses_effective_snapshot_policy(self):
        import audit
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            root = work/'repo'
            root.mkdir()
            (root/'build').mkdir()
            policy = work/'policy.json'
            policy.write_text('{"excluded_directories":[]}')
            output = work/'report'
            args = SimpleNamespace(root=root,policy=policy,output_dir=output,mode='full',preflight=False)
            def fail(_):
                (root/'build/new.txt').write_text('changed during failed audit')
                raise RuntimeError('injected failure')
            with mock.patch.object(audit,'parse_args',return_value=args), mock.patch.object(audit,'run_audit',side_effect=fail), redirect_stdout(io.StringIO()):
                self.assertEqual(audit.main(),3)
            report = json.loads((output/'code-health-audit.json').read_text())
            self.assertFalse(report['audit']['read_only_verified'])
            self.assertTrue(any(e['code']=='READ_ONLY_BREACH' for e in report['errors']))

    def test_non_git_policy_can_include_build_and_snapshot_new_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'build').mkdir()
            (root/'build/m.py').write_text('X = 1\n')
            policy = {**POLICY, 'excluded_directories': []}
            files, _ = discover_scope(root, policy)
            self.assertEqual(files, ['build/m.py'])
            before = snapshot_repository(root, files, policy)
            (root/'build/new.txt').write_text('new content')
            self.assertIn('build/new.txt', compare_snapshots(before, snapshot_repository(root, files, policy)))

    def test_policy_rejects_typo_bad_thresholds_and_escaping_paths(self):
        for override in ({'metrcs':{}}, {'metrics':{'line_coverage':{'watch':10}}}, {'include_paths':['../outside']}, {'watch_hotspot_limit':True}):
            with self.subTest(override=override), tempfile.TemporaryDirectory() as directory:
                path = Path(directory)/'policy.json'
                path.write_text(json.dumps(override))
                with self.assertRaises(ValueError):
                    load_policy(path)

    def test_scope_and_configured_test_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in ('src/a.py','src/generated/a.py','checks/contract.py','migrations/old.py'):
                path = root/relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('X = 1\n')
            policy = {**POLICY,'include_paths':['src','checks'],'excluded_paths':['src/generated'],'test_paths':['checks']}
            included, excluded = discover_scope(root,policy)
            self.assertEqual(included,['checks/contract.py','src/a.py'])
            self.assertEqual(len(excluded),2)
            self.assertEqual(source_kind('checks/contract.py',policy),'test')

    def test_acceptance_binds_effective_policy_and_low_direction(self):
        def measurement(value, policy=POLICY):
            return make_measurement(metric='line_coverage',value=value,path='m.py',symbol='<file>',start_line=1,end_line=2,
                                    analyzer={'name':'coverage','version':'7.15.4'}, evidence_fingerprint=sha256_text('stable'), evidence={},policy=policy)
        original = measurement(60)
        entry = {**{key:original[key] for key in ('metric','path','symbol','analyzer','evidence_fingerprint','policy_sha256')},
                 'finding_id':original['measurement_id'],'policy_version':POLICY['policy_version'],
                 'accepted_value':60,'ceiling':60,'reason':'Existing measured debt is accepted.'}
        def result(m, e=entry):
            return apply_acceptance(Path.cwd(),[m],[e],POLICY['policy_version'])[0][0]
        self.assertEqual(result(measurement(50))['reopen_reason'],'REGRESSION')
        self.assertTrue(result(measurement(65))['baseline_can_tighten'])
        changed = copy.deepcopy(POLICY)
        changed['metrics']['line_coverage']['watch'] = 90
        self.assertEqual(result(measurement(60,changed))['reopen_reason'],'POLICY_CHANGED')
        old_entry = {key:value for key,value in entry.items() if key != 'policy_sha256'}
        self.assertEqual(result(original,old_entry)['reopen_reason'],'POLICY_CHANGED')

    def test_registry_rejects_unknown_metric_and_unsafe_guard(self):
        from test_core import AcceptanceTests
        original = AcceptanceTests()._measurement(fingerprint=sha256_text('stable'))
        entry = AcceptanceTests()._entry(original)
        for override in ({'metric':'imaginary'}, {'guard_files':[{'path':'../secret','sha256':'a'*64}]}):
            entries,errors = _parse_registry(json.dumps({'schema_version':'1.0.0','entries':[{**entry,**override}]}).encode(),'fixture')
            self.assertEqual(entries,[])
            self.assertTrue(errors)

    def test_repeated_method_names_have_unique_ids(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'m.py').write_text('class C:\n    @property\n    def x(self): return 1\n    @x.setter\n    def x(self, value): pass\n')
            index, errors = build_ast_index(root,['m.py'])
            self.assertEqual(errors,[])
            self.assertEqual([s['symbol'] for s in index['m.py']['symbols']],['C.x','C.x#2'])

    def test_schema_rejects_wrong_measurement_types_and_nonfinite_numbers(self):
        schema = schema_for('report')['$defs']['measurement']
        for value in (False, float('nan'), '10'):
            with self.assertRaises(ValueError):
                validate({'value':value},schema)

    def test_snapshot_detects_second_dirty_edit_new_file_and_staged_registry(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.run(['git',*args],cwd=root,check=True,capture_output=True)
            git('init','-b','main')
            path = root/ACCEPTANCE_PATH
            path.parent.mkdir()
            original = '{"schema_version":"1.0.0","entries":[]}\n'
            path.write_text(original)
            (root/'README.md').write_text('initial')
            git('add','.')
            git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-m','fixture')
            self.assertTrue(acceptance_is_clean(root))
            path.write_text(original+' ')
            git('add',ACCEPTANCE_PATH)
            path.write_text(original)
            self.assertFalse(acceptance_is_clean(root))
            (root/'README.md').write_text('first dirty edit')
            before = snapshot_repository(root,[])
            (root/'README.md').write_text('second dirty edit')
            (root/'new.py').write_text('X = 1\n')
            changed = compare_snapshots(before,snapshot_repository(root,[]))
            self.assertIn('README.md',changed)
            self.assertIn('new.py',changed)


@unittest.skipUnless(pins_available(),'requires pinned analyzers')
class NamespaceTests(unittest.TestCase):
    def test_namespace_from_import_cycle_is_not_zero(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = ['src/pkg/a.py','src/pkg/b.py']
            for relative,other in zip(files,['b','a']):
                path = root/relative
                path.parent.mkdir(parents=True,exist_ok=True)
                path.write_text(f'from pkg import {other}\n\ndef value():\n    return {other}.value()\n')
            index,_ = build_ast_index(root,files)
            result = run_pylint(root,files,index,{},POLICY,'4.0.7')
            self.assertEqual(result['tool']['status'],'OK',result)
            self.assertTrue(result['cycle_signatures'],result)
            cycle = next(m for m in result['measurements'] if m['metric']=='dependency_cycle')
            self.assertEqual(set(cycle['evidence']['paths']),set(files))


@unittest.skipUnless(pins_available(),'requires pinned analyzers')
class CaptureReadOnlyTest(unittest.TestCase):
    def test_test_command_changing_readme_cannot_be_attested(self):
        fixture = test_workflows.WorkflowTests()
        fixture.setUp()
        try:
            fixture.write('README.md','dirty already')
            fixture.write('test_m.py','from pathlib import Path\nfrom m import f\nassert f(1)==2\nPath("README.md").write_text("changed again")\n')
            cap,_ = fixture.capture(expected=2)
            evidence = json.loads((cap/'coverage-provenance.json').read_text())
            self.assertIn('README.md',evidence['changed_paths'])
            self.assertEqual(evidence['status'],'FAILED')
        finally:
            fixture.doCleanups()
