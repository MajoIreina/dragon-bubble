//! Rear-foot stance shared by the body, planted soles and lifted tail.
use bevy::prelude::*;

pub fn weight(clock: f32) -> f32 {
    let smooth = |v: f32| {
        let v = v.clamp(0., 1.);
        v * v * (3. - 2. * v)
    };
    smooth(clock / 1.2) * smooth((5.5 - clock) / 1.2)
}

pub fn rise(clock: f32) -> Quat {
    Quat::from_rotation_x(-0.52 * weight(clock))
}

pub fn raise_body(transform: &mut Transform, rear_ankles: Vec3, clock: f32) {
    let before = transform.rotation;
    let after = before * rise(clock);
    let pivot = rear_ankles * transform.scale;
    transform.translation += before * pivot - after * pivot;
    transform.rotation = after;
}

pub fn tail(clock: f32) -> Quat {
    // Counter the body's pitch first, then lift the tail clear of the supporting feet.
    rise(clock).inverse() * Quat::from_rotation_x(0.65 * weight(clock))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rear_soles_stay_planted_through_rise_hold_and_lower_in_both_directions() {
        let pivot = Vec3::new(0., -0.835, -0.69);
        for yaw in [-1.57, -0.65, 0., 0.65, 1.57] {
            for i in 0..=110 {
                let clock = i as f32 * 0.05;
                let mut body = Transform::from_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(0.63));
                let before = body;
                raise_body(&mut body, pivot, clock);
                for side in [-1., 1.] {
                    let ankle = pivot + Vec3::X * side * 0.47;
                    // Check sole centre and heel/toe: the complete foot stays horizontal.
                    for sole in [Vec3::new(0., -0.19, 0.), Vec3::new(0., -0.15, 0.2)] {
                        let planted = ankle + rise(clock).inverse() * sole;
                        assert!(
                            body.transform_point(planted)
                                .abs_diff_eq(before.transform_point(ankle + sole), 0.00001)
                        );
                    }
                }
                let front = Vec3::new(0.54, -0.835, 0.28);
                assert!(body.transform_point(front).y >= before.transform_point(front).y - 0.00001);
            }
        }
    }
}
