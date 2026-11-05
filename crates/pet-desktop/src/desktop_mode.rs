//! Yield the pet's monitor to full-screen applications without stealing focus.
use crate::{
    avatar::{OutputCamera, RenderViews},
    runtime::Runtime,
};
use bevy::prelude::*;
use pet_core::desktop::Rect;

pub(crate) fn covers_display(window: Rect, display: Rect) -> bool {
    let tolerance = Vec2::splat(2.);
    window.min.cmple(display.min + tolerance).all()
        && window.max.cmpge(display.max - tolerance).all()
}

/// Desktop hosts can cover a monitor without being a full-screen application.
/// Keep this filter shared with the window-edge/perching enumeration.
#[cfg(any(windows, test))]
pub(crate) fn is_desktop_host_class(name: &[u16]) -> bool {
    [
        "Progman",
        "WorkerW",
        "SHELLDLL_DefView",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
    ]
    .iter()
    .any(|candidate| name.iter().copied().eq(candidate.encode_utf16()))
}

#[cfg(windows)]
#[path = "desktop_mode_windows.rs"]
mod windows;

#[derive(Default)]
pub struct FullscreenDetector {
    #[cfg(windows)]
    native: windows::Detector,
}

#[cfg(windows)]
fn fullscreen_at(point: Vec2, detector: &mut FullscreenDetector) -> bool {
    detector.native.poll(point)
}
#[cfg(target_os = "macos")]
fn fullscreen_at(point: Vec2, _: &mut FullscreenDetector) -> bool {
    crate::macos::fullscreen_at(point)
}
#[cfg(not(any(windows, target_os = "macos")))]
fn fullscreen_at(_: Vec2, _: &mut FullscreenDetector) -> bool {
    false
}

pub fn detect(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    mut elapsed: Local<f32>,
    mut detector: Local<FullscreenDetector>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    *elapsed += time.delta_secs();
    if *elapsed < 0.25 {
        return;
    }
    *elapsed = 0.;
    if rt.smoke {
        return;
    }
    let hidden = if rt.prefs.hide_for_any_fullscreen {
        #[cfg(windows)]
        {
            detector.native.any_fullscreen()
        }
        #[cfg(target_os = "macos")]
        {
            crate::macos::any_fullscreen()
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            false
        }
    } else {
        fullscreen_at(rt.position + rt.desktop.extent * 0.5, &mut detector)
    };
    if hidden != rt.obscured {
        rt.obscured = hidden;
        rt.pressed = false;
        rt.dragging = false;
        let position = rt.position;
        rt.motion.stop(position);
        info!("FULLSCREEN: {}", if hidden { "paused" } else { "restored" });
    }
}

/// Stop offscreen rendering and readbacks too, rather than merely hiding a window.
pub fn sync(
    mut commands: Commands,
    rt: Res<Runtime>,
    views: Res<RenderViews>,
    mut cameras: Query<&mut Camera, With<OutputCamera>>,
    readbacks: Query<(), With<bevy::render::gpu_readback::Readback>>,
) {
    for mut camera in &mut cameras {
        camera.is_active = !rt.obscured;
    }
    if let Some(entity) = views.readback {
        if rt.obscured && readbacks.contains(entity) {
            commands
                .entity(entity)
                .remove::<bevy::render::gpu_readback::Readback>();
        } else if !rt.obscured && !readbacks.contains(entity) {
            commands
                .entity(entity)
                .insert(bevy::render::gpu_readback::Readback::texture(
                    views.output.clone(),
                ));
        }
    }
}

#[cfg(windows)]
pub fn native_status() {
    let mut detector = FullscreenDetector::default();
    println!("FULLSCREEN STATUS: {}", detector.native.any_fullscreen());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_hosts_are_shared_between_fullscreen_and_perching_filters() {
        for class in [
            "Progman",
            "WorkerW",
            "SHELLDLL_DefView",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
        ] {
            assert!(is_desktop_host_class(
                &class.encode_utf16().collect::<Vec<_>>()
            ));
        }
        for class in [
            "",
            "Chrome_WidgetWin_1",
            "SDL_app",
            "WorkerWindow",
            "Notepad",
        ] {
            assert!(!is_desktop_host_class(
                &class.encode_utf16().collect::<Vec<_>>()
            ));
        }
    }
    #[test]
    fn fullscreen_uses_full_monitor_and_supports_negative_monitor_origins() {
        let display = Rect::new(-1920., 0., 1920., 1080.);
        assert!(covers_display(display, display));
        assert!(covers_display(
            Rect::new(-1921., -1., 1922., 1082.),
            display
        ));
        assert!(!covers_display(
            Rect::new(-1920., 0., 1920., 1040.),
            display
        ));
        assert!(!covers_display(Rect::new(0., 0., 1920., 1080.), display));
        assert!(!covers_display(
            Rect::new(-1800., 20., 1000., 800.),
            display
        ));
    }
}
