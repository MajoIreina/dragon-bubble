use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skin {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub asset: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub animations: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub rotation_degrees: [f32; 3],
    #[serde(default = "yes")]
    pub flip_with_direction: bool,
    #[serde(skip)]
    pub root: PathBuf,
    #[serde(flatten)]
    pub options: serde_json::Map<String, serde_json::Value>,
}
pub const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tga", "svg", "gif"];
pub const MODEL_EXT: &[&str] = &["glb", "gltf", "vrm", "fbx", "obj", "blend", "stl", "ply"];
pub fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(':')
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
}
pub fn read_manifest(path: &Path) -> Result<Skin> {
    if fs::metadata(path)?.len() > 65536 {
        bail!("清单超过 64 KB")
    }
    let mut s: Skin = serde_json::from_str(&fs::read_to_string(path)?)?;
    validate(&s)?;
    s.root = path.parent().unwrap().to_path_buf();
    if s.r#type != "procedural" {
        let asset = s.asset.as_deref().unwrap();
        let file = contained(&s.root, asset)?;
        if fs::metadata(file)?.len() > 256 * 1024 * 1024 {
            bail!("资源超过 256 MB")
        }
    }
    Ok(s)
}
pub fn contained(root: &Path, relative: &str) -> Result<PathBuf> {
    if !safe_relative(relative) {
        bail!("asset 必须为包内相对路径，不允许 .. 或绝对路径")
    }
    let root = root.canonicalize()?;
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(&root) {
        bail!("资源路径越界")
    }
    Ok(path)
}
pub fn validate(s: &Skin) -> Result<()> {
    if s.schema_version != 1 {
        bail!("仅支持 schema_version: 1")
    }
    if s.name.trim().is_empty()
        || !regex::Regex::new(r"^[a-z0-9][a-z0-9._-]{0,79}$")
            .unwrap()
            .is_match(&s.id)
    {
        bail!("id 仅允许小写字母、数字、点、横线、下划线，最长 80 字符")
    }
    if !["procedural", "sprite", "model3d"].contains(&s.r#type.as_str()) {
        bail!("type 必须为 sprite 或 model3d")
    }
    if s.r#type != "procedural" {
        let asset = s.asset.as_deref().unwrap_or("");
        if !safe_relative(asset) {
            bail!("asset 必须为包内相对路径，不允许 .. 或绝对路径")
        }
        let ext = extension(Path::new(asset));
        if (s.r#type == "sprite" && !IMAGE_EXT.contains(&ext.as_str()))
            || (s.r#type == "model3d" && !MODEL_EXT.contains(&ext.as_str()))
        {
            bail!("不支持的外观资源格式")
        }
    }
    if !["", "auto", "blob", "character", "flat"].contains(&s.profile.as_str()) {
        bail!("profile 必须为 auto、blob、character 或 flat")
    }
    if s.rotation_degrees.iter().any(|v| !v.is_finite()) {
        bail!("rotation_degrees 必须为有限数值")
    }
    if let Some(g) = s.options.get("grid") {
        let g = g
            .as_array()
            .filter(|g| g.len() == 2)
            .ok_or_else(|| anyhow::anyhow!("grid 必须为 [列数, 行数]"))?;
        if g.iter()
            .any(|v| v.as_u64().is_none_or(|x| x == 0 || x > 256))
            || g[0].as_u64().unwrap() * g[1].as_u64().unwrap() > 256
        {
            bail!("精灵网格最多 256 帧")
        }
    }
    if let Some(frames) = s.options.get("frames") {
        let frames = frames
            .as_array()
            .filter(|a| !a.is_empty() && a.len() <= 256)
            .ok_or_else(|| anyhow::anyhow!("frames 需要 1–256 个相对图片路径"))?;
        for f in frames {
            if !f.as_str().is_some_and(safe_relative) {
                bail!("序列帧必须使用包内相对图片路径")
            }
        }
    }
    if let Some(clips) = s.options.get("clips") {
        let clips = clips
            .as_object()
            .filter(|c| c.len() <= 128)
            .ok_or_else(|| anyhow::anyhow!("clips 必须为动作定义对象（最多 128 个）"))?;
        for clip in clips.values() {
            let f = clip
                .get("frames")
                .and_then(|f| f.as_array())
                .filter(|a| !a.is_empty() && a.len() <= 4096)
                .ok_or_else(|| anyhow::anyhow!("每个 2D 动作需要 frames 整数索引列表"))?;
            if f.iter().any(|v| v.as_u64().is_none_or(|v| v > 255)) {
                bail!("2D 帧索引必须为 0–255 的整数")
            }
            if let Some(fps) = clip.get("fps")
                && !fps.as_f64().is_some_and(|v| v > 0. && v <= 120.)
            {
                bail!("2D 动作 fps 需要在 0–120 之间")
            }
            if let Some(l) = clip.get("loop")
                && !l.is_boolean()
            {
                bail!("loop 必须为布尔值")
            }
            if let Some(d) = clip.get("durations") {
                let d = d
                    .as_array()
                    .filter(|d| d.len() == f.len())
                    .ok_or_else(|| anyhow::anyhow!("durations 必须与动作帧数一致"))?;
                if d.iter()
                    .any(|v| !v.as_f64().is_some_and(|v| v > 0. && v <= 60.))
                {
                    bail!("帧时长必须为 0–60 秒之间的有限数")
                }
            }
        }
    }
    if let Some(p) = s.options.get("physics") {
        let p = p
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("physics 需要对象，soft_bodies 最多 8 个软体部件"))?;
        if let Some(b) = p.get("soft_bodies") {
            let b = b.as_array().filter(|a| a.len() <= 8).ok_or_else(|| {
                anyhow::anyhow!("physics 需要对象，soft_bodies 最多 8 个软体部件")
            })?;
            let mut names = std::collections::HashSet::new();
            for body in b {
                let def: crate::soft_body::Definition = serde_json::from_value(body.clone())?;
                def.validate()?;
                if !names.insert(def.node) {
                    bail!("软体 node 必须为不重复的模型内部路径")
                }
            }
        }
    }
    Ok(())
}
pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}
pub fn scan(roots: &[PathBuf]) -> (Vec<Skin>, Vec<String>) {
    let mut skins = Vec::new();
    let mut errors = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path().join("skin.json");
            if !p.is_file() {
                continue;
            }
            match read_manifest(&p) {
                Ok(s) => {
                    if skins.iter().any(|x: &Skin| x.id == s.id) {
                        errors.push(format!("{}: duplicate id", p.display()))
                    } else {
                        skins.push(s)
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", p.display())),
            }
        }
    }
    skins.sort_by(|a, b| a.id.cmp(&b.id));
    (skins, errors)
}
