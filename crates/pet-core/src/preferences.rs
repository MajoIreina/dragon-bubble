use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderOptions {
    pub resolution: u32,
    pub fps: u32,
    pub quality: u8,
    pub aa: String,
    pub ao: bool,
    pub vsync: bool,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            resolution: 512,
            fps: 60,
            quality: 1,
            aa: "msaa4".into(),
            ao: false,
            vsync: true,
        }
    }
}
impl RenderOptions {
    pub const LEVELS: [&'static str; 5] = ["低", "中", "高", "超高", "极致"];
    pub const RESOLUTIONS: [u32; 5] = [256, 512, 768, 1024, 1536];

    /// Existing custom resolutions remain valid; show their closest named level.
    pub fn resolution_level(&self) -> usize {
        Self::RESOLUTIONS
            .iter()
            .enumerate()
            .min_by_key(|(_, value)| value.abs_diff(self.resolution))
            .map_or(1, |(index, _)| index)
    }

    pub fn shadow_map_size(&self) -> usize {
        [512, 512, 1024, 2048, 4096][self.quality.min(4) as usize]
    }

    pub fn from_value(v: &Value) -> Self {
        let mut d = Self::default();
        if let Some(n) = v["resolution"].as_f64()
            && n.fract() == 0.
            && (256.0..=1536.0).contains(&n)
        {
            d.resolution = n as u32
        }
        if let Some(n) = v["fps"].as_f64()
            && n.fract() == 0.
            && (n == 0. || (24.0..=240.0).contains(&n))
        {
            d.fps = n as u32
        }
        if let Some(n) = v["quality"].as_f64()
            && [0., 1., 2., 3., 4.].contains(&n)
        {
            d.quality = n as u8
        }
        if let Some(s) = v["aa"].as_str()
            && ["off", "fxaa", "msaa2", "msaa4", "msaa8", "taa"].contains(&s)
        {
            d.aa = s.into()
        }
        if let Some(b) = v["ao"].as_bool() {
            d.ao = b
        }
        if let Some(b) = v["vsync"].as_bool() {
            d.vsync = b
        }
        d
    }
    pub fn preset(name: &str) -> Self {
        match name {
            "power" => Self {
                resolution: 256,
                fps: 30,
                quality: 0,
                aa: "fxaa".into(),
                ..Self::default()
            },
            "high" => Self {
                resolution: 768,
                quality: 2,
                ao: true,
                ..Self::default()
            },
            "ultra" => Self {
                resolution: 1024,
                quality: 3,
                ao: true,
                ..Self::default()
            },
            "extreme" => Self {
                resolution: 1536,
                quality: 4,
                aa: "msaa8".into(),
                ao: true,
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
    pub skin_id: String,
    pub appearance_revision: u32,
    pub zoom: f32,
    pub roaming: bool,
    #[serde(default)]
    pub autostart: bool,
    pub always_on_top: bool,
    pub locale: String,
    pub position: [i32; 2],
    pub performance: RenderOptions,
    #[serde(default)]
    pub behavior: crate::behavior::BehaviorOptions,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            skin_id: "builtin.shu".into(),
            appearance_revision: 1,
            zoom: 1.,
            roaming: true,
            autostart: false,
            always_on_top: true,
            locale: "zh_CN".into(),
            position: [-99999, -99999],
            performance: RenderOptions::default(),
            behavior: Default::default(),
        }
    }
}
pub fn zoom(v: f32) -> f32 {
    if v.is_finite() {
        (v.clamp(0.2, 3.) * 100.).round() / 100.
    } else {
        1.
    }
}
pub fn user_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("LPP_DATA_DIR") {
        return p.into();
    }
    if cfg!(target_os = "macos") {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support/LPPDesktopPet");
    }
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("LPPDesktopPet")
}
pub fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)
}
impl Preferences {
    pub fn from_value(v: &Value) -> Self {
        let mut p = Self::default();
        if let Some(s) = v["skin_id"].as_str() {
            p.skin_id = s.into()
        }
        if v["appearance_revision"].as_u64() != Some(1)
            && [
                "builtin.mochi",
                "builtin.mint",
                "example.sprite",
                "example.glb",
            ]
            .contains(&p.skin_id.as_str())
        {
            p.skin_id = "builtin.shu".into()
        }
        if let Some(z) = v["zoom"].as_f64() {
            p.zoom = zoom(z as f32)
        }
        if let Some(b) = v["roaming"].as_bool() {
            p.roaming = b
        }
        if let Some(b) = v["always_on_top"].as_bool() {
            p.always_on_top = b
        }
        p.autostart = v["autostart"].as_bool().unwrap_or(false);
        p.locale = crate::i18n::normalize(v["locale"].as_str().unwrap_or("zh_CN"));
        p.performance = RenderOptions::from_value(&v["performance"]);
        p.behavior = crate::behavior::BehaviorOptions::from_value(&v["behavior"]);
        if let Some(pos) = v["position"].as_array()
            && pos.len() == 2
            && let (Some(x), Some(y)) = (pos[0].as_f64(), pos[1].as_f64())
        {
            p.position = [x as i32, y as i32]
        }
        p
    }
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|v| serde_json::from_str::<Value>(&v).ok())
            .map_or_else(Self::default, |v| Self::from_value(&v))
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        atomic_write(path, &serde_json::to_vec_pretty(self)?)
    }
}
pub fn overrides(skin: &crate::assets::Skin, path: &Path) -> crate::assets::Skin {
    let mut s = skin.clone();
    let v = fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or(json!({}));
    if let Some(entry) = v.get(&skin.id).and_then(Value::as_object) {
        let mut data = serde_json::to_value(skin).unwrap();
        for key in [
            "profile",
            "rotation_degrees",
            "animations",
            "flip_with_direction",
        ] {
            if let Some(v) = entry.get(key) {
                data[key] = v.clone()
            }
        }
        if let Ok(candidate) = serde_json::from_value::<crate::assets::Skin>(data)
            && crate::assets::validate(&candidate).is_ok()
        {
            s = candidate;
            s.root = skin.root.clone()
        }
    }
    s
}
pub fn save_override(id: &str, value: Option<Value>, path: &Path) -> anyhow::Result<()> {
    let mut all = fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(Value::is_object)
        .unwrap_or(json!({}));
    if let Some(v) = value {
        all[id] = v
    } else {
        all.as_object_mut().unwrap().remove(id);
    }
    atomic_write(path, &serde_json::to_vec_pretty(&all)?)?;
    Ok(())
}
