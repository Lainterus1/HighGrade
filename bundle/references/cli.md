# Команды и записи кандидата High Grade

Используй `highgrade.exe` **из выбранной проверенной основы**, а не одноимённую команду в PATH. После подключения прочитай `.highgrade/active.json`, проверь его через `doctor`, затем вызывай `.highgrade/releases/<release>/highgrade.exe`. До установки вызывай исполняемый файл из явно выбранного исходного каталога после локальной сборки; не ищи соседний checkout. В PowerShell передай абсолютный путь к exe через `&`.

```powershell
& '<исходный каталог>/target/release/highgrade.exe' inventory --root '<проект>'
& '<исходный каталог>/target/release/highgrade.exe' install --root '<проект>' --source '<исходный каталог>/bundle' --audit '.highgrade/project/audit-receipt.json'
& '<проект>/.highgrade/releases/<active>/highgrade.exe' doctor --root '<проект>'
& '<проект>/.highgrade/releases/<active>/highgrade.exe' inspect --root '<проект>' --registry '.highgrade/project/documents.json'
```

Перед install агент делает полный аудит и получает согласованное подтверждение. Receipt имеет `schema_version:1`, `status:"approved"`, `scope:"full-project"`, непустое `evidence` и `files` — точный снимок пути→SHA256 из `inventory`. Если существует `.highgrade/project/inventory-exclusions.json`, добавь `exclusions_sha256` из отчёта `inventory`; изменение этого файла делает receipt устаревшим. Исключённые области классифицируй отдельно по [аудиту](audit.md). CLI сверяет структуру, не смысл и не полномочия. Не генерируй approved автоматически.

Для OpenSpec сохрани штатный заголовок и добавь ID: `### Requirement: [SNAF-011-R1] ...`, `#### Scenario: [SNAF-011-S1] ...`. Префикс задаёт проект: 2–32 заглавные латинские буквы, затем один или несколько непустых сегментов из заглавных букв и цифр через дефис; полная длина ID — 6–80 символов. У каждого Requirement и Scenario в анализируемой области должен быть ID. После native запуска запиши фактический код возврата, исходный машинный отчёт и JSON-запись. SHA256 — нижний регистр hex от исходных байтов; все пути записи относительны корню проекта, с `/`. Пример формы:

```json
{
  "schema_version": 1,
  "tool": "rust-nextest",
  "command": "cargo nextest run --profile ci",
  "scope": "math",
  "captured_at": "2026-09-23T00:00:00Z",
  "capture_root": "C:/absolute/project/path/at/capture",
  "exit_code": 0,
  "report": "rust/target/nextest/ci/junit.xml",
  "report_sha256": "<sha256 JUnit>",
  "spec_files": ["openspec/changes/example/specs/math/spec.md"],
  "test_sources": ["rust/tests/math.rs"],
  "source_hashes": {
    "openspec/changes/example/specs/math/spec.md": "<sha256>",
    "rust/tests/math.rs": "<sha256>",
    "evidence/rust-list.json": "<sha256>"
  },
  "inventory": "evidence/rust-list.json"
}
```

Для Rust `inventory` — сохранённый stdout `cargo nextest list --message-format json`, `report` — JUnit из `[profile.ci.junit] path = "junit.xml"` с `report-skipped = "all"`. Первый кандидат трассирует обычные **top-level** integration tests `tests/<binary-name>.rs`, чьё происхождение совпадает с `cwd`, `kind` и `binary-name` из nextest list. Комментарий `// highgrade: HG-X-001` стоит перед `#[test] fn ...`. Вложенные модули, нестандартные Rust targets и unit tests требуют отдельного проверенного адаптера; сейчас статус unknown.

Для Playwright Test используй JSON reporter и аннотацию теста `{ type: 'highgrade', description: 'HG-X-001' }`. В записи поставь `tool:"playwright"`, `report` на JSON, убери `inventory`, остальные поля сохрани. `capture_root` — абсолютный корень проекта при выполнении native команды; он позволяет перенести сохранённый отчёт в другой checkout. `test_sources` содержит пути относительно корня проекта, а `spec.file` native отчёта — относительно `config.rootDir`; CLI переносит прежний rootDir через `capture_root` и приводит к текущему корню. Для Rust так же переносится `cwd` из nextest list. По умолчанию скриншот/видео не требуются. Отчёт с пустым suite, skip/failure/retry или повреждёнными hash не подтверждает сценарий.

```powershell
& '<проект>/.highgrade/releases/<active>/highgrade.exe' trace --root '<проект>' --record 'evidence/run.json'
```

Для обновления сначала локально собери CLI выбранного кандидата и проверь его `--version`. Поле `cli_version` в `bundle/manifest.json` должно точно совпасть с версией выбранного exe; CLI сверяет это до переключения. Запускай preview **старым активным exe**, но передай `--candidate-exe` с абсолютным путём к собранному **новому** exe. Preview и apply используют тот же путь и хеш; это позволяет не зависеть от PATH и действительно доставить изменения CLI. Передай **тот же** путь решения, который будет применён; стандартное имя — `.highgrade/project/update-decision.json`. Preview возвращает `manifest_sha256`, `candidate_exe_sha256`, `adaptation_sha256`, `project_sha256` и текущую версию. Сравни смысл правил с адаптацией и документами, запусти применимые проверки. После этого создай JSON решения:

```json
{
  "status": "approved",
  "from": "v0-1-0",
  "to": "v0-1-1",
  "active_journal_sha256": "<из active.json>",
  "candidate_manifest_sha256": "<из preview>",
  "candidate_exe_sha256": "<из preview>",
  "adaptation_sha256": "<из preview>",
  "project_sha256": "<из preview>",
  "evidence": "<ссылка на проверенное смысловое сравнение и команды>"
}
```

```powershell
& '<проект>/.highgrade/releases/<active>/highgrade.exe' update --root '<проект>' --source '<новый источник>/bundle' --candidate-exe '<новый источник>/target/release/highgrade.exe' --decision '.highgrade/project/update-decision.json'
& '<проект>/.highgrade/releases/<active>/highgrade.exe' update --root '<проект>' --source '<новый источник>/bundle' --candidate-exe '<новый источник>/target/release/highgrade.exe' --decision '.highgrade/project/update-decision.json' --apply true
& '<проект>/.highgrade/releases/<новый active>/highgrade.exe' doctor --root '<проект>'
```

После переключения команду берут уже из нового активного release. При проверенной необходимости отката старый exe вызывает `update --root '<проект>' --rollback '<старый release>'`; затем проверь doctor и навыки. Изменение исходников, адаптации, проверяемых проектных файлов или самого решения между preview и apply требует нового решения.
