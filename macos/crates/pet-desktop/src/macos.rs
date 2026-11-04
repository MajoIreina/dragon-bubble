//! AppKit integration, using logical points throughout desktop simulation.
//! All callers run on Bevy/winit's main thread; no event taps or input injection.
use bevy::prelude::Vec2;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSEvent, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior};
use objc2_foundation::{NSPoint, NSUserDefaults, ns_string};
use pet_core::desktop::{Desktop, Rect};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("AppKit must run on the main thread")
}

fn primary_top() -> f64 {
    NSScreen::screens(main_thread())
        .firstObject()
        .map(|screen| {
            let frame = screen.frame();
            frame.origin.y + frame.size.height
        })
        .unwrap_or(0.)
}

pub fn desktop() -> Desktop {
    let screens = NSScreen::screens(main_thread());
    let top = primary_top();
    let areas: Vec<_> = screens
        .iter()
        .map(|screen| {
            // visibleFrame excludes the Dock and menu bar. Flip around the
            // primary screen, preserving negative coordinates on other displays.
            let frame = screen.visibleFrame();
            Rect::new(
                frame.origin.x as f32,
                (top - frame.origin.y - frame.size.height) as f32,
                frame.size.width as f32,
                frame.size.height as f32,
            )
        })
        .collect();
    if areas.is_empty() {
        Desktop::default()
    } else {
        Desktop {
            areas,
            ..Default::default()
        }
    }
}

pub fn pointer() -> (Vec2, bool, bool) {
    let point = NSEvent::mouseLocation();
    let buttons = NSEvent::pressedMouseButtons();
    (
        Vec2::new(point.x as f32, (primary_top() - point.y) as f32),
        buttons & 1 != 0,
        buttons & 2 != 0,
    )
}

fn native_window(window: &winit::window::Window) -> Option<Retained<NSWindow>> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    let _mtm = main_thread();
    // SAFETY: winit owns this NSView for the live Window and we are on its main thread.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    view.window()
}

pub fn over_pet(window: &winit::window::Window) -> bool {
    let Some(window) = native_window(window) else {
        return false;
    };
    NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
        NSEvent::mouseLocation(),
        0,
        main_thread(),
    ) == window.windowNumber()
}

pub fn sync_pet(window: &winit::window::Window, position: Vec2) {
    if let Some(window) = native_window(window) {
        window.setHasShadow(false);
        window.setHidesOnDeactivate(false);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        window.setFrameTopLeftPoint(NSPoint::new(
            position.x as f64,
            primary_top() - position.y as f64,
        ));
    }
}

pub fn position_window(window: &winit::window::Window, position: Vec2) {
    if let Some(window) = native_window(window) {
        window.setFrameTopLeftPoint(NSPoint::new(
            position.x as f64,
            primary_top() - position.y as f64,
        ));
    }
}

pub fn set_hit_test(window: &winit::window::Window, enabled: bool) {
    if let Some(window) = native_window(window) {
        window.setIgnoresMouseEvents(!enabled);
    }
}

pub fn prefers_dark_theme() -> bool {
    NSUserDefaults::standardUserDefaults()
        .stringForKey(ns_string!("AppleInterfaceStyle"))
        .is_some_and(|style| style.to_string() == "Dark")
}

pub fn fullscreen_at(point: Vec2) -> bool {
    let top = primary_top();
    let display = NSScreen::screens(main_thread()).iter().find_map(|screen| {
        let f = screen.frame();
        let rect = Rect::new(
            f.origin.x as f32,
            (top - f.origin.y - f.size.height) as f32,
            f.size.width as f32,
            f.size.height as f32,
        );
        rect.contains(point).then_some(rect)
    });
    let Some(display) = display else {
        return false;
    };
    visible_windows()
        .into_iter()
        .find(|(_, _, normal)| *normal)
        .is_some_and(|(_, bounds, _)| crate::desktop_mode::covers_display(bounds, display))
}

/// Quartz reports window bounds in global logical points, top-left origin.
/// Read geometry only; no screen capture, window titles, or accessibility hooks.
fn desktop_window_layer(layer: i64) -> bool {
    // Dock (20), Notification Center (23), and menu-bar (24) windows can have
    // full-screen bounds even while visually transparent. They must not hide
    // normal window tops. Keep ordinary and floating application windows.
    (0..20).contains(&layer)
}

pub fn visible_windows() -> Vec<(isize, Rect, bool)> {
    use core_foundation::{
        base::{CFType, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    };
    use core_graphics::{
        geometry::CGRect,
        window::{
            copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements,
            kCGWindowListOptionOnScreenOnly,
        },
    };
    fn value(dict: &CFDictionary, key: &str) -> Option<CFType> {
        let key = CFString::new(key);
        let pointer = dict.find(key.as_CFTypeRef())?;
        // SAFETY: values belong to the live retained Core Foundation dictionary.
        Some(unsafe { CFType::wrap_under_get_rule(*pointer) })
    }
    fn number(dict: &CFDictionary, key: &str) -> Option<i64> {
        value(dict, key)?.downcast::<CFNumber>()?.to_i64()
    }
    let Some(windows) = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    ) else {
        return vec![];
    };
    windows
        .iter()
        .filter_map(|pointer| {
            // SAFETY: CGWindowListCopyWindowInfo returns an array of CFDictionary.
            let entry =
                unsafe { CFType::wrap_under_get_rule(*pointer) }.downcast::<CFDictionary>()?;
            if number(&entry, "kCGWindowOwnerPID")? == std::process::id() as i64
                || value(&entry, "kCGWindowAlpha")?
                    .downcast::<CFNumber>()?
                    .to_f64()?
                    <= 0.
            {
                return None;
            }
            let bounds = value(&entry, "kCGWindowBounds")?.downcast::<CFDictionary>()?;
            let rect = CGRect::from_dict_representation(&bounds)?;
            if rect.size.width < 64. || rect.size.height < 48. {
                return None;
            }
            let layer = number(&entry, "kCGWindowLayer")?;
            if !desktop_window_layer(layer) {
                return None;
            }
            let id = number(&entry, "kCGWindowNumber")? as isize;
            Some((
                id,
                Rect::new(
                    rect.origin.x as f32,
                    rect.origin.y as f32,
                    rect.size.width as f32,
                    rect.size.height as f32,
                ),
                layer == 0,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_overlay_bounds_do_not_occlude_perching_surfaces() {
        for layer in [0, 3, 8] {
            assert!(desktop_window_layer(layer));
        }
        for layer in [-2147483623, -1, 20, 23, 24, 101] {
            assert!(!desktop_window_layer(layer));
        }
    }
}
