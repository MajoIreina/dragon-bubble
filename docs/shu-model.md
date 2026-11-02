# 黍泡泡毛绒模型

当前版本依据用户提供的 18 张实物照片及 2026-11-02 补充的四张脚部、尾部照片，在本机 Blender 中构建和修正。横向水滴身体、分叉渐变角、四只圆底短柱软脚、连续三峰背鳍、双尖叠折尾鳍、侧面五格刺绣、玉珠环及红色飘带均为可单独编辑的部件。碗、背景和商品布标不进入模型。

## 文件

- `source_models/shu-plush/shu-plush.blend`：本次重建的可编辑 Blender 工程；角色位于 `ShuPlush` collection，摄影机和灯光独立于角色。
- `tools/build_shu_plush.py`：可重复执行的 Blender 建模入口。
- `tools/shu_dorsal.py`：按侧视手稿定义的连续背鳍轮廓，与完整重建和局部更新共用。
- `tools/update_shu_dorsal.py`：只更新背鳍、顶部缝线和短绒，保留其他部件与动作，并检查网格闭合性。
- `tools/shu_gaits.py`、`tools/update_shu_gaits.py`：生成走路和跑步部件动画，支持只更新动作并校验网格、材质、父子关系及其他动作未变。
- `tools/shu_rest_actions.py`、`tools/update_shu_rest_actions.py`：只更新待机、睡眠和原地踏步转身，校验网格与其他动作保留。
- `skins/shu/shu.glb`：桌宠使用的自包含模型，包含材质、纹理和八组动作。采用 glTF +Y 向上、+Z 朝前，清单旋转为零。
- `skins/shu/skin.json`：保留 `builtin.shu` ID，继续使用原有外观选择设置。
- `tools/export_shu.py`：只导出 `ShuPlush` collection，按共享名称的 NLA tracks 合并部件动作，排除灯光和相机。
- `source_models/shu/龙泡泡.blend`：原样保留的用户旧源文件。
- `source_models/shu-plush/pre-plush-shu.glb`：首次导出前保存的运行时模型备份；后续导出不会覆盖。
- `outputs/shu-turnaround.png`：Godot 实际渲染的正面、半侧面和侧面预览。

## 表面和动作

毛绒由表面法线纹理和短绒几何共同表现，使 GLB 在 Godot 中也能显示细小绒毛，而不依赖仅 Blender 支持的毛发效果。脚底、角、尾鳍和刺绣边缘保留缝线细节。

背鳍依据补充的侧视手稿调整为向后延伸的连续三峰：各峰前缘为较长的上升斜坡，后缘陡降，峰顶和峰谷圆滑衔接。后两峰增加相对背部的高度，使身体下沉后仍保留清晰轮廓；三峰的绝对高度随背部向后降低。根部连续埋入背面，短绒和顶部缝线采用同一轮廓，随原有身体软体共同变形。`outputs/shu-dorsal-comparison.png` 为开启身体软体后的调整前后对照，可在运行 `tools/render_model_physics.gd` 后用 `python tools/make_shu_dorsal_preview.py` 重新生成。

根据后续高度反馈，三峰相对局部背面的露出高度统一降低 15%，根部位置和嵌入深度保持原有设置。`tools/shu_dorsal.py` 中的 `HEIGHT_SCALE` 同时作用于背鳍和顶部缝线，短绒从调整后的表面重建。降低前的版本保存在 `source_models/shu-plush/revisions/before-dorsal-lowering/`，运行 `python tools/make_shu_dorsal_preview.py --lowering` 生成本轮对照 `outputs/shu-dorsal-lowering.png`。

尾侧最后一峰在上述版本上再降低 15%，由 `REAR_PEAK_HEIGHT_SCALE` 控制。以最后一峰露出背部的高度计算下降量，连同前方峰谷平滑下移，保留前缘上升的轮廓；额外下降从第二峰峰顶后渐入，在尾侧根部渐出，前两峰峰顶保持当前高度。调整前的版本保存在 `source_models/shu-plush/revisions/before-rear-dorsal-lowering/`，运行 `python tools/make_shu_dorsal_preview.py --rear-lowering` 生成对照 `outputs/shu-rear-dorsal-lowering.png`。

