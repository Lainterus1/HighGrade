# Report contract

`code-health-audit.json` is canonical; Markdown renders the same measurements. Contextual review belongs in the assistant response and never overwrites these artifacts.

Independent axes:

- `audit.completeness`: COMPLETE, INCOMPLETE, FAILED.
- `audit.health`: worst active OK, WATCH, REFACTOR, CRITICAL classification.
- `audit.verdict`: combines them; incomplete evidence can never produce HEALTHY.

Exit codes: **0** complete evidence, including potentially critical findings; **2** incomplete/failed input, tool or coverage evidence; **3** detected repository change. Preflight uses 0 for readiness and 2 for unavailable/invalid prerequisites. CI consumers must inspect completeness and active classifications independently of exit code. No deployment decision is implied.

Collections: `measurements` includes OK values; `findings` contains non-OK/policy findings; `hotspots` groups active subjects; `accepted_findings` records acceptance/reopening; `errors` preserves diagnostic reasons. `scope` records included/excluded files, effective roots/patterns, diff state and the contextual WATCH limit. `runtime` records the actual interpreter and dependency identity. Output undergoes bundled schema and semantic validation, including unique measurement IDs and percentage bounds. Invalid output yields a diagnostic failure report.

## Diff semantics

Diff requires the Git worktree root and a resolvable local base. Default resolution prefers origin/main, then main; no fetch. It compares the merge-base with current files, including staged/unstaged and nonignored untracked Python additions. Deleted files have no current measurements.

Function metrics cover whole changed/intersecting functions. Duplication percentage uses eligible changed lines and unique duplicated changed lines. Cycles are new static cycle signatures relative to a successfully analyzed base. Missing current/base Pylint evidence cannot certify zero new cycles. Coverage in diff mode measures changed executable lines; **changed branch coverage is not measured**, and scope states that explicitly. Full mode measures branch coverage where branches exist. A verified empty executable-line denominator yields 100% by convention, not evidence of additional tests. Missing production files make coverage partial; repository coverage totals are omitted when they would cover only an unreported subset.

## Read-only evidence

Snapshots bracket both coverage execution and analyzer execution, including failure paths. Git snapshots hash all tracked and nonignored untracked files plus index entries/status; previously dirty content and newly created files are checked. In non-Git folders, traversal excludes VCS metadata and configured excluded directories, while explicitly included Python files are also hashed. Excluded/ignored caches, external paths, Git internal metadata and transient edits restored before the final snapshot are outside this guarantee. Snapshot comparison is detection, not an OS write sandbox.

`read_only_verified=true` means the available pre/post snapshots matched; if an initial snapshot could not be established, it is false. A mismatch fails with READ_ONLY_BREACH and exact paths. Preserve those files for the user; do not automatically revert.

`audit.content_digest` excludes generation time, absolute repository/interpreter/output paths. It includes source state, diff base, effective policy, runtime/runner/tool versions, coverage evidence and trusted acceptance results. Repeating the audit on identical inputs yields the same digest. Re-running coverage creates new evidence and may change the digest even when measured percentages stay equal.
