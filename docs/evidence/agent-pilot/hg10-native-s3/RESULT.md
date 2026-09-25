# Локальная проба HG-0010-S7/S8

- HG-0010-S7: выполнена команда `D:\Progs\Python3.12\python.exe -B -m unittest -v test_codec` в синтетическом `s3`. Код выхода `0`; в [исходном отчёте](s7-native-unittest.txt) три реально выполненных теста `ok` с ID `HG-0001-S1/S2/S3`. Это доказательство штатного `unittest` для действующих сценариев `HG-0001`.
- Граница trace: отчёт `unittest` не является поддерживаемым run-record для `trace`. `trace` не запускался; его результат **unknown**, PASS ему не присваивался.
- HG-0010-S8: только на время второго прогона `test_v2` был помечен `@unittest.skip`. [Исходный отчёт](s8-skipped-unittest.txt) показывает общий код выхода `0` и `OK (skipped=1)`, но `HG-0001-S1` остался **пробелом / unknown**. `HG-0001-S2/S3` выполнены с `ok`.
- `test_codec.py` восстановлен побайтово: SHA-256 до и после `d26382cd92fc47791333f06dfe5a67311c8b4f99638c4698783bbdde38ba0d18`. Временная правка — в [diff](temporary-test.diff); [итоговый diff](final-vs-before.diff) пустой. Все шесть контролируемых входов совпали с `setup.json` и `before/`, включая `spec.json` и `results.json`.

Точные времена, команды, коды выхода и SHA-256: [журнал](actions.jsonl) и [машинный итог](run-manifest.json). Записи спецификации и результаты High Grade для этой пробы не менялись.
