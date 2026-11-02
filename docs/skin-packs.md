# My Dragon Bubble 外观包与动作适配

## 支持的文件

在设置 → **外观与模型 → 导入新外观**，直接选择文件即可，无需编写清单。

| 类型 | 格式 | 实际处理 |
| --- | --- | --- |
| 通用 3D | GLB、glTF 2.0 | 保留网格、PBR 材质、骨骼、形态键及动画；外置缓冲区和贴图打包到 GLB |
| FBX | .fbx | Godot 原生解码并转换为自包含 GLB，保留支持的贴图、骨骼及动画；普通桌面运行无需 Blender |
| Wavefront | OBJ + MTL | 支持 UV、法线、负索引、多边形三角化、Kd / d / Tr / map_Kd；作为静态模型适配动作 |
| VRM 基础模型 | .vrm | 按 glTF 加载基础网格、骨架、贴图及形态键；不提供 VRM 专用表情绑定、MToon 或弹簧骨实现 |
| Blender / 静态几何 | .blend、STL、PLY | 使用本机 Blender 4.2+ 在后台转换；自动寻找安装位置，或在“动作与适配”设置路径 |
| 图片 | PNG、JPG / JPEG、WebP、BMP、TGA、SVG | 自动裁去透明留白，按比例适配尺寸和落地点 |
| 动图 | GIF87a / GIF89a | 解码逐帧动画、帧时长、透明色、局部调色板、交错及 disposal 2 / 3 |
| 2D 动作包 | 精灵图 / 图片序列 + skin.json | 按动作切换帧序列，可配置帧率、循环和逐帧时长 |

`.blend` 转换禁用自动脚本执行，不保存或修改源文件；源场景中可渲染的网格和有效关联骨架参与导出，摄影机和灯光不参与桌宠渲染。导入有独立任务，不阻塞桌面互动；失败时清理本次未完成的外观目录并保留当前角色。

glTF 的外置文件必须位于模型所在目录或子目录内，支持 URI 百分号编码，不支持绝对路径、上级目录或网络 URI。OBJ 的 MTL 和贴图也应放在同目录或子目录，`map_Kd` 暂不支持额外命令选项。STL 本身不包含角色骨架或标准贴图，无法凭空恢复这些信息。

Live2D Cubism、Spine 和其他专用 2D 工程当前没有原生运行时，请先导出 GIF、精灵图或透明图片序列。PSD、压缩包、任意引擎场景 / 插件，以及 Draco 等引擎未支持的压缩模型不在当前兼容范围；建议导出普通 glTF 2.0。VRM 支持基础模型导入，不等于完整 VRM 渲染器。

## 动作选择顺序

每种外观统一覆盖八个状态：

| 状态 | 自带动画名称示例 | 没有专用动画时 |
| --- | --- | --- |
| idle 待机 | Idle、Breathe、Stand、待机、呼吸 | 整体呼吸；已识别的头部 / 脊柱轻动、眨眼形态键 |
| walk 行走 | Walk、Walking、行走、走路 | 轻微行走起伏；已识别的四肢和尾巴摆动 |
| run 跑步 | Run、Running、Sprint、Jog、跑步、奔跑 | 更快的腾跃、较大步幅与肢体摆动 |
| sleep 睡眠 | Sleep、Sleeping、Nap、睡眠 | 整体压低、慢呼吸；已识别的头部 / 肢体放松、闭眼形态键 |
| petted 摸摸回应 | Happy、Wave、Pet、摸摸 | 短暂弹性回应；已识别的头部 / 手臂轻动、微笑形态键 |
| dragged 提起 | PickedUp、Held、Grab、提起 | 整体拉伸；已识别的手臂轻摆 |
| airborne 空中 / 下落 | Falling、Jump、Fly、下落 | 轻拉伸、下落姿态；惯性和重力由桌面物理控制 |
| fall 落地 | Land、Landing、Impact、落地 | 脚底锚定的压缩回弹；已识别的膝部 / 躯干轻动 |

