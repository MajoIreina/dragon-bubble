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

在默认分支运行 `Release Windows and macOS`，选择版本递增类型。工作流准备版本提交和标签，再从该提交分别构建 Windows x64 与 macOS Apple Silicon。发布当前开发版本或重试时选择 `none`；缺少标签时自动创建，已有标签必须与当前提交一致。若验证失败后需要修改源码，保留原标签并使用下一补丁版本重新发布；不得移动已有标签来掩盖失败记录。

两个平台都须通过格式、Clippy、回归测试、内嵌模型构建和实际渲染冒烟检查。最终作业只在两端全部成功后创建 Release，并一次上传两个 ZIP、Windows 安装 EXE 与 SHA256 校验文件；任一失败均不创建或更新公开 Release。公开发布前必须已经具备两个完整产物，禁止先发 Windows 后补 macOS。当前 macOS 构建使用临时签名，保留现有最低系统版本与分发方式。

发布 ZIP 使用统一命名：

```
dragon-bubble-v<版本>-windows-x64.zip
dragon-bubble-v<版本>-windows-x64-setup.exe
dragon-bubble-v<版本>-macos-arm64.zip
SHA256SUMS.txt
```

角色资源仅在构建阶段从仓库已保留的模型历史恢复、校验并内嵌；不得提交独立模型、贴图或 Blender 工程。公开文件检查覆盖两个 workspace，并检查打包资源清单。新 macOS 源码已纳入主仓库，`macos/target/`、`outputs/`、`work/` 与 `skins/` 持续忽略。设置界面、菜单、开机自启实现及配置定义共用根 workspace 源码，平台集成及 Metal / Vulkan 渲染保留各自实现。

## Windows 安装与更新（自 v0.1.5 起强制执行）

每次正式编译必须同时保留便携 ZIP 和安装 EXE；缺少安装器视为构建失败。使用 Inno Setup 6.7+，本机可通过 INNO_COMPILER 指定 ISCC.exe。安装器使用微软雅黑、Windows 11 灰色卡片与蓝色强调色，支持系统深浅主题（安装器在启动时读取系统主题）。安装目录始终显示并允许选择，默认安装到当前用户 LocalAppData/Programs，无需管理员权限；提供开始菜单、可选桌面快捷方式和标准卸载入口，升级复用原目录，卸载保留 AppData 用户设置。

CI 必须验证带空格和中文的自选路径、覆盖升级、卸载，以及应用渲染和菜单回归。三份平台产物须全部准备完成后再公开同一 Release；SHA256SUMS.txt 必须同时覆盖 ZIP 与安装 EXE。

应用每 24 小时后台检测 GitHub 正式 Release，检测时间与结果跨重启保存；设置的“其他”与“检查更新”区域提醒新版本。Windows 更新必须由用户明确确认，下载同一 Release 下的 x64 安装器，校验 SHA256 后保存设置、退出并运行安装器，沿用当前目录并重启。网络或校验失败保留现有应用并允许重试。macOS 保持 GitHub 下载入口。设置、菜单和更新确认共用深浅主题，支持手动选择与运行时跟随系统。
