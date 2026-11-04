param([string]$Url, [string]$Destination)
$ErrorActionPreference = 'Stop'
$uri = [Uri]$Url
if ($uri.Scheme -ne 'https' -or $uri.Host -ne 'github.com' -or $uri.AbsolutePath -notmatch '^/MajoIreina/dragon-bubble/releases/download/v[0-9]+\.[0-9]+\.[0-9]+/dragon-bubble-v[0-9]+\.[0-9]+\.[0-9]+-windows-x64-setup\.exe$') { throw 'Invalid update URL' }
$checksumsUrl = $Url.Substring(0, $Url.LastIndexOf('/') + 1) + 'SHA256SUMS.txt'
$sums = (Invoke-WebRequest -Uri $checksumsUrl -TimeoutSec 60 -UseBasicParsing).Content
if ($sums -is [byte[]]) { $sums = [Text.Encoding]::UTF8.GetString($sums) }
$name = $uri.Segments[-1]
$line = @($sums -split "`n" | Where-Object { $_.Trim() -match ('^[a-fA-F0-9]{64}\s+\*?' + [Regex]::Escape($name) + '$') })
if ($line.Count -ne 1) { throw 'Missing update checksum' }
Invoke-WebRequest -Uri $Url -OutFile $Destination -TimeoutSec 600 -UseBasicParsing
$expected = ($line[0].Trim() -split '\s+')[0]
if ((Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash -ne $expected) { Remove-Item -LiteralPath $Destination; throw 'Update checksum mismatch' }