优先使用手动映射，再按中英文名称、大小写、命名空间和单词边界自动识别。`mixamo.com|Walking` 等名称可以匹配；不把任意无关动画当作 Idle，也区分 Falling 与 Landing。默认循环待机、行走、跑步、睡眠、提起和空中动作，回应与落地为单次动作。旧外观只有一种移动动画时，自动在走路和跑步间复用，跑步复用走路时加快播放；两个状态的手动选择互相独立。

缺少对应动作时，允许 Idle 作为动画背景，并叠加通用动作。手动选“通用动作”则停止自带动画，仅使用通用表现。切换通用状态时恢复网格、骨骼和形态键的基础姿态，避免上一个动作留下变形；内置毛绒模型的原生动作之间使用平滑混合。自带动作的水平根位移被固定，桌面位置统一由软件物理控制。

到屏幕外侧边缘时，桌宠暂停移动并执行平滑转身，再反向起步；3D 角色通过正面转过 180°，2D 外观在换步中点翻转。存在名为 `Turn` 的模型动画时用于这一过渡，不增加需要手动匹配的行为状态。

自动风格识别结合模型比例和常见人形骨骼名称：龙泡泡 / 软团有更明显的弹性，Q 版人物的整体形变更轻，图片使用平面动作。模型自带对应动作时，减弱整体形变叠加。可识别常见 Mixamo / VRM / 英文及部分中日文头部、躯干、四肢和尾骨命名，并换算骨骼静止坐标轴。未知骨骼保持不动，不自动生成权重。

这套通用动作覆盖所有受支持外观的状态反馈，但不会把静态图片变成可独立摆手的角色，也不会把没有绑定的模型自动变成完整骨骼角色。不进行跨模型的任意动画重定向；精细步态、口型、复杂 IK 和陌生骨架命名仍需要模型作者制作或适配。

设置 → **动作与适配** 显示动画、骨骼、已匹配动作数量，支持八个状态的逐项选择和预览、风格选择、3D 朝向修正及 2D 翻转开关。所有覆盖按外观 ID 保存在 `appearance-overrides.json`，不修改原模型或内置清单，可恢复默认适配。

## 可选清单：3D

```json
{
  "schema_version": 1,
  "id": "yourname.chibi",
  "name": "我的 Q 版角色",
  "type": "model3d",
  "asset": "character.glb",
  "profile": "auto",
  "rotation_degrees": [0, 0, 0],
  "animations": {
    "idle": "Standing",
    "petted": "Wave",
    "sleep": ""
  }
}
```

省略的映射自动识别，空字符串强制使用通用动作。`profile` 可为 `auto`、`blob`、`character`、`flat`。默认 Y 向上、正脸朝 +Z，相机位于 +Z 看向原点；`rotation_degrees` 在源场景姿态之外追加旋转，不覆盖源模型根节点变换。自动按网格最长边归一化到 2.35 个场景单位并居中，脚底取可见网格最低点。复杂动画振幅仍应由作者控制，超大跳跃、长配饰和不适合桌宠的构图可能需要调整。

## 可选清单：毛绒软体

3D 清单可添加 `physics.soft_bodies`。它为显式指定的静态身体部件生成截面蒙皮，后段在重力下沉低、压缩，腹部由承托面和脚支撑，并响应桌面移动惯性。黍泡泡的配置如下：

```json
"physics": {
  "soft_bodies": [{
    "node": "PlushBody",
    "profile_mesh": "BodyShell • 横向水滴",
    "anchors": ["FrontFoot_L", "FrontFoot_R", "RearFoot_L", "RearFoot_R"],
    "axis": [0, 0, -1],
    "start": 0.0,
    "length": 2.3,
    "sections": 28,
    "support_height": -0.82,
    "floor_height": -1.01,
    "compression": 0.18,
    "gravity": 9.8,
    "stiffness": 42.0,
    "damping": 10.0
  }]
}
```

