//! Exercise the real animation/pose/skinning pipeline with synthetic pointer motion.
use crate::{
    avatar::{Avatar, RenderViews, ViewCamera},
    runtime::Runtime,
};
use bevy::prelude::*;
use pet_core::behavior::State;

#[derive(Default)]
pub struct Probe {
    start: Option<f64>,
    stage: usize,
}
#[allow(clippy::too_many_arguments)]
pub fn run(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    mut probe: Local<Probe>,
    avatars: Query<&Avatar>,
    lattices: Query<&crate::soft_body::Lattice>,
    mut cameras: Query<&mut Transform, With<ViewCamera>>,
    views: Res<RenderViews>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    const STAGES: [(&str, Vec2); 8] = [
        ("neutral", Vec2::ZERO),
        ("up-slow", Vec2::new(0., -160.)),
        ("up-fast", Vec2::new(0., -900.)),
        ("down-fast", Vec2::new(0., 900.)),
        ("right-fast", Vec2::new(900., 0.)),
        ("left-fast", Vec2::new(-900., 0.)),
        ("stopped", Vec2::ZERO),
        ("released", Vec2::ZERO),
    ];
    assert!(
        time.elapsed_secs() < 45.,
        "Drag probe timed out: {}",
        rt.status
    );
    rt.show_settings = false;
    let Some(a) = rt
        .active
        .and_then(|entity| avatars.get(entity).ok())
        .filter(|a| a.ready)
    else {
        return;
    };
    let start = *probe.start.get_or_insert(time.elapsed_secs_f64());
    let stage = ((time.elapsed_secs_f64() - start) / 1.4) as usize;
    if stage != probe.stage {
        let previous = probe.stage;
        match previous {
            1 => assert!(a.tail.drag_pitch < -0.04 && a.tail.drag_pitch > -0.07),
            2 => assert!(
                a.tail.drag_pitch < -0.24,
                "upward motion must pull tail down"
            ),
            3 => assert!(a.tail.drag_pitch > 0.24, "downward motion must lift tail"),
            6 | 7 => assert!(a.tail.drag_pitch.abs() < 0.005 && a.tail.drag_yaw.abs() < 0.005),
            _ => (),
        }
        println!(
            "DRAG stage {}: pitch {:.4}, yaw {:.4}, speed {:?}",
            STAGES[previous].0, a.tail.drag_pitch, a.tail.drag_yaw, a.drag_motion.velocity
        );
        if (1..=3).contains(&previous) {
            let (owner, front, rear, tip) = lattices
                .iter()
                .map(|l| l.drag_bend_samples())
                .find(|sample| Some(sample.0) == rt.active)
                .expect("skinned rear body");
            let _ = owner;
            let sign = if previous == 3 { 1. } else { -1. };
            assert!(front.length() < 0.0001, "front body must stay anchored");
            assert!(
                rear.y * sign > 0.01,
                "rear BODY section must bend with the tail"
            );
            assert!(
                tip.y * sign > rear.y * sign * 2.,
                "bend must continue through the tail tip"
            );
            println!("DRAG REAR: front {front:?}, rear {rear:?}, tip {tip:?}");
        }
        if stage >= STAGES.len() {
            println!(
                "DRAG PASS: directional slow/fast/reversed motion, rear-body skinning, stop and release"
            );
            exit.write(AppExit::Success);
            return;
        }
        let folder = rt.data.join("screenshots");
        std::fs::create_dir_all(&folder).expect("drag screenshot folder");
        let path = folder.join(format!("drag-{}.png", STAGES[previous].0));
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                views.output.clone(),
            ))
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .expect("drag pixels")
                        .save(&path)
                        .expect("drag screenshot");
                },
            );
        probe.stage = stage;
    }
    for mut camera in &mut cameras {
        *camera = Transform::from_xyz(6., 0.05, 0.).looking_at(Vec3::ZERO, Vec3::Y);
    }
    let held = stage < 7;
    rt.pressed = held;
    rt.dragging = held;
    rt.motion.active = false;
    rt.position = rt.desktop.place_on(0);
    rt.brain
        .change(if held { State::Dragged } else { State::Idle }, 30.);
    let zoom = rt.prefs.zoom;
    rt.drag_cursor += STAGES[stage].1 * time.delta_secs() * zoom;
}
