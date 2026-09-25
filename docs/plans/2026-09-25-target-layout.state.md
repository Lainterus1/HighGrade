# Состояние: управляемый каталог `target/`

План: `docs/plans/2026-09-25-target-layout.md`, версия 4
Workspace: `D:\my_projects\MyCodex`
Observed: HEAD `88b4c23427142f53e39432b430181565e971bcee`, исходное рабочее дерево чистое; `target/` 761,5 МиБ, 16 каталогов и 139 файлов верхнего уровня.
Status: COMPLETE
Current: все этапы T1–T5 завершены
Next: нет
Blockers: none

## Прогресс и доказательства

- T1: VERIFIED; нативная HG-0019 создана и `spec-validate` passed; исходный `target/` 761,5 МиБ, в корне 139 файлов, доказательства и аварийный кандидат выделены как защищённые классы.
- T2: VERIFIED; `build-release.py`, его тесты и проектные инструкции используют `target/highgrade`; существующий кандидат проверен до и после переноса, 7/7 тестов жизненного цикла passed.
- T3: VERIFIED; 145 старых элементов (2417 файлов, 40 385 829 байт) сохранены в `.highgrade/local/archive/target-root-before-layout-2026-09-25.zip`, SHA256 `755e62bb852b8832e697ffaeb972ff806a2b620edfb8ee62e413a7316ff9143d`, каждый файл сверён с архивом. После точной очистки `cargo clean --release` убрал 134,8 МиБ и `cargo clean --profile test` 425,3 МиБ; `target-maintenance --check` passed, размер 162,93 МиБ, неизвестных путей нет. Первая широкая очистка была отклонена автоматической проверкой из-за неполного архива; причина устранена полной проверенной копией перед удалением.
- T4: VERIFIED; 12/12 Python-тестов в `specs/changes/HG-0019/evidence/python-unittest.txt`, `cargo test --locked`, `cargo fmt --check`, проверка импорта репозитория и `git diff --check` прошли. После тестов `target-maintenance --check` показал 430,92 МиБ, неизвестных путей и ссылок нет. `inspect` сообщил только 9 непроверенных внешних ссылок. Первое независимое ревью обнаружило удаление неизвестного файла внутри `tmp`; исправлено явным `--scratch`, проверено новым тестом. Повторное ревью `target_layout_review` — REVIEW_GO; три сценария имеют актуальные доказательства, `spec-check` прошёл, HG-0019 интегрирована (`technical=ready`, `human=pending`).
- T5 release preflight: VERIFIED; после первого коммита HG-0019 `b8fe9aea20b8b00a529787f02207f282fdb547c9` выявлен конфликт неизменяемого `v0-2-15` (тот же манифест, иной CLI SHA). Коммит `dceab1c319b443cb1ed151783662f9646ca2e09b` назначил `v0-2-16`; `cargo test --locked`, Nextest 116/116, связь 38/38 автоматических сценариев, проверка манифеста/репозитория, `fmt` и независимое REVIEW_GO прошли. Правило повышения версии перенесено на этап до коммита и сборки.
- T5 local activation: VERIFIED; уточнение правил в коммите `d67fe096f3752499af2cf30311870caf376deaa0` проверено независимым REVIEW_GO. Кандидат собран из этого SHA; preview `candidate_sha256=12bcd3638a212f3f4a91b004d2babb697c555a2d9b59e36192e5f191a384b1cf`, apply passed, `global-status` подтверждает `v0-2-16`, хеш CLI совпадает с кандидатом. После повторного `cargo clean --profile test` размер `target/` 176,65 МиБ без неизвестных путей. Удалённый `origin/main` до отправки совпадал с исходным HEAD `88b4c23427142f53e39432b430181565e971bcee`.
- T5 push/CI: VERIFIED; диапазон задачи и исправление опубликованы до `888db931955b8b0a6a74d7fc0329ea569028a878`, `origin/main` совпал. Первый CI [run 36139179298](https://github.com/Lainterus1/HighGrade/actions/runs/36139179298) выявил сравнение короткого и полного пути Windows в тесте очистки; тест теперь сравнивает канонический путь. Локально повторно прошли 12/12 Python-тестов, независимое ревью дало REVIEW_GO, повторный `spec-check` прошёл. CI [run 36139759914](https://github.com/Lainterus1/HighGrade/actions/runs/36139759914) на исправленном SHA завершился успешно.
