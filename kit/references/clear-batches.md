# Isolated Clear batches

[Review](review.md#mutation-checks) owns selection. Use approved Python 3.12+; no source repository or dependency installation is required.

Run `python <active-release>/tools/clear_batch.py --root <project> --plan clear-plan.json --output <new-owned-output>`. Pass `--reuse <previous-output>/summary.json` only after checking scope. Exit 0 means all selected mutations were detected with passing baseline/final and exact restoration. Exit 1 means a recorded open result; exit 2 means an invalid plan or unavailable operation. Keep failed output.

This is copy isolation, not a security sandbox. Review commands and dependencies. Commands must work in the copied subject and use its sources/resources. Runtime probes must be read-only. No live data, network/service actions or global dependency installation are assigned. Only jobs=1 is supported. Use separate outputs/resources for separate calls. Catalog writes remain serial.

## Plan and observation

Schema 1:

- `inputs`: complete relative files to copy and fingerprint, including source, tests, contracts, fixtures, config, shared modules and dependencies. Reject directories, links, traversal, `.pyc`, Windows aliases (trailing dot/space or case collisions) and the root `.hg-clear-report.json` in any case.
- `tests`, `contracts`: nonempty subsets of inputs; they cannot be mutated.
- `scope_complete`: true only after review confirms all causal dependencies, including runtime/external state. Unknown scope prevents reuse. It does not make a supplied adapter trustworthy.
- `runtime_probes`: argv arrays that report relevant runtime/dependency versions. No shell. `timeout_seconds` defaults to 60.
- `checks`: unique `id`, argv `command`, exact `expected_tests` IDs and witnessed `sources`. Optional `assertion_exit_codes` defaults to `[1]`; choose the framework's normal test failure codes.
- `mutations`: unique `id`, implementation `path`, UTF-8 `before`/`after` strings, selected `checks`, `requirement`, `scenario` and `failures` containing expected `test` and a nonempty `contains` assertion message. Each replacement must match once. Confirm independently that the change violates the contract.

The only supported adapter report is JSON `highgrade-clear-v1`. The command writes a fresh report to `HG_CLEAR_REPORT`:

```json
{"format":"highgrade-clear-v1","tests":[{"id":"test_number","outcome":"assertion","message":"number contract: expected 2, got 0"}],"source_hashes":{"calc.py":"SHA256 of the source actually compiled or loaded"}}
```

Allowed test outcomes are `passed`, `assertion`, `error`, `skipped`. Preserve real framework results; do not invent cases from exit codes. A passing command needs exit 0 and every expected case passed. Detection needs an executed relevant assertion and a nonzero normal test exit. Error, skip, empty/duplicate selection, timeout, unsupported/malformed report, missing or stale source witness is `invalid`. These are not equivalent mutations or detected violations. A valid passing mutant is `survived`; strengthen the test and repeat the same violation plus restored check. Initial results remain history.

The adapter must bind its actual execution artifact to source bytes. A current file hash does not prove which bytes a cached binary executed. For Python, compile the read bytes and record their hash from that same load. For compiled projects, record the build's input hash in the executed artifact and invalidate caches as needed. If this binding cannot be established, keep the execution gap. Other report formats require a reviewed project adapter or the project's own mutation tool.

## Preservation and reuse

The helper saves copied inputs, one snapshot, mutation diffs and each command's stdout/stderr, raw adapter report and run record. It checks hashes/modes after each restore; unknown edits survive and stop the worker. Baseline/final and repeat rules follow the [mutation method](review.md#mutation-checks).

Read the short stdout card, then `summary.json`; open original reasons only as needed. `complete` refers to this selected batch, not semantic coverage or Clear completion. The summary links originals and their hashes. It does not publish environment values or administrative run tokens. Probes and test commands are counted separately. Times, retries and evidence bytes are measured; tokens remain unavailable without telemetry.

Reuse requires a complete reviewed causal scope, equal plan/helper/source/test/contract/data/config/dependency/runtime/environment fingerprints, an intact complete prior result and accessible original reports. The helper checks these before copying original references into the new output. Reused outcomes retain their original time and run references; `commands: 0` means no new test execution, not zero prior cost. Changed or unknown inputs cause fresh execution. Hashes prove integrity, not authenticity or dependency completeness.

Use [Work storage](../procedures/work.md#finish-storage) for export and guarded cleanup; the helper never deletes previous outputs. Compare control and compatible batches on identical inputs and mandatory outcomes. Report preparation, probes, time, retries, data volume and limits separately; run counts prove neither quality nor token savings.
