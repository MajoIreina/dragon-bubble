//! Native desktop integration. Rendering and pet simulation do not depend on Win32.
use crate::runtime::{Action, Runtime};
use bevy::{
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowCloseRequested, WindowLevel, WindowPosition},
};
use pet_core::{
    behavior::State,
    desktop::{Desktop, Rect},
};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
#[derive(Component)]
pub struct SettingsWindow;
#[derive(Component)]
pub struct SettingsCamera;
/// Match the Windows app theme without changing any system preferences.
pub fn prefers_dark_theme() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::*;
        let path: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();
        let mut value: u32 = 1;
        let mut size = std::mem::size_of_val(&value) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                path.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut value as *mut _ as *mut _,
                &mut size,
            )
        };
        status == 0 && value == 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Let Winit pace app updates without blocking the event-loop thread. The
/// layered-window mouse hook is serviced on that same thread, so sleeping in
/// a frame system adds avoidable input latency.
pub fn frame_update_mode(
    fps: u32,
    vsync: bool,
    refresh_hz: Option<u32>,
) -> bevy::winit::UpdateMode {
    let limit = if vsync {
        let refresh = refresh_hz.unwrap_or(if fps == 0 { 60 } else { fps }).max(1);
        if fps == 0 { refresh } else { fps.min(refresh) }
    } else {
        fps
    };
    if limit == 0 {
        bevy::winit::UpdateMode::Continuous
    } else {
        bevy::winit::UpdateMode::reactive_low_power(std::time::Duration::from_secs_f64(
            1. / limit.clamp(1, 360) as f64,
        ))
    }
}
#[derive(Default)]
pub(crate) struct FramePacingCache {
    refresh_hz: Option<u32>,
    config: Option<(u32, bool, Option<u32>)>,
    refresh_poll: f32,
}
pub fn frame_pacing(
    time: Res<Time>,
    rt: Res<Runtime>,
    mut settings: ResMut<bevy::winit::WinitSettings>,
    mut cache: Local<FramePacingCache>,
) {
    cache.refresh_poll += time.delta_secs();
    if cache.refresh_poll >= 0.5 || cache.config.is_none() {
        cache.refresh_hz = display_refresh_rate(&rt.position);
        cache.refresh_poll = 0.;
    }
    let key = (
        if rt.obscured && !rt.show_settings {
            5
        } else {
            rt.prefs.performance.fps
        },
        rt.prefs.performance.vsync,
        cache.refresh_hz,
    );
    if cache.config != Some(key) {
        let mode = frame_update_mode(key.0, key.1, key.2);
        settings.focused_mode = mode;
        settings.unfocused_mode = mode;
        cache.config = Some(key);
    }
}

#[cfg(windows)]
fn display_refresh_rate(position: &Vec2) -> Option<u32> {
    use windows_sys::Win32::{Foundation::POINT, Graphics::Gdi::*};
    unsafe {
        let monitor = MonitorFromPoint(
            POINT {
                x: position.x.round() as i32,
                y: position.y.round() as i32,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        if monitor.is_null() {
            return None;
        }
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, (&mut info as *mut MONITORINFOEXW).cast()) == 0 {
            return None;
        }
        let mut mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        if EnumDisplaySettingsW(info.szDevice.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode) == 0
            || mode.dmDisplayFrequency <= 1
        {
            None
        } else {
            Some(mode.dmDisplayFrequency)
        }
    }
}
#[cfg(not(windows))]
fn display_refresh_rate(_: &Vec2) -> Option<u32> {
    None
}
pub struct Tray {
    _icon: TrayIcon,
}

#[cfg(windows)]
pub fn desktop() -> Desktop {
    use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*};
    unsafe extern "system" fn visit(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
        let list = unsafe { &mut *(data as *mut Vec<(bool, Rect)>) };
        let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if unsafe { GetMonitorInfoW(m, &mut info) } != 0 {
            let r = info.rcWork;
            list.push((
                info.dwFlags & 1 != 0,
                Rect::new(
                    r.left as f32,
                    r.top as f32,
                    (r.right - r.left) as f32,
                    (r.bottom - r.top) as f32,
                ),
            ));
        }
        1
    }
    let mut areas: Vec<(bool, Rect)> = vec![];
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(visit),
            &mut areas as *mut _ as isize,
        );
    }
    areas.sort_by_key(|(primary, _)| !primary);
    if areas.is_empty() {
        return Desktop::default();
    }
    Desktop {
        areas: areas.into_iter().map(|(_, r)| r).collect(),
        ..Default::default()
    }
}
#[cfg(not(windows))]
pub fn desktop() -> Desktop {
    Desktop::default()
}
/// Enumerate in Z order and remove covered pieces of each title-bar edge.
/// All coordinates are physical pixels, matching GetCursorPos and rcWork.
#[cfg(windows)]
pub fn window_surfaces() -> Vec<pet_core::desktop::Surface> {
    use windows_sys::Win32::{Foundation::*, Graphics::Dwm::*, UI::WindowsAndMessaging::*};
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        let windows = unsafe { &mut *(data as *mut Vec<(isize, Rect, bool)>) };
        unsafe {
            if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return 1;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == std::process::id() {
                return 1;
            }
            let mut cloaked: u32 = 0;
            DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, &mut cloaked as *mut _ as _, 4);
            if cloaked != 0 {
                return 1;
            }
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if ex & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT) != 0 {
                return 1;
            }
            let mut class = [0u16; 128];
            let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
            if crate::desktop_mode::is_desktop_host_class(&class[..len.max(0) as usize]) {
                return 1;
            }
            let mut r: RECT = std::mem::zeroed();
            if DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                &mut r as *mut _ as _,
                std::mem::size_of::<RECT>() as u32,
            ) < 0
                && GetWindowRect(hwnd, &mut r) == 0
            {
                return 1;
            }
            if r.right <= r.left || r.bottom <= r.top {
                return 1;
            }
            windows.push((
                hwnd as isize,
                Rect::new(
                    r.left as f32,
                    r.top as f32,
                    (r.right - r.left) as f32,
                    (r.bottom - r.top) as f32,
                ),
                style & WS_CAPTION != 0,
            ));
        }
        1
    }
    let mut windows: Vec<(isize, Rect, bool)> = vec![];
    unsafe {
        EnumWindows(Some(visit), &mut windows as *mut _ as isize);
    }
    exposed_surfaces(&windows)
}
#[cfg(not(windows))]
pub fn window_surfaces() -> Vec<pet_core::desktop::Surface> {
    vec![]
}

