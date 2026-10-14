<a id="хранение-результатов-проверок"></a>

# Verification result storage

Routine reading: highgrade spec-read --root . --id HG-ID --view evidence. This projects existing records without another document/catalog format change. Open full reports for claim verification, diagnostics and review. Correct hashes do not prove authenticity.

<a id="запуск-и-сохранение"></a>

## Run and preserve

python scripts/verify.py tests creates a managed run. Original Nextest files remain in target/nextest/highgrade for freshness/trace. Current-attempt copies, including failure, enter run nextest/. After review, choose JUnit and necessary attempt/list/completion snapshots instead of duplicate run.log information. For trace, run --name scenarios -- python scripts/verify.py scenarios copies originals into nextest/. These are temporary copies, not another permanent set.

For browser tests from root: python scripts/evidence_store.py run --name ui -- npm --prefix ui test. Playwright saves JSON/attachments in run playwright/. Explicit HIGHGRADE_UI_REPORT remains supported. HIGHGRADE_UI_BASE_URL selects your Storybook on a busy port. After review, pass selected relative paths to finish, e.g. --file playwright/report.json; add needed images/diagnostics. Exit code alone neither selects evidence nor authorizes deletion.

python scripts/evidence_store.py run --name check -- COMMAND ARGS creates target/highgrade/runs/check-UUID, passes HIGHGRADE_RUN_DIR and captures stdout/stderr in run.log. It passes administrative HIGHGRADE_RUN_TOKEN. Rust/Playwright verify it against run.json before writing; HIGHGRADE_RUN_DIR alone is insufficient. Export metadata excludes the token. Commands run without shell in Unix process groups/Windows Job. A remaining background child leaves the run incomplete; Windows Job terminates children when owner exits. Commands must not deliberately escape groups/Job or hand files to external processes. Completion records command, OS and exit; failure stays failure. Before execution, maintain previously completed managed folders. Preserve interrupted/unexported results for review.

After original-result review:

```text
python scripts/evidence_store.py finish --run target/highgrade/runs/check-UUID --file run.log
```

If nothing needs keeping, finish --run PATH --discard creates no export; current links/preservation checks still block deletion. This is an explicit choice, not an exit-code inference.

Prefer a machine report over run.log when it contains required observations. Multiple --file values are allowed. Other files of that managed run are temporary. Finish saves/verifies the set, registers exact temp-folder contents and performs safe maintenance. Active owners, new/changed files, corrupt exports, catalog/doc links or unfinished recovery block removal. Unknown folders are not automatically registered/deleted. Age does not establish safety.

Export existing unmanaged results without removal:

```text
python scripts/evidence_store.py export --metadata .highgrade/local/run-summary.json --file target/nextest/highgrade/junit.xml
python scripts/evidence_store.py show --manifest docs/evidence/objects/HASH.json
```

Metadata states actual command/environment/outcome/time/observation/limits. File existence is not passed. Manifest lists selected sources, hashes, sizes and objects. Bytes are stored once by SHA-256 in docs/evidence/objects; repeats are idempotent. spec-evidence report may point to the original unchanged .blob. Manifest is not observation. Moving recorded evidence requires separate link/fingerprint checks; export changes neither catalog nor history.

<a id="обслуживание"></a>

## Maintenance

.gitattributes uses -diff for .blob/UI-result folders. Raw reports appear as artifacts; Markdown summaries/manifests remain text. This changes display, not bytes/hashes or source compression/removal. For targeted review, read originals or git diff --text. Export selected new reports to objects; do not extend exclusions to source/specs. Tools ignoring Git attributes may still show old volume.

python scripts/target-maintenance.py --sweep previews selection; --apply deletes after checking. run/finish maintains completed managed folders; after interruption, next run handles them. Active build/UI, links and recovery block apply with reasons. Unexported results remain until evidence is selected.

Do not remove Cargo cache after every test. Maintain temporary results first. For the existing 600 MiB overrun, explicitly select unnecessary profiles under BUILD. Keep current debug/tools/candidate while needed. --check states unresolved remainder, not a hard cap at any cost. Search starts in src/tests/scripts/kit or chosen scope; target/blobs stay out of ordinary context. Replace temp-file trees with a brief maintenance summary.
