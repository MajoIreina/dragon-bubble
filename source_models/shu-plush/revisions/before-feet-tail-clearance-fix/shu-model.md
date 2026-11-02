# 黍泡泡毛绒模型

当前版本依据用户提供的 18 张实物照片，在本机 Blender 中重新构建。横向水滴身体、分叉渐变角、四只扁圆脚、连续三峰背鳍、双片折叠尾鳍、侧面五格刺绣、玉珠环及红色飘带均为可单独编辑的部件。碗、背景和商品布标不进入模型。

## 文件

- `source_models/shu-plush/shu-plush.blend`：本次重建的可编辑 Blender 工程；角色位于 `ShuPlush` collection，摄影机和灯光独立于角色。
- `tools/build_shu_plush.py`：可重复执行的 Blender 建模入口。
- `skins/shu/shu.glb`：桌宠使用的自包含模型，包含材质、纹理和六组动作。采用 glTF +Y 向上、+Z 朝前，清单旋转为零。
- `skins/shu/skin.json`：保留 `builtin.shu` ID，继续使用原有外观选择设置。
- `tools/export_shu.py`：只导出 `ShuPlush` collection，按共享名称的 NLA tracks 合并部件动作，排除灯光和相机。
- `source_models/shu/龙泡泡.blend`：原样保留的用户旧源文件。
- `source_models/shu-plush/pre-plush-shu.glb`：首次导出前保存的运行时模型备份；后续导出不会覆盖。
- `outputs/shu-turnaround.png`：Godot 实际渲染的正面、半侧面和侧面预览。

## 表面和动作

毛绒由表面法线纹理和短绒几何共同表现，使 GLB 在 Godot 中也能显示细小绒毛，而不依赖仅 Blender 支持的毛发效果。脚底、角、尾鳍和刺绣边缘保留缝线细节。

左角珠串根据补充的三张近照修正：围绕含前短枝的完整角根，沿头壳斜坡排列，内侧高、外侧低。`tools/shu_bracelet.py` 对实际角壳和头壳计算球面间距，再按弧长均匀放置珠子；每次重建会检查珠子、壳体与穿珠线是否相交，并在 `work/shu-bracelet-fit.json` 输出数值。当前为 22 颗光滑乳白珠子。正脸中央的竖向缝线已移除。

右角饰采用完整角根上的白色三股编绳，两端穿入银夹卷耳，夹住橙红布带。`tools/shu_horn_cord.py` 根据角壳和头壳拟合绳圈，输出 `work/shu-right-cord-fit.json`；银夹、卷耳和绳归属静态 `HornCordAttachment`，布带从夹口下方的 `PendantSway` 支点摆动。布带长宽比约 4.15，贴着头侧弧面下垂。

模型提供 `Idle`、`Walk`、`Sleep`、`Happy`、`PickedUp`、`Land` 六组部件动画，分别映射到待机、行走、睡觉、抚摸、拖拽/腾空和落地。它们是为此造型制作的风格化动作，不是照片推导出的真实骨骼运动。桌宠的惯性、重力、多屏、右键菜单和缩放仍由通用软件框架负责，落地弹性叠加在动作上。

## 重建和导出

在项目根目录运行：

```powershell
python tools/make_plush_textures.py
& 'D:\SteamLibrary\steamapps\common\Blender\blender.exe' --background --disable-autoexec --python tools/build_shu_plush.py
& 'D:\SteamLibrary\steamapps\common\Blender\blender.exe' --background --disable-autoexec --python tools/export_shu.py
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --headless --path . --editor --import --quit
./build.ps1
```

导出脚本使用 `use_scripts=False` 打开工程，不执行文件中内嵌的自动脚本，也不改写源工程。它要求角色 collection 存在且含六个预期动作，避免意外导出其他场景。

源工程默认显示静态检查姿态，NLA tracks 暂时静音；编辑动作时可单独启用对应 track。导出入口会启用这些轨道，并保留睡眠闭眼等恒定通道，防止导出优化误删。原旧模型不会被覆盖。

生成实际桌宠预览需要图形显示：

```powershell
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --path . --script tools/render_shu.gd
python tools/make_shu_preview.py
```

动作验证使用：

```powershell
& './work/tools/godot/Godot_v4.5.1-stable_win64_console.exe' --headless --path . --script tests/test_runtime.gd
```

运行测试除了检查动作名称和映射，还会实际采样睡眠时的闭眼、身体压缩、行走前脚运动及切回待机后的睁眼，防止恒定姿态关键帧被导出优化丢弃。
