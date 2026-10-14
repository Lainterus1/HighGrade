"""Exercise archive integrity against later catalog retirement and unexplained loss."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ArchiveCurrencyTests(unittest.TestCase):
    def test_later_retirement_is_distinct_from_loss_or_reused_retired_id(self):
        scratch = ROOT / 'target/highgrade/tmp'
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='archive-currency-', dir=scratch) as temporary:
            root = Path(temporary)
            for relative in ['docs/archive/legacy-openspec', 'docs/archive/legacy-native-specs/2026-09-25', 'specs/requirements']:
                shutil.copytree(ROOT / relative, root / relative)
            (root / 'scripts').mkdir()
            shutil.copyfile(ROOT / 'scripts/check-spec-archive.mjs', root / 'scripts/check-spec-archive.mjs')
            shutil.copyfile(ROOT / 'specs/catalog.json', root / 'specs/catalog.json')

            def run():
                return subprocess.run(['node', 'scripts/check-spec-archive.mjs'], cwd=root,
                                      capture_output=True, text=True, timeout=15)

            initial = run()
            self.assertEqual(initial.returncode, 0, initial.stderr)
            self.assertGreater(json.loads(initial.stdout)['later_retirements'], 0)
            target = root / 'specs/requirements/HG-0009-R2.json'
            original = target.read_bytes()
            requirement = json.loads(original)
            scenario = requirement['scenarios'].pop(0)
            target.write_text(json.dumps(requirement))
            missing = run()
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn('scenario fate', missing.stderr)
            target.write_bytes(original)
            catalog_path = root / 'specs/catalog.json'
            catalog = json.loads(catalog_path.read_text())
            catalog['retired_ids'].append(scenario['id'])
            catalog_path.write_text(json.dumps(catalog))
            reused = run()
            self.assertNotEqual(reused.returncode, 0)
            self.assertIn('scenario fate', reused.stderr)


if __name__ == '__main__':
    unittest.main()