四脚以独立的圆形底片、圆钝底缘和短柱侧壁重建，替换原先过扁的椭球；底缘保留一圈缝线。尾巴采用双尖轮廓，内片覆盖外片，布褶由曲面凹陷和鼓起形成。根据厚度反馈，两片尾布的填充厚度、横向错位和褶皱起伏均减至上一版约一半，边缘过渡稍收薄，保留原有侧面轮廓与折叠结构。脚和尾片均为闭合网格，保留原脚部及尾部动作支点。

左角珠串根据补充的三张近照修正：围绕含前短枝的完整角根，沿头壳斜坡排列，内侧高、外侧低。`tools/shu_bracelet.py` 对实际角壳和头壳计算球面间距，再按弧长均匀放置珠子；每次重建会检查珠子、壳体与穿珠线是否相交，并在 `work/shu-bracelet-fit.json` 输出数值。当前为 22 颗光滑乳白珠子。正脸中央的竖向缝线已移除。

正视图右侧角饰采用完整角根上的白色三股编绳，两端穿入银夹卷耳，夹住橙红布带。`tools/shu_horn_cord.py` 根据角壳和头壳拟合绳圈，银夹检查范围包含夹齿和卷耳。布带同时避让头壳和角壳，并按完整摆动范围求出间隙，避免静态不相交、摆动后穿入头部。拟合结果位于 `work/shu-right-cord-fit.json`；银夹、卷耳和绳归属静态 `HornCordAttachment`，布带从夹口下方的 `PendantSway` 支点摆动。

模型提供 `Idle`、`Walk`、`Run`、`Turn`、`Sleep`、`Happy`、`PickedUp`、`Land` 八组部件动画；`Turn` 是移动期间的转身过渡，其余映射到待机、走路、跑步、睡觉、抚摸、拖拽/腾空和落地。它们是为此造型制作的风格化动作，不是照片推导出的真实骨骼运动。桌宠的惯性、重力、多屏、右键菜单和缩放仍由通用软件框架负责，落地弹性叠加在动作上。

左右直线移动保持正侧面（±90°），停止后逐渐回到半侧面。走路周期为 29/24 秒，四脚相差四分之一周期，支撑期占 70%。跑步参考猫科双腾空奔跑：周期为 16/24 秒，后腿蹬伸、展体腾空、前腿接地、收腹腾空；每脚支撑期占 25%，左右脚略微错开，前后腿分组发力。身体同步俯仰、伸展，运行时截面软体在收腹时抬高中后背、将后段向前收拢，展体时舒展；脸部保留稳定。脚掌支撑时保持水平，并补偿父节点运动。100% 大小时桌面速度分别为 42 和 112 像素/秒，缩放时按比例改变位移，避免脚掌相对地面滑动。原生移动动画不再叠加通用整体弹跳。

屏幕外侧边缘按最后一小段步幅减速，步态同步减慢；到边缘后暂停桌面位移，走路用 0.8 秒、跑步用 0.66 秒执行原地踏步转身。朝向通过正面连续转过 180°，交替抬脚且至少保留两脚支撑，再用 0.22 秒恢复移动速度。转身不消耗行为剩余时间；菜单暂停动作和时钟，拖拽、腾空或其他状态可打断。旧模型没有 `Turn` 时仍平滑转向，2D 外观在换步中点翻转。

待机周期为 6 秒，包含柔和呼吸、身体顾盼、尾部滞后和一次双眨眼；睡眠周期为 5.5 秒，闭眼、身体放松压低并慢呼吸，四脚在世界坐标保持承托。模型原生状态之间保留动画混合，入睡和醒来用 0.42 秒过渡。`state_effects.gd` 绘制少量待机闪光及睡眠气泡、交错上浮的 Z 字；鼻息位置投影实际头部，醒来淡出。`tools/render_turn_rest.gd` 与 `tools/make_turn_rest_preview.py` 生成包含运行时特效的预览。

`tools/update_shu_gaits.py` 当前在现有源工程上只更新 `Run`，校验其他动作及几何保留；完整建模入口仍生成走路与跑步。猫科跑步调整前的模型位于 `source_models/shu-plush/revisions/before-feline-run/`，更早备份位于 `source_models/shu-plush/revisions/before-locomotion-rework/`。实际 Godot 预览可运行 `tools/render_locomotion.gd`，再运行 `python tools/make_locomotion_preview.py`，生成走路、跑步 GIF 和 `outputs/locomotion-preview.png` 逐帧图。

## 运行时软体

