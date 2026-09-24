$ErrorActionPreference='Stop'
$repo='D:\my_projects\MyCodex'
$utf8=[Text.UTF8Encoding]::new($false)
$lab=Join-Path $env:TEMP ('highgrade-update-check-'+[guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($lab)|Out-Null
$base=Get-Content (Join-Path $repo 'target/portable-delivery/release-source.json') -Raw|ConvertFrom-Json
$exe=Join-Path $base.copy 'target/release/highgrade.exe'
$profile=Join-Path $lab 'profile'
[IO.Directory]::CreateDirectory($profile)|Out-Null
$candidate=Join-Path $lab 'candidate'
Copy-Item -LiteralPath (Join-Path $repo 'kit') -Destination $candidate -Recurse
function Manifest($root,$release) {
 $p=Join-Path $root 'manifest.json'; $m=Get-Content $p -Raw|ConvertFrom-Json; $m.release=$release
 foreach($prop in $m.files.psobject.Properties){$prop.Value=(Get-FileHash (Join-Path $root $prop.Name) -Algorithm SHA256).Hash.ToLower()}
 [IO.File]::WriteAllText($p,($m|ConvertTo-Json -Depth 8)+"`n",$utf8)
}
Manifest $candidate 'update-safety-test-next'
function Run($name,$arguments,$expected) {
 $out=& $exe @arguments; $code=$LASTEXITCODE
 [IO.File]::WriteAllText((Join-Path $lab ($name+'.json')),($out -join "`n")+"`n",$utf8)
 if($code -ne $expected){throw "$name exit=$code expected=$expected"}
 return ($out -join "`n"|ConvertFrom-Json)
}
$null=Run 'install' @('global-install','--profile',$profile,'--source',(Join-Path $base.copy 'kit'),'--candidate-exe',$exe) 0
[IO.File]::WriteAllText((Join-Path $profile 'foreign.txt'),'KEEP',$utf8)
$oldExe=Join-Path $lab 'previous.exe'; Copy-Item -LiteralPath $exe -Destination $oldExe
$preview=Run 'preview' @('global-update','--profile',$profile,'--source',$candidate,'--candidate-exe',$exe) 2
$null=Run 'apply' @('global-update','--profile',$profile,'--source',$candidate,'--candidate-exe',$exe,'--apply','true','--candidate-sha256',$preview.measurements[0].candidate_sha256) 0
$null=Run 'repeat' @('global-update','--profile',$profile,'--source',$candidate,'--candidate-exe',$exe) 1
$status=Run 'status-after-repeat' @('global-status','--profile',$profile) 0
$journal=Get-Content (Join-Path $profile '.highgrade/global/releases/update-safety-test-next/journal.json') -Raw|ConvertFrom-Json
$sameManifest=$journal.manifest_sha256 -eq (Get-FileHash (Join-Path $candidate 'manifest.json') -Algorithm SHA256).Hash.ToLower()
$sameExe=(Get-FileHash $exe).Hash -eq (Get-FileHash (Join-Path $profile '.highgrade/global/releases/update-safety-test-next/highgrade.exe')).Hash
if(!$sameManifest -or !$sameExe){throw 'Exact repeat mismatch'}
$conflict=Join-Path $lab 'conflict'; Copy-Item -LiteralPath $candidate -Destination $conflict -Recurse
[IO.File]::AppendAllText((Join-Path $conflict 'rules.md'),"`nTest changed content.`n",$utf8)
Manifest $conflict 'update-safety-test-next'
$null=Run 'same-id-different-content' @('global-update','--profile',$profile,'--source',$conflict,'--candidate-exe',$exe) 1
if($journal.manifest_sha256 -eq (Get-FileHash (Join-Path $conflict 'manifest.json')).Hash.ToLower()){throw 'Conflict not detected'}
$next=Join-Path $lab 'following'; Copy-Item -LiteralPath $candidate -Destination $next -Recurse
Manifest $next 'update-safety-test-following'
$preview=Run 'following-preview' @('global-update','--profile',$profile,'--source',$next,'--candidate-exe',$exe) 2
[IO.File]::AppendAllText((Join-Path $next 'rules.md'),"`nChanged after preview.`n",$utf8)
Manifest $next 'update-safety-test-following'
$null=Run 'stale-fingerprint' @('global-update','--profile',$profile,'--source',$next,'--candidate-exe',$exe,'--apply','true','--candidate-sha256',$preview.measurements[0].candidate_sha256) 1
$preview=Run 'failure-preview' @('global-update','--profile',$profile,'--source',$next,'--candidate-exe',$exe) 2
$blocked=Join-Path $profile '.highgrade/global/releases/update-safety-test-following/rules.md'
[IO.Directory]::CreateDirectory((Split-Path $blocked -Parent))|Out-Null
[IO.File]::WriteAllText($blocked,'FOREIGN BLOCKER',$utf8)
$null=Run 'stage-failure' @('global-update','--profile',$profile,'--source',$next,'--candidate-exe',$exe,'--apply','true','--candidate-sha256',$preview.measurements[0].candidate_sha256) 1
$out=& $oldExe global-status --profile $profile
if($LASTEXITCODE -ne 0){throw 'Previous CLI cannot verify status'}
$out|Set-Content (Join-Path $lab 'status-after-failure.json') -Encoding utf8
$final=$out -join "`n"|ConvertFrom-Json
if($final.measurements[0].release -ne 'update-safety-test-next'){throw 'Unexpected active release'}
if([IO.File]::ReadAllText($blocked) -ne 'FOREIGN BLOCKER'){throw 'Foreign blocker changed'}
if([IO.File]::ReadAllText((Join-Path $profile 'foreign.txt')) -ne 'KEEP'){throw 'Foreign file changed'}
@{lab=$lab;source_sha=$base.sha;candidate_manifest=(Get-FileHash (Join-Path $candidate 'manifest.json')).Hash.ToLower();status='passed';same_manifest=$sameManifest;same_exe=$sameExe;active=$final.measurements[0].release}|ConvertTo-Json|Set-Content (Join-Path $repo 'target/portable-delivery/update-safety-results.json') -Encoding utf8
Get-Content (Join-Path $repo 'target/portable-delivery/update-safety-results.json')
