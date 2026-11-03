# 0.1.2a macOS 实机测试

M 系列 Mac 使用 ARM64 安装包。把 ZIP 原样复制到 Mac，用 Finder 解压，将 `My Dragon Bubble.app` 拖入“应用程序”后双击启动。应用显示桌宠和菜单栏图标，不常驻 Dock。若临时签名被系统拦截，在“系统设置 → 隐私与安全性”中允许此次打开。

## 打开后验证

1. 桌宠显示、透明区域可点击后方应用；右键桌宠和菜单栏均能打开菜单。
2. 拖动、抛掷、左右朝向、滚轮缩放，以及停止后保持抓取状态正常。
3. 检查四种点击反应、眨眼、睡觉/唤醒和行为设置。
4. 开关设置、更换画质/抗锯齿/缩放后桌宠继续显示；退出重启后配置保留。
5. 多屏移动、Retina 缩放、Dock/菜单栏边界、窗口停靠与窗口移动跟随正常。

## 双击后退出或没有反应

应用在进入渲染前即开始记录日志。优先检查：

`~/Library/Application Support/LPPDesktopPet/logs/latest.log`

也可在终端以前台方式运行（环境变量只影响此次启动）：

```bash
LPP_LOG_TO_STDERR=1 RUST_BACKTRACE=1 RUST_LOG=info \
  "/Applications/My Dragon Bubble.app/Contents/MacOS/dragon-bubble" \
  > "$HOME/Desktop/dragon-bubble.log" 2>&1
```

需要排除旧配置时，在命令前增加 `LPP_DATA_DIR="$HOME/Desktop/dragon-bubble-test"`。原配置不受影响。反馈时请提供 macOS 版本、M 系列芯片型号、日志末尾和具体表现；原生日志缺失时，检查“控制台”中的应用崩溃报告。

可选启动探测：

```bash
LPP_DATA_DIR="$HOME/Desktop/dragon-bubble-probe" \
  "/Applications/My Dragon Bubble.app/Contents/MacOS/dragon-bubble" --startup-test
```

成功后自动退出，在上述目录生成 `startup-pass.json` 和 `startup-pet.png`，失败详情保存在 `logs/latest.log`。
