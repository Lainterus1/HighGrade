# Переносимый бюджет DEVELOPMENT

Исходная редакция: `c561dbaed7a55c54319ed957dd176b37deee5bdf`.
Локальная ветка проверки: `fix/portable-document-budget`.

## Причина и исправление

`docs/DEVELOPMENT.md` занимает 8978 байт с 103 LF при действующем
согласованном бюджете 9000 байт. Настоящий Git checkout с
`core.autocrlf=true` без явного атрибута превращает эти переводы строк
в CRLF: 9081 байт. CLI `inspect` правильно измеряет получившиеся байты
и выдаёт `BudgetExceeded`.

Точечное правило `docs/DEVELOPMENT.md text eol=lf` сохраняет LF при checkout
на любой платформе. Смысл и исходные байты документа, реестр бюджетов,
прочие правила и архив не изменены. Второе добавленное правило
`docs/evidence/portable-document-budget/** -text -whitespace` сохраняет
исходные байты этой области доказательств. Оно добавлено до native-прогона.
Это исправление исполнения действующего контракта; новая продуктовая
спецификация и повышение бюджета не требуются.

## Воспроизведение

- До исправления сохранены измерение, гипотеза и критерии в
  [baseline-reproduction.json](baseline-reproduction.json)
- [Регрессия](../../../scripts/tests/test_document_budget.py) создаёт отдельный
  временный Git-репозиторий, берёт действующие `.gitattributes`, документ
  и потолок из реестра, затем реально восстанавливает файл командой
  `git checkout --` при `core.autocrlf=true`, `false`, `input`
- Незафиксированный атрибутами контрольный файл подтверждает, что Git
  действительно выполнил CRLF-конверсию в варианте `true`. Сравниваются
  настоящие байты документа, их потолок и полное равенство LF-источнику
- Без исправления тест падает на `9081 > 9000`:
  [исходный RED](regression-before.log). С исправлением проходит:
  [исходный GREEN](regression-after.log)
- Отдельная полная копия исходного SHA проверена настоящим CLI `inspect`.
  После переноса только кандидатного `.gitattributes` и повторного checkout
  документа предупреждение исчезает: [сравнение](checkout-comparison.json),
  [исходный inspect](checkout-baseline-inspect.json),
  [inspect после исправления](checkout-candidate-inspect.json)

Git-настройки менялись только в собственных временных fixtures и через
параметры конкретных команд. Пользовательские и системные настройки
не изменены; их чтение в Git-fixtures отключено. Временные fixtures убраны.

## Результаты и границы

- Python: 20/20 PASS, включая новую регрессию; [лог](python-tests.log)
- Native Nextest: 196/196 PASS, 0 skipped; [лог](native-tests.log),
  [JUnit](native/junit.xml), [инвентарь](native/list.json)
- UI собран из закреплённого lockfile с локальным npm-кэшем:
  [лог](ui-build.log)
- Формат Rust, render-skills и структура: PASS;
  [лог](structure-checks.log)
- Все действующие specs прошли `python scripts/verify.py specs`;
  [лог](specs.log)
- prepare/trace/verify: 118/118 автоматических сценариев PASS;
  131 сценарий остаётся вне автоматической трассировки, как в исходном каталоге.
  196 результатов тестов, 0 missing/skipped/unknown;
  [исходный trace](native/trace.json), [снимок входов](native/run.json)
- Проверки масштаба 100/200 specs и защиты переиспользования: PASS;
  [общий лог](scenario-checks.log)
- Финальный `inspect`: 0 BudgetExceeded, 0 failed/unknown,
  9 прежних ExternalLinkUnchecked; [отчёт](inspect.json)
- `git diff --check`: PASS; документ, реестр и архив совпадают с исходным SHA

Это Linux-прогон с реальными Git-преобразованиями CRLF, а не прогон
Windows CI. В `inspect` остаются прежние `ExternalLinkUnchecked`;
исправленное предупреждение бюджета отсутствует. Активация, коммит,
push и пользовательская приёмка в этой проверке не выполнялись.

Промежуточный запуск scenarios пересёкся с активным spec validation и
получил штатный `StoreBusy`: [исходный лог](scenario-checks-busy.log).
Lock вручную не удалялся; после штатного завершения владельца
последовательный повтор прошёл успешно.
