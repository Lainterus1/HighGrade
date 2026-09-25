# Поштучная сверка сценариев прежнего каталога

Это история решения от 25.09.2026, а не действующий источник требований. Действующие сценарии читаются в `specs/requirements/`; указанный здесь прежний ID нужен только для проверки полноты переноса.

Оценка качества: **точный тест** означает, что локальный тест воспроизвёл вход, действие и проверил наблюдаемый результат; успешный отчёт привязан к сценарию. **Ручной маршрут** означает подготовленную проверку действия агента, которая здесь не выполнялась. **Без явного checks** означает, что тест и отчёт найдены, но старый change не содержит точной связи. Старые доказательства не становятся новыми только от смены ID.

| Прежний сценарий | Судьба | Проверка и качество |
|---|---|---|
| `HG-0002-S1` | HG-0002-S1: тест пройден | Точный тест `tests/catalog_contracts.rs::project_catalog_survives_service_removal_and_preserves_siblings`, `tests/catalog_contracts.rs::foreign_structured_files_are_not_deleted`, `tests/catalog_contracts.rs::selected_directory_is_project_owned_and_self_contained`; сохранён отчёт. |
| `HG-0002-S2` | HG-0002-S2: тест пройден | Точный тест `tests/catalog_contracts.rs::explicit_migration_preserves_all_data_and_detects_legacy_writer`, `tests/catalog_contracts.rs::interrupted_transaction_refuses_reads_and_recovers_only_observed_targets`, `tests/catalog_contracts.rs::recovery_reconstructs_partial_staging_and_missing_first_document`, `tests/catalog_contracts.rs::recovery_cannot_delete_a_required_specification`, `tests/catalog_contracts.rs::malformed_recovery_never_unblocks_catalog`, `tests/catalog_contracts.rs::two_catalog_writers_cannot_allocate_the_same_number`; сохранён отчёт. |
| `HG-0002-S3` | HG-0002-S3: тест пройден | Точный тест `tests/catalog_contracts.rs::integrated_recheck_retains_history_and_invalidates_old_acceptance`; сохранён отчёт. |
| `HG-0003-S1` | HG-0003-S1: тест пройден | Точный тест `tests/catalog_contracts.rs::explicit_links_reject_missing_cycles_and_do_not_inherit_content`; сохранён отчёт. |
| `HG-0003-S2` | HG-0003-S2: тест пройден | Точный тест `tests/catalog_contracts.rs::tags_merge_atomically_and_filters_keep_dependencies_discoverable`; сохранён отчёт. |
| `HG-0003-S3` | HG-0003-S3: тест пройден | Точный тест `tests/catalog_contracts.rs::tags_merge_atomically_and_filters_keep_dependencies_discoverable`; сохранён отчёт. |
| `HG-0004-S1` | HG-0004-S1: тест пройден | Точный тест `tests/catalog_contracts.rs::multiple_checks_cannot_hide_failed_sibling_and_manual_is_explicit`, `tests/catalog_contracts.rs::cargo_runner_requires_a_real_exact_test`; сохранён отчёт. |
| `HG-0004-S2` | HG-0004-S2: тест пройден | Точный тест `tests/catalog_contracts.rs::explicit_runner_records_real_pass_fail_skip_empty_timeout_and_drift`, `tests/catalog_contracts.rs::batch_failure_keeps_prior_results_and_each_scenario_owns_its_report`, `tests/catalog_contracts.rs::cargo_runner_requires_a_real_exact_test`; сохранён отчёт. |
| `HG-0004-S3` | HG-0004-S3: неподтверждён | Ручной сквозной маршрут; прежнее доказательство устарело. Старое ручное доказательство относится к прежней редакции; нужен новый сквозной прогон, сейчас неподтверждено. |
| `HG-0008-S1` | HG-0008-S1: тест пройден | Точный Rust-тест; отчёт `docs/evidence/spec-verification/2026-09-25-nextest.xml`; без явного checks. Текущий отчёт включает оба целевых теста; в старом change нет точного checks. |
| `HG-0008-S2` | HG-0008-S2: тест пройден | Точный Rust-тест; отчёт `docs/evidence/spec-verification/2026-09-25-nextest.xml`; без явного checks. Текущий отчёт включает целевой тест отказа; в старом change нет точного checks. |
| `HG-EXE-S01` | HG-0017-S1: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-EXE-S02` | HG-0017-S2: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-EXE-S03` | HG-0017-S3: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-EXE-S06` | архив | Не относится к текущему поведению. |
| `HG-EXE-S07` | архив | Не относится к текущему поведению. |
| `HG-PB-S01` | HG-0009-S1: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S02` | HG-0009-S2: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S03` | HG-0009-S3: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S04` | HG-0009-S4: тест пройден | Точный тест `tests/bootstrap_contracts.rs::bootstrap_reports_all_slots_before_registry_exists`; сохранён отчёт. |
| `HG-PB-S05` | HG-0009-S5: тест пройден | Точный тест `tests/bootstrap_contracts.rs::alternative_name_is_candidate_but_never_accepted_as_role`; сохранён отчёт. |
| `HG-PB-S06` | HG-0009-S6: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. Тест отчёта не наблюдал последующий поиск агента; теперь только ручной сценарий. |
| `HG-PB-S07` | HG-0009-S7: тест пройден | Точный тест `tests/bootstrap_contracts.rs::complete_project_still_reports_missing_navigation_without_candidate_walk`; сохранён отчёт. |
| `HG-PB-S08` | HG-0009-S8: тест пройден | Точный тест `tests/bootstrap_contracts.rs::migrated_architecture_with_old_readme_link_is_reported`; сохранён отчёт. |
| `HG-PB-S09` | HG-0009-S9: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S10` | HG-0009-S10: тест пройден | Точный тест `tests/bootstrap_contracts.rs::exclusions_are_safe_and_bad_registry_does_not_hide_candidates`; сохранён отчёт. |
| `HG-PB-S11` | HG-0009-S11: тест пройден | Точный тест `tests/bootstrap_contracts.rs::alternative_name_is_candidate_but_never_accepted_as_role`; сохранён отчёт. |
| `HG-PB-S12` | HG-0009-S12: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S13` | HG-0009-S13: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S20` | HG-0009-S14: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S14` | HG-0009-S15: тест пройден | Точный тест `tests/bootstrap_contracts.rs::missing_link_is_distinct_from_missing_file_and_registry_path_mismatch`; сохранён отчёт. |
| `HG-PB-S15` | HG-0009-S16: тест пройден | Точный тест `tests/bootstrap_contracts.rs::canonical_project_has_separate_path_and_link_evidence`; сохранён отчёт. |
| `HG-PB-S16` | HG-0009-S17: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S17` | HG-0009-S18: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PB-S18` | HG-0009-S19: тест пройден | Точный тест `tests/bootstrap_contracts.rs::canonical_project_has_separate_path_and_link_evidence`; сохранён отчёт. |
| `HG-PB-S19` | HG-0009-S20: тест пройден | Точный тест `tests/bootstrap_contracts.rs::bootstrap_reports_multiple_unvisited_locations_with_a_count`; сохранён отчёт. |
| `HG-PD-S01` | HG-0010-S1: тест пройден | Точный тест `tests/global_contracts.rs::installed_runtime_references_survive_removal_of_source_copy`; сохранён отчёт. Тест проверяет установленную копию и ссылки; неизменность маршрутизаторов в этом сценарии не покрыта. |
| `HG-PD-S02` | HG-0010-S2: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. Структура реестра проверена; смысловая адаптация требует отдельного наблюдения. |
| `HG-PD-S03` | HG-0010-S3: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S05` | HG-0010-S4: тест пройден | Точный тест `tests/global_contracts.rs::shipped_registry_example_is_accepted_without_inventing_budgets`; сохранён отчёт. |
| `HG-PD-S04` | HG-0010-S5: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S06` | HG-0010-S6: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S07` | HG-0010-S7: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S08` | HG-0010-S8: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S09` | HG-0010-S9: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S10` | HG-0010-S10: тест пройден | Точный тест `tests/global_contracts.rs::supported_adapter_does_not_claim_semantic_readiness`; сохранён отчёт. |
| `HG-PD-S11` | HG-0010-S11: тест пройден | Точный тест `tests/global_contracts.rs::unsupported_project_adapter_is_a_failure`; сохранён отчёт. |
| `HG-PD-S12` | HG-0010-S12: тест пройден | Точный тест `tests/global_contracts.rs::global_update_preserves_project_adaptation_in_profile`; сохранён отчёт. |
| `HG-PD-S13` | HG-0010-S13: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S14` | HG-0010-S14: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S15` | HG-0010-S15: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-PD-S16` | HG-0010-S16: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S01` | HG-0011-S1: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S02` | HG-0011-S2: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S10` | HG-0011-S3: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S03` | HG-0011-S4: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S11` | HG-0011-S5: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S04` | HG-0011-S6: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S05` | HG-0011-S7: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S06` | HG-0011-S8: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S07` | HG-0011-S9: тест пройден | Точный тест `tests/global_contracts.rs::global_update_preview_switch_and_cleanup_keep_unrelated_files`; сохранён отчёт. |
| `HG-RW-S12` | HG-0011-S10: тест пройден | Точный тест `tests/global_contracts.rs::update_preserves_foreign_file_in_old_release`; сохранён отчёт. |
| `HG-RW-S08` | архив | Не относится к текущему поведению. Одноразовое переключение старой версии; тест остаётся регрессией Update. |
| `HG-RW-S13` | HG-0011-S11: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-RW-S09` | HG-0011-S12: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-SH-S01` | HG-0014-S1: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-SH-S02` | HG-0014-S2: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-SH-S05` | архив | Не относится к текущему поведению. |
| `HG-SH-S06` | архив | Не относится к текущему поведению. |
| `HG-SP-S1` | HG-0012-S1: тест пройден | Точный тест `tests/spec_contracts.rs::automatic_numbers_survive_abandon_failure_and_competing_cli_writers`; сохранён отчёт. |
| `HG-SP-S2` | HG-0012-S2: тест пройден | Точный тест `tests/spec_contracts.rs::v1_migration_preserves_exact_backup_ids_evidence_and_review_digest`; сохранён отчёт. |
| `HG-SP-S3` | HG-0012-S3: тест пройден | Точный тест `tests/spec_contracts.rs::human_history_is_atomic_protected_and_bound_to_spec_and_implementation`; сохранён отчёт. |
| `HG-SP-S4` | HG-0012-S4: тест пройден | Точный тест `tests/spec_contracts.rs::json_progress_rejects_missing_skipped_unknown_and_negative_review`; сохранён отчёт. |
| `HG-SP-S5` | HG-0012-S5: тест пройден | Точный тест `tests/spec_contracts.rs::isolated_three_spec_lifecycle_keeps_exact_decisions_and_pending_work`; сохранён отчёт. |
| `HG-SS-S01` | HG-0013-S1: тест пройден | Точный тест `tests/spec_contracts.rs::drafts_are_readable_but_unknown_versions_and_fields_never_write`; сохранён отчёт. |
| `HG-SS-S02` | HG-0013-S2: тест пройден | Точный тест `tests/spec_contracts.rs::competing_processes_and_failed_replace_preserve_whole_store`; сохранён отчёт. |
| `HG-SS-S03` | HG-0013-S3: тест пройден | Точный тест `tests/spec_contracts.rs::integration_rejects_stale_base_and_requires_removal_reason`; сохранён отчёт. |
| `HG-SS-S04` | HG-0013-S4: тест пройден | Точный тест `tests/spec_contracts.rs::evidence_and_review_go_stale_without_losing_unaffected_observations`; сохранён отчёт. |
| `HG-SS-S05` | HG-0013-S5: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-SS-S06` | HG-0008-S3: тест пройден | Точный тест `tests/trace_contracts.rs::reviewed_transfer_trace_selects_one_owner_and_preserves_neighbor`; сохранён отчёт. Долговременная гарантия выборочного переноса сохранена и проверена цепочкой import → transfer → trace. |
| `HG-TR-S01` | HG-0015-S1: тест пройден | Точный тест `tests/global_contracts.rs::doctor_reports_unavailable_tool_without_claiming_version`; сохранён отчёт. |
| `HG-TR-S03` | архив | Не относится к текущему поведению. Проектная процедура не проверяла заявленную диагностику doctor. |
| `HG-TW-S01` | HG-0016-S1: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-TW-S02` | HG-0016-S2: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |
| `HG-TW-S03` | HG-0016-S3: неподтверждён | Ручной маршрут: исходное состояние, действие и наблюдение указаны в checks; испытание не выполнено. |

Итог: 85 старых сценариев рассмотрены поштучно; 6 сняты, 38 имеют свежий локальный тестовый отчёт (36 с точным checks и 2 с сохранённым отчётом без checks), 41 остаётся неподтверждённым. Из 41 один имеет устаревшее ручное доказательство, которое не засчитывается для текущей редакции. Открытое HG-0006 здесь не оценивалось.
