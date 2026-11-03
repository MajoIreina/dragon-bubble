//! Mesh accents work in both the transparent pet view and the settings preview.
use crate::{avatar::Avatar, runtime::Runtime};
use bevy::{
    asset::RenderAssetUsages, camera::visibility::RenderLayers, mesh::Indices, prelude::*,
    render::render_resource::PrimitiveTopology,
};
use pet_core::behavior::State;

#[derive(Component)]
pub struct Accent {
    owner: Entity,
    anchor: Option<Entity>,
    kind: usize,
    material: Handle<StandardMaterial>,
    sleep: f32,
    idle: f32,
}
#[derive(Component)]
pub struct BubbleEye;
/// One growing Z every seven seconds, with a quiet gap between emissions.
fn sleep_glyph(clock: f32) -> Option<(f32, f32, f32)> {
    let age = (clock - 0.7).rem_euclid(7.);
    if clock < 0.7 || age >= 2.6 {
        return None;
    }
    let phase = age / 2.6;
    let size = 0.055 + phase * 0.28;
    let alpha = (phase / 0.18).min((1. - phase) / 0.25).clamp(0., 1.) * 0.82;
    Some((phase, size, alpha))
}
fn glyph() -> Mesh {
    let mut positions = vec![];
    let mut indices = vec![];
    for (x, y, w, h, angle) in [
        (0., 0.4, 0.9, 0.13, 0.),
        (0., -0.4, 0.9, 0.13, 0.),
        (0., 0., 1.05, 0.13, 0.76),
    ] {
        let offset = positions.len() as u32;
        let q = Quat::from_rotation_z(angle);
        for p in [
            Vec3::new(-w / 2., -h / 2., 0.),
            Vec3::new(w / 2., -h / 2., 0.),
            Vec3::new(w / 2., h / 2., 0.),
            Vec3::new(-w / 2., h / 2., 0.),
        ] {
            positions.push((q * p + Vec3::new(x, y, 0.)).to_array());
        }
        indices.extend([
            offset,
            offset + 1,
            offset + 2,
            offset,
            offset + 2,
            offset + 3,
        ]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 12]);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
pub fn install(
    commands: &mut Commands,
    owner: Entity,
    preview: bool,
    descendants: &[Entity],
    names: &Query<&Name>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let anchor = descendants.iter().copied().find(|e| {
        names.get(*e).is_ok_and(|n| {
            let n = n.as_str().to_lowercase();
            n == "head" || n.ends_with(":head") || n.contains("snout")
        })
    });
    let z = meshes.add(glyph());
    let glint = meshes.add(Rectangle::new(1., 0.12));
    for kind in [0, 4, 5, 6, 7] {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.51, 0.72, 0.76, 0.),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Accent {
                owner,
                anchor,
                kind,
                material: material.clone(),
                sleep: 0.,
                idle: 0.,
            },
            Mesh3d(if kind < 3 { z.clone() } else { glint.clone() }),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(if preview { 2 } else { 1 }),
            ChildOf(owner),
        ));
    }
}
#[allow(clippy::type_complexity)] // Filters prove that eye and accent transforms are disjoint.
pub fn update(
    time: Res<Time>,
    rt: Res<Runtime>,
    avatars: Query<(&Avatar, &GlobalTransform)>,
    globals: Query<&GlobalTransform>,
    mut accents: Query<(&mut Accent, &mut Transform, &mut Visibility), Without<Avatar>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut eyes: Query<
        (&ChildOf, &mut Transform),
        (With<BubbleEye>, Without<Accent>, Without<Avatar>),
    >,
) {
    let dt = time.delta_secs().min(0.1);
    for (mut effect, mut transform, mut visible) in &mut accents {
        let Ok((avatar, global)) = avatars.get(effect.owner) else {
            continue;
        };
        if avatar.preview && !(rt.show_settings && (rt.page == 0 || rt.page == 3)) {
            *visible = Visibility::Hidden;
            continue;
        }
        effect.sleep = (effect.sleep
            + if avatar.state == State::Sleep {
                dt * 2.
            } else {
                -dt * 2.
            })
        .clamp(0., 1.);
        effect.idle = (effect.idle
            + if avatar.state == State::Idle {
                dt * 3.
            } else {
                -dt * 3.
            })
        .clamp(0., 1.);
        let anchor = effect
            .anchor
            .and_then(|e| globals.get(e).ok())
            .map(|g| g.translation())
            .unwrap_or_else(|| global.transform_point(Vec3::new(0.35, 0.45, 0.)));
        let (offset, size, alpha, rotation) = if effect.kind < 3 {
            let (phase, size, alpha) = sleep_glyph(avatar.clock).unwrap_or((0., 0.055, 0.));
            (
                Vec2::new(
                    0.23 + phase * 0.32 + (phase * std::f32::consts::TAU).sin() * 0.05,
                    0.16 + phase * 0.69,
                ),
                size,
                alpha * effect.sleep,
                (phase * std::f32::consts::TAU).sin() * 0.12,
            )
        } else {
            let i = (effect.kind - 4) / 2;
            let vertical = effect.kind % 2 == 1;
            let pulse = (1. - ((avatar.clock % 6. - 4.6) / 0.6).abs()).max(0.) * effect.idle;
            (
                Vec2::new(-0.34 + i as f32 * 0.68, 0.16 + i as f32 * 0.13),
                0.1 * pulse,
                pulse * 0.7,
                if vertical {
                    std::f32::consts::FRAC_PI_2
                } else {
                    0.
                },
            )
        };
        let position = Vec3::new(
            (anchor.x + offset.x).clamp(-1.7, 1.7),
            (anchor.y + offset.y).clamp(-1.1, 1.65),
            2.5,
        );
        *transform = Transform::from_matrix(
            global.to_matrix().inverse()
                * Mat4::from_scale_rotation_translation(
                    Vec3::splat(size.max(0.001)),
                    Quat::from_rotation_z(rotation),
                    position,
                ),
        );
        *visible = if alpha > 0.001 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(material) = materials.get_mut(&effect.material) {
            material.base_color = if effect.kind < 3 {
                Color::srgba(0., 0., 0., alpha)
            } else {
                Color::srgba(0.51, 0.72, 0.76, alpha)
            };
        }
    }
    for (parent, mut transform) in &mut eyes {
        if let Ok((a, _)) = avatars.get(parent.parent()) {
            let blink = crate::expressions::eye_closure(
                a.state,
                a.expression_clock,
                a.clock,
                transform.translation.x.signum(),
            );
            transform.scale.y = 1. - blink * 0.9;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_z_grows_then_leaves_a_long_quiet_gap() {
        assert!(sleep_glyph(0.5).is_none());
        let small = sleep_glyph(0.9).unwrap();
        let large = sleep_glyph(2.8).unwrap();
        assert!(small.1 < large.1 && small.2 > 0. && large.2 > 0.);
        for time in [3.4, 4., 5., 6., 7.6] {
            assert!(sleep_glyph(time).is_none());
        }
        let next = sleep_glyph(7.9).unwrap();
        assert!((small.1 - next.1).abs() < 0.00001);
    }
}
