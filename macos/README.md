# Dragon Bubble 0.1.2a for macOS

基于 Windows **0.1.2a**，源码基准提交 `73d6224`。`macos/` 是独立 Rust workspace；保留当前版本的动作、点击表情、拖拽/抛掷、滚轮缩放、窗口停靠、行为设置和事件循环帧调度。渲染使用 Bevy 0.17 / wgpu Metal。

## 运行和构建

需要支持 Metal 的 Mac，最低 macOS 11；开发环境需要 Rust stable（至少 1.88）和 Xcode Command Line Tools。

```bash
bash start.sh --release
bash build.sh
# 纯公开源码没有角色模型，可构建后从设置导入自己的外观：
bash build.sh --model-free
# 同时安装两个 Rust target 后可生成通用包：
bash build.sh --universal
```

发布构建默认启用 `embedded-shu`，要求本地 `skins/shu/shu.glb` 和 `skin.json`。该模型与 Windows 0.1.2a 完全一致，构建时校验并规范化后内嵌到可执行文件。打包只复制明确列出的公开资源，ZIP 中没有独立模型、贴图或示例外观。源码树也不存储这些私有资源。

产物：`outputs/aarch64-apple-darwin/My Dragon Bubble.app` 与 ZIP。使用临时签名，正式发行仍需开发者签名和 Apple 公证。安装及日志采集见 [TESTING.md](TESTING.md)。

## 启动排查与验证

- 屏幕布局查询和菜单栏创建在 winit 初始化之后于主线程执行；AppKit 系统使用主线程约束。
- Finder 启动优先读取包内 `Contents/Resources`，不依赖当前目录或外置 `skins/`。
- 自动保留启动、Rust panic、Metal/渲染错误到 `~/Library/Application Support/LPPDesktopPet/logs/latest.log`。
- 设置、缓存和截图写入用户数据目录；`LPP_DATA_DIR` 可指定隔离测试目录。冒烟测试不会修改已签名应用包。
- CI 分别执行格式/Clippy、回归测试、Release 构建与签名，再将最终 ZIP 解压到带空格路径，从 `/` 目录直接启动和通过 LaunchServices 启动，检查 Metal 渲染非透明桌宠截图。后一次启动覆盖超出屏幕的旧位置配置。
- `python3 tests/launch_app.py "outputs/aarch64-apple-darwin/My Dragon Bubble.zip"` 可在 Mac 上复验打包启动。

旧版 ZIP 和校验结果仅说明旧代码的编译/签名成功，不代表此次 0.1.2a 的启动验证结果；新构建结果另存于 `outputs/` 下。

## macOS 集成

使用 AppKit 处理全局鼠标、透明像素穿透、菜单栏、多屏与 Retina 逻辑坐标；Quartz 只读取窗口边界以支持窗口停靠，不采集屏幕图像或窗口标题。全屏、Spaces 和其他应用窗口类型受系统窗口管理策略约束。Blender 导入优先查找 `/Applications/Blender.app`、用户应用程序目录和 PATH，可通过 `LPP_BLENDER_PATH` 指定。
