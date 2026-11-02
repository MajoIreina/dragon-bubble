use crate::assets::{self, Skin};
use anyhow::{Context, Result, bail};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub const MAX_BYTES: usize = 256 * 1024 * 1024;
fn read_limited(path: &Path, max: usize) -> Result<Vec<u8>> {
    if fs::metadata(path)?.len() > max as u64 {
        bail!("资源超过 256 MB")
    }
    Ok(fs::read(path)?)
}
fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .context("GLB 头部或 JSON 块无效")?
            .try_into()?,
    ))
}
pub fn gltf_json(path: &Path) -> Result<(Value, Vec<u8>)> {
    let bytes = read_limited(path, MAX_BYTES)?;
    if assets::extension(path) == "gltf" {
        return Ok((serde_json::from_slice(&bytes)?, vec![]));
    }
    if bytes.len() < 20
        || u32_at(&bytes, 0)? != 0x46546c67
        || u32_at(&bytes, 4)? != 2
        || u32_at(&bytes, 8)? as usize != bytes.len()
        || u32_at(&bytes, 16)? != 0x4e4f534a
    {
        bail!("GLB 头部或 JSON 块无效")
    }
    let end = 20usize
        .checked_add(u32_at(&bytes, 12)? as usize)
        .context("GLB size overflow")?;
    let data = serde_json::from_slice(bytes.get(20..end).context("GLB JSON 无效")?)?;
    let bin = if end + 8 <= bytes.len() {
        let len = u32_at(&bytes, end)? as usize;
        if u32_at(&bytes, end + 4)? != 0x004e4942 {
            vec![]
        } else {
            bytes
                .get(end + 8..end + 8 + len)
                .context("glTF 缓冲区被截断")?
                .to_vec()
        }
    } else {
        vec![]
    };
    Ok((data, bin))
}
fn uri_payload(root: &Path, uri: &str, total: &mut usize) -> Result<Vec<u8>> {
    let payload = if uri.starts_with("data:") {
        base64::engine::general_purpose::STANDARD
            .decode(uri.split_once(',').context("invalid data URI")?.1)?
    } else {
        let rel = percent_encoding::percent_decode_str(uri).decode_utf8()?;
        read_limited(&assets::contained(root, &rel)?, MAX_BYTES)?
    };
    *total = total
        .checked_add(payload.len())
        .context("resource size overflow")?;
    if *total > MAX_BYTES {
        bail!("模型与依赖合计超过 256 MB")
    }
    Ok(payload)
}
fn pad(bytes: &mut Vec<u8>, value: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(value)
    }
}
pub fn write_glb(data: &Value, binary: &[u8], path: &Path) -> Result<()> {
    let mut json = serde_json::to_vec(data)?;
    pad(&mut json, b' ');
    let mut bin = binary.to_vec();
    pad(&mut bin, 0);
    let length = 28 + json.len() + bin.len();
    if length > MAX_BYTES {
        bail!("标准化模型超过 256 MB")
    }
    let mut out = Vec::with_capacity(length);
    for n in [0x46546c67, 2, length as u32, json.len() as u32, 0x4e4f534a] {
        out.extend(n.to_le_bytes())
    }
    out.extend(json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004e4942u32.to_le_bytes());
    out.extend(bin);
    fs::write(path, out)?;
    Ok(())
}
pub fn repack_gltf(source: &Path, dest: &Path) -> Result<()> {
    let (mut data, source_bin) = gltf_json(source)?;
    normalize_texture_coordinates(&mut data);
    let root = source.parent().context("missing parent")?;
    let mut total = fs::metadata(source)?.len() as usize;
    let buffers = data
        .get("buffers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut bin = vec![];
    let mut offsets = vec![];
    let mut lengths = vec![];
    for buffer in &buffers {
        let payload = if let Some(uri) = buffer.get("uri") {
            uri_payload(
                root,
                uri.as_str().context("glTF URI 必须是字符串")?,
                &mut total,
            )?
        } else {
            source_bin.clone()
        };
        if payload.len() < buffer["byteLength"].as_u64().unwrap_or(0) as usize {
            bail!("glTF 缓冲区被截断")
        }
        pad(&mut bin, 0);
        offsets.push(bin.len());
        lengths.push(payload.len());
        bin.extend(payload)
    }
    if let Some(views) = data.get_mut("bufferViews").and_then(Value::as_array_mut) {
        for view in views {
            let i = view["buffer"].as_u64().context("glTF 缓冲视图越界")? as usize;
            let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let len = view["byteLength"].as_u64().context("glTF 缓冲视图越界")? as usize;
            if i >= lengths.len() || offset.checked_add(len).is_none_or(|v| v > lengths[i]) {
                bail!("glTF 缓冲视图越界")
            }
            view["byteOffset"] = json!(offset + offsets[i]);
            view["buffer"] = json!(0)
        }
    }
    let mut views = data
        .get("bufferViews")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(images) = data.get_mut("images").and_then(Value::as_array_mut) {
        for img in images {
            if let Some(uri) = img.get("uri").and_then(Value::as_str).map(str::to_string) {
                let payload = uri_payload(root, &uri, &mut total)?;
                pad(&mut bin, 0);
                let mime = if uri.starts_with("data:") {
                    uri.split_once(';')
                        .map(|v| v.0.trim_start_matches("data:"))
                        .unwrap_or("image/png")
                        .to_string()
                } else {
                    match assets::extension(Path::new(&uri)).as_str() {
                        "jpg" | "jpeg" => "image/jpeg",
                        "webp" => "image/webp",
                        _ => "image/png",
                    }
                    .into()
                };
                img["bufferView"] = json!(views.len());
                img["mimeType"] = json!(mime);
                img.as_object_mut()
                    .context("glTF 资源条目无效")?
                    .remove("uri");
                views.push(json!({"buffer":0,"byteOffset":bin.len(),"byteLength":payload.len()}));
                bin.extend(payload)
            }
        }
    }
    data["bufferViews"] = json!(views);
    data["buffers"] = json!([{"byteLength":bin.len()}]);
    write_glb(&data, &bin, dest)?;
    gltf::Gltf::open(dest).context("模型解码失败")?;
    Ok(())
}
pub fn blender_path(data: &Path, root: &Path) -> Option<PathBuf> {
    let configured = fs::read_to_string(data.join("converter-path.txt"))
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .or_else(|| std::env::var_os("LPP_BLENDER_PATH").map(PathBuf::from));
    if let Some(p) = configured
        && p.is_file()
    {
        return Some(p);
    }
    for p in [
        root.join("work/tools/blender/blender.exe"),
        PathBuf::from("C:/Program Files/Blender Foundation/Blender 4.2/blender.exe"),
    ] {
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(entries) = fs::read_dir("C:/Program Files/Blender Foundation") {
        for e in entries.flatten() {
            let p = e.path().join("blender.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}
pub fn convert(
    source: &Path,
    dest: &Path,
    blender: &Path,
    script: &Path,
    data: &Path,
) -> Result<()> {
    let mut command = Command::new(blender);
    command
        .args([
            "--background",
            "--factory-startup",
            "--disable-autoexec",
            "--python",
        ])
        .arg(script)
        .arg("--")
        .arg("--input")
        .arg(source)
        .arg("--output")
        .arg(dest);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output()?;
    if !output.status.success() || !dest.is_file() {
        let _ = fs::write(
            data.join("conversion-last.log"),
            [output.stdout, output.stderr].concat(),
        );
        bail!("Blender 转换失败，详情见运行数据目录中的 conversion-last.log")
    }
    Ok(())
}
/// Private staging directory, published only after decode and manifest validation.
/// Existing directories and source files are never modified or removed.
pub fn install(source: &Path, user: &Path, project: &Path) -> Result<Skin> {
    fs::create_dir_all(user)?;
    let mut skin = if assets::extension(source) == "json" {
        assets::read_manifest(source)?
    } else {
        let ext = assets::extension(source);
        if !assets::IMAGE_EXT.contains(&ext.as_str()) && !assets::MODEL_EXT.contains(&ext.as_str())
        {
            bail!("不支持此文件格式。Live2D / Spine 专用工程暂需导出为精灵图或 GIF。")
        }
        read_limited(source, MAX_BYTES)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        serde_json::from_value::<Skin>(
            json!({"schema_version":1,"id":format!("custom.{stamp}"),"name":source.file_stem().unwrap_or_default().to_string_lossy(),"type":if assets::MODEL_EXT.contains(&ext.as_str()){"model3d"}else{"sprite"},"asset":source.file_name().unwrap_or_default().to_string_lossy(),"source_format":ext}),
        )?
    };
    if skin.root.as_os_str().is_empty() {
        skin.root = source.parent().context("missing parent")?.to_path_buf()
    }
    assets::validate(&skin)?;
    if skin.r#type == "procedural" {
        bail!("procedural 仅供内置演示使用")
    }
    let skins = user.join("skins");
    fs::create_dir_all(&skins)?;
    let target = skins.join(&skin.id);
    if target.exists() {
        bail!("外观 ID 已存在，请修改 ID 后导入")
    }
    let staging = skins.join(format!(".import-{}-{}", skin.id, std::process::id()));
    fs::create_dir(&staging)?;
    let result = (|| -> Result<Skin> {
        let asset = skin.asset.as_deref().context("missing asset")?;
        let from = assets::contained(&skin.root, asset)?;
        if skin.r#type == "model3d" {
            let dest = staging.join("model.glb");
            match assets::extension(&from).as_str() {
                "glb" | "gltf" | "vrm" => repack_gltf(&from, &dest)?,
                "obj" => crate::model_import::obj(&from, &dest)?,
                "fbx" => crate::model_import::fbx(&from, &dest)?,
                _ => {
                    let blender=blender_path(user,project).context("此格式需要 Blender 4.2+。请在“动作与适配”中设置 blender.exe 路径，或导出为 GLB 后导入。")?;
                    convert(
                        &from,
                        &dest,
                        &blender,
                        &project.join("converters/model_to_glb.py"),
                        user,
                    )?
                }
            }
            gltf::Gltf::open(&dest)?;
            skin.asset = Some("model.glb".into())
        } else {
            let mut files: Vec<String> = skin
                .options
                .get("frames")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            if !files.contains(&asset.to_string()) {
                files.push(asset.to_string())
            }
            let mut total = 0;
            for relative in files {
                let from = assets::contained(&skin.root, &relative)?;
                total += fs::metadata(&from)?.len();
                if total > MAX_BYTES as u64 {
                    bail!("图片文件合计超过 256 MB")
                }
                let to = staging.join(relative);
                fs::create_dir_all(to.parent().unwrap())?;
                fs::copy(from, to)?;
            }
            let mut candidate = skin.clone();
            candidate.root = staging.clone();
            crate::sprite::SpritePack::load(&candidate)?;
        }
        skin.root = staging.clone();
        fs::write(staging.join("skin.json"), serde_json::to_vec_pretty(&skin)?)?;
        assets::read_manifest(&staging.join("skin.json"))?;
        fs::rename(&staging, &target)?;
        skin.root = target.clone();
        Ok(skin.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

// Some legacy Blender exports use -1 to mean the default UV set.
// Keep the source file untouched and normalize only this known exporter sentinel.
fn normalize_texture_coordinates(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.get("texCoord").and_then(Value::as_i64) == Some(-1) {
                map.insert("texCoord".into(), json!(0));
            }
            for child in map.values_mut() {
                normalize_texture_coordinates(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                normalize_texture_coordinates(child);
            }
        }
        _ => {}
    }
}
pub fn cached_model(skin: &Skin, user: &Path) -> Result<PathBuf> {
    use std::hash::{Hash, Hasher};
    let source = assets::contained(&skin.root, skin.asset.as_deref().context("missing asset")?)?;
    let metadata = fs::metadata(&source)?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    "rust-import-v2".hash(&mut hash);
    source.hash(&mut hash);
    metadata.len().hash(&mut hash);
    metadata.modified()?.hash(&mut hash);
    let cache = user.join("cache");
    fs::create_dir_all(&cache)?;
    let path = cache.join(format!("{:016x}.glb", hash.finish()));
    // External dependencies can change independently of the model's timestamp.
    // Installed models are self-contained GLB; loose dependency formats are repacked per request.
    if !path.exists() || ["gltf", "obj", "fbx"].contains(&assets::extension(&source).as_str()) {
        let tmp = path.with_extension("partial.glb");
        match assets::extension(&source).as_str() {
            "obj" => crate::model_import::obj(&source, &tmp)?,
            "fbx" => crate::model_import::fbx(&source, &tmp)?,
            _ => repack_gltf(&source, &tmp)?,
        }
        fs::rename(tmp, &path)?;
    }
    Ok(path)
}
