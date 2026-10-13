<a id="контекст-проекта-high-grade"></a>

# High Grade Project context

Scope: local Delivery development. Installed [audit](../../kit/references/audit.md#context-and-adaptation) owns common document/adaptation/budget/consistency rules; [archive](../../kit/procedures/archive.md) owns history. Connected projects do not need this file. [SPECIFICATION](../archive/workflow/SPECIFICATION.md) is history.

<a id="d25--документы-и-их-ответственность"></a>

## D25 — document responsibilities

The [local instruction](../../.highgrade/project/INSTRUCTIONS.md) lists owners. README/AGENTS are entry points; ARCHITECTURE, ENGINEERING and DEVELOPMENT own structure, conventions and commands. kit/ owns executable workflow; native JSON owns current requirements/tasks. Former OpenSpec sources and transfer map stay archived. QUALITY owns Delivery checks.

<a id="d26--непротиворечивость"></a>

## D26 — consistency

Compare common procedures, adaptation, current specs and affected code. When moving a rule to kit/, leave an owner link. Historical SPECIFICATION is not a second current owner.

<a id="d27-b--объём-и-бюджеты"></a>

## D27-B — volume and budgets

[Registry](../../.highgrade/project/documents.json) owns ranges/history/triggers; the [proposal](../proposals/context-budget-2026-09-23.md) explains grounds. Follow common audit rules. Current specs/catalog.json is excluded from ordinary loading: selective spec-list/read suffices. Tasks are in specs/changes/`<ID>`, current requirements in specs/requirements; former origin is history only. CLI checks readiness. null means unagreed budget.

<a id="d28-a--реакции"></a>

## D28-A — reactions

Known overrun needs scope/repetition review. Never increase ceilings just to hide warnings. A material conflict blocks only dependent action.

<a id="d29-a--архив"></a>

## D29-A — archive

Git stores ordinary revisions. Pre-v0-2-1 decisions stay in archive/manifests. Follow common archive rules; do not duplicate spec history/machine reports.

<a id="d30-a--временные-обходы"></a>

## D30-A — temporary workarounds

An existing issue record owns reusable solutions; tasks link it and state impact. Without a record, task notes suffice. [Fixer](../../kit/procedures/fix.md) owns rules. Historical workarounds become no Delivery rule without current requirements.

<a id="d32--штатная-работа-и-дополнительная-актуализация"></a>

## D32 — routine work and additional review

Maintain affected docs with changes. highgrade-clear uses common procedures on request. Hooks/background monitoring are not enabled.

<a id="d33-a--оформление"></a>

## D33-A — style

Primary documents contain current rules/links. Plans, research and long evidence have separate registry roles and load by task.

<a id="снятый-d31"></a>

## Retired D31

Feature/document versioning is not introduced. Edit current documents in place; Delivery versions/format compatibility have a separate contract.

## Reproducible token measurement

Run scripts/context-tokens.py with tiktoken 0.12.0/o200k_base. Install the optional measurement dependency in target/highgrade/tools/tokenizer, not globally. --ref `<SHA>` measures committed inputs; without it, measure current files. --output names the report. Compare the same declared routes/encoding before translation, after translation and after reduction. Reports include per-file hashes, tokens and bytes. This measures selected instruction texts, not full-session billing or model-specific hidden context. Keep budget ceilings unchanged unless the user agrees otherwise.
