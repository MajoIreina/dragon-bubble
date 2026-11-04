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

#[cfg(windows)]
fn fullscreen_at(point: Vec2) -> bool {
    use windows_sys::Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        UI::WindowsAndMessaging::*,
    };
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null()
            || window == GetShellWindow()
            || IsIconic(window) != 0
            || IsWindowVisible(window) == 0
        {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        if pid == std::process::id() {
            return false;
        }
        let monitor = MonitorFromPoint(
            POINT {
                x: point.x as i32,
                y: point.y as i32,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        if monitor != MonitorFromWindow(window, MONITOR_DEFAULTTONULL) {
            return false;
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let mut bounds: RECT = std::mem::zeroed();
        if GetMonitorInfoW(monitor, &mut info) == 0 || GetWindowRect(window, &mut bounds) == 0 {
            return false;
        }
        // Use visible bounds, excluding the invisible resize border.
        DwmGetWindowAttribute(
            window,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut bounds as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        );
        let rect = |r: RECT| {
            Rect::new(
                r.left as f32,
                r.top as f32,
                (r.right - r.left) as f32,
                (r.bottom - r.top) as f32,
            )
        };
        covers_display(rect(bounds), rect(info.rcMonitor))
    }
}
#[cfg(target_os = "macos")]
fn fullscreen_at(point: Vec2) -> bool {
    crate::macos::fullscreen_at(point)
}
#[cfg(not(any(windows, target_os = "macos")))]
fn fullscreen_at(_: Vec2) -> bool {
    false
}

pub fn detect(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    mut elapsed: Local<f32>,
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
    let hidden = fullscreen_at(rt.position + rt.desktop.extent * 0.5);
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

#[cfg(test)]
mod tests {
    use super::*;
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
