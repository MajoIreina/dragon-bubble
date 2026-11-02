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
pub fn reset(mut feet: Query<(&mut SuspendedFoot, &mut Transform)>) {
    for (mut foot, mut transform) in &mut feet {
        if foot.modified {
            *transform = foot.rest;
            foot.modified = false;
        }
    }
}
pub fn apply(avatars: Query<&Avatar>, mut feet: Query<(&mut SuspendedFoot, &mut Transform)>) {
    for (mut foot, mut transform) in &mut feet {
        let Ok(a) = avatars.get(foot.owner) else {
            continue;
        };
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
