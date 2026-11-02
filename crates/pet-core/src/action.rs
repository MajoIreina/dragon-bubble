use crate::behavior::State;
use regex::Regex;
use std::collections::HashMap;
pub fn words(value: &str) -> Vec<String> {
    let camel = Regex::new(r"([a-z])([A-Z])")
        .unwrap()
        .replace_all(value, "$1 $2")
        .to_lowercase();
    let numbered = Regex::new(r"([a-z])([0-9])")
        .unwrap()
        .replace_all(&camel, "$1 $2");
    Regex::new(r"[^a-z0-9\u{4e00}-\u{9fff}]+")
        .unwrap()
        .split(&numbered)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect()
}
const ALIASES: [(&str, &[&str]); 8] = [
    (
        "idle",
        &[
            "idle",
            "stand",
            "standing",
            "breathe",
            "breathing",
            "rest",
            "待机",
            "站立",
            "呼吸",
            "休息",
        ],
    ),
    (
        "walk",
        &["walk", "walking", "locomotion", "散步", "走路", "行走"],
    ),
    (
        "run",
        &[
            "run",
            "running",
            "sprint",
            "sprinting",
            "jog",
            "jogging",
            "跑步",
            "奔跑",
            "小跑",
        ],
    ),
    (
        "sleep",
        &["sleep", "sleeping", "nap", "doze", "睡觉", "睡眠", "打盹"],
    ),
    (
        "petted",
        &[
            "happy", "pet", "petted", "wave", "greet", "joy", "cheer", "开心", "摸摸", "挥手",
            "互动",
        ],
    ),
    (
        "dragged",
        &[
            "pickedup", "pickup", "grab", "held", "drag", "dragged", "提起", "抓起", "拖拽",
        ],
    ),
    (
        "airborne",
        &[
            "airborne", "falling", "float", "fly", "jump", "下落", "空中", "跳跃", "飞行",
        ],
    ),
    ("fall", &["land", "landing", "impact", "落地", "着地"]),
];
pub fn resolve(clips: &[String], explicit: &HashMap<String, String>) -> HashMap<String, String> {
    let mut sorted = clips.to_vec();
    sorted.sort();
    let mut out = HashMap::new();
    for (state, aliases) in ALIASES {
        let mut best = 0;
        for clip in &sorted {
            if clip == "RESET" {
                continue;
            }
            let tokens = words(clip);
            let compact = tokens.join("");
            for alias in aliases {
                let score = if compact == *alias {
                    100
                } else if tokens.iter().any(|t| t == alias) {
                    80
                } else if !alias.is_ascii() && compact.contains(alias) {
                    70
                } else if compact.ends_with(alias) && clip.contains([':', '/', '|']) {
                    60
                } else {
                    0
                };
                if score > best {
                    best = score;
                    out.insert(state.into(), clip.clone());
                }
            }
        }
    }
    if !out.contains_key("run") {
        if let Some(v) = out.get("walk").cloned() {
            out.insert("run".into(), v);
        }
    } else if !out.contains_key("walk")
        && let Some(v) = out.get("run").cloned()
    {
        out.insert("walk".into(), v);
    }
    for state in State::ALL {
        if let Some(clip) = explicit.get(state.key()) {
            out.remove(state.key());
            if clip != "RESET" && clips.contains(clip) {
                out.insert(state.key().into(), clip.clone());
            }
        }
    }
    out
}
pub fn state_key(state: State) -> &'static str {
    state.key()
}

/// Explicit profiles win; otherwise infer a conservative deformation style.
pub fn profile_for(
    kind: &str,
    size: crate::Vec3,
    bones: &[String],
    requested: &str,
) -> &'static str {
    match requested {
        "blob" => return "blob",
        "character" => return "character",
        "flat" => return "flat",
        _ => {}
    }
    if kind == "sprite" {
        return "flat";
    }
    let hints = bones
        .iter()
        .filter(|name| {
            let name = words(name).join("");
            [
                "hips", "pelvis", "spine", "head", "upperarm", "thigh", "小腿", "上臂", "脊柱",
            ]
            .iter()
            .any(|hint| name.contains(hint))
        })
        .count();
    if hints >= 2 || size.y > size.x.max(size.z) * 1.25 {
        "character"
    } else {
        "blob"
    }
}
