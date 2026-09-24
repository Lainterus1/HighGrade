# Session state: executable scenario adoption

Plan: `openspec/changes/archive/2026-09-24-adopt-executable-scenarios/tasks.md`
Workspace: `D:\my_projects\MyCodex`
Observed: HEAD `b562196faa1505eac638ad54174a8e4442c9e84d`; pre-existing user edits in `Cargo.toml` and `docs/DEVELOPMENT.md` must be preserved.
Status: COMPLETE
Current: complete
Next: none; subsequent candidate approval and publication require their own explicit requests.
Blockers: none

## Progress and evidence

- 1.1: VERIFIED; OpenSpec 1.13.1 strict validation passed; active delta and tasks registered in `.highgrade/project/documents.json`; active v0-2-7 status passed.
- 1.2: VERIFIED; 5 baseline specs had 19 requirements and 38 scenarios; one compound scenario was split into 2. The active change adds 3 requirements and 7 scenarios, making 22/46. Each has a selected method. The full active route is 109,600 bytes over the agreed 96,000-byte ceiling; scoped `tests` is 76,160 bytes. The ceiling remains unchanged and the warning is recorded for review.
- 2.1: VERIFIED; all 19 requirements and 39 scenarios have unique IDs; the compound tool-readiness case was split without changing its outcomes.
- 2.2: VERIFIED; local nextest 0.9.146 ran `bootstrap_reports_all_slots_before_registry_exists` with one pass; real `list.json` and `junit.xml` under ignored `target/nextest/highgrade`; `trace` measured `HG-PB-S04=passed`. The whole file remains `unknown` because 18 scenarios are not yet linked. Offline installation lacked one cached crate; an authorized network install into `target/tools` resolved it without changing the user profile installation.
- 2.3: VERIFIED; full nextest 0.9.146 run passed 74/74 tests, and native `trace` measured 14 automatic scenarios passed and 32 manual scenarios unlinked. Four baseline manual scenarios and seven change scenarios have concrete observations; 21 baseline procedural scenarios remain explicitly open in `docs/reviews/executable-scenarios-review.md`. A temporary-directory fixture collision in `global_contracts.rs` was fixed.
- 3.1-3.3: VERIFIED locally; quality rules, kit procedures/templates, project docs and CI route updated; kit hashes verified; synthetic 100/200-spec fixtures select one 967-byte spec route, reject duplicate IDs/unlinked automatic scenarios, and warn on a 145,839-byte full route. Candidate release and CLI version advanced to v0-2-8/0.2.8 because global-update rejects reinstalling active v0-2-7. Installed v0-2-7 remained untouched.
- 4.1: VERIFIED; strict OpenSpec validation, Rust format/build, native nextest and trace, repository import check, scoped/full inspect and scale checks passed. The first independent review found two P2 defects in report validation and Markdown IDs; the second found two P2 semantic marker mismatches. All four were fixed with focused tests. The third independent review found no substantial issues.
- 4.2: VERIFIED; OpenSpec change archived as `2026-09-24-adopt-executable-scenarios`, its 3 requirements and 7 scenarios merged into the active self-hosted spec. Post-archive native nextest passed 74/74; `trace` measured 14 automatic passes and 32 manual unlinked, with 11 manual observations and 21 explicit gaps recorded in `docs/reviews/executable-scenarios-review.md`. Full context route remains a visible 106,090-byte warning against the unchanged 96,000-byte ceiling; scoped `tests` route is 76,160 bytes. The candidate remains uncommitted, unpublished and not installed; the existing installed release is v0-2-7.
