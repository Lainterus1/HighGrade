# Инструменты Поставки High Grade

`highgrade <команда> --help` показывает параметры, пример и входную схему без чтения проекта. Все операции возвращают JSON Report. Используйте CLI активного выпуска; при разработке нового формата — явно обозначенный совместимый кандидат.

| Задача | Справочник |
|---|---|
| Обследовать проект, сохранить решения, карту и продолжение | [Обследование](tools/survey.md) |
| Создать, прочитать, изменить спеку; теги, CAS, миграция | [Спецификации](tools/specifications.md) |
| Запустить проверку, импортировать результат, записать evidence/review | [Проверки](tools/verification.md) |
| Установить, обновить и восстановить общую установку | [Установка](tools/installation.md) |
| Найти проблему, сохранить решение и повторное наблюдение | [Реестр проблем](tools/issues.md) |
| Проверить проект, документы, реестр и исключения inventory | [Диагностика](tools/diagnostics.md) |

Правила и полномочия определяет [rules](../rules.md), последовательность работы — [процедуры](../procedures/task.md), методику обследования — [audit](audit.md). Эти справочники владеют командами и форматами, шаблоны задают только местную адаптацию. Отдельный Tools-документ в каждом целевом проекте не требуется: команды проекта остаются в DEVELOPMENT.

Прежние ссылки сохранены для совместимости:

## Структурированные спецификации
[Формат и операции](tools/specifications.md#структурированные-спецификации).

### Номера, версии и приёмка
[Идентификаторы и решения](tools/specifications.md#номера-версии-и-приёмка).

### Каталог, миграция и восстановление
[Хранение и восстановление](tools/specifications.md#каталог-миграция-и-восстановление).

### Связи и теги
[Навигация](tools/specifications.md#связи-и-теги).

### Привязки и явный запуск
[Раннеры](tools/verification.md#привязки-и-явный-запуск).

## Короткий цикл редактирования
[Рабочие представления](tools/specifications.md#короткий-цикл-редактирования).

## Предварительная проверка документов (v0-2-5)
[Bootstrap](tools/diagnostics.md#предварительная-проверка-документов-v0-2-5).

## Начальный реестр документов
[Реестр](tools/diagnostics.md#начальный-реестр-документов).

## Совместимость проекта
[Границы диагностики](tools/diagnostics.md#совместимость-проекта).

## Именованные результаты и последовательная запись

Report сохраняет `measurements`, status и коды завершения. `result` содержит ключи, встречающиеся ровно в одном объекте measurements: `result.store_sha256`, `result.survey_sha256`, `result.value`, `result.change`, `result.catalog_path`. Повторяющиеся ключи перечислены в `ambiguous_result_keys` и отсутствуют в result; их строки выбирают из measurements по предметному ID. Не использовать номер строки как контракт. Store/local/survey SHA относятся к разным объектам; подмена одного другим не допускается. Spec-init также возвращает directory и pointer_path относительно root.

Пример PowerShell для уже подготовленного patch.json:

```powershell
$hgRead = & $hgExe survey-read --root "$PWD" --view editable
if ($LASTEXITCODE -ne 0) { throw 'survey-read failed' }
$hgState = $hgRead | ConvertFrom-Json
if ($hgState.status -ne 'passed' -or !$hgState.result.survey_sha256) { throw 'Missing survey state' }
$hgWrite = & $hgExe survey-edit --root "$PWD" --expected $hgState.result.survey_sha256 --input patch.json
if ($LASTEXITCODE -ne 0) { throw 'survey-edit failed; reread before retry' }
$hgNext = $hgWrite | ConvertFrom-Json
```

Для Markdown: проверив успешный survey-read --view markdown тем же способом, выведите `$hgState.result.value`. JSON — источник, сохранение производного файла необязательно; перенаправление в файл требует выбранного пути и обычных правил сохранности.
