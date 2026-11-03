//! Transparent RGBA TAA, adapted from Bevy 0.17.3 (MIT OR Apache-2.0).
//! Copyright Bevy Contributors. Unlike the stock pass, alpha is coverage history.
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    camera::{Exposure, primitives::Frustum},
    core_pipeline::{
        FullscreenShader,
        core_3d::graph::{Core3d, Node3d},
        prepass::{DepthPrepass, MotionVectorPrepass, ViewPrepassTextures},
    },
    diagnostic::FrameCount,
    ecs::query::QueryItem,
    image::ToExtents,
    math::vec2,
    prelude::*,
    render::{
        ExtractSchedule, MainWorld, Render, RenderApp, RenderStartup, RenderSystems,
        camera::{ExtractedCamera, MipBias, TemporalJitter},
        diagnostic::RecordDiagnostics,
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, texture_depth_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        sync_component::SyncComponentPlugin,
        sync_world::RenderEntity,
        texture::CachedTexture,
        view::{
            ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
            prepare_view_uniforms,
        },
    },
};
/// Plugin for temporal anti-aliasing.
///
/// See [`TemporalAntiAliasing`] for more details.
#[derive(Default)]
pub struct TemporalAntiAliasPlugin;

impl Plugin for TemporalAntiAliasPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "temporal.wgsl");

        app.add_plugins(SyncComponentPlugin::<TemporalAntiAliasing>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<TaaPipeline>>()
            .add_systems(RenderStartup, init_taa_pipeline)
            .add_systems(ExtractSchedule, extract_taa_settings)
            .add_systems(
                Render,
                (
                    prepare_taa_jitter.in_set(RenderSystems::ManageViews),
                    prepare_orthographic_uniforms
                        .after(prepare_view_uniforms)
                        .in_set(RenderSystems::PrepareResources),
                    prepare_taa_pipelines
                        .after(prepare_taa_history_textures)
                        .in_set(RenderSystems::Prepare),
                    prepare_taa_history_textures.in_set(RenderSystems::PrepareResources),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<TemporalAntiAliasNode>>(Core3d, Node3d::Taa)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::StartMainPassPostProcessing,
                    Node3d::Taa,
                    Node3d::Tonemapping,
                ),
            );
    }
}

/// Motion-reprojected temporal antialiasing with premultiplied RGBA history.
/// MSAA must be off; alpha-blended surfaces lack motion vectors and use
/// neighborhood rejection. Reset on scene/size discontinuities.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(TemporalJitter, MipBias, DepthPrepass, MotionVectorPrepass)]
#[doc(alias = "Taa")]
pub struct TemporalAntiAliasing {
    /// Set to true to delete the saved temporal history (past frames).
    ///
    /// Useful for preventing ghosting when the history is no longer
    /// representative of the current frame, such as in sudden camera cuts.
    ///
    /// After setting this to true, it will automatically be toggled
    /// back to false at the end of the frame.
    pub reset: bool,
}

impl Default for TemporalAntiAliasing {
    fn default() -> Self {
        Self { reset: true }
    }
}

/// Render [`bevy_render::render_graph::Node`] used by temporal anti-aliasing.
#[derive(Default)]
pub struct TemporalAntiAliasNode;

impl ViewNode for TemporalAntiAliasNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static TemporalAntiAliasHistoryTextures,
        &'static ViewPrepassTextures,
        &'static TemporalAntiAliasPipelineId,
        &'static Msaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, view_target, taa_history_textures, prepass_textures, taa_pipeline_id, msaa): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if *msaa != Msaa::Off {
            warn!("Temporal anti-aliasing requires MSAA to be disabled");
            return Ok(());
        }

        let (Some(pipelines), Some(pipeline_cache)) = (
            world.get_resource::<TaaPipeline>(),
            world.get_resource::<PipelineCache>(),
        ) else {
            return Ok(());
        };
        let (Some(taa_pipeline), Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) = (
            pipeline_cache.get_render_pipeline(taa_pipeline_id.0),
            &prepass_textures.motion_vectors,
            &prepass_textures.depth,
        ) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let view_target = view_target.post_process_write();

        let taa_bind_group = render_context.render_device().create_bind_group(
            "taa_bind_group",
            &pipelines.taa_bind_group_layout,
            &BindGroupEntries::sequential((
                view_target.source,
                &taa_history_textures.read.default_view,
                &prepass_motion_vectors_texture.texture.default_view,
                &prepass_depth_texture.texture.default_view,
                &pipelines.nearest_sampler,
                &pipelines.linear_sampler,
            )),
        );

        {
            let mut taa_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("taa"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: view_target.destination,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                    Some(RenderPassColorAttachment {
                        view: &taa_history_textures.write.default_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let pass_span = diagnostics.pass_span(&mut taa_pass, "taa");

            taa_pass.set_render_pipeline(taa_pipeline);
            taa_pass.set_bind_group(0, &taa_bind_group, &[]);
            if let Some(viewport) = camera.viewport.as_ref() {
                taa_pass.set_camera_viewport(viewport);
            }
            taa_pass.draw(0..3, 0..1);

            pass_span.end(&mut taa_pass);
        }

        taa_history_textures
            .valid
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

#[derive(Resource)]
struct TaaPipeline {
    taa_bind_group_layout: BindGroupLayout,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

fn init_taa_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("taa_nearest_sampler"),
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        ..SamplerDescriptor::default()
    });
    let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("taa_linear_sampler"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..SamplerDescriptor::default()
    });

    let taa_bind_group_layout = render_device.create_bind_group_layout(
        "taa_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // View target (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // TAA History (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Motion Vectors
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Depth
                texture_depth_2d(),
                // Nearest sampler
                sampler(SamplerBindingType::NonFiltering),
                // Linear sampler
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    commands.insert_resource(TaaPipeline {
        taa_bind_group_layout,
        nearest_sampler,
        linear_sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "temporal.wgsl"),
    });
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct TaaPipelineKey {
    hdr: bool,
    reset: bool,
}

impl SpecializedRenderPipeline for TaaPipeline {
    type Key = TaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];

        let format = if key.hdr {
            shader_defs.push("TONEMAP".into());
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if key.reset {
            shader_defs.push("RESET".into());
        }

        RenderPipelineDescriptor {
            label: Some("taa_pipeline".into()),
            layout: vec![self.taa_bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
                ..default()
            }),
            ..default()
        }
    }
}

