"""Structural workflow checks, not evidence of an agent executing the procedures."""
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
KIT = ROOT / 'kit'
spec = importlib.util.spec_from_file_location('render_skills', ROOT / 'scripts/render-skills.py')
renderer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(renderer)


class WorkflowContractTests(unittest.TestCase):
    def test_canonical_commit_skill_is_rendered_without_a_second_approve_route(self):
        manifest = json.loads((KIT / 'manifest.json').read_text(encoding='utf-8'))['files']
        self.assertIn('procedures/commit.md', manifest)
        self.assertIn('skills/highgrade-commit/SKILL.md', manifest)
        self.assertNotIn('procedures/approve.md', manifest)
        self.assertNotIn('skills/highgrade-approve/SKILL.md', manifest)
        self.assertFalse((KIT / 'procedures/approve.md').exists())
        self.assertFalse((KIT / 'skills/highgrade-approve').exists())
        for path in (KIT / 'skills').glob('highgrade-*/SKILL.md'):
            with self.subTest(skill=path.parent.name):
                text = path.read_text(encoding='utf-8')
                self.assertEqual(text, renderer.render(path))
                self.assertIn(f'name: {path.parent.name}\n', text)
                self.assertNotIn('procedures/approve.md', text)

    def test_init_links_the_adaptation_that_commit_and_push_consume(self):
        init = (KIT / 'procedures/init.md').read_text(encoding='utf-8')
        self.assertIn('../templates/INSTRUCTIONS.md', init)
        self.assertIn('не переписывай принятые местные правила молча', init)
        template = (KIT / 'templates/INSTRUCTIONS.md').read_text(encoding='utf-8')
        for field in ('Git и идентичность результата', 'Pre-submit', 'Финальная готовность',
                      'highgrade-commit', 'highgrade-push', 'base/head', 'Последствия отправки',
                      'Решения и полномочия', 'build ID', 'squash/rebase/merge'):
            self.assertIn(field, template)
        for procedure in ('commit', 'push'):
            body = (KIT / f'procedures/{procedure}.md').read_text(encoding='utf-8')
            for boundary in ('pre-submit', 'Work', 'Git', 'SHA', 'приёмк'):
                self.assertIn(boundary, body)

    def test_planner_route_does_not_grant_goal_or_plan_only_execution(self):
        # A structural regression tripwire. Semantic review and actor observations
        # remain mandatory; this does not claim a real Goal API was exercised.
        task = (KIT / 'procedures/task.md').read_text(encoding='utf-8')
        template = (KIT / 'templates/AGENTS.md').read_text(encoding='utf-8')
        planner = (KIT / 'procedures/planner.md').read_text(encoding='utf-8')
        self.assertIn('Автоматический выбор Planner не разрешает Goal', task)
        self.assertIn('Запрос только на план не разрешает Goal или реализацию', task)
        self.assertIn('Принятие этого маршрута не разрешает создание Goal', template)
        self.assertIn('подготовь план без Goal и реализации', template)
        self.assertNotIn('это принятое проектом поручение на Goal', template)
        self.assertNotIn('ограничь Goal подготовкой плана', template)
        self.assertIn('сам по себе не разрешает создание Goal', planner)
        self.assertIn('запрос только на план не разрешают создавать Goal', planner)

    def test_all_current_kit_markdown_links_resolve_after_the_rename(self):
        for path in KIT.rglob('*.md'):
            if {'skills', 'templates'} & set(path.relative_to(KIT).parts):
                continue  # Skills use release paths; templates resolve in the adapted project.
            for target in re.findall(r'\]\(([^)]+)\)', path.read_text(encoding='utf-8')):
                if '://' in target or target.startswith('#'):
                    continue
                relative = target.split('#')[0]
                with self.subTest(source=str(path.relative_to(KIT)), target=relative):
                    self.assertTrue((path.parent / relative).exists())


if __name__ == '__main__':
    unittest.main()
