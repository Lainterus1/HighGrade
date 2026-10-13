<a id="спецификации-и-каталог"></a>

# Specifications and catalog

<a id="структурированные-спецификации"></a>

## Structured specifications

Native directory formats v3/v4/v5 use project specs/: catalog.json has version/counter, requirements/`<ID>`.json current requirements, changes/`<ID>`/spec.json changes, results.json evidence, check history and human decisions. Empty results are not created. The catalog belongs to the project and survives High Grade removal. Do not move accepted specifications; status is data. New projects need no OpenSpec. Set highgrade_spec_format: native-v1 in the instruction; this marks the workflow family, not store version. Historical projects may explicitly retain earlier routes. Consumer CLI works without Node/Rust/OpenSpec; project tests need their own environment.

All commands return JSON Report: findings with code/location, measurements with results, limitations with check limits. spec-read --view editable exposes working fields in measurements[].value. spec-edit takes editable fields only. Full/spec-save retain compatibility. spec-schema returns directory_format (catalog/spec/results/tags/runners), legacy store, change/evidence_input/decision_input JSON schemas from CLI types. Unknown versions/fields reject. Structure does not prove semantic quality.

| Command, all with --root PATH | Parameters/result |
|---|---|
| spec-schema | Schemas/required fields |
| spec-list | JSON summary; optional --tag ID --technical STATUS --human STATUS; selected/total and short dependencies |
| spec-new | --title TITLE --expected HASH; next HG number/common template |
| spec-read | --id HG-CHANGE or --requirement HG-REQ; selected object |
| spec-save | --id HG-CHANGE --expected HASH --input edit.json; save change revision |
| spec-diff | --id HG-CHANGE; operations/current requirements |
| spec-validate | --id HG-CHANGE; requirements/scenarios/tasks completeness, questions/base |
| spec-integrate | --id HG-CHANGE --expected HASH; readiness check, requirement integration/change archive |
| spec-transfer | --id HG-CHANGE --expected HASH; checked prior-contract import without passed behavior claims |
| spec-abandon | --id HG-CHANGE --expected HASH --reason TEXT; rejected draft history, unchanged requirements |
| spec-migrate | --expected HASH --to directory: v1/v2 to directory; without --to, prior v1 to v2 |
| spec-decide | --expected HASH --input decisions.json; atomic human decisions for selected revisions |
| spec-import | --id HG-IMPORT --expected HASH --input requirement.json --source legacy.md; selective-transfer draft |

First spec-list returns store_sha256: absent. Pass it to spec-new. Later writes use store_sha256 from the actual edited snapshot. Concurrent writes yield StoreConflict: reread, compare and repeat only your edit. Protected spec-save fields: id, baseline, scoped_baseline, revision_tags, imports, evidence, review, archived, abandoned_reason, created_at, acceptance, history, runs. add/modify takes requirement; remove takes id/nonempty reason. Requirement/scenario IDs use uppercase project prefix and hyphen segments, such as HG-CART-R1/S1. Removed IDs are never reused.

Import takes a schema Requirement and Markdown with ### Requirement: [ID] / #### Scenario: [ID]. CLI checks the complete scenario-ID set and preserves the original block in imports. This is a draft, not automatic translation confirmation. Check meaning, transfer completeness and evidence before integrate. Preserve old files; after integration, JSON owns selected IDs. Unsupported Markdown remains a historical source pending a separate decision.

Legacy stores remain readable. v1 writes need migration. Directory writing/recovery rules follow. Direct file edits bypass tool checks.

<a id="номера-версии-и-приёмка"></a>

### IDs, versions and acceptance

Normal spec-new allocates HG-0001, HG-0002, etc.; the agent writes title. next_number/change are one transaction; abandonment does not free numbers. --id remains for old clients/selective import, not new tasks. Dependent IDs use R1/S1/T1 suffixes. title is editable; id/creation date are not. created_at/decided_at are Unix UTC seconds, set by CLI. Numbering belongs to one catalog; independent Git branches have no shared counter. Create changes in an agreed primary catalog before branching. Same-ID merge conflicts need explicit resolution, never automatic version choice.

CLI reads v1 without byte changes, but writes need spec-migrate. It saves exact .highgrade/specs/backups/v1-`<sha>`.json before atomic replacement; repeat is safe. IDs/evidence/old reviews remain. Unknown title/created_at stay empty/null; acceptance starts empty. Never invent dates. JSON migration owns the backup; OpenSpec files remain untouched. A new release does not migrate other projects automatically.

spec-list computes technical in_progress|ready|abandoned and human pending|accepted|needs_changes|stale. summary counts these separately; do not add the two sets together. Completed tasks without current evidence/positive review are not ready. Integrated changes are checked against their own inputs/reports; lost files remain gaps. Previous decisions stay in change.acceptance even when stale.

