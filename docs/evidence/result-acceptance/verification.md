# Проверка простой приёмки результата

Изменение: HG-0070. Дата: 2026-10-09. Проверена локальная редакция исходников. Коммит, активация, публикация и человеческая приёмка не выполнены. Финальная готовность требует Windows/Ubuntu CI точного SHA; HG-0070-T6 открыт.

## Наблюдаемое поведение

Форма результата содержит выбор статуса и необязательный комментарий. SHA/build ID и имя не запрашиваются. Источник `Локальный интерфейс` записывает UI; время и хеши проверенного снимка записывает CLI. Версия из обращения переносится автоматически. Неизвестная версия остаётся пустой. HEAD не используется для её получения.

Прямой `spec-decide` также берёт однозначную версию открытых обращений того же снимка. Неоднозначность и явное несовпадение отклоняют пакет до записи. `spec-attention respond` относится к выбранному обращению и сохраняет строгий повтор. Исторические решения не переписываются.

Основной текст удаляет только распознанные реквизиты с метками `Проверенная редакция` и `Итог/проверки` либо `Проверки и версия`. Полная исходная строка доступна в раскрываемом блоке. Действия, адрес localhost и ограничения Google/ИИ сохранены. Код формы не вызывает LLM и не формирует новое резюме.

## Исполненные проверки

| Проверка | Исходный отчёт | Исход |
|---|---|---|
| `cargo test --locked --test attention_contracts --test spec_contracts` | rust-contracts-reviewed.txt | 16 + 19 passed |
| `PATH=target/highgrade/tools/bin:$PATH python3 scripts/verify.py tests` | nextest-reviewed.txt; nextest-junit.xml; nextest-attempt.json; nextest-completion.json | 221 passed; 0 skipped |
| Playwright с Chromium, реальной локальной CLI и Storybook на 6007 | ui-reviewed.json; ui-reviewed.txt | 39 passed; 1 skipped; 0 flaky |
| `python3 -m unittest discover -s scripts/tests -v` | python-reviewed.txt | 62 passed |
| `npm run build`, `npm run build-storybook`, `cargo build --locked` | ui-build-final-2.txt; storybook-build.txt; candidate-build-reviewed.txt | passed |
| `cargo fmt --all -- --check`, `python3 scripts/render-skills.py --check` | fmt-reviewed.txt; render-skills-reviewed.txt | passed |
| `node scripts/check-repository.mjs` | repository-check-reviewed.txt | passed |
| Проверки масштаба и повторного использования сценариев | scenario-scale-reviewed.txt; scenario-reuse-reviewed.txt | passed |
| `python3 scripts/verify.py specs`, `python3 scripts/verify.py scenarios` | spec-validation-reviewed.txt; scenarios-reviewed.txt | passed |
| `highgrade inspect` | inspect-reviewed.json | unknown: прежняя область presentation; внешние ссылки не проверялись |

Для браузера использована копия конфигурации `playwright-local.config.ts`: она меняет адрес Storybook на 6007 и абсолютные пути тестов/отчётов. Команда: `HIGHGRADE_CHROME=/usr/bin/chromium HIGHGRADE_UI_REPORT=docs/evidence/result-acceptance/ui-reviewed.json npm test -- --config ../target/highgrade/tmp/simplify-acceptance/playwright.config.ts` из `ui/`; переменная отчёта в исполненной команде содержала абсолютный путь. `ui/playwright.config.ts` не изменён. Проверка установки HG57 пропущена: эта редакция не собрана как выпуск точного SHA и не активирована.

Nextest фиксирует входы до и после запуска. Общий список хешей исходников — `reviewed-source-hashes.json`; он снят во время окончательного прогона и совпал с файлами после завершения. Это дополнительная сверка, а не заявление о предварительном браузерном снимке. Значимых правок во время окончательных прогонов не было. Встроенный `ui/dist/highgrade-ui.json` совпал с хешем снимка Nextest. Его текущий хеш включён в evidence интерфейса. Область входов каждого evidence выбирается по исполненному пути, а не по всему списку этого общего снимка.

## Сопоставление сценариев

