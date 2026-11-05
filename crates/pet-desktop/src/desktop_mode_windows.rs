//! Foreground-window sampling. Normal polling uses stack buffers and keeps no history.
use super::{covers_display, is_desktop_host_class};
use bevy::prelude::*;
use pet_core::desktop::Rect;
use std::{io::Write, path::PathBuf};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    System::Threading::*,
    UI::{Shell::SHQueryUserNotificationState, WindowsAndMessaging::*},
};

#[derive(Clone, Debug, PartialEq)]
struct Sample {
    window: usize,
    root: usize,
    pid: u32,
    class: [u16; 256],
    root_class: [u16; 256],
    desktop_host: bool,
    own_window: bool,
    visible: bool,
    minimized: bool,
    cloaked: bool,
    passive: bool,
    monitor: usize,
    window_monitor: usize,
    bounds: Option<[i32; 4]>,
    display: Option<[i32; 4]>,
    notification_state: Option<i32>,
}

fn class_name(window: HWND) -> [u16; 256] {
    let mut name = [0; 256];
    if !window.is_null() {
        unsafe { GetClassNameW(window, name.as_mut_ptr(), name.len() as i32) };
    }
    name
}

fn unpadded(name: &[u16]) -> &[u16] {
    &name[..name.iter().position(|&c| c == 0).unwrap_or(name.len())]
}

fn rect_array(r: RECT) -> [i32; 4] {
    [r.left, r.top, r.right, r.bottom]
}

fn rect(r: [i32; 4]) -> Rect {
    Rect::new(
        r[0] as f32,
        r[1] as f32,
        (r[2] - r[0]) as f32,
        (r[3] - r[1]) as f32,
    )
}

impl Sample {
    fn capture(window: HWND, point: Vec2, own_pid: u32) -> Self {
        unsafe {
            // GA_ROOT follows the parent chain: Wallpaper Engine render children
            // under WorkerW must be excluded even when their own class is custom.
            let root = GetAncestor(window, GA_ROOT);
            let class = class_name(window);
            let root_class = class_name(root);
            let desktop_host = (!window.is_null()
                && (window == GetShellWindow() || window == GetDesktopWindow()))
                || is_desktop_host_class(unpadded(&class))
                || is_desktop_host_class(unpadded(&root_class));
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            let monitor = MonitorFromPoint(
                POINT {
                    x: point.x as i32,
                    y: point.y as i32,
                },
                MONITOR_DEFAULTTONEAREST,
            );
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            let display =
                (GetMonitorInfoW(monitor, &mut info) != 0).then(|| rect_array(info.rcMonitor));
            let mut bounds: RECT = std::mem::zeroed();
            // DWM bounds exclude the invisible resize border. Fall back when
            // unavailable, and never turn an unsuccessful query into full-screen.
            let has_bounds = DwmGetWindowAttribute(
                window,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                (&mut bounds as *mut RECT).cast(),
                std::mem::size_of::<RECT>() as u32,
            ) >= 0
                || GetWindowRect(window, &mut bounds) != 0;
            let bounds = (has_bounds && bounds.right > bounds.left && bounds.bottom > bounds.top)
                .then(|| rect_array(bounds));
            let mut cloaked = 0u32;
            DwmGetWindowAttribute(
                window,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
            let ex_style = GetWindowLongW(window, GWL_EXSTYLE) as u32;
            Self {
                window: window as usize,
                root: root as usize,
                pid,
                class,
                root_class,
                desktop_host,
                own_window: pid == own_pid,
                visible: IsWindowVisible(window) != 0,
                minimized: IsIconic(window) != 0,
                cloaked: cloaked != 0,
                passive: ex_style & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT) != 0,
                monitor: monitor as usize,
                window_monitor: MonitorFromWindow(window, MONITOR_DEFAULTTONULL) as usize,
                bounds,
                display,
                notification_state: None,
            }
        }
    }

    fn reason(&self) -> &'static str {
        if self.window == 0 {
            return "no_foreground";
        }
        if self.desktop_host {
            return "desktop_host";
        }
        if self.own_window {
            return "own_window";
        }
        if !self.visible {
            return "invisible";
        }
        if self.minimized {
            return "minimized";
        }
        if self.cloaked {
            return "cloaked";
        }
        if self.passive {
            return "passive_window";
        }
        if self.monitor == 0 || self.window_monitor != self.monitor {
            return "other_monitor";
        }
        match (self.bounds, self.display) {
            (Some(window), Some(display)) if covers_display(rect(window), rect(display)) => {
                "fullscreen"
            }
            (Some(_), Some(_)) => "windowed",
            _ => "unavailable_bounds",
        }
    }

    fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "unix_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_millis() as u64,
            "window": format!("0x{:x}", self.window),
            "root": format!("0x{:x}", self.root),
            "pid": self.pid,
            "process": process_name(self.pid),
            "class": String::from_utf16_lossy(unpadded(&self.class)),
            "root_class": String::from_utf16_lossy(unpadded(&self.root_class)),
            "bounds": self.bounds,
            "monitor": format!("0x{:x}", self.monitor),
            "window_monitor": format!("0x{:x}", self.window_monitor),
            "monitor_bounds": self.display,
            "reason": self.reason(),
            "paused": self.reason() == "fullscreen",
            "notification_state": self.notification_state,
        })
    }
}

