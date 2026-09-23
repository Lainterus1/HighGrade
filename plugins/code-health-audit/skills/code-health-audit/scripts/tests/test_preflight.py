from __future__ import annotations
import json
import os
from pathlib import Path
import sys
import time
import unittest
from unittest import mock
import importlib.metadata
import re
SCRIPTS=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(SCRIPTS))
from code_health.acceptance import apply_acceptance
from code_health.git_scope import snapshot_repository, compare_snapshots
from code_health.validation import validate_report
from test_workflows import pins_available
import test_workflows
import test_core


class RuntimeAcceptanceTests(unittest.TestCase):
    def test_runtime_change_reopens_otherwise_identical_finding(self):
        helper=test_core.AcceptanceTests()
        measurement=helper._measurement()
        measurement['runtime_sha256']='a'*64
        entry=helper._entry(measurement)
        entry['runtime_sha256']='b'*64
        findings,_=apply_acceptance(Path.cwd(),[measurement],[entry],test_workflows.POLICY['policy_version'])
        self.assertEqual(findings[0]['reopen_reason'],'RUNTIME_CHANGED')


@unittest.skipUnless(pins_available(),'requires pinned analyzers')
class PreflightWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.fixture=test_workflows.WorkflowTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)

    def preflight(self,*extra,expected=0):
        fixture=self.fixture
        result=fixture.run_cmd([sys.executable,'-B',str(SCRIPTS/'audit.py'),'--root',str(fixture.root),'--mode','full','--preflight',*extra],expected)
        return json.loads(result.stdout)

    def test_active_dependency_closure_is_fully_pinned(self):
        # Test-only marker parser; the runner itself needs neither packaging nor pip.
        try:
            from packaging.requirements import Requirement
        except ImportError:
            from pip._vendor.packaging.requirements import Requirement
        toolchain=json.loads((SCRIPTS/'data/toolchain.json').read_text())
        declared={**toolchain['tools'],**toolchain['dependencies']}
        found={}
        queue=list(toolchain['tools'])
        while queue:
            name=re.sub(r'[-_.]+','-',queue.pop(0)).lower()
            if name in found:
                continue
            found[name]=importlib.metadata.version(name)
            for raw in importlib.metadata.requires(name) or []:
                req=Requirement(raw)
                if req.marker is None or req.marker.evaluate({'extra':''}):
                    self.assertFalse(req.extras,'explicit extras require a reviewed dependency closure')
                    queue.append(req.name)
        self.assertEqual(found,declared)
        requirements=dict(line.split('==') for line in (SCRIPTS/'data/requirements-analyzers.txt').read_text().splitlines() if line)
        self.assertEqual(requirements,declared)

    def test_transitive_dependency_drift_changes_runtime_identity(self):
        from code_health import runtime
        runtime.runtime_evidence.cache_clear()
        original=runtime.runtime_evidence()
        real_version=runtime.package_version
        try:
            with mock.patch.object(runtime,'package_version',side_effect=lambda name:'0.0.0' if name=='rich' else real_version(name)):
                runtime.runtime_evidence.cache_clear()
                changed=runtime.runtime_evidence()
            self.assertFalse(changed['validated'])
            self.assertNotEqual(original['sha256'],changed['sha256'])
        finally:
            runtime.runtime_evidence.cache_clear()

    def test_preflight_reports_runtime_scope_without_changing_repository(self):
        fixture=self.fixture
        before=snapshot_repository(fixture.root,[])
        result=self.preflight()
        self.assertTrue(result['ready'])
        self.assertEqual(result['runtime']['executable'],sys.executable)
        self.assertEqual(result['scope']['python_files'],['m.py','test_m.py'])
        self.assertEqual(compare_snapshots(before,snapshot_repository(fixture.root,[])),[])

    def test_preflight_missing_base_is_actionable(self):
        result=self.preflight('--mode','diff','--base','absent-base',expected=2)
        self.assertFalse(result['ready'])
        self.assertIn('no local base',result['errors'][0])

    def test_diff_real_changed_line_coverage_and_stable_digest(self):
        fixture=self.fixture
        fixture.write('m.py','def f(x):\n    if x > 0:\n        return x + 1\n    return 0\n')
        fixture.write('test_m.py','from m import f\nassert f(1)==2\nassert f(0)==0\n')
        cap,_=fixture.capture()
        args=['--mode','diff','--coverage-json',str(cap/'coverage.json'),'--coverage-provenance',str(cap/'coverage-provenance.json')]
        first=fixture.audit(args)
        second=fixture.audit(args)
        validate_report(first)
        self.assertEqual(first['audit']['completeness'],'COMPLETE')
        self.assertEqual(first['scope']['diff_branch_coverage'],'NOT_MEASURED')
        self.assertFalse(any(m['metric']=='branch_coverage' for m in first['measurements']))
        self.assertTrue(any(m['metric']=='line_coverage' and m['value']==100 for m in first['measurements']))
        self.assertEqual(first['audit']['content_digest'],second['audit']['content_digest'])

    def test_touch_without_content_change_keeps_observed_coverage_valid(self):
        fixture=self.fixture
        cap,_=fixture.capture()
        os.utime(fixture.root/'m.py',(time.time()+60,time.time()+60))
        report=fixture.audit(['--coverage-json',str(cap/'coverage.json'),'--coverage-provenance',str(cap/'coverage-provenance.json')])
        self.assertEqual(report['audit']['completeness'],'COMPLETE')

    def test_incomplete_provenance_cannot_attest_coverage(self):
        fixture=self.fixture
        cap,_=fixture.capture()
        path=cap/'coverage-provenance.json'
        data=json.loads(path.read_text())
        data.pop('auto_configs')
        path.write_text(json.dumps(data))
        report=fixture.audit(['--coverage-json',str(cap/'coverage.json'),'--coverage-provenance',str(path)],expected=2)
        self.assertTrue(any(e['code']=='COVERAGE_PROVENANCE_UNVERIFIED' for e in report['errors']))
        self.assertFalse(any(m['metric'].endswith('coverage') for m in report['measurements']))
