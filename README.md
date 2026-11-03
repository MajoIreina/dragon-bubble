# 龙泡泡 · My Dragon Bubble

当前默认开发版本为 **Rust Windows 0.1.2**：Bevy 0.17 / wgpu Vulkan、egui、Win32 透明窗口和系统托盘。运行与构建不再依赖 Godot。

0.1.2 调整了落地尾巴回弹、四脚中心转圈，增加四种点击表情，支持悬停滚轮缩放、随拖动速度改变侧向角度、停住后转向正面，修复左右贴边，并简化睡眠特效；详见 [本次版本说明](docs/releases/v0.1.2.md)。

[下载 v0.1.2](https://github.com/MajoIreina/dragon-bubble/releases/tag/v0.1.2) · [问题反馈](https://github.com/MajoIreina/dragon-bubble/issues)

## 运行与构建

需要 Windows Vulkan 驱动、Rust stable MSVC（至少 1.88）和 Visual Studio C++ Build Tools。

```powershell
./start.ps1             # 本地开发；也可双击 start.cmd
./start.ps1 -Release    # 本地优化构建
./build.ps1             # 测试并生成内嵌黍泡泡的发布版
```

正式包位于 `outputs/rust/`。**黍泡泡模型、贴图和外观清单内嵌 EXE**，不需要外置 `skins/shu/`。其他示例角色已移除。保留包内的许可证和辅助资源；自定义模型仍可从设置中导入。

源码仓库不存放角色模型。完整开发及发布需本机 `skins/shu/skin.json` 和 `shu.glb`，GLB 必须包含全部贴图与缓冲区。开发启动直接读取本地资源；发布脚本启用 `embedded-shu`，由 Bevy 内存资源读取模型，不向磁盘释放黍泡泡模型。`cargo build --release -p pet-desktop --features embedded-shu` 也可生成内嵌版本。EXE 包含模型数据，不构成加密或防提取措施。

从 GitHub 获取的纯源码可以独立验证：

```powershell
cargo test --locked --workspace
./build.ps1 -WithoutBundledModel
```

纯源码构建不带角色，可在设置中导入自己的外观。完整黍泡泡版本从 Releases 下载。模型源文件和独立模型不进入当前 Git 树；按维护者选择，既有历史提交、标签和旧发布保留。

## 功能

左键拖动、抛掷和轻点互动，支持多显示器与 20%–300% 缩放；鼠标悬停在角色上时，向上滚动放大、向下滚动缩小，每格调整 5 个百分点。右键桌宠或系统托盘打开菜单，提供一键睡觉 / 唤醒、自由散步、置顶、完整设置和退出。

支持站在普通桌面窗口顶边、待机原地转圈、可调动作间隔、睡眠慢呼吸与黑色 Z 字、独立眨眼和单眼表情。界面使用微软雅黑。设置提供中 / 日 / 英语言、外观导入与动作适配、画质性能、GitHub 更新检查及项目链接。

详见 [Windows 动画与交互](docs/windows-animation-update.md)、[性能设置](docs/performance.md) 和 [外观格式与动作适配](docs/skin-packs.md)。

运行数据在 `%APPDATA%/LPPDesktopPet/`，包括用户导入的 `skins/`、`preferences.json`、`appearance-overrides.json`、可选转换器配置和缓存。`LPP_DATA_DIR` 可指定隔离数据目录。

## 开发目录

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

macOS 独立移植工作保留在本地 `macos/`，其工作流和源码由既有 `codex/macos-app-build-20261103` 分支维护；本次不合并到 Windows 主线。原有本地修改完整保留。

## 验证与发布约束

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p pet-desktop --features embedded-shu -- --smoke-test
python tools/check_public_files.py
```

测试素材按需生成在临时目录或 `work/`，不依赖角色模型。可选 Blender FBX 验证需先运行 `tests/create_blender_fixtures.py`，再运行 `cargo test -p pet-core --test native_models -- --include-ignored`。

发布脚本仅打包 `tools/public-resources.txt` 明确列出的资源，绝不递归复制本地 `skins/`。提交前运行公开文件检查；发布 ZIP 可加 `--zip <文件>` 校验文件清单和完整性。正式发行可以包含内嵌模型的 EXE，但不能包含独立模型、Blender 工程、模型贴图或本地归档。
