<a id="диагностика-и-документы-проекта"></a>

# Project diagnostics and documents

Use [survey](survey.md) for sequential connection. With a current registry, ordinary inspect checks accepted roles. Bootstrap below is pre-registry diagnostics.

<a id="предварительная-проверка-документов-v0-2-5"></a>

## Document preflight (v0-2-5)

inspect --bootstrap --root `<absolute-path>` works before registry creation. measurements[0] has slots for seven paths: README.md, AGENTS.md, docs/ARCHITECTURE.md, docs/ENGINEERING.md, docs/DEVELOPMENT.md, .highgrade/project/INSTRUCTIONS.md and .highgrade/project/documents.json. path_state shows exact presence. candidates gives bounded alternatives by filename or labeled Markdown link. candidate_status=not_needed means the exact path exists; not_found_in_scan does not mean content is absent. required_links separately lists six required relationships: README to four documents, AGENTS to project instruction, instruction to registry. Do not combine --bootstrap with --registry/--scope. Invalid root returns failed; missing exact paths/links return structural failed; inaccessible scope/limits yield unknown findings. No files are created/moved.

candidate_scan=performed means missing-path alternatives were searched. If all seven paths exist, candidate_scan=not_needed, no tree walk occurs and visited_entries=0. Required links/exclusion list still get checked. Structural failed before connection gives adaptation tasks; it does not cancel audit. Root errors or no usable JSON report stop dependent work. Record content and move decisions in init's adaptation table.

When needed, candidate search includes new files/hidden instructions independently of Git. It reads only inventory-exclusions.json and three canonical link sources: README, AGENTS and project instruction. Valid project exclusions apply before traversal; their free-text reasons are not printed. An invalid list yields unknown and is not partially applied. .git, node_modules, target, .pnpm-store, .output, .tanstack, .highgrade/releases, caches, known private names, symlinks and Windows reparse points are not read. .highgrade/project stays in scope. Exclusions do not prove audit approval.

Limits: 50,000 entries, 128 reads, 64 KiB per input file, 12 candidates per role, 32 displayed exclusions, 200 bytes per path, depth 64 and 64 KiB JSON report. Reaching a limit yields unknown; undisplayed scope remains agent work. Ordinary inspect requires a registry and checks assigned roles/paths, mandatory links, other registered links and budgets. It neither replaces semantic audit nor repairs projects automatically.

<a id="начальный-реестр-документов"></a>

## Initial document registry

Init foundation for .highgrade/project/documents.json follows. Set real scope, additional sources and agreed budgets. null deliberately yields BudgetNotAgreed, not PASS. active: true with loading: history checks sources/links but excludes them from ordinary loading. active: false excludes the row from checking. Do not widen scope just to hide AreaUnclassified.

```json
{
  "schema_version": 1,
  "documents": [
    {"id":"readme","path":"README.md","role":"purpose-navigation","loading":"entry","scope":[],"budget":null},
    {"id":"agents","path":"AGENTS.md","role":"agent-rules","loading":"entry","scope":[],"budget":null},
    {"id":"architecture","path":"docs/ARCHITECTURE.md","role":"current-architecture","loading":"task","scope":[],"budget":null},
    {"id":"engineering","path":"docs/ENGINEERING.md","role":"engineering-rules","loading":"task","scope":[],"budget":null},
    {"id":"development","path":"docs/DEVELOPMENT.md","role":"commands-procedures","loading":"task","scope":[],"budget":null}
  ],
  "additional_sources": [
    {"path":".highgrade/project/INSTRUCTIONS.md","role":"project-adaptation","active":true,"loading":"entry","scope":[".highgrade/project/documents.json"]}
  ],
  "route_budget": null
}
```

Paths/scope are root-relative. An agreed budget uses {"unit":"bytes","agreed":true,"min":1000,"baseline":2000,"current":2000,"ceiling":3000,"history":[],"review_trigger":"after two tasks"}. These numbers are illustrative, not accepted values. current changes retain history with from, to, reason, reduction_considered and next_review. Assign the five primary roles unambiguously. Use role string or nonempty roles array for a multi-role document. Paths/IDs are unique. Count one combined file budget once. Template paths are optional; inspect checks registered paths/links. Administrative INSTRUCTIONS.md/documents.json remain in .highgrade/project/.

<a id="совместимость-проекта"></a>

## Project compatibility

Doctor supports highgrade_project_schema: 1 at the start of a UTF-8 instruction between --- lines with LF/CRLF. project_instruction=compatible is limited to compatibility_scope=schema-only and semantic_compatibility=not_assessed. It checks neither completeness nor meaning. Unknown schemas are not downgraded automatically. For a mandatory runner, use doctor --action spec-run. A project command confirms tool version. [init](../../procedures/init.md)/[clear](../../procedures/clear.md) own adaptation/conflicts.

CLI exit codes: 1 failed, 2 unknown, 0 other states including warnings. Capture $LASTEXITCODE immediately before another process. A shell/wrapper code may differ. If JSON/code disagree, do not guess; repeat minimal diagnostics with explicit capture.

doctor --root PATH --action spec-read checks catalog. --action spec-run --id HG-ID [--check CHECK] checks selected inputs/executables without executing project code. No action keeps earlier diagnostics. Use global-status/preview for installation/update.

Bootstrap returns validation, adaptation, applied. Missing standard paths/links indicate adaptation; corruption/unavailability remain obstacles. Previous status/exit/findings stay compatible. New fields separate technical checks from human decisions.

<a id="исключения-inventory-и-прежний-receipt"></a>

## Inventory exclusions and legacy receipt

.highgrade/project/inventory-exclusions.json: {"schema_version":1,"entries":[{"path":"relative/path","reason":"specific reason"}]}. Paths are relative, unique and outside .highgrade; at most 128 entries, each with a nonempty reason. CLI returns exclusions/file SHA256. Snapshot exclusion does not prove scope reading; see [audit](../audit.md).

Only legacy-install checks exclusions_sha256 in the audit receipt. It must match configuration or be absent/null when no file exists. Changed exclusions need a new matching legacy receipt. global-install/global-update need no receipt and do not survey projects.

<a id="состав-передачи-доказательств"></a>

## Evidence delivery contents

doctor --action spec-delivery --id HG-ID --root PATH [--bundle files.json] checks required reports and recorded inputs by hash. Without bundle, it uses Git index and distinguishes tracked-but-edited, untracked and ignored files. It compares actual index blob bytes without Git filters. Even normalization differences yield index_differs. Preserve selected recorded bytes in delivery. It changes neither index/ignore nor decisions. Unavailable contents yield unknown. Run after index preparation and before commit.

Without Git, files.json is an explicit JSON array of selected relative package paths. It checks current files, not a future remote copy. Missing mandatory reports/changed hashes yield failed. Excluded files/unknown contents yield unknown. All recorded inputs are included conservatively; the tool cannot infer recoverability. Unrelated logs, caches or screenshots are unnecessary. A URL does not prove accessibility/retention; check external routes under local rules. Delivery verification replaces neither spec-check nor human acceptance. It is optional for local work without handoff. noGit Commit/Git-push are unnecessary; Work uses project-assigned immutable revision/package. files.json alone is not that identifier.

Inspect diagnostic_summary separates errors, unknown and warnings, retaining findings/exit. Compare accepted limits with survey basis/sources and local instructions; summary does not certify acceptance. Unknown budgets remain null/BudgetNotAgreed even with authority to continue. Structural errors need fixes; a limit statement cannot suppress them.
