# Rust 双平台发版规则

Windows 与 macOS 从本仓库的同一提交同步发版，使用同一版本号、同一 Git 标签和同一个 GitHub Release。不得单独发布其中一个平台，也不得继续从旧 macOS 移植分支独立发版。

## 版本管理

根目录 `Cargo.toml` 的 `workspace.package.version` 是唯一版本源。后续版本使用标准 SemVer `major.minor.patch`；历史 `v0.1.2a` 标签保留。

- 发布时由 `Release Windows and macOS` 工作流按所选 patch / minor / major 自动递增，默认 patch。
- `python tools/release.py bump patch` 可在本地准备相同的版本变更，无需手动编辑多个文件。
- 工具同步两个 workspace 的 Cargo 版本、两个 lockfile 和 macOS Info.plist。设置界面及更新检查直接读取 `CARGO_PKG_VERSION`；Windows 文件资源由 Cargo 包版本生成。
- `python tools/release.py check` 检查版本一致性。CI 和两个打包脚本都必须执行此检查；禁止只修改 UI 文案、包名或 plist 中的版本。
- 日常构建不递增版本。已发布标签不能覆盖；首次发布当前开发版本或重试失败的发布可选择 none，不能把单平台成功当成发行成功。

## 自动发布

在默认分支运行 `Release Windows and macOS`，选择版本递增类型。工作流准备版本提交和标签，再从该提交分别构建 Windows x64 与 macOS Apple Silicon。发布当前开发版本或重试时选择 `none`；缺少标签时自动创建，已有标签必须与当前提交一致。

两个平台都须通过格式、Clippy、回归测试、内嵌模型构建和实际渲染冒烟检查。最终作业只在两端全部成功后创建 Release，并一次上传两个 ZIP 与 SHA256 校验文件；任一失败均不创建或更新公开 Release。公开发布前必须已经具备两个完整产物，禁止先发 Windows 后补 macOS。当前 macOS 构建使用临时签名，保留现有最低系统版本与分发方式。

发布 ZIP 使用统一命名：

```
dragon-bubble-v<版本>-windows-x64.zip
dragon-bubble-v<版本>-macos-arm64.zip
SHA256SUMS.txt
```

角色资源仅在构建阶段从仓库已保留的模型历史恢复、校验并内嵌；不得提交独立模型、贴图或 Blender 工程。公开文件检查覆盖两个 workspace，并检查打包资源清单。新 macOS 源码已纳入主仓库，`macos/target/`、`outputs/`、`work/` 与 `skins/` 持续忽略。设置界面、菜单、开机自启实现及配置定义共用根 workspace 源码，平台集成及 Metal / Vulkan 渲染保留各自实现。
