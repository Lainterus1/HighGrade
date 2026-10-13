<a id="подготовка-спецификации"></a>

# Prepare a specification

When a decision on prepared content is needed, create an explicit `requirements` request through spec-attention (see [human requests](../references/tools/specifications.md#explicit-human-requests)). A draft or stale status is not automatically a user question. Agreement saves a content snapshot; it does not authorize implementation. Do not impose another approval when authority already exists.

Start from the agreed result or equivalent explicit request. Read AGENTS, the project instruction and related requirements. Find affected components through targeted search. First check [task grounds](task.md#choose-a-sufficient-route): a new/changed contract or explicitly requested formalization. Technical work and restoration of specified behavior need no new change. For a new change, use the installed High Grade CLI and native JSON. Commands and formats belong to the [specification reference](../references/tools/specifications.md#structured-specifications). If the project explicitly retains OpenSpec, follow that route until an agreed transition. Old files alone do not select it.

Use selective spec-list and relevant requirements, then `spec-new --title TITLE` with the returned store_sha256. The CLI allocates the ID. Read `spec-read --view editable`. Fill goal, rationale, scope, questions, concrete tasks and operations. Send only changed fields through spec-edit with CAS from the read snapshot. Before modify/remove, read the current requirement; the CLI records its base. On conflict, reread and compare concurrent edits. Use full output/spec-save only when needed. Never edit catalog files directly. Preserve IDs. Migrate an earlier format only through an explicitly agreed transition.

Select tags from spec-tags by domain, usually 1–3 without a quota. Do not add verification or documentation merely because work includes tests or docs. If a durable domain is missing, extend the vocabulary with a clear description. Do not add status, date or individual-task tags. For old integrated specifications, use separate metadata preview/apply. Explain actual depends_on/related_to relationships; links do not inherit content.

Each task describes a concrete outcome, such as rejecting the whole batch for an unknown tag. Avoid a generic implement-and-check stage. Do not rewrite completed historical tasks for formatting.

<a id="контракт-сценария"></a>

## Scenario contract

A requirement defines behavior; a scenario gives a concrete example. Record initial conditions, one meaningful action, observable outcome and material constraints or prohibited effects. Derive the expectation from the agreed rule before implementation. It must distinguish compliance from violation. Test runner details are not product requirements. File existence alone does not prove its content behavior.

Use separate scenarios for a different observable outcome or material risk, not repeated wording. There is no fixed quota; every mandatory outcome needs evidence. One prepared run can cover several scenarios if each outcome is observed and recorded separately. Reading instructions does not prove agent actions.

Form examples, not ready-made requirements for a connected project:

- Application: a valid catalog exists; the user submits a record without a required title; the operation rejects it and leaves the catalog unchanged. The check calls the real save operation and observes state.
- Event process: stock and reservation facts exist; a reservation is requested; check response and new facts. Check repeats only if idempotency belongs to the contract.
- Agent procedure: task and write scope are authorized, an unrelated file exists beside them; the agent reaches the result, preserves that file and does not claim an unperformed check. Observe actions and files, not just response wording.

For each new/changed scenario, choose evidence under [work](work.md#substantive-checks): a standard test, direct observation or semantic review of normative content. Semantic review confirms the rule only; actual execution requires observation. Define reproducible setup, action and observation. Name command/test or observation and outcome. Trace support is a tool property, not a test eligibility criterion. Selecting a check does not prove execution.

Run spec-validate. It checks contract completeness, not task or test execution. An empty template remains a draft. Check meaning: expectation follows the agreed rule, not accidental implementation. Do not adjust requirements to a bug. With a prepared contract and authority, continue to work without formal reapproval.

Historical Markdown/OpenSpec is available at project history paths. Selective transfer needs an explicit request. spec-import creates a draft, preserves original text and checks scenario IDs. Executor and independent reviewer check equivalence. With scenario evidence, spec-integrate transfers requirement ownership to JSON. For transfer of a previous contract only, spec-transfer after completed review and fresh GO records requirements and origin without false PASS. Technical readiness remains open until checks. The original becomes history; unaffected requirements stay current. Do not keep two editable owners of one requirement.

If checks are used, fill scenario links, test file, exact selector, configured runner or manual method, setup, action, observation and causal inputs. shared_inputs reduces repetition. Formats belong to the [verification reference](../references/tools/verification.md). Rechecking an unchanged integrated specification updates results. A new contract requires a new change.

<a id="начальная-проработка-при-init"></a>

## Initial preparation during init

A full init of a new project saves initial specifications for the nearest outcomes under [init](init.md#initial-specifications). Include goal, scope, known requirements and concrete tasks in logical order. Store open questions in questions. Do not present assumptions as agreed rules. Later outcomes may be less detailed; an empty template is not preparation.

Save drafts through spec-edit with CAS. Use optional --validate true for complete contracts. Run spec-validate and explain actual preparation gaps. Invalid format/path is a fault to fix, distinct from incomplete requirements. Validation proves structure, not sufficiency; questions and meaning need review. spec-check assesses execution, so its ready is not an init completion barrier.

On the next task/spec request, refine the same unintegrated ID. A completed contract changes through a new change. Before work, define requirements and checks and resolve material questions. Recommended specification order is not depends_on; dependencies represent necessary results. Specifications own tasks, not a second backlog in survey.
