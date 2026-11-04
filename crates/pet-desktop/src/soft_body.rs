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
use pet_core::soft_body::{Definition, FloorContacts, SoftBody};
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
    floor_hulls: Vec<FloorHull>,
}
struct FloorHull {
    mesh: Entity,
    section: usize,
    min: Vec3,
    max: Vec3,
}
impl Lattice {
    /// Native drag probe compares the actual skinned rear-body pose with its
    /// unbent counterpart; this is only evaluated at diagnostic checkpoints.
    pub(crate) fn drag_bend_samples(&self) -> (Entity, Vec3, Vec3, Vec3) {
        let mut neutral = self.body.clone();
        neutral.inertial_bend = Vec3::ZERO;
        neutral.rebuild(false, 0.);
        let axis = Vec3::from(self.body.def.axis).normalize();
        let difference = |t: f32| {
            let point = axis * (self.body.def.start + self.body.def.length * t);
            self.body.deform_point(point) - neutral.deform_point(point)
        };
        (
            self.owner,
            difference(-0.2),
            difference(0.5),
            difference(1.),
        )
    }
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
            if avatar.skin.id == "builtin.shu" {
                // Include the full tail tip instead of clamping it to one rigid end bone.
                let axis = Vec3::from(body.def.axis).normalize();
                let end = candidates
                    .iter()
                    .flat_map(|(_, _, p, _)| p.iter())
                    .map(|p| p.dot(axis))
                    .fold(body.def.start + body.def.length, f32::max);
                let mut def = body.def.clone();
                let ratio = (end - def.start) / def.length;
                def.length = end - def.start;
                def.sections = ((def.sections as f32 * ratio).ceil() as usize).clamp(4, 48);
                body = SoftBody::sample(def, &points).expect("validated tail envelope");
                body.local_floor = false;
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
            let mut floor_hulls = vec![];
            for (e, mut mesh, points, relative) in candidates {
                let mut bounds = vec![
                    (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
                    body.def.sections
                ];
                let mesh_inverse = relative.inverse();
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
                    let original = mesh_inverse.transform_point3(p);
                    for section in [i, j] {
                        bounds[section].0 = bounds[section].0.min(original);
                        bounds[section].1 = bounds[section].1.max(original);
                    }
                }
                for (section, (min, max)) in bounds.into_iter().enumerate() {
                    if min.is_finite() {
                        floor_hulls.push(FloorHull {
                            mesh: e,
                            section,
                            min,
                            max,
                        });
                    }
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
                    floor_hulls,
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
        let acceleration = if a.drag_motion.active() {
            a.drag_motion.acceleration
        } else {
            ((velocity - lattice.last_velocity) / dt).clamp_length_max(9000.)
        };
        lattice.last_velocity = velocity;
        lattice.last_position = rt.position;
        let landed = a.tail.contact_weight() > 0.;
        let grounded = landed
            || (!a.preview && rt.motion.active && rt.physically_supported())
            || (!matches!(
                a.state,
                pet_core::behavior::State::Dragged | pet_core::behavior::State::Airborne
            ) && (a.preview || (!rt.pressed && !rt.motion.active)));
        lattice.body.support_response = if a.skin.id == "builtin.shu" { 2.2 } else { 1. };
        lattice.body.inertial_bend = if a.skin.id == "builtin.shu" {
            Vec3::new(-a.tail.drag_yaw, a.tail.drag_pitch, 0.)
        } else {
            Vec3::ZERO
        };
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
        let Ok(target) = globals.p0().get(lattice.target).copied() else {
            continue;
        };
        let inverse = target.affine().inverse();
        lattice.body.advance(
            dt,
            Vec3::new(acceleration.x, acceleration.y, 0.),
            local,
            1800.,
            grounded,
            flex,
        );
        if grounded && a.skin.id == "builtin.shu" {
            // This plane stays horizontal while the model rotates about its rear feet.
            let floor = a
                .base
                .transform_point(Vec3::Y * ((160. - a.ground) * 3.9 / 300. + 0.05))
                .y;
            let mut contact = FloorContacts::new(target.affine(), floor);
            for hull in &lattice.floor_hulls {
                if let Ok(g) = globals.p0().get(hull.mesh) {
                    contact.include_hull(
                        &lattice.body,
                        hull.section,
                        inverse * g.affine(),
                        hull.min,
                        hull.max,
                    );
                }
            }
            let lift = contact.apply(&mut lattice.body);
            if rt.smoke && a.state == pet_core::behavior::State::Stretch {
                let mut check = FloorContacts::new(target.affine(), floor);
                for hull in &lattice.floor_hulls {
                    if let Ok(g) = globals.p0().get(hull.mesh) {
                        check.include_hull(
                            &lattice.body,
                            hull.section,
                            inverse * g.affine(),
                            hull.min,
                            hull.max,
                        );
                    }
                }
                assert!(
                    check.apply(&mut lattice.body) < 0.0001,
                    "tail penetrated world floor"
                );
                if (1.2..1.3).contains(&a.clock) {
                    assert!(lift > 0.01, "standing tail should encounter the plane");
                    println!(
                        "STANCE CONTACT: world correction {lift:.4}, sections {}",
                        lattice.body.def.sections
                    );
                }
            }
        }
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
