# Разработка и проверки

Нужны Python 3.12+, Node.js 24 и Rust 1.89. Команды — из корня. Кандидат CLI: Nextest либо `cargo build --locked`.

## Изменения требований: нативные спецификации

Маршрут — по [task](../kit/procedures/task.md#выбор-достаточного-маршрута), зависимая работа — через [Planner](../kit/procedures/planner.md); Goal требует поручения. CLI активной Поставки:

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

Статус: `spec-check --id HG-CHANGE --brief true`; контекст: `spec-read --view summary|editable|full`.

Каталог изменяй совместимой CLI: `spec-new` с хешем → `spec-read --view editable` → `spec-edit --input patch.json --expected HASH --validate true`. Активную версию определяет global-status, target/debug — кандидат. Полный spec-save и параметры — в [справочнике](../kit/references/tools/specifications.md#структурированные-спецификации).

Далее: `spec-validate` → работа/тесты → `spec-evidence` → независимое ревью/`spec-review` → `spec-integrate --brief true` с проверкой готовности. Причины — spec-check, решение человека — spec-decide. Фокус CLI: `cargo test --locked --test catalog_contracts --test spec_contracts --test trace_contracts`.

## Сценарии и исходные результаты

Метка `// highgrade: HG-...` связывает сценарий с Rust-тестом; trace требует фактический результат. Checks для spec-run необязательны, но объявленная связь должна совпадать. Требования к доказательствам — в [QUALITY](workflow/QUALITY.md).

Установите Nextest 0.9.146 в `target/highgrade/tools/bin` и добавьте каталог в PATH.

`verify.py tests` сверяет входы до/после Nextest. `prepare` сверяет хеши, состав входов и отчёты завершённой попытки; после сбоя/прерывания повтори `tests`. mtime не доказывает актуальность, снимок — подлинность. После `spec-integrate` повтори `prepare`/`trace`/`verify`: эквивалентный каталог допускает reuse, иначе повтори `tests`; причина — `native_report_reason`.

Отчёты tests/UI: [запуск, экспорт и очистка](EVIDENCE-STORAGE.md).

## Выбор проверок

| Изменение | Достаточная проверка | Когда расширять |
|---|---|---|
| Документы, UI/API | Смысловая сверка [UI](UI.md), [ui/README](../ui/README.md) и diff; check-repository, render-skills --check, inspect | Новое поведение — его тесты |
| Rust CLI, формат, установка | Узкие контрактные тесты; один полный Nextest/trace для итоговой редакции цепочки | При конкретном непокрытом риске или отказе переиспользования |
| Только результаты/метаданные каталога | spec-validate/check; prepare/trace/verify при изменении трассировки | Nextest лишь при изменении его значимых входов |
| Сборка/активация | build-release.py из SHA, preview/apply одного кандидата, global-status | Ошибка — адресная диагностика и восстановление |

Внешний пилот требует поручения.

Контракт — spec-list/read; причинные входы — [work](../kit/procedures/work.md#область-входов-доказательства), [примеры](evidence/current-requirements/2026-10-01/verification.md). Общий снимок их не заменяет.

`spec-stats --id HG-CHANGE` читает длительности без запуска. Для наблюдения человека сохрани ссылку на сообщение.

## Rust CLI и глобальная поставка

JUnit: spec-run-inputs до запуска, spec-run-import после. Снимок задним числом не подтверждает актуальность; формат — в справочнике CLI.

Диагностика активной CLI: `doctor` и `inspect --root <проект>`. Установку проверяют global_contracts; дополнительные профили нужны при непокрытом риске. unknown не является PASS.

Временные пробы — только в `target/highgrade/tmp/<запуск>`; после задачи убирай их. Структура и обслуживание — в [BUILD](BUILD.md); контроль размера: `python scripts/target-maintenance.py --check`.

## Интерфейс и Storybook

ui: `npm ci`, `npm run build`, `npm run build-storybook`, `npm test`; корень: `cargo build --locked`. `npm run storybook`: 127.0.0.1:6006. [Installed E2E и браузер](BUILD.md#обязательный-installed-e2e).

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Проверки поведения — в собственных окружениях плагинов.

## Принятие и публикация

Полномочия и последовательность — в [местной инструкции](../.highgrade/project/INSTRUCTIONS.md), сборка — в [BUILD](BUILD.md). Push разрешается отдельно.

## Общие проверки и CI

Штатные команды по матрице выше:

```bash
cargo fmt --all -- --check
python -m unittest discover -s scripts/tests -v
node --test ui/scripts/tests/*.test.mjs
python scripts/render-skills.py --check
python scripts/verify.py tests
python scripts/verify.py specs
python scripts/verify.py scenarios
node scripts/check-scenario-scale.mjs
node scripts/check-scenario-reuse.mjs
node scripts/check-repository.mjs
python scripts/verify.py inspect
```

CI асинхронен: ожидание не блокирует Work, интеграцию и порученные коммит, активацию или push. Статус точного SHA сообщается отдельно; известный отказ разбирается по местной инструкции.

| Проверка | Linux | Windows |
|---|---|---|
| render-skills, fmt, структура, inspect, specs | Да | — |
| Python lifecycle, Nextest inventory/run/trace, Node guards | Да | Да |
| Scale/reuse, Storybook, native/decision/edit Playwright | Да | — |
| Embedded UI, exact release build, required installed upgrade/recovery | Да | Да |

`ubuntu-24.04` — [образ GitHub Linux runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), не требование к пользователю. Тонкие native UI отказы отдельно проверены только на Linux; при Windows UI дефекте повтори затронутый selector на Windows.

`verify.py specs` читает каталог один раз. Cargo cache имеет ключ SHA и fallback по платформе/toolchain/dependencies. Проверка исторического кэша — [BUILD](BUILD.md#обязательный-installed-e2e); область кэшей — [GitHub](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching). Новый запуск отменяет прежний запуск той же ветки. Кэш и локальный PASS не доказывают результат другой ОС или приёмку. Экономия времени требует измерения нового run.
