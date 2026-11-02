//! Opt-in cel shading retains glTF textures, alpha, skinning and morph targets.
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
const SHADER: Handle<Shader> = uuid_handle!("e54c28bf-68b5-46ce-8665-996c145d832e");
pub type ToonMaterial = ExtendedMaterial<StandardMaterial, ToonExtension>;
pub struct ToonPlugin;
impl Plugin for ToonPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "toon.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<ToonMaterial>::default());
    }
}
#[derive(Component)]
pub struct ToonStyled;
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ToonExtension {
    #[uniform(100)]
    pub settings: Vec4,
    /// Shadow threshold, light threshold, transition softness and outline softness.
    #[uniform(101)]
    pub transitions: Vec4,
}
impl Default for ToonExtension {
    fn default() -> Self {
        Self {
            settings: Vec4::new(0.68, 0.88, 0.10, 0.035),
            transitions: Vec4::new(-0.12, 0.46, 0.14, 0.055),
        }
    }
}
impl MaterialExtension for ToonExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}
