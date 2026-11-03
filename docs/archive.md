# 开发目录归档

2026-11-03 整理后，根目录 Rust Windows 工作区是默认开发版本。`archives/` 是统一的本地资料库，整个目录被 Git 忽略；当前源码不再跟踪 Godot 工程或角色资产。

## 本地布局

| 路径（相对于 archives/） | 内容 |
| --- | --- |
| `cleanup-2026-11-03/legacy-godot/` | 原 Godot 场景、脚本、工程配置、旧测试 / 工具、引擎缓存和导入元数据 |
| `cleanup-2026-11-03/private-models/` | 黍泡泡 Blender 源工程、模型修订、贴图源文件与原建模说明 |
| `cleanup-2026-11-03/historical-materials/` | 原 archive/ 中的历史开发、诊断与参考资料 |
| `cleanup-2026-11-03/documentation/` | 历史文档与预览媒体 |
| `cleanup-2026-11-03/example-skins/` | 已移除的其他示例角色，仅本地留档 |
| `cleanup-2026-11-03/outputs/`、`work/` | 整理前的构建、预览、诊断和临时快照 |
| `cleanup-2026-11-03/previous-archives/` | 2026-11-02 已有的 Git bundle、预览包和清单 |
| `builds/` | 此后重新打包时自动收纳的旧构建 |

黍泡泡当前可用资源仍保留于根目录 `skins/shu/`，不进入 Git。`macos/` 独立移植目录及其已有修改保留在原位；其内的模型同样不加入 Windows 主线。Cargo 缓存与仍使用的开发工具保留在 `target/` 和 `work/tools/`。

`cleanup-2026-11-03/files.json` 记录本次批量归档文件的原路径、目标路径、字节数和 SHA-256；移动后已逐项校验。追加归档的示例外观清单保存在 `example-skins-manifest.json`。`local-shu-manifest.json` 记录仍在使用的本地黍泡泡资源。所有归档均不上传 GitHub。

## 恢复资料

`cleanup-2026-11-03/before-cleanup.bundle` 保存整理前全部本地分支、远端引用和标签；已通过 `git bundle verify`。`preexisting.patch` 保存整理前 README 修改，`refs-before.txt` 保存引用清单。

```powershell
git bundle verify ./archives/cleanup-2026-11-03/before-cleanup.bundle
git clone ./archives/cleanup-2026-11-03/before-cleanup.bundle ../dragon-bubble-restored
```

历史脚本可能依赖原目录结构或本机路径；需要重现旧 Godot 工程时应使用 bundle 恢复到单独目录。归档用于保存资料，不是当前运行入口。

## GitHub 边界

当前源码排除角色模型、纹理源文件、历史媒体和本地归档；Releases 允许内嵌黍泡泡模型的 EXE，不再包含外置模型或其他示例角色。既有 Git 历史、标签和旧发布按维护者选择保留，因此历史版本仍可能含原始模型；本次不进行历史重写或强制推送。
