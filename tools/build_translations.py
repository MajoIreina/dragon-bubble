"""Build the application's English/Japanese gettext catalogs from UTF-8 text.

Chinese source strings remain the fallback. Preserve printf placeholders;
model clip names, skin IDs and user-supplied names are never translation keys.
"""
import json
import re
from pathlib import Path


DATA = r"""
设置|Settings|設定
外观与模型|Appearance|外観とモデル
选择你的桌面伙伴，或带来新的模样。|Choose your companion's look or import a new one.|デスクトップの仲間の外観を選ぶか、新しいモデルを追加します。
行为与互动|Behavior|ふるまいと交流
调整小伙伴的活动习惯，了解互动方式。|Choose how your companion behaves and interacts.|仲間の行動や、ふれあい方を設定します。
桌面与显示|Desktop & display|デスクトップと表示
让桌宠以合适的大小，待在合适的位置。|Set your companion's size, screen and window position.|仲間のサイズ、表示画面、ウィンドウ位置を設定します。
动作与适配|Motion & mapping|モーションと適用
识别角色能力，让每个动作都有合适的表现。|Preview animations and customize action mappings.|アニメーションを確認し、動作の割り当てを調整します。
性能与画质|Performance|パフォーマンス
调整清晰度、帧率与画面细节。|Adjust resolution, frame rate and visual detail.|解像度、フレームレート、描画の細かさを調整します。
你的桌面伙伴|Your desktop companion|デスクトップの仲間
查找设置|Find a setting|設定を検索
搜索外观、置顶、分辨率等设置|Search appearance, always on top, resolution and more|外観、最前面、解像度などの設定を検索
没有匹配的设置|No matching settings|一致する設定がありません
界面语言|Display language|表示言語
退出桌宠|Exit|終了
更改自动保存|Changes saved automatically|変更は自動保存されます
没有找到匹配项。\n请尝试“置顶”“外观”或“帧率”。|No matching settings.\nTry "top", "appearance" or "FPS".|一致する設定がありません。\n「最前面」「外観」「FPS」などをお試しください。
设置即时生效|Settings apply immediately|設定はすぐに反映されます
当前伙伴|Your companion|現在の仲間
自定义桌宠外观，让它更合你的心意。|Make your companion feel more like you.|仲間の外観を好みに合わせて変えられます。
选择外观|Choose appearance|外観を選択
导入新外观|Import appearance|外観をインポート
外观文件夹|Open skins folder|外観フォルダー
直接选择 3D 模型、图片、GIF 或 skin.json。支持 GLB / glTF / FBX / OBJ / VRM；Blender、STL、PLY 文件可自动转换。|Select a 3D model, image, GIF or skin.json. Supports GLB / glTF / FBX / OBJ / VRM, with automatic conversion for Blender, STL and PLY.|3D モデル、画像、GIF、skin.json を選択できます。GLB / glTF / FBX / OBJ / VRM に対応し、Blender、STL、PLY は自動変換できます。
日常活动|Daily activity|日常の行動
自由散步|Roaming|自由に散歩
开启后，桌宠会随机走路、小跑、停留和休息。|Let your companion walk, run, pause and rest on its own.|オンにすると、仲間が自由に歩いたり、小走りしたり、休んだりします。
和它互动|Interact with your companion|仲間とふれあう
轻点|Pet|なでる
摸摸它，看看小小的回应。|Give it a little pat and watch its response.|軽くなでると、小さな反応を見せてくれます。
左键单击|Left click|左クリック
提起与抛出|Pick up & throw|持ち上げる・投げる
松手后沿拖动方向滑行，再落到屏幕底部。|Release after dragging to let it glide and fall to the bottom of the screen.|ドラッグして離すと、その方向に滑り、画面の下へ着地します。
拖动后松手|Drag and release|ドラッグして離す
快捷菜单|Quick settings|クイック設定
快速切换外观、大小、散步和置顶。|Quickly change appearance, size, roaming and always on top.|外観、サイズ、散歩、最前面表示をすばやく変更できます。
右键单击|Right click|右クリック
关闭自由散步后，点击和拖动互动仍然可用。|Clicking and dragging still work when roaming is off.|自由に散歩をオフにしても、クリックやドラッグでふれあえます。
桌宠始终置顶|Always on top|常に最前面に表示
开启后，桌宠显示在其他普通窗口上方；关闭后，其他窗口可以遮挡桌宠。|Keep your companion above other normal windows. Turn off to let other windows cover it.|オンにすると、仲間をほかの通常ウィンドウより前に表示します。オフにすると、ほかのウィンドウの後ろにも表示されます。
所在显示器|Display|表示するディスプレイ
选择显示器可立即移动桌宠。也可直接拖过屏幕边缘；相邻工作区之间支持惯性跨屏和散步。断开显示器后会自动找回。|Choose a display to move your companion, or drag it across a screen edge. It can roam and glide between adjoining work areas, and returns automatically if a display disconnects.|ディスプレイを選ぶと仲間を移動できます。画面端を越えてドラッグすることもでき、隣接する作業領域の間を散歩したり滑ったりできます。画面の接続が切れると自動で戻ります。
拖动滑块实时调节；也可输入百分比后按 Enter 应用。|Drag the slider, or enter a percentage and press Enter.|スライダーで調整するか、割合を入力して Enter キーで適用します。
窗口与位置|Window & position|ウィンドウと位置
找回桌宠|Bring companion back|仲間を呼び戻す
将它移回主屏幕右下角。|Move your companion to the bottom right of the primary display.|仲間をメイン画面の右下へ戻します。
重置位置|Reset position|位置をリセット
位置会自动保存，下次启动时继续待在原来的地方。|The position is saved automatically and restored next time.|位置は自動保存され、次回起動時にも引き継がれます。
每种外观，独立适配|Settings for each appearance|外観ごとの設定
模型转换器|Model converter|モデル変換
GLB、glTF、FBX、OBJ 和 VRM 无需外部软件。导入 .blend、STL、PLY 需要本机安装 Blender 4.2+，程序会自动查找，也可手动选择。|GLB, glTF, FBX, OBJ and VRM need no external software. Importing .blend, STL or PLY requires Blender 4.2+; it is detected automatically or can be selected below.|GLB、glTF、FBX、OBJ、VRM は外部ソフト不要です。.blend、STL、PLY のインポートには Blender 4.2 以降が必要です。自動検出されない場合は、下で選択できます。
blender.exe 路径|Path to blender.exe|blender.exe のパス
浏览|Browse|参照
Blender 程序|Blender application|Blender アプリケーション
选择 Blender 程序|Select Blender|Blender を選択
平面图片使用整体动作；骨骼与形态键使用模型自带动画。Live2D / Spine 工程请先导出为 GIF、精灵图或序列帧。VRM 当前保留基础网格、材质和骨架，不包含专用表情、MToon 和弹簧骨物理。|Images use whole-body motion; rigs and shape keys use the model's animations. Export Live2D / Spine projects as GIFs, sprite sheets or image sequences first. VRM keeps basic meshes, materials and rigs, without dedicated expressions, MToon or spring-bone physics.|画像には全体の動きを、ボーンやシェイプキーにはモデルのアニメーションを使用します。Live2D / Spine は GIF、スプライトシート、連番画像に書き出してください。VRM は基本メッシュ、マテリアル、ボーンに対応し、専用表情、MToon、スプリングボーン物理は含みません。
请选择有效的 blender.exe 文件。|Select a valid blender.exe file.|有効な blender.exe を選択してください。
%s · %d 个动画 · %d 根骨骼\n已匹配 %d / %d 个动作，其余由通用模组完成。\n在下方逐项预览或调整。|%s · %d animations · %d bones\n%d / %d actions mapped; the rest use generic motion.\nPreview or adjust each action below.|%s · アニメーション %d 個 · ボーン %d 本\n%d / %d 動作を割り当て済み。残りは汎用モーションです。\n下で各動作を確認・調整できます。
外观|Appearance|外観
%s · %d 个动画 · %d 根骨骼\n已匹配 %d / %d 个动作，其余使用通用模组。|%s · %d animations · %d bones\n%d / %d actions mapped; the rest use generic motion.|%s · アニメーション %d 個 · ボーン %d 本\n%d / %d 動作を割り当て済み。残りは汎用モーションです。
已识别 %d 个骨骼部位，缺少专用动作时可叠加轻量肢体动作。|%d rig parts detected. Lightweight limb motion can fill in missing animations.|%d か所のボーン部位を検出。専用アニメーションがない場合は、軽い手足の動きを補えます。
已启用 %d 处软体效果：身体自然下沉、腹部承托、移动惯性与弹性回稳。|%d soft-body effects enabled: natural sag, belly support, inertia and elastic settling.|%d か所のソフトボディ効果が有効です。自然なたわみ、腹部の支え、慣性、弾力のある復元を表現します。
已识别 %d 个眨眼 / 微笑形态键，可在通用动作中使用。|%d blink / smile shape keys detected for generic motion.|まばたき・笑顔のシェイプキーを %d 個検出。汎用モーションで使用できます。
动作风格|Motion style|モーションの種類
当前风格：%s。人物使用更轻的形变，软团使用更明显的弹性；自带动作播放时会减弱叠加形变。|Current style: %s. Characters use subtle deformation; plush shapes use more elasticity. Extra deformation is reduced while native animations play.|現在の種類：%s。キャラクターは控えめに、柔らかい形のモデルは弾力を強めに変形します。モデル固有のアニメーション再生中は、追加の変形を弱めます。
动作匹配与预览|Action mapping & preview|動作の割り当てとプレビュー
自动 · %s|Auto · %s|自動 · %s
通用动作|Generic motion|汎用モーション
通用模组|Generic motion|汎用モーション
%s：%s|%s: %s|%s：%s
预览|Preview|プレビュー
跟随行走方向翻转|Flip with movement direction|移動方向に合わせて反転
左右移动时，图片外观跟随方向翻转。|Flip an image appearance when the companion changes direction.|左右へ移動すると、画像の外観も移動方向に合わせて反転します。
站立朝向（度）|Standing orientation (degrees)|立ち姿の向き（度）
恢复此角色的默认适配|Reset this appearance's mapping|この外観の設定をリセット
适配设置保存失败。|Could not save action mapping.|動作の設定を保存できませんでした。
正在导入…|Importing…|インポート中…
正在读取模型 / 图片并匹配动作。转换期间桌宠可以继续互动。|Reading the model / image and mapping actions. You can keep interacting while it converts.|モデル・画像を読み込み、動作を割り当てています。変換中も仲間とふれあえます。
模型、图片或外观包|Models, images or skin packs|モデル・画像・外観パック
3D 模型|3D model|3D モデル
图片与动图|Images and animations|画像とアニメーション
外观清单|Skin manifest|外観マニフェスト
选择模型、图片或外观包|Select a model, image or skin pack|モデル・画像・外観パックを選択
搜索设置|Search settings|設定の検索
试试其他关键词，或清空搜索框查看全部设置。|Try another keyword, or clear the search to see all settings.|別のキーワードを試すか、検索欄を空にしてすべての設定を表示します。
%d 个可用外观|%d appearances available|%d 個の外観が利用可能
2D 图片 / 动画|2D image / animation|2D 画像・アニメーション
3D 模型外观|3D model|3D モデル
部分外观未加载：|Some appearances could not load: |一部の外観を読み込めませんでした：
一键方案|Presets|プリセット
根据设备和使用场景选择，也可逐项调整。|Choose a preset for your device, then fine-tune individual options.|端末や使い方に合う設定を選び、各項目を調整できます。
省电 · 30 FPS / 低画质|Power saving · 30 FPS / Low|省電力 · 30 FPS / 低画質
均衡 · 60 FPS / 标准画质|Balanced · 60 FPS / Standard|バランス · 60 FPS / 標準画質
高画质 · 60 FPS / 精细阴影|High quality · 60 FPS / Shadows|高画質 · 60 FPS / 精細な影
自定义|Custom|カスタム
渲染与帧率|Rendering & frame rate|描画とフレームレート
内部渲染分辨率|Render resolution|内部描画解像度
提高分辨率可改善大尺寸桌宠的细节；不会改变桌宠大小或脚底位置。|Higher resolution adds detail at larger sizes. It does not change the companion's size or foot position.|解像度を上げると、仲間を大きく表示したときの細部が鮮明になります。仲間のサイズや足元の位置は変わりません。
帧率上限|Frame rate limit|フレームレート上限
不限帧率|Unlimited|制限なし
垂直同步|VSync|垂直同期
垂直同步开启时，实际帧率还受显示器刷新率限制。|VSync also limits frame rate to the display's refresh rate.|垂直同期をオンにすると、実際のフレームレートは画面のリフレッシュレートにも制限されます。
画面细节|Visual detail|描画の詳細
画质|Quality|画質
低 · 简化短绒|Low · Reduced fur|低 · 毛並みを簡略化
标准 · 完整短绒|Standard · Full fur|標準 · 毛並みをすべて表示
高 · 短绒与柔和阴影|High · Fur and soft shadows|高 · 毛並みと柔らかい影
抗锯齿|Antialiasing|アンチエイリアス
关闭|Off|オフ
FXAA · 较低开销|FXAA · Low overhead|FXAA · 低負荷
环境光遮蔽（SSAO）|Ambient occlusion (SSAO)|環境遮蔽（SSAO）
增强脚部、背鳍根部及褶皱附近的接触层次，会增加 GPU 开销。|Adds contact shading around feet, fin roots and folds, at additional GPU cost.|足元、背びれの付け根、しわなどの陰影を強めます。GPU の負荷が増えます。
当前使用兼容渲染，环境光遮蔽不可用。Windows 高级渲染支持此效果。|Ambient occlusion is unavailable in Compatibility rendering. Advanced rendering on Windows supports this effect.|互換レンダラーでは環境遮蔽を使用できません。Windows の高度な描画で利用できます。
运行状态|Runtime status|動作状況
所有选项即时应用并自动保存；外观预览共用相同画质设置。|All options apply immediately and save automatically. Previews use the same quality settings.|設定はすぐに反映され、自動保存されます。外観のプレビューにも同じ画質設定が適用されます。
当前 %d FPS · 渲染 %d × %d\n%s|%d FPS · Rendering at %d × %d\n%s|現在 %d FPS · 描画 %d × %d\n%s
高级渲染 · 支持环境光遮蔽|Advanced rendering · Ambient occlusion supported|高度な描画 · 環境遮蔽に対応
兼容渲染|Compatibility rendering|互換描画
开|On|オン
关|Off|オフ
大小调节|Adjust size|サイズ調整
桌宠大小|Companion size|仲間のサイズ
输入百分比后按 Enter，或点击其他位置应用|Enter a percentage and press Enter, or click elsewhere to apply|割合を入力し、Enter キーを押すか、ほかの場所をクリックして適用
恢复 100%|Reset to 100%|100% に戻す
打开设置|Open settings|設定を開く
切换外观|Change appearance|外観を切り替え
移到显示器|Move to display|ディスプレイへ移動
桌宠大小 · %d%%…|Companion size · %d%%…|仲間のサイズ · %d%%…
性能与画质…|Performance…|パフォーマンス…
左键打开设置，右键快速设置|Left click for settings; right click for quick settings|左クリックで設定、右クリックでクイック設定
当前状态：|Current activity: |現在の動作：
发呆|Idle|待機
走路|Walking|歩く
跑步|Running|走る
打盹|Sleeping|居眠り
被摸摸|Being petted|なでられ中
被提起|Picked up|持ち上げられ中
回弹|Bouncing|着地の弾み
惯性 / 下落|Gliding / falling|滑る・落下
显示器 %d%s · %d × %d|Display %d%s · %d × %d|ディスプレイ %d%s · %d × %d
（主屏）| (Primary)|（メイン）
主屏幕|Primary display|メイン画面
正在导入外观，完成后即可退出。|An appearance is being imported. You can exit when it finishes.|外観のインポート中です。完了後に終了できます。
找不到外观包：|Skin pack not found: |外観パックが見つかりません：
外观加载失败，保留当前外观：|Could not load appearance; keeping the current one: |外観を読み込めないため、現在の外観を維持します：
外观加载失败：|Could not load appearance: |外観を読み込めませんでした：
无法启动导入任务：|Could not start import: |インポートを開始できません：
导入失败：|Import failed: |インポートに失敗しました：
设置保存失败：|Could not save settings: |設定を保存できませんでした：
待机|Idle|待機
睡眠|Sleep|睡眠
摸摸回应|Pet response|なでたときの反応
提起|Pick up|持ち上げる
空中 / 下落|Airborne / falling|空中・落下
落地|Landing|着地
自动识别|Auto-detect|自動検出
龙泡泡 / 软团|Dragon / plush|ドラゴン・ぬいぐるみ
Q 版人物|Chibi character|デフォルメキャラ
图片 / 平面角色|Image / flat character|画像・平面キャラ
手动指定|Manual mapping|手動で指定
黍泡泡|Shu Bubble|シューバブル
奶白团子 · 3D 框架演示|Milky Mochi · 3D demo|ミルクもち · 3D デモ
薄荷团子 · 3D 框架演示|Mint Mochi · 3D demo|ミントもち · 3D デモ
橘子团子 · PNG 外观演示|Orange Mochi · PNG demo|みかんもち · PNG デモ
蓝莓球 · GLB 动画演示|Blueberry · GLB demo|ブルーベリー · GLB デモ
2D 动画|2D animation|2D アニメーション
静态图片|Still image|静止画像
无法创建外观目录|Could not create the skin folder|外観フォルダーを作成できません
文件不包含可显示的网格|The file contains no visible mesh|表示できるメッシュがありません
找不到图片：|Image not found: |画像が見つかりません：
图片文件合计超过 256 MB|Image files exceed 256 MB in total|画像の合計サイズが 256 MB を超えています
图片复制失败|Could not copy images|画像をコピーできません
清单写入失败|Could not write the manifest|マニフェストを書き込めません
无法读取资源或文件超过 256 MB|The asset could not be read or exceeds 256 MB|リソースを読み込めないか、256 MB を超えています
需要 glTF 2.0 二进制文件|A binary glTF 2.0 file is required|glTF 2.0 バイナリ形式が必要です
GLB 头部或 JSON 块无效|Invalid GLB header or JSON chunk|GLB ヘッダーまたは JSON チャンクが無効です
glTF JSON 无效|Invalid glTF JSON|glTF の JSON が無効です
glTF 资源列表无效|Invalid glTF asset list|glTF のリソース一覧が無効です
glTF 资源条目无效|Invalid glTF asset entry|glTF のリソース項目が無効です
glTF URI 必须是字符串|glTF URI must be a string|glTF の URI は文字列で指定してください
纹理与缓冲区需位于模型目录内，不支持绝对路径、上级目录或网络 URI|Textures and buffers must be inside the model folder; absolute paths, parent folders and network URIs are not supported|テクスチャとバッファはモデルのフォルダー内に置いてください。絶対パス、上位フォルダー、ネットワーク URI には対応していません
缺少模型依赖：|Missing model dependency: |モデルの依存ファイルがありません：
模型与依赖合计超过 256 MB|The model and its dependencies exceed 256 MB|モデルと依存ファイルの合計が 256 MB を超えています
模型解码失败：|Could not decode the model: |モデルをデコードできません：
未生成有效模型场景|No valid model scene was created|有効なモデルシーンを生成できませんでした
模型标准化失败：|Could not normalize the model: |モデルを標準化できません：
glTF 缓冲区被截断|The glTF buffer is truncated|glTF のバッファが途中で切れています
glTF 缓冲视图越界|The glTF buffer view is out of bounds|glTF のバッファビューが範囲外です
标准化模型超过 256 MB|The normalized model exceeds 256 MB|標準化後のモデルが 256 MB を超えています
无法写入模型文件|Could not write the model file|モデルファイルを書き込めません
FBX 读取失败：|Could not read FBX: |FBX を読み込めません：
FBX 未生成有效场景|FBX did not produce a valid scene|FBX から有効なシーンを生成できませんでした
带形态键的 FBX 需要图形渲染器进行转换，请正常启动桌宠后导入。|FBX files with shape keys require a graphical renderer. Start the companion normally before importing.|シェイプキーを含む FBX の変換には、グラフィカルレンダラーが必要です。通常起動したアプリでインポートしてください。
此格式需要 Blender 4.2+。请在“动作与适配”中设置 blender.exe 路径，或导出为 GLB 后导入。|This format requires Blender 4.2+. Set the blender.exe path under Motion & mapping, or export to GLB first.|この形式には Blender 4.2 以降が必要です。「モーションと適用」で blender.exe のパスを設定するか、GLB に書き出してからインポートしてください。
无法写入模型转换脚本|Could not write the model conversion script|モデル変換スクリプトを書き込めません
Blender 转换失败，详情见运行数据目录中的 conversion-last.log|Blender conversion failed. See conversion-last.log in the app data folder.|Blender での変換に失敗しました。アプリのデータフォルダーにある conversion-last.log を確認してください。
GIF 头部无效|Invalid GIF header|GIF ヘッダーが無効です
GIF 尺寸需在 1–4096 内|GIF dimensions must be between 1 and 4096|GIF のサイズは 1～4096 にしてください
GIF 控制块无效|Invalid GIF control block|GIF の制御ブロックが無効です
GIF 图像块无效|Invalid GIF image block|GIF の画像ブロックが無効です
GIF 帧超出画布|A GIF frame is outside the canvas|GIF のフレームがキャンバスの範囲外です
GIF 超过 256 帧或解码后超过 128 MB，请先压缩|The GIF exceeds 256 frames or 128 MB decoded. Compress it first.|GIF が 256 フレーム、または展開後 128 MB を超えています。先に圧縮してください。
GIF LZW 数据无效或被截断|Invalid or truncated GIF LZW data|GIF の LZW データが無効か、途中で切れています
GIF 调色板索引越界|The GIF palette index is out of bounds|GIF のパレット番号が範囲外です
GIF 文件不完整或没有图像帧|The GIF is incomplete or contains no frames|GIF が不完全か、画像フレームがありません
OBJ 无法读取或超过 256 MB|The OBJ could not be read or exceeds 256 MB|OBJ を読み込めないか、256 MB を超えています
OBJ 顶点格式无效|Invalid OBJ vertex format|OBJ の頂点形式が無効です
OBJ 包含无效坐标|The OBJ contains invalid coordinates|OBJ に無効な座標が含まれています
MTL 必须位于模型目录内|The MTL must be inside the model folder|MTL はモデルのフォルダー内に置いてください
OBJ 面顶点数量无效（3–512）|Invalid OBJ face vertex count (3–512)|OBJ の面の頂点数が無効です（3～512）
OBJ 索引越界|An OBJ index is out of bounds|OBJ の番号が範囲外です
OBJ 含无法三角化的面|The OBJ has a face that cannot be triangulated|OBJ に三角形化できない面があります
OBJ 超过一百万顶点或三角面，请先简化|The OBJ exceeds one million vertices or triangles. Simplify it first.|OBJ が 100 万頂点または三角形を超えています。先に簡略化してください。
OBJ 没有可显示的面|The OBJ contains no visible faces|OBJ に表示できる面がありません
MTL 缺失或超过 1 MB|The MTL is missing or exceeds 1 MB|MTL が見つからないか、1 MB を超えています
OBJ 贴图必须位于 MTL 目录内；暂不支持 map_Kd 命令选项|OBJ textures must be inside the MTL folder. map_Kd command options are not supported.|OBJ のテクスチャは MTL のフォルダー内に置いてください。map_Kd のコマンドオプションには未対応です。
无法读取 OBJ 贴图或贴图尺寸超过 8192|An OBJ texture could not be read or exceeds 8192 pixels|OBJ のテクスチャを読み込めないか、サイズが 8192 を超えています
: 外观 ID 重复|: Duplicate skin ID|：外観 ID が重複しています
缺少 skin.json|Missing skin.json|skin.json がありません
清单超过 64 KB|The manifest exceeds 64 KB|マニフェストが 64 KB を超えています
清单必须为 JSON 对象|The manifest must be a JSON object|マニフェストは JSON オブジェクトにしてください
仅支持 schema_version: 1|Only schema_version: 1 is supported|schema_version: 1 のみ対応しています
 必须是非空字符串| must be a non-empty string| は空でない文字列にしてください
id 仅允许小写字母、数字、点、横线、下划线，最长 80 字符|The ID must be at most 80 characters: lowercase letters, numbers, dots, hyphens or underscores|ID は 80 文字以内の英小文字、数字、ピリオド、ハイフン、アンダースコアで指定してください
type 必须为 sprite 或 model3d|type must be sprite or model3d|type は sprite または model3d にしてください
procedural 仅供内置演示使用|procedural is reserved for built-in demos|procedural は内蔵デモ専用です
animations 必须为对象|animations must be an object|animations はオブジェクトにしてください
动画映射值必须是字符串|Animation mappings must be strings|アニメーションの割り当ては文字列で指定してください
asset 必须为包内相对路径，不允许 .. 或绝对路径|asset must be a relative path inside the pack; .. and absolute paths are not allowed|asset はパック内の相対パスにしてください。.. や絶対パスは使用できません
不支持的外观资源格式|Unsupported skin asset format|対応していない外観リソース形式です
找不到资源: |Asset not found: |リソースが見つかりません：
资源超过 256 MB|The asset exceeds 256 MB|リソースが 256 MB を超えています
rotation_degrees 必须为三个数|rotation_degrees must contain three numbers|rotation_degrees には 3 つの数値を指定してください
rotation_degrees 必须为有限数值|rotation_degrees must contain finite numbers|rotation_degrees は有限の数値にしてください
不支持此文件格式。Live2D / Spine 专用工程暂需导出为精灵图或 GIF。|Unsupported format. Export Live2D / Spine projects as sprite sheets or GIFs first.|この形式には対応していません。Live2D / Spine のプロジェクトは、先にスプライトシートや GIF に書き出してください。
无法读取文件或文件超过 256 MB|The file could not be read or exceeds 256 MB|ファイルを読み込めないか、256 MB を超えています
外观 ID 已存在，请修改 ID 后导入|This skin ID already exists. Change the ID before importing.|外観 ID がすでに存在します。ID を変更してからインポートしてください。
目标文件夹已存在，请在外观目录检查同名包|The destination folder already exists. Check for a pack with the same name.|保存先フォルダーがすでに存在します。同名の外観パックを確認してください。
profile 必须为 auto、blob、character 或 flat|profile must be auto, blob, character or flat|profile は auto、blob、character、flat のいずれかにしてください
flip_with_direction 必须为布尔值|flip_with_direction must be a boolean|flip_with_direction は真偽値にしてください
grid 必须为 [列数, 行数]|grid must be [columns, rows]|grid は [列数, 行数] にしてください
精灵网格必须为 1–256 的整数|Sprite grid dimensions must be integers from 1 to 256|スプライトの行数・列数は 1～256 の整数にしてください
精灵网格最多 256 帧|A sprite grid may contain at most 256 frames|スプライトグリッドは最大 256 フレームです
frames 需要 1–256 个相对图片路径|frames requires 1–256 relative image paths|frames には 1～256 個の相対画像パスを指定してください
序列帧必须使用包内相对图片路径|Image sequence paths must be relative to the pack|連番画像はパック内の相対パスで指定してください
clips 必须为动作定义对象（最多 128 个）|clips must be an action definition object (at most 128 entries)|clips は動作定義のオブジェクトにしてください（最大 128 項目）
每个 2D 动作需要 frames 整数索引列表|Each 2D action requires a frames list of integer indices|各 2D 動作には、整数番号の frames リストが必要です
2D 帧索引必须为 0–255 的整数|2D frame indices must be integers from 0 to 255|2D フレーム番号は 0～255 の整数にしてください
2D 动作 fps 需要在 0–120 之间|2D animation fps must be between 0 and 120|2D 動作の fps は 0～120 にしてください
loop 必须为布尔值|loop must be a boolean|loop は真偽値にしてください
durations 必须与动作帧数一致|durations must match the number of animation frames|durations の数は動作のフレーム数に合わせてください
帧时长必须为 0–60 秒之间的有限数|Frame durations must be finite values from 0 to 60 seconds|フレームの長さは 0～60 秒の有限数にしてください
physics 需要对象，soft_bodies 最多 8 个软体部件|physics must be an object; soft_bodies supports at most 8 parts|physics はオブジェクトにしてください。soft_bodies は最大 8 部位です
软体部件需要对象|A soft-body part must be an object|ソフトボディの部位はオブジェクトにしてください
软体 %s 需要模型内部名称或相对节点路径|Soft-body %s requires an internal model name or relative node path|ソフトボディの %s には、モデル内の名前または相対ノードパスが必要です
软体 node 必须为不重复的模型内部路径|Soft-body node paths must be unique and inside the model|ソフトボディの node は、重複しないモデル内部のパスにしてください
软体 anchors 最多 32 个承托节点|Soft-body anchors supports at most 32 support nodes|ソフトボディの anchors は最大 32 ノードです
软体 anchors 需要模型内部名称或相对节点路径|Soft-body anchors requires internal names or relative node paths|ソフトボディの anchors には、モデル内の名前または相対ノードパスが必要です
软体 axis 需要三个有限数值|Soft-body axis requires three finite numbers|ソフトボディの axis には、3 つの有限数値が必要です
软体 axis 需要非零的水平向量（模型局部 Y 向上）|Soft-body axis must be a nonzero horizontal vector (model-local Y is up)|ソフトボディの axis はゼロでない水平ベクトルにしてください（モデルのローカル Y が上方向）
软体 sections 需要 4–48 的整数|Soft-body sections must be an integer from 4 to 48|ソフトボディの sections は 4～48 の整数にしてください
软体缺少 %s|Soft-body is missing %s|ソフトボディに %s がありません
软体 %s 超出允许范围|Soft-body %s is outside the allowed range|ソフトボディの %s が許容範囲外です
软体 floor_height 不能高于 support_height|Soft-body floor_height must not exceed support_height|ソフトボディの floor_height は support_height 以下にしてください
内置 GLB 场景读取失败|Could not read the built-in GLB scene|内蔵 GLB シーンを読み込めません
GLB 未生成有效的 3D 场景|The GLB did not produce a valid 3D scene|GLB から有効な 3D シーンを生成できませんでした
GLB 缺少有效网格尺寸|The GLB has no valid mesh bounds|GLB に有効なメッシュの寸法がありません
序列帧无法读取或尺寸超过 8192：|An image frame could not be read or exceeds 8192 pixels: |連番画像を読み込めないか、サイズが 8192 を超えています：
图片无法读取、内容为空或尺寸超过 8192 × 8192|The image could not be read, is empty or exceeds 8192 × 8192|画像を読み込めないか、内容が空か、8192 × 8192 を超えています
精灵图尺寸必须能被网格列数和行数整除|Sprite sheet dimensions must be divisible by the grid's columns and rows|スプライトシートのサイズは、グリッドの行数・列数で割り切れる値にしてください
需要 1–256 张图片帧|Between 1 and 256 image frames are required|1～256 枚の画像フレームが必要です
序列帧为空或解码后超过 128 MB|The image sequence is empty or exceeds 128 MB decoded|連番画像が空か、展開後 128 MB を超えています
动作 %s 的帧索引超出图片范围|The frame index for action %s is outside the image range|動作 %s のフレーム番号が画像の範囲外です
唤醒|Wake up|起こす
一键睡觉|Sleep now|今すぐ寝る
一键睡觉后持续休息，点击、拖动或唤醒即可结束|Keep sleeping until clicked, dragged, or woken up|クリック・ドラッグ・起こす操作まで眠り続けます
桌面窗口捕捉|Stand on windows|ウィンドウの上に立つ
落在可见窗口顶边；窗口关闭或最小化后自然下落|Land on visible window tops; fall when a window closes or minimizes|表示中のウィンドウ上辺に着地し、閉じる・最小化で落下します
待机原地转圈|Idle twirl|待機中の回転
待机时偶尔转一圈，保持原地站立|Occasionally turn around in place while idle|待機中にその場で一回転します
动作时间|Activity timing|動作の時間
各动作持续时间（秒），自动活动有少量随机变化|Duration in seconds, with slight variation during autonomous activity|各動作の秒数。自動動作には少し変化が加わります
待机间隔|Idle interval|待機時間
走路时长|Walking duration|歩く時間
跑步时长|Running duration|走る時間
自动睡眠时长|Automatic sleep duration|自動睡眠の時間
转圈间隔|Twirl interval|回転の間隔
恢复默认动作时间|Reset activity timing|動作時間を初期化
查看此版本|View this release|このリリースを見る
下载 Windows 版本|Download for Windows|Windows 版をダウンロード
更新说明|Release notes|リリースノート
GitHub 项目|GitHub project|GitHub プロジェクト
无互动后入睡|Sleep after no interaction|操作しないと眠る
站起扒屏|Stand and paw at screen|立って画面に前足をつく
界面主题|Appearance theme|表示テーマ
深色、浅色或跟随系统自动切换|Light, dark, or follow the system|ライト、ダーク、システムに合わせる
跟随系统|Follow system|システムに合わせる
浅色|Light|ライト
深色|Dark|ダーク
每天自动检查一次，发现新版本时在此提醒|Checks daily and shows new releases here|毎日確認し、新しいバージョンをここで通知します
下载并更新…|Download and update…|ダウンロードして更新…
确认更新|Confirm update|更新の確認
从 GitHub 下载并校验安装包，保存设置后退出并安装更新。安装完成后可重新启动。|Download and verify the GitHub installer, save settings, then exit to install the update.|GitHub のインストーラーを検証し、設定保存後に終了して更新します。
确认下载并更新|Confirm download and update|ダウンロードと更新を確認
"""


