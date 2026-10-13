<a id="проверки-поставки-в-проекте-high-grade"></a>

# Verify the Delivery in the High Grade Project

Scope: this repository's Delivery. Installed kit/ owns [adaptation](../../kit/references/audit.md#context-and-adaptation), [execution/evidence](../../kit/procedures/work.md#evidence-and-review) and [archive](../../kit/procedures/archive.md). Connected projects do not depend on this file. [SPECIFICATION](../archive/workflow/SPECIFICATION.md) holds history.

<a id="адаптация-проекта"></a>

## Project adaptation

Check installed-package independence and adaptation to empty/existing projects. Files/hashes prove content, not skill execution. Agent exercises record input, actions, observation, candidate state and limits.

<a id="выполнение-и-ревью"></a>

## Execution and review

[DEVELOPMENT](../DEVELOPMENT.md) owns commands; the [local instruction](../../.highgrade/project/INSTRUCTIONS.md) owns review authority. Common work criteria apply. CLI checks use temporary data. Product projects need separately authorized pilots.

<a id="доказательства"></a>

## Evidence

Requirements/scenarios have stable unique IDs. Native changes need validation, observable checks under DEVELOPMENT, evidence and independent review. spec-integrate checks readiness itself; no mandatory extra spec-check. Normative edits use semantic review, not mandatory agent pilots. Agent-recorded passed is not proof of execution; check original reports/input scope. Historical sources are immutable; [archive](../archive/legacy-native-specs/2026-09-25/) retains transfer/canonicalization decisions. Future selective-import spec-transfer records only checked contract: it neither confirms behavior nor closes spec-check nor records human decisions.

source-scenarios.mjs checks current native catalog and automatic Rust links without loading archive. trace gets catalog/semantic trace_sha256; CI checks actual Nextest reports. PASS closes no manual scenarios and replaces no spec-check. Rust Nextest/Playwright are supported; other formats/manual observations need explicit comparison.

CI checks IDs, automatic-test linkage, source freshness and actual results. Skipped/failed/stale/unknown links cannot pass. outside_automatic_trace marks report scope, not manual acceptance. Rust test, linked-test and scenario counts have different denominators. Show manual/no-check scenarios separately without fictitious coverage.

<a id="границы-выпуска"></a>

## Release boundaries

Delivery distributes source for Windows/Linux through GitHub. Asynchronous CI checks exact SHA and installed-E2E under [BUILD](../BUILD.md#required-installed-e2e). Local checks/review readiness needs no CI wait; report its observed status separately under local rules. Distinguish local checks, candidate installation, pilot, publication, user-profile activation and acceptance. The bounded [pilot](../archive/plans/2026-09-24/2026-09-24-portable-delivery.md) is complete; it grants no authority for further connected-project work or production deploy.

<a id="проектный-каталог-и-повторные-проверки"></a>

## Project catalog and rechecks

Changes live in specs/changes/`<ID>`/spec.json; results/decisions in results.json. checks defines reproducible setup, action, observation, file/selector and causal inputs. Explicit spec-run uses standard runners; catalog reads execute nothing. One passed test cannot hide a failing bound check of the same scenario. Empty, skipped, timed-out or input-drift results remain unconfirmed. Equivalent repeats retain current review/decision; material changes need reconsideration. Old fingerprints use the prior conservative algorithm. Independent review checks input completeness, expectation meaning and original reports.
