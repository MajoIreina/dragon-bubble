param([string]$GodotPath = "", [switch]$Editor)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($GodotPath)) {
    $localEngine = Join-Path $PSScriptRoot 'work/tools/godot/Godot_v4.5.1-stable_win64.exe'
    if (Test-Path -LiteralPath $localEngine) { $GodotPath = $localEngine }
    else {
        $engineCommand = Get-Command godot, godot4 -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($null -eq $engineCommand) { throw '请安装 Godot 4.5.1+，或传入 -GodotPath 指定引擎路径。' }
        $GodotPath = $engineCommand.Source
    }
}
if ($Editor) { & $GodotPath --editor --path $PSScriptRoot }
else { & $GodotPath --path $PSScriptRoot }
