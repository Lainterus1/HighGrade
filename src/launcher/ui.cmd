@echo off
setlocal DisableDelayedExpansion
rem HighGrade project UI launcher, protocol 1. No saved project or release path.
set "HIGHGRADE_UI_ROOT=%~dp0"
set "HIGHGRADE_UI_NO_OPEN=false"
if "%~1"=="--no-open" set "HIGHGRADE_UI_NO_OPEN=true"
if not "%~1"=="" if not "%~1"=="--no-open" goto usage
if not "%~2"=="" goto usage
powershell.exe -NoLogo -NoProfile -Command "$ErrorActionPreference='Stop'; try { $profileRoot=$env:USERPROFILE; if(-not [IO.Path]::IsPathRooted($profileRoot)){throw 'USERPROFILE must be absolute'}; $base=Join-Path $profileRoot '.highgrade/global'; $pointer=Get-Content -LiteralPath (Join-Path $base 'active.json') -Raw | ConvertFrom-Json; $release=[string]$pointer.release; if($pointer.schema_version -ne 3 -or $release -cnotmatch '^[a-z0-9-]{1,64}$'){throw 'Invalid active release'}; $exe=Join-Path $base ('releases/'+$release+'/highgrade.exe'); if(-not (Test-Path -LiteralPath $exe -PathType Leaf)){throw 'Active High Grade executable is missing'}; & $exe global-status --profile $profileRoot | Out-Null; if($LASTEXITCODE -ne 0){throw 'Installation needs repair; run global-status'}; & $exe ui --root $env:HIGHGRADE_UI_ROOT --no-open $env:HIGHGRADE_UI_NO_OPEN; exit $LASTEXITCODE } catch { [Console]::Error.WriteLine('HighGrade UI: '+$_.Exception.Message); exit 1 }"
set "HIGHGRADE_UI_EXIT=%ERRORLEVEL%"
if not "%HIGHGRADE_UI_EXIT%"=="0" if "%HIGHGRADE_UI_NO_OPEN%"=="false" pause
exit /b %HIGHGRADE_UI_EXIT%
:usage
echo Usage: HighGrade UI.cmd [--no-open] 1>&2
exit /b 1
