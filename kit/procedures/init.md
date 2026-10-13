<a id="инициализация-проекта"></a>

# Initialize a project

Init ends with a prepared repository and initial specifications for the agreed scope, plus a clear next action. It does not implement product features or a prototype. Survey/plan-only requests retain that boundary. A separate implementation request follows task/spec/work after init; do not repeat known permissions.

<a id="решения-до-глубокого-обследования"></a>

## Decisions before deep survey

Read current instructions and entry documents. The first meaningful choice is:

> Do we continue an existing project or create a new one?

Ask only when intent is unknown. Empty directories, templates and file counts cannot replace the answer; an empty folder may receive an existing project. Clarify material contradictions. Separately establish High Grade connection state. Repeated init reuses decisions/survey. Rewriting the product is a separate task.

Use request/local language, otherwise the request language with a stated assumption. English templates/instructions do not require English product documents. An unavailable root stops dependent work. Save fixed decisions with their source; ask short rounds through [task](task.md):

| Decision | Content |
|---|---|
| purpose | New product or continued/adapted process |
| scope | Whole project or named area |
| preservation | Files, behavior, interfaces, data and compatibility to preserve |
| changes | Allowed foundation, document and automation changes |
| local_rules | Required local rules/skills and conflict resolution |
| environment | Environment, access, services and cost limits |
| mandate | Survey/plan only or repository and initial-spec preparation |

<a id="новый-проект-первичные-требования"></a>

## New project: initial requirements

Through task, establish the problem/users, first useful outcome and verification, included/excluded first-version features, data/integrations and runtime. Ask about interface, access and operations when applicable. Reuse known answers. Propose ordinary technical choices with short reasons; clarify material choices before dependent action. Do not require every library choice in advance.

Goal, scope and concrete nearest tasks are enough for meaningful initial specifications. Unknown future implementation details remain explicit questions. Clarify uncertain goals/preparation limits. README may own the original product description. Add PRODUCT.md only for size/responsibility. Specifications own requirements/tasks; survey owns decisions/links.

<a id="обследование"></a>

## Survey

Collect structure through inventory --mode structure and read material sources under [audit](../references/audit.md). An empty directory needs one initial inspection, not repeated missing-file diagnostics without changed inputs. For existing projects, preservation/adaptation determine depth; no default full product audit.

Produce project [survey JSON](../references/tools/survey.md). With write authority, call survey-new. Read-only requests keep results in the response. Survey needs neither specs nor documents.json. Fill map, knowledge owners, processes, skills, observations, findings, adaptation and routes.

Distinguish observed absence, accepted future design, proposal and unknown. Confirmed verifies one specific claim with basis: no code yet or a decision to create a component does not mean implementation. Do not mark not-yet-created as not_applicable; inapplicability needs a reason. Put proposals in adaptation, actual structure in components. An empty section means unexamined scope; never invent facts for readiness.

In processes/findings, describe formatter/linter/ignore/CI coverage for created files, serialization owner, available environment and standard tool invocation. Compare check purpose/cost, duplicates and reuse conditions. Use the current environment before proposing another. Exact exclusions may apply to CLI data; do not exclude all JSON/.highgrade or weaken gates without a request. Identify mandatory evidence owner, location, retention and delivery in an existing suitable document.

Select existing source_paths/areas and save a snapshot. Future files are not sources. An empty snapshot does not prove code reading. Link unknowns to dependent actions: future product-spec questions do not block defined preparation. Keep material preparation gaps. After semantic review, record review. check verifies prerequisites, not authority/truth. Plan, adaptation and completion differ; record a new finding at its actual stage without rewinding or deleting the blocker.

<a id="подготовка-репозитория"></a>

## Prepare the repository

For an existing project, reuse owners of audit's five roles and check content. Explain moves/splits by usefulness, responsibility or size. Preserve agreed guarantees and fix links. For a new project, assign minimum owners; one document may cover several roles. Do not create empty placeholders.

Example guidance, not a mandatory tree: README.md/AGENTS.md in root; .highgrade/project/ for process configuration; docs/ for knowledge; specs/ or a chosen requirement directory; implementation/tests under stack conventions. Create scripts/assets/deploy only for real responsibility. Tests may sit beside code. Keep tool-required root configuration. Group code by feature when it helps local changes. Do not precreate shared layers or apps/packages. Separate caches, temporary material and mandatory evidence.

