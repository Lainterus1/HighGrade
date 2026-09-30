"""Check registered document bytes after real cross-platform Git checkouts."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
DOCUMENT = 'docs/DEVELOPMENT.md'


class DocumentBudgetTests(unittest.TestCase):
    def test_development_budget_survives_git_checkout_line_endings(self):
        registry = json.loads((ROOT / '.highgrade/project/documents.json').read_text(encoding='utf-8'))
        budget = next(doc['budget'] for doc in registry['documents'] if doc['path'] == DOCUMENT)
        self.assertEqual(budget['unit'], 'bytes')
        self.assertTrue(budget['agreed'])
        source = (ROOT / DOCUMENT).read_bytes().replace(b'\r\n', b'\n')
        scratch = ROOT / 'target/highgrade/tmp'
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='document-budget-', dir=scratch) as temporary:
            root = Path(temporary)
            environment = dict(os.environ, GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)

            def git(*args):
                return subprocess.check_output(
                    ['git', '-c', f'core.attributesFile={os.devnull}', '-c', 'core.safecrlf=false', *args],
                    cwd=root, env=environment, stderr=subprocess.STDOUT)

            git('init', '-q')
            git('config', '--local', 'core.autocrlf', 'false')
            git('config', '--local', 'core.eol', 'lf')
            (root / 'docs').mkdir()
            (root / DOCUMENT).write_bytes(source)
            (root / '.gitattributes').write_bytes((ROOT / '.gitattributes').read_bytes())
            control = root / 'control.txt'
            control.write_bytes(b'checkout control\n')
            git('add', '.gitattributes', DOCUMENT, control.name)
            for autocrlf in ('true', 'false', 'input'):
                with self.subTest(autocrlf=autocrlf):
                    (root / DOCUMENT).unlink()
                    control.unlink()
                    git('-c', f'core.autocrlf={autocrlf}', 'checkout', '--', DOCUMENT, control.name)
                    # The unprotected file proves Git actually applied the requested conversion.
                    self.assertEqual(control.read_bytes(),
                                     b'checkout control\r\n' if autocrlf == 'true' else b'checkout control\n')
                    actual = (root / DOCUMENT).read_bytes()
                    self.assertLessEqual(len(actual), budget['ceiling'])
                    self.assertEqual(actual, source)


if __name__ == '__main__':
    unittest.main()