def rows():
    result = {}
    for line in DATA.strip("\n").splitlines():
        source, english, japanese = (part.replace(r"\n", "\n") for part in line.split("|"))
        if source in result:
            raise ValueError(f"Duplicate translation key: {source}")
        expected = re.findall(r"%(?:%|[dsf])", source)
        for text in [english, japanese]:
            if not text or re.findall(r"%(?:%|[dsf])", text) != expected:
                raise ValueError(f"Mismatched translation placeholders: {source!r}")
        result[source] = (english, japanese)
    return result


def main():
    root = Path(__file__).resolve().parents[1]
    target = root / "i18n"
    target.mkdir(exist_ok=True)
    entries = rows()
    for index, locale in enumerate(["en", "ja"]):
        header = f"Language: {locale}\nContent-Type: text/plain; charset=UTF-8\nMIME-Version: 1.0\nContent-Transfer-Encoding: 8bit\n"
        lines = ['# My Dragon Bubble display translations.', 'msgid ""', 'msgstr ' + json.dumps(header), '']
        for source, values in entries.items():
            lines += ['msgid ' + json.dumps(source, ensure_ascii=False), 'msgstr ' + json.dumps(values[index], ensure_ascii=False), '']
        (target / f"{locale}.po").write_text("\n".join(lines), encoding="utf-8")
    print(f"Translation catalogs: {len(entries)} complete entries per language")


if __name__ == "__main__":
    main()
