<a id="отправка-выбранных-коммитов"></a>

# Push selected commits

Run on an explicit highgrade-push request. Read project remote, target branch, base/PR requirement, pre-submit and final barriers. Identify a continuous range of local commits; it may cover several tasks. Do not repeat permission when range, target and effects are clear. Otherwise show the concrete package, target and material consequences before sending. Without Git, this route is inapplicable; immutable-package delivery needs the separate project method and request.

Compare each commit with its task source, local recording, origin, license and absence of private data. A commit message does not prove ownership. Check files and mandatory spec/results/evidence in the actual sent SHAs under [delivery rules](../references/tools/diagnostics.md#evidence-delivery-contents). Compare git status with scope: required files cannot remain only in the working tree. Preserve unrelated changes and report them. An unknown commit stops the range.

<a id="отправка-кандидата-и-возврат-в-work"></a>

## Candidate push and return to Work

After pre-submit checks, an explicitly requested candidate push for remote CI/PR may precede final technical readiness. Reuse native spec-check --brief true. An explicitly assigned unfinished criterion remains a gap under [work](work.md). Do not claim integration or acceptance for sending. Reuse successful local checks unless inputs/risk change. Return failed mandatory pre-submit checks to Work first.

Check current remote branch state and project rules. Push does not automatically authorize PR, merge, tag, release or deployment. Perform only explicitly included actions. If push triggers automatic deployment or another material external effect, disclose it and check authority for the effect/environment before sending. Without authority, stop dependent push. Do not bypass the rule through an invented branch. After authorized automatic deployment, check actual target state and linkage to the sent SHA through the project mechanism. State pending/unknown/failed; do not redeploy just to check. For a PR, check base/head and project route. Branch push proves neither PR existence nor acceptance.

Use ordinary push without force by default. Verify target remote SHA. Read available CI for that exact commit once and report it under [work](work.md). Pending/unavailable CI does not require blocking wait. Diagnose a known failure and return the actual defect to Work with a concrete remainder. Fixes need affected checks and authorized Commit/Push within scope. Publication/green CI alone closes neither other readiness criteria nor acceptance.

After squash/rebase/merge, apply [result identity](commit.md#user-decision) and local final checks for the new SHA. Push does not change verified_revision or transfer a human decision to changed content. Merge/release remain separately authorized.

Report range/tasks, target/remote SHA, PR/CI and material remainder. Show local/remote activation and acceptance separately. Push creates no new commit, including for Planner completion. On failure, commits remain local. Publication proves neither local activation nor human acceptance.
