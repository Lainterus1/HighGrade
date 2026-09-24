# Разработка и проверки

Команды исходного High Grade выполняются из корня проекта. Устройство — в [ARCHITECTURE](ARCHITECTURE.md), задачи поведения — в `openspec/changes/`. Первый выпуск остаётся Windows/source-only.

## Проектный OpenSpec

Нужен Node.js 20.19.0 или новее. На Windows установи закреплённый CLI в пользовательский npm-профиль; это не копирует общие навыки в репозиторий:

```powershell
npm.cmd install -g @fission-ai/openspec@1.13.1
$env:OPENSPEC_TELEMETRY = '0'
$openspecBin = Join-Path (npm.cmd prefix -g) 'openspec.cmd'
& $openspecBin --version
$env:PATH = "$(npm.cmd prefix -g);$env:PATH"
& $openspecBin validate --all --strict --no-interactive
```

Ожидаемая версия — `1.13.1`. Перед каждым вызовом отключай телеметрию через `OPENSPEC_TELEMETRY=0`. Проект уже инициализирован с `--tools none`; новые навыки OpenSpec здесь не нужны. Изменение проверяй командой `& $openspecBin validate <имя> --strict --no-interactive`, завершённое архивируй штатным OpenSpec. `highgrade doctor` обнаруживает CLI в `PATH`, но не проверяет его версию; её показывает `& $openspecBin --version`.

## Сценарии и исходные результаты

У действующих требований и сценариев OpenSpec есть устойчивые ID. У каждого сценария указан способ проверки. Метка `// highgrade: HG-...` ставится рядом с содержательным интеграционным Rust-тестом. `trace` сверяет её с инвентарём, исполнением и хешами исходников; `ручная` означает отдельное наблюдение, а не автоматический успех. Порядок и критерии — в [QUALITY](workflow/QUALITY.md).

Локально установи закреплённый `cargo-nextest` 0.9.146, затем из корня репозитория выполни команды. Если используешь изолированную установку, замени `cargo nextest` на путь к своему `cargo-nextest.exe` с аргументом `nextest`.

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

`prepare` отвергает отчёт, если после него изменены спеки, тесты или закреплённые исходники; запусти native тесты повторно. `trace` может вернуть 2, когда остаются ручные сценарии; заключительный `verify` принимает только известные ручные пробелы и требует реального успеха всех помеченных автоматических сценариев. Ручное упражнение записывай с входным состоянием, действием, наблюдением, временем и SHA либо перечнем изменённых файлов. Сверка исходника не подменяет просмотр содержательности теста.

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

Сначала проверяй установку в изолированном профиле. [Справочник](../kit/references/cli.md) описывает обновление, очистку старых выпусков и пределы bootstrap. `doctor`/`inspect` могут вернуть unknown; код 2 не является PASS. Rust и Playwright запускаются штатно, затем `trace` читает отчёты. `bundle/` — история прежней поставки.

Профиль `dev`, от которого наследуется `test`, сохраняет номера строк для диагностики и отключает инкрементальную сборку, чтобы результаты повторных проверок не разрастались в `target/debug`. Повторная компиляция может занимать больше времени. `release` использует собственный профиль.

## Структура и документы

```powershell
node scripts/check-repository.mjs
node scripts/check-repository.mjs --verify-import
```

Скрипт проверяет пять документных ролей и входы плагинов, измеряет размер и сверяет импорт по флагу. Он не запускает плагины, не выполняет смысловой анализ и не назначает бюджет. Реестр находится в `.highgrade/project/documents.json`; `null` означает не согласовано.

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Их тестируют в собственных окружениях. Сверка импорта не проверяет поведение и не переключает установленные копии.

## Публикация

`highgrade-approve` по [проектной инструкции](../.highgrade/project/INSTRUCTIONS.md) создаёт коммит и локально активирует его из изолированного SHA. `highgrade-push` по отдельному поручению отправляет выбранный диапазон готовых коммитов. Перед публикацией проверь состав, лицензию, происхождение, секреты и кэши; хеши `kit/` сверь с архивом SHA (`.gitattributes` закрепляет LF). После push сверь удалённый SHA, после местной активации — `global-status`. CI в `.github/workflows/verify.yml` проверяет отправленный исходник, но не активирует его. Это не означает приёмку целевых проектов.
