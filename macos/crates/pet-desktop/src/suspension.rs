//! Procedural suspended feet layered after the model's native body animation.
use crate::avatar::{Avatar, SceneContainer};
use bevy::prelude::*;

#[derive(Component)]
pub struct SuspendedFoot {
    pub owner: Entity,
    rest: Transform,
    rear: bool,
    side: f32,
    modified: bool,
}
#[derive(Component)]
pub struct ResponsiveTail {
    owner: Entity,
    rest: Transform,
    modified: bool,
}
#[derive(Component)]
pub struct StanceBody {
    owner: Entity,
    rest: Transform,
}
pub fn capture(
    commands: &mut Commands,
    owner: Entity,
    descendants: &[Entity],
    spatial: &Query<(&Transform, &GlobalTransform, Option<&Name>), Without<SceneContainer>>,
) {
    for &e in descendants {
        let Ok((rest, _, Some(name))) = spatial.get(e) else {
            continue;
        };
        let name = name.as_str();
        if name == "PlushBody" {
            commands.entity(e).insert(StanceBody { owner, rest: *rest });
        }
        if name == "TailWag" {
            commands.entity(e).insert(ResponsiveTail {
                owner,
                rest: *rest,
                modified: false,
            });
        }
        if !["FrontFoot.L", "FrontFoot.R", "RearFoot.L", "RearFoot.R"].contains(&name) {
            continue;
        }
        commands.entity(e).insert(SuspendedFoot {
            owner,
            rest: *rest,
            rear: name.starts_with("Rear"),
            side: if name.ends_with(".L") { -1. } else { 1. },
            modified: false,
        });
    }
}
pub fn reset(
    mut feet: Query<(&mut SuspendedFoot, &mut Transform), Without<ResponsiveTail>>,
    mut tails: Query<(&mut ResponsiveTail, &mut Transform), Without<SuspendedFoot>>,
) {
    for (mut foot, mut transform) in &mut feet {
        if foot.modified {
            *transform = foot.rest;
            foot.modified = false;
        }
    }
    for (mut tail, mut transform) in &mut tails {
        if tail.modified {
            *transform = tail.rest;
            tail.modified = false;
        }
    }
}
#[allow(clippy::type_complexity)] // Disjoint Bevy transform queries.
pub fn apply(
    avatars: Query<&Avatar>,
    mut feet: Query<(&mut SuspendedFoot, &mut Transform), Without<ResponsiveTail>>,
    mut tails: Query<(&mut ResponsiveTail, &mut Transform), Without<SuspendedFoot>>,
    mut bodies: Query<
        (&StanceBody, &mut Transform),
        (Without<SuspendedFoot>, Without<ResponsiveTail>),
    >,
) {
    for (body, mut transform) in &mut bodies {
        if let Ok(a) = avatars.get(body.owner)
            && a.ready
            && a.skin.id == "builtin.shu"
            && a.state == pet_core::behavior::State::Stretch
        {
            // A stopped native clip retains its last pose. Remove that pose before
            // applying the rear-foot hinge, otherwise its residual tilt moves the soles.
            *transform = body.rest;
        }
    }
    for (mut tail, mut transform) in &mut tails {
        if let Ok(a) = avatars.get(tail.owner)
            && a.ready
        {
            // Layer pitch over native yaw; never accumulate frame-to-frame rotations.
            // Shu's rear body AND fin share the same curved skinning envelope.
            // Keep this node's native wag/contact pitch without doubling the bend.
            transform.rotation *= if a.skin.id == "builtin.shu" {
                Quat::from_rotation_x(a.tail.pitch)
            } else {
                Quat::from_rotation_y(a.tail.drag_yaw)
                    * Quat::from_rotation_x(a.tail.pitch + a.tail.drag_pitch)
            };
            tail.modified = true;
        }
    }
    for (mut foot, mut transform) in &mut feet {
        let Ok(a) = avatars.get(foot.owner) else {
            continue;
        };
        if a.ready && a.state == pet_core::behavior::State::Stretch {
            // Cat/dog rear-leg support: rise, place front paws, alternate kneading, lower.
            let w = crate::rear_stance::weight(a.clock);
            let knead = (a.clock * 3.5 + foot.side * 1.2).sin() * 0.025 * w;
            if foot.rear {
                // Keep both ankle centres and the entire sole fixed while the body rises.
                transform.rotation =
                    crate::rear_stance::rise(a.clock).inverse() * foot.rest.rotation;
            } else {
                transform.rotation *= Quat::from_rotation_x(-1.05 * w);
                transform.translation += Vec3::new(0., 0.35 * w + knead, 0.18 * w);
            }
            foot.modified = true;
            continue;
        }
        if !a.ready || a.suspension.weight < 0.001 {
            continue;
        }
        let (pitch, drop, reach) =
            a.suspension
                .foot(a.clock, foot.rear, foot.side, a.direction as f32);
        let mut target = foot.rest;
        target.translation += Vec3::new(foot.side * 0.018 * a.suspension.weight, drop, reach);
        target.rotation *= Quat::from_rotation_x(pitch);
        // Fade over native animation on release, restoring walk/land poses cleanly.
        let blend = a.suspension.weight;
        transform.translation = transform.translation.lerp(target.translation, blend);
        transform.rotation = transform.rotation.slerp(target.rotation, blend);
        foot.modified = true;
    }
}
