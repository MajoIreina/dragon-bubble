param([switch]$SkipTests, [switch]$WithoutBundledModel)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
& python tools/release.py check
if ($LASTEXITCODE -ne 0) { throw 'Version consistency check failed' }
$featureArgs = @()
if (-not $WithoutBundledModel) {
    if (-not (Test-Path -LiteralPath 'skins/shu/shu.glb')) {
        throw '缺少本地黍泡泡模型。纯源码验证可使用 -WithoutBundledModel；完整发布需本地 skins/shu 资源。'
    }
    $featureArgs = @('--features', 'embedded-shu')
}
if (-not $SkipTests) {
    & cargo test --locked --workspace @featureArgs
    if ($LASTEXITCODE -ne 0) { throw 'Rust 回归测试失败' }
}
& cargo build --locked --release -p pet-desktop @featureArgs
if ($LASTEXITCODE -ne 0) { throw 'Rust 编译失败' }
$packageName = if ($WithoutBundledModel) { 'source-check' } else { 'rust' }
$destination = Join-Path $PSScriptRoot "outputs/$packageName"
$staging = Join-Path $PSScriptRoot ('outputs/rust-stage-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging -Force | Out-Null
Copy-Item -LiteralPath 'target/release/dragon-bubble.exe' -Destination (Join-Path $staging 'My Dragon Bubble.exe') -Force
Copy-Item -LiteralPath 'LICENSE-RENDERING.txt' -Destination $staging
foreach ($line in Get-Content -LiteralPath 'tools/public-resources.txt') {
    $relative = $line.Trim()
    if (-not $relative -or $relative.StartsWith('#')) { continue }
    if ($relative -match '(^/|\\|:|(^|/)\.\.(/|$)|(^|/)shu(/|$))') {
        throw "Invalid public resource: $relative"
    }
    $resourceDestination = Join-Path $staging $relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $resourceDestination) -Force | Out-Null
    Copy-Item -LiteralPath $relative -Destination $resourceDestination -Force
}
if (Test-Path -LiteralPath $destination) {
    $previous = Join-Path $PSScriptRoot ('archives/builds/' + $packageName + '-' + [guid]::NewGuid().ToString('N'))
    # Both resolved paths must remain inside this workspace's output directory.
    $outputRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'outputs')) + [IO.Path]::DirectorySeparatorChar
    foreach ($path in @($destination, $staging)) {
        if (-not [IO.Path]::GetFullPath($path).StartsWith($outputRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Invalid package destination'
        }
    }
    $archiveRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'archives/builds')) + [IO.Path]::DirectorySeparatorChar
    if (-not [IO.Path]::GetFullPath($previous).StartsWith($archiveRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Invalid build archive destination'
    }
    New-Item -ItemType Directory -Path (Split-Path -Parent $previous) -Force | Out-Null
    Move-Item -LiteralPath $destination -Destination $previous
}
Move-Item -LiteralPath $staging -Destination $destination
Write-Output "Rust + Vulkan 版本已生成：$destination/My Dragon Bubble.exe；请保留旁边的资源目录。"
if (-not $WithoutBundledModel) { Write-Output '黍泡泡模型已内嵌 EXE；包内不提供独立角色模型。' }
if (-not $WithoutBundledModel) {
    $version = & python tools/release.py version
    $compiler = $env:INNO_COMPILER
    if (-not $compiler) { $compiler = 'C:/Program Files (x86)/Inno Setup 6/ISCC.exe' }
    if (-not (Test-Path -LiteralPath $compiler)) { throw '安装器编译器缺失：需要 Inno Setup 6.7+，或设置 INNO_COMPILER。正式构建必须同时生成安装器。' }
    if (-not (Test-Path 'installer/ChineseSimplified.isl')) {
        Invoke-WebRequest 'https://raw.githubusercontent.com/jrsoftware/issrc/main/Files/Languages/ChineseSimplified.isl' -OutFile installer/ChineseSimplified.isl
    }
    & $compiler "/DAppVersion=$version" installer/windows.iss
    if ($LASTEXITCODE -ne 0) { throw 'Windows 安装器编译失败' }
    $zip = Join-Path $PSScriptRoot "outputs/installers/dragon-bubble-v$version-windows-x64.zip"
    Compress-Archive -Path "$destination/*" -DestinationPath $zip -Force
    & python tools/check_public_files.py --zip $zip
    if ($LASTEXITCODE -ne 0) { throw 'ZIP validation failed' }
}