依据用户的侧面草图，`skin.json` 为 `PlushBody` 声明 28 段独立的可压缩截面，以白色身体网格采样原始上下轮廓。重力使后段填充逐渐沉到四脚上方的承托高度，并适度压缩和横向鼓起；前部保持圆高，下腹逐渐趋平，背部向尾根沉低。身体、背鳍、缝线、绒毛及尾片共享连续权重，四脚及其绒毛排除在形变之外。该效果在运行时生成，不改写 Blender 工程或 GLB 的原始网格。

固定 120 Hz 的弹簧响应叠加桌面移动加速度和阻尼。自由下落时减小有效重量，落地后重新承托，身体逐渐回稳。地面约束包含尾片及绒毛，避免动态下摆穿过支撑面；切屏、缩放和长帧停顿复位惯性。八组动作继续由原节点播放，尾巴摆动等变换合入 GPU 蒙皮矩阵。

执行 `tests/test_soft_body.gd` 验证软体约束；运行 `tools/render_model_physics.gd` 后，`python tools/make_shu_physics_preview.py` 生成 `outputs/shu-soft-comparison.png`，左侧关闭软体，右侧开启软体，镜头、姿态和比例一致。此处为毛绒造型设计的简化软体，不计算布料自碰撞或完整填充材料力学。

## 重建和导出

在项目根目录运行：

```powershell
python tools/make_plush_textures.py
& './work/tools/blender/blender-5.2.0-windows-x64/blender.exe' --background --disable-autoexec --python tools/build_shu_plush.py
& './work/tools/blender/blender-5.2.0-windows-x64/blender.exe' --background --disable-autoexec --python tools/check_shu_clearance.py
& './work/tools/blender/blender-5.2.0-windows-x64/blender.exe' --background --disable-autoexec --python tools/export_shu.py
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --headless --path . --editor --import --quit
./build.ps1
```

导出脚本使用 `use_scripts=False` 打开工程，不执行文件中内嵌的自动脚本，也不改写源工程。它要求角色 collection 存在且含八个预期动作，避免意外导出其他场景。所有动作从零秒开始，避免循环开头多停一帧；GLB 的默认节点变换恢复为静态姿态，防止导出器最后采样的跑步姿态影响尺寸归一化和通用动作。

源工程默认显示静态检查姿态，NLA tracks 暂时静音；编辑动作时可单独启用对应 track。导出入口会启用这些轨道，并保留睡眠闭眼等恒定通道，防止导出优化误删。原旧模型不会被覆盖。

导出时仅在内存中按三倍时间轴、72 Hz 采样，再按对应帧率输出相同秒数的动作；源工程保留 24 FPS。密集采样改善猫科跑步中父节点缩放、俯仰与脚掌反向补偿的插值精度，避免低频采样导致支撑脚滑动。

生成实际桌宠预览需要图形显示：

```powershell
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --path . --script tools/render_shu.gd
python tools/make_shu_preview.py
```

动作验证使用：

```powershell
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --headless --path . --script tests/test_runtime.gd
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --headless --path . --script tests/test_locomotion.gd
```

运行测试除了检查动作名称和映射，还会实际采样睡眠时的闭眼、身体压缩、行走前脚运动及切回待机后的睁眼，防止恒定姿态关键帧被导出优化丢弃。移动专项检查验证左右完整侧身、跑步状态与兼容回退、脚底接地、走路支撑数、跑步腾空、步幅与屏幕位移抵消及无跳变循环。

`tools/check_shu_clearance.py` 检查四脚和尾片的闭合性，并逐个动作采样挂饰网格的顶点、边中点和面中心，对实际头壳、右角壳测距；报告为 `work/shu-correction-validation.json`。`tools/render_shu_corrections.py` 生成整体、侧面、脚部、脚底、尾部和挂饰近景，保存在 `outputs/shu-corrected-*.png`。

本次修正前的 Blender 工程、GLB 和建模脚本已备份至 `source_models/shu-plush/revisions/before-feet-tail-clearance-fix/`。

尾巴减薄前的模型和近景保存在 `source_models/shu-plush/revisions/before-tail-thinning/`。

背鳍手稿调整前的模型、建模脚本和运行时侧视图保存在 `source_models/shu-plush/revisions/before-dorsal-sketch-fix/`。仅更新背鳍时可运行 Blender 的 `--background --disable-autoexec --python tools/update_shu_dorsal.py`，然后按上述步骤重新导出和导入；报告写入 `work/shu-dorsal-validation.json`。
