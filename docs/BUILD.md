<a id="сборки-и-временные-файлы-проекта"></a>

# Project builds and temporary files

target/ is an ignored work directory, not permanent evidence storage. This document owns structure; tools restore it after cargo clean. Root permits only native Cargo outputs debug/, release/, tmp/, CACHEDIR.TAG, .rustc_info.json, plus nextest/ and highgrade/. Never put project probes, JSON or scripts directly at root.

```text
target/
  debug/                         Cargo development/tests
  release/                       Ordinary release build
  tmp/                           Cargo test temporary directory
  nextest/                       Standard test runner
  highgrade/
    build-cache/                 Exact-SHA candidate Cargo cache
    candidate/                   Last candidate: CLI, kit, candidate.json
    candidate.previous/          Emergency recovery only
    reports/                     Last candidate-build log
    runs/<name>-<UUID>/           Managed checks awaiting review/export
    ui-playwright/               Unmanaged local browser results
    tools/                       Local tools
    work/                        Lock/temp sources; empty after build
    tmp/<run>/                   Short-lived probes; clean after task
```

python scripts/build-release.py `<exact-commit-SHA>` prepares activation candidates. It is the only mandatory release build; do not prebuild cargo build --release --locked without a separate risk. The script exports that SHA, builds --release --locked and uses shared target/highgrade/build-cache independently of target/release. A parallel dev build cannot substitute the candidate. Binary comes from current Cargo compiler-artifact, including CARGO_BUILD_TARGET. Temporary sources are removed after ordinary success/failure; failure keeps the old candidate. The script activates/publishes nothing. On Windows, check that application context can read future candidate output. If sandbox/application users differ, build/apply in one authorized context instead of broadening ready-candidate ACL manually.

For executable CLI/kit changes or explicit activation requests, check active ID with global-status. **Before the commit to be activated**, use the requested version or increment patch in Cargo.toml, Cargo.lock and manifest release/cli_version. Update expected test IDs/check manifest. Commit, then build its SHA. Intermediate chain commits need no new release ID unless active new CLI is needed; the [local instruction](../.highgrade/project/INSTRUCTIONS.md) owns details. Never reuse release IDs: another binary under the same ID rejects update. Other project edits without activation authority need no candidate/new version.

candidate.previous exists only after both replacement and rollback fail. A new build refuses until manual diagnosis. Scripts do not delete edited/foreign candidate files or links/junctions. Concurrent builds reject through lock. After a crash, confirm no build before handling work/ remnants.

<a id="экономичная-обычная-сборка"></a>

## Economical ordinary builds

Cargo.toml defines one dev profile: debug=0, incremental=false; test inherits it. Nextest also builds target/debug/highgrade, so no separate cargo build is needed before verify.py tests. A different test debug profile would duplicate dependencies. For source debugging, temporarily set CARGO_PROFILE_DEV_DEBUG=line-tables-only for the relevant run; return ordinary checks to project settings. Debug assertions remain; detailed DWARF is off by default. Cache variants may coexist after setting changes. Remove old debug caches only by explicit --cache debug --apply after usage checks.

Before capture/Cargo, verify.py tests requires at least 1 GiB free for project/effective Cargo target, resolved through cargo metadata --locked --offline --no-deps. Low space preserves old reports and starts no build/automatic cleanup. This is conservative headroom, not a peak guarantee. Other processes/profiles/UI/project size may need more. Check before expensive work and after task completion. Worktrees have separate targets, not automatically shared caches. Never clean another tree before checking owner/activity/artifact need.

<a id="обслуживание"></a>

## Maintenance

<a id="ресурсы-интерфейса"></a>

### Interface assets

Before Cargo, the builder runs npm ci --no-audit --no-fund and npm run build in exported ui/. Build developers need Node 24/npm; installed users need only CLI/browser. Lockfile pins dependencies. bundle.mjs copies assets/licenses and records CLI/API versions, source SHA, asset hashes and font/icon origins. build.rs checks hashes/versions/required assets and embeds them. Incomplete release UI rejects. Builder calls ui --check true and checks SHA/version before replacing candidates. Installer checks UI before profile writes. CLI checks before listening; absent/corrupt/incompatible assets yield UiBundleInvalid, with no remote fallback. Local /licenses/ exposes licenses; /highgrade-ui.json lists assets. Storybook is not shipped.

Ordinary development: npm ci/npm run build in ui/, then cargo build --locked at root. Debug without dist is allowed for non-UI commands, but not UI/installation. An existing incomplete dist rejects even debug. Frontend edits need both builds.

```text
python scripts/target-maintenance.py
python scripts/target-maintenance.py --cache build-cache --report release-build.log
python scripts/target-maintenance.py --apply --scratch run-20260925
python scripts/target-maintenance.py --apply --cache release
python scripts/target-maintenance.py --check
```

