# LPP 桌宠

面向 Windows 的 Godot 4.5 桌宠框架，角色外观与桌面功能、行为状态分开。所有项目文件均位于 `L:\LPP`。

## 启动

- 双击 `outputs/LPP.exe` 运行开发预览版；`LPP.pck` 必须和它放在一起。
- 从源码运行：双击 `start.cmd`，或在 PowerShell 运行 `./start.ps1`。
- 编辑项目：`./start.ps1 -Editor`，也可以用 Godot 4.5.1 打开 `project.godot`。
- 重新打包预览版：`./build.ps1`。本地引擎位于 `work/tools/godot/`，启动器也支持 `-GodotPath` 指定其他 Godot 4.5.1+ 引擎。

预览 EXE 使用官方 Godot 编辑器二进制和项目 PCK，因此体积较大。正式发布时安装对应版本的 Windows export templates，从 `Windows Desktop` 预设导出精简发行版。Godot 许可证随预览版一起提供。

## 使用

左键拖动移动，轻点播放互动回弹。拖动时松开鼠标，桌宠会沿最近的拖动方向继续运动，受到重力落向所在屏幕工作区底部（任务栏上方），碰到外侧边缘和落地时衰减回弹，最终因摩擦停止。停住鼠标再松手则自然下落；飞行中再次抓住可打断运动。角色脚底作为落地基准，PNG 透明留白不会变成悬空距离。

支持多显示器的负坐标、不同分辨率和横竖屏组合。完整设置的“桌面与显示”和右键菜单均可选择“移到显示器”；设置窗口跟随桌宠所在屏幕打开，快捷菜单使用鼠标所在屏幕的工作区。可以直接拖到其他屏幕，松手后在那里继续运动。相接的工作区支持散步和惯性跨屏；较低的底边会触发继续下落，较高的底边是不可直接走上的台阶。上下工作区相接时也可惯性通过；屏幕间的空隙和任务栏区域不会被当作可通行桌面。程序每秒检测布局变化，拔屏或更改分辨率后自动把桌宠找回到最近的可用屏幕。

右键打开快捷菜单：自由散步、始终置顶、切换外观、大小调节、找回桌宠、退出，以及“打开完整设置”。“桌宠大小”打开带滑动条和百分比输入框的小面板。菜单或大小面板展开时暂时冻结运动，关闭后继续；在完整设置中仍可正常下落。快捷菜单与完整设置共享配置并即时同步。首次启动自动展示设置；关闭设置窗口后桌宠继续运行。

设置采用浅色卡片与侧边导航，按用途分为四个页面：

- **外观与模型**：当前角色实时预览、外观选择、导入和外观文件夹入口。
- **动作与适配**：自动识别动画和角色风格、七个状态逐项映射与预览、3D 朝向和 2D 翻转、模型转换器配置。
- **行为与互动**：自由散步开关、当前状态、鼠标互动说明。
- **桌面与显示**：滑动条与百分比输入同步的尺寸调节、置顶开关、位置重置。

桌宠大小范围为 **20%–300%**，以 1% 为步长。滑动即时生效；手动输入后按 Enter 或移开焦点应用，超出范围时自动限制到上下限。两处大小控件均提供“恢复 100%”，数值会自动保存到下次启动。

各页独立滚动，修改即时生效并自动保存。角色预览在关闭设置或切换到其他分类后暂停渲染。

默认角色为 **黍泡泡**，使用按实物照片重建的毛绒模型，包含六组专用动作。身体后半段会受重力下沉和压缩，使下腹形成较平的承托面；背鳍、缝线、绒毛和尾巴一起变形，四只脚保持支撑。拖动时带有惯性和阻尼回稳，下落时填充卸载，落地时恢复承托。模型使用标准 GLB 外观包接入，详见 [模型说明与导出方法](docs/shu-model.md)。原来的四种示例仍可切换，已有外观及大小、行为、多屏位置设置继续沿用。

## 用户自定义

详见 [外观格式与动作适配](docs/skin-packs.md)。可直接导入 GLB、glTF、FBX、OBJ、VRM、PNG、JPEG、WebP、BMP、TGA、SVG 和 GIF；支持带清单的精灵图和序列帧。Blender、STL、PLY 文件可使用本机 Blender 4.2+ 自动转换。导入在后台完成，模型统一安装为自包含 GLB，无需重启。

动作优先使用模型自带动画，识别中英文及带命名空间的动作名称，缺少对应动作时按软团、Q 版人物或平面图片风格使用通用模组。已识别骨骼和眨眼 / 微笑形态键可参与通用动作；没有骨架或表情的图片、静态模型使用整体动作。支持手动覆盖和恢复自动匹配。Live2D / Spine 专用工程目前需先导出为 GIF、精灵图或图片序列；VRM 暂不包含专用表情绑定、MToon 或弹簧骨物理。

运行数据保存在 `%APPDATA%/LPPDesktopPet/`：

- `skins/`：导入的用户外观，设置页有“打开外观目录”按钮。
- `preferences.json`：外观 ID、显示大小、散步开关、置顶和窗口位置。
- `appearance-overrides.json`：各外观独立的动作、风格、朝向和翻转设置。
- `converter-path.txt`：可选的 Blender 可执行文件路径。
- `logs/`：Godot 运行日志。

## 代码结构

