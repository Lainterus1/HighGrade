# High Grade global workflow v2 — implementation plan

Status: COMPLETE (source release; target-project pilots deferred). Authority: user requested implementation and GitHub publication on 2026-09-23. Target projects (SNAF, PlantsNotify) are deferred. Source: docs/workflow/SPECIFICATION.md, docs/workflow/CONTEXT.md, and the agreed global-toolkit decisions in the current session.

## Outcome

A globally installed High Grade suite with six shared skills (init, task, spec, work, clear, update), one Rust CLI, and per-project `.highgrade/project/INSTRUCTIONS.md`. AGENTS.md routes work into the suite; project-specific skills remain optional and conflicts are surfaced. No project-local copies of the shared skills or CLI in new installations. Publish the verified source to github.com/Lainterus1/HighGrade.

## Stages

- G1: Reconcile product and technical documents with the global model, record superseded project-local release behavior, and establish one current Next.
- G2: Implement user-level package installation/update and project adapter checks, preserving rollback and read-only diagnosis; retire project-local installation for new projects.
- G3: Add shared task/spec skills with deterministic discussion, confirmation, OpenSpec handoff; rename refresh to clear; update init/work routing and conflict audit.
- G4: Apply the new workflow to High Grade itself, set context-budget proposal from measured evidence, and synchronize project documents and templates.
- G5: Verify Rust tests, repository checks, isolated end-to-end scenarios, Codex skill discovery, and independent review; fix findings.
- G6: Prepare license/provenance and GitHub source installation instructions, establish Git checkout, commit and push. Verify remote result. Target-project pilots remain later.

## Acceptance

- A1: Shared skills and CLI install/update once at user scope; project files contain only adapter/docs/evidence. Failed update preserves prior working installation.
- A2: Init identifies overlapping local skills and produces a project adaptation; task/spec route follows confirmed goals into OpenSpec, while direct small work remains possible.
- A3: Clear is on-demand review; routine work maintains docs/archive; compatibility check is lightweight and routes meaningful conflicts to clear.
- A4: Native Rust/Playwright trace behavior and diagnostic safety remain covered by tests; global discovery works in Codex app and CLI in an isolated environment.
- A5: High Grade documents and project adapter agree with implementation; publication is verified on the intended GitHub repository.