fn extract_taa_settings(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d =
        main_world.query::<(RenderEntity, &Camera, Option<&mut TemporalAntiAliasing>)>();

    for (entity, camera, taa_settings) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        let enabled = camera.is_active && taa_settings.is_some();
        if let Some(mut settings) = taa_settings {
            if enabled {
                entity_commands.insert(settings.clone());
                settings.reset = false;
            } else {
                settings.reset = true;
            }
        }
        if !enabled {
            entity_commands.remove::<(
                TemporalAntiAliasing,
                TemporalAntiAliasHistoryTextures,
                TemporalAntiAliasPipelineId,
            )>();
        }
    }
}

#[allow(clippy::type_complexity)] // Required camera prepasses.
fn prepare_taa_jitter(
    frame_count: Res<FrameCount>,
    mut query: Query<
        &mut TemporalJitter,
        (
            With<TemporalAntiAliasing>,
            With<Camera3d>,
            With<TemporalJitter>,
            With<DepthPrepass>,
            With<MotionVectorPrepass>,
        ),
    >,
) {
    // Halton sequence (2, 3) - 0.5
    let halton_sequence = [
        vec2(0.0, 0.0),
        vec2(0.0, -0.16666666),
        vec2(-0.25, 0.16666669),
        vec2(0.25, -0.3888889),
        vec2(-0.375, -0.055555552),
        vec2(0.125, 0.2777778),
        vec2(-0.125, -0.2777778),
        vec2(0.375, 0.055555582),
    ];

    let offset = halton_sequence[frame_count.0 as usize % halton_sequence.len()];

    for mut jitter in &mut query {
        jitter.offset = offset;
    }
}

#[derive(Component)]
pub struct TemporalAntiAliasHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
    size: UVec2,
    valid: std::sync::atomic::AtomicBool,
}

fn prepare_taa_history_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut views: Query<(
        Entity,
        &ExtractedCamera,
        &mut TemporalAntiAliasing,
        Option<&mut TemporalAntiAliasHistoryTextures>,
    )>,
) {
    for (entity, camera, mut settings, history) in &mut views {
        let Some(size) = camera.physical_target_size else {
            continue;
        };
        if let Some(mut history) = history
            && history.size == size
        {
            settings.reset |= !history
                .valid
                .swap(false, std::sync::atomic::Ordering::Relaxed);
            let history = &mut *history;
            std::mem::swap(&mut history.write, &mut history.read);
        } else {
            settings.reset = true;
            let create = |label| {
                let texture = render_device.create_texture(&TextureDescriptor {
                    label: Some(label),
                    size: size.to_extents(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba16Float,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let default_view = texture.create_view(&TextureViewDescriptor::default());
                CachedTexture {
                    texture,
                    default_view,
                }
            };
            // Persistent per-camera ownership prevents the preview and pet from
            // exchanging histories when either camera is inactive or resized.
            commands
                .entity(entity)
                .insert(TemporalAntiAliasHistoryTextures {
                    write: create("pet_taa_history_a"),
                    read: create("pet_taa_history_b"),
                    size,
                    valid: std::sync::atomic::AtomicBool::new(false),
                });
        }
    }
}

#[derive(Component)]
pub struct TemporalAntiAliasPipelineId(CachedRenderPipelineId);

fn prepare_taa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TaaPipeline>>,
    pipeline: Res<TaaPipeline>,
    views: Query<(Entity, &ExtractedView, &TemporalAntiAliasing)>,
) {
    for (entity, view, taa_settings) in &views {
        let mut pipeline_key = TaaPipelineKey {
            hdr: view.hdr,
            reset: taa_settings.reset,
        };
        let pipeline_id = pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key.clone());

        // Prepare non-reset pipeline anyways - it will be necessary next frame
        if pipeline_key.reset {
            pipeline_key.reset = false;
            pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key);
        }

        commands
            .entity(entity)
            .insert(TemporalAntiAliasPipelineId(pipeline_id));
    }
}