fn exposed_surfaces(windows: &[(isize, Rect, bool)]) -> Vec<pet_core::desktop::Surface> {
    let mut result = vec![];
    for (i, &(id, rect, perchable)) in windows.iter().enumerate() {
        if !perchable {
            continue;
        }
        let mut spans = vec![(rect.min.x, rect.max.x)];
        for &(_, cover, _) in &windows[..i] {
            if cover.min.y <= rect.min.y && cover.max.y > rect.min.y {
                spans = spans
                    .into_iter()
                    .flat_map(|(l, r)| {
                        if cover.max.x <= l || cover.min.x >= r {
                            vec![(l, r)]
                        } else {
                            vec![(l, cover.min.x.max(l)), (cover.max.x.min(r), r)]
                        }
                    })
                    .filter(|(l, r)| r - l >= 32.)
                    .collect();
            }
        }
        result.extend(
            spans
                .into_iter()
                .filter(|(l, r)| r - l >= 32.)
                .map(|(left, right)| pet_core::desktop::Surface {
                    id,
                    window_left: rect.min.x,
                    left,
                    right,
                    y: rect.min.y,
                }),
        );
    }
    result
}
#[cfg(windows)]
fn pointer() -> (Vec2, bool, bool) {
    use windows_sys::Win32::{
        Foundation::POINT,
        UI::{Input::KeyboardAndMouse::GetAsyncKeyState, WindowsAndMessaging::GetCursorPos},
    };
    let mut p = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut p);
        (
            Vec2::new(p.x as f32, p.y as f32),
            GetAsyncKeyState(1) < 0,
            GetAsyncKeyState(2) < 0,
        )
    }
}
#[cfg(not(windows))]
fn pointer() -> (Vec2, bool, bool) {
    (Vec2::ZERO, false, false)
}
pub fn setup_tray(world: &mut World) {
    let bytes = include_bytes!("../../../assets/tray.ico");
    let result = (|| -> anyhow::Result<TrayIcon> {
        let img = image::load_from_memory(bytes)?.to_rgba8();
        let (w, h) = img.dimensions();
        let icon = tray_icon::Icon::from_rgba(img.into_raw(), w, h)?;
        Ok(TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("龙泡泡 · My Dragon Bubble")
            .with_menu_on_left_click(false)
            .build()?)
    })();
    match result {
        Ok(icon) => world.insert_non_send_resource(Tray { _icon: icon }),
        Err(e) => eprintln!("Tray initialization failed: {e}"),
    }
}
#[cfg(windows)]
pub fn wheel(mut rt: ResMut<Runtime>) {
    for (delta, cursor) in crate::layered::take_wheel() {
        rt.scroll_zoom(delta, cursor);
    }
}
#[cfg(not(windows))]
pub fn wheel() {}

