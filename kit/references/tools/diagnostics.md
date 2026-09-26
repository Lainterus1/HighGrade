# Диагностика и документы проекта

Для последовательного подключения используй [обследование](survey.md). При действующем реестре обычный inspect проверяет принятые роли; bootstrap ниже остаётся предварительной диагностикой без реестра.

## Предварительная проверка документов (v0-2-5)

`inspect --bootstrap --root <абсолютный-путь>` работает до создания реестра. JSON-отчёт `measurements[0]` содержит `slots` для семи путей: `README.md`, `AGENTS.md`, `docs/ARCHITECTURE.md`, `docs/ENGINEERING.md`, `docs/DEVELOPMENT.md`, `.highgrade/project/INSTRUCTIONS.md`, `.highgrade/project/documents.json`. `path_state` показывает точное наличие, `candidates` — ограниченные возможные альтернативы по имени файла или подписанной Markdown-ссылке, `candidate_status=not_needed` означает наличие точного пути; `not_found_in_scan` не означает отсутствия содержания. `required_links` отдельно показывает шесть обязательных связей: из README к четырём документам, из AGENTS к проектной инструкции, из неё к реестру. `--bootstrap` нельзя сочетать с `--registry` или `--scope`. Неверный корень даёт `failed`; отсутствующий точный путь или связь — структурное `failed`; недоступная область или предел — `unknown` в находках. Результат не создаёт и не переносит файлы.

`candidate_scan=performed` означает поиск альтернатив для отсутствующих путей. Если все семь точных путей есть, `candidate_scan=not_needed`, дерево не обходится и `visited_entries=0`; обязательные связи и список исключений всё равно проверяются. Структурный `failed` на ещё не подключённом проекте даёт задачи адаптации, а не отменяет аудит. Ошибка корня или отсутствие пригодного JSON-отчёта останавливает зависимую работу. Содержание и решение о переносе агент фиксирует в таблице адаптации из процедуры init.

Когда поиск кандидатов нужен, обход включает новые файлы и скрытые инструкции без зависимости от Git. Читаются только `.highgrade/project/inventory-exclusions.json` и три канонических источника ссылок — README, AGENTS, проектная инструкция. Валидные проектные исключения применяются до обхода; свободный текст их причин не выводится. Некорректный список даёт `unknown` и не применяется частично. `.git`, `node_modules`, `target`, `.pnpm-store`, `.output`, `.tanstack`, `.highgrade/releases`, кэши, известные закрытые имена, символические ссылки и Windows reparse points не читаются; `.highgrade/project` остаётся в области. Исключения не доказывают одобрение аудита.

Пределы: 50 000 записей, 128 чтений, 64 КиБ на входной файл, 12 кандидатов на роль, 32 показанных исключения, 200 байт на путь, глубина 64 и 64 КиБ на JSON-отчёт. Достижение предела отмечается как `unknown`; непоказанная область остаётся задачей агента. Обычный `inspect` требует реестр и проверяет назначенные роли и пути, обязательные связи, прочие зарегистрированные ссылки и бюджеты. Он не заменяет смысловой аудит и не исправляет проект автоматически.

## Начальный реестр документов

Основа `.highgrade/project/documents.json` для init приведена ниже. Укажи реальные области `scope`, дополнительные источники и согласованные бюджеты; `null` намеренно даёт `BudgetNotAgreed`, а не PASS. `active: true, loading: history` проверяет источник и ссылки, исключая его из обычного маршрута; `active: false` вообще исключает строку из проверки. Не назначай широкую область ради скрытия `AreaUnclassified`.

```json
{
  "schema_version": 1,
  "documents": [
    {"id":"readme","path":"README.md","role":"purpose-navigation","loading":"entry","scope":[],"budget":null},
    {"id":"agents","path":"AGENTS.md","role":"agent-rules","loading":"entry","scope":[],"budget":null},
    {"id":"architecture","path":"docs/ARCHITECTURE.md","role":"current-architecture","loading":"task","scope":[],"budget":null},
    {"id":"engineering","path":"docs/ENGINEERING.md","role":"engineering-rules","loading":"task","scope":[],"budget":null},
    {"id":"development","path":"docs/DEVELOPMENT.md","role":"commands-procedures","loading":"task","scope":[],"budget":null}
  ],
  "additional_sources": [
    {"path":".highgrade/project/INSTRUCTIONS.md","role":"project-adaptation","active":true,"loading":"entry","scope":[".highgrade/project/documents.json"]}
  ],
  "route_budget": null
}
```

Путь и `scope` задаются относительно корня проекта. Для согласованного бюджета используется объект `{"unit":"bytes","agreed":true,"min":1000,"baseline":2000,"current":2000,"ceiling":3000,"history":[],"review_trigger":"после двух задач"}`; числа здесь иллюстративные, переносить их как принятые нельзя. Изменение `current` сохраняет цепочку `history` с `from`, `to`, `reason`, `reduction_considered`, `next_review`. Пять основных ролей обязательны и назначаются однозначно. В записи используй либо role (строка), либо roles (непустой массив строк), если один документ выполняет несколько ролей. Пути и ID уникальны; бюджет совмещённого файла учитывается один раз. Шаблонные пути необязательны: inspect проверяет зарегистрированные пути и связи между ними. Служебные INSTRUCTIONS.md и documents.json остаются в .highgrade/project/.

## Совместимость проекта

Doctor поддерживает `highgrade_project_schema: 1` в начале UTF-8 инструкции между строками `---` (LF/CRLF). `project_instruction=compatible` имеет границу `compatibility_scope=schema-only`, `semantic_compatibility=not_assessed`. Это не проверка заполнения или смысла. Неизвестная схема не понижается автоматически. Для проверки обязательного runner используйте doctor --action spec-run; версию инструмента подтверждает команда проекта. Адаптация и решение конфликтов — в [init](../../procedures/init.md) и [clear](../../procedures/clear.md).

Код самого CLI: 1 — failed, 2 — unknown, 0 — остальные состояния (включая предупреждения). Фиксируй `$LASTEXITCODE` сразу после команды, до следующего процесса; код оболочки или инструмента-обёртки может отличаться. При расхождении с JSON не угадывай код, повтори минимальную диагностику с явным захватом.

`doctor --root PATH --action spec-read` проверяет каталог; `--action spec-run --id HG-ID [--check CHECK]` — входы и исполняемые файлы выбранных проверок без исполнения проектного кода. Без action сохраняется прежняя диагностика. Для установки и обновления используйте global-status и preview.

Bootstrap возвращает `validation`, `adaptation`, `applied`: отсутствие стандартных путей и связей означает адаптацию; повреждение или недоступность остаются препятствием. Прежние status/exit code и находки сохранены для совместимости; новые поля разделяют техническую проверку и решение человека.

## Исключения inventory и прежний receipt

`.highgrade/project/inventory-exclusions.json` имеет вид `{"schema_version":1,"entries":[{"path":"relative/path","reason":"конкретная причина"}]}`. Пути относительные, уникальные, не внутри .highgrade; до 128 записей, причина непустая. CLI возвращает список исключений и SHA256 файла. Исключение из снимка не подтверждает прочтение области; методика — в [audit](../audit.md).

Только прежний проектный legacy-install проверяет `exclusions_sha256` в audit receipt: он должен совпасть с конфигурацией (или отсутствовать/null, если файла нет). Изменённые исключения требуют нового соответствующего receipt по legacy-маршруту. Глобальные global-install/global-update не требуют этот receipt и не обследуют проекты.
