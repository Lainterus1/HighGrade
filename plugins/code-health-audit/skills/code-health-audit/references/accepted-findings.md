# Accepted findings

The optional `.code-health-audit/accepted-findings.json` uses `scripts/data/accepted-findings.schema.json`. Audit only reads it.

Create an entry from a canonical finding after the user accepts that concrete debt and reason. Copy `finding_id`, `metric`, `path`, `symbol`, `analyzer`, `evidence_fingerprint`, `policy_sha256`, `runtime_sha256`; add report `audit.policy_version`, numeric `accepted_value`, numeric `ceiling`, and a specific `reason` (at least 10 characters). Optional `guard_files` contain relative paths and SHA-256 hashes; optional `review_after` is an ISO date.

The historical field `ceiling` is the permitted bound: a maximum for high-is-bad metrics, a minimum for coverage. A better value sets `baseline_can_tighten`; the audit never edits the bound. Old entries without policy/runtime hashes parse but reopen and must be reviewed again. Changing thresholds without changing policy_version also reopens debt.

Full mode requires the same registry content in HEAD, index and worktree (ordinary LF/CRLF conversion allowed); custom Git clean filters are not executed. Diff mode trusts only the merge-base registry. Registry edits in the current diff cannot accept their own findings and create a policy finding. An untrusted full registry is ignored.

Accepted findings retain severity but become `active=false`, `disposition=ACCEPTED`. Reopen on identity, policy hash/version, runtime/runner/analyzer changes, evidence changes, regression beyond the directional bound, missing/changed guard, or expired review date. Invalid/duplicate registry entries fail closed. Inline required-analyzer suppressions do not create accepted debt.

Normalized AST fingerprints ignore formatting and comments. Coverage/duplication fingerprints also contain their relevant measured evidence, so their reopening rules can differ from AST-only metrics. Repeated same-name functions/methods use a deterministic ordinal suffix to prevent colliding IDs.
