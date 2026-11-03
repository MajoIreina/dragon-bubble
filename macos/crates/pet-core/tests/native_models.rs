use pet_core::import;
use std::{fs, path::PathBuf};

#[test]
fn obj_material_texture_and_submesh_survive_install() {
    let input = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(input.path().join("materials")).unwrap();
    fs::write(input.path().join("pet.obj"), "mtllib materials/pet.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\no face\nusemtl fabric\nf 1/1/1 2/2/1 3/3/1\n").unwrap();
    fs::write(
        input.path().join("materials/pet.mtl"),
        "newmtl fabric\nKd 0.8 0.6 0.2\nmap_Kd color.png\n",
    )
    .unwrap();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([230, 180, 40, 255]))
        .save(input.path().join("materials/color.png"))
        .unwrap();
    let skin = import::install(&input.path().join("pet.obj"), data.path(), input.path()).unwrap();
    let path = skin.root.join(skin.asset.unwrap());
    let gltf = gltf::Gltf::open(&path).unwrap();
    assert_eq!(gltf.meshes().count(), 1);
    let primitive = gltf.meshes().next().unwrap().primitives().next().unwrap();
    assert!(primitive.get(&gltf::Semantic::TexCoords(0)).is_some());
    assert!(primitive.get(&gltf::Semantic::Normals).is_some());
    assert!(
        primitive
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .is_some()
    );
    fs::remove_dir_all(input.path()).unwrap();
    assert!(
        gltf::import(path).is_ok(),
        "installed model must contain its dependencies"
    );
}

// Optional real exporter regression. Generate using tests/create_blender_fixtures.py.
#[test]
#[ignore = "requires generated Blender FBX fixture in work/adapter-fixtures"]
fn fbx_preserves_rig_shapes_materials_and_native_animation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = tempfile::tempdir().unwrap();
    let skin = import::install(
        &root.join("work/adapter-fixtures/chibi.fbx"),
        data.path(),
        &root,
    )
    .unwrap();
    let path = skin.root.join(skin.asset.unwrap());
    let (doc, buffers, _) = gltf::import(&path).unwrap();
    assert!(doc.skins().count() > 0);
    assert!(doc.animations().count() > 0);
    assert!(doc.materials().count() > 0);
    assert!(
        doc.meshes()
            .flat_map(|m| m.primitives())
            .any(|p| p.morph_targets().count() >= 2)
    );
    for animation in doc.animations() {
        for channel in animation.channels() {
            let reader = channel.reader(|b| Some(buffers[b.index()].0.as_slice()));
            assert!(reader.read_inputs().unwrap().count() > 0);
            assert!(reader.read_outputs().is_some());
        }
    }
}
