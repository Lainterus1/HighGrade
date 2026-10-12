# CI и локальная готовность — HG-0072

Поручение автора 2026-10-12: убрать ожидание CI, исправить Linux-терминологию и оставить эффективные важные проверки. Цепочка HG-0071/HG-0072 использует один ещё неактивированный v0-3-12. Commit, push и activation поручены; приёмка не поручена. Посторонняя .gitignore исключена.

## Нормативная сверка

| Условие | Ожидаемое действие |
|---|---|
| Локальные проверки/ревью прошли; CI pending/unavailable | Завершить Work и разрешённые действия; сообщить статус CI отдельно |
| CI success другого SHA | Не объявлять текущий CI PASS |
| CI failed с ошибкой поведения | Назвать ошибку, исправить её и проверить затронутый состав |
| CI failed из-за среды | Назвать ограничение; не выдавать отсутствие исполнения за PASS |
| Пользователь/подключаемый проект явно требует ожидания | Соблюсти только назначенный зависимый барьер |
| Push успешен | Проверить удалённый SHA и один доступный снимок CI; это не приёмка, merge или deploy |

Сверены Work/Commit/Push/Planner, Init, шаблон и местная инструкция. HG-0063 согласован только в части CI; остальные задачи и его интеграция не закрываются. Прежние записи сохраняют историю, но не доказывают изменённый контракт. Нормативное ревью не является наблюдением будущего агента.

Поддержка названа Linux/Windows. Местная ОС: /etc/os-release — Omarchy, ID_LIKE=arch; kernel Linux. ubuntu-24.04 сохранён только как образ GitHub Linux runner. Исторические отчёты не переписаны. Матрица и команды принадлежат [DEVELOPMENT](../../DEVELOPMENT.md#общие-проверки-и-ci), установочный путь — [BUILD](../../BUILD.md#обязательный-installed-e2e).

## Основание сокращений

Прежний CI 30376f4 завершился success на обеих ОС; [неизменяемый исходный снимок](../objects/e2a5cb48c692b189dcfc47f1e4c40dda62d638a39d0f6b5315add6d017176b09.json) содержит exact SHA и timings. Run: [38228729531](https://github.com/Lainterus1/HighGrade/actions/runs/38228729531).

| Шаг прежнего запуска | Linux | Windows |
|---|---|---|
| Полная задача | 11m10s | 14m10s |
| specs validation | 3m50s | 3m10s |
| Native UI | 1m10s | 1m48s |
| Исторический release build | 29s | 1m22s |
| Storybook build | 6s | 10s |

Это длительности одного старого запуска. Численная экономия нового workflow не измерена. Общие проверки и полный native UI оставлены на Linux; Python lifecycle, Rust inventory/run/trace, Node guards и installed upgrade/recovery сохранены на обеих ОС. Storybook строится один раз и продолжает проверять stories/imports/config. Тонкие native UI отказы Windows отдельно не покрываются; конкретный Windows UI дефект требует адресного прогона.

Cargo cache обновляется по exact source SHA с fallback по платформе/toolchain/dependencies. Predecessor cache содержит только готовый кандидат одной ОС/архитектуры/закреплённого SHA; cache miss использует прежний собственный builder. Installed fixture проверяет полный состав, байты, версию и probe исполняемого файла после обоих режимов. Кэш не заменяет выполнение. Новый push отменяет старый запуск своей ветки. Третий job, path filters и новые раннеры не добавлены. В artifacts включены run.json/trace.json вместе с inventory/completion/JUnit.

## Доступные проверки

- [Python suite: 68 passed](../objects/bde6765fddf59046b8dd253876db3075991d9fc2dfee5445adb9c76c63cdc34c.json); реальные Git/Cargo lifecycle, UI guards и сохранность. Предыдущая попытка застала DEVELOPMENT 9033 bytes и отклонила её; итог — 8965 bytes, лимит 9000 не повышен.
- [Node 24: 10 passed](../objects/7a2afd406b4daca29da4bc3996b9958f10f766c2058b41322965f34e9c691f6b.json); required-mode и неверные SHA/версии/байты отвергаются. Запуск без sandbox нужен для настоящих дочерних процессов.
- [Nextest: 224 passed, 0 skipped](../objects/b53feed0263fba4ff7f6b054c1f43a88999262311ca3b05a6703eb0c754d6dbc.json); пакетная CLI совпадает с адресными findings, не пишет каталог, исключает отменённое и сохраняет ActiveSpecConflict. spec-check продолжает отвергать незавершённую задачу.
- [Trace: 224 executed, 0 missing/skipped/unknown](../objects/2c8307264e0b8d1edf285952dac2869a3a94f324834067abda732d755ec1ea6e.json); 137 automatic passed из 137; 164 сценария вне автоматической трассировки не объявлены успешными.
- [Scale 100/200](../objects/329ac2b8f03ef5e902d9251e60e8cfa9c94ebc6cd2e7aaa1d7cda5f65a783f5a.json) и [reuse guard](../objects/0edf44d22b2a5f1f190f36048cb0190e1aa42a21f912dac5babb8226dbb73202.json) прошли. Первые sandbox-попытки не исполнили дочерний Node (EPERM); повтор со штатным доступом прошёл.
- [actionlint 1.7.12](../objects/92840dfda88996679e7c9a8a366eb0c02f349c3417f90bb6d7c8ee521ca3fa1c.json) прошёл; shellcheck/pyflakes отключены. YAML отдельно прочитан; fmt, render-skills --check, check-repository и diff --check прошли.

specs-фаза использует одну пакетную spec-validate вместо 69 повторных чтений; полнота структуры проверяется теми же правилами и глобальной проверкой конфликтов. Раннее локальное одновременное чтение дало StoreBusy; штатный последовательный повтор прошёл. Это не remote PASS.

Inspect: общий маршрут 102058 bytes > 95600; бюджет не повышен. Посторонняя локальная presentation остаётся AreaUnclassified. Внешние ссылки отмечены unchecked; официальные страницы runner/cache прочитаны отдельно.

Новый remote CI, реальный installed E2E и активация фиксируются отдельно после точного коммита. Прежний CI не объявляется проверкой новой редакции. Успех технической установки не означает пользовательскую приёмку.
