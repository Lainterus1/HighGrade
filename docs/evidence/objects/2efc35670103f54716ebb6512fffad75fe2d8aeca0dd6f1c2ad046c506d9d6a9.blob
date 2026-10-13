<a id="разработка-и-проверки"></a>

# Development and verification

Use Python 3.12+, Node.js 24 and Rust 1.89. Run from root. Nextest or cargo build --locked builds candidate CLI.

<a id="изменения-требований-нативные-спецификации"></a>

## Requirement changes: native specifications

Select [task route](../kit/procedures/task.md#choose-a-sufficient-route). [Planner](../kit/procedures/planner.md) coordinates dependencies; Goal needs authority. Active CLI:

```powershell
$hgProfile = $env:USERPROFILE
$hgActive = Get-Content "$hgProfile/.highgrade/global/active.json" -Raw | ConvertFrom-Json
$hgExe = "$hgProfile/.highgrade/global/releases/$($hgActive.release)/highgrade.exe"
& $hgExe spec-list --root "$PWD"
& $hgExe spec-schema --root "$PWD"
```

```bash
hgRelease=$(python -c 'import json,pathlib; print(json.loads((pathlib.Path.home()/".highgrade/global/active.json").read_text())["release"])')
hgExe="$HOME/.highgrade/global/releases/$hgRelease/highgrade"
"$hgExe" spec-list --root "$PWD"
"$hgExe" spec-schema --root "$PWD"
```

Status: spec-check --id HG-CHANGE --brief true. Context: spec-read --view summary|editable|full.

Edit through compatible CLI: spec-new with hash → spec-read --view editable → spec-edit --input patch.json --expected HASH --validate true. global-status identifies active version; target/debug is a candidate. [Reference](../kit/references/tools/specifications.md#structured-specifications) owns spec-save/options.

Continue spec-validate → work/tests → spec-evidence → independent review/spec-review → spec-integrate --brief true with readiness check. spec-check explains gaps; spec-decide records human decisions. Focus: cargo test --locked --test catalog_contracts --test spec_contracts --test trace_contracts.

<a id="сценарии-и-исходные-результаты"></a>

## Scenarios and original results

// highgrade: HG-... links Rust tests/scenarios; trace requires actual results. spec-run checks are optional, but declared links must match. [QUALITY](workflow/QUALITY.md) owns evidence requirements.

Install Nextest 0.9.146 into target/highgrade/tools/bin and add it to PATH.

verify.py tests compares inputs before/after Nextest. prepare checks hashes, input contents and completed-attempt reports. After failure/interruption, repeat tests. mtime proves no freshness; a snapshot proves no authenticity. After integration, repeat prepare/trace/verify. Equivalent catalogs allow reuse; otherwise repeat tests. native_report_reason explains why.

Tests/UI reports: [run/export/cleanup](EVIDENCE-STORAGE.md).

<a id="выбор-проверок"></a>

## Select checks

| Change | Sufficient check | Expand when |
|---|---|---|
| Documents, UI/API | Semantic review of [UI](UI.md), [ui/README](../ui/README.md), diff; check-repository, render-skills --check, inspect | New behavior needs its tests |
| Rust CLI, format, installation | Narrow contract tests; one full final-chain Nextest/trace | Specific uncovered risk or reuse refusal |
| Catalog results/metadata only | spec-validate/check; prepare/trace/verify if trace changes | Nextest only for changed causal inputs |
| Build/activation | build-release.py from SHA; one candidate preview/apply/global-status | Failure needs targeted diagnosis/recovery |

External pilots need a request.

Contract: spec-list/read. Causal inputs: [work](../kit/procedures/work.md#evidence-input-scope), [examples](evidence/current-requirements/2026-10-01/verification.md). A shared snapshot does not replace them.

spec-stats reads timings without execution. Preserve the message link for human observations.

<a id="rust-cli-и-глобальная-поставка"></a>

## Rust CLI and global Delivery

JUnit: spec-run-inputs before execution, spec-run-import afterwards. A retrospective snapshot does not prove freshness. CLI reference owns format.

Active CLI diagnostics: doctor and inspect --root `<project>`. global_contracts checks installation; add isolated profiles for uncovered risks. unknown is not PASS.

Use only target/highgrade/tmp/`<run>` for temporary probes; remove after work. [BUILD](BUILD.md) owns structure/maintenance. Size: python scripts/target-maintenance.py --check.

<a id="интерфейс-и-storybook"></a>

## UI and Storybook

ui/: npm ci, npm run build, npm run build-storybook, npm test. Root: cargo build --locked. npm run storybook serves 127.0.0.1:6006. [Installed E2E/browser](BUILD.md#required-installed-e2e).

<a id="отдельные-плагины"></a>

## Separate plugins

SVG Vectorizer: [README](../plugins/svg-vectorizer/README.md). Code Health Audit: [SKILL](../plugins/code-health-audit/skills/code-health-audit/SKILL.md). Their own environments check behavior.

<a id="принятие-и-публикация"></a>

## Acceptance and publication

The [local instruction](../.highgrade/project/INSTRUCTIONS.md) owns authority/order. [BUILD](BUILD.md) owns builds. Push needs separate authority.

<a id="общие-проверки-и-ci"></a>

## Common checks and CI

Standard commands selected by the matrix:

```bash
cargo fmt --all -- --check
python -m unittest discover -s scripts/tests -v
node --test ui/scripts/tests/*.test.mjs
python scripts/render-skills.py --check
python scripts/verify.py tests
python scripts/verify.py specs
python scripts/verify.py scenarios
node scripts/check-scenario-scale.mjs
node scripts/check-scenario-reuse.mjs
node scripts/check-repository.mjs
python scripts/verify.py inspect
```

CI is asynchronous; waiting does not block Work, integration or authorized commit/activation/push. Report exact-SHA status separately; diagnose known failure under local rules.

| Check | Linux | Windows |
|---|---|---|
| render-skills, fmt, structure, inspect, specs | Yes | — |
| Python lifecycle, Nextest inventory/run/trace, Node guards | Yes | Yes |
| Scale/reuse, Storybook, native/decision/edit Playwright | Yes | — |
| Embedded UI, exact release build, required installed upgrade/recovery | Yes | Yes |

ubuntu-24.04 is the [GitHub runner image](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), not a user requirement. Narrow native UI failures were separately tested on Linux only; repeat affected selectors on Windows for a Windows UI defect.

verify.py specs reads catalog once. Cargo cache keys use SHA with platform/toolchain/dependency fallbacks. [BUILD](BUILD.md#required-installed-e2e) owns historical-cache checks; [GitHub](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching) defines cache scope. A new run cancels the previous run on the same branch. Cache/local PASS proves neither another OS result nor acceptance. Time savings need new-run measurement.
