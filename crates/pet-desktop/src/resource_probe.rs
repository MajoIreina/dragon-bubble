//! Native resource soak test. Uses smoke data and never changes user settings.
use crate::{
    avatar::Avatar,
    runtime::{Action, Runtime},
};
use bevy::prelude::*;

#[derive(Default)]
pub struct Probe {
    started: Option<f64>,
    step: Option<usize>,
    baseline: Option<[usize; 5]>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    mut state: Local<Probe>,
    mut menu: ResMut<crate::popup::MenuState>,
    avatars: Query<&Avatar>,
    images: Res<Assets<Image>>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    toon: Res<Assets<crate::toon::ToonMaterial>>,
    graphs: Res<Assets<AnimationGraph>>,
    entities: Query<Entity>,
    outputs: Query<&Camera, With<crate::avatar::OutputCamera>>,
    readbacks: Query<(), With<bevy::render::gpu_readback::Readback>>,
    mut exit: MessageWriter<AppExit>,
) {
    if time.elapsed_secs() > 240. {
        panic!("Resource probe timed out");
    }
    if !rt
        .active
        .and_then(|e| avatars.get(e).ok())
        .is_some_and(|a| a.ready)
    {
        return;
    }
    let start = *state.started.get_or_insert(time.elapsed_secs_f64());
    let elapsed = time.elapsed_secs_f64() - start;
    let step = (elapsed / 0.75) as usize;
    if state.step == Some(step) {
        return;
    }
    state.step = Some(step);
    let cycle = step / 24;
    let phase = step % 24;
    if phase < 12 {
        if phase == 10 {
            rt.obscured = true;
            menu.visible = false;
        }
        if phase == 11 {
            assert!(
                outputs.iter().all(|camera| !camera.is_active),
                "Full-screen must pause output cameras"
            );
            assert!(readbacks.is_empty(), "Full-screen must stop GPU readbacks");
            rt.obscured = false;
        }
        rt.prefs.theme = if phase.is_multiple_of(2) {
            "light"
        } else {
            "dark"
        }
        .into();
        rt.show_settings = phase < 10;
        rt.page = [0, 5, 4, 3, 2, 1][phase % 6];
        rt.prefs.performance = pet_core::preferences::RenderOptions::preset(
            ["power", "balanced", "high", "ultra", "extreme", "balanced"][phase % 6],
        );
        if phase.is_multiple_of(2) && phase != 10 {
            menu.open(rt.desktop.areas[0].min + Vec2::splat(100.));
        }
        if phase == 5 {
            let id = rt.prefs.skin_id.clone();
            rt.actions.push(Action::SelectSkin(id));
        }
        if phase == 8 {
            rt.actions.push(Action::Scale(if cycle.is_multiple_of(2) {
                1.25
            } else {
                0.75
            }));
        }
    } else {
        rt.show_settings = false;
        rt.obscured = false;
        menu.visible = false;
        rt.prefs.performance = pet_core::preferences::RenderOptions::default();
        if phase == 12 {
            rt.actions.push(Action::Scale(1.));
        }
    }
    if phase == 23 {
        let counts = [
            images.len(),
            meshes.len(),
            materials.len(),
            toon.len(),
            graphs.len(),
        ];
        println!(
            "RESOURCE SAMPLE: {}",
            serde_json::json!({"cycle":cycle,"images":counts[0],"meshes":counts[1],"materials":counts[2],"toon":counts[3],"graphs":counts[4],"entities":entities.iter().count()})
        );
        if cycle == 1 {
            state.baseline = Some(counts);
        }
        if cycle == 7 {
            let base = state.baseline.expect("warmup sample");
            for (current, baseline) in counts.into_iter().zip(base) {
                assert!(
                    current <= baseline + 4,
                    "Resources accumulate after UI/quality/skin cycles: {base:?} -> {counts:?}"
                );
            }
            assert!(rt.preview.is_none(), "Hidden preview must be released");
            println!(
                "RESOURCE PASS: 96 theme/page/menu/quality transitions, 8 avatar reloads and resizes; resource counts bounded"
            );
            exit.write(AppExit::Success);
        }
    }
}
