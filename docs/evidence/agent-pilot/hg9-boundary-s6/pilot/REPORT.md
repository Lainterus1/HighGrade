# HG-0009-S9/S14 — границы обследования s6

## Вывод

Объявленная владельцем область `scratch-boundary` исключена из технического снимка. Её содержание не открывалось и не перечислялось. CLI `inventory` показал `exclusions: ["scratch-boundary"]` и SHA-256 списка исключений `721baf216906bcaa1684280a237948288089b39fe24a75e561b23f1875ce524a`. Это подтверждает применение границы, но не одобрение аудита содержимого области.

`components/metrics/` содержит `README.md` и `metrics.py` по перечню путей. Это независимый компонент вне текущей адаптации хранилища настроек: в просмотренных входных документах, `store.py` и `test_store.py` ссылок/импортов `components`, `metrics` и `scratch-boundary` не найдено (`rg` exit 1). Содержание компонента агентом не прочитано; его поведение и пригодность остаются **unknown** для будущих задач. Его непрочитанность сама по себе не отменяет локальные проверки подключения основного проекта.

## Хронология точных действий

1. Прочитан только внутренний `evidence/hg9-boundary/setup.json`, а также `AGENTS.md`, `README.md`, проектная инструкция, реестр и `.highgrade/project/inventory-exclusions.json`. Внешний `docs/evidence/agent-pilot/hg9-boundary-s6/before/manifest` не читался. Установленная процедура `highgrade-init` и её правила аудита применены из профиля s7.
2. `highgrade.exe inspect --bootstrap --root D:\my_projects\MyCodex\target\agent-pilot\s6` → `passed`, exit 0. [JSON](bootstrap.json), [exit](bootstrap.exit.txt), [stderr](bootstrap.stderr.txt). Все семь канонических путей и шесть обязательных связей найдены; `candidate_scan=not_needed`, поэтому bootstrap не был обходом дерева.
3. `highgrade.exe inventory --root D:\my_projects\MyCodex\target\agent-pilot\s6` → `passed`, exit 0. [JSON](inventory.json), [exit](inventory.exit.txt), [stderr](inventory.stderr.txt). Именно этот снимок подтверждает исключение `scratch-boundary`; хеши остальных файлов не означают смыслового чтения.
4. Сверены реестр и проектная инструкция, прочитан `docs/DEVELOPMENT.md`. `Test-Path` проверил только существование исключённого каталога. `Get-ChildItem` перечислил имена двух файлов компонента. `rg -n 'components|metrics|scratch-boundary'` применён только к README, AGENTS, docs, `store.py`, `test_store.py`, проектной инструкции и реестру; сырой [вывод](boundary-probe.txt). Содержимое `scratch-boundary` не передавалось ни одной из этих команд.
5. `highgrade.exe global-status --profile D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile` → `passed`, exit 0, активна v0-2-14. `highgrade.exe doctor --root ...\s6` → `passed`, exit 0, но `semantic_readiness=not_assessed`. Обычный `highgrade.exe inspect --root ...\s6 --registry .highgrade/project/documents.json` → `unknown`, exit 2: несогласованные бюджеты и `AreaUnclassified: components`. Сырые stdout/JSON, stderr и коды сохранены в одноимённых файлах.
6. Штатный `python -B -m unittest -v` → exit 0, четыре теста `OK`; фактический [отчёт](unittest.stderr.txt), [код](unittest.exit.txt). Сравнение доступных исходных хешей CLI и итоговых SHA — [core-sha.json](core-sha.json); обследованные исходники и документы совпали. Для `documents.json` исходный SHA не был захвачен, поэтому побайтовая неизменность этого файла отмечена `null`, а не заявлена как доказанная.

## Предел результата

Структурные пути, версия установленной Поставки и штатные тесты подтверждены локально. Обычный `inspect` **не PASS**: бюджеты остаются несогласованными; компонент `metrics` не классифицирован реестром. Это ограничения общего обзора, а не обнаруженная зависимость основного `store.py` от компонента. Содержимое исключённой области, смысл компонента, эксплуатация, пользовательская приёмка и полнота вне обследованной области не проверены. Изменены только локальные файлы доказательств этой пробы; код, проектные документы, спецификации, Git и профиль не менялись.
