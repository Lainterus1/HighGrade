# Structured specifications implementation

Version: 1. Authorized in conversation on 2026-09-24.

## Result and authority

Implement the agreed High Grade JSON specification workflow: native Rust tooling, reproducible evidence, drift detection, safe persistence and portable skills without a mandatory OpenSpec dependency for new work. Preserve legacy requirements and history. No SNAF, global activation, commit/push, deployment, UI, server, database, event sourcing or executable GWT.

The transition itself uses one OpenSpec change, `structured-specifications`, until the new tool is verified. It owns acceptance and detailed tasks. Plan/state only coordinate execution.

## Minimal design

One versioned `.highgrade/specs/store.json` holds current requirements and changes (including archived changes and evidence references). A project-local lock and expected store hash protect writes; replacement of the whole bounded document makes integration and archive one atomic operation. Native report contents are not copied into the store. Selective CLI reads serve agents and future UI. Coarse write conflicts require rereading; no automatic merge engine.

Formal JSON Schema is generated from Rust types, avoiding separately maintained structural contracts; semantic checks remain explicit. Drafts are structurally valid but not ready. Tool reports never claim semantic correctness, user approval, or authenticity of a human-provided observation. Existing native trace results may support evidence; unsupported formats retain explicit agent attestation and provenance.

## Stages

| Stage | Result | Gate |
|---|---|---|
| P1 | Contract, schema and draft model | Strict transition spec; invalid/unknown input tests |
| P2 | Create/read/save/diff/validate/integrate | CAS, concurrent writes, failure before replace, explicit removals |
| P3 | Evidence and drift | Current/stale/skipped results; requirement/base/report/input changes |
| P4 | Skills, diagnostics and legacy compatibility | Installed workflow usable without OpenSpec; old contracts still readable |
| P5 | Isolated end-to-end exercise and independent review | Fresh reader, actual business test, resume and negative cases; final gates |

State: [checkpoint](2026-09-24-structured-specifications.state.md).
