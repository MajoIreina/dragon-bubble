param([string]$GodotPath = "$PSScriptRoot/work/tools/godot/Godot_v4.5.1-stable_win64_console.exe")
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
New-Item -ItemType Directory -Path outputs -Force | Out-Null
& $GodotPath --headless --path $PSScriptRoot --script res://tools/build_windows_icon.gd
if ($LASTEXITCODE -ne 0) { throw 'Windows 图标生成失败' }
& $GodotPath --headless --path $PSScriptRoot --export-pack 'Windows Desktop' 'outputs/My Dragon Bubble.pck'
if ($LASTEXITCODE -ne 0) { throw '资源包导出失败' }
$guiEngine = Join-Path (Split-Path -Parent $GodotPath) 'Godot_v4.5.1-stable_win64.exe'
if (!(Test-Path -LiteralPath $guiEngine)) { throw '未找到开发预览引擎，请使用 Godot 编辑器正常导出。' }
# Prepare the branded launcher separately so an unchanged running EXE can be
# reused. Patching the source engine would also affect the editor and tools.
$launcherStage = Join-Path $PSScriptRoot 'work/windows-launcher'
New-Item -ItemType Directory -Path $launcherStage -Force | Out-Null
$brandedEngine = Join-Path $launcherStage 'My Dragon Bubble.exe'
Copy-Item -LiteralPath $guiEngine -Destination $brandedEngine
& "$PSScriptRoot/tools/set_windows_icon.ps1" -Executable $brandedEngine -IconPath "$PSScriptRoot/assets/tray.ico"
if (!(Test-Path -LiteralPath 'outputs/My Dragon Bubble.exe') -or
    (Get-FileHash -LiteralPath $brandedEngine).Hash -ne (Get-FileHash -LiteralPath 'outputs/My Dragon Bubble.exe').Hash) {
    Copy-Item -LiteralPath $brandedEngine -Destination 'outputs/My Dragon Bubble.exe'
    & "$PSScriptRoot/tools/refresh_windows_icon.ps1" -Executable 'outputs/My Dragon Bubble.exe'
}
Copy-Item -LiteralPath LICENSE-GODOT.txt -Destination outputs/LICENSE-GODOT.txt
Write-Output '开发预览版已保存至 outputs/My Dragon Bubble.exe 和 outputs/My Dragon Bubble.pck；两者须在同一目录。'
