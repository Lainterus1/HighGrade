<a id="глобальный-workflow-high-grade"></a>

# High Grade global workflow

Purpose: product boundaries and Project navigation. [Release records](../releases/) retain history; the [self-contained Delivery plan](../archive/plans/2026-09-24/2026-09-24-portable-delivery.md) is complete. global-status shows installed release.

<a id="область-документа"></a>

## Document scope

**Delivery** is portable skills, procedures, templates and CLI. **Project** is this development repository. **Connected project** is an external adapted project. Delivery works without Project access. kit/ owns common rules; connected projects own local decisions. [SPECIFICATION](../archive/workflow/SPECIFICATION.md) is history.

<a id="владение-и-границы"></a>

## Ownership and boundaries

Nine common skills/CLI install into the user profile. Connected projects keep their .highgrade/project/INSTRUCTIONS.md, docs, native High Grade JSON, tests/evidence and earlier specs under local rules. No central project list exists. SVG Vectorizer/Code Health Audit are separate packages.

<a id="пользовательский-маршрут"></a>

## User route

[Common rules](../../kit/rules.md) define routing. [Init](../../kit/procedures/init.md) adapts; [task](../../kit/procedures/task.md) refines; [spec](../../kit/procedures/spec.md) defines requirement changes; [work](../../kit/procedures/work.md) implements/checks. [Planner](../../kit/procedures/planner.md) coordinates dependent tasks/specs when project routing accepts it; user request/Planner determine Goal authority. Technical tasks/fixes under unchanged contracts need no new spec.

[Commit](../../kit/procedures/commit.md) records exact local scope. [Push](../../kit/procedures/push.md) separately publishes selected ranges. [Clear](../../kit/procedures/clear.md) reviews on request. [Update](../../kit/procedures/update.md) separately updates installation. Procedures own details/authority, not this document.

<a id="инструмент-и-доказательства"></a>

## Tools and evidence

[CLI reference](../../kit/references/cli.md) owns specifications, evidence, issues, installation and diagnostics. Standard project tools run tests. trace reads supported reports, not scenario meaning. Unsupported format does not invalidate real standard evidence.

Delivery distributes source through GitHub. Build/install/update support Windows/Linux; CI checks both. GitHub Linux runner image does not define user distribution. [BUILD](../BUILD.md) owns route. Init chooses connected-project stack. Hooks, Covers, binary packages and other platforms are deferred.

<a id="условия-готовности"></a>

## Readiness conditions

[QUALITY](QUALITY.md) defines Delivery checks; [CONTEXT](CONTEXT.md) local context. Confirm installation, skill behavior, publication, activation and human acceptance separately. The bounded pilot is complete; new connected-project work needs a separate request.
