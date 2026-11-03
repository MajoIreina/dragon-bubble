//! Conservative fallback animation: preserve imported rest poses and let native clips win.
use crate::avatar::Avatar;
use bevy::{
    mesh::{morph::MorphWeights, skinning::SkinnedMesh},
    prelude::*,
};
use pet_core::behavior::State;
use std::collections::HashSet;

#[derive(Component)]
pub struct RestPose {
    owner: Entity,
    pose: Transform,
    axis_x: Vec3,
    axis_y: Vec3,
    role: &'static str,
    side: f32,
    lock_root: bool,
    modified: bool,
}
#[derive(Component)]
pub struct RestExpression {
    owner: Entity,
    weights: Vec<f32>,
    blink: Vec<usize>,
    smile: Vec<usize>,
    modified: bool,
}
pub fn capture(
    commands: &mut Commands,
    owner: Entity,
    descendants: &[Entity],
    spatial: &Query<
        (&Transform, &GlobalTransform, Option<&Name>),
        Without<crate::avatar::SceneContainer>,
    >,
    skins: &Query<&SkinnedMesh>,
    morphs: &Query<&MorphWeights>,
    meshes: &Assets<Mesh>,
) {
    let joints = descendants
        .iter()
        .filter_map(|e| skins.get(*e).ok())
        .flat_map(|s| s.joints.iter().copied())
        .collect::<HashSet<_>>();
    for &e in descendants {
        if let Ok((pose, global, name)) = spatial.get(e) {
            let name = name.map_or("", |n| n.as_str());
            let tokens = pet_core::action::words(name);
            let compact = tokens.join("");
            let lower = name.to_lowercase();
            let mut role = "";
            if joints.contains(&e) && !compact.ends_with("end") && !compact.ends_with("tip") {
                for (part, hints) in [
                    (
                        "calf",
                        &[
                            "lowerleg", "leftleg", "rightleg", "calf", "shin", "小腿", "ひざ",
                        ][..],
                    ),
                    ("leg", &["upperleg", "upleg", "thigh", "大腿", "太もも"]),
                    ("forearm", &["lowerarm", "forearm", "前腕", "ひじ"]),
                    ("arm", &["upperarm", "leftarm", "rightarm", "上腕"]),
                    ("head", &["head", "頭"]),
                    ("spine", &["spine", "chest", "脊柱", "上半身"]),
                    ("tail", &["tail", "尾", "しっぽ"]),
                ] {
                    if hints
                        .iter()
                        .any(|h| lower.contains(h) || compact.contains(h))
                    {
                        role = part;
                        break;
                    }
                }
            }
            let inverse = global.rotation().inverse();
            commands.entity(e).insert(RestPose {
                owner,
                pose: *pose,
                axis_x: inverse * Vec3::X,
                axis_y: inverse * Vec3::Y,
                role,
                side: if compact.contains("left")
                    || compact.contains('左')
                    || tokens.iter().any(|t| t == "l")
                {
                    -1.
                } else {
                    1.
                },
                lock_root: tokens
                    .iter()
                    .any(|t| ["root", "hips", "pelvis"].contains(&t.as_str())),
                modified: false,
            });
        }
        if let Ok(weights) = morphs.get(e) {
            let names = weights
                .first_mesh()
                .and_then(|h| meshes.get(h))
                .and_then(Mesh::morph_target_names)
                .unwrap_or(&[]);
            let mut blink = vec![];
            let mut smile = vec![];
            for (i, name) in names.iter().enumerate() {
                let name = name.to_lowercase().replace(['_', ' ', '-'], "");
                if ["blink", "eyeclose", "eyesclosed", "まばたき", "闭眼"]
                    .iter()
                    .any(|s| name.contains(s))
                {
                    blink.push(i);
                }
                if ["smile", "happy", "笑"].iter().any(|s| name.contains(s)) {
                    smile.push(i);
                }
            }
            commands.entity(e).insert(RestExpression {
                owner,
                weights: weights.weights().to_vec(),
                blink,
                smile,
                modified: false,
            });
        }
    }
}
// Restore before Bevy evaluates clips, so switching from fallback to native cannot retain a pose.
pub fn reset(
    avatars: Query<&Avatar>,
    mut poses: Query<(&mut RestPose, &mut Transform)>,
    mut morphs: Query<(&mut RestExpression, &mut MorphWeights)>,
) {
    for (mut rest, mut pose) in &mut poses {
        if avatars
            .get(rest.owner)
            .is_ok_and(|a| !a.native || rest.modified)
        {
            *pose = rest.pose;
            rest.modified = false;
        }
    }
    for (mut rest, mut weights) in &mut morphs {
        if avatars
            .get(rest.owner)
            .is_ok_and(|a| !a.native || rest.modified)
        {
            weights.weights_mut().copy_from_slice(&rest.weights);
            rest.modified = false;
        }
    }
}
pub fn apply(
    avatars: Query<&Avatar>,
    mut poses: Query<(&mut RestPose, &mut Transform)>,
    mut morphs: Query<(&mut RestExpression, &mut MorphWeights)>,
) {
    for (mut rest, mut pose) in &mut poses {
        let Ok(a) = avatars.get(rest.owner) else {
            continue;
        };
        if !a.ready {
            continue;
        }
        if rest.lock_root {
            pose.translation.x = rest.pose.translation.x;
            pose.translation.z = rest.pose.translation.z;
        }
        if a.native || a.sprite.is_some() {
            continue;
        }
        let t = a.clock;
        let run = a.state == State::Run;
        let stride = (t * if run { 10.75 } else { 5.25 }).sin();
        let mut angle = match rest.role {
            "leg" if a.state.moving() => stride * rest.side * if run { 0.43 } else { 0.25 },
            "calf" if a.state.moving() => {
                (-stride * rest.side).max(0.) * if run { 0.52 } else { 0.25 }
            }
            "leg" if a.state == State::Sleep => 0.08,
            "calf" if a.state == State::Sleep => 0.12,
            "arm" | "forearm" if a.state.moving() => {
                -stride * rest.side * if run { 0.28 } else { 0.14 }
            }
            "arm" | "forearm" if a.state == State::Petted => (t * 10.).sin() * rest.side * 0.14,
            "arm" | "forearm" if matches!(a.state, State::Dragged | State::Airborne) => -0.1,
            "head" => {
                if a.state == State::Petted {
                    (t * 8.).sin() * (-t).exp() * 0.12
                } else if a.state == State::Sleep {
                    0.14
                } else {
                    (t * 1.7).sin() * 0.012
                }
            }
            "spine" => {
                if a.state == State::Sleep {
                    0.07
                } else if run {
                    -0.045 + stride * 0.012
                } else {
                    (t * 2.2).sin() * 0.008
                }
            }
            "tail" => {
                (t * if run {
                    10.75
                } else if a.state == State::Petted {
                    7.
                } else {
                    3.
                })
                .sin()
                    * if run {
                        0.23
                    } else if a.state == State::Walk {
                        0.16
                    } else {
                        0.07
                    }
            }
            _ => 0.,
        };
        if a.state == State::Fall && ["calf", "spine"].contains(&rest.role) {
            angle += (t * 12.).sin() * (-t * 5.).exp() * 0.14;
        }
        let axis = if rest.role == "tail" {
            rest.axis_y
        } else {
            rest.axis_x
        };
        pose.rotation = rest.pose.rotation
            * Quat::from_axis_angle(axis.normalize_or_zero(), angle * (t * 6.).min(1.));
        rest.modified = true;
    }
    for (mut rest, mut weights) in &mut morphs {
        let Ok(a) = avatars.get(rest.owner) else {
            continue;
        };
        if !a.ready || a.native {
            continue;
        }
        let blink = if a.state == State::Sleep {
            1.
        } else {
            (1. - ((a.clock % 4.3 - 3.7) / 0.11).abs()).max(0.)
        };
        let smile = if a.state == State::Petted {
            (a.clock * 5.).min(0.7)
        } else {
            0.
        };
        for &i in &rest.blink {
            if let Some(w) = weights.weights_mut().get_mut(i) {
                *w = blink;
            }
        }
        for &i in &rest.smile {
            if let Some(w) = weights.weights_mut().get_mut(i) {
                *w = smile;
            }
        }
        rest.modified = true;
    }
}
