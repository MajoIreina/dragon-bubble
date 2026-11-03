# 龙泡泡 · macOS 移植版

这是独立的 Rust workspace，包含源码、模型、翻译和转换器，可直接把整个 `macos/` 文件夹复制到 Mac。它以项目现有 Bevy 0.17 / wgpu Vulkan 版本为基础，使用 **wgpu Metal 后端**；macOS 不原生支持 Vulkan，因此这个版本无需安装 Vulkan SDK 或 MoltenVK。

## 开发运行

支持 Apple Silicon 和 Intel，最低 macOS 11，显卡需要支持 Metal。安装 Xcode Command Line Tools（`xcode-select --install`）以及 Rust stable，版本至少 1.88。

```bash
cd macos
bash start.sh
bash start.sh --release
```

左键拖拽、抛掷和轻点互动；右键桌宠或点击菜单栏图标打开菜单，选择“设置”打开完整设置。关闭设置会保留桌宠；通过菜单“退出”结束程序。`.app` 采用菜单栏应用模式，不常驻 Dock。

## 构建与打包

```bash
bash build.sh                       # 当前 Mac 架构
rustup target add aarch64-apple-darwin x86_64-apple-darwin
bash build.sh --universal           # Apple Silicon + Intel 通用包
bash build.sh --target x86_64-apple-darwin
```

脚本运行格式、Clippy 和回归测试，生成 `outputs/<架构>/My Dragon Bubble.app` 和 ZIP。`--skip-tests` 可跳过回归测试。资源位于 `.app/Contents/Resources/`，支持 Finder 双击启动，不依赖当前目录。打包使用本机临时签名；对外发行时仍需要开发者签名和 Apple 公证。

运行数据保存在 `~/Library/Application Support/LPPDesktopPet/`，也可用 `LPP_DATA_DIR` 指定隔离目录。用户外观、配置和缓存与应用包分离。

Blender 格式的可选转换优先查找 `/Applications/Blender.app/Contents/MacOS/Blender`、用户的 `~/Applications/Blender.app` 和 Homebrew/PATH 安装；也可设置 `LPP_BLENDER_PATH`，或在设置中填写应用包内的可执行文件路径。

## 平台适配

- 透明桌宠使用 Metal 预乘 alpha 窗口；沿用 WGSL 材质、TAA、模型和软体逻辑。
- 使用 AppKit 获取全局鼠标按键及坐标，透明像素穿透，避免依赖窗口焦点启动拖拽；不使用事件监听钩子。
- 多屏使用 `NSScreen.visibleFrame`，避开菜单栏和 Dock，支持负坐标与屏幕热插拔。物理模拟及窗口定位统一使用逻辑点，兼容 Retina 和普通屏幕。
- 菜单栏和右键菜单共用原有动作；字体使用 macOS 自带的中日文字体，目录入口使用 Finder。
- 原项目位于上一级目录，本文件夹的源码和资源是独立快照，后续修改需分别同步。

## 验证

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
bash start.sh --release --smoke-test
```

冒烟测试输出到 `work/rust-smoke-data/screenshots/`。首次在 Mac 上验收时，请检查透明边缘、透明区域点击穿透、抓取/抛掷、缩放后继续显示、菜单栏与右键菜单、中日文字体，以及 Retina/普通多屏之间移动和 Dock 边界。独占全屏、不同 Spaces 和系统窗口层级仍受 macOS 窗口管理策略约束。

当前已在 Windows 主机通过 25 项单元/回归测试（另有 1 项需要 Blender 生成的 FBX 样本而跳过），并借助交叉 C 编译器通过 `aarch64-apple-darwin` 和 `x86_64-apple-darwin` 两个目标的全 workspace / all-targets Clippy 检查（`-D warnings`）。格式、脚本语法和应用清单检查也已通过。

2026-11-03 已通过 GitHub 的 Apple Silicon macOS 构建机完成原生 ARM64 Release `.app` 构建，25 项测试通过、1 项可选 Blender 测试跳过，Clippy、应用清单和 `codesign --verify --deep --strict` 检查通过。构建源码提交为 `5feb7dffc1382a66ef3b041e6b7b53fd74a47c15`，Actions run 为 `41559582411`，ARM64 job 为 `125666673196`。

实机测试包已下载至 `outputs/ci/dragon-bubble-aarch64-apple-darwin/My Dragon Bubble.zip`（39,273,869 字节）。已验证 ZIP 完整性、ARM64 Mach-O 架构、macOS 11.0 最低版本、可执行权限、资源及签名记录；动态库依赖均来自 macOS 系统。包的 SHA-256 为 `96f4b202c0b97f79dabe32ef637d7160c753e9ea6431720fe053b6993dcac7e5`，同目录的 `verification.json` 保存详细校验信息。

安装和日志采集见 [实机测试说明](TESTING.md)。图形界面、透明区域穿透和多屏交互尚待实机验收。工作流同时提供 Intel 构建；此次优先交付用户测试机所需的 Apple Silicon 包。
