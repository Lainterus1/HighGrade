# Итог разрешённой init-пробы s2

## Область и вход

Изменения ограничены синтетическим `D:\my_projects\MyCodex\target\agent-pilot\s2`. Навык, правила, процедура и CLI взяты из установленной Поставки `D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile` v0-2-14, CLI SHA-256 `555f93dfff46cdad1db224ddec738ee978bbf7b087e1e2385e014cd8e83bdbe4`. Исходные [setup.json](setup.json), [before/](before/), [bootstrap.json](bootstrap.json), [inventory.json](inventory.json) сохранены. Все 14 входных SHA совпали с текущими файлами до изменений и снимком `before/` ([agent-input-verification.json](agent-input-verification.json)). Сеть, реальные проекты и исходники High Grade вне установленного профиля не использованы.

## Хронология с командами и сырым выводом

Для команд ниже `$root = 'D:\my_projects\MyCodex\target\agent-pilot\s2'`, `$profile = 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile'`, `$cli = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'`. Точные команды, время UTC, код выхода и путь вывода записаны в [agent-initial-commands.json](agent-initial-commands.json), [agent-check-commands.json](agent-check-commands.json) и [agent-final-cli.json](agent-final-cli.json); исполнявшиеся скрипты лежат рядом.

| Порядок | Действие и наблюдение | Сырой вывод |
|---|---|---|
| 1 | Сверка setup и `before/`: 14/14 SHA совпали до изменений | [agent-input-verification.json](agent-input-verification.json) |
| 2 | `& $cli global-status --profile $profile`: код 0, v0-2-14 активна в установленном профиле | [agent-global-status.json](agent-global-status.json) |
| 3 | `& $cli inspect --bootstrap --root $root`: код 1, `failed`; отсутствуют пять канонических путей и пять входящих связей, сканер не нашёл кандидатов | [agent-bootstrap.json](agent-bootstrap.json) |
| 4 | `& $cli inventory --root $root`: код 0, `passed`; источник обзора файлов, не смысловой аудит | [agent-inventory.json](agent-inventory.json) |
| 5 | `& $cli doctor --root $root` до адаптации: код 2, `unknown`, местная инструкция отсутствует | [agent-doctor-before.json](agent-doctor-before.json) |
| 6 | Прочитаны правила, README, PLAN, смешанное руководство, профильная стратегия, код, тесты, данные и прежний отчёт; до правок составлены пробелы и план | [agent-audit-plan.md](agent-audit-plan.md) |
| 7 | README и AGENTS получили обязательные ссылки; смешанное руководство разделено между архитектурой, инженерными правилами и командами; прежний путь оставлен картой переноса. Профильная стратегия сохранила самостоятельное владение выбором сценариев | [agent-final.diff](agent-final.diff) |
| 8 | Созданы проектная инструкция и реестр с действующими источниками, маршрутами и неизвестными бюджетами | [agent-final.diff](agent-final.diff) |
| 9 | `& $cli spec-init --root $root`: код 0, `passed`; создан пустой каталог `specs/` без нового требования | [agent-spec-init.json](agent-spec-init.json) |
| 10 | `& $cli doctor --root $root`: код 0, `passed`, совместимость только `schema-only`, семантика `not_assessed` | [agent-doctor-after.json](agent-doctor-after.json) |
| 11 | `& $cli inspect --root $root --registry .highgrade/project/documents.json`: код 2, `unknown`; только шесть `BudgetNotAgreed`, шесть обязательных связей `present` | [agent-inspect-after.json](agent-inspect-after.json) |
| 12 | `python -B -m unittest discover -v` из s2: код 0, 7/7 тестов; девять входов кода/данных/тестов имеют одинаковые SHA до и после | [agent-tests.txt](agent-tests.txt), [agent-tests.json](agent-tests.json) |
| 13 | Устранено дублирование выбора проверок в инженерном документе: детали остались у профильной стратегии; код и тесты не менялись | [agent-final.diff](agent-final.diff) |
| 14 | Повторены зависящие от правки `doctor` (код 0) и `inspect` (код 2, только шесть `BudgetNotAgreed`) | [agent-doctor-final.json](agent-doctor-final.json), [agent-inspect-final.json](agent-inspect-final.json) |
| 15 | Проверены 37 локальных ссылок на существующие файлы; итоговые SHA, diff и статус всех шести обязательных связей | [agent-links.json](agent-links.json), [agent-sha-manifest.json](agent-sha-manifest.json), [agent-final-verification.json](agent-final-verification.json) |

## Наблюдаемый результат и предел

Три канонических документа созданы из смешанного руководства, а `docs/project-guide.md` оставлен картой для старых входящих ссылок. `docs/test-strategy.md` остаётся отдельным действующим источником выбора проверок. Текст правила в README сохранён; исходные шесть модулей, `orders.json`, два тестовых файла, завершённый `PLAN.md` и исходный `test-results.txt` побайтово неизменны. Их SHA приведены в [манифесте](agent-sha-manifest.json); полный diff — [agent-final.diff](agent-final.diff). Новая продуктовая спецификация не создавалась, потому что контракт расчёта не менялся.

`doctor` подтвердил только схему подключения, не смысловую полноту. Финальный `inspect` имеет статус `unknown` по пяти бюджетам документов и бюджету маршрута, сохранённым как `null`; неизвестное не объявлено успешным. Тесты проверяют локальные сценарии синтетической фикстуры, а не реальные оплаты, доставку или внешнюю CI. Право будущего коммита, эксплуатационная активация и человеческая приёмка не установлены. Смысловой вывод требует независимого ревью; агент не присваивал PASS сценариям. Коммита, публикации, активации и глобального переключения не было.
