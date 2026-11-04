$ErrorActionPreference = 'Stop'
$dir = Join-Path $PSScriptRoot '../work/update-download-test'
New-Item -ItemType Directory -Path $dir -Force | Out-Null
$fixture = Join-Path $dir 'fixture.exe'
[IO.File]::WriteAllText($fixture, 'update verification fixture')
$global:DragonBubbleTestHash = (Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash
$global:DragonBubbleTestAsset = 'dragon-bubble-v9.0.0-windows-x64-setup.exe'
function Invoke-WebRequest {
    param($Uri, $OutFile, $TimeoutSec, [switch]$UseBasicParsing)
    if ($OutFile) { Copy-Item -LiteralPath $fixture -Destination $OutFile -Force }
    else { @{ Content = "$global:DragonBubbleTestHash  $global:DragonBubbleTestAsset" } }
}
$destination = Join-Path $dir 'verified.exe'
$url = "https://github.com/MajoIreina/dragon-bubble/releases/download/v9.0.0/$global:DragonBubbleTestAsset"
& "$PSScriptRoot/download_update.ps1" -Url $url -Destination $destination
if (-not (Test-Path -LiteralPath $destination)) { throw 'Valid download rejected' }
$global:DragonBubbleTestHash = '0' * 64
$rejected = $false
try { & "$PSScriptRoot/download_update.ps1" -Url $url -Destination $destination } catch { $rejected = $true }
if (-not $rejected -or (Test-Path -LiteralPath $destination)) { throw 'Checksum mismatch accepted' }
$rejected = $false
try { & "$PSScriptRoot/download_update.ps1" -Url 'https://example.com/update.exe' -Destination $destination } catch { $rejected = $true }
if (-not $rejected) { throw 'Untrusted URL accepted' }
Write-Output 'Update verification: matching checksum accepted, mismatched checksum and unrelated host rejected.'