/// Clip-space translation gives the same subpixel shift at every depth.
fn jitter_projection(mut projection: Mat4, offset: Vec2, size: Vec2) -> Mat4 {
    if projection.w_axis.w == 1.0 {
        let shift = offset * Vec2::new(2., -2.) / size;
        projection.w_axis.x += shift.x;
        projection.w_axis.y += shift.y;
    } else {
        TemporalJitter { offset }.jitter_projection(&mut projection, size);
    }
    projection
}

// Bevy 0.17's orthographic TemporalJitter modifies z_axis (a depth-dependent
// shear). Upload a corrected clip-space translation while preserving unjittered
// matrices for motion vectors. The shared buffer also contains non-TAA views.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn prepare_orthographic_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<ViewUniforms>,
    views: Query<(
        Entity,
        Option<&ExtractedCamera>,
        &ExtractedView,
        Option<&Frustum>,
        Option<&TemporalJitter>,
        Option<&MipBias>,
    )>,
    frame_count: Res<FrameCount>,
    temporal_views: Query<(), With<TemporalAntiAliasing>>,
) {
    if temporal_views.is_empty() {
        return;
    }
    let view_iter = views.iter();
    let view_count = view_iter.len();
    let Some(mut writer) =
        view_uniforms
            .uniforms
            .get_writer(view_count, &render_device, &render_queue)
    else {
        return;
    };
    for (entity, extracted_camera, extracted_view, frustum, temporal_jitter, mip_bias) in &views {
        let viewport = extracted_view.viewport.as_vec4();
        let main_pass_viewport = viewport;

        let unjittered_projection = extracted_view.clip_from_view;
        let mut clip_from_view = unjittered_projection;

        if let Some(temporal_jitter) = temporal_jitter {
            clip_from_view = jitter_projection(
                clip_from_view,
                temporal_jitter.offset,
                main_pass_viewport.zw(),
            );
        }

        let view_from_clip = clip_from_view.inverse();
        let world_from_view = extracted_view.world_from_view.to_matrix();
        let view_from_world = world_from_view.inverse();

        let clip_from_world = if temporal_jitter.is_some() {
            clip_from_view * view_from_world
        } else {
            extracted_view
                .clip_from_world
                .unwrap_or_else(|| clip_from_view * view_from_world)
        };

        // Map Frustum type to shader array<vec4<f32>, 6>
        let frustum = frustum
            .map(|frustum| frustum.half_spaces.map(|h| h.normal_d()))
            .unwrap_or([Vec4::ZERO; 6]);

        let view_uniforms = ViewUniformOffset {
            offset: writer.write(&ViewUniform {
                clip_from_world,
                unjittered_clip_from_world: unjittered_projection * view_from_world,
                world_from_clip: world_from_view * view_from_clip,
                world_from_view,
                view_from_world,
                clip_from_view,
                view_from_clip,
                world_position: extracted_view.world_from_view.translation(),
                exposure: extracted_camera
                    .map(|c| c.exposure)
                    .unwrap_or_else(|| Exposure::default().exposure()),
                viewport,
                main_pass_viewport,
                frustum,
                color_grading: extracted_view.color_grading.clone().into(),
                mip_bias: mip_bias.unwrap_or(&MipBias(0.0)).0,
                frame_count: frame_count.0,
            }),
        };

        commands.entity(entity).insert(view_uniforms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orthographic_jitter_is_subpixel_and_independent_of_depth() {
        let projection = Mat4::orthographic_rh(-1.95, 1.95, -1.95, 1.95, 0.1, 1000.);
        let size = Vec2::new(512., 512.);
        let offset = Vec2::new(0.375, -0.2777778);
        let jittered = jitter_projection(projection, offset, size);
        for z in [-0.2, -3., -6., -25., -900.] {
            let point = Vec3::new(0.4, -0.6, z);
            let before = projection.project_point3(point);
            let after = jittered.project_point3(point);
            let pixels = (after - before).truncate() * size * Vec2::new(0.5, -0.5);
            assert!((pixels - offset).length() < 0.00002);
            assert_eq!(before.z, after.z);
        }
    }
}
