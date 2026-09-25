# HG-0009-S18 — слепой поиск и локальная адаптация s5

## Итог

Ложный кандидат `legacy/architecture.md` отклонён после чтения: он описывает офисное помещение и прямо не относится к программе. Поиск содержания нашёл `ledger/part-b.txt`; его проверенные сведения о `total.py` и `orders.json` перенесены в `docs/ARCHITECTURE.md`. Известное вычисление отделено от открытых решений об округлении, разделителе CSV и часовом поясе. Исходник, ложный кандидат, код, тест и фикстура сохранены побайтово. Внешние снимки и manifest из `docs/evidence/agent-pilot/hg9-false-s5` не читались.

## Фактическая последовательность

1. Прочитаны только внутренний `setup.json`, инструкции и README s5, активный указатель и процедура `highgrade-init` установленной Поставки v0.2.14 в s7.
2. `highgrade.exe inspect --bootstrap --root D:\my_projects\MyCodex\target\agent-pilot\s5`: первоначально структурный `failed`, exit 1; сырой [bootstrap.json](bootstrap.json). Сканер предложил `legacy/architecture.md` по имени.
3. Прочитан кандидат, затем `highgrade.exe inventory --root ...\s5`: `passed`, exit 0; сырой [inventory.json](inventory.json). `rg --files --hidden -g '!evidence/**' -g '!**/.git/**' .` и `rg -n -i 'сумм|скид|округ|заказ|csv|часов|разделит|total|архитектур|устройств' -g '!evidence/**' .` дали [список](file-search.txt) и [совпадения](content-search.txt). После этого прочитаны `ledger/part-b.txt`, `total.py`, `test_total.py`, `orders.json` и составлен [план](PLAN.md).
4. Снята локальная копия исходных семи файлов в [before](before). Изменены только README, AGENTS и созданные канонические документы, инструкция и реестр. `highgrade.exe spec-init --root ...\s5`: `passed`, exit 0; [сырой JSON](spec-init.json).
5. Выполнены `highgrade.exe global-status --profile ...\s7\.pilot-profile`, `doctor --root ...\s5`, `inspect --root ...\s5 --registry .highgrade/project/documents.json`, `inspect --bootstrap --root ...\s5`, `python -B -m unittest discover -v`. Их stdout, stderr и код выхода сохранены в одноимённых файлах. `python -B evidence\hg9-false\finalize.py` создал [SHA](after.json), [diff](changes.diff) и [проверку ссылок](links.json); сырой результат и код выхода тоже сохранены.

## Проверки и границы

- `global-status`, `doctor`, повторный `inspect --bootstrap`, `spec-init` и Python unittest завершились с exit 0. `unittest` выполнил один существующий тест; его фактический отчёт находится в `unittest.stderr.txt`.
- Обычный `inspect` вернул `status=unknown`, exit 2: пять `BudgetNotAgreed`, `context-route` без согласованного бюджета, `AreaUnclassified` для `evidence`, `legacy`, `specs`. Эти области не скрыты фиктивной широкой областью или придуманным бюджетом. Все шесть обязательных связей отмечены `present`; собственная проверка 17 ссылок не нашла битых.
- Документы основаны на прочитанном содержании и исходниках; структурные отчёты сами по себе не удостоверяют полноту смыслового переноса. Неизвестные продуктовые решения и бюджеты остаются открытыми. Это локальная синтетическая адаптация, не принятие, не коммит и не публикация.
