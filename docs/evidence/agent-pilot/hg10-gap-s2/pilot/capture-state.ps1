param([Parameter(Mandatory = $true)][string]$Output)

$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$evidenceRoot = Join-Path $projectRoot 'evidence'
$rows = Get-ChildItem -LiteralPath $projectRoot -Force -Recurse -File |
    Where-Object { -not $_.FullName.StartsWith($evidenceRoot + '\', [System.StringComparison]::OrdinalIgnoreCase) } |
    Sort-Object FullName |
    ForEach-Object {
        [pscustomobject]@{
            path = $_.FullName.Substring($projectRoot.Length + 1).Replace('\', '/')
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
$rows | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Output -Encoding utf8
Write-Output ("count=" + @($rows).Count)
