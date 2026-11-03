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
