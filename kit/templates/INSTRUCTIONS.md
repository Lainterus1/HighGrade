---
highgrade_project_schema: 1
highgrade_spec_format: native-v1
---
<a id="проектная-инструкция-high-grade"></a>

# High Grade project instruction

Status: initialization proposal. Mark acceptance only after the user's decision.

Fill from survey and user decisions. For material settings, use defined with source, inapplicable with reason, or unknown with dependent action. Never present template fields as accepted values. Instruction language does not set the language of generated project/user documents.

<a id="проект-и-владельцы"></a>

## Project and owners

- Init result/continuation: [prepared foundation, initial/reused specification IDs, first recommended specification/refinement action; questions/tasks remain in specs. Init completion does not prove implementation].
- Connection basis: [survey.json, completed revision and material limits; current knowledge belongs to owners below].
- Authored-document/specification language: [language and decision source, also for saved plans, state, reports and human handoff]. English agent instructions do not override it. Preserve technical IDs/original machine messages.
- Purpose/scope: [README and agreed product-source links].
- Material risks/limits: [owner link or specific unknown].
- Current documents: [AGENTS, ARCHITECTURE, ENGINEERING, DEVELOPMENT and justified additional sources].
- Accepted exceptions/resolved conflicts: [decision, reason, affected rule and review condition].

<a id="маршрут-задачи-и-спеки"></a>

## Task and specification route

- New changes: project specs/ or init-selected directory, active-CLI-supported format and spec-* commands. spec/results are separate and survive High Grade removal. CLI assigns numbers. Migrate earlier stores explicitly with spec-migrate --to directory and backup.
- Navigation: [broad domain tags]; depends_on needs an integrated result, related_to explains a relationship; no content inheritance.
- Rechecks: existing specification/result history; equivalent repeats need no new human decision. For material drift, use active Delivery work.
- Previous specifications: [paths, current owners and agreed exceptions; no mass transfer].
- Additional specification method: [link if present, no copied content].
- Standard checks/evidence: [DEVELOPMENT commands, test policy and existing mandatory-report/unrecoverable-input storage/delivery owner].
- Local readiness: [checked environment, available checks and gaps; technical connection is separate from acceptance].
- Local skills: [name, distinct responsibility and common-skill relationship].

<a id="сопровождение"></a>

## Maintenance

- Context registry: [documents.json](documents.json); fill agreed ranges or unknown after audit.
- Git/result identity: [Git or inapplicable; noGit immutable package/build ID, verification/storage and Work completion without commit. For Git, checked SHA/content linkage after squash/rebase/merge, drift checks; preserve human decision history].
- Pre-submit: [checks/review before local commit/candidate push, explicitly allowed remainder]. Final readiness: [mandatory criteria/review]. CI: [available exact-SHA status; default no blocking wait; a mandatory wait only from an explicit local rule/user decision].
- highgrade-commit: [selected scope/candidate-commit policy; inapplicable without Git]. Local activation: [inapplicable or assigned stage, exact SHA/build ID commands, preview/check/recovery; release ID/version only when required]. Commit is not acceptance; its call authorizes only this project local activation.
- highgrade-push: [remote, branch/range; direct push or PR base/head and rules; remote SHA/CI checks; inapplicable without Git]. Push effects: [automatic deployment/external actions, environment and authority, or confirmed absence]. Push/PR require explicit scope authority.
- Decisions/authority: [who accepts which exact result, who authorizes merge/tag/release/deploy, conditions/separate barriers; never infer authority from commit/CI/activation]. Activate alone is not acceptance.

Select routes through active Delivery task. Changed requirements use spec; technical work/fixes with unchanged contracts use work, with a plan for dependent stages. Risk defines checks/review, not a spec ID.
