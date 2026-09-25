$ErrorActionPreference = 'Stop'
$base = 'D:\my_projects\MyCodex\target\update-pilot'
$case = Join-Path $base 'case'
$candidate = Join-Path $base 'highgrade-v0-2-14-exact.exe'
$profile = Join-Path $base 'profile'
$kit = Join-Path $base 'source-v0-2-14\kit'
$arguments = @('global-update','--profile',$profile,'--source',$kit,'--candidate-exe',$candidate)
$started = (Get-Date).ToUniversalTime().ToString('o')
$output = & $candidate @arguments 2>&1
$code = $LASTEXITCODE
$output | Set-Content -LiteralPath (Join-Path $case 'agent-good-preview.json') -Encoding utf8
$record = [ordered]@{name='good-preview';exe=$candidate;arguments=$arguments;started_at_utc=$started;ended_at_utc=(Get-Date).ToUniversalTime().ToString('o');exit_code=$code;report='case/agent-good-preview.json'}
$record | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'agent-good-preview-command.json') -Encoding utf8
$record | ConvertTo-Json -Depth 5
