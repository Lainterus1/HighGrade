# Разработка и проверки

Область: воспроизводимые команды для исходного High Grade. Архитектура — в [ARCHITECTURE](ARCHITECTURE.md), текущий план — в [global-workflow](plans/2026-09-23-global-workflow.md). Команды выполняются из корня проекта. Rust 1.97.0 Windows MSVC использовался для проверенного кандидата; первый выпуск остаётся Windows/source-only.

## Rust CLI и глобальная поставка

```powershell
cargo fmt --all -- --check
cargo test --locked
cargo build --release --locked
.\target\release\highgrade.exe global-install --profile '<пустой-временный-профиль>' --source (Join-Path $PWD 'kit') --candidate-exe (Join-Path $PWD 'target\release\highgrade.exe')
.\target\release\highgrade.exe global-status --profile '<пустой-временный-профиль>'
.\target\release\highgrade.exe doctor --root "$PWD"
.\target\release\highgrade.exe inspect --root "$PWD" --registry '.highgrade/project/documents.json'
```

Сначала проверяй установку в изолированном профиле. После этого пользовательскую установку можно обновить явным маршрутом. [Справочник](../kit/references/cli.md) содержит команды preview, применения и отката. Общие навыки и CLI не копируются в проект. `doctor`/`inspect` могут вернуть unknown из-за несогласованных бюджетов или недоступной зависимости; код 2 не является PASS. Native тесты Rust и Playwright выполняются своими командами, затем `trace` читает их отчёты. Старые проектные команды находятся в `bundle/` только для исторической проверки.

## Структура и документы

```powershell
node scripts/check-repository.mjs
node scripts/check-repository.mjs --verify-import
```

Скрипт проверяет пять документных ролей и входы плагинов, измеряет размер и сверяет импорт по флагу. Он не запускает плагины, не выполняет смысловой анализ и не назначает бюджет. Реестр находится в `.highgrade/project/documents.json`; `null` означает не согласовано.

## Отдельные плагины

Исходник SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md). Вход Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md) и [toolchain](../plugins/code-health-audit/skills/code-health-audit/references/toolchain.md). Эти плагины тестируются в своих окружениях. Структурная сверка импорта не подтверждает их функциональное поведение; изменение исходников здесь не переключает установленные копии.

## Публикация

Перед GitHub push проверь полный состав файлов, лицензию, происхождение импортированного кода и отсутствие секретов/кешей. Коммитуй только исходники и доказательства задачи. Push и фактическое состояние удалённой ветки проверяй отдельно; публичный GitHub не означает пользовательскую приёмку целевых проектов.
