param([string]$Executable = 'outputs/rust/My Dragon Bubble.exe', [string]$Output = 'work/memory-check')
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Output | Out-Null
$folder = (Resolve-Path -LiteralPath $Output).Path
$process = Start-Process -FilePath (Resolve-Path -LiteralPath $Executable).Path -ArgumentList '--resource-test' -PassThru -WindowStyle Hidden -RedirectStandardOutput "$folder/run.log" -RedirectStandardError "$folder/error.log"
$samples = [Collections.Generic.List[object]]::new()
$timer = [Diagnostics.Stopwatch]::StartNew()
try {
    while (-not $process.HasExited -and $timer.Elapsed.TotalSeconds -lt 250) {
        $process.Refresh()
        $samples.Add([pscustomobject]@{ seconds=[Math]::Round($timer.Elapsed.TotalSeconds,1); privateBytes=$process.PrivateMemorySize64; workingSet=$process.WorkingSet64; handles=$process.HandleCount })
        Start-Sleep -Seconds 1
    }
    if (-not $process.HasExited) { Stop-Process -Id $process.Id; throw 'Memory regression timed out' }
    $process.WaitForExit()
    $samples | Export-Csv -LiteralPath "$folder/process.csv" -NoTypeInformation
    if ($process.ExitCode -ne 0 -or -not (Select-String -LiteralPath "$folder/run.log" -Pattern 'RESOURCE PASS:' -Quiet)) { throw "Resource stress failed; inspect $folder/run.log and error.log" }
    # Compare warmed samples rather than startup allocations. Repeated quality
    # changes intentionally warm a finite set of GPU pipelines and textures.
    $early = @($samples | Where-Object { $_.seconds -ge 45 -and $_.seconds -le 65 })
    $late = @($samples | Select-Object -Last 20)
    if ($early.Count -lt 10 -or $late.Count -lt 10) { throw 'Insufficient memory samples' }
    $baseline = ($early | Measure-Object privateBytes -Average).Average
    $final = ($late | Measure-Object privateBytes -Average).Average
    $growth = $final - $baseline
    $report = [ordered]@{ baselinePrivateMiB=[Math]::Round($baseline/1MB,2); finalPrivateMiB=[Math]::Round($final/1MB,2); growthMiB=[Math]::Round($growth/1MB,2); peakWorkingSetMiB=[Math]::Round(($samples | Measure-Object workingSet -Maximum).Maximum/1MB,2); initialHandles=$early[-1].handles; finalHandles=$late[-1].handles; samples=$samples.Count }
    $report | ConvertTo-Json | Set-Content -LiteralPath "$folder/result.json" -Encoding utf8
    $report | ConvertTo-Json | Write-Output
    $resources = @(Get-Content -LiteralPath "$folder/run.log" | Where-Object { $_.StartsWith('RESOURCE SAMPLE: ') } | ForEach-Object { $_.Substring(17) | ConvertFrom-Json })
    if ($resources.Count -ne 8 -or $resources[-1].entities -gt $resources[1].entities+8) { throw 'Entities accumulated during repeated preview/model changes' }
    if ($growth -gt 48MB) { throw 'Private memory continued growing after warmup (>48 MiB)' }
    if ($late[-1].handles -gt $early[-1].handles+20) { throw 'Native handles continued growing' }
    Write-Output 'PROCESS MEMORY PASS: warmed private bytes and native handles remain bounded.'
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id }
}
