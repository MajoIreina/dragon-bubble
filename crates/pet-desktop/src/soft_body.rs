//! Cross-section GPU skinning; native transform nodes remain animation targets.
use crate::{avatar::Avatar, runtime::Runtime};
use bevy::{
    camera::visibility::NoFrustumCulling,
    mesh::{
        VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    prelude::*,
};
use pet_core::soft_body::{Definition, SoftBody};
#[derive(Component)]
pub struct InstallSoftBody;
#[derive(Component)]
pub struct Lattice {
    owner: Entity,
    target: Entity,
    body: SoftBody,
    joints: Vec<Entity>,
    bindings: Vec<(Entity, Handle<SkinnedMeshInverseBindposes>)>,
    last_velocity: Vec2,
    last_position: Vec2,
}
fn find(
    root: Entity,
    path: &str,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    if path.contains('/') {
        let mut node = root;
        for part in path.split('/') {
            node = children
                .get(node)
                .ok()?
                .iter()
                .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == part))?;
        }
        Some(node)
    } else {
        std::iter::once(root)
            .chain(children.iter_descendants(root))
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == path))
    }
}
#[allow(clippy::too_many_arguments)] // Independent Bevy queries preserve component borrow checks.
pub fn install(
    mut commands: Commands,
    avatars: Query<(Entity, &Avatar, Ref<InstallSoftBody>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    global: Query<&GlobalTransform>,
    mesh_entities: Query<(&Mesh3d, Option<&SkinnedMesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut binds: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    rt: Res<Runtime>,
) {
    for (root, avatar, install) in &avatars {
        // Normalization changes local transforms in finish; wait for propagation.
        if install.is_added() {
            continue;
        }
        commands.entity(root).remove::<InstallSoftBody>();
        let defs = avatar
            .skin
            .options
            .get("physics")
            .and_then(|v| v.get("soft_bodies"))
            .cloned()
            .and_then(|v| serde_json::from_value::<Vec<Definition>>(v).ok())
            .unwrap_or_default();
        for def in defs {
            let Some(target) = find(root, &def.node, &children, &names) else {
                continue;
            };
            let Some(reference) = find(target, &def.profile_mesh, &children, &names) else {
                continue;
            };
            let anchors = def
                .anchors
                .iter()
                .map(|a| find(target, a, &children, &names))
                .collect::<Option<Vec<_>>>();
            let Some(anchors) = anchors else { continue };
            let Ok(target_global) = global.get(target) else {
                continue;
            };
            let inverse = target_global.affine().inverse();
            let reference_mesh = std::iter::once(reference)
                .chain(children.iter_descendants(reference))
                .find_map(|e| mesh_entities.get(e).ok().map(|(m, _)| (e, m)));
            let Some((reference, reference_mesh)) = reference_mesh else {
                continue;
            };
            let Some(VertexAttributeValues::Float32x3(points)) = meshes
                .get(&reference_mesh.0)
                .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
            else {
                continue;
            };
            let Ok(reference_global) = global.get(reference) else {
                continue;
            };
            let relative = inverse * reference_global.affine();
            let points = points
                .iter()
                .map(|p| relative.transform_point3(Vec3::from(*p)))
                .collect::<Vec<_>>();
            let Ok(mut body) = SoftBody::sample(def, &points) else {
                continue;
            };
            let mut candidates = vec![];
            let mut valid = true;
            for e in std::iter::once(target).chain(children.iter_descendants(target)) {
                if anchors
                    .iter()
                    .any(|a| *a == e || children.iter_descendants(*a).any(|c| c == e))
                {
                    continue;
                }
                let Ok((mesh, skin)) = mesh_entities.get(e) else {
                    continue;
                };
                let Some(mesh) = meshes.get(&mesh.0) else {
                    continue;
                };
                let Some(VertexAttributeValues::Float32x3(points)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    continue;
                };
                let Ok(g) = global.get(e) else { continue };
                let relative = inverse * g.affine();
                let points = points
                    .iter()
                    .map(|p| relative.transform_point3(Vec3::from(*p)))
                    .collect::<Vec<_>>();
                if !points
                    .iter()
                    .any(|p| p.dot(Vec3::from(body.def.axis)) > body.def.start)
                {
                    continue;
                }
                if skin.is_some()
                    || mesh.has_morph_targets()
                    || mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some()
                {
                    valid = false;
                    break;
                }
                candidates.push((e, mesh.clone(), points, relative));
            }
            if !valid || candidates.is_empty() {
                continue;
            }
            let joints = (0..body.def.sections)
                .map(|_| {
                    let e = commands
                        .spawn((Transform::default(), Visibility::Inherited))
                        .id();
                    commands.entity(root).add_child(e);
                    e
                })
                .collect::<Vec<_>>();
            let mut bindings = vec![];
            for (e, mut mesh, points, relative) in candidates {
                let mut indices = Vec::with_capacity(points.len());
                let mut weights = Vec::with_capacity(points.len());
                for p in points {
                    let coordinate = body.coordinate(p);
                    let i = coordinate.floor() as usize;
                    let j = (i + 1).min(body.def.sections - 1);
                    let blend = coordinate - i as f32;
                    indices.push([i as u16, j as u16, 0, 0]);
                    weights.push([1. - blend, blend, 0., 0.]);
                    body.include_vertex(p);
                }
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_JOINT_INDEX,
                    VertexAttributeValues::Uint16x4(indices),
                );
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
                let handle = binds.add(SkinnedMeshInverseBindposes::from(vec![
                    Mat4::from(relative);
                    body.def.sections
                ]));
                commands.entity(e).insert((
                    Mesh3d(meshes.add(mesh)),
                    SkinnedMesh {
                        inverse_bindposes: handle.clone(),
                        joints: joints.clone(),
                    },
                    NoFrustumCulling,
                ));
                bindings.push((e, handle));
            }
            info!(
                "Installed soft body for {} with {} sections",
                avatar.skin.id, body.def.sections
            );
            let lattice = commands
                .spawn(Lattice {
                    owner: root,
                    target,
                    body,
                    joints,
                    bindings,
                    last_velocity: Vec2::ZERO,
                    last_position: rt.position,
                })
                .id();
            commands.entity(root).add_child(lattice);
        }
    }
}
pub fn update(
    time: Res<Time>,
    rt: Res<Runtime>,
    avatars: Query<&Avatar>,
    mut lattices: Query<&mut Lattice>,
    mut globals: ParamSet<(Query<&GlobalTransform>, Query<&mut GlobalTransform>)>,
    mut binds: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let dt = time.delta_secs().min(0.1);
    if dt <= 0. {
        return;
    }
    for mut lattice in &mut lattices {
        let Ok(a) = avatars.get(lattice.owner) else {
            continue;
        };
        if a.preview && !rt.show_settings {
            continue;
        }
        let velocity = if a.preview {
            Vec2::ZERO
        } else {
            (rt.position - lattice.last_position) / (dt * rt.prefs.zoom)
        };
        let acceleration = ((velocity - lattice.last_velocity) / dt).clamp_length_max(9000.);
        lattice.last_velocity = velocity;
        lattice.last_position = rt.position;
        let grounded = !matches!(
            a.state,
            pet_core::behavior::State::Dragged | pet_core::behavior::State::Airborne
        ) && (a.preview || (!rt.pressed && !rt.motion.active));
        lattice.body.suspension = if a.state == pet_core::behavior::State::Dragged {
            a.suspension.weight
        } else {
            0.
        };
        let local = Quat::from_rotation_y(-a.yaw)
            * Vec3::new(acceleration.x, -acceleration.y, 0.)
            * (3.9 / 300.);
        let flex = if a.state == pet_core::behavior::State::Run && !a.turning {
            (a.clock * 10.75).cos()
        } else {
            0.
        };
        lattice.body.advance(
            dt,
            Vec3::new(acceleration.x, acceleration.y, 0.),
            local,
            1800.,
            grounded,
            flex,
        );
        let Ok(target) = globals.p0().get(lattice.target).copied() else {
            continue;
        };
        let inverse = target.affine().inverse();
        for (mesh, handle) in &lattice.bindings {
            if let Ok(g) = globals.p0().get(*mesh) {
                let matrix = Mat4::from(inverse * g.affine());
                if let Some(old) = binds.get(handle)
                    && old.first().is_some_and(|m| m.abs_diff_eq(matrix, 0.000001))
                {
                    continue;
                }
                let _ = binds.insert(
                    handle.id(),
                    SkinnedMeshInverseBindposes::from(vec![matrix; lattice.body.def.sections]),
                );
            }
        }
        for (i, e) in lattice.joints.iter().enumerate() {
            if let Ok(mut g) = globals.p1().get_mut(*e) {
                *g =
                    GlobalTransform::from(Mat4::from(target.affine() * lattice.body.transforms[i]));
            }
        }
    }
}
