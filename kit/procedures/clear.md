<a id="актуализация-состояния-проекта"></a>

# Review and refresh project state

Clear performs requested review or maintenance of documentation, tests and their project context. It is not a deletion command. A request to analyze authorizes reading only; requested refresh permits fixes within its scope. Routine maintenance during Work does not require Clear.

<a id="область-и-план"></a>

## Scope and plan

Determine the selected areas, coverage and permitted actions from the request and available context. Areas may be documentation, tests or both, including named components. Focused coverage deeply checks that area and its material dependencies; comprehensive coverage checks all main areas of the selected project scope. Coverage does not lower the required quality.

If a material parameter is unknown, ask one short round about what to review and its coverage. Reuse known answers. Do not ask the user to enumerate files the project already identifies. Do not begin work that depends on the missing answer.

Always apply [Planner](planner.md), including a short plan for a focused review. Planner owns Goal authority, mode choice, continuation and tool conflicts. State the selected mode and reuse the user's known choice under its rules. For strict read-only requests, keep plan, coverage and continuation in the response without creating files. Before baseline execution, extract mandatory methods from selected contracts and project rules. Define stage outcomes, required observations and completion criteria.

<a id="поэтапный-обход"></a>

## Staged review

1. Map the selected components, main directories, source owners and material relationships through [audit](../references/audit.md). Include relevant hidden configuration, build/CI, operations and dependencies. Classify generated, historical and excluded areas with reasons. For an uninitialized project, `inspect --bootstrap` gives a structural starting point; it does not prove content review.
2. Read each selected area's authoritative sources and its material dependencies. Track files found, sources actually read, checks executed and remaining gaps separately. A directory listing, hash or successful link check cannot mark content reviewed.
3. Compare documentation with current requirements, implementation, configuration and commands under [documentation](../references/documentation.md). Find both incorrect descriptions and important undocumented behavior. Check prerequisites and material effects; execute relevant checks only within authority. Determine whether a mismatch is a documentation error, implementation defect or requirement decision before changing it.
4. If tests are selected, apply the [test quality method](../references/review.md#test-currency-and-effectiveness) to every attached current requirement, including unchanged sources, and actual assertions. Review owns contract gaps, method selection, baseline and mutation checks. Record semantic contract coverage separately from mutation selection and results. A documentation-only call does not assign this audit; ordinary checks of edited documentation still apply.
5. Fix authorized findings through [Work](work.md). Changed requirements go through Spec. A finding does not expand write or execution scope. Recheck affected outcomes and current project barriers; use existing independent review for significant changes. Preserve unrelated data and history.
6. Reconcile the coverage map, findings and original reports with the initial criteria. Report authorized fixes, semantic contract coverage, test/mutation outcomes and open mandatory criteria separately. Successful mutations do not close an unread contract or missing mandatory method. A partial result keeps a concrete Next; do not shrink the initial scope to complete a Goal. Use measured results for cost claims, with missing telemetry stated as unavailable.

Use the project's existing plan/evidence locations. The coverage map is a compact record of observed work, not another requirement catalog or backlog. Later stages may change when a finding establishes a material dependency; name the reason and its scope.

<a id="существующие-контракты"></a>

## Existing contracts and preservation

Use `doctor`/`inspect` and applicable standard checks. Apply the [compatibility contract](../references/tools/diagnostics.md#project-compatibility): schema and meaning are separate, accepted exceptions remain, and project instructions are not rewritten automatically. Compare source ownership, links, budgets, archive, workarounds and skill responsibilities within scope. Budget increases and functionality removal need a separate material decision. Check OpenSpec only where the project explicitly retains that route.

Check specs (or specs-location.json), current requirements, orphan links, evidence freshness and broad tags. spec-list/spec-read do not run checks. Repeat spec-run only within authorized scope. Do not move files by status or delete the project catalog with .highgrade. Assess context before merging tag synonyms. An agreed merge uses spec-tag-merge in one transaction.

If issues/ exists within review scope, use issue-list/read and selective issue-validate. Check solution applicability, verification_current, evidence links and material open workarounds. A faulty or stale record is a finding, not authority to rerun its recipe. Do not create a missing registry. [Fixer](fix.md) owns reusable solutions.
