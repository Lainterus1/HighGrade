<a id="проработка-задачи"></a>

# Develop the task

For a supported specification, send a material question through an explicit `spec-attention` request (see [decisions](../references/tools/specifications.md#explicit-human-requests)). An answer closes only that question. Agreement on requirements does not authorize execution; the user request defines authority.

Read AGENTS, the project instruction and only the needed current sources. If scope is unclear, locate it through targeted search and read those sources. If the project instruction is missing or incompatible, propose init or clear; do not guess local commands. Only for an unclear task with no chosen mode, offer A — discuss step by step, or B — develop automatically from context. Do not delay a clear authorized request with mode selection. Analysis-only requests remain read-only.

Use short cards for material unresolved questions. Ask independent questions in one round; ask dependent questions after their premise is answered:

```text
D<N> — <decision needed>
Context and consequences: <why this affects the result>
Recommendation: <A/B/C> — <reason>
A) <option> — <benefit and cost>
B) <option> — <benefit and cost>
C) <only if useful> — <consequence>
Custom answer: <the user can describe their own solution>
```

For a real choice, offer two or three mutually exclusive meaningful options. For a unique fact, such as a server address, ask directly; do not invent alternatives. Explain the choice and consequences; do not repeat accepted decisions or permissions. If one line explains the options, do not expand the whole template.

When no questions remain, show a short result: goal, scope, observable behavior, material exceptions and checks. For explicitly chosen stepwise discussion, allow one round of additions or corrections. If the original request already authorizes work and leaves no material choice, pass the short result to spec or work without another confirmation. Choose the route below; do not separately approve a clear authorized route.

<a id="поведение-и-примеры"></a>

## Behavior and examples

Identify the actor, action, intended result and material domain rules. For a complex process, a short chain is enough: event → actor → constraint → change. No mandatory workshop or event architecture is needed. Use project terms. An agreed rule takes priority over accidental code behavior; name a mismatch explicitly.

Task produces goal, scope, material decisions and observable behavior. Do not copy architecture or specifications here; link to the existing owner. Requirement changes go to spec. Explicitly requested formalization of an existing significant contract is also valid.

With an existing catalog, find related specifications through the JSON summary and broad tags before a new task. Consider short dependencies and current requirements, then load only needed spec-read. Links do not inherit text. Do not invent a task link. Verification without a behavior change uses the existing specification.

<a id="выбор-достаточного-маршрута"></a>

## Choose a sufficient route

The criterion is a change in agreed requirements and normative scenarios, not file count, line count or duration. Risk determines checks and review for every route.

| Condition | Action |
|---|---|
| Analysis or consultation | Answer or requested report; no implementation |
| Clear technical task with unchanged contract | Direct work and verification |
| Dependent technical stages with unchanged contract | One technical plan; use Planner when accepted by the project, then work; check Goal authority separately |
| New or changed durable requirement, including access, preservation, compatibility or workflow guarantees | spec before implementation |
| Implementation defect under an existing requirement and scenario | work under that contract, regression check and updated results |

Cache cleanup and link fixes do not need a specification. A multistage refactor may need a plan without a new contract. A small new compatibility rule needs a specification. A plan organizes actions; it does not replace requirements. For a mixed task, specify changed requirements and include related technical steps in its tasks, without artificial requirements or a second plan.

For sequential tasks of one or related specifications, pass coordination to [Planner](planner.md) when the project accepts that route. Keep the requirements route. Automatic Planner selection does not authorize a Goal: it requires an explicit Goal request or a Planner call to execute the chain. A plan-only request authorizes neither a Goal nor implementation. With authority, Planner continues a matching Goal or creates one for the whole chain; its procedure checks availability and conflicts. An explicit call selects Planner independently of automatic routing. The request bounds the work.

For a bugfix, first find the agreed expectation. If requirement and scenario already describe the correct outcome, do not duplicate them. Add a relevant regression test and update evidence. A changed normative expectation or new normative scenario requires a specification change. Do not edit an integrated contract retrospectively. A new test of an existing scenario alone does not change the contract. Resolve a material unknown or conflicting expectation before dependent implementation; do not derive it from faulty code. For a retained earlier format, use the project evidence route without forced import.

Do not create a change just for a task number or to track every activity. spec-list shows specified changes, not a general activity log. For a small technical task, the final result, checks and project edits are enough. Dependent work uses its existing technical plan. Mandatory tracking of all work is unnecessary. Reusable incidental problems may use the optional [Fixer registry](fix.md).

For project creation, [init](init.md#new-project-initial-requirements) defines initial decisions. Task uses the same questioning rules and saves context. After init, start from its ID/Next and open questions. Refine the existing draft instead of creating a duplicate. Completing init alone does not authorize implementation.