pub fn input(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    mut pet: Query<(Entity, &Window, &mut bevy::window::CursorOptions), With<PrimaryWindow>>,

    mut popup: ResMut<crate::popup::MenuState>,
    #[cfg(windows)] native: NonSend<crate::layered::NativePet>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    // Automated probes supply their own pointer state; physical input must not
    // release a synthetic drag or change its trajectory.
    if rt.smoke {
        return;
    }
    if rt.obscured {
        let (_, left, right) = pointer();
        rt.last_left = left;
        rt.last_right = right;
        return;
    }
    bevy::winit::WINIT_WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let Ok((entity, win, mut cursor_options)) = pet.single_mut() else {
            return;
        };
        let (cursor, left, right) = pointer();
        let local = (cursor - rt.position) / rt.prefs.zoom;
        let inside =
            local.x >= 38. && local.x <= 282. && local.y >= 35. && local.y <= rt.ground + 10.;
        cursor_options.hit_test = inside || rt.pressed;
        // Only the pet's native window may initiate a drag; controls overlapping it keep their input.
        let over_pet = windows.get_window(entity).is_some_and(|w| {
            #[cfg(windows)]
            {
                use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
                if let Ok(handle) = w.window_handle()
                    && let RawWindowHandle::Win32(_) = handle.as_raw()
                {
                    let p = windows_sys::Win32::Foundation::POINT {
                        x: cursor.x as i32,
                        y: cursor.y as i32,
                    };
                    return unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::WindowFromPoint(p)
                    } == native.hwnd;
                }
                false
            }
            #[cfg(not(windows))]
            {
                w.has_focus()
            }
        });
        if rt.game.active() {
            if left && !rt.last_left && inside && over_pet {
                crate::game::hit(&mut rt, std::time::Instant::now());
            }
            if right && !rt.last_right {
                crate::game::cancel(&mut rt, "本轮已取消，不计入历史。");
            }
            rt.last_left = left;
            rt.last_right = right;
            return;
        }
        if left && !rt.last_left && inside && over_pet {
            rt.brain.interact();
            rt.pressed = true;
            rt.dragging = false;
            rt.press_cursor = cursor;
            rt.drag_cursor = cursor;
            rt.drag_facing_x = cursor.x;
            rt.press_origin = rt.position;
            rt.grab_point = local;
            let p = rt.position;
            rt.motion.begin_drag(p, time.elapsed_secs_f64());
        }
        if rt.pressed && left {
            rt.drag_cursor = cursor;
            if cursor.distance(rt.press_cursor) > 4. {
                rt.dragging = true;
            }
            if rt.dragging {
                // The drag stays latched even after stopping or returning to the press point.
                if rt.brain.state != State::Dragged {
                    rt.brain.change(State::Dragged, 1.);
                }
                let mut anchor = rt.drag_facing_x;
                rt.brain.direction = drag_direction(cursor.x, &mut anchor, rt.brain.direction);
                rt.drag_facing_x = anchor;
                rt.position = rt
                    .desktop
                    .clamp_origin(rt.press_origin + cursor - rt.press_cursor);
                let p = rt.position;
                rt.motion.record_drag(p, time.elapsed_secs_f64());
            }
        }
        if !left && rt.pressed {
            if rt.dragging {
                let v = rt.motion.release_velocity(time.elapsed_secs_f64());
                let p = rt.position;
                rt.motion.launch(p, v);
                rt.brain.change(State::Airborne, 2.);
            } else {
                rt.pet_reaction = rt.pet_reaction.wrapping_add(1);
                rt.brain.change(State::Petted, 1.8);
            }
            rt.pressed = false;
            rt.dragging = false;
            rt.dirty = true;
        }
        if right && !rt.last_right && inside && over_pet {
            rt.brain.interact();
            popup.open(cursor);
        }
        rt.last_left = left;
        rt.last_right = right;
        let _ = win;
    });
}
// Accumulate small horizontal movements, while ignoring pointer jitter and vertical motion.
fn drag_direction(cursor_x: f32, anchor: &mut f32, previous: i32) -> i32 {
    let delta = cursor_x - *anchor;
    if delta.abs() < 3. {
        return previous;
    }
    *anchor = cursor_x;
    if delta > 0. { 1 } else { -1 }
}

