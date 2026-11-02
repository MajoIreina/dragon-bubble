//! The 3D render target already contains premultiplied alpha. Compositing it
//! with the normal sprite blend would multiply edge pixels a second time.
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    render::render_resource::{
        AsBindGroup, BlendState, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin},
};
const SHADER: Handle<Shader> = uuid_handle!("3ea386c2-7e0d-49ec-baa0-f19f7c0783b4");
pub struct CompositePlugin;
impl Plugin for CompositePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "composite.wgsl", Shader::from_wgsl);
        app.add_plugins(Material2dPlugin::<PetComposite>::default());
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PetComposite {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
}
impl Material2d for PetComposite {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        _: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            }
        }
        Ok(())
    }
}
