# macOS 实机测试

## 选择和启动

- Apple Silicon（M 系列）：选择 `dragon-bubble-aarch64-apple-darwin/My Dragon Bubble.zip`。
- Intel Mac：选择 `dragon-bubble-x86_64-apple-darwin/My Dragon Bubble.zip`。

CI 下载的包保存在本项目 `macos/outputs/ci/` 下。将 ZIP 原样传到 Mac，再用 Finder 解压，将 `My Dragon Bubble.app` 拖到“应用程序”，双击启动。不要在 Windows 解压后重新压缩，以免丢失可执行权限和 macOS 包信息。

需要 macOS 11 或更新版本，以及支持 Metal 的显卡。运行安装包无需 Rust、Xcode、Vulkan SDK 或 MoltenVK。此测试包使用临时签名，未经 Apple 公证；若系统阻止启动，在“系统设置 → 隐私与安全性”中允许此次打开。

应用启动后显示桌宠和菜单栏图标，不常驻 Dock。右键桌宠或点击菜单栏图标打开设置/退出菜单。

## 验收操作

1. 确认模型出现、边缘透明，模型周围透明区域可点击到后方应用。
2. 左键拖动、松手抛掷、轻点互动，观察落地和待机动画。
3. 打开设置，更换外观，调整缩放、抗锯齿和性能选项，确认桌宠持续显示。
4. 切换中文、日文和英文，检查字体和设置窗口。
5. 有多屏时在 Retina/普通屏之间拖动，检查缩放、屏幕边界和 Dock 避让。
6. 关闭设置后确认桌宠继续运行，再通过菜单退出，重启后确认配置保留。

## 启动失败或渲染异常时采集日志

先退出已运行的桌宠，在终端执行：

```bash
RUST_BACKTRACE=1 RUST_LOG=info \
  "/Applications/My Dragon Bubble.app/Contents/MacOS/dragon-bubble" \
  > "$HOME/Desktop/dragon-bubble.log" 2>&1
```

复现问题并退出应用后，桌面上的 `dragon-bubble.log` 包含此次启动日志。反馈时同时提供芯片类型、macOS 版本、具体操作和截图。

正常运行数据位于 `~/Library/Application Support/LPPDesktopPet/`。需要隔离测试数据时，可在上述启动命令前添加 `LPP_DATA_DIR="$HOME/Desktop/dragon-bubble-test"`。

内置 `--smoke-test` 目前面向源码目录运行，参见 README 的开发验证命令；实机安装包请按以上操作验收。
