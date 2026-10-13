<a id="метод-независимого-ревью"></a>

# Independent review method

A separate reviewer reads this method when [Work](../procedures/work.md#evidence-and-review) calls it. Work owns the trigger and readiness barrier. Review does not replace tests or a human decision.

<a id="вход-и-область"></a>

## Input and scope

Obtain requirements and scenarios, the relevant diff, original reports, declared inputs, limits and readiness criteria. Identify the result revision and material untracked files. For an import, compare original_text. Mark an unavailable necessary input as a gap; do not confirm the dependent conclusion.

Check independently. The executor's summary does not replace the original result. Expand scope for a specific risk or contradiction. Preserve unrelated changes.

<a id="сверка"></a>

## Checks

1. Compare each mandatory outcome with the changed implementation and actual product boundary.
2. Compare setup, selector, executed branch and test assertions with the required outcome. Identify the material violation the check detects and whether expectations follow the contract. If expectations changed, check the grounds: correcting a faulty test is valid; changing a normative outcome requires Spec. Weaker assertions for a green result do not confirm a requirement. This belongs to final review; no mandatory early reviewer or mutation tool is required. Check the original report, skips, time and revision. Exit code, ID and file alone do not prove execution. A script or browser test is still automatic outside `trace`. In a shared run, check each scenario outcome.
3. Check completeness and freshness of causal inputs: code, test, data, configuration, shared modules and important dependencies. A shared report or the whole repository is not automatically an input of each scenario. Do not exclude an unknown dependency by guesswork.
4. Assess regressions, boundary outcomes, data preservation and unrelated changes by risk. For normative text, compare meaning and the decision table. Reading instructions does not prove agent actions.

Read the [verification reference](tools/verification.md) for `runs`, `trace` or evidence formats when needed.

<a id="вердикт"></a>

## Verdict

Return **GO** only with current confirmation of mandatory outcomes and no blocking gaps. Otherwise return **NO_GO**. For a finding, state scenario, location, observation, consequence and required fix or check. State review limits. Missing evidence cannot become GO.

After a material fix, review the new revision and affected evidence. The main agent records the verdict. The reviewer does not record human acceptance or run commit/push.
