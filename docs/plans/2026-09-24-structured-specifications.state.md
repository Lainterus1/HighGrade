# Session state: structured specifications

Plan: [structured specifications](2026-09-24-structured-specifications.md), version 1
Workspace: D:/my_projects/MyCodex
Observed: HEAD 889c8699b1ddc885b36c7d29beacbd5e74b13e2e plus authorized local implementation; candidate v0-2-12. Main global release remains v0-2-11.
Status: DONE
Current: completed
Next: none
Blockers: none

## Progress and evidence

- P1 VERIFIED: Rust types and generated schemas; readable drafts, unknown version/fields reject writes.
- P2 VERIFIED: CAS, concurrent CLI writers/readers, failure before replacement, add/modify/remove, base drift, retained history.
- P3 VERIFIED: scenario/common-rule/input/report drift, skipped results, negative review and unaffected sibling evidence.
- P4 VERIFIED: installed procedures use native CLI; legacy selected import preserves source and IDs with one effective owner; native doctor works without external tool executables.
- P5 VERIFIED: independent agent resumed PILOT-RESERVE using installed delivery, reproduced two failures and obtained four real test passes; independent review GO and integrate PASS. Final nextest 90/90, automatic trace 25/25, release/installation/OpenSpec/structure gates pass.
- Durable evidence and limits: [review](../reviews/structured-specifications.md). Raw native results have one location under target/nextest/highgrade; isolated exercise under target/structured-specifications.
- Intermediate review defects and Windows read/lock race fixed and regression-tested. Package test PID reuse fixed with exclusive unique temporary directories; final suite complete.
- Known limits: agent-attested meaning/authenticity/input coverage; 8 MiB store; stable router legacy description; full-context budget warning retained after reducing entry duplication. No SNAF, commit/push or main activation.
