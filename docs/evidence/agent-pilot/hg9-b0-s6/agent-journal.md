# Итог локального пилота s6

## Граница и вход

Работа выполнена только в синтетическом `D:\my_projects\MyCodex\target\agent-pilot\s6`. Навык, процедура и CLI прочитаны только из установленной Поставки `D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile` версии 0.2.14. Исходный снимок `before/`, разрешение и пять SHA-256 находятся в [setup.json](setup.json); они совпали с файлами s6 перед правкой. Сырые входные отчёты — [bootstrap.json](bootstrap.json) и [inventory.json](inventory.json). CLI имеет SHA-256 `555f93dfff46cdad1db224ddec738ee978bbf7b087e1e2385e014cd8e83bdbe4`. Внешние проекты и сеть не использованы. Коммита, публикации, активации не было.

Координатор заранее назвал путь `notes/mechanism.txt`; агент прочитал его даже до собственного повторного bootstrap. Поэтому последующий адресный поиск не является доказательством самостоятельного обнаружения источника после отрицательного ответа сканера.

## Фактический порядок и команды

Ниже `$root = 'D:\my_projects\MyCodex\target\agent-pilot\s6'`, `$profile = 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile'`, `$cli = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'`. Все JSON-файлы с префиксом `agent-` сохраняют непосредственный вывод CLI; явные коды выхода зафиксированы при запуске. Текст тестов сохранён как непосредственный вывод `unittest`.

| Вызов / действие | Код и наблюдение | Сырой результат |
|---|---|---|
| `& $cli global-status --profile $profile` | 0; активна установленная 0.2.14 | [agent-global-status.json](agent-global-status.json) |
| `& $cli inspect --bootstrap --root $root` | 1, `failed`; 5 отсутствующих канонических путей, 5 отсутствующих связей, `current-architecture: not_found_in_scan` | [agent-bootstrap.json](agent-bootstrap.json) |
| `& $cli inventory --root $root` | 0, `passed`; список внутри s6 | [agent-inventory.json](agent-inventory.json) |
| `& $cli doctor --root $root` до адаптации | 2, `unknown`; установка не подключена к проекту | [agent-doctor-before.json](agent-doctor-before.json) |
| Адресное чтение `AGENTS.md`, `README.md`, `store.py`, `test_store.py`, `notes/mechanism.txt`; `rg` по тем же файлам | Содержание устройства и команда теста найдены в заметке; путь заметки был подсказан | [agent-manual-search.txt](agent-manual-search.txt), [agent-audit-plan.md](agent-audit-plan.md) |
| Смысловой аудит, таблица пробелов и план до изменения | Сохранены исходное правило и открытые решения | [agent-audit-plan.md](agent-audit-plan.md) |
| Созданы три канонических документа, проектная инструкция и реестр; дополнены README и AGENTS | Текст правила README не изменён; исходная заметка оставлена | [agent-final.diff](agent-final.diff) |
| `& $cli spec-init --root $root` | 0, `passed`; пустое проектное хранилище спецификаций, без нового требования | [agent-spec-init.json](agent-spec-init.json) |
| `& $cli doctor --root $root` после создания инструкции | 0, `passed`; совместимость `schema-only`, семантика `not_assessed` | [agent-doctor-after.json](agent-doctor-after.json) |
| `& $cli inspect --root $root --registry '.highgrade/project/documents.json'` до назначения областей | 2, `unknown`; 6 `BudgetNotAgreed`, 2 `AreaUnclassified` для `specs` и `evidence` | [agent-inspect-after.json](agent-inspect-after.json) |
| Дополнены реестр и инструкция владельцами `specs` и `evidence` | Обе неопределённости классификации устранены, бюджеты оставлены `null` | [agent-final.diff](agent-final.diff) |
| `python -B -m unittest -v` из `$root` | 0; Python 3.12.10, 3 теста прошли; SHA кода и теста до/после одинаковы | [agent-tests.txt](agent-tests.txt), [agent-tests.json](agent-tests.json) |
| Проверка относительных Markdown-ссылок в шести местных документах | 23 вхождения ссылок разрешаются в существующие файлы | [agent-links.json](agent-links.json) |
| Финальный `& $cli doctor --root $root` | 0, `passed`; `schema-only`, `semantic_readiness: not_assessed` | [agent-doctor-final.json](agent-doctor-final.json) |
| Финальный `& $cli inspect --root $root --registry '.highgrade/project/documents.json'` | 2, `unknown`; только 6 `BudgetNotAgreed`; все 6 обязательных связей `present` | [agent-inspect-final.json](agent-inspect-final.json) |
| Сравнение с `before/`, SHA-256 файлов и отчётов | Изменения только в 11 документных/конфигурационных файлах; `store.py`, `test_store.py`, `notes/mechanism.txt` побайтово прежние | [agent-final.diff](agent-final.diff), [agent-sha-manifest.json](agent-sha-manifest.json) |

## Наблюдаемый результат и границы сценариев

- **S6:** исходный bootstrap действительно сообщил `not_found_in_scan` для архитектуры и потребовал агентного аудита. После проверки содержания `notes/mechanism.txt` создан канонический документ с сохранением исходной заметки. Поскольку путь был дан заранее и чтение произошло до повторного bootstrap, строгую проверку самостоятельного поиска это не закрывает.
- **S12:** самостоятельный поиск необычно названного действующего источника **не доказан** и PASS не заявляется. Есть лишь проверка его содержания и корректное использование в адаптации.
- **S13:** сохранены русскоязычные README, AGENTS и заметка; действующее правило прежнего `settings.json` оставлено дословно. Добавлены русскоязычные документы, ссылки и реестр. SHA неизменённых источников и diff дают проверяемое основание; независимое смысловое ревью ещё требуется для вывода о полноте.
- **S17:** агент продолжил после неполного bootstrap, оформил локальное подключение и выполнил проверки. `doctor` прошёл только схему; финальный `inspect` остаётся `unknown` из-за шести несогласованных бюджетов. Полное подключение и будущие полномочия не объявляются подтверждёнными.

Бюджеты документов и маршрута, общие полномочия на будущие коммиты, внешняя CI, эксплуатация, условия человеческой приёмки и активации остались неизвестными и прямо отмечены в проектной инструкции. Никакой числовой бюджет или разрешение не выдуманы. Тесты подтверждают только три локальных поведения `store.py`, не работу внешней службы или сохранность после сбоя питания. `doctor` и `inspect` не заменяют смысловое ревью. Формального PASS для HG-0009 из одного bootstrap или этого локального пилота не записано.
