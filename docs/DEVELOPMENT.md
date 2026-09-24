# Разработка и проверки

Команды выполняются из корня Проекта. Задачи — в `.highgrade/specs/store.json`; устройство — в [ARCHITECTURE](ARCHITECTURE.md).

## Новые задачи: нативные спецификации

Используй CLI активной Поставки, чтобы поведение не зависело от незавершённых правок исходников:

```powershell
$hgProfile = $env:USERPROFILE
$hgActive = Get-Content "$hgProfile/.highgrade/global/active.json" -Raw | ConvertFrom-Json
$hgExe = "$hgProfile/.highgrade/global/releases/$($hgActive.release)/highgrade.exe"
& $hgExe global-status --profile $hgProfile
& $hgExe spec-list --root "$PWD"
& $hgExe spec-schema --root "$PWD"
```

Текущий store v2 обслуживает установленная Поставка 0.2.13; исходную CLI используй при разработке инструмента. Первый spec-list в пустом проекте возвращает store_sha256=absent. Для разрешённой задачи вызови spec-new с --title и этим хешем; номер назначает CLI; затем spec-read, редактирование объекта change и spec-save с хешем прочитанного снимка. Не редактируй store напрямую. Полные параметры — в [справочнике](../kit/references/cli.md#структурированные-спецификации).

Маршрут: spec-validate → реализация и штатные тесты → spec-evidence → независимое ревью и spec-review → spec-check → spec-integrate. Пустой шаблон остаётся черновиком. Включение требований не разрешает commit/push. spec-decide сохраняет человеческое решение, spec-list — JSON-прогресс; миграция v1 → v2 через spec-migrate с backup. Фокусные тесты самого инструмента: `cargo test --locked --test spec_contracts --test trace_contracts`.

## Сохранённый OpenSpec-каталог

Старые спеки остаются без изменений и переноса. Эти команды и CI проверяют сохранённый каталог; новые задачи не используют OpenSpec. Нужен Node.js 20.19+:

```powershell
npm.cmd install -g @fission-ai/openspec@1.13.1
$env:OPENSPEC_TELEMETRY = '0'
$openspecBin = Join-Path (npm.cmd prefix -g) 'openspec.cmd'
& $openspecBin validate --all --strict --no-interactive
```

Версия закреплена: 1.13.1; телеметрию отключай перед каждым вызовом. Проверка сохранённого каталога не подтверждает готовность JSON-изменения.

## Сценарии и исходные результаты

Метка `// highgrade: HG-...` связывает сценарий с настоящим Rust-тестом. Требования к доказательствам — в [QUALITY](workflow/QUALITY.md).

Связка ниже проверяет прежний каталог. JSON-сценарии — через spec-evidence/check; нативный trace описан в справочнике.

Локально установи закреплённый `cargo-nextest` 0.9.146, затем из корня репозитория выполни команды. Для изолированной установки используй путь к `cargo-nextest.exe` и аргумент `nextest`.

```powershell
New-Item -ItemType Directory -Force target/nextest/highgrade | Out-Null
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

`prepare` отвергает устаревший отчёт: повтори затронутые тесты. `trace` возвращает 2 при ручных пробелах; `verify` допускает только известные ручные сценарии и требует успеха автоматических. Ручные доказательства сохраняй по QUALITY.

## Rust CLI и глобальная поставка

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

Профиль `dev`/`test` сохраняет номера строк и отключает incremental для ограничения размера `target/debug`; `release` независим.

## Структура и документы

```powershell
node scripts/check-repository.mjs
node scripts/check-repository.mjs --verify-import
```

Скрипт проверяет роли, входы, размер и импорт по флагу; смысл, тесты плагинов и бюджеты проверяются отдельно. Реестр находится в `.highgrade/project/documents.json`; `null` означает не согласовано.

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Их тестируют в собственных окружениях. Сверка импорта не проверяет поведение и не переключает установленные копии.

## Публикация

`highgrade-approve` по [проектной инструкции](../.highgrade/project/INSTRUCTIONS.md) создаёт коммит и локально активирует его из изолированного SHA. `highgrade-push` по отдельному поручению отправляет выбранный диапазон готовых коммитов. Перед публикацией проверь состав, лицензию, происхождение, секреты и кэши; хеши `kit/` сверь с архивом SHA (`.gitattributes` закрепляет LF). После push сверь удалённый SHA, после местной активации — `global-status`. CI в `.github/workflows/verify.yml` проверяет отправленный исходник, но не активирует его. Это не означает приёмку целевых проектов.
