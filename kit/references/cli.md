<a id="инструменты-поставки-high-grade"></a>

# High Grade Delivery tools

highgrade `<command>` --help shows options, example and input schema without reading a project. All operations return JSON Report. Use active-release CLI; for a new format under development, use an explicitly identified compatible candidate.

| Task | Reference |
|---|---|
| Survey a project; save decisions, map and continuation | [Survey](tools/survey.md) |
| Create/read/edit specifications; tags, CAS, migration | [Specifications](tools/specifications.md) |
| Run checks, import results, record evidence/review | [Verification](tools/verification.md) |
| Install/update/recover global installation | [Installation](tools/installation.md) |
| Find problems; save solutions/repeated observations | [Issues](tools/issues.md) |
| Check project, documents, registry and inventory exclusions | [Diagnostics](tools/diagnostics.md) |

[rules](../rules.md) owns authority, [procedures](../procedures/task.md) sequence, and [audit](audit.md) survey method. References own commands/formats. Templates only guide local adaptation. A separate Tools document per project is unnecessary; project commands belong to DEVELOPMENT.

Previous links remain compatible:

<a id="структурированные-спецификации"></a>

## Structured specifications
[Format and operations](tools/specifications.md#structured-specifications).

<a id="номера-версии-и-приёмка"></a>

### IDs, versions and acceptance
[Identifiers and decisions](tools/specifications.md#ids-versions-and-acceptance).

<a id="каталог-миграция-и-восстановление"></a>

### Catalog, migration and recovery
[Storage and recovery](tools/specifications.md#catalog-migration-and-recovery).

<a id="связи-и-теги"></a>

### Links and tags
[Navigation](tools/specifications.md#links-and-tags).

<a id="привязки-и-явный-запуск"></a>

### Check bindings and explicit execution
[Runners](tools/verification.md#check-bindings-and-explicit-execution).

<a id="короткий-цикл-редактирования"></a>

## Short editing cycle
[Working views](tools/specifications.md#short-editing-cycle).

<a id="предварительная-проверка-документов-v0-2-5"></a>

## Document preflight (v0-2-5)
[Bootstrap](tools/diagnostics.md#document-preflight-v0-2-5).

<a id="начальный-реестр-документов"></a>

## Initial document registry
[Registry](tools/diagnostics.md#initial-document-registry).

<a id="совместимость-проекта"></a>

## Project compatibility
[Diagnostic limits](tools/diagnostics.md#project-compatibility).

<a id="именованные-результаты-и-последовательная-запись"></a>

## Named results and sequential writes

Report retains measurements, status and exit codes. result contains keys occurring in exactly one measurements object: result.store_sha256, result.survey_sha256, result.value, result.change, result.catalog_path. Repeated keys appear in ambiguous_result_keys and are absent from result. Select their measurements by domain ID, not row number. Store/local/survey SHA refer to different objects and cannot substitute for each other. spec-init also returns root-relative directory/pointer_path.

PowerShell example for a prepared patch.json:

```powershell
$hgRead = & $hgExe survey-read --root "$PWD" --view editable
if ($LASTEXITCODE -ne 0) { throw 'survey-read failed' }
$hgState = $hgRead | ConvertFrom-Json
if ($hgState.status -ne 'passed' -or !$hgState.result.survey_sha256) { throw 'Missing survey state' }
$hgWrite = & $hgExe survey-edit --root "$PWD" --expected $hgState.result.survey_sha256 --input patch.json
if ($LASTEXITCODE -ne 0) { throw 'survey-edit failed; reread before retry' }
$hgNext = $hgWrite | ConvertFrom-Json
```

For Markdown, check successful survey-read --view markdown the same way and print $hgState.result.value. JSON is the source. Saving a derived file is optional; redirection needs a selected path and normal preservation rules.

ui-launcher --root PATH [--apply true] prepares portable UI launchers. Without apply, it only checks. See [installation](tools/installation.md#launch-ui-from-a-project).
