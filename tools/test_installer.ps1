param([Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference = 'Stop'
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../work/installer-test'))
$destination = Join-Path $root ('custom path 中文-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $destination -Force | Out-Null
foreach ($attempt in 1..2) {
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', '/NOICONS', ('/DIR="' + $destination + '"'))
    $process = Start-Process -FilePath $Installer -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "Installation/upgrade failed: $($process.ExitCode)" }
    if (-not (Test-Path -LiteralPath (Join-Path $destination 'My Dragon Bubble.exe'))) { throw 'Custom install path ignored' }
    if (-not (Test-Path -LiteralPath (Join-Path $destination 'converters/model_to_glb.py'))) { throw 'Missing installed resources' }
}
$uninstall = Join-Path $destination 'unins000.exe'
$process = Start-Process -FilePath $uninstall -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -ne 0) { throw 'Uninstall failed' }
if (Test-Path -LiteralPath (Join-Path $destination 'My Dragon Bubble.exe')) { throw 'Application was not uninstalled' }
Write-Output 'Installer custom Unicode/space directory, upgrade and uninstall passed.'
