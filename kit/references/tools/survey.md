<a id="обследование-проекта"></a>

# Project survey

[init](../../procedures/init.md) owns route/questions; [audit](../audit.md) owns method. Project .highgrade/project/survey.json works before specs/registry. Views derive from JSON; no second editable report is needed.

<a id="команды"></a>

## Commands

All commands take --root PATH. Except creation, writes require --expected HASH from survey-read/check. Reading/checking runs neither project code nor network and writes nothing.

| Command | Parameters/result |
|---|---|
| survey-schema | document/content schemas from CLI types |
| survey-new | Create draft; never overwrite an existing document |
| survey-read | --view summary\|full\|editable\|markdown or section name; default summary; survey_sha256 |
| survey-edit | --input patch.json; top-level content patch, whole-array replacement |
| survey-snapshot | Record selected files/area contents |
| survey-review | --reviewer NAME --conclusion TEXT --verdict go\|no_go; actual semantic review record of content/snapshot |
| survey-check | Structure, input drift, obstacles, review freshness; unknown exits 2 |
| survey-reopen | Preserve completed revision in history, start next with previous decisions |

Route: new → read editable → edit → snapshot → semantic review → review → check. Each write uses a fresh HASH. edit transitions: {"stage":"planned"}, then {"stage":"adapting"}; completion: {"stage":"completed","next":""}. Content edits retain actual stage unless specified. Material diff makes review stale; refresh affected snapshot/review before dependent action. Do not retrospectively record intermediate stages.

<a id="модель"></a>

## Model

[Schema](../survey-schema.json) and [content template](../../templates/survey.json) are shipped. CLI adds link/path/state constraints.

Content: goal, scope (relative areas, . means root), stage, next, decisions, sections, source_paths (hashed files), areas (contents). Sources/areas stay within scope. Seven initial unknown decisions: purpose, scope, preservation, changes, local_rules, environment, mandate.

Sections: components, knowledge, processes, skills, observations, findings, adaptation, routes. Empty means unexamined. Record inapplicability with reason. Map describes components/paths/links; knowledge records owners; observations record reading/actions/limits; adaptation holds proposals/results/task links; routes show representative future chains without quotas.

Item: id, state confirmed|proposed|unknown|not_applicable, description, basis (decision/observation/reason sources), sources (from source_paths), blocks (actions dependent on unanswered questions). IDs are unique per revision. confirmed/not_applicable need nonempty basis; a record does not prove truth. Command observations describe conditions, command, date/revision, result/report. Never record secrets.

System blocks: plan, adaptation, completion. execute-adaptation is a legacy synonym of adaptation; other strings are other_dependencies. Planning needs examined sections, current snapshot/go and no plan blocker, but retains adaptation blockers. Adaptation rejects unresolved key decisions, empty sections and adaptation/execute-adaptation blocks. Completion requires no blockers. An unknown independent component may have empty blocks and remain a limit. All dependent actions within the request must be authorized. Out-of-scope actions are limits, not fictitious mandatory blocks.

<a id="актуальность-и-сохранность"></a>

## Freshness and preservation

Check separates structure, snapshot_current, changed_sources, unknown_sources, blockers, review_record_current, review_go, plan_ready/adaptation_ready/completion_ready and historically_completed. Semantic correctness remains not_certified. The tool records no human acceptance. Negative review blocks readiness. Stage changes check technical prerequisites/current go; permission=not_assessed. Request/mandate supply authority. Saving actual stage with stale go is allowed for facts, not dependent execution.

Snapshot holds file hashes, area contents/completeness/skips and exclusions hash. Diagnose material drift. An unrelated file/new Git SHA alone does not invalidate a result. Read new components by risk. Survey/administrative files are not their own inputs. Keep derived views outside tracked scope or explicitly exclude them.

Completed revisions cannot be edited/snapshotted/reviewed; reopen preserves them in history and clears current snapshot/review. Historical completion does not mean freshness. Document including history is limited to 8 MiB. No automatic history removal.

CAS, lock and atomic replacement protect one JSON. Conflict, unknown schema/fields and invalid input preserve old data. After a crash, survey.lock/survey.next block writes. Confirm no writer, compare/preserve needed data before removing a confirmed remnant. No automatic cleanup. This is not a whole-project adaptation transaction.

<a id="структурный-обзор"></a>

## Structural overview

inventory --mode structure [--scope PATH] returns names/types without content reading except exclusions configuration. Limit 5000 entries, depth 64. A local obstacle returns available scope with unknown. Unsafe/invalid root/scope is rejected. Complete means allowed traversal, not a complete semantic audit.

Bootstrap private-name/cache/exclusion rules apply. .highgrade/project is accessible; other .highgrade administrative areas are excluded. Links/junctions are not followed. Snapshot reads selected regular files up to 32 MiB, 256 sources and 32 areas. Forbidden paths reject; unavailable regular files stay unknown.

Inventory without mode and legacy-install retain earlier results/errors. Partial overview cannot replace an audit receipt. With a registry, ordinary inspect checks assigned roles; bootstrap remains pre-registry diagnostics.

<a id="новый-проект"></a>

## New project

[init](../../procedures/init.md#survey) defines tool use/scope. Observed absence may be confirmed with an observation basis. An accepted future design is a specific adaptation claim, not component existence. proposed means suggestion, unknown means unknown; not_applicable needs actual inapplicability, not missing implementation.

Empty projects may have empty source_paths. snapshot/check then do not prove code reading. Future paths are not existing sources. Update the map after creating files. Completed refers to agreed preparation: future product questions stay in specifications; material preparation gaps remain blockers. An incomplete draft can coexist with completed survey. This authorizes neither work nor product acceptance. The executor checks actual foundation existence; structural go does not certify record meaning.
