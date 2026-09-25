$ErrorActionPreference = 'Stop'
$base = 'D:\my_projects\MyCodex\target\update-pilot'
$case = Join-Path $base 'case'
$candidate = Join-Path $base 'highgrade-v0-2-14-exact.exe'
$old = Join-Path $base 'highgrade-v0-2-13.exe'
$profile = Join-Path $base 'profile-fail'
$badKit = Join-Path $base 'bad-kit-v0-2-14'

$badStart = (Get-Date).ToUniversalTime().ToString('o')
$badOutput = & $candidate global-update --profile $profile --source $badKit --candidate-exe $candidate 2>&1
$badCode = $LASTEXITCODE
$badEnd = (Get-Date).ToUniversalTime().ToString('o')
$badOutput | Set-Content -LiteralPath (Join-Path $case 'agent-fail-preview.json') -Encoding utf8

# Immediately diagnose the existing installation with the saved old CLI.
$statusStart = (Get-Date).ToUniversalTime().ToString('o')
$statusOutput = & $old global-status --profile $profile 2>&1
$statusCode = $LASTEXITCODE
$statusEnd = (Get-Date).ToUniversalTime().ToString('o')
$statusOutput | Set-Content -LiteralPath (Join-Path $case 'agent-fail-status-after.json') -Encoding utf8

$records = @(
    [ordered]@{name='fail-preview';exe=$candidate;arguments=@('global-update','--profile',$profile,'--source',$badKit,'--candidate-exe',$candidate);started_at_utc=$badStart;ended_at_utc=$badEnd;exit_code=$badCode;report='case/agent-fail-preview.json'},
    [ordered]@{name='fail-status-after';exe=$old;arguments=@('global-status','--profile',$profile);started_at_utc=$statusStart;ended_at_utc=$statusEnd;exit_code=$statusCode;report='case/agent-fail-status-after.json'}
)
$records | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-fail-commands.json') -Encoding utf8
$records | ConvertTo-Json -Depth 5
