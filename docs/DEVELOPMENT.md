# Разработка и проверки

Область: воспроизводимые команды для исходного High Grade. Архитектура — в [ARCHITECTURE](ARCHITECTURE.md); текущие задачи поведения — в проектном `openspec/changes/`, зависимые технические планы — в `docs/plans/`. Команды выполняются из корня проекта. Rust 1.97.0 Windows MSVC использовался для проверенного кандидата; первый выпуск остаётся Windows/source-only.

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

Ожидаемая версия — `1.13.1`. У OpenSpec по умолчанию включена анонимная телеметрия; `OPENSPEC_TELEMETRY=0` отключает её для этой сессии и проверки обновлений CLI. Устанавливай эту переменную перед каждым вызовом OpenSpec в работе над High Grade. Проектный `openspec/config.yaml` уже создан командой `openspec init --tools none --language Russian`; повторять init в обычной работе не нужно. `--tools none` не создаёт параллельные навыки OpenSpec для Codex: используйте общие `highgrade-task/spec/work`. Перед реализацией изменения проверь `& $openspecBin validate <имя-изменения> --strict --no-interactive`; после проверки результата архивируй завершённое изменение штатным OpenSpec. `highgrade doctor` видит OpenSpec только когда каталог npm CLI есть в `PATH`.

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

Сначала проверяй установку в изолированном профиле. После этого пользовательскую установку можно обновить явным маршрутом. [Справочник](../kit/references/cli.md) содержит команды preview, применения, отката и пределы `inspect --bootstrap`. Общие навыки и CLI не копируются в проект. `doctor`/`inspect` могут вернуть unknown из-за неполного измерения или недоступного входа; код 2 не является PASS. Native тесты Rust и Playwright выполняются своими командами, затем `trace` читает их отчёты. Старые проектные команды находятся в `bundle/` только для исторической проверки.

## Структура и документы

```powershell
node scripts/check-repository.mjs
node scripts/check-repository.mjs --verify-import
```

Скрипт проверяет пять документных ролей и входы плагинов, измеряет размер и сверяет импорт по флагу. Он не запускает плагины, не выполняет смысловой анализ и не назначает бюджет. Реестр находится в `.highgrade/project/documents.json`; `null` означает не согласовано.

## Отдельные плагины

Исходник SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md). Вход Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md) и [toolchain](../plugins/code-health-audit/skills/code-health-audit/references/toolchain.md). Эти плагины тестируются в своих окружениях. Структурная сверка импорта не подтверждает их функциональное поведение; изменение исходников здесь не переключает установленные копии.

## Публикация

`highgrade-deploy` по [проектной инструкции](../.highgrade/project/INSTRUCTIONS.md) завершается локальным коммитом. Push и активация требуют отдельных поручений. Перед публикацией проверь состав, лицензию, происхождение, секреты и кэши; хеши `kit/` сверь с архивом SHA (`.gitattributes` закрепляет LF). После разрешённого push сверь удалённый SHA; после разрешённой активации проверь `global-status`. Это не означает приёмку целевых проектов.