`node` 为整个身体的节点名或模型内相对路径；`profile_mesh` 和 `anchors` 相对于该节点查找。使用导入 Godot 后的精确名称（例如 Blender 的点号可能变成下划线）。`profile_mesh` 用于采样身体上下轮廓；`anchors` 排除四脚等承托子树，其他附件和绒毛一起变形。`axis` 是身体局部水平的前往后方向，局部 Y 必须向上；`start`、`length` 和两个高度均使用该节点坐标，跟随模型自动归一化。`support_height` 是后腹目标高度，`floor_height` 是包含尾片和绒毛的下限。

每个外观最多 8 处软体，每处 4–48 个截面。`compression` 范围 0–0.5，`gravity` 范围 0–9.8（0 关闭静态重力沉降），`stiffness` 范围 1–200，`damping` 范围 0.5–40。只对作者声明且没有已有蒙皮或形态键的子树生成权重；遇到冲突则保留原绑定并跳过该处软体。原始 GLB 资源不被改写，原生节点动画仍可叠加。该功能不是通用布料或完整体积软体求解器，未声明的导入模型保持原有表现。

## 可选清单：静态图片

```json
{
  "schema_version": 1,
  "id": "yourname.picture",
  "name": "我的图片桌宠",
  "type": "sprite",
  "asset": "pet.webp",
  "flip_with_direction": true
}
```

建议透明背景；JPEG 等不透明图片会保留矩形背景。透明留白按所有帧共同范围裁剪，最长边缩放到 232 个逻辑像素。所有帧共享画布与落地点，切换帧时不会因独立裁剪产生跳动。

## 可选清单：2D 精灵图

将各帧放入规则网格，帧号从 0 开始，从左到右、从上到下：

```json
{
  "schema_version": 1,
  "id": "yourname.sprite",
  "name": "我的 2D 伙伴",
  "type": "sprite",
  "asset": "sheet.png",
  "grid": [4, 2],
  "clips": {
    "Idle": {"frames": [0, 1], "fps": 6, "loop": true},
    "Walk": {"frames": [2, 3, 4, 5], "fps": 12, "loop": true},
    "Happy": {"frames": [6, 7], "fps": 8, "loop": false},
    "Sleep": {"frames": [1, 0], "durations": [0.8, 1.2], "loop": true}
  }
}
```

也可省略 `grid`，增加 `frames` 相对路径数组：`["frames/00.png", "frames/01.png", "frames/02.png"]`。此时 `asset` 指向任意一张基础帧，`clips` 使用数组中的帧号。没有 `clips` 时，将全部帧作为循环 Idle。GIF 直接导入时将原始帧时长保留为循环 Idle。

## 限制与安装目录

单文件 / glTF 资源合计上限 256 MB；静态图片不超过 8192 × 8192；GIF 不超过 4096 × 4096。2D 最多 256 张源帧，解码后合计不超过 128 MB，每个动作最多 4096 个帧引用，帧率不超过 120 FPS。建议桌宠模型采用低面数和 1K–2K 贴图。

导入后资源保存在 `%APPDATA%/LPPDesktopPet/skins/<id>/`。3D 安装为自包含 GLB，2D 安装资源和声明的序列帧；不执行外观包代码。外观 ID 最长 80 个字符，仅允许小写字母、数字、点、横线和下划线，重复 ID 不覆盖。手动放入外观目录时，3D 建议直接使用自包含 GLB；需要转换的源格式应通过导入入口安装。

Blender 路径可在设置中保存，也可设置 `LPP_BLENDER_PATH` 环境变量。转换失败详情记录为运行数据目录内的 `conversion-last.log`。

底层格式兼容性依据 [Godot GLTFDocument](https://docs.godotengine.org/en/4.5/classes/class_gltfdocument.html)、[FBXDocument](https://docs.godotengine.org/en/4.5/classes/class_fbxdocument.html) 和 [SkeletonModifier3D](https://docs.godotengine.org/en/4.5/classes/class_skeletonmodifier3d.html)。
