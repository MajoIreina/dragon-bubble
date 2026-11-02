param([string]$GodotPath = "$PSScriptRoot/work/tools/godot/Godot_v4.5.1-stable_win64_console.exe")
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
New-Item -ItemType Directory -Path outputs -Force | Out-Null
& $GodotPath --headless --path $PSScriptRoot --export-pack 'Windows Desktop' 'outputs/LPP.pck'
if ($LASTEXITCODE -ne 0) { throw '资源包导出失败' }
$guiEngine = Join-Path (Split-Path -Parent $GodotPath) 'Godot_v4.5.1-stable_win64.exe'
if (!(Test-Path -LiteralPath $guiEngine)) { throw '未找到开发预览引擎，请使用 Godot 编辑器正常导出。' }
# The preview launcher is the engine binary. Reuse an identical launcher,
# including while the user has the desktop pet open; only the PCK changed.
if (!(Test-Path -LiteralPath outputs/LPP.exe) -or
    (Get-FileHash -LiteralPath $guiEngine).Hash -ne (Get-FileHash -LiteralPath outputs/LPP.exe).Hash) {
    Copy-Item -LiteralPath $guiEngine -Destination outputs/LPP.exe
}
Copy-Item -LiteralPath LICENSE-GODOT.txt -Destination outputs/LICENSE-GODOT.txt
Write-Output '开发预览版已保存至 outputs/LPP.exe 和 outputs/LPP.pck；两者须在同一目录。'
