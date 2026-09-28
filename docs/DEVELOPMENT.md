# Разработка и проверки

Команды — из корня; нужны Python 3.12, Node.js и Rust/Cargo. Для операций каталога и диагностики используй установленную CLI (ниже); кандидат target/debug нужен для проверки изменённого Rust-кода и создаётся Nextest либо `cargo build --locked`. После очистки не считай его существующим.

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

Nextest 0.9.146 установите готовым бинарником для своей ОС в `target/highgrade/tools/bin` и добавьте этот каталог в PATH. Rust проекта закреплён отдельно: сборка Nextest из исходников может требовать более новый компилятор. Команды общего Windows/Linux CI и локального барьера приведены ниже в разделе «Linux и общий CI».

`prepare` отвергает устаревший отчёт. После `spec-integrate` повтори `prepare`/`trace`/`verify`. Nextest переиспользуется лишь при эквивалентном каталоге и неизменных входах; иначе повтори его. Причину показывает `native_report_reason`.

## Выбор проверок

| Изменение | Достаточная проверка | Когда расширять |
|---|---|---|
| Формулировки, инструкции, навигация | Смысловое ревью diff, check-repository, render-skills --check, inspect | При новом исполнимом контракте — соответствующие тесты |
| Rust CLI, формат, установка | Узкие контрактные тесты; один полный Nextest/trace для итоговой редакции цепочки | При конкретном непокрытом риске или отказе переиспользования |
| Только результаты/метаданные каталога | spec-validate/check; prepare/trace/verify при изменении трассировки | Nextest лишь при изменении его значимых входов |
| Сборка/активация | build-release.py из SHA, preview/apply одного кандидата, global-status | Ошибка — адресная диагностика и восстановление |

Нормативные инструкции проверяются смысловым ревью; агентное упражнение нужно только для вопроса об исполнении, не закрытого чтением. Пилот внешнего проекта требует поручения. Новая обязательная роль без отдельной необходимости не вводится.

Общий отчёт Nextest и входы сценария различаются по [work](../kit/procedures/work.md#область-входов-доказательства). Полный барьер нужен для итоговой редакции цепочки; после интеграции переиспользуй его через prepare/trace/verify.

`spec-stats --id HG-CHANGE` читает длительности без запуска. Наблюдение человека фиксируй со ссылкой на сообщение.

## Rust CLI и глобальная поставка

Для JUnit доступен spec-run-inputs до запуска и spec-run-import после; формат — в справочнике CLI. Снимок задним числом не подтверждает актуальность.

Активация: build-release.py из SHA по [BUILD](BUILD.md).

Диагностика: `& $hgExe doctor --root "$PWD"` и `& $hgExe inspect --root "$PWD"` (выбор $hgExe выше). Установку/обновление проверяют global_contracts; дополнительные профили нужны только для непокрытого риска. Код unknown не является PASS.

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

## Linux и общий CI

Rust закреплён в `rust-toolchain.toml`. На Linux имя CLI и Nextest не имеет `.exe`; для повторяемого общего барьера после установки Nextest 0.9.146 в PATH:

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

Те же Python entry points выполняются в Windows/Linux CI. Локальный Linux-прогон не доказывает Windows CI. Проверка технической установки не является пользовательским пилотом навыков.
