# Analyzer environment and scope

Canonical versions: `scripts/data/toolchain.json`. A reproducible installation list is `scripts/data/requirements-analyzers.txt`. No helper installs packages automatically. Prefer an existing external environment; if installing is necessary, honor existing authorization or obtain it before using pip. Never alter the audited project's dependencies.

This release is tested on Windows AMD64, CPython 3.12.10. Other Python/platform combinations or changed pinned dependencies produce `RUNTIME_UNVERIFIED` and an incomplete audit until validated. The report records the executable, Python/platform, analyzer and dependency versions, and runner source hash. Python 3.14 and POSIX process cleanup are not certified by this release. The project test interpreter may differ; its runtime and Coverage.py version are recorded separately. An analyzer interpreter must understand the project's Python syntax.

| Metric | Evidence |
|---|---|
| Cyclomatic complexity, function/file NLOC | Lizard 1.24.0 Python API; no stateful NS extension |
| Nesting | Python AST algorithm 1.1.0; top-level control statement depth 1, actual `elif` stays flat, `else: if` adds a level; nested functions measured independently |
| Cognitive complexity | Complexipy 7.0.1 with `no_ignore=True` |
| Duplication | Pylint 4.0.7 JSON2 ranges intersected with that version's significant-line sets; identical numerator/denominator rules, unique duplicated lines |
| Dependency cycles | Pylint 4.0.7 static import analysis with explicit file targets and source roots; not a proof about dynamic imports or runtime-only dependency cycles |
| Coverage | Coverage.py 7.15.4 branch run and observed provenance; no JSON timestamp freshness heuristic |

The analyzer boundary isolates each tool in a bounded process. Defaults: 120 seconds per analyzer and 60 seconds for a nested Pylint process; policy overrides are capped at 3600 seconds. Missing, failed, suppressed or timed-out global evidence does not become zero/OK. Relevant suppression comments are reported; unrelated disabled lint messages and text inside strings are not required-tool suppressions.

## Preflight and project policy

`audit.py --root <root> --mode full|diff --preflight` prints JSON and exits 0 when the runtime, scope and required local base are ready, otherwise 2. It does not certify coverage or execute analyzers/tests. Run normal audit for diagnostic artifacts if evidence remains unavailable.

`--policy` is a JSON override merged with `data/default-policy.json` and validated structurally and semantically. Unknown fields, unsafe paths, nonfinite values and reversed thresholds fail closed. The implementation checks the bundled JSON Schema subset; it is not a general JSON Schema library. The subset is cross-checked against Draft 2020-12 validation.

Useful fields:

- `include_paths`: repository-relative directories/files/globs, default `["."]`.
- `excluded_directories`: directory names pruned from source discovery; an empty array disables these policy exclusions. VCS metadata is always excluded in non-Git traversal.
- `excluded_paths`: directories/files/globs, for example `["src/generated", "migrations"]`.
- `source_roots`: import roots, default `[".", "src"]`; existing roots are ordered from most specific to least specific. Declare custom layouts explicitly.
- `test_paths`: files/directories/globs classified as tests; these remain in static analysis but are excluded from production coverage totals.
- `watch_hotspot_limit`: maximum WATCH hotspots for contextual review; does not remove measurements.
- `analyzer_timeout_seconds`, `subprocess_timeout_seconds`: finite positive process limits.
- `metrics`: overrides of known thresholds; direction/unit remain fixed for each metric.

Paths use `/`, are case-sensitive for pattern matching, and must stay within the repository. A literal directory includes descendants; glob `*` can match `/`. Git source discovery uses tracked and nonignored untracked files. Pruned directories and excluded candidates are visible in the scope report. `.pyi` files are excluded by default; `include_pyi_loc=true` includes their LOC evidence.

## Observed coverage

`capture_coverage.py` requires an existing `--module` or `--script` entrypoint. `--python` selects the project interpreter; `--timeout` defaults to 300 seconds (maximum 3600). Output must be a new empty directory outside root. Pass the same `--policy` to capture and audit.

The helper redirects coverage data and temporary/bytecode paths outside the project, adds a unique data context, requires successful tests, exports JSON, and checks source/configuration/file snapshots. Keep `.coverage`, `coverage.json` and `coverage-provenance.json` together. Audit rejects missing or changed evidence, a changed effective policy, or a changed source/configuration state. Merely touching an unchanged source does not invalidate hashes. The provenance is an observed-workflow attestation, not protection against deliberate fabrication of all evidence.

Parallel data routing, commands that replace the coverage session, and opaque multi-step coverage workflows are unsupported by this helper. Use an appropriate existing bounded entrypoint or report unavailable evidence. Do not disable a project's required behavior merely to obtain a complete result.
