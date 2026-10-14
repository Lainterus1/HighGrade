# HG-0076: полнота Clear и батчи

Локальная реализация проверена на Linux. HG-0063-T10 проверена отдельно; другие задачи HG-0063 остаются вне этой цепочки. Commit, activation, push, Windows CI и human acceptance не выполнялись. Presentation не проверялась.

Clear требует смыслового аудита всех текущих контрактов, связанных с выбранными тестами. Методы извлекаются до baseline. Helper поставляется автономно, принимает явный JSON адаптер реального проекта, выполняет по одной мутации и сохраняет точные inputs/plan/diffs/оригиналы. Это copy isolation, не security sandbox. Shared baseline/final и reuse требуют неизменных причинных входов; неизвестные не переносятся как PASS.

Текущий Python suite:78PASS; после последних helper правок адресно9PASS. Final Rust:239/239, без skip/fail. Native C1:1PASS/37filtered, final helper8ab257dc… и manifest1d6df824… совпадают before/after/current. Общие structure, render-skills, fmt и scoped spec-validate PASS. Inspect: только прежние external-link warnings; route95583≤95600. Предварительный Rust sandbox EPERM сохранён как неудачный запуск, не detected.

Оригиналы fixtures/actor/actual refresh/методов/адресных checks: docs/evidence/objects/6a17ccfa2dcfb1cae22d1af63710c1c90eddf0a7dc915846fcc026f27d203eb8.json

Оригиналы final Rust: docs/evidence/objects/5dc9f435847c3d984cb85a7f9c91084c1226250858ef222f3a61c34d4c4267cd.json

Исходная trace перед окончательными catalog metadata операциями: docs/evidence/objects/4cb65dfd7d9029b2b68b1ff872f79a42409dd459b37ea80083a75283093edb21.json. После integration prepare/trace/verify выполняются отдельно; этот отчёт не выдаёт старую trace за текущую.

Последний native C1: specs/changes/HG-0076/evidence/HG-0076-C1-1791970451332548748.log. Общий scope сценариев и оригиналы различаются ниже.

| Сценарий | Подтверждённый исход / предел |
|---|---|
| HG-0075-S1 | Нормативное независимое ревью: один раунд только неизвестных параметров; будущая стабильность модели не доказана. |
| HG-0075-S2 | Известные параметры / анализ без Goal: actor semantic_all; повторных вопросов и исполнения нет. |
| HG-0076-S1 | Один настоящий root Goal до цепочки; actual повторное get_goal перед bounded Clear; тот же Goal, без второго Goal. Controlled кейсы отдельно. |
| HG-0075-S3 | Прочитаны все основные области minimal fixture, включая скрытый config; coverage и methods отделены от перечня файлов. |
| HG-0075-S4 | Обнаружено current F-R1=2 против code/test/doc=1; actual RED before source fix, затем source/doc correction и final3PASS. |
| HG-0076-S3 | Оба связанных текущих F-R прочитаны по смыслу; зелёный старый тест выявлен; unlinked roundtrip оставлен у engineering. Before/after method назначен до baseline. |
| HG-0075-S5 | Реальный survivor weak assertion сохранён; тот же diff обнаружен strong assertion; восстановленные проверки прошли. |
| HG-0075-S6 | Compile/setup, timeout, skip, empty, unsupported и stale-source report не detected; baseline failure не оценивает мутации. |
| HG-0075-S7 | Controlled partial сохраняет полный охват и конкретный Next; реальный этап после sandbox failure продолжен от непроверенного действия. |
| HG-0075-S8 | Read-only fixture bytes и sentinel сохранены. Документы-only не назначают mutation audit по независимому смысловому ревью. Unrelated inherited bytes сохранены. |
| HG-0076-S4 | Controlled все3mutations detected не закрывает непрочитанный F-R2 и unavailable required observation; partial, Next открыт. |
| HG-0076-S5 | Реальный установленный helper после удаления source copy; isolated Python project, baseline/mutation/final; exact bytes. Также9subprocess regressions. |
| HG-0076-S6 | Отдельные причины исходов, normal assertion exit only; слабый/сильный тест одного нарушения, исходная история сохранена. |
| HG-0076-S7 | Unknown third-state restore останавливает worker и сохраняет bytes; foreign/live sentinel цел. Existing outputs/jobs2 отклонены. Descendant процесс не даёт detection/продолжение. |
| HG-0076-S8 | Compact refs, exact saved plan, один causal snapshot на batch, оригиналы не обрезаны. Сохранены существующим CAS, без второго каталога. |
| HG-0076-S9 | Complete matching intact result reused; contract/helper/fixture/dependency/environment change, unknown scope и corrupt original дают fresh execution. |
| HG-0076-S10 | Одинаковые3detected: test commands9→5, probes3→1, helper starts3→1. Подготовка fixture измерена; разработка adapter/helper и токены — нет данных. |

Сравнение на одной небольшой fixture:0.766s→0.386s, raw preserved evidence18376→7901bytes. Время включает helper и probes. Размер исключает рабочие копии; все выбранные обязательные исходы одинаковы. Fixture preparation0.000275s, adapter2515bytes. Cost разработки и токены неизвестны. Это подтверждение сокращения повторных команд/данных на этой fixture; общий net effect и экономия любого проекта не заявлены. Полное внедрение включало дополнительные проверки и повтор после ограничения среды.

Независимые controlled упражнения показывают фактические чтения/решения агента на этих входах. Они не доказывают живой отказ API, deployed observation или все будущие действия. Положительный root Goal и bounded refresh наблюдались отдельно.

Export verified before cleanup. Отдельный closure record хранит integration, повтор trace и обслуживание. Унаследованные чужие/необработанные runs сохраняются.
