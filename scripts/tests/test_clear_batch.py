"""Real processes and byte fixtures for HG-0076-S5..S10."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

HELPER = Path(__file__).resolve().parents[2] / 'kit/tools/clear_batch.py'
ADAPTER = r'''import hashlib, json, os, sys, unittest
from pathlib import Path
MODE = 'normal'
raw = Path('calc.py').read_bytes()
hashes = {'calc.py': hashlib.sha256(raw).hexdigest()}
namespace = {}
cases = []
try:
    exec(compile(raw, 'calc.py', 'exec'), namespace)
    class Test(unittest.TestCase):
        def test_number(self):
            if MODE == 'badbaseline': self.fail('broken baseline fixture')
            if MODE == 'skip' or (MODE == 'skipmutant' and b'return 0' in raw): self.skipTest('unavailable')
            if MODE == 'weak': self.assertGreaterEqual(namespace['number'](), 0)
            else: self.assertEqual(namespace['number'](), 2, 'number contract')
    class Result(unittest.TestResult):
        def addSuccess(self, test):
            super().addSuccess(test); cases.append({'id':'test_number','outcome':'passed','message':''})
        def addFailure(self, test, err):
            super().addFailure(test,err); cases.append({'id':'test_number','outcome':'assertion','message':str(err[1])})
        def addError(self, test, err):
            super().addError(test,err); cases.append({'id':'test_number','outcome':'error','message':str(err[1])})
        def addSkip(self, test, reason):
            super().addSkip(test,reason); cases.append({'id':'test_number','outcome':'skipped','message':reason})
    result = Result()
    if MODE != 'empty' and not (MODE == 'emptymutant' and b'return 0' in raw): unittest.defaultTestLoader.loadTestsFromTestCase(Test).run(result)
    if MODE == 'timeout' and b'return 0' in raw:
        import time; time.sleep(4)
    if MODE == 'setup' and b'return 0' in raw: raise RuntimeError('setup unavailable')
    if MODE == 'restore' and b'return 0' in raw: Path('calc.py').write_bytes(b'foreign edit\x00')
    if MODE == 'stale' and b'return 0' in raw:
        hashes['calc.py'] = hashlib.sha256(raw.replace(b'return 0', b'return 2')).hexdigest()
    report = {'format':'highgrade-clear-v1','tests':cases,'source_hashes':hashes}
    if MODE == 'unsupported' and b'return 0' in raw: report['format'] = 'other'
    if MODE == 'descendant' and b'return 0' in raw:
        import subprocess; subprocess.Popen([sys.executable,'-c','import time; time.sleep(4)'])
except BaseException as error:
    report = {'format':'highgrade-clear-v1','tests':[{'id':'test_number','outcome':'error','message':str(error)}],'source_hashes':hashes}
Path(os.environ['HG_CLEAR_REPORT']).write_text(json.dumps(report))
sys.exit(0 if all(c['outcome']=='passed' for c in report['tests']) else 1)
'''


class ClearBatchTests(unittest.TestCase):
    def setUp(self):
        preparation_start=time.monotonic()
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        if os.environ.get('HG_CLEAR_ARTIFACTS'):
            destination=Path(os.environ['HG_CLEAR_ARTIFACTS'])/self._testMethodName
            self.addCleanup(lambda: shutil.copytree(self.root,destination))
        self.helper = self.root / 'delivery/clear_batch.py'
        self.helper.parent.mkdir()
        shutil.copyfile(HELPER, self.helper)
        self.project = self.root / 'project'
        self.project.mkdir()
        for name, data in {'calc.py':b'def number():\r\n    return 2\r\n',
                           'adapter.py':ADAPTER.encode(), 'contract.txt':b'number must equal 2',
                           'data.txt':b'fixture', 'dependency.txt':b'v1', 'sentinel':b'foreign\x00\r\n'}.items():
            (self.project / name).write_bytes(data)
        self.plan = {'schema':1, 'inputs':['calc.py','adapter.py','contract.txt','data.txt','dependency.txt','sentinel'],
                     'tests':['adapter.py'], 'contracts':['contract.txt'], 'scope_complete':True,
                     'timeout_seconds':1, 'runtime_probes':[[sys.executable,'--version']],
                     'checks':[{'id':'number','command':[sys.executable,'-B','adapter.py'],
                                'expected_tests':['test_number'], 'sources':['calc.py']}],
                     'mutations':[self.mutation(i, v) for i, v in enumerate([0,3,4])]}
        self.preparation_seconds=time.monotonic()-preparation_start

    def mutation(self, i, value):
        return {'id':f'm{i}', 'path':'calc.py','before':'return 2','after':f'return {value}',
                'checks':['number'], 'failures':[{'test':'test_number','contains':'number contract'}],
                'requirement':'fixture-R1','scenario':'fixture-S1'}

    def mode(self, value):
        (self.project / 'adapter.py').write_text(ADAPTER.replace("MODE = 'normal'", f'MODE = {value!r}'))

    def run_batch(self, output='batch', reuse=None, helper=None):
        (self.project / 'plan.json').write_text(json.dumps(self.plan))
        args = [sys.executable,str(helper or self.helper),'--root',str(self.project),'--plan','plan.json','--output',output]
        if reuse: args += ['--reuse',reuse]
        process = subprocess.run(args,capture_output=True,text=True,timeout=20)
        path = self.project / output / 'summary.json'
        value = json.loads(path.read_text()) if path.exists() else None
        return process, value

    def test_standalone_batch_restores_exact_bytes_and_detects_actual_assertions(self):
        before = {p.name:p.read_bytes() for p in self.project.iterdir()}
        process, result = self.run_batch()
        self.assertEqual(0,process.returncode,process.stderr)
        self.assertEqual(['detected']*3,[m['outcome'] for m in result['mutations']])
        self.assertEqual(5,result['metrics']['commands'])
        self.assertEqual(['test_number'],result['baseline'][0]['executed_tests'])
        self.assertTrue(result['restored'])
        self.assertEqual(json.loads((self.project/'plan.json').read_text()),json.loads((self.project/'batch/plan.json').read_text()))
        for name,data in before.items(): self.assertEqual(data,(self.project/name).read_bytes())
        for ref,digest in result['originals'].items():
            self.assertEqual(digest,hashlib.sha256((self.project/'batch'/ref).read_bytes()).hexdigest())
        self.assertEqual(1,len(list((self.project/'batch').glob('snapshot.json'))))
        self.assertLess((self.project/'batch'/'summary.json').stat().st_size,result['metrics']['evidence_bytes'])

    def test_survivor_then_stronger_assertion_keeps_initial_history(self):
        self.plan['mutations']=self.plan['mutations'][:1]
        self.mode('weak'); process, first = self.run_batch()
        self.assertEqual('survived',first['mutations'][0]['outcome'])
        original=(self.project/'batch'/'summary.json').read_bytes()
        self.mode('normal'); process, second=self.run_batch('strong')
        self.assertEqual('detected',second['mutations'][0]['outcome'])
        self.assertTrue(second['complete'])
        self.assertTrue(second['final'])
        self.assertTrue(all(check['outcome']=='passed' for check in second['final']))
        self.assertEqual(original,(self.project/'batch'/'summary.json').read_bytes())
        self.assertEqual(first['mutations'][0]['diff_sha256'],second['mutations'][0]['diff_sha256'])
        self.assertTrue(second['restored'])

    def test_invalid_baseline_does_not_assess_mutations(self):
        for mode in ['skip','empty']:
            with self.subTest(mode=mode):
                self.mode(mode); _, result=self.run_batch(mode)
                self.assertEqual([],result['mutations'])
                self.assertFalse(result['complete'])
        self.mode('badbaseline'); _,result=self.run_batch('badbaseline')
        self.assertEqual([],result['mutations'])
        self.assertEqual(1,result['metrics']['commands'])

    def test_non_assertion_failures_never_detect(self):
        self.plan['mutations']=self.plan['mutations'][:1]
        for mode in ['setup','timeout','unsupported','stale','skipmutant','emptymutant']:
            with self.subTest(mode=mode):
                self.mode(mode); _,result=self.run_batch(mode)
                self.assertEqual('invalid',result['mutations'][0]['outcome'])
                self.assertTrue(all(check['reason'] for check in result['mutations'][0]['checks']))
                self.assertTrue(result['restored'])
        self.mode('normal'); self.plan['mutations'][0]['after']='return ('
        _,result=self.run_batch('compile')
        self.assertEqual('invalid',result['mutations'][0]['outcome'])

    def test_descendants_cannot_turn_a_failure_into_detection_or_continue_worker(self):
        self.mode('descendant');_,result=self.run_batch()
        self.assertEqual(1,len(result['mutations']))
        self.assertEqual('invalid',result['mutations'][0]['outcome'])
        self.assertFalse(result['complete'])

    def test_unknown_restore_edit_stops_worker_and_preserves_live_and_foreign(self):
        self.mode('restore'); _,result=self.run_batch()
        self.assertEqual(1,len(result['mutations']))
        self.assertEqual('restore_failed',result['mutations'][0]['outcome'])
        self.assertEqual([],result['final'])
        self.assertEqual(b'foreign edit\x00',(self.project/'batch/subject/calc.py').read_bytes())
        self.assertEqual(b'def number():\r\n    return 2\r\n',(self.project/'calc.py').read_bytes())
        self.assertEqual(b'foreign\x00\r\n',(self.project/'batch/subject/sentinel').read_bytes())

    def test_existing_output_and_unsafe_inputs_do_not_overwrite(self):
        (self.project/'batch').mkdir(); (self.project/'batch/foreign').write_bytes(b'keep')
        process,_=self.run_batch(); self.assertNotEqual(0,process.returncode)
        self.assertEqual(b'keep',(self.project/'batch/foreign').read_bytes())
        for bad in ['../foreign','/absolute','calc.py/../sentinel']:
            with self.subTest(bad=bad):
                self.plan['inputs'].append(bad)
                process,_=self.run_batch('unsafe'); self.assertNotEqual(0,process.returncode)
                self.plan['inputs'].pop()
        self.plan['jobs']=2
        process,_=self.run_batch('shared'); self.assertNotEqual(0,process.returncode)

    def test_windows_trailing_path_aliases_are_rejected_before_output(self):
        namespace = {'__name__': 'clear_path_guard', '__file__': str(HELPER)}
        exec(compile(HELPER.read_bytes(), str(HELPER), 'exec'), namespace)
        original = (self.project / 'adapter.py').read_bytes()
        for name in ['adapter.py.', 'adapter.py ', 'folder./calc.py', 'folder /calc.py']:
            with self.subTest(name=name):
                with self.assertRaisesRegex(ValueError, 'nonportable path'):
                    namespace['safe'](self.project, name)
        for i, name in enumerate(['adapter.py.', 'adapter.py ']):
            with self.subTest(cli=name):
                if os.name != 'nt':
                    (self.project / name).write_bytes(original)
                self.plan['inputs'].append(name)
                process, _ = self.run_batch(f'alias-{i}')
                self.assertEqual(2, process.returncode, process.stderr)
                self.assertIn('nonportable path', process.stderr)
                self.assertFalse((self.project / f'alias-{i}').exists())
                self.assertEqual(original, (self.project / 'adapter.py').read_bytes())
                self.plan['inputs'].pop()

    def test_case_colliding_inputs_cannot_alias_protected_tests(self):
        name = 'Adapter.py'
        original = (self.project / 'adapter.py').read_bytes()
        if os.name != 'nt':
            (self.project / name).write_bytes(original)
        self.plan['inputs'].append(name)
        process, _ = self.run_batch('case-alias')
        self.assertEqual(2, process.returncode, process.stderr)
        self.assertIn('nonportable', process.stderr)
        self.assertFalse((self.project / 'case-alias').exists())
        self.assertEqual(original, (self.project / 'adapter.py').read_bytes())

    def test_owned_report_case_aliases_are_rejected_before_output(self):
        for i, name in enumerate(['.hg-clear-report.json', '.HG-CLEAR-REPORT.JSON', '.Hg-Clear-Report.json']):
            with self.subTest(name=name):
                protected = self.project / name
                original = b'preserve owned-name input\x00\r\n'
                protected.write_bytes(original)
                self.plan['inputs'].append(name)
                try:
                    process, _ = self.run_batch(f'owned-report-{i}')
                    self.assertEqual(2, process.returncode, process.stderr)
                    self.assertIn('input conflicts with owned report', process.stderr)
                    self.assertFalse((self.project / f'owned-report-{i}').exists())
                    self.assertEqual(original, protected.read_bytes())
                finally:
                    self.plan['inputs'].pop()
                    protected.unlink()

    def test_reuse_requires_complete_matching_causal_set_and_intact_originals(self):
        _,first=self.run_batch()
        _,same=self.run_batch('same','batch/summary.json')
        self.assertTrue(same['reused']); self.assertEqual(0,same['metrics']['commands'])
        for entry in same['baseline']+same['final']:
            self.assertTrue((self.project/'same'/entry['original']).is_file())
        for mutation in same['mutations']:
            self.assertTrue((self.project/'same'/mutation['diff']).is_file())
        for name in ['contract.txt','data.txt','dependency.txt','adapter.py']:
            with self.subTest(name=name):
                p=self.project/name; original=p.read_bytes(); p.write_bytes(original+b'\n')
                _,changed=self.run_batch(name.replace('.','-'),'batch/summary.json')
                self.assertFalse(changed['reused']); self.assertGreater(changed['metrics']['commands'],0)
                p.write_bytes(original)
        original=self.helper.read_bytes(); self.helper.write_bytes(original+b'\n# changed\n')
        _,changed=self.run_batch('helper','batch/summary.json'); self.assertFalse(changed['reused'])
        self.helper.write_bytes(original)
        with mock.patch.dict(os.environ,{'HG_CLEAR_FIXTURE_DEPENDENCY':'changed'}):
            _,changed=self.run_batch('env','batch/summary.json'); self.assertFalse(changed['reused'])
        self.plan['scope_complete']=False
        _,unknown=self.run_batch('unknown','batch/summary.json'); self.assertFalse(unknown['reused'])
        self.plan['scope_complete']=True
        ref=next(iter(first['originals']))
        (self.project/'batch'/ref).write_bytes(b'corrupt')
        _,corrupt=self.run_batch('corrupt','batch/summary.json'); self.assertFalse(corrupt['reused'])

    def test_measured_control_and_batch_preserve_same_outcomes(self):
        start=time.monotonic(); control=[]
        mutations=self.plan['mutations']
        for i,mutation in enumerate(mutations):
            self.plan['mutations']=[mutation]; _,value=self.run_batch(f'control-{i}'); control.append(value)
        control_time=time.monotonic()-start
        self.plan['mutations']=mutations
        start=time.monotonic(); _,batch=self.run_batch('batch'); batch_time=time.monotonic()-start
        self.assertEqual([v['mutations'][0]['outcome'] for v in control],[v['outcome'] for v in batch['mutations']])
        self.assertEqual(9,sum(v['metrics']['commands'] for v in control))
        self.assertEqual(5,batch['metrics']['commands'])
        comparison={'control_commands':9,'batch_commands':5,
                    'control_probe_commands':sum(v['metrics']['probe_commands'] for v in control),'batch_probe_commands':batch['metrics']['probe_commands'],
                    'control_helper_invocations':3,'batch_helper_invocations':1,
                    'fixture_preparation_seconds':self.preparation_seconds,'adapter_bytes':len(ADAPTER.encode()),
                    'adapter_authoring_cost':'нет данных','helper_development_cost':'нет данных',
                    'control_seconds':control_time,'batch_seconds':batch_time,
                    'control_bytes':sum(v['metrics']['evidence_bytes'] for v in control),'batch_bytes':batch['metrics']['evidence_bytes'],
                    'outcomes':['detected']*3,'tokens':'нет данных','retries':0,'limits':'minimal Python fixture; bytes count preserved originals, not working copies; measured time includes helper/probes; no net development-cost or universal saving forecast'}
        if os.environ.get('HG_CLEAR_COMPARISON'):
            Path(os.environ['HG_CLEAR_COMPARISON']).write_text(json.dumps(comparison,ensure_ascii=False,indent=2))


if __name__ == '__main__':
    unittest.main()
