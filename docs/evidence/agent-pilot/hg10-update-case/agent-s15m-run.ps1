$ErrorActionPreference = 'Stop'
$base = 'D:\my_projects\MyCodex\target\update-pilot'
$case = Join-Path $base 'case'
$profile = Join-Path $base 'profile'
$candidate = Join-Path $base 'highgrade-v0-2-14-exact.exe'
$syntheticKit = Join-Path $base 'same-id-different-kit-v0-2-14'
$installed = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'
$journalPath = Join-Path $profile '.highgrade\global\releases\v0-2-14\journal.json'
$package = Get-Content -LiteralPath (Join-Path $case 'agent-s15m-package.json') -Raw | ConvertFrom-Json
if (-not $package.all_manifest_files_match -or $package.source_manifest_sha256_before -ne $package.source_manifest_sha256_after) { throw 'Package preflight failed; preview stopped.' }
$arguments = @('global-update','--profile',$profile,'--source',$syntheticKit,'--candidate-exe',$candidate)
$previewStart = (Get-Date).ToUniversalTime().ToString('o')
$previewOutput = & $candidate @arguments 2>&1
$previewCode = $LASTEXITCODE
$previewOutput | Set-Content -LiteralPath (Join-Path $case 'agent-s15m-preview.json') -Encoding utf8
$previewEnd = (Get-Date).ToUniversalTime().ToString('o')

# GlobalSameRelease is still an error; immediately establish active state.
$statusStart = (Get-Date).ToUniversalTime().ToString('o')
$statusOutput = & $installed global-status --profile $profile 2>&1
$statusCode = $LASTEXITCODE
$statusOutput | Set-Content -LiteralPath (Join-Path $case 'agent-s15m-status-after.json') -Encoding utf8
$statusEnd = (Get-Date).ToUniversalTime().ToString('o')

$records = @(
    [ordered]@{name='synthetic-same-id-preview';exe=$candidate;arguments=$arguments;started_at_utc=$previewStart;ended_at_utc=$previewEnd;exit_code=$previewCode;report='case/agent-s15m-preview.json'},
    [ordered]@{name='status-after-preview';exe=$installed;arguments=@('global-status','--profile',$profile);started_at_utc=$statusStart;ended_at_utc=$statusEnd;exit_code=$statusCode;report='case/agent-s15m-status-after.json'}
)
$records | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-s15m-commands.json') -Encoding utf8
$preview = $previewOutput | Out-String | ConvertFrom-Json
$status = $statusOutput | Out-String | ConvertFrom-Json
if ($previewCode -ne 1 -or $preview.findings[0].message -ne 'GlobalSameRelease') { throw 'Unexpected preview result; no apply was attempted.' }
if ($statusCode -ne 0 -or $status.status -ne 'passed' -or $status.measurements[0].release -ne 'v0-2-14') { throw 'Active profile not confirmed; no apply was attempted.' }
$journal = Get-Content -LiteralPath $journalPath -Raw | ConvertFrom-Json
$candidateManifestSha = (Get-FileHash -LiteralPath (Join-Path $syntheticKit 'manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant()
$candidateCliSha = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
$installedCliSha = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToLowerInvariant()
$journalCliSha = $journal.files.'.highgrade/global/releases/v0-2-14/highgrade.exe'
$decision = [ordered]@{
    captured_at_utc=(Get-Date).ToUniversalTime().ToString('o')
    preview_error=$preview.findings[0].message
    active_status=$status.status
    active_release=$status.measurements[0].release
    synthetic_candidate_manifest_sha256=$candidateManifestSha
    installed_journal_manifest_sha256=$journal.manifest_sha256
    manifest_match=($candidateManifestSha -eq $journal.manifest_sha256)
    candidate_cli_sha256=$candidateCliSha
    installed_cli_sha256=$installedCliSha
    installed_journal_cli_sha256=$journalCliSha
    cli_match=($candidateCliSha -eq $installedCliSha -and $installedCliSha -eq $journalCliSha)
    decision='same release ID, different manifest; immutable release ID conflict; do not claim already installed and do not apply'
    apply_invoked=$false
}
$decision | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-s15m-decision.json') -Encoding utf8
$decision | ConvertTo-Json -Depth 5
