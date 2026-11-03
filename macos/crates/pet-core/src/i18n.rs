use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};
pub const LOCALES: [&str; 3] = ["zh_CN", "ja", "en"];
pub fn normalize(value: &str) -> String {
    match value
        .replace('-', "_")
        .split('_')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "en" => "en",
        "ja" => "ja",
        _ => "zh_CN",
    }
    .into()
}
fn parse(po: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut id = String::new();
    let mut value = String::new();
    let mut reading_value = false;
    for line in po.lines().chain(std::iter::once("")) {
        if line.trim().is_empty() {
            if !id.is_empty() {
                map.insert(id.clone(), value.clone());
            }
            id.clear();
            value.clear();
            reading_value = false;
        } else if let Some(s) = line.strip_prefix("msgid ") {
            id = serde_json::from_str(s).unwrap_or_default();
            reading_value = false;
        } else if let Some(s) = line.strip_prefix("msgstr ") {
            value = serde_json::from_str(s).unwrap_or_default();
            reading_value = true;
        } else if line.starts_with('"') {
            let s: String = serde_json::from_str(line).unwrap_or_default();
            if reading_value { value += &s } else { id += &s }
        }
    }
    map
}
fn table(locale: &str) -> &'static HashMap<String, String> {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static JA: OnceLock<HashMap<String, String>> = OnceLock::new();
    if locale == "ja" {
        JA.get_or_init(|| parse(include_str!("../../../i18n/ja.po")))
    } else {
        EN.get_or_init(|| parse(include_str!("../../../i18n/en.po")))
    }
}
pub fn tr(locale: &str, key: &str) -> String {
    let locale = normalize(locale);
    if locale == "zh_CN" {
        return key.into();
    }
    table(&locale)
        .get(key)
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| key.into())
}
pub fn message(locale: &str, source: &str) -> String {
    fn inner(locale: &str, source: &str, depth: usize) -> String {
        if depth > 8 {
            return source.into();
        }
        let direct = tr(locale, source);
        if direct != source {
            return direct;
        }
        for sep in ["\n", "; "] {
            if source.contains(sep) {
                return source
                    .split(sep)
                    .map(|s| inner(locale, s, depth + 1))
                    .collect::<Vec<_>>()
                    .join(sep);
            }
        }
        for (template, pattern) in [
            (
                "动作 %s 的帧索引超出图片范围",
                r"^动作 (.+) 的帧索引超出图片范围$",
            ),
            ("软体缺少 %s", r"^软体缺少 (.+)$"),
        ] {
            if let Some(c) = Regex::new(pattern).unwrap().captures(source) {
                return tr(locale, template).replace("%s", &c[1]);
            }
        }
        let mut keys = table(locale)
            .keys()
            .filter(|k| k.len() >= 4 && !k.contains('%'))
            .collect::<Vec<_>>();
        keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
        for key in keys {
            if let Some(tail) = source.strip_prefix(key) {
                return tr(locale, key) + &inner(locale, tail, depth + 1);
            }
            if let Some(head) = source.strip_suffix(key) {
                return head.to_string() + &tr(locale, key);
            }
        }
        source.into()
    }
    let locale = normalize(locale);
    if locale == "zh_CN" {
        source.into()
    } else {
        inner(&locale, source, 0)
    }
}