fn process_name(pid: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut path = [0u16; 32768];
        let mut len = path.len() as u32;
        let success = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut len) != 0;
        // Process handles are never retained, including on query failure.
        CloseHandle(process);
        success.then(|| {
            let path = String::from_utf16_lossy(&path[..len as usize]);
            path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned()
        })
    }
}

pub(super) struct Detector {
    diagnostic_path: Option<PathBuf>,
    previous: Option<Sample>,
    write_failed: bool,
}

impl Default for Detector {
    fn default() -> Self {
        Self {
            diagnostic_path: std::env::args()
                .any(|arg| arg == "--fullscreen-diagnostics")
                .then(|| pet_core::preferences::user_dir().join("fullscreen-diagnostics.jsonl")),
            previous: None,
            write_failed: false,
        }
    }
}

impl Detector {
    pub(super) fn any_fullscreen(&mut self) -> bool {
        unsafe extern "system" fn visit(window: HWND, context: LPARAM) -> i32 {
            let found = unsafe { &mut *(context as *mut bool) };
            let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONULL) };
            let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
                let point = Vec2::new(
                    info.rcMonitor.left as f32 + 1.,
                    info.rcMonitor.top as f32 + 1.,
                );
                if Sample::capture(window, point, std::process::id()).reason() == "fullscreen" {
                    *found = true;
                    return 0;
                }
            }
            1
        }
        let mut found = false;
        unsafe { EnumWindows(Some(visit), (&mut found as *mut bool) as LPARAM) };
        found
    }

    pub(super) fn poll(&mut self, point: Vec2) -> bool {
        let mut sample =
            Sample::capture(unsafe { GetForegroundWindow() }, point, std::process::id());
        if let Some(path) = &self.diagnostic_path {
            // Global shell state is diagnostic only; it cannot identify which
            // monitor should be yielded and may include presentation settings.
            let mut state = 0;
            if unsafe { SHQueryUserNotificationState(&mut state) } >= 0 {
                sample.notification_state = Some(state);
            }
            if self.previous.as_ref() != Some(&sample) {
                let record = sample.diagnostic();
                if let Err(error) = append_diagnostic(path, &record)
                    && !self.write_failed
                {
                    warn!("Full-screen diagnostic write failed: {error}");
                    self.write_failed = true;
                }
                info!("FULLSCREEN diagnostic: {record}");
            }
        }
        let hidden = sample.reason() == "fullscreen";
        if self.diagnostic_path.is_some() {
            self.previous = Some(sample);
        }
        hidden
    }
}

