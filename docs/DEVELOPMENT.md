# Разработка и проверки

Команды — из корня Проекта; нужен Python 3.12. Задачи и результаты — в `specs/changes/<ID>/spec.json` и `results.json`.

## Изменения требований: нативные спецификации

[Выбор маршрута](../kit/procedures/task.md#выбор-достаточного-маршрута): технические задачи идут прямо в work либо через план. spec-list учитывает специфицированные изменения. CLI активной Поставки:

```powershell
$hgProfile = $env:USERPROFILE
$hgActive = Get-Content "$hgProfile/.highgrade/global/active.json" -Raw | ConvertFrom-Json
$hgExe = "$hgProfile/.highgrade/global/releases/$($hgActive.release)/highgrade.exe"
& $hgExe spec-list --root "$PWD"
& $hgExe spec-schema --root "$PWD"
```

Каталог v3 обслуживает CLI активной Поставки; её версию определяет `global-status`. При разработке инструмента используй сборку исходников. Первый spec-list в пустом проекте возвращает store_sha256=absent. Маршрут: spec-new с хешем → spec-read → редактирование change → spec-save с хешем снимка. Каталог напрямую не редактируй. Параметры — в [справочнике](../kit/references/cli.md#структурированные-спецификации).

Маршрут: spec-validate → реализация и тесты → spec-evidence → независимое ревью и spec-review → spec-check → spec-integrate. Включение требований не разрешает commit/push. spec-decide сохраняет решение человека, spec-list — JSON-прогресс; старый store мигрирует через spec-migrate --to directory с backup. Повторная проверка требует свежего ревью и не переносит прежнюю приёмку. Фокусные тесты: `cargo test --locked --test catalog_contracts --test spec_contracts --test trace_contracts`.

## Архив прежнего каталога

Исходники OpenSpec побайтно сохранены в [архиве](archive/legacy-openspec/) с [картой первого переноса](archive/legacy-openspec/migration.json). Промежуточные импортные изменения и соответствие прежних ID действующим — в [архиве канонизации](archive/legacy-native-specs/2026-09-25/). Действующий каталог — `specs/`; он не читает эти материалы для текущей работы. Новые изменения включай через `spec-integrate` с доказательствами. `spec-transfer` остаётся способом будущего выборочного импорта и сам по себе не означает проверку поведения или человеческую приёмку.

## Сценарии и исходные результаты

Метка `// highgrade: HG-...` связывает сценарий с настоящим Rust-тестом. Требования к доказательствам — в [QUALITY](workflow/QUALITY.md).

Связка ниже проверяет действующий нативный каталог и связь автоматических сценариев с тестами; готовность каждого изменения отдельно определяет `spec-check`.

Из корня запусти `cargo-nextest` 0.9.146 (изолированный файл вызывай с аргументом `nextest`).

```powershell
New-Item -ItemType Directory -Force target/nextest/highgrade | Out-Null
node scripts/source-scenarios.mjs check
if ($LASTEXITCODE -ne 0) { throw 'native scenario catalog check failed' }
cargo nextest list --locked --message-format json | Set-Content -Encoding utf8 target/nextest/highgrade/list.json
if ($LASTEXITCODE -ne 0) { throw 'nextest list failed' }
cargo nextest run --locked --profile highgrade
if ($LASTEXITCODE -ne 0) { throw 'nextest run failed' }
node scripts/source-scenarios.mjs prepare
if ($LASTEXITCODE -ne 0) { throw 'scenario preparation failed' }
target/debug/highgrade.exe trace --root "$PWD" --record target/nextest/highgrade/run.json | Set-Content -Encoding utf8 target/nextest/highgrade/trace.json
if ($LASTEXITCODE -notin @(0, 2)) { throw 'trace failed' }
node scripts/source-scenarios.mjs verify
if ($LASTEXITCODE -ne 0) { throw 'scenario verification failed' }
```

`prepare` отвергает устаревший отчёт. `trace` возвращает 2 при неподтверждённых сценариях; `verify` требует успеха автоматических связей и не выдаёт ручные пробелы за PASS. Ручные доказательства — по QUALITY.

## Rust CLI и глобальная поставка

Для активации и проверки её сборки — [BUILD](BUILD.md). Ниже — разработка и изолированные испытания.

```powershell
cargo fmt --all -- --check
cargo test --locked
cargo build --release --locked
.\target\release\highgrade.exe global-install --profile '<пустой-временный-профиль>' --source (Join-Path $PWD 'kit') --candidate-exe (Join-Path $PWD 'target\release\highgrade.exe')
.\target\release\highgrade.exe global-status --profile '<пустой-временный-профиль>'
.\target\release\highgrade.exe doctor --root "$PWD"
.\target\release\highgrade.exe inspect --root "$PWD" --registry '.highgrade/project/documents.json'
.\target\release\highgrade.exe inspect --bootstrap --root "$PWD"
```

Установку проверяй в изолированном профиле; обновление описано в [справочнике](../kit/references/cli.md). Код 2/unknown у диагностики не является PASS.

## Структура и документы

```powershell
node scripts/check-repository.mjs
node scripts/check-repository.mjs --verify-import
```

Структура и импорт проверяются скриптом; бюджеты — inspect. Реестр: `.highgrade/project/documents.json`.

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Проверки поведения — в собственных окружениях плагинов.

## Публикация

`highgrade-approve` по [проектной инструкции](../.highgrade/project/INSTRUCTIONS.md) создаёт коммит и локально активирует его из изолированного SHA. `highgrade-push` по отдельному поручению отправляет выбранный диапазон готовых коммитов. Перед публикацией проверь состав, лицензию, происхождение, секреты и кэши; хеши `kit/` сверь с архивом SHA (`.gitattributes` закрепляет LF). После push сверь удалённый SHA, после местной активации — `global-status`. CI в `.github/workflows/verify.yml` проверяет отправленный исходник, но не активирует его. Это не означает приёмку целевых проектов.
