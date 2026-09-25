# Архив High Grade

Назначение: сохранить историю и доказательства, не включать заменённые инструкции в обычный маршрут работы. Действующий маршрут — в [GLOBAL](../workflow/GLOBAL.md), правила контекста — в [CONTEXT](../workflow/CONTEXT.md), история решений — в [SPECIFICATION](workflow/SPECIFICATION.md). Установленный выпуск проверяет `global-status`; последний оформленный выпуск — [v0-2-16](../releases/v0-2-16.md).

| Материал | Почему здесь | Замена или статус |
|---|---|---|
| [Ранний проект организации](designs/2026-09-22/mycodex-organization.md) | Автор уточнил: сначала отдельное сопровождение плагинов, распределение по направлениям позже | [ARCHITECTURE](../ARCHITECTURE.md) и [plugins/README](../../plugins/README.md) |
| [Исходный протокол концепции](designs/2026-09-22/codex-workflow-design.md) | Накопил обсуждение, промежуточные статусы и отчёты; сохранён без редактирования | История — [SPECIFICATION](workflow/SPECIFICATION.md), действующие правила — [CONTEXT](../workflow/CONTEXT.md) |
| [Протокол дополнения о контексте](designs/2026-09-22/context-governance-proposal.md) | Обсуждение завершено, действующие правила выделены | [CONTEXT](../workflow/CONTEXT.md) |
| [Исходный технический проект](designs/2026-09-22/mycodex-technical-design.md) | Снимок, по которому проводилось прежнее ревью | [Технический проект](legacy-workflow/proposals/technical-design.md) — исторический кандидат |
| [Предыдущее ревью концепции](designs/2026-09-22/codex-workflow-review.md) | Исторический обзор более ранней концепции | [Ревью организации](../reviews/project-organization-review.md) |
| [Завершённый bootstrap](plans/bootstrap.md) | Перенос плагинов и начальные документы уже выполнены | [Манифест импорта](../imports/plugins-import.json) и действующая документация |
| [Проверка и канонизация спек](plans/2026-09-25-spec-verification.md) | Завершён поштучный разбор прежних правил и сценариев | [Итог](plans/2026-09-25-spec-verification.state.md), [архивная карта и ревью](legacy-native-specs/2026-09-25/README.md); действующее поведение — в `specs/` |
| [Завершённые планы 25 сентября](plans/2026-09-25-target-layout.md) | Публичный вход, подтверждение спек, очистка репозитория, выбор маршрута и структура `target/` завершили согласованную работу | Состояния и точные ограничения остались в `plans/`; текущие задачи — в `../plans/` |
| [Упорядочивание `docs/`](plans/2026-09-25-docs-organization.md) | Действующие правила отделены от завершённых планов и длинной истории решений | [Итог и проверки](plans/2026-09-25-docs-organization.state.md); навигация — в [карте документов](../README.md) |
| [Highgrade Planner](plans/2026-09-25-highgrade-planner.md) | План HG-0020 завершён после проверки и интеграции спеки | [Итоговое состояние](plans/2026-09-25-highgrade-planner.state.md), [техническая передача](../../specs/changes/HG-0020/evidence/handoff.md); локальное принятие отдельно |

Ранние восстановительные снимки с SHA256 сохраняют исходные байты. В позднее перенесённых Markdown исправлены пути ссылок; их утверждения о статусах и следующих шагах относятся к моменту записи. Обычная работа не должна брать из архива действующие указания. Текущие требования — в [нативном каталоге](../../specs/README.md). История первого переноса — в [карте](legacy-openspec/migration.json), история канонизации — в [архивном снимке](legacy-native-specs/2026-09-25/README.md).

Планы многоэтапных задач до завершения находятся в `docs/plans/`, затем архивируются. Документальные снимки в этом архиве не доказывают наличие Git-истории. Исполняемый код плагинов и существующий скрипт сюда не перемещались.

## Снимок до организации

В `project-docs/2026-09-22-before-organization/` сохранены прежние пять основных документов и реестр. Их относительные пути относятся к первоначальному расположению, а не к текущей навигации. Это восстановительный снимок; актуальные ссылки проверяются отдельно. [Манифест перемещений и SHA256](../imports/organization-archive.json).