fn append_diagnostic(path: &std::path::Path, record: &serde_json::Value) -> std::io::Result<()> {
    // Bound disk use as well as memory; retain only the current and previous MiB.
    if path.metadata().is_ok_and(|meta| meta.len() >= 1024 * 1024) {
        let previous = path.with_extension("previous.jsonl");
        if previous.exists() {
            std::fs::remove_file(&previous)?;
        }
        std::fs::rename(path, previous)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{record}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fullscreen() -> Sample {
        Sample {
            window: 1,
            root: 1,
            pid: 99,
            class: [0; 256],
            root_class: [0; 256],
            desktop_host: false,
            own_window: false,
            visible: true,
            minimized: false,
            cloaked: false,
            passive: false,
            monitor: 1,
            window_monitor: 1,
            bounds: Some([-1920, 0, 0, 1080]),
            display: Some([-1920, 0, 0, 1080]),
            notification_state: None,
        }
    }

    #[test]
    fn fullscreen_requires_a_visible_active_app_on_the_pets_display() {
        let sample = fullscreen();
        assert_eq!(sample.reason(), "fullscreen");
        let variants = [
            (
                Sample {
                    desktop_host: true,
                    ..sample.clone()
                },
                "desktop_host",
            ),
            (
                Sample {
                    own_window: true,
                    ..sample.clone()
                },
                "own_window",
            ),
            (
                Sample {
                    visible: false,
                    ..sample.clone()
                },
                "invisible",
            ),
            (
                Sample {
                    minimized: true,
                    ..sample.clone()
                },
                "minimized",
            ),
            (
                Sample {
                    cloaked: true,
                    ..sample.clone()
                },
                "cloaked",
            ),
            (
                Sample {
                    passive: true,
                    ..sample.clone()
                },
                "passive_window",
            ),
            (
                Sample {
                    window_monitor: 2,
                    ..sample.clone()
                },
                "other_monitor",
            ),
            (
                Sample {
                    bounds: None,
                    ..sample.clone()
                },
                "unavailable_bounds",
            ),
            (
                Sample {
                    bounds: Some([-1920, 0, 0, 1040]),
                    ..sample.clone()
                },
                "windowed",
            ),
        ];
        for (sample, reason) in variants {
            assert_eq!(sample.reason(), reason);
        }
    }

    #[test]
    fn global_notification_state_does_not_hide_a_desktop_host_or_another_display() {
        // All shell states, including API failure, are independent of our per-monitor decision.
        for state in [
            None,
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
        ] {
            assert_eq!(
                Sample {
                    notification_state: state,
                    ..fullscreen()
                }
                .reason(),
                "fullscreen"
            );
            assert_eq!(
                Sample {
                    desktop_host: true,
                    notification_state: state,
                    ..fullscreen()
                }
                .reason(),
                "desktop_host"
            );
            assert_eq!(
                Sample {
                    window_monitor: 2,
                    notification_state: state,
                    ..fullscreen()
                }
                .reason(),
                "other_monitor"
            );
        }
    }

    #[test]
    fn native_workerw_and_its_custom_renderer_child_are_desktop_hosts() {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let name: Vec<u16> = "WorkerW\0".encode_utf16().collect();
            let class = WNDCLASSW {
                lpfnWndProc: Some(DefWindowProcW),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..std::mem::zeroed()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            assert_ne!(GetMonitorInfoW(monitor, &mut info), 0);
            let r = info.rcMonitor;
            let root = CreateWindowExW(
                0,
                name.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            assert!(!root.is_null());
            let child_class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let child = CreateWindowExW(
                0,
                child_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD,
                0,
                0,
                r.right - r.left,
                r.bottom - r.top,
                root,
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            assert!(!child.is_null());
            ShowWindow(root, SW_SHOWNOACTIVATE);
            ShowWindow(child, SW_SHOWNOACTIVATE);
            let root_sample = Sample::capture(root, Vec2::ZERO, u32::MAX);
            let child_sample = Sample::capture(child, Vec2::ZERO, u32::MAX);
            DestroyWindow(root);
            UnregisterClassW(name.as_ptr(), instance);
            assert_eq!(root_sample.reason(), "desktop_host");
            assert_eq!(child_sample.reason(), "desktop_host");
            assert_eq!(child_sample.root, root as usize);
            // Both would be incorrectly treated as full-screen by geometry alone.
            assert_eq!(
                Sample {
                    desktop_host: false,
                    ..root_sample
                }
                .reason(),
                "fullscreen"
            );
            assert_eq!(
                Sample {
                    desktop_host: false,
                    ..child_sample.clone()
                }
                .reason(),
                "fullscreen"
            );
            assert_eq!(
                String::from_utf16_lossy(unpadded(&child_sample.class)),
                "Static"
            );
        }
    }

    #[test]
    fn native_popup_distinguishes_fullscreen_windowed_hidden_and_passive() {
        unsafe {
            let point = Vec2::ZERO;
            let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            assert_ne!(GetMonitorInfoW(monitor, &mut info), 0);
            let r = info.rcMonitor;
            let name: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let window = CreateWindowExW(
                0,
                name.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!window.is_null());
            // Show without activating: exercise real Win32 geometry and visibility
            // without changing the user's foreground application.
            ShowWindow(window, SW_SHOWNOACTIVATE);
            let full = Sample::capture(window, point, u32::MAX);
            let own = Sample::capture(window, point, std::process::id());
            SetWindowLongW(window, GWL_EXSTYLE, WS_EX_NOACTIVATE as i32);
            let passive = Sample::capture(window, point, u32::MAX);
            SetWindowLongW(window, GWL_EXSTYLE, 0);
            SetWindowPos(
                window,
                std::ptr::null_mut(),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top - 80,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
            // Invalidate DWM's cached frame bounds before sampling the resize.
            DwmFlush();
            let windowed = Sample::capture(window, point, u32::MAX);
            ShowWindow(window, SW_HIDE);
            let hidden = Sample::capture(window, point, u32::MAX);
            DestroyWindow(window);
            assert_eq!(full.reason(), "fullscreen", "{full:?}");
            assert_eq!(own.reason(), "own_window");
            assert_eq!(passive.reason(), "passive_window");
            assert_eq!(windowed.reason(), "windowed", "{windowed:?}");
            assert_eq!(hidden.reason(), "invisible");
        }
    }

    #[test]
    fn diagnostics_rotate_and_do_not_keep_process_or_file_handles_open() {
        let directory = std::env::temp_dir().join(format!(
            "dragon-fullscreen-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("fullscreen-diagnostics.jsonl");
        std::fs::write(&path, vec![b' '; 1024 * 1024]).unwrap();
        for _ in 0..32 {
            let record = fullscreen().diagnostic();
            // Exercise successful process queries as well as failed PIDs in the fixture.
            assert!(process_name(std::process::id()).is_some());
            append_diagnostic(&path, &record).unwrap();
        }
        let rotated = path.with_extension("previous.jsonl");
        assert_eq!(rotated.metadata().unwrap().len(), 1024 * 1024);
        let entries = std::fs::read_to_string(&path).unwrap();
        assert_eq!(entries.lines().count(), 32);
        for line in entries.lines() {
            let entry: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(entry["reason"], "fullscreen");
            assert_eq!(entry["bounds"], serde_json::json!([-1920, 0, 0, 1080]));
        }
        // Deletion also checks file handles were released on Windows.
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(rotated).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
