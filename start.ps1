param([switch]$Release, [switch]$SmokeTest)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$cargoArgs = @('run', '--locked', '-p', 'pet-desktop')
if ($Release) { $cargoArgs += '--release' }
if ($SmokeTest) { $cargoArgs += @('--', '--smoke-test') }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { throw "Rust 桌宠启动失败，退出码 $LASTEXITCODE" }
