//! Native, bounded OBJ/FBX adapters. All runtime model formats converge on GLB.
use crate::{
    assets,
    import::{MAX_BYTES, write_glb},
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufReader, Cursor},
    path::Path,
};

struct Glb {
    doc: Value,
    bin: Vec<u8>,
}
impl Glb {
    fn new() -> Self {
        Self {
            doc: json!({"asset":{"version":"2.0","generator":"LPP native Rust importer"},
            "scene":0,"scenes":[{"nodes":[0]}],"nodes":[],"meshes":[],"materials":[],
            "accessors":[],"bufferViews":[],"images":[],"textures":[],"skins":[],"animations":[]}),
            bin: vec![],
        }
    }
    fn push(&mut self, field: &str, value: Value) -> usize {
        let values = self.doc[field].as_array_mut().unwrap();
        let index = values.len();
        values.push(value);
        index
    }
    fn bytes(&mut self, bytes: &[u8]) -> Result<usize> {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        ensure!(
            self.bin.len().saturating_add(bytes.len()) <= MAX_BYTES,
            "模型与依赖合计超过 256 MB"
        );
        let offset = self.bin.len();
        self.bin.extend_from_slice(bytes);
        Ok(self.push(
            "bufferViews",
            json!({"buffer":0,"byteOffset":offset,"byteLength":bytes.len()}),
        ))
    }
    fn floats(&mut self, values: &[f32], width: usize, bounds: bool) -> Result<usize> {
        ensure!(
            !values.is_empty()
                && values.len().is_multiple_of(width)
                && values.iter().all(|v| v.is_finite()),
            "模型包含无效浮点数据"
        );
        let bytes = values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let view = self.bytes(&bytes)?;
        let mut accessor = json!({"bufferView":view,"componentType":5126,"count":values.len()/width,
            "type":match width {1=>"SCALAR",2=>"VEC2",3=>"VEC3",4=>"VEC4",16=>"MAT4",_=>bail!("invalid accessor width")}});
        if bounds {
            let mut min = vec![f32::INFINITY; width];
            let mut max = vec![f32::NEG_INFINITY; width];
            for row in values.chunks_exact(width) {
                for i in 0..width {
                    min[i] = min[i].min(row[i]);
                    max[i] = max[i].max(row[i]);
                }
            }
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        Ok(self.push("accessors", accessor))
    }
    fn indices(&mut self, values: &[u32]) -> Result<usize> {
        let view = self.bytes(
            &values
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )?;
        Ok(self.push(
            "accessors",
            json!({"bufferView":view,"componentType":5125,"count":values.len(),"type":"SCALAR"}),
        ))
    }
    fn joints(&mut self, values: &[u16]) -> Result<usize> {
        let view = self.bytes(
            &values
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )?;
        Ok(self.push(
            "accessors",
            json!({"bufferView":view,"componentType":5123,"count":values.len()/4,"type":"VEC4"}),
        ))
    }
    fn texture(&mut self, bytes: &[u8]) -> Result<usize> {
        let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
        reader.limits(image::Limits::default());
        let image = reader.decode()?;
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png)?;
        let view = self.bytes(png.get_ref())?;
        let source = self.push("images", json!({"bufferView":view,"mimeType":"image/png"}));
        Ok(self.push("textures", json!({"source":source})))
    }
    fn finish(mut self, path: &Path) -> Result<()> {
        ensure!(
            !self.doc["meshes"].as_array().unwrap().is_empty(),
            "模型缺少有效网格"
        );
        self.doc["buffers"] = json!([{"byteLength":self.bin.len()}]);
        for field in ["images", "textures", "skins", "animations", "materials"] {
            if self.doc[field].as_array().is_some_and(Vec::is_empty) {
                self.doc.as_object_mut().unwrap().remove(field);
            }
        }
        write_glb(&self.doc, &self.bin, path)
    }
}
fn dependency(root: &Path, name: &str) -> Result<Vec<u8>> {
    let relative = name.replace('\\', "/");
    let path = assets::contained(root, &relative)
        .or_else(|_| assets::contained(root, relative.rsplit('/').next().unwrap_or("")))?;
    ensure!(
        fs::metadata(&path)?.len() <= MAX_BYTES as u64,
        "资源超过 256 MB"
    );
    Ok(fs::read(path)?)
}
pub fn obj(source: &Path, dest: &Path) -> Result<()> {
    let root = source.parent().context("missing parent")?;
    let mut input = BufReader::new(fs::File::open(source)?);
    let (models, materials) = tobj::load_obj_buf(
        &mut input,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |name| {
            let bytes = dependency(root, &name.to_string_lossy())
                .map_err(|_| tobj::LoadError::ReadError)?;
            let (mut materials, names) =
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(bytes)))?;
            // MTL texture paths are relative to the MTL, which may live in a subdirectory.
            let normalized = name.to_string_lossy().replace('\\', "/");
            let parent = Path::new(&normalized).parent().unwrap_or(Path::new(""));
            for material in &mut materials {
                for texture in [&mut material.diffuse_texture, &mut material.normal_texture] {
                    if let Some(name) = texture.as_mut().filter(|n| !n.is_empty()) {
                        *name = parent
                            .join(name.replace('\\', "/"))
                            .to_string_lossy()
                            .replace('\\', "/");
                    }
                }
            }
            Ok((materials, names))
        },
    )?;
    let mut glb = Glb::new();
    for material in materials? {
        let color = material.diffuse.unwrap_or([0.8; 3]);
        let alpha = material.dissolve.unwrap_or(1.);
        let mut m = json!({"name":material.name,"doubleSided":true,"pbrMetallicRoughness":{
            "baseColorFactor":[color[0],color[1],color[2],alpha],"metallicFactor":0.,
            "roughnessFactor":(2./(material.shininess.unwrap_or(0.)+2.)).sqrt().clamp(0.04,1.)}});
        if alpha < 1. {
            m["alphaMode"] = json!("BLEND");
        }
        if let Some(name) = material.diffuse_texture.filter(|n| !n.is_empty()) {
            let texture = glb.texture(&dependency(root, &name)?)?;
            m["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":texture});
        }
        if let Some(name) = material.normal_texture.filter(|n| !n.is_empty()) {
            let texture = glb.texture(&dependency(root, &name)?)?;
            m["normalTexture"] = json!({"index":texture});
        }
        glb.push("materials", m);
    }
    let mut nodes = vec![];
    glb.push("nodes", json!({"name":"OBJ"}));
    for model in models {
        let mesh = model.mesh;
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            continue;
        }
        let mut attributes = json!({"POSITION":glb.floats(&mesh.positions,3,true)?});
        if !mesh.normals.is_empty() {
            attributes["NORMAL"] = json!(glb.floats(&mesh.normals, 3, false)?);
        }
        if !mesh.texcoords.is_empty() {
            let uv = mesh
                .texcoords
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|p| [p[0], 1. - p[1]])
                .collect::<Vec<_>>();
            attributes["TEXCOORD_0"] = json!(glb.floats(&uv, 2, false)?);
        }
        if !mesh.vertex_color.is_empty() {
            attributes["COLOR_0"] = json!(glb.floats(&mesh.vertex_color, 3, false)?);
        }
        let mut primitive = json!({"attributes":attributes,"indices":glb.indices(&mesh.indices)?});
        if let Some(m) = mesh.material_id {
            primitive["material"] = json!(m);
        }
        let index = glb.push(
            "meshes",
            json!({"name":model.name,"primitives":[primitive]}),
        );
        nodes.push(glb.push("nodes", json!({"name":model.name,"mesh":index})));
    }
    glb.doc["nodes"][0]["children"] = json!(nodes);
    glb.finish(dest)
}
fn matrix(m: &fbx_rs::Matrix) -> [f32; 16] {
    let m = m.0.map(|v| v as f32);
    [
        m[0], m[1], m[2], 0., m[3], m[4], m[5], 0., m[6], m[7], m[8], 0., m[9], m[10], m[11], 1.,
    ]
}
fn animation_channel<const N: usize>(
    glb: &mut Glb,
    animation: &mut Value,
    node: usize,
    path: &str,
    keys: &[fbx_rs::BakedKey<N>],
) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let times = keys
        .iter()
        .map(|k| k.time.max(0.) as f32)
        .collect::<Vec<_>>();
    let input = glb.floats(&times, 1, true)?;
    let output = glb.floats(
        &keys
            .iter()
            .flat_map(|k| k.value.map(|v| v as f32))
            .collect::<Vec<_>>(),
        N,
        false,
    )?;
    let samplers = animation["samplers"].as_array_mut().unwrap();
    let sampler = samplers.len();
    samplers.push(json!({"input":input,"output":output,"interpolation":"LINEAR"}));
    animation["channels"]
        .as_array_mut()
        .unwrap()
        .push(json!({"sampler":sampler,"target":{"node":node,"path":path}}));
    Ok(())
}
pub fn fbx(source: &Path, dest: &Path) -> Result<()> {
    ensure!(
        fs::metadata(source)?.len() <= MAX_BYTES as u64,
        "资源超过 256 MB"
    );
    let scene = fbx_rs::load(&fs::read(source)?, &fbx_rs::LoadOptions::default())?;
    let mut glb = Glb::new();
    let root = source.parent().context("missing parent")?;
    let used_textures = scene
        .materials()
        .iter()
        .flat_map(|material| {
            [
                material
                    .pbr_map("base_color")
                    .or_else(|| material.fbx_map("diffuse_color")),
                material
                    .pbr_map("normal_map")
                    .or_else(|| material.fbx_map("normal_map")),
            ]
        })
        .flatten()
        .filter(|map| map.texture_enabled)
        .filter_map(|map| map.texture)
        .collect::<std::collections::BTreeSet<_>>();
    let mut textures = vec![];
    for (index, texture) in scene.textures().iter().enumerate() {
        if texture.is_layered() || !used_textures.contains(&index) {
            textures.push(None);
            continue;
        }
        let bytes = if texture.content().is_empty() {
            dependency(root, texture.filename())?
        } else {
            texture.content().to_vec()
        };
        textures.push(Some(glb.texture(&bytes)?));
    }
    for material in scene.materials() {
        let base = material
            .pbr_map("base_color")
            .or_else(|| material.fbx_map("diffuse_color"));
        let color = base
            .filter(|m| m.has_value)
            .map(|m| m.value)
            .unwrap_or([0.8, 0.8, 0.8, 1.]);
        let alpha = material
            .pbr_map("opacity")
            .filter(|m| m.has_value)
            .map_or(1., |m| m.value[0].clamp(0., 1.));
        let mut m = json!({"name":material.name(),"doubleSided":true,"pbrMetallicRoughness":{
            "baseColorFactor":[color[0],color[1],color[2],alpha],
            "metallicFactor":material.pbr_map("metalness").filter(|m|m.has_value).map_or(0.,|m|m.value[0].clamp(0.,1.)),
            "roughnessFactor":material.pbr_map("roughness").filter(|m|m.has_value).map_or(0.8,|m|m.value[0].clamp(0.04,1.))}});
        if alpha < 1. {
            m["alphaMode"] = json!("BLEND");
        }
        if let Some(t) = base
            .filter(|m| m.texture_enabled)
            .and_then(|m| m.texture)
            .and_then(|i| textures.get(i).copied().flatten())
        {
            m["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":t});
        }
        if let Some(t) = material
            .pbr_map("normal_map")
            .or_else(|| material.fbx_map("normal_map"))
            .filter(|m| m.texture_enabled)
            .and_then(|m| m.texture)
            .and_then(|i| textures.get(i).copied().flatten())
        {
            m["normalTexture"] = json!({"index":t});
        }
        glb.push("materials", m);
    }
    for node in scene.nodes() {
        let t = node.local_transform();
        let mut n = json!({"name":node.name(),"translation":t.translation,"rotation":t.rotation,"scale":t.scale});
        if !node.children().is_empty() {
            n["children"] = json!(node.children());
        }
        glb.push("nodes", n);
    }
    // Mesh nodes are separate from transform/animation nodes so FBX geometric transforms remain local.
    let mut morph_nodes: Vec<(usize, Vec<usize>)> = vec![];
    for (node_index, node) in scene.nodes().iter().enumerate() {
        let Some(mesh_index) = node.mesh() else {
            continue;
        };
        let mesh = &scene.meshes()[mesh_index];
        if mesh.triangles().is_empty() {
            continue;
        }
        let count = mesh.num_corners();
        ensure!(count <= 4_000_000, "模型顶点过多");
        let positions = (0..count)
            .flat_map(|i| mesh.positions().get(i).unwrap_or([0.; 3]).map(|v| v as f32))
            .collect::<Vec<_>>();
        let mut attributes = json!({"POSITION":glb.floats(&positions,3,true)?});
        if let Some(normals) = mesh.normals() {
            attributes["NORMAL"] = json!(
                glb.floats(
                    &(0..count)
                        .flat_map(|i| normals.get(i).unwrap_or([0., 1., 0.]).map(|v| v as f32))
                        .collect::<Vec<_>>(),
                    3,
                    false
                )?
            );
        }
        for (i, set) in mesh.uv_sets().iter().take(2).enumerate() {
            let uv = (0..count)
                .flat_map(|c| {
                    let uv = set.uv().get(c).unwrap_or([0.; 2]);
                    [uv[0] as f32, 1. - uv[1] as f32]
                })
                .collect::<Vec<_>>();
            attributes[format!("TEXCOORD_{i}")] = json!(glb.floats(&uv, 2, false)?);
        }
        if let Some(set) = mesh.color_sets().first() {
            let colors = (0..count)
                .flat_map(|c| set.color().get(c).unwrap_or([1.; 4]).map(|v| v as f32))
                .collect::<Vec<_>>();
            attributes["COLOR_0"] = json!(glb.floats(&colors, 4, false)?);
        }
        let mut skin_index = None;
        if let Some(&index) = mesh.skin_deformers().first() {
            let skin = &scene.skin_deformers()[index];
            ensure!(skin.clusters.len() <= u16::MAX as usize, "骨骼数量过多");
            let mut joints = vec![];
            let mut weights = vec![];
            for &point in mesh.positions().indices() {
                let mut ji = [0u16; 4];
                let mut we = [0f32; 4];
                if let Some(vertex) = skin.vertices.get(point as usize) {
                    for (i, w) in skin.weights
                        [vertex.weight_begin..vertex.weight_begin + vertex.num_weights]
                        .iter()
                        .take(4)
                        .enumerate()
                    {
                        ji[i] = w.cluster_index as u16;
                        we[i] = w.weight.max(0.) as f32;
                    }
                }
                let sum = we.iter().sum::<f32>();
                if sum > 0. {
                    we.iter_mut().for_each(|w| *w /= sum);
                } else {
                    we[0] = 1.;
                }
                joints.extend(ji);
                weights.extend(we);
            }
            attributes["JOINTS_0"] = json!(glb.joints(&joints)?);
            attributes["WEIGHTS_0"] = json!(glb.floats(&weights, 4, false)?);
            let bones = skin
                .clusters
                .iter()
                .map(|&c| {
                    scene.skin_clusters()[c]
                        .bone_node
                        .context("FBX 蒙皮缺少骨骼节点")
                })
                .collect::<Result<Vec<_>>>()?;
            let inverse = skin
                .clusters
                .iter()
                .flat_map(|&c| matrix(&scene.skin_clusters()[c].geometry_to_bone))
                .collect::<Vec<_>>();
            let inverse = glb.floats(&inverse, 16, false)?;
            skin_index = Some(glb.push(
                "skins",
                json!({"joints":bones,"inverseBindMatrices":inverse}),
            ));
        }
        let channels = mesh
            .blend_deformers()
            .iter()
            .flat_map(|&i| scene.blend_deformers()[i].channels.iter().copied())
            .filter(|&c| scene.blend_channels()[c].target_shape.is_some())
            .collect::<Vec<_>>();
        let mut targets = vec![];
        let mut names = vec![];
        let mut defaults = vec![];
        for &c in &channels {
            let channel = &scene.blend_channels()[c];
            let Some(shape) = channel.target_shape else {
                continue;
            };
            let shape = &scene.blend_shapes()[shape];
            let offsets = shape
                .offset_vertices
                .iter()
                .enumerate()
                .map(|(i, &v)| (v, shape.position_offsets[i]))
                .collect::<BTreeMap<_, _>>();
            let values = mesh
                .positions()
                .indices()
                .iter()
                .flat_map(|p| offsets.get(p).copied().unwrap_or([0.; 3]).map(|v| v as f32))
                .collect::<Vec<_>>();
            targets.push(json!({"POSITION":glb.floats(&values,3,false)?}));
            names.push(channel.name.clone());
            defaults.push(channel.weight);
        }
        let mut slots: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut face_for_corner = vec![0; count];
        for (i, face) in mesh.faces().iter().enumerate() {
            for c in face.begin..face.begin + face.count {
                face_for_corner[c as usize] = i;
            }
        }
        for triangle in mesh.triangles().as_chunks::<3>().0 {
            let face = face_for_corner[triangle[0] as usize];
            let slot = mesh.face_material().get(face).copied().unwrap_or(0);
            slots.entry(slot).or_default().extend_from_slice(triangle);
        }
        let mut primitives = vec![];
        for (slot, indices) in slots {
            let mut p = json!({"attributes":attributes,"indices":glb.indices(&indices)?});
            if let Some(&material) = node
                .materials()
                .get(slot as usize)
                .or_else(|| mesh.materials().get(slot as usize))
            {
                p["material"] = json!(material);
            }
            if !targets.is_empty() {
                p["targets"] = json!(targets);
            }
            primitives.push(p);
        }
        let mut out = json!({"name":mesh.name(),"primitives":primitives});
        if !targets.is_empty() {
            out["weights"] = json!(defaults);
            out["extras"] = json!({"targetNames":names});
        }
        let mesh_index = glb.push("meshes", out);
        let mut n = json!({"name":format!("{} geometry",node.name()),"mesh":mesh_index,"matrix":matrix(node.geometry_to_node())});
        if let Some(skin) = skin_index {
            n["skin"] = json!(skin);
        }
        let n = glb.push("nodes", n);
        let parent = &mut glb.doc["nodes"][node_index];
        if parent.get("children").is_none() {
            parent["children"] = json!([]);
        }
        parent["children"].as_array_mut().unwrap().push(json!(n));
        if !targets.is_empty() {
            morph_nodes.push((n, channels));
        }
    }
    for (i, stack) in scene.animation_stacks().iter().enumerate() {
        let mut options = fbx_rs::BakeOptions::default();
        options.trim_start_time = true;
        options.max_keys = 2_000_000;
        options.max_allocation = MAX_BYTES;
        let baked = scene.bake_animation(Some(i), &options)?;
        let mut animation = json!({"name":stack.name,"samplers":[],"channels":[]});
        for node in &baked.nodes {
            animation_channel(
                &mut glb,
                &mut animation,
                node.node,
                "translation",
                &node.translation_keys,
            )?;
            animation_channel(
                &mut glb,
                &mut animation,
                node.node,
                "rotation",
                &node.rotation_keys,
            )?;
            animation_channel(
                &mut glb,
                &mut animation,
                node.node,
                "scale",
                &node.scale_keys,
            )?;
        }
        // FBX DeformPercent is a percentage. Bake each morph channel to glTF's shared weight vector.
        for (node, channels) in &morph_nodes {
            let properties = channels
                .iter()
                .map(|&c| {
                    let ch = &scene.blend_channels()[c];
                    baked
                        .properties
                        .iter()
                        .find(|p| Some(p.source_id) == ch.source_id && p.name == b"DeformPercent")
                })
                .collect::<Vec<_>>();
            if properties.iter().all(Option::is_none) {
                continue;
            }
            let mut times = properties
                .iter()
                .flatten()
                .flat_map(|p| p.keys.iter().map(|k| k.time.max(0.) as f32))
                .collect::<Vec<_>>();
            times.sort_by(f32::total_cmp);
            times.dedup();
            if times.is_empty() {
                continue;
            }
            let mut weights = vec![];
            for &time in &times {
                for (j, &c) in channels.iter().enumerate() {
                    let value = if let Some(p) = properties[j] {
                        let upper = p.keys.partition_point(|k| k.time <= time as f64);
                        let a = &p.keys[upper.saturating_sub(1)];
                        let b = p.keys.get(upper).unwrap_or(a);
                        let t = if b.time > a.time {
                            ((time as f64 - a.time) / (b.time - a.time)).clamp(0., 1.)
                        } else {
                            0.
                        };
                        (a.value[0] + (b.value[0] - a.value[0]) * t) * 0.01
                    } else {
                        scene.blend_channels()[c].weight
                    };
                    weights.push(value as f32);
                }
            }
            let input = glb.floats(&times, 1, true)?;
            let output = glb.floats(&weights, 1, false)?;
            let samplers = animation["samplers"].as_array_mut().unwrap();
            let sampler = samplers.len();
            samplers.push(json!({"input":input,"output":output,"interpolation":"LINEAR"}));
            animation["channels"]
                .as_array_mut()
                .unwrap()
                .push(json!({"sampler":sampler,"target":{"node":node,"path":"weights"}}));
        }
        if !animation["channels"].as_array().unwrap().is_empty() {
            glb.push("animations", animation);
        }
    }
    glb.finish(dest)
}
