# 龙泡泡 · My Dragon Bubble

> **罗德岛桌面终端 · 日常驻留记录**
>
> 连接已建立。博士，今天也给黍泡泡留一块歇脚的地方吧。

**龙泡泡**是以《明日方舟》中的黍为灵感制作的非官方同人桌宠。把一点炎国田间的悠闲搬进桌面：黍泡泡会沿着屏幕散步，站在窗口边缘歇脚，偶尔原地转上一圈，困了便伴着黑色的 Z 安静睡去。

博士处理文书的时候，她也有自己的小日常。轻轻点一下，看看这次是开心笑眼、惊讶，还是一个俏皮的单眼表情；提起来挪个位置，四只脚和尾巴也会跟着晃动。

**当前版本：v0.1.2 · Windows x64 · Rust / Vulkan**

[接黍泡泡回桌面](https://github.com/MajoIreina/dragon-bubble/releases/tag/v0.1.2) · [本次更新记录](docs/releases/v0.1.2.md) · [向工程部反馈问题](https://github.com/MajoIreina/dragon-bubble/issues)

## 终端接入 · 下载与启动

1. 前往 [Releases](https://github.com/MajoIreina/dragon-bubble/releases/tag/v0.1.2)，下载 `dragon-bubble-v0.1.2-windows-x64.zip`。
2. 完整解压，运行 `My Dragon Bubble.exe`，保留包内许可证及辅助资源。
3. 右键黍泡泡或系统托盘图标，即可打开设置，安排她的桌面日常。

运行环境为 **Windows x64**，显卡及驱动需要支持 **Vulkan**。升级时先从托盘退出旧版，再将新版解压到独立目录；已有设置继续保留。

**黍泡泡模型、贴图和外观清单已内嵌 EXE**，运行无需外置 `skins/shu/`。当前发行包仅保留黍泡泡，自定义外观可在设置中导入。

## 驻留须知 · 和黍泡泡相处

| 博士的操作 | 黍泡泡的反应 |
| --- | --- |
| 左键轻点 | 依次触发挤眼张嘴、开心笑眼、惊讶、俏皮单眼；连续点击可重新触发 |
| 按住左键拖动 | 跟着鼠标移动，四脚悬挂、身体轻晃；横向拖得越快，侧向程度越大 |
| 拖动停下，保持按住 | 慢慢转向博士，轻微左右摇晃 |
| 松开鼠标 | 保留松手时的朝向，在重力下落地；抛掷与连续回弹会带动尾巴下垂、回稳 |
| 悬停后滚动滚轮 | 向上放大、向下缩小，每格 5 个百分点，范围 20%～300% |
| 拖到普通窗口上方再松手 | 落在可见的窗口顶边，随窗口移动；窗口关闭、最小化或失去支撑后自然下落 |
| 右键角色 / 托盘图标 | 打开一键睡觉 / 唤醒、自由散步、置顶和完整设置等选项 |

拖拽朝向以面对博士为 **0°**，向左最低 **−90°**、向右最高 **+90°**，随速度平滑变化。支持多显示器，角色可以贴近屏幕左右边缘。

### 今天的桌面值班安排

- **田间小憩**：待机时以四脚中心原地转圈；困了慢慢闭眼、趴低呼吸，黑色 Z 低频浮起，由小变大后淡出。
- **一点小表情**：独立眨眼、眯眼、交替单眼，加上四种点击反馈，让每次互动多一点变化。
- **按博士的节奏来**：待机、走路、跑步、自动睡眠和转圈间隔均可调整；也可以一键让她睡个好觉。
- **终端设置**：微软雅黑界面，中 / 日 / 英语言，画质与性能调整、外观导入及动作适配。
- **工程部通讯**：内置 GitHub 更新检查、版本说明、下载与反馈入口，更新由博士自行下载。

窗口驻留支持普通顶层窗口的可见顶边。应用内部子面板，以及上方空间不足的最大化窗口，不作为落脚点。

操作细节见 [Windows 动画与交互](docs/windows-animation-update.md)、[性能设置](docs/performance.md) 和 [外观格式与动作适配](docs/skin-packs.md)。

运行数据保存在 `%APPDATA%/LPPDesktopPet/`，包括用户导入的 `skins/`、`preferences.json`、`appearance-overrides.json`、可选转换器配置和缓存。`LPP_DATA_DIR` 可指定隔离数据目录。

## 工程部档案 · 本地开发

当前默认开发版本为 **Rust Windows**，使用 Bevy 0.17 / wgpu Vulkan、egui、Win32 透明窗口和系统托盘，运行与构建不再依赖 Godot。

开发需要 Rust stable MSVC（至少 1.88）、Visual Studio C++ Build Tools，以及支持 Vulkan 的显卡驱动。

```powershell
./start.ps1             # 本地开发；也可双击 start.cmd
./start.ps1 -Release    # 本地优化构建
./build.ps1             # 测试并生成内嵌黍泡泡的发布版
```

正式包生成在 `outputs/rust/`。

源码仓库不存放角色模型。完整开发及发布需本机 `skins/shu/skin.json` 和 `shu.glb`，GLB 必须包含全部贴图与缓冲区。开发启动直接读取本地资源；发布脚本启用 `embedded-shu`，由 Bevy 内存资源读取模型，不向磁盘释放黍泡泡模型。`cargo build --release -p pet-desktop --features embedded-shu` 也可生成内嵌版本。EXE 包含模型数据，不构成加密或防提取措施。

从 GitHub 获取的纯源码可以独立验证：

```powershell
cargo test --locked --workspace
./build.ps1 -WithoutBundledModel
```

纯源码构建不带角色，可在设置中导入自己的外观。完整黍泡泡版本从 Releases 下载。模型源文件和独立模型不进入当前 Git 树；按维护者选择，既有历史提交、标签和旧发布保留。

### 物资归档 · 开发目录

| 路径 | 用途 |
| --- | --- |
| `crates/pet-core/` | 行为、运动、窗口表面、多屏、导入、配置、软体与外观格式 |
| `crates/pet-desktop/` | Windows 集成、Vulkan 渲染、动画、界面、内嵌模型和更新 |
| `assets/`、`i18n/`、`converters/` | 图标、翻译和可选 Blender 转换器 |
| `tests/`、`tools/` | 测试素材生成、翻译构建、公开文件校验和资源白名单 |
| `docs/` | 当前版本开发与使用文档 |
| `skins/shu/` | 本地黍泡泡运行 / 发布资源，Git 忽略 |
| `archives/` | 本地统一归档，Git 忽略；见 [归档说明](docs/archive.md) |
| `target/`、`work/`、`outputs/` | Cargo 缓存、当前临时验证和当前产物，Git 忽略 |

macOS 独立移植工作保留在本地 `macos/`，其工作流和源码由 `codex/macos-app-build-20261103` 分支单独维护。

### 出勤检查 · 验证与发布

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p pet-desktop --features embedded-shu -- --smoke-test
python tools/check_public_files.py
```

测试素材按需生成在临时目录或 `work/`，不依赖角色模型。可选 Blender FBX 验证需先运行 `tests/create_blender_fixtures.py`，再运行 `cargo test -p pet-core --test native_models -- --include-ignored`。

发布脚本仅打包 `tools/public-resources.txt` 明确列出的资源，绝不递归复制本地 `skins/`。提交前运行公开文件检查；发布 ZIP 可加 `--zip <文件>` 校验文件清单和完整性。正式发行可以包含内嵌模型的 EXE，但不能包含独立模型、Blender 工程、模型贴图或本地归档。
