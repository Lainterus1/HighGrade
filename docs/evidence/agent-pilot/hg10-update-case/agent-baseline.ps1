$ErrorActionPreference = 'Stop'
$base = 'D:\my_projects\MyCodex\target\update-pilot'
$case = Join-Path $base 'case'
$old = Join-Path $base 'highgrade-v0-2-13.exe'
$new = Join-Path $base 'highgrade-v0-2-14-exact.exe'
$good = Join-Path $base 'profile'
$fail = Join-Path $base 'profile-fail'
$records = @()

function Invoke-Recorded([string]$name, [string]$exe, [string[]]$arguments) {
    $started = (Get-Date).ToUniversalTime().ToString('o')
    $output = & $exe @arguments 2>&1
    $code = $LASTEXITCODE
    $output | Set-Content -LiteralPath (Join-Path $case "agent-$name.json") -Encoding utf8
    $script:records += [ordered]@{
        name=$name; exe=$exe; arguments=$arguments;
        started_at_utc=$started; ended_at_utc=(Get-Date).ToUniversalTime().ToString('o');
        exit_code=$code; report="case/agent-$name.json"
    }
}

Invoke-Recorded 'old-version' $old @('version')
Invoke-Recorded 'candidate-version' $new @('version')
Invoke-Recorded 'good-baseline-status' $old @('global-status','--profile',$good)
Invoke-Recorded 'fail-baseline-status' $old @('global-status','--profile',$fail)
$records | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-baseline-commands.json') -Encoding utf8
$records | ConvertTo-Json -Depth 5
