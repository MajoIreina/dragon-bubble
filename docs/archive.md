# 项目归档与恢复

归档日期：2026-11-02（Asia/Hong_Kong）。

## 收录范围

Git 收录程序、场景、外观、翻译、图标、测试、构建 / 建模工具、文档、当前 Blender 源工程、原始旧模型和 `source_models/shu-plush/revisions/` 中的历史模型。Blender 的 `.blend1` 自动备份也作为本次初始快照保留。

原有目录不移动，以保留脚本和 Godot 资源引用。临时工作区中有保留价值的资料复制整理到 `archive/`；预览图和动画复制到 `docs/media/`。这些副本是历史材料，可能引用归档前的本机绝对路径，不作为当前运行入口。

| 目录 | 说明 |
| --- | --- |
| `archive/development/` | `work/before-*` 与 `physics-before-settling` 历史代码和对照 |
| `archive/references/` | 建模参考图、细节参考与角饰 / 脚尾修正参考 |
| `archive/diagnostics/` | 工作区的一次性诊断脚本、JSON 报告与检查截图 |
| `docs/media/` | `outputs/` 中的 PNG / GIF 预览快照，排除 Godot 缓存元数据 |

`archive/` 和 `docs/media/` 放置 `.gdignore`，避免 Godot 将历史脚本和文档图片导入运行工程。

## 不进入 Git 的本机文件

- `.godot/`：Godot 可重建缓存。
- `work/`：Godot / Blender / GitHub CLI、本地测试样本、逐帧渲染、日志、临时文件；原文件仍保留。
- `outputs/`：生成的 EXE、PCK、预览图和旧的 `LPP.exe` / `LPP.pck`；原文件仍保留。
- `archives/`：本次本地归档产物。
- Python 缓存、编辑器配置、临时文件、日志和本地凭据。

## 从 Git 恢复

```powershell
git clone <GitHub 仓库地址> MyDragonBubble
cd MyDragonBubble
./start.ps1 -GodotPath 'C:/Tools/Godot/Godot_v4.5.1-stable_win64.exe' -Editor
```

等待首次资源导入完成后运行主场景。亦可关闭编辑器后去掉 `-Editor` 启动。构建开发预览使用 `./build.ps1 -GodotPath 'Godot 控制台版路径'`，同一目录需有对应 GUI 引擎；具体版本与命令见项目首页和模型说明。

模型、GLB、纹理使用普通 Git 保存，本次各个文件均低于 GitHub 100 MiB 单文件限制。

## 本地离线备份

`archives/dragon-bubble-2026-11-02.bundle` 保存本次 Git 提交与归档标签，可验证和恢复：

```powershell
git bundle verify ./archives/dragon-bubble-2026-11-02.bundle
git clone ./archives/dragon-bubble-2026-11-02.bundle ../MyDragonBubble-restored
```

`archives/source-manifest-2026-11-02.json` 记录归档文件的路径、原始字节数和 SHA-256。`archives/manifest-sha256.txt` 记录清单本身的校验值。Git 的文本换行规范化可能改变其他平台检出的原始字节，跨平台时以 Git 提交校验为准。

`archives/My-Dragon-Bubble-Windows-preview-2026-11-02.zip` 收录现有开发预览程序、配套 PCK 与 Godot 许可证；这是开发预览快照，版本说明见 README。EXE 与 PCK 必须一起解压使用。

项目自身未新增开源许可证；`LICENSE-GODOT.txt` 仅说明 Godot 引擎的许可，不代表项目代码、角色、参考照片或其他素材已授权开源使用。

## 归档验证

本次从 Git 暂存区导出干净副本，使用本机 Godot 4.5.1 完成首次无界面资源导入，并执行现有 9 组测试：核心、运行时模型、运动、多屏桌面、性能、翻译、软体、步态、转身 / 待机。合计 914 项检查，0 失败。日志和结果位于本机 `archives/validation/`。这次验证不包含图形界面交互或新机器上的显示兼容性验证。