## Завершённая организация

[План](plans/2026-09-22-project-organization.md) и [состояние DONE](plans/2026-09-22-project-organization.state.md). Результат и пределы проверки: [ревью](../reviews/project-organization-review.md). Прежняя последовательность первого выпуска тоже сохранена [в архиве](plans/first-release.md); завершённая организация не возобновляется.

## Планы заменённой поставки

[План первого выпуска](plans/first-release.md) и [состояния P0–P2](plans/2026-09-23-p0-p2.state.md), [P3–P6](plans/2026-09-23-p3-p6.state.md) сохранены как история проектного кандидата. P3–P6 не считается целиком завершённым: пилот SNAF ожидал приёмки, PlantsNotify не начинался; новый глобальный маршрут заменил этот план, а пилоты остаются отложенными.

[План глобальной поставки](plans/2026-09-23-global-workflow.md) и [состояние COMPLETE](plans/2026-09-23-global-workflow.state.md) архивированы после публикации v0-2-1. Их указания о неподключённом OpenSpec отражают момент записи. Действующий маршрут — в [GLOBAL](../workflow/GLOBAL.md), требования — в `specs/`.

## Самоприменение workflow, 2026-09-23

Проверенное изменение [self-host-highgrade](legacy-openspec/changes/archive/2026-09-23-self-host-highgrade/proposal.md) было архивировано OpenSpec; его тогдашняя [спецификация](legacy-openspec/specs/self-hosted-workflow/spec.md) теперь также историческая. Завершённые [план](plans/2026-09-23-self-hosting.md) и [состояние](plans/2026-09-23-self-hosting.state.md) сохранены здесь. Итоговые проверки и их пределы — в [ревью](../reviews/self-hosting-review.md).

## Проверяемые сценарии, 2026-09-24

[Изменение adopt-executable-scenarios](legacy-openspec/changes/archive/2026-09-24-adopt-executable-scenarios/proposal.md) архивировано после тогдашнего объединения требований с [исходной спекой](legacy-openspec/specs/self-hosted-workflow/spec.md). [Проверки и открытые ручные пробелы](../reviews/executable-scenarios-review.md) сохраняют границу доказательства, а [состояние задачи](plans/2026-09-24-executable-scenarios.state.md) — ход работы. Установленная v0-2-7 не переключалась; исходный кандидат v0-2-8 не опубликован.

[Упрощение маршрута High Grade](legacy-openspec/changes/archive/2026-09-24-simplify-highgrade-workflow/proposal.md) завершено и архивировано; требования объединены в действующие спеки bootstrap, task и release. Независимое ревью выявило ошибку повторной очистки выпуска, исправленную до архивации. На момент архивации исходный кандидат v0-2-9 не активирован и не опубликован.

## Карта репозитория, 2026-09-23

[Изменение repository-map](legacy-openspec/changes/archive/2026-09-23-repository-map/proposal.md) завершено и архивировано OpenSpec. Его [спецификация](legacy-openspec/changes/archive/2026-09-23-repository-map/specs/repository-map/spec.md) и [ревью кандидата](reviews/repository-map-review.md) сохраняют историческое поведение карты.

[Уточнение по SNAF](legacy-openspec/changes/archive/2026-09-23-repository-map-project-context/proposal.md) также завершено и архивировано. Его результат и пределы проверки — в [отдельном ревью](reviews/repository-map-project-context-review.md); карта затем заменена проверкой структуры документов.

## Структура документов перед инициализацией, 2026-09-23

[Изменение bootstrap-document-structure](legacy-openspec/changes/archive/2026-09-23-bootstrap-document-structure/proposal.md) завершено и архивировано OpenSpec. Публичная карта удалена; прежняя [спецификация](legacy-openspec/specs/project-bootstrap/spec.md) и [ревью кандидата](reviews/bootstrap-document-structure-review.md) описывают `inspect --bootstrap`, канонические пути, ограничения сканирования и проверку SNAF.