pub fn events(
    mut rt: ResMut<Runtime>,
    mut popup: ResMut<crate::popup::MenuState>,
    mut close: MessageReader<WindowCloseRequested>,
    settings: Query<Entity, With<SettingsWindow>>,
    menus: Query<Entity, With<crate::popup::MenuWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    #[cfg(windows)]
    if crate::layered::close_requested() {
        rt.actions.push(Action::Quit);
    }
    for e in close.read() {
        if settings.contains(e.window) {
            rt.show_settings = false;
        } else if menus.contains(e.window) {
            popup.visible = false;
        } else {
            rt.actions.push(Action::Quit);
        }
    }
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        match event {
            TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                ..
            } => {
                popup.open(pointer().0);
            }
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                rt.actions.push(Action::ShowSettings(0));
            }
            _ => {}
        }
    }
    if rt.quit {
        exit.write(AppExit::Success);
    }
}
pub fn sync_windows(
    rt: Res<Runtime>,
    mut windows: Query<(
        Entity,
        &mut Window,
        Option<&PrimaryWindow>,
        Option<&SettingsWindow>,
    )>,
    mut cameras: Query<&mut Camera, With<SettingsCamera>>,
    mut branded: Local<Option<Entity>>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    for (entity, mut w, pet, settings) in &mut windows {
        if pet.is_some() {
            #[cfg(not(windows))]
            {
                w.visible = !rt.obscured;
            }
            let size = (320. * rt.prefs.zoom).round() as u32;
            if w.resolution.physical_width() != size {
                w.resolution.set_physical_resolution(size, size);
            }
            let position = WindowPosition::At(IVec2::new(
                rt.position.x.round() as i32,
                rt.position.y.round() as i32,
            ));
            if w.position != position {
                w.position = position;
            }
            let level = if rt.prefs.always_on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            if w.window_level != level {
                w.window_level = level;
            }
            let present_mode = if rt.prefs.performance.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            };
            if w.present_mode != present_mode {
                w.present_mode = present_mode;
            }
        } else if settings.is_some() {
            if *branded != Some(entity) {
                bevy::winit::WINIT_WINDOWS.with(|windows| {
                    if let Some(window) = windows.borrow().get_window(entity)
                        && let Ok(logo) =
                            image::load_from_memory(include_bytes!("../../../assets/tray.ico"))
                    {
                        let logo = logo.to_rgba8();
                        if let Ok(icon) = winit::window::Icon::from_rgba(
                            logo.as_raw().clone(),
                            logo.width(),
                            logo.height(),
                        ) {
                            window.set_window_icon(Some(icon.clone()));
                            #[cfg(windows)]
                            {
                                use winit::platform::windows::WindowExtWindows;
                                window.set_taskbar_icon(Some(icon));
                            }
                            *branded = Some(entity);
                        }
                    }
                });
            }
            let visible = rt.show_settings;
            if w.visible != visible {
                w.visible = visible;
            }
        }
    }
    for mut c in &mut cameras {
        c.is_active = rt.show_settings;
    }
}
#[cfg(test)]
mod tests {
    use super::drag_direction;
    #[test]
    fn frame_pacing_uses_event_loop_and_keeps_unlimited_mode() {
        use bevy::winit::UpdateMode;
        assert_eq!(
            super::frame_update_mode(0, false, None),
            UpdateMode::Continuous
        );
        assert!(
            matches!(super::frame_update_mode(60, false, None), UpdateMode::Reactive { wait, react_to_device_events: false, .. } if wait.as_millis() == 16)
        );
        assert!(
            matches!(super::frame_update_mode(240, true, Some(144)), UpdateMode::Reactive { wait, .. } if wait.as_micros() == 6944)
        );
        assert!(
            matches!(super::frame_update_mode(0, true, Some(120)), UpdateMode::Reactive { wait, .. } if wait.as_micros() == 8333)
        );
    }
    #[test]
    fn foreground_windows_occlude_only_covered_title_edge() {
        use pet_core::desktop::Rect;
        let edges = super::exposed_surfaces(&[
            (1, Rect::new(300., 100., 200., 500.), true),
            (2, Rect::new(100., 300., 600., 400.), true),
        ]);
        let back: Vec<_> = edges
            .iter()
            .filter(|s| s.id == 2)
            .map(|s| (s.left, s.right))
            .collect();
        assert_eq!(back, vec![(100., 300.), (500., 700.)]);
        let hidden = super::exposed_surfaces(&[
            (1, Rect::new(0., 0., 1000., 800.), false),
            (2, Rect::new(100., 300., 600., 400.), true),
        ]);
        assert!(hidden.is_empty());
    }

    #[test]
    fn drag_facing_tracks_reversals_and_keeps_direction_while_stationary() {
        let mut anchor = 100.;
        let mut direction = -1;
        for (cursor_x, expected) in [
            (101., -1),
            (102., -1),
            (103., 1),
            (120., 1),
            (120., 1),
            (119., 1),
            (117., -1),
            (90., -1),
            (90., -1),
            (93., 1),
        ] {
            direction = drag_direction(cursor_x, &mut anchor, direction);
            assert_eq!(direction, expected);
        }
    }
}
