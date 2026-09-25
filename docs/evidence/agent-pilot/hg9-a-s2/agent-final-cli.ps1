$ErrorActionPreference = 'Stop'
$root = 'D:\my_projects\MyCodex\target\agent-pilot\s2'
$profile = 'D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile'
$cli = Join-Path $profile '.highgrade\global\releases\v0-2-14\highgrade.exe'
$evidence = Join-Path $root 'evidence\hg9-a'
$records = @()

foreach ($item in @(
    @{ name='doctor-final'; arguments=@('doctor','--root',$root) },
    @{ name='inspect-final'; arguments=@('inspect','--root',$root,'--registry','.highgrade/project/documents.json') }
)) {
    $started = (Get-Date).ToUniversalTime().ToString('o')
    $output = & $cli @($item.arguments) 2>&1
    $code = $LASTEXITCODE
    $output | Set-Content -LiteralPath (Join-Path $evidence "agent-$($item.name).json") -Encoding utf8
    $records += [ordered]@{
        name = $item.name
        command = "& `$cli " + ($item.arguments -join ' ')
        started_at_utc = $started
        ended_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        exit_code = $code
        report = "evidence/hg9-a/agent-$($item.name).json"
    }
}
$records | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $evidence 'agent-final-cli.json') -Encoding utf8
$records | ConvertTo-Json -Depth 4