[Уточнение передачи bootstrap в init](legacy-openspec/changes/archive/2026-09-23-improve-bootstrap-init-handoff/proposal.md) также архивировано. Итоги временного пилота, проверки кандидата v0-2-4 и границы доказательства — в [ревью](reviews/bootstrap-init-handoff-review.md); действующий контракт объединён в спецификации `project-bootstrap`. Временный проект удалён.

## Выпуск принятой доработки, 2026-09-23

[Изменение deploy](legacy-openspec/changes/archive/2026-09-23-deploy-accepted-work/proposal.md) завершено и архивировано OpenSpec. Прежний [контракт выпуска](legacy-openspec/specs/release-workflow/spec.md) и [ревью кандидата](../reviews/deploy-release-review.md) описывают отбор, очистку, публикацию и активацию. Фактические push и локальная активация проверяются отдельно.

[Уточнение highgrade-deploy](legacy-openspec/changes/archive/2026-09-23-deploy-publication-consent/proposal.md) задаёт локальный коммит по умолчанию, отдельные полномочия на публикацию и активацию и переименование маршрутизатора. Итоговое [ревью](reviews/deploy-publication-consent-review.md) фиксирует проверки кандидата v0-2-6 и границу с пользовательской установкой; действующие требования объединены в контракте выпуска.

## Сведение первого выпуска, 2026-09-23

В `project-docs/2026-09-23-before-first-release-consolidation/` сохранены изменяемые активные документы до консолидации. `manifest.json` в этом снимке содержит исходные SHA256 и список путей; проверяется сохранность байтов. Предыдущие снимки `2026-09-23-before-*` сохраняют отдельные шаги согласования. Это история, не действующие инструкции; относительные ссылки отражают расположение в исходном проекте.

Результат консолидации — [ревью](../reviews/first-release-review.md). Текущее устройство описано в [ARCHITECTURE](../ARCHITECTURE.md), маршрут — в [GLOBAL](../workflow/GLOBAL.md), правила контекста — в [CONTEXT](../workflow/CONTEXT.md). TOOLING и план первого выпуска остаются историей; снимки не создают альтернативный backlog.

## Реализация P0–P2, 2026-09-23

Исходные изменяемые документы сохранены в `project-docs/2026-09-23-before-p0-p2/` с manifest.json и SHA256. Причина: прежние указания «код не разрешён/не создан» заменены по явному поручению автора. Текущие указания находятся в основных документах; [контракт P0–P2](legacy-workflow/implementation/p0-p2-contract.md) сохраняет историю этапа. Это восстановительный снимок; его относительные ссылки относятся к прежнему расположению.

## Реализация P3–P6a, 2026-09-23

Прежние версии семи основных документов сохранены в `project-docs/2026-09-23-before-p3-p6/` с manifest.json и SHA256. Действующие указания находятся в основных документах; [контракт P3–P5](legacy-workflow/implementation/p3-p5-contract.md) и [проверка кандидата](../reviews/p3-p6-candidate-review.md) сохраняют историю этапа. Снимок сохраняет первоначальные относительные ссылки и не заменяет текущий план.

## Уборка 2026-09-25

Завершённые планы 24 сентября — в plans/2026-09-24; заменённые проекты и контракты — в legacy-workflow, неактивные ревью — в reviews. Пример v1 — [в архиве](examples/native-specification/README.md), действующий v3 — [в документации](../examples/spec-catalog/README.md). Две одинаковые миграционные копии store v2 сведены к одной в data. [Пути и хеши](../imports/repository-hygiene.json) фиксируют переносы; у этих Markdown исправлены относительные ссылки, JSON сохранён побайтно.

[Манифест очистки target](../imports/target-cleanup.json) связывает старые пути с сохранёнными первичными результатами. Полные журналы пилотов и профили сохранены локально в .highgrade/local/archive/target-history.zip; они не входят в Git. Два файла, на которые напрямую ссылается неизменяемая OpenSpec-история, также оставлены по прежним путям. Сборочные копии воспроизводимы из Git; основной кэш Cargo сохранён.
