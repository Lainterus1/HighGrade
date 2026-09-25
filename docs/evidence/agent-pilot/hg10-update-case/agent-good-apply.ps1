$ErrorActionPreference = 'Stop'
$base = 'D:\my_projects\MyCodex\target\update-pilot'
$case = Join-Path $base 'case'
$candidate = Join-Path $base 'highgrade-v0-2-14-exact.exe'
$old = Join-Path $base 'highgrade-v0-2-13.exe'
$profile = Join-Path $base 'profile'
$kit = Join-Path $base 'source-v0-2-14\kit'
$preview = Get-Content -LiteralPath (Join-Path $case 'agent-good-preview.json') -Raw | ConvertFrom-Json
$preflight = Get-Content -LiteralPath (Join-Path $case 'agent-preflight.json') -Raw | ConvertFrom-Json
if ($preview.status -ne 'unknown' -or $preview.findings[0].code -ne 'SemanticReviewRequired') { throw 'Unexpected preview status; apply stopped.' }
if ($preview.measurements[0].from -ne 'v0-2-13' -or $preview.measurements[0].candidate -ne 'v0-2-14') { throw 'Unexpected release transition; apply stopped.' }
if ($preview.measurements[0].manifest_sha256 -ne $preflight.manifest_sha256_actual) { throw 'Manifest changed since preview; apply stopped.' }
if (-not $preflight.all_manifest_files_match -or -not $preflight.all_stable_router_skills_match) { throw 'Preflight is incomplete; apply stopped.' }
$fingerprint = $preview.measurements[0].candidate_sha256
$arguments = @('global-update','--profile',$profile,'--source',$kit,'--candidate-exe',$candidate,'--apply','true','--candidate-sha256',$fingerprint)
$started = (Get-Date).ToUniversalTime().ToString('o')
$output = & $candidate @arguments 2>&1
$code = $LASTEXITCODE
$ended = (Get-Date).ToUniversalTime().ToString('o')
$output | Set-Content -LiteralPath (Join-Path $case 'agent-good-apply.json') -Encoding utf8
$applyReport = $output | Out-String | ConvertFrom-Json
$record = [ordered]@{name='good-apply';exe=$candidate;arguments=$arguments;started_at_utc=$started;ended_at_utc=$ended;exit_code=$code;report='case/agent-good-apply.json'}
$record | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-good-apply-command.json') -Encoding utf8

if ($code -ne 0 -or $applyReport.status -ne 'passed') {
    $statusExe = $old
    $statusName = 'agent-good-status-after-failure.json'
} else {
    $statusExe = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'
    $statusName = 'agent-good-status-after.json'
}
$statusStart = (Get-Date).ToUniversalTime().ToString('o')
$statusOutput = & $statusExe global-status --profile $profile 2>&1
$statusCode = $LASTEXITCODE
$statusOutput | Set-Content -LiteralPath (Join-Path $case $statusName) -Encoding utf8
$statusRecord = [ordered]@{name='good-status-after';exe=$statusExe;arguments=@('global-status','--profile',$profile);started_at_utc=$statusStart;ended_at_utc=(Get-Date).ToUniversalTime().ToString('o');exit_code=$statusCode;report=('case/'+$statusName)}
$statusRecord | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-good-status-command.json') -Encoding utf8
@($record,$statusRecord) | ConvertTo-Json -Depth 5
