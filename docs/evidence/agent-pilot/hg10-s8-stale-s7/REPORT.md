# Проба HG-0010-S8: устаревшее доказательство после дрейфа кода

В синтетическом `s7` существующее evidence для `HG-0001-S1` ссылалось на `counter.py` с SHA-256 `e5c5552f882bee9c85f79ca444260f94d514cf54b034a72463ec22c3d918c4ac`. Во время пробы фактический `counter.py` имел SHA-256 `97f35629976767c9b148ec2c1a177c61705a668bbf8fbdb863bfdb5c50c48f63`: [точный diff](changed-counter.diff). Прежний отчёт показывал общий `OK`, но относился к прежним байтам кода и не закрывал сценарий в изменённом состоянии.

## Хронология и сырые результаты

1. Сверены `setup.json`, `before/`, действующий `counter.py`, тест, спецификация, результаты и исходный native-отчёт. Все снимки `before/` совпали с `before_sha256`; среди контролируемых входов в рабочем состоянии отличался только `counter.py`. Хеш установленного CLI v0-2-14 совпал с `setup.json`.
2. В изменённом состоянии вызвана установленная команда `highgrade.exe spec-check --root D:\my_projects\MyCodex\target\agent-pilot\s7 --id HG-0001`. Код выхода `1`; [сырой ответ](actual-changed-spec-check.json): `failed`, `EvidenceMissingOrStale` для `HG-0001-S1`. Сценарий оставлен пробелом / `unknown`.
3. До восстановления вызвана `highgrade.exe spec-list --root D:\my_projects\MyCodex\target\agent-pilot\s7`. Код выхода `0` означает успешное чтение списка; [сырой ответ](actual-changed-spec-list.json) показывает `HG-0001` с `technical=in_progress`, `human=pending`.
4. `git diff --no-index -- evidence/hg10-s8-stale/before/counter.py counter.py` вернул `1` из-за различия файлов; [сырой diff](changed-counter.diff). [Хеши и наблюдения изменённого состояния](changed-state.json) фиксируют несовпадение входа evidence и фактической логики.
5. После проверки точных исходных хешей выполнено `Copy-Item -LiteralPath evidence/hg10-s8-stale/before/counter.py -Destination counter.py -Force`. Восстановленный SHA-256 совпал с исходным `e5c5552f882bee9c85f79ca444260f94d514cf54b034a72463ec22c3d918c4ac`; [итоговый diff](restored-counter.diff) пуст.
6. Повторена только `highgrade.exe spec-check --root D:\my_projects\MyCodex\target\agent-pilot\s7 --id HG-0001`: код выхода `0`, [сырой ответ](actual-restored-spec-check.json) — `passed`, находок нет. [Итоговые хеши](restored-state.json) для всех входов совпали с `before/` и `setup.json`; [итоговый Git status](restored-status.txt) сохранён.

Граница: проверена одна ветка устаревания при изменении логики. Новый native-тест не запускался, новое evidence PASS для `HG-0001` не записывалось, вариант `skipped` не проверялся. `spec.json` и `results.json` не менялись. Коммит, публикация и сеть не использовались.
