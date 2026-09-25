$ErrorActionPreference = 'Stop'
$root = 'D:\my_projects\MyCodex\target\agent-pilot\s2'
$profile = 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile'
$cli = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'
$evidence = Join-Path $root 'evidence\hg9-a'
$records = @()

function Invoke-Recorded([string]$name, [string[]]$arguments) {
    $started = (Get-Date).ToUniversalTime().ToString('o')
    $output = & $cli @arguments 2>&1
    $code = $LASTEXITCODE
    $rawPath = Join-Path $evidence "agent-$name.json"
    $output | Set-Content -LiteralPath $rawPath -Encoding utf8
    $script:records += [ordered]@{
        name = $name
        command = "& `$cli " + ($arguments -join ' ')
        started_at_utc = $started
        ended_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        exit_code = $code
        report = "evidence/hg9-a/agent-$name.json"
    }
}

Invoke-Recorded 'global-status' @('global-status', '--profile', $profile)
Invoke-Recorded 'bootstrap' @('inspect', '--bootstrap', '--root', $root)
Invoke-Recorded 'inventory' @('inventory', '--root', $root)
Invoke-Recorded 'doctor-before' @('doctor', '--root', $root)
$records | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $evidence 'agent-initial-commands.json') -Encoding utf8
$records | ConvertTo-Json -Depth 4
