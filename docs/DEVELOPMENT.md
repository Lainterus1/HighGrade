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

Краткий статус: `spec-check --id HG-CHANGE --brief true`. Рабочий контекст: `spec-read --view summary|editable`; full — для истории.

Каталог ведёт совместимая CLI; global-status определяет установленную, target/debug — кандидат разработки. Маршрут: `spec-new` с хешем → `spec-read --view editable` → `spec-edit --input patch.json --expected HASH --validate true`. Полный spec-save остаётся совместимым. Каталог напрямую не редактируй; параметры — в [справочнике](../kit/references/tools/specifications.md#структурированные-спецификации).

Далее: `spec-validate` → работа и тесты → `spec-evidence` → независимое ревью и `spec-review` → `spec-integrate --brief true` (включает проверку готовности). spec-check показывает причины. Решение человека пишет spec-decide. Фокус CLI: `cargo test --locked --test catalog_contracts --test spec_contracts --test trace_contracts`.

## Сценарии и исходные результаты

Метка `// highgrade: HG-...` связывает сценарий с Rust-тестом; trace требует фактический результат. Checks для spec-run необязательны, но объявленная связь должна совпадать. Требования к доказательствам — в [QUALITY](workflow/QUALITY.md).

Связка проверяет каталог и автоматические сценарии; готовность изменения — spec-check.

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

`prepare` отвергает устаревший отчёт. После `spec-integrate` повтори `prepare`/`trace`/`verify`. Nextest переиспользуется лишь при эквивалентном каталоге и неизменных входах; иначе повтори его. Причину показывает `native_report_reason`.

## Выбор проверок

| Изменение | Достаточная проверка | Когда расширять |
|---|---|---|
| Формулировки, инструкции, навигация | Смысловое ревью diff, check-repository, render-skills --check, inspect | При новом исполнимом контракте — соответствующие тесты |
| Rust CLI, формат, установка | Узкие контрактные тесты; один полный Nextest/trace для итоговой редакции цепочки | При конкретном непокрытом риске или отказе переиспользования |
| Только результаты/метаданные каталога | spec-validate/check; prepare/trace/verify при изменении трассировки | Nextest лишь при изменении его значимых входов |
| Сборка/активация | build-release.py из SHA, preview/apply одного кандидата, global-status | Ошибка — адресная диагностика и восстановление |

Малые правки промптов и инструкций не требуют агентных упражнений на импровизированных проектах. Для изменения поведения агента упражнение назначается только при конкретном риске, который не закрывают смысловое ревью и доступные проверки; пилот внешнего проекта требует поручения. Структура текста не доказывает будущее поведение агента. Роли добавляются только под отдельную необходимую работу; обязательного набора специалистов нет.

`spec-run` — фактический запуск, а не бесплатная фиксация уже готового отчёта. После интеграции переиспользуй актуальный Nextest через prepare/trace/verify. Фокусные проверки допустимы на промежуточных этапах; полный барьер обязателен до итогового approve.

`spec-stats --id HG-CHANGE` читает время запусков без выполнения тестов. Ручное наблюдение человек подтверждает в чате для точной версии; сохрани источник и не повторяй автоматически.

## Rust CLI и глобальная поставка

Для JUnit доступен spec-run-inputs до запуска и spec-run-import после; формат — в справочнике CLI. Снимок задним числом не подтверждает актуальность.

Активация: build-release.py из SHA по [BUILD](BUILD.md).

Диагностика: `target/debug/highgrade.exe doctor --root "$PWD"` и `inspect --root "$PWD"`. Установку/обновление проверяют global_contracts; дополнительные профили нужны только для непокрытого риска. Код unknown не является PASS.

Временные пробы — только в `target/highgrade/tmp/<запуск>`; после задачи убирай их. Структура и обслуживание — в [BUILD](BUILD.md); контроль размера: `python scripts/target-maintenance.py --check`.

## Структура и документы

```powershell
node scripts/check-repository.mjs
python scripts/render-skills.py --check
```

Структура и импорт проверяются скриптом; бюджеты — inspect. Реестр: `.highgrade/project/documents.json`.

## Отдельные плагины

SVG Vectorizer — [README](../plugins/svg-vectorizer/README.md); Code Health Audit — [SKILL.md](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Проверки поведения — в собственных окружениях плагинов.

## Принятие и публикация

Полномочия и последовательность — в [местной инструкции](../.highgrade/project/INSTRUCTIONS.md), сборка — в [BUILD](BUILD.md). Push разрешается отдельно.