Without --apply, review paths/cargo_commands; apply the same selection. --scratch NAME selects one highgrade/tmp/ child; --report NAME selects one existing reports file. Other content remains. --cache release calls cargo clean --release; debug calls cargo clean --profile test; build-cache calls cargo clean --release --target-dir target/highgrade/build-cache. Selected Cargo profiles are entirely reproducible build output. Unknown paths, candidate/tools remain.

Inventory reports mib (unique bytes, hardlinks once), logical_mib (sum by names), physical_mib (allocated file/directory blocks or null), hardlink_duplicates_mib and areas. Budget compares unique bytes. Areas count independently: a cross-area hardlink enters both area counts but total once. This is read-only measurement, not guaranteed freed space.

Validate all parameters/paths before deletion. Running Cargo/rustc/Nextest, nonempty work, candidate.previous or a target link/junction stops apply. Cargo refusal is an error; already removed temporary files are not restored. Cleanup is not one transaction. --check reports unknown paths, probes and **600 MiB after-maintenance** overruns. No forced cleanup to meet budget. Full cargo clean removes candidates/tools and is not routine maintenance.

<a id="срок-жизни-данных"></a>

### Data lifetime

| Data | Location/removal condition |
|---|---|
| Reproducible builds | debug/release/build-cache; explicit selection, rebuilt later |
| Probe/intermediate log | highgrade/tmp or reports; after task/evidence preservation |
| Task preparation | .highgrade/local/; input JSON/scripts/diff copies after use unless continuation/evidence needs them |
| Continuation | .highgrade/local/ until chain completion; then short result with SHA/links |
| Evidence | specs/changes/`<ID>`/evidence or docs/evidence while needed for checks/history; keep the only original report |
| Candidate/tools | Preserved by maintenance; backup CLI until update confirmation/recovery end |

Only the builder owns highgrade/work, for locks/temp sources, never draft specs. Before deletion, check owner, usage, links and history need. A link does not make the whole archive mandatory; preserve needed evidence separately, update links and verify transfer. Never archive reproducible caches/duplicates just to clean. Required private history stays in .highgrade/local/archive. Tools do not arbitrarily clean local. specs/ and issues/ are project data, outside build cleanup.

The [local instruction](../.highgrade/project/INSTRUCTIONS.md) owns preview/apply with one candidate_sha256. Use CLI/kit from one candidate. Source SHA does not guarantee identical binaries across builds; apply retains this particular candidate fingerprint. Before build-route tests, run cargo fetch --locked: isolated build.rs tests use offline dependencies, so prepare an empty cache explicitly. Then python -m unittest discover -s scripts/tests -v checks real Git/Cargo, compilation failure, foreign files/junctions. Requires Python 3.12, Node/npm, Cargo; no extra Python packages.

<a id="linux"></a>

## Linux

Build from SHA: python scripts/build-release.py `<SHA>`. Candidate highgrade has no .exe; builder preserves execute permission. [Installation](../kit/references/tools/installation.md#linux-bash) owns Linux install/update. Maintenance uses ps, not PowerShell.

<a id="обязательный-installed-e2e"></a>

## Required installed-E2E

Playwright uses its Chromium or HIGHGRADE_CHROME executable.

Windows/Linux CI builds exact PR head, or github.sha on push, through the standard builder. Platform-specific old releases use pinned SHAs below. Cache miss builds through their historical builders; hit reuses the same OS/architecture/SHA candidate. Both verify candidate.json, file set/hashes, version and CLI execution identically. Cache is not installation evidence. Install only in temporary profiles. Never relabel current binaries as older versions.

For local npx playwright test installed.spec.ts in ui/, set HIGHGRADE_UI_REQUIRED=true, HIGHGRADE_UI_CANDIDATE, HIGHGRADE_UI_SOURCE_SHA, HIGHGRADE_UI_VERSION, HIGHGRADE_OLD_SOURCE, HIGHGRADE_OLD_EXE, HIGHGRADE_OLD_SOURCE_SHA and HIGHGRADE_OLD_VERSION. Paths identify two candidates/kit/CLI; SHAs/versions come from their source commits. Verify candidate.json, hashes and both binary versions, then installation/update/recovery and two-project UI. CI always requires inputs; absent/partial/mismatched values fail. Only local nonrequired runs without configured inputs may skip. node --test ui/scripts/tests/installed-config.test.mjs checks this barrier without releases, using target/debug CLI for the executable probe.

Previous platform fixtures: Windows e60b6d4d4c3631c7140b037581f0bcdb2573befd (0.3.5); Linux fda4117312ca4c5af4388c960081dac94c10ea8f (0.3.4). These pre-0.3.6 source branches differ: Windows 0.3.5 installer loses Unix execute-bit. Never fix historical sources/binaries for a test.

[DEVELOPMENT](DEVELOPMENT.md#common-checks-and-ci) owns CI/waiting matrix. [Evidence storage](EVIDENCE-STORAGE.md) owns reports/cleanup.