| 文件 | 职责 |
| --- | --- |
| `scripts/main.gd` | 组装模块、窗口移动、输入、切换外观时保留旧角色 |
| `scripts/core/pet_brain.gd` | 待机 / 散步 / 睡觉 / 摸摸 / 拖动 / 落下状态 |
| `scripts/core/pet_motion.gd` | 拖动速度采样、惯性、重力、边缘碰撞与落地摩擦 |
| `scripts/core/desktop_layout.gd` | 屏幕工作区集合、共享边缘通行、屏幕归属与离屏恢复 |
| `scripts/core/skin_catalog.gd` | 扫描、清单校验、导入和重复 ID 处理 |
| `scripts/core/preferences.gd` | 配置读写，损坏数据使用默认设置 |
| `scripts/avatars/avatar_base.gd` | 统一状态接口和程序化弹性动作 |
| `scripts/avatars/sprite_avatar.gd` | 图片、GIF、精灵图和序列帧播放、透明裁剪与落地点 |
| `scripts/core/action_adapter.gd` | 通用状态、中英文动作匹配、角色风格识别 |
| `scripts/core/asset_io.gd`、`asset_importer.gd` | 格式解码、依赖封装、后台导入和失败回滚 |
| `scripts/core/obj_reader.gd`、`gif_reader.gd` | 运行时 OBJ / MTL 与 GIF 解码 |
| `scripts/core/skin_overrides.gd` | 按角色保存动作适配覆盖 |
| `scripts/avatars/procedural_rig.gd` | 常见骨骼部位识别、坐标轴换算和轻量动作 |
| `converters/model_to_glb.py` | Blender 模型转换，禁用自动执行源文件脚本 |
| `scripts/avatars/model_avatar.gd` | 3D 渲染、运行时 GLB 解码、尺寸归一化、动画映射 |
| `scripts/avatars/model_physics.gd`、`soft_body.gd` | 显式配置的身体截面软体形变、承托约束、惯性与 GPU 蒙皮 |
| `scripts/ui/settings_window.gd` | 独立原生设置窗口和文件导入入口 |
| `scripts/ui/quick_menu.gd` | 右键快捷菜单、外观子菜单和独立大小调节面板 |
| `scripts/ui/scale_control.gd` | 完整设置与快捷面板共享的滑动条、百分比输入和复位控件 |

增加新渲染格式时实现 `PetAvatar.configure(skin)`、`set_state(state)`，再在 `main.gd` 添加工厂分支；已有行为层与桌面层无需重写。

## 验证

```powershell
$engine = './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe'
& $engine --headless --path . --script tests/test_core.gd
& $engine --headless --path . --script tests/test_runtime.gd
& $engine --headless --path . --script tests/test_soft_body.gd
& $engine --headless --path . --script tests/test_motion.gd
& $engine --headless --path . --script tests/test_desktop.gd
& $engine --path . -- --smoke-test
& $engine --headless --main-pack outputs/LPP.pck -- --smoke-test
```

扩展适配检查先运行 `python tests/create_adapter_fixtures.py`（需 Pillow），再用 Blender `--background --factory-startup --disable-autoexec --python tests/create_blender_fixtures.py` 生成实际格式样本。以图形模式运行 Godot `--path . --script tests/test_adapters.gd` 和 `--path . --script tests/test_adaptation_ui.gd`；FBX 形态键再导出依赖图形渲染器，不使用 headless。

核心检查覆盖路径越界、无效清单、重复 ID、行为中断和配置恢复。运行时检查验证用户 GLB 导入、嵌入动画和缺少动作的降级。冒烟模式逐一切换内置外观后退出；它会使用并保存当前用户设置，仅供开发。

物理检查覆盖抛出方向、重力、衰减回弹、静止松手、速度上限、边缘碰撞、长帧防穿透、飞行中抓取、帧率一致性和负坐标显示器。

软体检查验证后半身下沉及截面压缩、腹部承托、四脚保留、绒毛和尾巴的地面约束、原生动画叠加、GPU 形变一致性、下落卸载及缩放复位。`tools/render_model_physics.gd` 生成实际 Godot 渲染，`python tools/make_shu_physics_preview.py` 生成侧面对比图。

## 当前边界

- 已实现透明置顶小窗口、惯性拖动与重力、落地回弹、右键快捷菜单、基本互动、散步、工作区边界限制、多格式外观热切换、自动及手动动画映射和设置保存。
- 鼠标可点击区域当前为包围角色的矩形区域，矩形外可穿透；矩形内部分透明像素仍会接收点击。精确按角色轮廓穿透、系统托盘、开机启动和窗口边缘攀爬尚未实现。
- 软体采用需要外观作者声明的截面形变及承托约束，适合毛绒身体；不提供任意模型的自动软体识别、布料自碰撞或完整体积有限元模拟。已有蒙皮或形态键的部件保留原绑定，不覆盖生成。
- 多屏按各自工作区处理，禁止穿入布局空隙；已在当前三块真实显示器上验证移动与设置定位。混合 DPI 多屏的视觉尺寸一致性、独占全屏和各种显卡的透明兼容性仍需专项验证。显示器编号使用操作系统的枚举顺序，重接设备时编号可能变化。
- 惯性和重力作用于桌面角色窗口，不依赖模型刚体；子步内重新计算屏幕归属，可以沿相接工作区跨屏。GLB 落地基准取静态网格包围盒，复杂动画或自带根位移动画仍需模型适配。
- 图片、GIF、精灵图和序列帧均参与统一动作状态；通用模组不生成图片里不存在的可动部位。常见骨架可进行轻量动作，不提供任意骨架的自动蒙皮、跨模型动画重定向、复杂 IK 或精细步态。
- 资源包为数据文件，不加载用户 GDScript、DLL 或 Godot 场景。导入前限制资源体积，glTF 的外部依赖仅允许同目录及子目录并嵌入安装文件，但不是针对恶意二进制文件的完整隔离沙箱。

运行时 GLB 导入依据 [Godot GLTFDocument](https://docs.godotengine.org/en/stable/classes/class_gltfdocument.html)；透明与鼠标区域依据 [Godot Window](https://docs.godotengine.org/en/4.5/classes/class_window.html)。
