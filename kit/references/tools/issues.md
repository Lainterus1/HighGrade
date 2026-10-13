<a id="реестр-проблем"></a>

# Issue registry

Optional project catalog: issues/catalog.json and issues/entries/ISS-0001.json, schema 1. Records are portable JSON; .highgrade/issues/ holds only locks/temp files. First write creates the catalog. Reading an absent catalog creates nothing. Store neither secrets nor full logs here.

<a id="команды"></a>

## Commands

highgrade `<command>` --help and issue-schema show schemas. Except help, commands take --root PATH.

| Command | Parameters/result |
|---|---|
| issue-list | Optional --query, --tag, --status; summary, verification_current, store_sha256 |
| issue-read | --id ISS-0001, --view summary\|full; full includes occurrence receipts |
| issue-new | --expected HASH --input content.json; new open record |
| issue-edit | --id ID --expected HASH --input patch.json; content fields |
| issue-record | --id ID --expected HASH --input observation.json; one observation |
| issue-validate | --id ID; catalog structure, state and record evidence availability |
| issue-recover | --expected HASH; complete detected interrupted transaction |

Use store_sha256 CAS from a fresh read; absent catalog uses absent. On conflict, reread/merge deliberately. content schema is issue-new input without a wrapper; patch is issue-edit. Arrays/nested objects replace whole values. Unknown/administrative fields are rejected. Never edit catalog files directly.

<a id="карточка-и-состояния"></a>

## Record and states

content has title, tags, status, symptom, applicability (environment/conditions), cause (status unknown|confirmed, description), resolution (description/prevention) and related_specs. Tags are short lowercase ASCII names with digits/hyphens. query is case-insensitive across content. Specification links aid navigation; they inherit no requirements and prove neither existence nor readiness.

- open: problem remains; solution unconfirmed.
- mitigated: original action repeated successfully; operation restored, cause may remain unknown.
- resolved: successful solution check with confirmed cause.
- stale: earlier verification no longer applies after material edits or failed repetition.

issue-record takes occurrence_key, outcome passed|failed|unknown, action, observed and evidence (relative file paths). A key identifies a real occurrence and stays on command retry. Identical content/files do nothing; different content with the same key conflicts. A new observation needs a new key. CLI owns occurrences, Unix UTC seconds and retry receipts. These are compact receipts, not full-log histories.

After a passed record, edit status to mitigated/resolved. Both require solution description and current passed verification; resolved also needs confirmed cause. Changes to symptom/applicability/cause/solution reset last_verified_at and make confirmed records stale. Title/tag/link edits do not. An old occurrence_key cannot verify edited content. Failed/unknown reset confirmation; use a new key for a new check. edit status=open reopens.

Evidence files are hashed. Changed/missing files warn on read/validate and yield verification_current=false in lists. Reading leaves status unchanged; stale files cannot justify confirmed state. issues catalog/.highgrade administrative files cannot be evidence. Empty evidence is allowed for direct observation, but action/observed remain required. CLI neither runs checks nor certifies observations/diagnosis. Records are never executed as commands.

<a id="целостность-и-восстановление"></a>

## Integrity and recovery

Writes use a lock and whole-catalog CAS. Validate before edits. Header/record updates use issues/transaction.json. Until completion, normal commands refuse and provide the recovery hash. Recovery accepts only original/already applied bytes for both targets. Preserve foreign edits for diagnosis. Inspect error/journal before recovery; repeating partially applied recovery is safe. Never delete another process's active lock. After a crash, confirm the named process ended, then remove only its orphan administrative lock.

Unknown schemas, foreign root files, unsafe paths and corrupt records are rejected. File limit is 8 MiB; receipt history is not truncated automatically. Issues do not replace specifications or use their integration/human-acceptance route. [Fixer](../../procedures/fix.md) defines when/what to record.
