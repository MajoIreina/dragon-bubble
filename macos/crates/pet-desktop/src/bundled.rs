//! Optional local model embedding for release builds; no private assets live in Git.
use bevy::prelude::*;
use pet_core::assets::Skin;

pub fn register(_app: &mut App) {
    #[cfg(feature = "embedded-shu")]
    _app.world()
        .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .insert_asset(
            std::path::PathBuf::new(),
            std::path::Path::new("dragon-bubble/shu.glb"),
            include_bytes!(concat!(env!("OUT_DIR"), "/bundled-shu.glb")).as_slice(),
        );
}

pub fn add_skin(_skins: &mut Vec<Skin>) {
    #[cfg(feature = "embedded-shu")]
    {
        let mut skin: Skin = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../skins/shu/skin.json"
        )))
        .expect("valid bundled Shu manifest");
        // A virtual root keeps ordinary user skins on the normal import/cache path.
        skin.root = "embedded://dragon-bubble".into();
        _skins.retain(|s| s.id != skin.id);
        _skins.insert(0, skin);
    }
}

pub fn asset_path(skin: &Skin) -> Option<&'static str> {
    if cfg!(feature = "embedded-shu")
        && skin.id == "builtin.shu"
        && skin.root == std::path::Path::new("embedded://dragon-bubble")
    {
        Some("embedded://dragon-bubble/shu.glb")
    } else {
        None
    }
}
