$ErrorActionPreference='Stop'
$global:DragonBubbleUpdateHandoffSteps = [Collections.Generic.List[string]]::new()
function Wait-Process {
    param([int]$Id, $ErrorAction)
    if ($Id -ne 12345) { throw 'Wrong process ID passed to update handoff' }
    $global:DragonBubbleUpdateHandoffSteps.Add('wait')
}
function Start-Process {
    param($FilePath, $ArgumentList, $WindowStyle)
    if ($global:DragonBubbleUpdateHandoffSteps.Count -ne 1 -or $global:DragonBubbleUpdateHandoffSteps[0] -ne 'wait') { throw 'Installer launched before process exit' }
    if ($FilePath -ne 'C:/updates/测试安装.exe' -or $ArgumentList[-1] -ne '/DIR="C:/custom path/中文"' -or $WindowStyle -ne 'Hidden') { throw 'Update arguments or directory changed' }
    $global:DragonBubbleUpdateHandoffSteps.Add('launch')
}
& "$PSScriptRoot/run_update.ps1" -ParentProcessId 12345 -Installer 'C:/updates/测试安装.exe' -InstallDirectory 'C:/custom path/中文'
if ($global:DragonBubbleUpdateHandoffSteps.Count -ne 2) { throw 'Incomplete update handoff' }
Write-Output 'UPDATE HANDOFF PASS: parent exit awaited before installer launch; Unicode/space directory preserved.'
Remove-Variable -Name DragonBubbleUpdateHandoffSteps -Scope Global

