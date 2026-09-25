$ErrorActionPreference = 'Stop'
$root = 'D:\my_projects\MyCodex\target\agent-pilot\s2'
$profile = 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile'
$cli = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'
$evidence = Join-Path $root 'evidence\hg9-a'
$records = @()

function Invoke-RecordedCli([string]$name, [string[]]$arguments) {
    $started = (Get-Date).ToUniversalTime().ToString('o')
    $output = & $cli @arguments 2>&1
    $code = $LASTEXITCODE
    $output | Set-Content -LiteralPath (Join-Path $evidence "agent-$name.json") -Encoding utf8
    $script:records += [ordered]@{
        name = $name
        command = "& `$cli " + ($arguments -join ' ')
        started_at_utc = $started
        ended_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        exit_code = $code
        report = "evidence/hg9-a/agent-$name.json"
    }
}

Invoke-RecordedCli 'spec-init' @('spec-init', '--root', $root)
Invoke-RecordedCli 'doctor-after' @('doctor', '--root', $root)
Invoke-RecordedCli 'inspect-after' @('inspect', '--root', $root, '--registry', '.highgrade/project/documents.json')

$sourceFiles = @('discount.py','quote.py','receipt.py','order_model.py','order_loader.py','order_report.py','orders.json','test_totals.py','test_order_report.py')
$before = [ordered]@{}
foreach ($path in $sourceFiles) { $before[$path] = (Get-FileHash -LiteralPath (Join-Path $root $path) -Algorithm SHA256).Hash.ToLowerInvariant() }
$started = (Get-Date).ToUniversalTime().ToString('o')
Push-Location $root
try {
    $output = & python -B -m unittest discover -v 2>&1
    $code = $LASTEXITCODE
} finally { Pop-Location }
$output | Set-Content -LiteralPath (Join-Path $evidence 'agent-tests.txt') -Encoding utf8
$after = [ordered]@{}
foreach ($path in $sourceFiles) { $after[$path] = (Get-FileHash -LiteralPath (Join-Path $root $path) -Algorithm SHA256).Hash.ToLowerInvariant() }
$testRecord = [ordered]@{
    name = 'tests'
    command = 'python -B -m unittest discover -v'
    started_at_utc = $started
    ended_at_utc = (Get-Date).ToUniversalTime().ToString('o')
    exit_code = $code
    report = 'evidence/hg9-a/agent-tests.txt'
    input_sha256_before = $before
    input_sha256_after = $after
}
$testRecord | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $evidence 'agent-tests.json') -Encoding utf8
$records += $testRecord
$records | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $evidence 'agent-check-commands.json') -Encoding utf8
$records | ConvertTo-Json -Depth 3
