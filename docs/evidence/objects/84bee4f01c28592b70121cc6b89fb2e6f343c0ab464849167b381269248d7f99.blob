---
name: code-health-audit
description: Audit Python maintainability with pinned static analyzers and observed Coverage.py evidence, in full or diff mode, without editing the project. Use for code-health metrics, hotspots, accepted-debt checks, or a read-only Python maintainability audit. Do not use for security review, general multi-language scoring, or refactoring.
---

# Code Health Audit

Produce analyzer evidence before interpreting it. Never estimate metrics, overwrite analyzer values, or change deterministic classifications.

## Prepare and run

1. Identify the repository and choose `full` for its current state or `diff` against a local base (default: `origin/main`, then `main`). Diff includes committed, staged, unstaged and nonignored untracked Python changes from the merge-base. Do not fetch.
2. Read [toolchain.md](references/toolchain.md). Select an explicit existing analyzer interpreter. Run `audit.py --preflight` before spending time on tests. It checks the interpreter, versions, source scope and local base without running analyzers or tests. Resolve reported configuration problems; unavailable tools can still yield an incomplete diagnostic audit. Respect existing authorization before requesting any new dependency installation.
3. Inspect the project's existing test entrypoint and coverage configuration. If a bounded local test run is appropriate and authorized, use `capture_coverage.py` with that entrypoint and the project interpreter. Capture uses a fresh external directory, successful execution, data context, source/configuration hashes and final file snapshots. An old JSON export is insufficient. If coverage cannot be captured safely, continue without it and report incomplete evidence.
4. Run the audit using the same effective policy as capture. Always keep output outside the repository. Example PowerShell commands (substitute actual paths and the existing test module):

```powershell
& $auditPython -B "$skill\scripts\audit.py" --root $repo --mode full --preflight
& $auditPython -B "$skill\scripts\capture_coverage.py" --root $repo --python $projectPython --output-dir $captureDir --module unittest -- discover
& $auditPython -B "$skill\scripts\audit.py" --root $repo --mode full --coverage-json "$captureDir\coverage.json" --coverage-provenance "$captureDir\coverage-provenance.json" --output-dir $reportDir
```

For diff, use `--mode diff --base main` in preflight and audit. Add `--policy $policyFile` to all three commands when overriding scope or thresholds. Pass an existing coverage config with `--rcfile $configPath`; use `--script relative/test_runner.py` instead of `--module` for an existing script. Test arguments follow `--`. Do not invent project test selection from these examples.

## Read-only boundaries

Do not autofix, modify source/configuration/acceptance data, install into the project, or update project dependencies. Scratch files and output belong outside the project. Tests execute project code; the capture helper is not a filesystem/network sandbox. Inspect their expected side effects first. A timeout terminates the owned process tree on the tested Windows runtime.

If `READ_ONLY_BREACH` occurs, stop and report the paths. Do not revert changes automatically or label the run successful. See [report-contract.md](references/report-contract.md) for snapshot coverage and exit codes.

## Interpret and review

Read [report-contract.md](references/report-contract.md). Read [accepted-findings.md](references/accepted-findings.md) when an acceptance registry exists or the user asks to accept debt.

Inspect every active CRITICAL and REFACTOR hotspot, then up to `scope.watch_hotspot_limit` WATCH hotspots (default 20). Limit context to the affected symbol, direct dependencies, relevant tests, or the complete reported cycle. These surrounding files may contain OK measurements; do not expand into an unrelated repository review.

For each reviewed hotspot, explain the concrete risk and a possible improvement without editing. Leave canonical JSON unchanged; put contextual judgments in the response. Report reviewed/unreviewed counts by severity. If the context budget cannot cover the required hotspots, state the remaining count and continue in bounded batches when authorized; never imply complete contextual review.

## Deliver

Lead with mode/base, completeness and health as separate facts, active counts, accepted/reopened debt, missing evidence, and contextual conclusions. Link the JSON and Markdown reports. Explain that exit code 0 means complete evidence, not absence of critical findings. Never describe an INCOMPLETE or FAILED audit as healthy. Creating acceptance entries or fixing findings is separate work requiring the user's authorization for those changes.
