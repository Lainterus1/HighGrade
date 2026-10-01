"""Check stage-specific entry instructions and single-source skill rendering."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import tempfile


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('render_skills', ROOT / 'scripts/render-skills.py')
RENDERER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RENDERER)


class RenderSkillsTests(unittest.TestCase):
    def test_update_entry_does_not_require_a_project(self):
        rendered = RENDERER.render(ROOT / 'kit/skills/highgrade-update/SKILL.md')
        entry = rendered.split('# Обновление общей поставки', 1)[0]
        self.assertIn('Обновление общей установки не требует проекта.', entry)
        self.assertIn('Если работа идёт в проекте, прочитай его AGENTS', entry)
        self.assertIn('при их наличии', entry)
        self.assertIn('их отсутствие не блокирует Update.', entry)
        self.assertIn('`global-status --profile <профиль>`', entry)
        self.assertNotIn('возможностью отката', entry)
        self.assertIn('восстановлением после сбоя', entry)

    def test_other_skills_keep_their_project_entry(self):
        paths = list((ROOT / 'kit/skills').glob('highgrade-*/SKILL.md'))
        self.assertGreater(len(paths), 1)
        for path in paths:
            if path.parent.name == 'highgrade-update':
                continue
            with self.subTest(skill=path.parent.name):
                rendered = RENDERER.render(path)
                self.assertIn('На первом входе прочитай `rules.md` выпуска, AGENTS '
                              'и местную инструкцию проекта; проверь', rendered)
                self.assertIn('Отсутствие проектной инструкции при init ожидаемо.', rendered)
                self.assertNotIn('их отсутствие не блокирует Update.', rendered)
                self.assertEqual(rendered, path.read_text(encoding='utf-8-sig'))

    def test_update_body_and_metadata_come_from_their_editable_sources(self):
        scratch = ROOT / 'target/highgrade/tmp'
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='render-skills-', dir=scratch) as temporary:
            kit = Path(temporary)
            skill = kit / 'skills/highgrade-update/SKILL.md'
            skill.parent.mkdir(parents=True)
            (kit / 'procedures').mkdir()
            skill.write_text('---\nname: highgrade-update\ndescription: test metadata\n'
                             '---\nObsolete generated body\n', encoding='utf-8')
            procedure = kit / 'procedures/update.md'
            procedure.write_text('# Test instruction\n[Reference](../references/example.md)\n',
                                 encoding='utf-8')
            with patch.object(RENDERER, 'KIT', kit):
                first = RENDERER.render(skill)
                self.assertIn('description: test metadata', first)
                self.assertIn('# Test instruction', first)
                self.assertIn('`references/example.md` внутри активного выпуска', first)
                self.assertNotIn('Obsolete generated body', first)
                procedure.write_text('# Revised instruction\n', encoding='utf-8')
                second = RENDERER.render(skill)
                self.assertIn('# Revised instruction', second)
                self.assertNotIn('# Test instruction', second)


if __name__ == '__main__':
    unittest.main()