spec-transfer applies only to selective spec-import with add-only operations: equal import/operation counts, matching source path/SHA, retained scenario IDs, completed comparison tasks and current independent GO. It atomically transfers requirements/origin, archives the change and returns BehaviorUnconfirmed. It does not replace integration: without current scenario evidence, spec-check stays failed, technical in_progress, human pending. New/changed requirements need ordinary verification before integration.

spec-check --brief true/full spec-read returns decision hashes: change_sha256 binds specification/evidence revision; inputs_sha256 binds actual declared checked inputs. The system records a decision for this snapshot. An exact Git SHA/immutable build ID is optional verified_revision. Its absence means unknown external version, not unconfirmed acceptance. A snapshot hash is not Git SHA. Dirty-tree HEAD does not identify checked implementation. After explicit human decision, prepare:

```json
{
  "decisions": [{
    "id": "HG-0001",
    "decision": "accepted",
    "decided_by": "project owner or decision source",
    "change_sha256": "snapshot shown to the human",
    "inputs_sha256": "checked input snapshot",
    "comment": ""
  }]
}
```

Comment/verified_revision are optional for both result decisions. Without a version, spec-decide uses the unambiguous version of open result requests for the same snapshot. Ambiguity/explicit mismatch rejects the batch. One invalid/stale item rejects all. accepted needs current spec-check. Recording does not change technical review. Decisions bind supplied hashes and preserve history. CLI certifies neither human identity/build ID nor input completeness. [work](../../procedures/work.md#handoff-to-a-human-and-decision-recording) owns authority/timing.

<a id="каталог-миграция-и-восстановление"></a>

### Catalog, migration and recovery

With initialization authority, call spec-init --root PATH, optionally --directory docs/specs. The directory must be absent/empty; do not claim foreign documents. A nonstandard path goes in project specs-location.json, independently of .highgrade. Without init, first spec-new creates the default catalog. spec-read joins the editable change; physical spec/results remain separate.

Existing v1/v2 needs explicit spec-migrate --expected HASH --to directory. Preserve exact bytes in root specs-backup-`<SHA>`.json. The former store stays historical. Editing it with old CLI yields LegacyStoreConflict, not silent data selection. Preserve IDs, requirements, origin, observations, reviews and decisions. No mass OpenSpec conversion. Global installation and project migration are distinct.

Each write changes only affected files under one lock. Pending transaction.json blocks reading. spec-recover --expected HASH from the error verifies all old/new bytes and explicitly completes writes. Never remove journals manually. After crash, check write.lock owner first; remove only a confirmed orphan. Staging recovers from the journal. Document/transaction limit is 8 MiB; exceeding rejects, not truncates history. Recovery repairs interruption, not disk corruption.

<a id="связи-и-теги"></a>

### Links and tags

Change depends_on/related_to arrays use {"id":"HG-0001","reason":"Required result"}. Targets must exist. Dependency cycles reject. Readiness needs integrated, nonabandoned depends_on targets; related_to aids navigation. Links inherit no content. Returned links give short context, including changes of the same requirements. Read their text separately.

tags is an ID array from flat tags.json. Commands: spec-tags; spec-tag-set --id ID --title TITLE --description TEXT --expected HASH; spec-tag-remove --id ID --expected HASH; spec-tag-merge --from OLD --into NEW --expected HASH. Used tags cannot be removed; merge updates links/registry in one transaction. Use broad domains, not dates/statuses. Navigation metadata changes do not change technical revisions. Filters do not guarantee complete context. summary_scope defines counters; selection.total is catalog count and selection.selected returned changes.

spec-migrate --to compact explicitly produces directory format 4 with SHA-256 shared history objects. Reading restores observations/order/dates. Transition is journal-protected; interruption needs spec-recover. Format 3 remains writable without implicit migration. Old CLI rejects format 4; use compatible CLI afterwards. Internal store model stays 3; catalog.json gives file-format version. Migration does not certify historical checks.

spec-migrate --to shared-inputs explicitly changes v3/v4 to format 5. Each results.json input_manifests table stores files/optional input_paths once per SHA-256. Current/historical evidence uses input_manifest_sha256. CLI restores the original model before history/freshness checks. Missing/corrupt references, bad hashes or mixed references/embedded inputs reject. Preserve history, reviews, decisions, dates and input scope. New writes retain format 5/reuse equal sets; v3/v4 stay unchanged without explicit migration. Old CLI rejects 5; reverse --to compact is forbidden. Same journal/8 MiB limit applies. Do not remove history for overflow or migrate during installation. Check compatible CLI/full transition on an isolated copy first. Afterwards edit only through compatible CLI.

<a id="метаданные-и-исходная-база"></a>

### Metadata and base revision

spec-metadata --expected HASH --input metadata.json previews without writing. Input: [{"id":"HG-0001","tags":["specifications"]}]. After review, apply the same batch/hash with --apply true. On drift, reread/compare. Unknown IDs/tags, duplicate IDs and invalid schemas reject all. Drafts/integrated changes are supported. Exact repeat writes nothing. Only tags and administrative compatibility support change, not contract/evidence/decision history.

Tags are excluded from semantic revision. Protected revision_tags retains previous tags only for historical fingerprint checks. Do not edit it. Current old review/decision stays current; semantic/evidence/input-stale records are not restored. Ordinary tag edits/merge use the same mechanism.

New changes have scoped_baseline:true and empty baseline. First modify/remove needs global --expected from a catalog read containing the requirement; local CAS cannot select an unseen target. CLI captures its current hash from CAS. Missing targets reject. Captured bases do not refresh on later edits or removed/readded operations; drift yields BaseDrift. Resolve through a deliberate new change, never manual protected hashes. Old full baselines stay unchanged.

JSON presentation order: id/title/tags → intent/links → operations/scenarios → tasks/checks → administrative fields. This is presentation, not a new nested format; canonical hashes ignore output field order.

spec-validate --id HG-ID checks one change. Without ID, it checks all nonabandoned changes from one snapshot. validated_changes lists IDs; findings use /changes/`<ID>`/… paths. Both modes use the same rules without writes. Success proves neither tasks/evidence/review nor acceptance.

<a id="короткий-цикл-редактирования"></a>

## Short editing cycle

spec-read --id HG-0001 --view summary shows goal/scope/questions/tasks/links. editable gives only editable fields. requirements with ID gives change operations; without ID gives current requirements, optionally --tag TAG by explicit tags. Tags do not prove domain completeness. full retains earlier full output.

spec-edit --id HG-0001 --input patch.json --expected HASH takes only needed editable fields; arrays replace entirely. History, ID, baseline, results, review and decisions cannot be supplied. --validate true checks completeness before writing and leaves catalog unchanged on failure. Without it, successive drafts get schema validation. Reading/editing runs no tests. After review, spec-integrate --id HG-0001 --expected HASH --brief true checks readiness/integrates/returns brief. Preliminary spec-check is needed only to inspect reasons.

shared_inputs is a common relative-file list added to each automatic check's inputs. Changed shared inputs need affected rechecks. Do not include result history/the whole repo without reason. shared_inputs/evidence.input_paths need compatible CLI. Old catalogs read unchanged; old CLI must not edit new fields.

spec-list --id/--tag limits readiness computation before technical/human filtering. summary_scope defines counters; selection.total is catalog size. Files are still read as one consistent snapshot. spec-read gives local_sha256. spec-edit --expected-local HASH allows unrelated catalog changes but detects selected specification, dependency, requirement and runner changes. Pass either expected or expected-local. Global CAS remains compatible.

[Verification](verification.md) owns evidence/checks/review/execution.

<a id="явные-обращения-к-человеку"></a>

## Explicit human requests

spec-ui returns one read-only projection: categories/counters, primary action, history and exact post-agreement field changes. directory-scoped-v1 means v3/v4/v5 catalog, scoped_baseline=true, creation date, nonempty title/operations. Older profiles are excluded with reasons. Corruption yields an error, not an empty list. Reading never migrates.

spec-attention --id ID --expected HASH --input request.json takes attention_input from spec-schema. Request: action=request, unique retry request_id, kind=question|requirements|result, reason and content_sha256 from spec-ui. Exact retries create no duplicate. requirements needs a prepared contract; result needs current evidence/review. Optional known verified_revision is allowed for result; empty means unknown, and the form does not ask the human for it. Questions do not depend on technical-input availability.

Response: action=respond, request_id, content_sha256, decision=answer|accepted|needs_changes, author, comment. Optional result verified_revision defaults to the request version if present. A different explicit version rejects. answer applies only to question. Question answers/returned requirements need text; returned results may have no comment. Record responses only on explicit human decisions. Requirements agreement is content approval, not implementation authority. Result acceptance uses spec-decide, input hashes and technical barriers. Concurrent/stale writes reject all.

Protected optional attention in spec.json stores requests, numbered responses and exact content snapshots. New result requests may store verified_revision; responses store a known version or empty string. A retry must match the original version and pass revision/input/readiness checks. Old responses without the field read without migration; retries need an unambiguous acceptance version. Mismatch/ambiguity rejects without writes. Once verified_revision is present, old CLI unaware of it must not edit. Absent attention means no decisions. Tags, completion flags and equivalent checks do not change agreed content; changed content makes agreement stale and needs a new explicit handoff. Old CLI unaware of attention must not edit it.

Queue order: question → requirements → result, then request time/ID. One row has all current requests. Technical unknowns appear separately and do not hide questions. Work-in-progress holds unintegrated records without open requests; it does not mean an agent is running. Integrated records without current readiness/acceptance use historical with integrated flag, visible through All without a new tab. Abandoned records are separate, not integrated results. Only technically ready accepted results count as completed. No AI summary.
