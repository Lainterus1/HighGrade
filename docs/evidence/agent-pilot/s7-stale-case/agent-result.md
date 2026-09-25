# HG-0017-S3: потеря отчёта доказательства

**Наблюдение:** при отсутствии `evidence/s7-continuation-unittest-verbose.txt` установленный CLI v0-2-14 вернул `spec-check: failed` с `EvidenceMissingOrStale`; `spec-list` показал `technical=in_progress`. Значит в этом состоянии сценарий HG-0001-S1 не подтверждён актуальным доказательством и изменение HG-0001 нельзя считать полностью проверенным. После побайтового восстановления отчёта `spec-check: passed`, `technical=ready`.

## Вход и команды

- Прочитаны [setup.json](setup.json), исходные [before-spec-check.json](before-spec-check.json) и [missing-spec-check.json](missing-spec-check.json), текущие `specs/changes/HG-0001/spec.json` и `results.json`, установленная `procedures/work.md`. Текущее отсутствие отчёта и SHA важных файлов проверены до запуска; все входные SHA совпали с setup.
- Использован только `D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile\.highgrade\global\releases\v0-2-14\highgrade.exe` с `--root D:\my_projects\MyCodex\target\agent-pilot\s7`.
- Точные вызовы в порядке выполнения (каждый вывод CLI сохранён в указанном ниже JSON):

```powershell
& 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile\.highgrade\global\releases\v0-2-14\highgrade.exe' spec-check --root 'D:\my_projects\MyCodex\target\agent-pilot\s7' --id HG-0001
& 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile\.highgrade\global\releases\v0-2-14\highgrade.exe' spec-list --root 'D:\my_projects\MyCodex\target\agent-pilot\s7'
Copy-Item -LiteralPath 'D:\my_projects\MyCodex\target\agent-pilot\s7\evidence\s7-stale-case\held-report.txt' -Destination 'D:\my_projects\MyCodex\target\agent-pilot\s7\evidence\s7-continuation-unittest-verbose.txt'
& 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile\.highgrade\global\releases\v0-2-14\highgrade.exe' spec-check --root 'D:\my_projects\MyCodex\target\agent-pilot\s7' --id HG-0001
& 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile\.highgrade\global\releases\v0-2-14\highgrade.exe' spec-list --root 'D:\my_projects\MyCodex\target\agent-pilot\s7'
```
- При отсутствующем отчёте, `2026-09-25T09:00:25Z`: `spec-check --root <s7> --id HG-0001`, код выхода `1`, сырой вывод [agent-missing-spec-check.json](agent-missing-spec-check.json). Следом `spec-list --root <s7>`, код выхода `0`, сырой вывод [agent-missing-spec-list.json](agent-missing-spec-list.json). Найдено `EvidenceMissingOrStale`; технический статус `in_progress`, человеческий `pending`.
- `2026-09-25T09:00:45Z`: `Copy-Item -LiteralPath evidence/s7-stale-case/held-report.txt -Destination evidence/s7-continuation-unittest-verbose.txt`. Источник и восстановленный файл — по 375 байт, каждый байт совпал; SHA-256 обоих `f117d681b13e52df998e542ca7bd8e52b7ac0c0afbe87588c5255086899bf3af`. Сырой результат сверки: [agent-restore.json](agent-restore.json).
- После восстановления, `2026-09-25T09:01:03Z`: та же команда `spec-check --root <s7> --id HG-0001`, код выхода `0`, сырой вывод [agent-restored-spec-check.json](agent-restored-spec-check.json). Затем `spec-list --root <s7>`, код выхода `0`, сырой вывод [agent-restored-spec-list.json](agent-restored-spec-list.json). Результат: `passed`, без findings; `technical=ready`, `human=pending`.

Конечные SHA и сопоставление с setup: [agent-final-manifest.json](agent-final-manifest.json). Хеши кода, теста, spec/results/catalog, установленного CLI и восстановленного отчёта совпадают с входным состоянием. `store_sha256=9f26f65bf3aa4ba386af86d17d7c5102231a4cc199b8aff3aaa19a299a50705f`, `change_sha256=45fb163973f1001e7201335c40bb55cf187fe5228e5d33d1285c76caa43aedd9`; после восстановления `inputs_sha256=2108a6294ac6ff73ee479358b89ee007b044fe295b2f027aac2609d3a253d66c`.

## Граница вывода

Это один локальный опыт на синтетическом s7 и установленной Поставке v0-2-14: он проверяет реакцию CLI на отсутствие заявленного файла доказательства и восстановление того же файла. Тесты поведения здесь повторно не запускались; наличие и SHA отчёта не подтверждают подлинность его содержания и не доказывают портативность без исходников. HG-0017-S3 — ID этого упражнения, а проверяемый сохранённый сценарий — HG-0001-S1. Спецификация, код и тест не изменялись; коммит и публикация не выполнялись.
