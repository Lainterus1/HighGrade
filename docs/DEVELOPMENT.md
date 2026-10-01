# Разработка и проверки

Нужны Python 3.12+, Node.js 24 и Rust 1.89. Команды — из корня. Кандидат CLI: Nextest либо `cargo build --locked`.

## Изменения требований: нативные спецификации

[Выбор маршрута](../kit/procedures/task.md#выбор-достаточного-маршрута): технические задачи идут прямо в work либо через план; зависимую цепочку ведёт [Planner](../kit/procedures/planner.md); Goal требует поручения. spec-list учитывает специфицированные изменения. CLI активной Поставки:

```powershell
$hgProfile = $env:USERPROFILE
$hgActive = Get-Content "$hgProfile/.highgrade/global/active.json" -Raw | ConvertFrom-Json
$hgExe = "$hgProfile/.highgrade/global/releases/$($hgActive.release)/highgrade.exe"
& $hgExe spec-list --root "$PWD"
& $hgExe spec-schema --root "$PWD"
```

```bash
hgRelease=$(python -c 'import json,pathlib; print(json.loads((pathlib.Path.home()/".highgrade/global/active.json").read_text())["release"])')
hgExe="$HOME/.highgrade/global/releases/$hgRelease/highgrade"
"$hgExe" spec-list --root "$PWD"
"$hgExe" spec-schema --root "$PWD"
```

Краткий статус: `spec-check --id HG-CHANGE --brief true`. Рабочий контекст: `spec-read --view summary|editable`; full — для истории.

Каталог ведёт совместимая CLI; global-status определяет установленную, target/debug — кандидат разработки. Маршрут: `spec-new` с хешем → `spec-read --view editable` → `spec-edit --input patch.json --expected HASH --validate true`. Полный spec-save остаётся совместимым. Каталог напрямую не редактируй; параметры — в [справочнике](../kit/references/tools/specifications.md#структурированные-спецификации).

Далее: `spec-validate` → работа и тесты → `spec-evidence` → независимое ревью и `spec-review` → `spec-integrate --brief true` (включает проверку готовности). spec-check показывает причины. Решение человека пишет spec-decide. Фокус CLI: `cargo test --locked --test catalog_contracts --test spec_contracts --test trace_contracts`.

## Сценарии и исходные результаты

Метка `// highgrade: HG-...` связывает сценарий с Rust-тестом; trace требует фактический результат. Checks для spec-run необязательны, но объявленная связь должна совпадать. Требования к доказательствам — в [QUALITY](workflow/QUALITY.md).

Связка проверяет каталог и автоматические сценарии; готовность изменения — spec-check.

Установите бинарник Nextest 0.9.146 в `target/highgrade/tools/bin` и добавьте каталог в PATH. Исходники могут требовать нового Rust.

`verify.py tests` сверяет входы до/после Nextest. `prepare` сверяет хеши, состав входов и отчёты завершённой попытки; после сбоя/прерывания повтори `tests`. mtime не доказывает актуальность, снимок — подлинность. После `spec-integrate` повтори `prepare`/`trace`/`verify`: эквивалентный каталог допускает reuse, иначе повтори `tests`; причина — `native_report_reason`.

## Выбор проверок

| Изменение | Достаточная проверка | Когда расширять |
|---|---|---|
| Документы, UI/API | Смысловая сверка [UI](UI.md), [ui/README](../ui/README.md) и diff; check-repository, render-skills --check, inspect | Новое поведение — его тесты |
| Rust CLI, формат, установка | Узкие контрактные тесты; один полный Nextest/trace для итоговой редакции цепочки | При конкретном непокрытом риске или отказе переиспользования |
| Только результаты/метаданные каталога | spec-validate/check; prepare/trace/verify при изменении трассировки | Nextest лишь при изменении его значимых входов |
| Сборка/активация | build-release.py из SHA, preview/apply одного кандидата, global-status | Ошибка — адресная диагностика и восстановление |

Нормативные инструкции требуют смыслового ревью; исполнение — наблюдения. Внешний пилот требует поручения; новые роли без необходимости не вводятся.

Владельца текущего контракта найди через spec-list/read. Входы — по [work](../kit/procedures/work.md#область-входов-доказательства), [примеры](evidence/current-requirements/2026-10-01/verification.md); общий снимок запуска их не заменяет.

`spec-stats --id HG-CHANGE` читает длительности без запуска. Для наблюдения человека сохрани ссылку на сообщение.

## Rust CLI и глобальная поставка

Для JUnit доступен spec-run-inputs до запуска и spec-run-import после; формат — в справочнике CLI. Снимок задним числом не подтверждает актуальность.

Активация: build-release.py из SHA по [BUILD](BUILD.md).

Диагностика: CLI активного выпуска `doctor --root <проект>` и `inspect --root <проект>`. Установку/обновление проверяют global_contracts; дополнительные профили нужны только для непокрытого риска. Код unknown не является PASS.

Временные пробы — только в `target/highgrade/tmp/<запуск>`; после задачи убирай их. Структура и обслуживание — в [BUILD](BUILD.md); контроль размера: `python scripts/target-maintenance.py --check`.

## Структура и документы

```text
node scripts/check-repository.mjs
python scripts/render-skills.py --check
```

Структура и импорт проверяются скриптом; бюджеты — inspect. Реестр: `.highgrade/project/documents.json`.

## Интерфейс и Storybook

ui: `npm ci`, `npm run build`, `npm run build-storybook`, `npm test`; корень: `cargo build --locked`. `npm run storybook`: 127.0.0.1:6006. [Installed E2E и браузер](BUILD.md#обязательный-installed-e2e).

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Проверки поведения — в собственных окружениях плагинов.

## Принятие и публикация

Полномочия и последовательность — в [местной инструкции](../.highgrade/project/INSTRUCTIONS.md), сборка — в [BUILD](BUILD.md). Push разрешается отдельно.

## Общие проверки и CI

Rust закреплён в `rust-toolchain.toml`. Для повторяемого общего барьера после установки Nextest 0.9.146 в PATH:

```bash
cargo fmt --all -- --check
python -m unittest discover -s scripts/tests -v
python scripts/render-skills.py --check
python scripts/verify.py tests
python scripts/verify.py specs
python scripts/verify.py scenarios
node scripts/check-scenario-scale.mjs
node scripts/check-scenario-reuse.mjs
node scripts/check-repository.mjs
python scripts/verify.py inspect
```

Те же Python entry points выполняются в Windows и Ubuntu CI. Локальный прогон подтверждает только текущую ОС; Windows CI объединённого коммита возможен после публикации. Проверка технической установки не является пользовательским пилотом навыков.
