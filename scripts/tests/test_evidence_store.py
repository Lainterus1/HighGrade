"""HG-0069-S2/S3: real selected export and conservative owned-run collection."""
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('evidence_store', Path(__file__).parents[1] / 'evidence_store.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)


class EvidenceStorageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def completed(self):
        run = m.begin(self.root, 'test')
        (self.root / run / 'result.log').write_text('observed result')
        (self.root / run / 'noise.log').write_text('temporary noise')
        m.state_write(self.root / run / 'run.json', {'schema': 1, 'owner_pid': None, 'state': 'awaiting_export', 'result': {'exit_code': 1}})
        return run

    def test_export_is_selected_idempotent_and_detects_corruption(self):
        run = self.completed(); files = [run + '/result.log']
        first = m.export(self.root, files, {'exit_code': 1})
        self.assertEqual(first, m.export(self.root, files, {'exit_code': 1}))
        manifest = m.verify(self.root, first)
        self.assertEqual(1, len(manifest['files']))
        self.assertEqual(1, manifest['metadata']['exit_code'])
        self.assertEqual(2, len(list((self.root / m.BASE).iterdir())))
        (self.root / manifest['files'][0]['path']).write_text('corrupt')
        with self.assertRaisesRegex(ValueError, 'hash mismatch'): m.verify(self.root, first)
        self.assertTrue((self.root / run / 'result.log').exists())

    def test_sweep_preserves_active_unknown_changed_and_unexported(self):
        ready = self.completed(); m.seal(self.root, ready, ['result.log'])
        active = m.begin(self.root, 'active')
        interrupted = self.completed()
        foreign = self.root / m.RUNS / 'foreign'; foreign.mkdir(); (foreign/'keep').write_text('user')
        (self.root/'docs/unrelated-diagnostic.json').write_text('')
        self.assertEqual([ready], m.sweep(self.root)['eligible'])
        self.assertTrue((self.root/ready).exists())
        self.assertEqual([], m.sweep(self.root, True, 'UI active')['removed'])
        (self.root / ready / 'empty').mkdir()
        self.assertEqual([], m.sweep(self.root, True)['removed'])
        (self.root / ready / 'empty').rmdir()
        (self.root / ready / 'added').write_text('later')
        self.assertEqual([], m.sweep(self.root, True)['removed'])
        (self.root / ready / 'added').unlink()
        self.assertEqual([ready], m.sweep(self.root, True)['removed'])
        self.assertEqual([], m.sweep(self.root, True)['removed'])
        for path in [active, interrupted, str(foreign.relative_to(self.root))]: self.assertTrue((self.root/path).exists())

    def test_damaged_export_blocks_collection_and_links_never_export(self):
        ready = self.completed(); name = m.seal(self.root, ready, ['result.log'])
        manifest = m.verify(self.root, name)
        (self.root / manifest['files'][0]['path']).unlink()
        self.assertEqual([], m.sweep(self.root, True)['removed'])
        with self.assertRaises(ValueError): m.export(self.root, ['../outside'], {})
        if os.name != 'nt':
            (self.root/'link').symlink_to(self.root / ready / 'result.log')
            with self.assertRaises(ValueError): m.export(self.root, ['link'], {})

    def test_catalog_reference_blocks_deleting_exported_source(self):
        run=self.completed(); m.seal(self.root,run,['result.log'])
        catalog=self.root/'specs/changes/fixture';catalog.mkdir(parents=True)
        for reference in [run+'/result.log',(run+'/result.log').replace('/',chr(92))]:
            (catalog/'results.json').write_text(json.dumps({'report':reference}))
            self.assertEqual([],m.sweep(self.root,True)['removed'])
        self.assertTrue((self.root/run/'result.log').exists())
        (catalog/'results.json').unlink()
        docs=self.root/'docs';docs.mkdir(exist_ok=True)
        (docs/'result.md').write_text('[run](../'+run+')')
        self.assertEqual([],m.sweep(self.root,True)['removed'])
        (docs/'result.md').unlink()
        custom=self.root/'custom';custom.mkdir()
        (custom/'results.json').write_text(json.dumps({'report':run+'/result.log'}))
        (self.root/'specs-location.json').write_text(json.dumps({'directory':'custom'}))
        self.assertEqual([],m.sweep(self.root,True)['removed'])

    def test_explicit_discard_keeps_referenced_run_but_exports_nothing(self):
        run=self.completed(); m.discard(self.root,run)
        self.assertFalse((self.root/m.BASE).exists())
        docs=self.root/'docs';docs.mkdir()
        (docs/'keep.md').write_text(run)
        self.assertEqual([],m.sweep(self.root,True)['removed'])
        (docs/'keep.md').unlink()
        self.assertEqual([run],m.sweep(self.root,True)['removed'])

    def test_manifest_corruption_and_live_owner_refuse_finish(self):
        run=self.completed()
        state=json.loads((self.root/run/'run.json').read_bytes()); state['owner_pid']=os.getpid()
        m.state_write(self.root/run/'run.json',state)
        with self.assertRaisesRegex(ValueError,'owner active'): m.seal(self.root,run,['result.log'])
        name=m.export(self.root,[run+'/result.log'],{})
        (self.root/name).write_text('{}')
        with self.assertRaisesRegex(ValueError,'Manifest hash'): m.verify(self.root,name)

    def test_interruption_after_export_before_inventory_preserves_scratch(self):
        run=self.completed()
        m.finish(self.root,run,['result.log'])
        self.assertEqual([],m.sweep(self.root,True)['removed'])
        self.assertTrue((self.root/run/'result.log').exists())

    def test_nested_native_run_record_is_evidence_but_wrapper_state_is_not(self):
        run = self.completed()
        folder = self.root / run / 'nextest'; folder.mkdir()
        (folder / 'run.json').write_text('{"tool":"rust-nextest"}')
        with self.assertRaisesRegex(ValueError, 'state is not evidence'):
            m.seal(self.root, run, ['run.json'])
        manifest = m.seal(self.root, run, ['nextest/run.json'])
        self.assertEqual(1, len(m.verify(self.root, manifest)['files']))
        self.assertEqual([run], m.sweep(self.root, True)['removed'])

    def test_real_wrapper_preserves_exit_code_and_finishes_selected_report(self):
        script=Path(m.__file__)
        code='import os,json,pathlib; state=json.loads((pathlib.Path(os.environ["HIGHGRADE_RUN_DIR"])/"run.json").read_text()); print("bound",os.environ["HIGHGRADE_RUN_TOKEN"]==state["run_token"]); raise SystemExit(3)'
        command=[sys.executable,str(script),'--root',str(self.root),'run','--name','real','--',sys.executable,'-c',code]
        result=subprocess.run(command,capture_output=True,text=True)
        self.assertEqual(3,result.returncode,result.stderr)
        value=json.loads(result.stdout.splitlines()[-1]); run=value['run']
        state=json.loads((self.root/run/'run.json').read_bytes())
        self.assertEqual('awaiting_export',state['state'])
        self.assertIsNone(state['owner_pid'])
        self.assertEqual('bound True', (self.root/run/'run.log').read_text().strip())
        name=m.seal(self.root,run,['run.log'])
        self.assertEqual(3,m.verify(self.root,name)['metadata']['exit_code'])
        self.assertNotIn('run_token',m.verify(self.root,name)['metadata'])
        self.assertEqual([run],m.sweep(self.root,True)['removed'])

    def test_background_descendant_is_not_marked_ready_for_collection(self):
        script=Path(m.__file__)
        code='import subprocess,sys; subprocess.Popen([sys.executable,"-c","import time;time.sleep(2)"])'
        result=subprocess.run([sys.executable,str(script),'--root',str(self.root),'run','--name','child','--',sys.executable,'-c',code],capture_output=True,text=True,check=True)
        run=json.loads(result.stdout.splitlines()[-1])['run']
        self.assertEqual('interrupted_descendants',json.loads((self.root/run/'run.json').read_bytes())['state'])
        with self.assertRaises(ValueError):m.seal(self.root,run,['run.log'])
        self.assertEqual([],m.sweep(self.root,True)['removed'])

if __name__ == '__main__': unittest.main()
