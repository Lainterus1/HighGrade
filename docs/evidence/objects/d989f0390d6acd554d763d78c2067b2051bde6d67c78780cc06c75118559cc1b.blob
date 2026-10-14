from __future__ import annotations
import json
from pathlib import Path
import sys
import tempfile
import unittest
SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from code_health.core import build_ast_index
from code_health.adapters import run_pylint, _module_map, _duplicate_locations, _scan_suppressions
from audit import filter_diff_measurements
from test_workflows import POLICY, pins_available


class MetricTests(unittest.TestCase):
    def test_explicit_else_if_is_nested_and_elif_is_flat(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'm.py').write_text('def nested(a,b):\n    if a:\n        pass\n    else:\n        if b:\n            pass\n\ndef flat(a,b):\n    if a:\n        pass\n    elif b:\n        pass\n')
            index, errors = build_ast_index(root, ['m.py'])
            self.assertEqual(errors, [])
            self.assertEqual({s['symbol']:s['nesting_depth'] for s in index['m.py']['symbols']}, {'nested':2,'flat':1})

    def test_missing_pylint_never_manufactures_zero_diff_measurements(self):
        self.assertEqual(filter_diff_measurements([], {'m.py':[1]}, set(), POLICY, '4.0.7'), [])

    def test_module_aliases_resolve_only_unambiguous_scoped_paths(self):
        mapping = _module_map(['src/pkg/a.py', 'other/pkg/a.py'])
        self.assertNotIn('pkg.a', mapping)
        self.assertEqual(_duplicate_locations('==pkg.a:[0:2]', mapping), [])

    def test_only_real_required_suppression_comments_are_findings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'m.py').write_text('text = "# pylint: disable=all"\n# pylint: disable=missing-function-docstring\n# pylint: disable-next=R0801\nx = 1\n# pylint: disable=cyclic-import\n# pylint: skip-file\n')
            index,_ = build_ast_index(root,['m.py'])
            measurements = _scan_suppressions(root,['m.py'],index,POLICY)
            self.assertEqual([m['start_line'] for m in measurements], [3,5,6])
            self.assertEqual(len({m['measurement_id'] for m in measurements}), 3)


@unittest.skipUnless(pins_available(), 'requires pinned analyzers')
class PylintMetricsTests(unittest.TestCase):
    def run_sample(self, root, prefix='', comments=False):
        files = []
        for name in ('a','b'):
            path = f'{prefix}{name}.py'
            target = root/path
            target.parent.mkdir(parents=True,exist_ok=True)
            body = ''.join(f'    x{i} = {i}\n' + ('    # note\n\n'*4 if comments else '') for i in range(8))
            target.write_text(f'def {name}():\n{body}    return x0\n')
            files.append(path)
        index,_ = build_ast_index(root,files)
        result = run_pylint(root,files,index,{},POLICY,'4.0.7')
        self.assertEqual(result['tool']['status'],'OK',result)
        return result

    def test_comments_do_not_inflate_duplication_count_or_percent(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            plain=self.run_sample(root)
            commented=self.run_sample(root,comments=True)
            values=lambda result:{m['metric']:m['value'] for m in result['measurements']}
            self.assertEqual(values(plain),values(commented))
            self.assertEqual(values(commented)['duplication_percent'],100)
            self.assertEqual(values(commented)['duplication_block_lines'],9)

    def test_src_and_namespace_paths_exist_and_diff_keeps_duplicate(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            result=self.run_sample(root,prefix='src/pkg/')
            for m in result['measurements']:
                for loc in m.get('evidence',{}).get('locations',[]):
                    self.assertTrue((root/loc['path']).is_file())
            scoped=filter_diff_measurements(result['measurements'],{'src/pkg/a.py':[2]},set(),POLICY,'4.0.7',True,True)
            self.assertTrue(any(m['metric']=='duplication_block_lines' for m in scoped))
            self.assertEqual(next(m['value'] for m in scoped if m['metric']=='duplication_percent'),100)

    def test_unrelated_pylint_disable_does_not_make_analysis_partial(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'m.py').write_text('# pylint: disable=missing-function-docstring\ndef f():\n    return 1\n')
            index,_=build_ast_index(root,['m.py'])
            result=run_pylint(root,['m.py'],index,{},POLICY,'4.0.7')
            self.assertEqual(result['tool']['status'],'OK',result)

    def test_unlisted_file_is_not_analyzed_by_recursive_directory_expansion(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            result=self.run_sample(root,prefix='src/pkg/')
            (root/'src/pkg/bad.py').write_text('def invalid(:\n')
            index,_=build_ast_index(root,['src/pkg/a.py','src/pkg/b.py'])
            result=run_pylint(root,['src/pkg/a.py','src/pkg/b.py'],index,{},POLICY,'4.0.7')
            self.assertEqual(result['tool']['status'],'OK',result)


if __name__=='__main__':
    unittest.main()