Prepare local instruction, navigation, known commands and budgets under [audit](../references/audit.md#context-and-adaptation). State unknown commands/budgets honestly. Init does not need a working app. If no runner is needed yet, leave setup unverified until the first real task; never add fake checks or demo functionality for a gate.

Fill the [instruction template](../templates/INSTRUCTIONS.md) delivery section from current documents, CI configuration and user decisions: Git/immutable noGit applicability, target branches and PR/base, pre-submit/readiness criteria, separate CI status and only an explicitly assigned waiting barrier, push effects including automatic deployment, activation and separate acceptance/merge/release/deploy authority. For squash/rebase, define linkage of checked content to the new SHA. Reuse known sourced facts; do not repeat questions or treat a found command as permission. For each applicable unknown, name the dependent action. Missing future remote does not block independent local work.

On repeated init/Delivery update, do not silently overwrite accepted local rules. Show the specific adaptation gap and propose a minimal agreed addition. Compatible schema does not prove new fields are filled. Clarify only a material unknown before its dependent action. Do not impose High Grade release IDs, global-update or Git where unnecessary.

Preserve useful local skills/exceptions. For overlapping routes, define trigger, input, output and owner. Do not copy the Delivery into a project. The technical scaffold is preparation only; an application generator that creates product functionality belongs to later requirement-based implementation.

Distinguish a file directory, local Git and remote repository. A request to set up a new Git repository authorizes git init. Preserve an existing .git. Remote, visibility, license, commit, push and deployment follow the request/known decisions. Git absence alone is not a local-project defect.

<a id="запуск-интерфейса-проекта"></a>

## Project interface launch

With preparation authority, run ui-launcher --root `<project>` to preview, then ui-launcher --root `<project>` --apply true. It creates HighGrade UI.sh/HighGrade UI.cmd at root. They locate the current global installation without pinning a release and work after project moves. Repeated Init restores missing files and its shell executable bit. Do not overwrite foreign/edited files or links; report conflicts. Check through doctor. Survey-only requests do not authorize launchers. Details belong to [installation](../references/tools/installation.md#launch-ui-from-a-project).

<a id="начальные-спецификации"></a>

## Initial specifications

For authorized new-project preparation, choose specs/ or a suitable empty directory, call spec-init and record the path. Do not overwrite foreign content. Existing projects retain current specs/format. Migration/mass formalization needs a request. Without new requirements, do not invent a specification for an init report.

Under [spec](spec.md#initial-preparation-during-init), save known requirements, goal, scope and concrete nearest tasks. Task order reflects logical implementation. Name the first recommended specification in handoff or existing Next. Use depends_on only for actual result dependencies, not sorting. Do not create a duplicate-task roadmap.

Develop the nearest specification more deeply from available facts. Later ones may retain explicit questions/incomplete details. An empty template is not initial requirements. Distant unclear ideas need no specifications. Tags represent real broad domains.

Do not present drafts as executable-ready. State missing information and refinement action. Init completion needs neither product implementation/integration, green spec-check nor acceptance of a future product. Later task/spec refines the same ID, then work executes a sufficiently defined contract.

<a id="проверка-и-продолжение"></a>

## Verification and continuation

Before a registry exists, inspect --bootstrap separates technical validation from adaptation work. Legacy status/exit may fail for expected absences; read validation/adaptation and actual findings, without claiming overall PASS. Before preparation, a missing path is a task. After promised creation, it is an unfinished result. Do not hide unavailability/corruption. After preparation, use registry-based inspect and applicable doctor. Do not require tools unnecessary for the nearest action.

Check changes under the local matrix, links and request → specification → tasks → intended scope → verification chain. For absent code, state that scope is intended. Check normative edits through work. Command execution claims require a real run. Do not launch an improvised app to check instructions.

Before finishing, update actual survey state, snapshot and review. Pass prepared content, new/reused specifications, recommended order, open questions, first specification/refinement action, actual checks and limits. Connection readiness differs from product-spec completeness and implementation. Owners retain current knowledge; survey history is not a backlog.

Repeated init checks affected changes and previous decisions. Edit completed through reopen with history retained. A new Git SHA alone does not reset agreement. On interruption, preserve actual state and concrete Next. After full init, implementation begins under a separate request through the standard workflow.
