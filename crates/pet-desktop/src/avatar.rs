//! Asset-backed avatars. Scene instances retain native hierarchy, skins and morph targets.
use crate::runtime::Runtime;
use bevy::{
    anti_alias::fxaa::Fxaa,
    asset::RenderAssetUsages,
    camera::{ScalingMode, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    gltf::Gltf,
    mesh::VertexAttributeValues,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    scene::SceneInstanceReady,
};
use pet_core::{assets::Skin, behavior::State, sprite::SpritePack};
use std::{collections::HashMap, f32::consts::FRAC_PI_2};
#[derive(Resource)]
pub struct RenderViews {
    pub output: Handle<Image>,
    pub pet: Handle<Image>,
    pub preview: Handle<Image>,
    pub resolution: u32,
    pub output_size: u32,
    pub readback: Option<Entity>,
}
#[derive(Component)]
pub struct OutputCamera;
#[derive(Component)]
pub struct ViewCamera(pub bool);
#[derive(Component)]
pub struct Avatar {
    pub skin: Skin,
    pub preview: bool,
    pub ready: bool,
    pub state: State,
    pub clock: f32,
    pub expression_clock: f32,
    pub spin_angle: f32,
    pub spinning: bool,
    pub turning: bool,
    pub turn_clock: f32,
    pub resume_clock: f32,
    pub direction: i32,
    pub yaw: f32,
    pub turn_from: f32,
    pub turn_to: f32,
    pub players: Vec<Entity>,
    pub graph: Handle<AnimationGraph>,
    pub source: Handle<Gltf>,
    pub clips: HashMap<String, AnimationNodeIndex>,
    pub mapping: HashMap<String, String>,
    pub native: bool,
    pub profile: &'static str,
    pub ground: f32,
    pub sprite: Option<SpritePack>,
    pub frames: Vec<Handle<Image>>,
    pub material: Option<Handle<StandardMaterial>>,
    pub base: Transform,
    pub suspension: pet_core::interaction::Suspension,
    pub support_pivot: Vec3,
    pub reaction: u64,
    pub tail: pet_core::interaction::TailResponse,
    landing_serial: u64,
    drag_still_time: f32,
    drag_cursor: Option<Vec2>,
    drag_velocity: Vec2,
    release_yaw: Option<f32>,
    hanging_sway: f32,
}
impl Avatar {
    fn new(skin: Skin, preview: bool) -> Self {
        Self {
            skin,
            preview,
            ready: false,
            state: State::Idle,
            clock: 0.,
            expression_clock: 0.,
            spin_angle: 0.,
            spinning: false,
            turning: false,
            turn_clock: 0.,
            resume_clock: 1.,
            direction: 1,
            yaw: 0.,
            turn_from: 0.,
            turn_to: 0.,
            players: vec![],
            graph: default(),
            source: default(),
            clips: HashMap::new(),
            mapping: HashMap::new(),
            native: false,
            profile: "blob",
            ground: 248.,
            sprite: None,
            frames: vec![],
            material: None,
            base: Transform::IDENTITY,
            suspension: default(),
            support_pivot: Vec3::ZERO,
            reaction: 0,
            tail: default(),
            landing_serial: 0,
            drag_still_time: 0.,
            drag_cursor: None,
            drag_velocity: Vec2::ZERO,
            release_yaw: None,
            hanging_sway: 0.,
        }
    }
    pub fn resume_rate(&self) -> f32 {
        (self.resume_clock / 0.22).clamp(0., 1.)
    }
}
/// Viewer-facing angle: left is negative, right positive; speed is unscaled px/s.
fn drag_yaw(horizontal_speed: f32) -> f32 {
    if horizontal_speed.abs() < 20. {
        0.
    } else {
        (horizontal_speed / 900.).clamp(-1., 1.) * FRAC_PI_2
    }
}
#[derive(Component)]
pub struct Loading {
    handle: Handle<Gltf>,
}
#[derive(Component)]
pub struct PendingScene;
#[derive(Component)]
pub struct SceneReady;
#[derive(Component)]
pub struct SceneContainer;
pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut composites: ResMut<Assets<crate::composite::PetComposite>>,
    rt: Res<Runtime>,
) {
    let output_size = crate::layered::output_size(rt.prefs.zoom);
    let resolution = rt.prefs.performance.resolution.max(output_size);
    let pet = images.add(Image::new_target_texture(
        resolution,
        resolution,
        TextureFormat::bevy_default(),
    ));
    let preview = images.add(Image::new_target_texture(
        512,
        512,
        TextureFormat::bevy_default(),
    ));
    for (is_preview, target, layer) in [(false, pet.clone(), 1), (true, preview.clone(), 2)] {
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: -1,
                target: target.into(),
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical {
                    viewport_height: 3.9,
                },
                ..OrthographicProjection::default_3d()
            }),
            Transform::from_xyz(0., 0.05, 6.).looking_at(Vec3::new(0., 0.05, 0.), Vec3::Y),
            RenderLayers::layer(layer),
            Tonemapping::None,
            Msaa::Sample4,
            ViewCamera(is_preview),
        ));
        commands.spawn((
            DirectionalLight {
                illuminance: 6500.,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.)),
            RenderLayers::layer(layer),
        ));
    }
    let output = images.add(output_image(output_size));
    let readback = spawn_readback(&mut commands, &output, output_size);
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            target: if cfg!(windows) {
                output.clone().into()
            } else {
                default()
            },
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 320.,
            },
            ..OrthographicProjection::default_2d()
        }),
        RenderLayers::layer(0),
        OutputCamera,
    ));
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(300., 300.))),
        MeshMaterial2d(composites.add(crate::composite::PetComposite { image: pet.clone() })),
        Transform::default(),
        RenderLayers::layer(0),
    ));
    commands.insert_resource(RenderViews {
        output,
        pet,
        preview,
        resolution,
        output_size,
        readback,
    });
}
fn output_image(size: u32) -> Image {
    let mut image = Image::new_target_texture(size, size, TextureFormat::Rgba8UnormSrgb);
    image.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::COPY_SRC;
    image
}
fn spawn_readback(commands: &mut Commands, image: &Handle<Image>, size: u32) -> Option<Entity> {
    cfg!(windows).then(|| {
        commands
            .spawn((
                bevy::render::gpu_readback::Readback::texture(image.clone()),
                crate::layered::FrameSize(size),
            ))
            .observe(crate::layered::receive)
            .id()
    })
}
/// Replace the readback entity with its texture so delayed old callbacks can
/// never interpret a previous row pitch as the new window size.
pub fn resize_output(
    mut commands: Commands,
    rt: Res<Runtime>,
    mut views: ResMut<RenderViews>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<&mut Camera, With<OutputCamera>>,
) {
    let size = crate::layered::output_size(rt.prefs.zoom);
    if size == views.output_size {
        return;
    }
    if let Some(entity) = views.readback.take() {
        commands.entity(entity).despawn();
    }
    views.output = images.add(output_image(size));
    views.output_size = size;
    views.readback = spawn_readback(&mut commands, &views.output, size);
    for mut camera in &mut cameras {
        camera.target = views.output.clone().into();
    }
}
fn pixel_image(img: &image::RgbaImage) -> Image {
    Image::new(
        Extent3d {
            width: img.width(),
            height: img.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        img.as_raw().clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
pub fn request(
    mut commands: Commands,
    mut rt: ResMut<Runtime>,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    loading: Query<Entity, With<Loading>>,
) {
    let mut requests = vec![];
    if let Some(id) = rt.loading.take() {
        for e in &loading {
            commands.entity(e).despawn();
        }
        requests.push((id, false));
    }
    if rt.need_preview && rt.show_settings {
        rt.need_preview = false;
        requests.push((rt.prefs.skin_id.clone(), true));
    }
    for (id, preview) in requests {
        info!("Request avatar {id}, preview={preview}");
        let Some(skin) = rt.effective_skin(&id) else {
            rt.status = format!("找不到外观：{id}");
            continue;
        };
        let layer = if preview { 2 } else { 1 };
        let mut avatar = Avatar::new(skin.clone(), preview);
        avatar.landing_serial = rt.landing_serial;
        let root = commands
            .spawn((
                Transform::default(),
                Visibility::Hidden,
                RenderLayers::layer(layer),
            ))
            .id();
        match skin.r#type.as_str() {
            "model3d" => {
                let handle = if let Some(path) = crate::bundled::asset_path(&skin) {
                    server.load(path)
                } else {
                    let path = match pet_core::import::cached_model(&skin, &rt.data) {
                        Ok(path) => path,
                        Err(e) => {
                            rt.status = format!("模型加载失败：{e:#}");
                            commands.entity(root).despawn();
                            continue;
                        }
                    };
                    let relative = path.strip_prefix(&rt.data).unwrap();
                    server.load(format!(
                        "user://{}",
                        relative.to_string_lossy().replace(char::from(92), "/")
                    ))
                };
                avatar.source = handle.clone();
                commands.entity(root).insert((avatar, Loading { handle }));
            }
            "sprite" => match SpritePack::load(&skin) {
                Ok(pack) => {
                    avatar.ground = pack.ground_offset;
                    avatar.frames = pack
                        .frames
                        .iter()
                        .map(|f| images.add(pixel_image(f)))
                        .collect();
                    avatar.mapping = pack.mapping.clone();
                    for name in pack.clips.keys() {
                        avatar
                            .clips
                            .insert(name.clone(), AnimationNodeIndex::new(0));
                    }
                    let size = Vec2::new(pack.size[0] as f32, pack.size[1] as f32)
                        * (232. / pack.size[0].max(pack.size[1]) as f32)
                        * (3.9 / 300.);
                    let material = materials.add(StandardMaterial {
                        base_color_texture: Some(avatar.frames[0].clone()),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        cull_mode: None,
                        ..default()
                    });
                    avatar.material = Some(material.clone());
                    avatar.sprite = Some(pack);
                    avatar.ready = true;
                    commands.entity(root).with_children(|p| {
                        p.spawn((
                            Mesh3d(meshes.add(Rectangle::new(size.x, size.y))),
                            MeshMaterial3d(material),
                            Transform::from_xyz(0., 0.05 + 14. * 3.9 / 300., 0.),
                            RenderLayers::layer(layer),
                        ));
                    });
                    commands.entity(root).insert((avatar, SceneReady));
                }
                Err(e) => {
                    commands.entity(root).despawn();
                    rt.status = format!("外观加载失败：{e}");
                }
            },
            _ => {
                let color = Color::srgb_u8(243, 237, 225);
                let color = skin
                    .color
                    .as_ref()
                    .and_then(|c| bevy::color::Srgba::hex(c).ok())
                    .map(Color::from)
                    .unwrap_or(color);
                let body = materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.8,
                    ..default()
                });
                let eye = materials.add(Color::srgb(0.07, 0.06, 0.08));
                let highlight = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    unlit: true,
                    ..default()
                });
                let blush = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.95, 0.61, 0.64),
                    unlit: true,
                    ..default()
                });
                commands.entity(root).with_children(|p| {
                    p.spawn((
                        Mesh3d(meshes.add(Sphere::new(1.))),
                        MeshMaterial3d(body.clone()),
                        Transform::from_scale(Vec3::new(1.05, 0.85, 0.8)),
                        RenderLayers::layer(layer),
                    ));
                    for x in [-0.36, 0.36] {
                        p.spawn((
                            crate::effects::BubbleEye,
                            Mesh3d(meshes.add(Sphere::new(0.085))),
                            MeshMaterial3d(eye.clone()),
                            Transform::from_xyz(x, 0.13, 0.75),
                            RenderLayers::layer(layer),
                        ))
                        .with_children(|eye| {
                            eye.spawn((
                                Mesh3d(meshes.add(Sphere::new(0.025))),
                                MeshMaterial3d(highlight.clone()),
                                Transform::from_xyz(-0.022, 0.027, 0.074),
                                RenderLayers::layer(layer),
                            ));
                        });
                        p.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.1))),
                            MeshMaterial3d(blush.clone()),
                            Transform::from_xyz(x * 1.45, -0.05, 0.70)
                                .with_scale(Vec3::new(1., 0.5, 0.2)),
                            RenderLayers::layer(layer),
                        ));
                        p.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.28))),
                            MeshMaterial3d(body.clone()),
                            Transform::from_xyz(x, -0.75, 0.15).with_scale(Vec3::new(1., 0.6, 1.4)),
                            RenderLayers::layer(layer),
                        ));
                    }
                    for (x, angle) in [(-0.04, -0.45), (0.04, 0.45)] {
                        p.spawn((
                            Mesh3d(meshes.add(Capsule3d::new(0.014, 0.08))),
                            MeshMaterial3d(eye.clone()),
                            Transform::from_xyz(x, -0.08, 0.8)
                                .with_rotation(Quat::from_rotation_z(angle)),
                            RenderLayers::layer(layer),
                        ));
                    }
                });
                avatar.ready = true;
                avatar.ground = 160. + 0.92 * 300. / 3.9;
                commands.entity(root).insert((avatar, SceneReady));
            }
        }
    }
}
pub fn load_scenes(
    mut commands: Commands,
    mut rt: ResMut<Runtime>,
    gltfs: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut loading: Query<(Entity, &Loading, &mut Avatar)>,
) {
    for (e, loading, mut a) in &mut loading {
        if let Some(gltf) = gltfs.get(&loading.handle) {
            let Some(scene) = gltf
                .default_scene
                .clone()
                .or_else(|| gltf.scenes.first().cloned())
            else {
                rt.status = "模型中没有场景".into();
                commands.entity(e).despawn();
                continue;
            };
            let mut names = gltf
                .named_animations
                .keys()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            names.sort();
            let (graph, indices) = AnimationGraph::from_clips(
                names
                    .iter()
                    .map(|n| gltf.named_animations[n.as_str()].clone()),
            );
            a.graph = graphs.add(graph);
            a.clips = names.iter().cloned().zip(indices).collect();
            a.mapping = pet_core::action::resolve(&names, &a.skin.animations);
            let r = a.skin.rotation_degrees;
            let scene_entity = commands
                .spawn((
                    SceneRoot(scene),
                    SceneContainer,
                    Visibility::Inherited,
                    Transform::from_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        r[0].to_radians(),
                        r[1].to_radians(),
                        r[2].to_radians(),
                    )),
                ))
                .observe(
                    |event: On<SceneInstanceReady>,
                     parents: Query<&ChildOf>,
                     pending: Query<(), With<PendingScene>>,
                     mut commands: Commands| {
                        if let Ok(parent) = parents.get(event.entity)
                            && pending.contains(parent.parent())
                        {
                            info!(
                                "Scene ready {:?}, avatar {:?}",
                                event.entity,
                                parent.parent()
                            );
                            commands.entity(parent.parent()).insert(SceneReady);
                        }
                    },
                )
                .id();
            commands
                .entity(e)
                .add_child(scene_entity)
                .remove::<Loading>()
                .insert(PendingScene);
        } else if let Some(bevy::asset::LoadState::Failed(err)) =
            server.get_load_state(loading.handle.id())
        {
            rt.status = format!("模型加载失败：{err}");
            commands.entity(e).despawn();
        }
    }
}
// Run after global transform propagation, before animation begins on the new instance.
// Bevy injects each resource and query as a system parameter.
#[allow(clippy::too_many_arguments)]
pub fn finish(
    mut commands: Commands,
    mut rt: ResMut<Runtime>,
    mut avatars: Query<(Entity, &mut Avatar, Option<&PendingScene>), With<SceneReady>>,
    children: Query<&Children>,
    spatial: Query<(&GlobalTransform, Option<&Mesh3d>, Option<&Name>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    players: Query<(), With<AnimationPlayer>>,
    mut transforms: Query<&mut Transform, With<SceneContainer>>,
    material_handles: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut toon_materials: ResMut<Assets<crate::toon::ToonMaterial>>,
    gltfs: Res<Assets<Gltf>>,
    rest_spatial: Query<(&Transform, &GlobalTransform, Option<&Name>), Without<SceneContainer>>,
    skins: Query<&bevy::mesh::skinning::SkinnedMesh>,
    morphs: Query<&bevy::mesh::morph::MorphWeights>,
    names: Query<&Name>,
) {
    for (root, mut a, pending) in &mut avatars {
        info!(
            "Finish avatar {root:?}, preview={}, pending={}, ready={}",
            a.preview,
            pending.is_some(),
            a.ready
        );
        if a.ready && pending.is_some() {
            commands
                .entity(root)
                .remove::<SceneReady>()
                .remove::<PendingScene>();
            continue;
        }
        let descendants = children.iter_descendants(root).collect::<Vec<_>>();
        crate::expressions::capture(
            &mut commands,
            root,
            &descendants,
            &rest_spatial,
            &skins,
            &morphs,
            &meshes,
        );
        crate::effects::install(
            &mut commands,
            root,
            a.preview,
            &descendants,
            &names,
            &mut meshes,
            &mut materials,
        );
        if a.skin.id == "builtin.shu" {
            crate::suspension::capture(&mut commands, root, &descendants, &rest_spatial);
            crate::face::install(
                &mut commands,
                root,
                a.preview,
                &descendants,
                &spatial,
                &mut meshes,
                &mut materials,
            );
        }
        let layer = if a.preview { 2 } else { 1 };
        let toon = a.skin.options.get("render_style").and_then(|v| v.as_str()) == Some("toon");
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for &e in &descendants {
            if toon {
                commands.entity(e).insert(crate::toon::ToonStyled);
                if let Ok(handle) = material_handles.get(e)
                    && let Some(base) = materials.get(&handle.0)
                {
                    let mut base = StandardMaterial {
                        normal_map_texture: None,
                        ..base.clone()
                    };
                    if let Some(name) = gltfs.get(&a.source).and_then(|g| {
                        g.named_materials
                            .iter()
                            .find(|(_, h)| h.id() == handle.0.id())
                            .map(|(n, _)| n.as_ref())
                    }) {
                        let colour = if name.contains("Ivory short pile") {
                            Some(Color::srgb(0.94, 0.92, 0.84))
                        } else if name.contains("Harvest yellow") {
                            Some(Color::srgb(0.91, 0.73, 0.29))
                        } else if name.contains("Indigo padded tail") {
                            Some(Color::srgb(0.25, 0.22, 0.47))
                        } else {
                            None
                        };
                        if let Some(colour) = colour {
                            base.base_color = colour;
                            base.base_color_texture = None;
                        }
                    }
                    let material = toon_materials.add(crate::toon::ToonMaterial {
                        base,
                        extension: default(),
                    });
                    commands
                        .entity(e)
                        .remove::<MeshMaterial3d<StandardMaterial>>()
                        .insert((MeshMaterial3d(material), crate::toon::ToonStyled));
                }
            }
            commands
                .entity(e)
                .insert_if_new(Visibility::Inherited)
                .insert(RenderLayers::layer(layer))
                .remove::<Camera>()
                .remove::<DirectionalLight>()
                .remove::<PointLight>()
                .remove::<SpotLight>();
            if players.contains(e) {
                commands.entity(e).insert((
                    AnimationGraphHandle(a.graph.clone()),
                    AnimationTransitions::new(),
                ));
                a.players.push(e);
            }
            if let Ok((global, Some(mesh), _)) = spatial.get(e)
                && let Some(VertexAttributeValues::Float32x3(points)) = meshes
                    .get(&mesh.0)
                    .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
            {
                for p in points {
                    let p = global.transform_point(Vec3::from(*p));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
        if pending.is_some() {
            let size = max - min;
            let longest = size.max_element();
            if !longest.is_finite() || longest <= 0.00001 {
                rt.status = "模型缺少有效网格尺寸".into();
                commands.entity(root).despawn();
                continue;
            }
            let scale = 2.35 / longest;
            // Use the planted feet, excluding the long tail from the spin axis.
            let feet = descendants
                .iter()
                .filter_map(|e| {
                    let (_, global, name) = rest_spatial.get(*e).ok()?;
                    let name = name?;
                    ["FrontFoot.L", "FrontFoot.R", "RearFoot.L", "RearFoot.R"]
                        .contains(&name.as_str())
                        .then_some(global.translation())
                })
                .collect::<Vec<_>>();
            if feet.len() == 4 {
                let centre = feet.iter().copied().sum::<Vec3>() / 4.;
                a.support_pivot = (centre - (min + max) * 0.5) * scale;
                a.support_pivot.y = 0.;
            }
            for &e in &descendants {
                if let Ok(mut transform) = transforms.get_mut(e) {
                    transform.scale = Vec3::splat(scale);
                    transform.translation = -(min + max) * 0.5 * scale;
                }
            }
            a.ground = 160. + (0.05 + size.y * 0.5 * scale) * 300. / 3.9;
        }
        a.ready = true;
        let bones = descendants
            .iter()
            .filter_map(|e| skins.get(*e).ok())
            .flat_map(|skin| skin.joints.iter())
            .filter_map(|e| names.get(*e).ok())
            .map(|name| name.as_str().to_owned())
            .collect::<Vec<_>>();
        a.profile =
            pet_core::action::profile_for(&a.skin.r#type, max - min, &bones, &a.skin.profile);
        commands
            .entity(root)
            .remove::<SceneReady>()
            .remove::<PendingScene>()
            .insert((Visibility::Visible, crate::soft_body::InstallSoftBody));
        if a.preview {
            if let Some(old) = rt.preview.replace(root).filter(|old| *old != root) {
                commands.entity(old).despawn();
            }
        } else {
            if let Some(old) = rt.active.replace(root).filter(|old| *old != root) {
                commands.entity(old).despawn();
            }
            rt.ground = a.ground;
            rt.prefs.skin_id = a.skin.id.clone();
            rt.desktop.extent = Vec2::new(320. * rt.prefs.zoom, rt.ground * rt.prefs.zoom);
            // Normalization centers model bounds at y=0; the camera looks at y=0.05.
            // Permit transparent top padding offscreen so the visible head reaches the edge.
            rt.desktop.top_inset = if a.sprite.is_none() {
                (320. + 0.1 * 300. / 3.9 - a.ground).max(0.) * rt.prefs.zoom
            } else {
                0.
            };
            rt.position = rt.desktop.clamp_origin(rt.position);
            rt.dirty = true;
            rt.need_preview = true;
            rt.status = String::new();
        }
    }
}
pub fn animate(
    time: Res<Time>,
    rt: Res<Runtime>,
    mut avatars: Query<(&mut Avatar, &mut Transform)>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs().min(0.1);
    for (mut a, mut transform) in &mut avatars {
        let preview_visible = rt.show_settings && (rt.page == 0 || rt.page == 3);
        if a.preview {
            for &entity in &a.players {
                if let Ok((mut player, _)) = players.get_mut(entity) {
                    if preview_visible {
                        player.resume_all();
                    } else {
                        player.pause_all();
                    }
                }
            }
        }
        if !a.ready || a.preview && !preview_visible {
            continue;
        }
        let desired = if a.preview {
            rt.preview_state
        } else {
            rt.animation_state()
        };
        let direction = if a.preview { 1 } else { rt.brain.direction };
        let reaction = if a.preview {
            rt.pet_reaction.max(1)
        } else {
            rt.pet_reaction
        };
        let clicked = desired == State::Petted && a.reaction != reaction;
        let changed = a.state != desired || a.clock == 0. || clicked;
        if desired == State::Petted {
            a.reaction = reaction;
        }
        if changed {
            a.state = desired;
            a.clock = 0.;
            a.turning = false;
        }
        a.clock += dt;
        a.expression_clock += dt;
        a.resume_clock += dt;
        let position = if a.preview {
            Vec2::new((a.clock * 1.7).sin() * 28., (a.clock * 1.3).cos() * 10.)
        } else {
            rt.position / rt.prefs.zoom
        };
        a.suspension.advance(
            dt,
            position,
            desired == State::Dragged,
            desired == State::Airborne,
        );
        let speed = a.suspension.speed;
        if !a.preview && a.landing_serial != rt.landing_serial {
            a.landing_serial = rt.landing_serial;
            if desired != State::Dragged {
                a.tail.impact(rt.landing_speed);
            }
        }
        if desired == State::Dragged && a.tail.contact_weight() > 0. {
            a.tail.cancel_contact();
        }
        let airborne = desired == State::Airborne
            && (a.preview || !rt.motion.active || !rt.physically_supported());
        a.tail
            .advance(dt, desired == State::Dragged || airborne, speed);
        let is_sprite = a.sprite.is_some();
        let held_drag = !a.preview && rt.pressed && rt.dragging && desired == State::Dragged;
        if desired != State::Airborne {
            a.release_yaw = None;
        } else if a.drag_cursor.is_some() && !held_drag {
            a.release_yaw = Some(a.yaw);
        }
        if held_drag {
            let cursor = rt.drag_cursor;
            let previous = a.drag_cursor.replace(cursor).unwrap_or(cursor);
            let measured =
                ((cursor - previous) / (dt.max(0.0001) * rt.prefs.zoom)).clamp_length_max(2200.);
            a.drag_velocity = a.drag_velocity.lerp(measured, 1. - (-dt * 18.).exp());
        } else {
            a.drag_cursor = None;
            a.drag_velocity = Vec2::ZERO;
        }
        // Allow short pauses and hand jitter before looking toward the viewer.
        // Use pointer motion: screen limits can reposition the body as its silhouette turns.
        a.drag_still_time = if held_drag && a.drag_velocity.x.abs() < 35. {
            a.drag_still_time + dt
        } else {
            0.
        };
        let side_on = desired.moving() || matches!(desired, State::Dragged | State::Airborne);
        let target = if held_drag {
            if a.drag_still_time >= 0.18 {
                0.
            } else {
                drag_yaw(a.drag_velocity.x)
            }
        } else if let Some(yaw) = a.release_yaw {
            yaw
        } else if side_on {
            direction as f32 * FRAC_PI_2
        } else {
            direction as f32 * 0.32
        };
        if !is_sprite && desired.moving() && a.direction != direction && !a.turning {
            a.turning = true;
            a.turn_clock = 0.;
            a.turn_from = a.yaw;
            a.turn_to = target;
        }
        let turn_started = a.turning && a.turn_clock == 0.;
        if a.turning {
            a.turn_clock += dt;
            let duration = if desired == State::Run { 0.66 } else { 0.8 };
            let t = (a.turn_clock / duration).clamp(0., 1.);
            let smooth = t * t * (3. - 2. * t);
            a.yaw = a.turn_from + (a.turn_to - a.turn_from) * smooth;
            if t >= 1. {
                a.turning = false;
                a.resume_clock = 0.;
            }
        } else if held_drag {
            // Facing changes independently of the looping hanging animation.
            a.yaw += (target - a.yaw) * (1. - (-dt * 9.).exp());
        } else if side_on {
            a.yaw = target;
        } else {
            a.yaw += (target - a.yaw) * (dt * 6.).min(1.);
        }
        a.direction = direction;
        let sway = if held_drag && a.drag_still_time >= 0.18 {
            let front = 1. - (a.yaw.abs() / FRAC_PI_2).clamp(0., 1.);
            (a.clock * std::f32::consts::TAU / 2.8).sin() * 0.028 * front
        } else {
            0.
        };
        a.hanging_sway += (sway - a.hanging_sway) * (1. - (-dt * 6.).exp());
        let spin_phase = (a.clock - 1.4).rem_euclid(rt.prefs.behavior.spin_interval);
        let spinning = desired == State::Idle
            && rt.prefs.behavior.idle_spin
            && a.clock >= 1.4
            && spin_phase < 2.6
            && !is_sprite;
        let spin_changed = a.spinning != spinning;
        a.spinning = spinning;
        if spinning {
            let t = spin_phase / 2.6;
            a.spin_angle = std::f32::consts::TAU * t * t * (3. - 2. * t);
        } else {
            let angle = (a.spin_angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            a.spin_angle = if side_on {
                0.
            } else {
                angle * (-dt * 10.).exp()
            };
        }
        let clip = if (a.turning || spinning) && a.clips.contains_key("Turn") {
            Some("Turn".to_string())
        } else {
            a.mapping.get(desired.key()).cloned()
        };
        a.native = clip.is_some();
        if changed || spin_changed || turn_started || (!a.turning && a.resume_clock < dt * 1.5) {
            for &entity in &a.players {
                if let Ok((mut player, mut transitions)) = players.get_mut(entity) {
                    if let Some(index) = clip.as_ref().and_then(|s| a.clips.get(s)) {
                        let active = transitions.play(
                            &mut player,
                            *index,
                            std::time::Duration::from_millis(160),
                        );
                        if clicked {
                            active.replay();
                        }
                        if desired.looping() && !a.turning {
                            active.repeat();
                        }
                    } else {
                        player.stop_all();
                    }
                }
            }
        }
        *transform = a.base;
        if is_sprite {
            if a.skin.flip_with_direction && direction < 0 {
                transform.scale.x = -1.;
            }
            if let Some(pack) = &a.sprite {
                let frame = pack.frame_at(desired, a.clock, &a.skin.animations);
                if let Some(mat) = a.material.as_ref().and_then(|h| materials.get_mut(h)) {
                    mat.base_color_texture = Some(a.frames[frame].clone());
                }
            }
        } else {
            transform.rotation = Quat::from_rotation_y(a.yaw + a.spin_angle);
        }
        {
            let t = a.clock;
            let stride = (t * if desired == State::Run { 10.75 } else { 5.25 }).sin();
            let (sx, sy, bob, lean) = match desired {
                State::Idle => (1., 1. + (t * 2.2).sin() * 0.012, 0., 0.),
                State::Walk | State::Run => (
                    1. - stride.abs() * 0.025,
                    1. + stride.abs() * 0.035,
                    stride.abs() * 0.045,
                    stride * 0.035,
                ),
                State::Sleep => {
                    let settle = (t / 0.8).clamp(0., 1.);
                    let breath = (t * std::f32::consts::TAU / 4.4).sin();
                    (
                        1. + settle * (0.025 - breath * 0.006),
                        1. - settle * (0.06 - breath * 0.012),
                        0.,
                        settle * (0.025 + breath * 0.012),
                    )
                }
                State::Petted => (
                    1.,
                    1. + (t * 10.).sin() * (-t).exp() * 0.08,
                    (t * 8.).sin().abs() * (-t).exp() * 0.10,
                    0.,
                ),
                State::Dragged | State::Airborne => (0.92, 1.12, 0., -0.08 * direction as f32),
                State::Fall => {
                    let s = (t * 12.).sin() * (-t * 5.).exp();
                    (1. + s * 0.14, 1. - s * 0.2, 0., 0.)
                }
            };
            let mut amplitude = match a.profile {
                "character" => 0.32,
                "flat" => 0.7,
                _ => 1.,
            };
            if a.native {
                amplitude *= if desired.moving() {
                    0.
                } else if desired == State::Sleep && a.skin.id == "builtin.shu" {
                    0.65
                } else {
                    0.12
                };
            }
            let sx = 1. + (sx - 1.) * amplitude;
            let sy = 1. + (sy - 1.) * amplitude;
            let bob = bob * amplitude;
            let lean = lean * amplitude;
            transform.scale *= Vec3::new(sx, sy, 1.);
            let foot = (160. - a.ground) * 3.9 / 300.;
            transform.translation.y = foot * (1. - sy) + bob;
            transform.rotation *= Quat::from_rotation_z(lean);
        }
        if a.skin.id == "builtin.shu" {
            let pivot = a.support_pivot * transform.scale;
            transform.translation += Quat::from_rotation_y(a.yaw) * pivot
                - Quat::from_rotation_y(a.yaw + a.spin_angle) * pivot;
            let response = &a.suspension;
            // Rotate in the screen plane around the actual grab point, independently of yaw.
            let pivot = if a.preview {
                Vec2::new(160., 125.)
            } else {
                rt.grab_point
            };
            let pivot = Vec3::new(
                (pivot.x - 160.) * 3.9 / 300.,
                (160. - pivot.y) * 3.9 / 300. + 0.05,
                0.,
            );
            let rotation = Quat::from_rotation_z(response.roll + a.hanging_sway);
            let stretch = Vec3::new(1. - response.stretch * 0.4, 1. + response.stretch, 1.);
            transform.translation = pivot
                + rotation * ((transform.translation - pivot) * stretch)
                + Vec3::new(response.lag.x, -response.lag.y, 0.);
            transform.rotation = rotation * transform.rotation;
            transform.scale *= stretch;
        }
    }
}
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Bevy camera and resource parameters.
pub fn performance(
    mut commands: Commands,
    rt: Res<Runtime>,
    mut views: ResMut<RenderViews>,
    mut images: ResMut<Assets<Image>>,
    mut composites: ResMut<Assets<crate::composite::PetComposite>>,
    mut cameras: Query<(
        Entity,
        &ViewCamera,
        &mut Camera,
        &mut Msaa,
        Option<&Fxaa>,
        Option<&bevy::pbr::ScreenSpaceAmbientOcclusion>,
        Option<&mut crate::temporal::TemporalAntiAliasing>,
    )>,
    mut lights: Query<&mut DirectionalLight>,
    mut shadow_map: ResMut<bevy::light::DirectionalLightShadowMap>,
    mut meshes: Query<(&Name, &mut Visibility, Option<&crate::toon::ToonStyled>)>,
    avatars: Query<(Entity, &Avatar)>,
    mut previous: Local<Option<(u32, bool, Option<Entity>, Option<Entity>, i32, State, State)>>,
) {
    let p = &rt.prefs.performance;
    let resolution = p.resolution.max(views.output_size);
    let active = rt.active.and_then(|e| avatars.get(e).ok());
    let preview = avatars
        .iter()
        .find(|(_, a)| a.preview && a.ready)
        .map(|(e, _)| e);
    let key = (
        resolution,
        p.ao,
        active.map(|(e, _)| e),
        preview,
        rt.brain.direction,
        rt.brain.state,
        rt.preview_state,
    );
    let reset = previous.as_ref() != Some(&key);
    *previous = Some(key);
    if resolution != views.resolution {
        if let Some(image) = images.get_mut(&views.pet) {
            image.resize(Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 1,
            });
        }
        // Resizing replaces the GPU texture. Rebuild material bind groups so
        // presentation samples the new target rather than the last old frame.
        for (_, composite) in composites.iter_mut() {
            composite.image = views.pet.clone();
        }
        views.resolution = resolution;
    }
    for (e, view, mut camera, mut msaa, fxaa, ao, taa) in &mut cameras {
        let was_active = camera.is_active;
        camera.is_active = !view.0 || (rt.show_settings && (rt.page == 0 || rt.page == 3));
        let next_msaa = match p.aa.as_str() {
            "msaa2" => Msaa::Sample2,
            "msaa4" => Msaa::Sample4,
            "msaa8" => Msaa::Sample8,
            _ => Msaa::Off,
        };
        // SSAO requires single-sample prepasses; TAA also smooths coverage alpha.
        let use_taa = p.aa == "taa" || (p.ao && p.aa.starts_with("msaa"));
        msaa.set_if_neq(if p.ao { Msaa::Off } else { next_msaa });
        let ao_quality = match p.quality {
            0 => bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::Low,
            1 => bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            2 => bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::High,
            _ => bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        };
        if p.ao && ao.is_none_or(|ao| ao.quality_level != ao_quality) {
            commands
                .entity(e)
                .insert(bevy::pbr::ScreenSpaceAmbientOcclusion {
                    quality_level: ao_quality,
                    ..default()
                });
        }
        if !p.ao && ao.is_some() {
            commands.entity(e).remove::<(
                bevy::pbr::ScreenSpaceAmbientOcclusion,
                bevy::core_pipeline::prepass::NormalPrepass,
            )>();
        }
        if use_taa {
            if let Some(mut taa) = taa {
                taa.reset |= reset || was_active != camera.is_active;
            } else {
                commands
                    .entity(e)
                    .insert(crate::temporal::TemporalAntiAliasing::default());
            }
        } else if taa.is_some() {
            commands.entity(e).remove::<(
                crate::temporal::TemporalAntiAliasing,
                bevy::render::camera::TemporalJitter,
                bevy::render::camera::MipBias,
                bevy::core_pipeline::prepass::MotionVectorPrepass,
            )>();
        }
        if !use_taa && !p.ao {
            commands
                .entity(e)
                .remove::<bevy::core_pipeline::prepass::DepthPrepass>();
        }
        let use_fxaa = p.aa == "fxaa";
        if fxaa.is_none_or(|fxaa| fxaa.enabled != use_fxaa) {
            commands.entity(e).insert(Fxaa {
                enabled: use_fxaa,
                ..default()
            });
        }
    }
    if shadow_map.size != p.shadow_map_size() {
        shadow_map.size = p.shadow_map_size();
    }
    for mut light in &mut lights {
        light.shadows_enabled = p.quality >= 2;
    }
    for (name, mut visibility, toon) in &mut meshes {
        if name.as_str().contains("MicroPile") {
            *visibility = if p.quality == 0 || toon.is_some() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercise Shu-specific motion without depending on any private model files.
    fn motion_test_skin() -> Skin {
        serde_json::from_value(serde_json::json!({
            "schema_version": 1, "id": "builtin.shu", "name": "Motion fixture",
            "type": "model3d", "asset": "fixture.glb",
            "animations": {"idle": "Idle", "walk": "Walk", "run": "Run",
                "sleep": "Sleep", "petted": "Happy", "dragged": "PickedUp",
                "airborne": "PickedUp", "fall": "Land"}
        }))
        .unwrap()
    }

    #[test]
    fn real_motion_impacts_reach_the_avatar_during_rebounds_and_floor_sliding() {
        let root = std::env::temp_dir().join(format!("lpp-bounce-tail-{}", std::process::id()));
        let mut rt = Runtime::new(root.clone(), true);
        rt.poll = -1000.; // Keep the deterministic test desktop throughout this simulation.
        rt.prefs.roaming = false;
        rt.brain.change(State::Airborne, 60.);
        rt.desktop = pet_core::desktop::Desktop {
            areas: vec![pet_core::desktop::Rect::new(0., 0., 1920., 900.)],
            ..default()
        };
        rt.position = Vec2::new(300., 200.);
        rt.motion.launch(rt.position, Vec2::new(1600., 1200.));
        let mut avatar = Avatar::new(motion_test_skin(), false);
        avatar.ready = true;
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(1. / 60.));
        app.insert_resource(time)
            .insert_resource(rt)
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, (crate::runtime::simulate, animate).chain());
        let entity = app.world_mut().spawn((avatar, Transform::default())).id();
        let mut seen = 0;
        let mut airborne_contacts = 0;
        let mut slide_seen = false;
        for _ in 0..300 {
            let before = app.world().get::<Avatar>(entity).unwrap().tail.pitch;
            app.update();
            let rt = app.world().resource::<Runtime>();
            let avatar = app.world().get::<Avatar>(entity).unwrap();
            if rt.landing_serial != seen {
                seen = rt.landing_serial;
                assert_eq!(avatar.landing_serial, seen);
                assert!(avatar.tail.contact_weight() > 0.5);
                assert!(
                    avatar.tail.pitch < before - 0.02,
                    "each physical impact must lower the tail"
                );
                if avatar.state == State::Airborne {
                    airborne_contacts += 1;
                }
            }
            if rt.motion.active && rt.physically_supported() {
                slide_seen = true;
                assert!(
                    avatar.tail.pitch < -0.1,
                    "tail must not lift during grounded sliding"
                );
            }
        }
        assert!(seen >= 3 && airborne_contacts >= 2);
        assert!(slide_seen);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn idle_spin_completes_a_full_turn_in_place_and_drag_interrupts() {
        let root = std::env::temp_dir().join(format!("lpp-spin-{}", std::process::id()));
        let mut rt = Runtime::new(root.clone(), true);
        rt.prefs.behavior.idle_spin = true;
        rt.brain.change(State::Idle, 120.);
        let skin = motion_test_skin();
        let mut avatar = Avatar::new(skin, false);
        avatar.ready = true;
        avatar.support_pivot = Vec3::new(0.05, 0., 0.43);
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(1. / 60.));
        app.insert_resource(time)
            .insert_resource(rt)
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, animate);
        let entity = app.world_mut().spawn((avatar, Transform::default())).id();
        let mut max_angle = 0_f32;
        for _ in 0..260 {
            app.update();
            let a = app.world().get::<Avatar>(entity).unwrap();
            max_angle = max_angle.max(a.spin_angle);
            let transform = app.world().get::<Transform>(entity).unwrap();
            let foot_centre = transform.transform_point(a.support_pivot);
            let expected = Quat::from_rotation_y(a.yaw) * a.support_pivot;
            assert!((foot_centre.x - expected.x).abs() < 0.0001);
            assert!((foot_centre.z - expected.z).abs() < 0.0001);
        }
        assert!(max_angle > 6.27, "must complete a full turn");
        let a = app.world().get::<Avatar>(entity).unwrap();
        assert!(!a.spinning && a.spin_angle.abs() < 0.01);
        app.world_mut()
            .get_mut::<Avatar>(entity)
            .unwrap()
            .spin_angle = 3.;
        app.world_mut()
            .resource_mut::<Runtime>()
            .brain
            .change(State::Dragged, 10.);
        app.update();
        let a = app.world().get::<Avatar>(entity).unwrap();
        assert!(!a.spinning);
        assert_eq!(a.spin_angle, 0.);
        app.world_mut()
            .resource_mut::<Runtime>()
            .brain
            .change(State::Petted, 1.8);
        app.world_mut().resource_mut::<Runtime>().pet_reaction = 1;
        for _ in 0..30 {
            app.update();
        }
        assert!(app.world().get::<Avatar>(entity).unwrap().clock > 0.4);
        app.world_mut().resource_mut::<Runtime>().pet_reaction = 2;
        app.update();
        let a = app.world().get::<Avatar>(entity).unwrap();
        assert_eq!(a.reaction, 2);
        assert!(
            a.clock < 0.02,
            "repeated click must restart the reaction immediately"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn taa_and_ao_share_depth_and_remove_jitter_when_disabled() {
        use crate::temporal::TemporalAntiAliasing;
        use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
        let root = std::env::temp_dir().join(format!("lpp-aa-{}", std::process::id()));
        let rt = Runtime::new(root.clone(), true);
        let mut app = App::new();
        app.insert_resource(rt)
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<crate::composite::PetComposite>>()
            .init_resource::<bevy::light::DirectionalLightShadowMap>()
            .insert_resource(RenderViews {
                output: default(),
                pet: default(),
                preview: default(),
                resolution: 512,
                output_size: 320,
                readback: None,
            })
            .add_systems(Update, performance);
        let entity = app
            .world_mut()
            .spawn((Camera3d::default(), ViewCamera(false), Msaa::Off))
            .id();
        for (aa, ao, expect_taa) in [
            ("taa", true, true),
            ("taa", false, true),
            ("off", true, false),
            ("msaa4", true, true),
            ("msaa4", false, false),
            ("taa", false, true),
            ("fxaa", false, false),
        ] {
            let mut rt = app.world_mut().resource_mut::<Runtime>();
            rt.prefs.performance.aa = aa.into();
            rt.prefs.performance.ao = ao;
            app.update();
            let camera = app.world().entity(entity);
            assert_eq!(camera.contains::<TemporalAntiAliasing>(), expect_taa);
            assert_eq!(
                camera.contains::<bevy::render::camera::TemporalJitter>(),
                expect_taa
            );
            assert_eq!(
                camera.contains::<bevy::render::camera::MipBias>(),
                expect_taa
            );
            assert_eq!(camera.contains::<MotionVectorPrepass>(), expect_taa);
            assert_eq!(camera.contains::<DepthPrepass>(), expect_taa || ao);
            assert_eq!(camera.contains::<NormalPrepass>(), ao);
            if expect_taa {
                assert_eq!(*camera.get::<Msaa>().unwrap(), Msaa::Off);
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dragging_and_throwing_face_fully_sideways_in_both_directions() {
        let root = std::env::temp_dir().join(format!("lpp-facing-{}", std::process::id()));
        let mut rt = Runtime::new(root.clone(), true);
        rt.show_settings = false;
        let skin = motion_test_skin();
        let mut avatar = Avatar::new(skin, false);
        avatar.ready = true;
        avatar.state = State::Walk;
        avatar.turning = true;
        avatar.yaw = 0.7;
        avatar.mapping = avatar.skin.animations.clone();
        let pickup = AnimationNodeIndex::new(0);
        let turn = AnimationNodeIndex::new(1);
        avatar.clips.insert("PickedUp".into(), pickup);
        avatar.clips.insert("Turn".into(), turn);
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(1. / 60.));
        app.insert_resource(time)
            .insert_resource(rt)
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, animate);
        let player = app
            .world_mut()
            .spawn((AnimationPlayer::default(), AnimationTransitions::default()))
            .id();
        avatar.players.push(player);
        let entity = app.world_mut().spawn((avatar, Transform::default())).id();
        for state in [State::Dragged, State::Airborne] {
            // Reverse twice while held: neither a previous turn nor the old yaw may persist.
            for direction in [-1, 1, -1] {
                app.world_mut()
                    .resource_mut::<Runtime>()
                    .brain
                    .change(state, 60.);
                app.world_mut().resource_mut::<Runtime>().brain.direction = direction;
                app.update();
                let avatar = app.world().get::<Avatar>(entity).unwrap();
                let transform = app.world().get::<Transform>(entity).unwrap();
                let forward = transform.rotation * Vec3::Z;
                assert!((forward.x - direction as f32).abs() < 0.00001);
                assert!(
                    forward.z.abs() < 0.00001,
                    "must not retain a front-facing angle"
                );
                assert!(!avatar.turning);
                let player = app.world().get::<AnimationPlayer>(player).unwrap();
                assert!(player.is_playing_animation(pickup));
                assert!(!player.is_playing_animation(turn));
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn held_drag_faces_viewer_when_stopped_and_sideways_when_moving() {
        for hz in [30, 60, 144] {
            for zoom in [0.2, 1., 3.] {
                let root = std::env::temp_dir().join(format!(
                    "lpp-held-facing-{}-{hz}-{zoom}",
                    std::process::id()
                ));
                let mut rt = Runtime::new(root.clone(), true);
                rt.show_settings = false;
                rt.prefs.zoom = zoom;
                rt.pressed = true;
                rt.dragging = true;
                // Incidental state changes must not end the hanging animation.
                rt.brain.change(State::Idle, 1.);
                let mut avatar = Avatar::new(motion_test_skin(), false);
                avatar.ready = true;
                avatar.mapping = avatar.skin.animations.clone();
                let pickup = AnimationNodeIndex::new(0);
                avatar.clips.insert("PickedUp".into(), pickup);
                let mut app = App::new();
                let mut time = Time::<()>::default();
                time.advance_by(std::time::Duration::from_secs_f32(1. / hz as f32));
                app.insert_resource(time)
                    .insert_resource(rt)
                    .init_resource::<Assets<StandardMaterial>>()
                    .add_systems(Update, animate);
                let player = app
                    .world_mut()
                    .spawn((AnimationPlayer::default(), AnimationTransitions::default()))
                    .id();
                avatar.players.push(player);
                let entity = app.world_mut().spawn((avatar, Transform::default())).id();
                app.update();
                for (direction, speed) in [(-1, 90.), (1, 450.), (-1, 1200.), (1, 600.)] {
                    // Move for a second, including reversing after the previous pause.
                    for _ in 0..hz {
                        let mut rt = app.world_mut().resource_mut::<Runtime>();
                        rt.brain.direction = direction;
                        rt.position.x += direction as f32 * speed * zoom / hz as f32;
                        rt.drag_cursor.x += direction as f32 * speed * zoom / hz as f32;
                        app.update();
                    }
                    let side_yaw = drag_yaw(direction as f32 * speed);
                    assert!(
                        (app.world().get::<Avatar>(entity).unwrap().yaw - side_yaw).abs() < 0.002
                    );
                    // Deceleration eases the current angle toward the viewer.
                    for _ in 0..hz / 10 {
                        app.update();
                    }
                    assert!(app.world().get::<Avatar>(entity).unwrap().yaw.abs() < side_yaw.abs());
                    let mut previous_yaw = side_yaw.abs();
                    for _ in 0..hz {
                        app.update();
                        let avatar = app.world().get::<Avatar>(entity).unwrap();
                        assert_eq!(avatar.state, State::Dragged);
                        if avatar.drag_still_time >= 0.18 {
                            assert!(avatar.yaw.abs() <= previous_yaw + 0.00001);
                        }
                        previous_yaw = avatar.yaw.abs();
                        assert!(!avatar.spinning && !avatar.turning);
                        assert!(avatar.suspension.weight > 0.99);
                        assert!(
                            app.world()
                                .get::<AnimationPlayer>(player)
                                .unwrap()
                                .is_playing_animation(pickup)
                        );
                    }
                    let transform = app.world().get::<Transform>(entity).unwrap();
                    assert!(
                        (transform.rotation * Vec3::Z).z > 0.999,
                        "stopped drag must face viewer at {hz} FPS / {zoom} zoom"
                    );
                    // Small position jitter must not turn the character back sideways.
                    for frame in 0..hz {
                        let mut rt = app.world_mut().resource_mut::<Runtime>();
                        rt.drag_cursor.x += if frame % 2 == 0 { 0.15 } else { -0.15 } * zoom;
                        // Simulate silhouette-dependent screen clamping independently of input.
                        rt.position.x += if frame % 2 == 0 { 8. } else { -8. } * zoom;
                        app.update();
                        assert!(app.world().get::<Avatar>(entity).unwrap().yaw.abs() < 0.01);
                    }
                    assert!(
                        app.world()
                            .get::<Avatar>(entity)
                            .unwrap()
                            .hanging_sway
                            .abs()
                            > 0.001
                    );
                }
                let released_yaw = app.world().get::<Avatar>(entity).unwrap().yaw;
                {
                    let mut rt = app.world_mut().resource_mut::<Runtime>();
                    rt.pressed = false;
                    rt.dragging = false;
                    rt.brain.change(State::Airborne, 2.);
                }
                for _ in 0..hz {
                    app.update();
                    let avatar = app.world().get::<Avatar>(entity).unwrap();
                    assert_eq!(avatar.state, State::Airborne);
                    assert_eq!(avatar.drag_still_time, 0.);
                    assert!(avatar.drag_cursor.is_none());
                    assert!((avatar.yaw - released_yaw).abs() < 0.00001);
                }
                // Regrabbing clears the release heading; releasing during movement keeps it too.
                {
                    let mut rt = app.world_mut().resource_mut::<Runtime>();
                    rt.pressed = true;
                    rt.dragging = true;
                    rt.brain.direction = -1;
                }
                for _ in 0..hz {
                    app.world_mut().resource_mut::<Runtime>().drag_cursor.x -=
                        600. * zoom / hz as f32;
                    app.update();
                }
                let released_yaw = app.world().get::<Avatar>(entity).unwrap().yaw;
                assert!((released_yaw - drag_yaw(-600.)).abs() < 0.002);
                {
                    let mut rt = app.world_mut().resource_mut::<Runtime>();
                    rt.pressed = false;
                    rt.dragging = false;
                    rt.brain.direction = 1;
                }
                for _ in 0..hz {
                    app.update();
                    assert!(
                        (app.world().get::<Avatar>(entity).unwrap().yaw - released_yaw).abs()
                            < 0.00001
                    );
                }
                app.world_mut()
                    .resource_mut::<Runtime>()
                    .brain
                    .change(State::Fall, 1.);
                app.update();
                assert!(
                    app.world()
                        .get::<Avatar>(entity)
                        .unwrap()
                        .release_yaw
                        .is_none()
                );
                std::fs::remove_dir_all(root).unwrap();
            }
        }
    }

    #[test]
    fn drag_angle_varies_with_horizontal_speed_and_has_signed_limits() {
        for hz in [30, 60, 144] {
            let dt = 1. / hz as f32;
            let mut velocity: f32 = 0.;
            let mut yaw = 0.;
            for speed in [-1800., -900., -450., -90., 0., 90., 450., 900., 1800.] {
                for _ in 0..hz * 2 {
                    velocity += (speed - velocity) * (1. - (-dt * 18.).exp());
                    yaw += (drag_yaw(velocity) - yaw) * (1. - (-dt * 9.).exp());
                    assert!(yaw.abs() <= FRAC_PI_2 + 0.00001);
                }
                let expected = (speed / 900.).clamp(-1., 1.) * FRAC_PI_2;
                assert!((yaw - expected).abs() < 0.0001, "{speed} px/s at {hz} FPS");
            }
        }
        assert_eq!(drag_yaw(-10.), 0.);
        assert_eq!(drag_yaw(10.), 0.);
    }
}
