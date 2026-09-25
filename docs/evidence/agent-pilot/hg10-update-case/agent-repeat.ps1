$ErrorActionPreference = 'Stop'
$base = 'D:\my_projects\MyCodex\target\update-pilot'
$case = Join-Path $base 'case'
$candidate = Join-Path $base 'highgrade-v0-2-14-exact.exe'
$profile = Join-Path $base 'profile'
$kit = Join-Path $base 'source-v0-2-14\kit'
$installed = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'
$journalPath = Join-Path $profile '.highgrade\global\releases\v0-2-14\journal.json'
$arguments = @('global-update','--profile',$profile,'--source',$kit,'--candidate-exe',$candidate)
$started = (Get-Date).ToUniversalTime().ToString('o')
$output = & $candidate @arguments 2>&1
$code = $LASTEXITCODE
$output | Set-Content -LiteralPath (Join-Path $case 'agent-repeat-preview.json') -Encoding utf8
$preview = $output | Out-String | ConvertFrom-Json
$record = [ordered]@{name='repeat-preview';exe=$candidate;arguments=$arguments;started_at_utc=$started;ended_at_utc=(Get-Date).ToUniversalTime().ToString('o');exit_code=$code;report='case/agent-repeat-preview.json'}
$record | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-repeat-command.json') -Encoding utf8
if ($code -ne 1 -or $preview.findings[0].message -notmatch 'GlobalSameRelease') { throw 'Unexpected repeat result; comparison stopped.' }

$statusStart = (Get-Date).ToUniversalTime().ToString('o')
$statusOutput = & $installed global-status --profile $profile 2>&1
$statusCode = $LASTEXITCODE
$statusOutput | Set-Content -LiteralPath (Join-Path $case 'agent-repeat-status.json') -Encoding utf8
$status = $statusOutput | Out-String | ConvertFrom-Json
$statusRecord = [ordered]@{name='repeat-status';exe=$installed;arguments=@('global-status','--profile',$profile);started_at_utc=$statusStart;ended_at_utc=(Get-Date).ToUniversalTime().ToString('o');exit_code=$statusCode;report='case/agent-repeat-status.json'}
$statusRecord | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-repeat-status-command.json') -Encoding utf8
if ($statusCode -ne 0 -or $status.status -ne 'passed' -or $status.measurements[0].release -ne 'v0-2-14') { throw 'Installed status is not confirmed; comparison stopped.' }

$journal = Get-Content -LiteralPath $journalPath -Raw | ConvertFrom-Json
$candidateManifest = (Get-FileHash -LiteralPath (Join-Path $kit 'manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant()
$candidateCli = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
$installedCli = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToLowerInvariant()
$installedJournalCli = $journal.files.'.highgrade/global/releases/v0-2-14/highgrade.exe'
$comparison = [ordered]@{
    captured_at_utc=(Get-Date).ToUniversalTime().ToString('o')
    repeat_error='GlobalSameRelease'
    status='passed'
    active_release=$status.measurements[0].release
    candidate_manifest_sha256=$candidateManifest
    installed_journal_manifest_sha256=$journal.manifest_sha256
    manifest_match=($candidateManifest -eq $journal.manifest_sha256)
    candidate_cli_sha256=$candidateCli
    installed_cli_sha256=$installedCli
    installed_journal_cli_sha256=$installedJournalCli
    cli_match=($candidateCli -eq $installedCli -and $installedCli -eq $installedJournalCli)
    apply_invoked=$false
}
$comparison | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-repeat-comparison.json') -Encoding utf8
$comparison | ConvertTo-Json -Depth 5