| Сценарий | Исполненный путь и наблюдение |
|---|---|
| HG-0012-S3 | Rust `human_history_is_atomic_protected_and_bound_to_spec_and_implementation`: полная история, отказ неготового/устаревшего решения и атомарного пакета; новые Rust-тесты записи без версии и переноса saved version |
| HG-0012-S5 | Rust `isolated_three_spec_lifecycle_keeps_exact_decisions_and_pending_work`: несколько решений, точные версии и незакрытые задачи; нормативная сверка Work/Commit ниже |
| HG-0052-S3 | Attention contracts согласования содержания; Playwright `HG56 S6 S7` сохраняет отдельное согласование без приёмки и запуска работы |
| HG-0052-S4 | Attention contracts readiness/replay и Playwright `HG56 S8 S9`; пустой возврат требований запрещён, возврат результата без текста разрешён |
| HG-0056-S8 | Playwright `HG56 S8 S10` и `HG70 S1 S2 S4`: сохранение, перечитывание, категория и сохранённая версия |
| HG-0056-S9 | Playwright `HG56 S8 S9`, `HG70 S3` и Rust snapshot/readiness: дрейф отклонён; возврат без комментария сохранён |
| HG-0056-S10 | Playwright `HG56 S8 S10`: ответ закрывает только выбранный вопрос; возврат результата отдаёт ход агенту |
| HG-0070-S1 | Playwright `HG70 S1 S2 S4`: нет полей SHA/имени, пустой комментарий, реальный POST/CLI-read и reload; Rust `result_status_without_build_id_keeps_snapshot_history_and_replay_gates` |
| HG-0070-S2 | Rust saved-version, direct-decision и ambiguous-batch тесты; Playwright автоматического переноса `saved-build-v2`; прежние Rust-тесты исторических решений и legacy responses |
| HG-0070-S3 | Rust CAS/readiness/replay; Playwright stale, закрытие/повторное открытие, потерянный ответ до/после записи и GET-восстановление без второго POST |
| HG-0070-S4 | Playwright проверяет полезные факты/адрес/ограничения, скрытые реквизиты, точную исходную строку в UI и store, отсутствие внешних запросов; чтение пути UI/CLI подтверждает отсутствие генерации |

## Нормативная сверка

Сверены `kit/procedures/work.md`, `kit/procedures/commit.md`, `kit/references/tools/specifications.md` и сгенерированные навыки. Эта сверка подтверждает содержание инструкций. Она не доказывает исполнение будущего workflow агентом.

| Условие | Правило после изменения |
|---|---|
| Есть проверенный локальный результат, но нет build ID | Снимок спеки/входов достаточен для записи решения; внешний ID необязателен. Проектные финальные барьеры сохраняются. |
| Build ID уже есть при передаче | Передать существующее значение; дополнительная генерация, commit или build для заполнения формы не нужны. |
| Результат возвращён | Комментарий необязателен. Возврат требований и ответ на вопрос требуют пояснения. |
| Человек выбрал статус | Сохранить решение только по явному действию. Согласование требований, техническая готовность и приёмка имеют разные состояния. |
| Результат принят | Это не разрешает commit, push или активацию. Отдельная обязательная последовательность SHA1 → приёмка → SHA2 удалена. |
| Сохранённый результат изменился | Старое решение остаётся историей; новый существенный снимок требует нового решения. |

## Неудачные попытки и сохранность

`red-rust.txt` воспроизводит прежний отказ без версии. `red-ui-host.txt` показывает прежнюю служебную строку в основном тексте. `red-review-direct.txt` воспроизводит замечание независимого ревью: отсутствие переноса версии при прямом решении. После исправлений соответствующие проверки прошли.

Первые сборки UI и узкие прогоны содержали ошибки JSX, прежних ожиданий и неоднозначного locator; исходные отчёты сохранены. `nextest.txt`, `red-ui.txt`, `scenario-scale.txt`, `scenario-reuse.txt` содержат ограничения песочницы (localhost/дочерние процессы, EPERM). Проверки повторены в разрешённой среде. `ui-full.txt` прерван: порт 6006 обслуживал существующий Storybook Toth. Чужой сервер не остановлен; High Grade проверен на отдельном 6007. Это не успешный прогон High Grade.

`ui-full-host.json` содержит три ошибки подготовки: два теста считали локальный порт 6007 внешним адресом, один fixture передавал отличную от saved version строку. Исправлены origin из фактического baseURL и версия выбранного обращения. Проверки отсутствия внешних запросов и строгого совпадения сохранены.

`presentation-preserved.json` подтверждает совпадение всех 34 исходных файлов `presentation/`. Область не изменена. Её прежний `AreaUnclassified` не устранён изменением чужого реестра. Бюджет чтения после сокращения повторов в документации соблюдён.
