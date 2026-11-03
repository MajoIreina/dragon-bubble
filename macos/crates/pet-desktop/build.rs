fn main() {
    if std::env::var_os("CARGO_FEATURE_EMBEDDED_SHU").is_some() {
        for file in ["../../skins/shu/shu.glb", "../../skins/shu/skin.json"] {
            println!("cargo:rerun-if-changed={file}");
            assert!(
                std::path::Path::new(file).is_file(),
                "embedded-shu requires local skins/shu files; use a model-free build for public source checkouts"
            );
        }
        #[cfg(feature = "embedded-shu")]
        pet_core::import::repack_gltf(
            std::path::Path::new("../../skins/shu/shu.glb"),
            &std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bundled-shu.glb"),
        )
        .expect("normalize and validate the local model before embedding");
    }
    println!("cargo:rerun-if-changed=../../assets/tray.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../../assets/tray.ico")
            .set("ProductName", "My Dragon Bubble")
            .set(
                "FileDescription",
                "My Dragon Bubble · Rust desktop companion",
            );
        resource.compile().expect("Windows icon resource");
    }
}
