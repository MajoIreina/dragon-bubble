param(
    [Parameter(Mandatory=$true)][int]$ParentProcessId,
    [Parameter(Mandatory=$true)][string]$Installer,
    [Parameter(Mandatory=$true)][string]$InstallDirectory
)
$ErrorActionPreference = 'Stop'
# The original process holds the same mutex checked by Inno Setup. Wait until
# its renderer and lock are gone, including when it exits before this helper starts.
Wait-Process -Id $ParentProcessId -ErrorAction SilentlyContinue
$arguments = @('/SILENT', '/NORESTART', '/UPDATE', ('/DIR="' + $InstallDirectory + '"'))
Start-Process -FilePath $Installer -ArgumentList $arguments -WindowStyle Hidden | Out-Null
