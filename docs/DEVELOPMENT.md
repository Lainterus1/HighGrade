# Разработка и проверки

Команды — из корня Проекта; нужен Python 3.12. Задачи и результаты — в `specs/changes/<ID>/spec.json` и `results.json`.

## Изменения требований: нативные спецификации

[Выбор маршрута](../kit/procedures/task.md#выбор-достаточного-маршрута): технические задачи идут прямо в work либо через план; зависимую цепочку ведёт [Planner](../kit/procedures/planner.md) под одним Goal. spec-list учитывает специфицированные изменения. CLI активной Поставки:

```powershell
$hgProfile = $env:USERPROFILE
$hgActive = Get-Content "$hgProfile/.highgrade/global/active.json" -Raw | ConvertFrom-Json
$hgExe = "$hgProfile/.highgrade/global/releases/$($hgActive.release)/highgrade.exe"
& $hgExe spec-list --root "$PWD"
& $hgExe spec-schema --root "$PWD"
```

Для короткой сверки перед переходом между этапами: `& $hgExe spec-check --root "$PWD" --id HG-CHANGE --brief true`. Вывод содержит точные хеши, статусы и причины повторной проверки без полного журнала; `spec-read` оставь для редактирования change или разбора конкретного пробела.

Каталог v3 ведёт CLI активной Поставки; версию проверяет `global-status`. В пустом проекте `spec-list` возвращает `store_sha256=absent`. Маршрут: `spec-new` с хешем → `spec-read` → правка change → `spec-save` с хешем снимка. Каталог напрямую не редактируй; параметры — в [справочнике](../kit/references/cli.md#структурированные-спецификации).

Далее: `spec-validate` → работа и тесты → `spec-evidence` → независимое ревью и `spec-review` → `spec-check` → `spec-integrate`. Решение человека пишет `spec-decide`; оно и право на commit/push не следуют из проверки. Старый store мигрирует через `spec-migrate --to directory` с backup. Фокусные тесты: `cargo test --locked --test catalog_contracts --test spec_contracts --test trace_contracts`.

## Архив прежнего каталога

Прежние OpenSpec-файлы и [карта переноса](archive/legacy-openspec/migration.json) сохранены в [архиве](archive/legacy-openspec/); соответствие прежних ID — в [архиве канонизации](archive/legacy-native-specs/2026-09-25/). Действующий каталог — `specs/`. Выборочный `spec-transfer` не доказывает поведение или приёмку.

## Сценарии и исходные результаты

Метка `// highgrade: HG-...` связывает сценарий с настоящим Rust-тестом. Требования к доказательствам — в [QUALITY](workflow/QUALITY.md).

Связка ниже проверяет действующий нативный каталог и связь автоматических сценариев с тестами; готовность каждого изменения отдельно определяет `spec-check`.

Из корня запусти `cargo-nextest` 0.9.146 из `target/highgrade/tools/bin/` (изолированный файл вызывай с аргументом `nextest`).

```powershell
New-Item -ItemType Directory -Force target/nextest/highgrade | Out-Null
$nextest = (Resolve-Path 'target/highgrade/tools/bin/cargo-nextest.exe').Path
node scripts/source-scenarios.mjs check
if ($LASTEXITCODE -ne 0) { throw 'native scenario catalog check failed' }
& $nextest nextest list --locked --message-format json | Set-Content -Encoding utf8 target/nextest/highgrade/list.json
if ($LASTEXITCODE -ne 0) { throw 'nextest list failed' }
& $nextest nextest run --locked --profile highgrade
if ($LASTEXITCODE -ne 0) { throw 'nextest run failed' }
node scripts/source-scenarios.mjs prepare
if ($LASTEXITCODE -ne 0) { throw 'scenario preparation failed' }
target/debug/highgrade.exe trace --root "$PWD" --record target/nextest/highgrade/run.json | Set-Content -Encoding utf8 target/nextest/highgrade/trace.json
if ($LASTEXITCODE -notin @(0, 2)) { throw 'trace failed' }
node scripts/source-scenarios.mjs verify
if ($LASTEXITCODE -ne 0) { throw 'scenario verification failed' }
```

`prepare` отвергает устаревший отчёт. `trace` отдельно считает тесты, связанные тесты, автоматические сценарии и сценарии вне области; последние не означают PASS или провал. `verify` требует успеха автоматических связей. Ручные доказательства проверяет `spec-check`; см. [аудит HG-0024](evidence/spec-verification/HG-0024-test-audit.md).

## Выбор проверок

`kit/` и документы: `check-repository`, `inspect` бюджета; при новом поведении — упражнение. Rust: фокусный тест и Nextest/trace; `cargo test` только для дополнительного покрытия. `spec-run` считай тестовым запуском. Сбой: адресный повтор.

`spec-stats --root "$PWD" --id HG-CHANGE` читает время запусков без повторного выполнения; используй при замедлении. Ручное наблюдение человек подтверждает в чате для точной версии; сохрани источник и не повторяй автоматически.

## Rust CLI и глобальная поставка

Активация — в [BUILD](BUILD.md). Для правки только навыков/документов Cargo не нужен. При изменении Rust проверь также `cargo build --release --locked`; кандидат принятия собирается отдельно из точного SHA. Ниже — изолированные испытания CLI.

```powershell
cargo fmt --all -- --check
cargo build --locked
.\target\debug\highgrade.exe global-install --profile '<пустой-временный-профиль>' --source (Join-Path $PWD 'kit') --candidate-exe (Join-Path $PWD 'target\debug\highgrade.exe')
.\target\debug\highgrade.exe global-status --profile '<пустой-временный-профиль>'
.\target\debug\highgrade.exe doctor --root "$PWD"
.\target\debug\highgrade.exe inspect --root "$PWD" --registry '.highgrade/project/documents.json'
.\target\debug\highgrade.exe inspect --bootstrap --root "$PWD"
```

Установку проверяй в изолированном профиле; обновление описано в [справочнике](../kit/references/cli.md). Код 2/unknown у диагностики не является PASS.

Временные пробы — только в `target/highgrade/tmp/<запуск>`; после задачи убирай их. Структура и обслуживание — в [BUILD](BUILD.md); контроль размера: `python scripts/target-maintenance.py --check`.

## Структура и документы

```powershell
node scripts/check-repository.mjs
node scripts/check-repository.mjs --verify-import
```

Структура и импорт проверяются скриптом; бюджеты — inspect. Реестр: `.highgrade/project/documents.json`.

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Проверки поведения — в собственных окружениях плагинов.

## Публикация

`highgrade-approve` по [проектной инструкции](../.highgrade/project/INSTRUCTIONS.md) создаёт коммит; при изменении CLI/Поставки или явном поручении активирует точный SHA. `highgrade-push` отдельно отправляет выбранный диапазон. Перед отправкой проверь состав, лицензию, секреты и хеши `kit/` по SHA; после — удалённый SHA и CI. Активацию проверяй через `global-status`; приёмка целевых проектов отдельна.
