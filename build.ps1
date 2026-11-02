param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
if (-not $SkipTests) {
    & cargo test --locked --workspace
    if ($LASTEXITCODE -ne 0) { throw 'Rust 回归测试失败' }
}
& cargo build --locked --release -p pet-desktop
if ($LASTEXITCODE -ne 0) { throw 'Rust 编译失败' }
$destination = Join-Path $PSScriptRoot 'outputs/rust'
$staging = Join-Path $PSScriptRoot ('outputs/rust-stage-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging -Force | Out-Null
Copy-Item -LiteralPath 'target/release/dragon-bubble.exe' -Destination (Join-Path $staging 'My Dragon Bubble.exe') -Force
Copy-Item -LiteralPath 'LICENSE-RENDERING.txt' -Destination $staging
foreach ($folder in @('assets', 'skins', 'i18n', 'converters')) {
    $resourceDestination = Join-Path $staging $folder
    New-Item -ItemType Directory -Path $resourceDestination -Force | Out-Null
    Get-ChildItem -LiteralPath $folder | Copy-Item -Destination $resourceDestination -Recurse -Force
}
if (Test-Path -LiteralPath $destination) {
    $previous = Join-Path $PSScriptRoot ('outputs/rust-previous-' + [guid]::NewGuid().ToString('N'))
    # Both resolved paths must remain inside this workspace's output directory.
    $outputRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'outputs')) + [IO.Path]::DirectorySeparatorChar
    foreach ($path in @($destination, $previous, $staging)) {
        if (-not [IO.Path]::GetFullPath($path).StartsWith($outputRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Invalid package destination'
        }
    }
    Move-Item -LiteralPath $destination -Destination $previous
}
Move-Item -LiteralPath $staging -Destination $destination
Write-Output "Rust + Vulkan 版本已生成：$destination/My Dragon Bubble.exe；请保留旁边的资源目录。"
