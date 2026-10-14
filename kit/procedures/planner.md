<a id="зависимая-работа-под-goal"></a>

# Coordinate dependent work

Planner coordinates sequential work; requirements and tasks remain in their primary sources. Use it when explicitly called or when an accepted project rule assigns it to a dependent chain: technical stages, one specification's tasks or related specifications. File count, complexity or independent tasks alone do not trigger it. Apply [route selection](task.md#choose-a-sufficient-route): requirements through spec, implementation and checks through work. A plan-only request limits the result to a plan.

[Clear](clear.md#scope-and-plan) assigns Planner to each requested review or refresh, including a short plan for focused coverage. This specific route changes neither Goal authority nor ordinary direct Work. Strict read-only calls keep the plan and continuation in the response.

<a id="цель-и-полномочия"></a>

## Goal and authority

Identify the final result, scope, criteria and actual dependencies. Locate relevant changes through `spec-list`. Read needed `spec-read` and current requirements, including short dependencies. A tag or link does not establish completeness. If a material outcome is unknown, clarify it through task before creating a Goal.

Creating or continuing a Goal requires the user's explicit choice of Goal mode. A direct Planner call to execute a chain, automatic routing and accepted project routing alone do not supply that choice. Reuse an already known choice without asking again. Before dependent execution, state whether this chain uses a real Goal or an ordinary plan. Read the current Goal when Goal mode is assigned. Continue a matching unfinished Goal; do not replace a different one. On conflict, finish independent preparation and clarify only the occupied Goal decision. If the tool is unavailable or creation is rejected, report the actual status; do not write fictitious state. If the request requires a Goal, dependent work waits for it with one concrete Next. Otherwise use an ordinary plan within authority. Plan mode and plan-only requests do not themselves authorize a Goal or implementation. A separate explicit Goal choice may assign a Goal limited to preparing the plan; it does not permit implementation or override mode restrictions.

With authority, create **one Goal for the entire chain** before dependent execution. Describe the final result, short stages, verifiable completion condition, scope and blocking condition. Link the primary source and continuation state. Do not assign a token budget without an explicit user number. A new stage does not create a new Goal. Follow actual tool conditions for pause, blocked and complete.

<a id="план-и-продолжение"></a>

## Plan and continuation

For one JSON specification, use its tasks and scenarios without a second plan. For several specifications, save one short plan of order, dependencies and barriers; requirements and results remain in those specifications. For dependent technical work with no new contract, save one technical plan. Use the project plan directory if assigned; otherwise `.codex/plans`. A plan is not a second backlog.

Keep one short session state beside the plan or in the assigned project location: source link, observed revision, current stage, one concrete `Next`, blocker and evidence links. Reuse a specification's existing state location. If the chain ends with an external action, use the project's local unpublished location for temporary state. Closing a Goal must not require another commit, push or CI just to update `Next`. Update state at logical checkpoints, not after every command. Describe later stages by outcome and barrier; make the next action executable.

Before each stage, compare files, inputs and evidence with its requirements. Reread sources under rules.md. After interruption, start from the first unconfirmed action. For a native specification, use `spec-check --brief true`; repeat only the named affected check, without a new handoff report. Before an expensive stage, check necessary authority, inputs and environment availability. Stop only dependent work for a real obstacle. Reuse current checks across stages. On failure, fix the authorized cause and recheck the affected part. New requirements or normative scenarios go through spec; do not retrospectively edit a completed contract.

<a id="завершение"></a>

## Completion

Before moving to another specification or technical stage, check its actual barrier. For new specifications, complete spec-validate, evidence and mandatory independent review. Apply spec-integrate after [work](work.md) readiness criteria. Track asynchronous CI separately; add no waiting stage without an explicit project or user condition. Candidate Commit/Push does not close other unfinished criteria. Technically ready tasks may follow each other without human acceptance. Compare all chain criteria with current evidence. Intermediate success does not complete the Goal.

Once the final result is verified, set `Next: none`, mark continuation complete, then complete the Goal. A Goal does not authorize highgrade-commit, local activation, highgrade-push, operational deployment or a human decision. State any open criterion and continue available authorized work.
