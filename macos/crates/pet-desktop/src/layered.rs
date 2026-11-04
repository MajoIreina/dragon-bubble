//! Offscreen readback supplies alpha hit testing and smoke captures on macOS.
//! Windows additionally presents these frames through a native layered window.
use bevy::{prelude::*, render::gpu_readback::ReadbackComplete};

#[cfg(windows)]
pub struct NativePet {
    pub hwnd: windows_sys::Win32::Foundation::HWND,
    hook: windows_sys::Win32::UI::WindowsAndMessaging::HHOOK,
    dc: windows_sys::Win32::Graphics::Gdi::HDC,
    bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    old_bitmap: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
    bits: *mut std::ffi::c_void,
    size: u32,
}
#[cfg(windows)]
static PET_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
#[cfg(windows)]
static WHEEL: std::sync::Mutex<Vec<(i32, Vec2)>> = std::sync::Mutex::new(Vec::new());
#[cfg(windows)]
pub fn take_wheel() -> Vec<(i32, Vec2)> {
    WHEEL
        .lock()
        .map(|mut events| std::mem::take(&mut *events))
        .unwrap_or_default()
}
#[cfg(windows)]
fn queue_wheel(delta: i32, point: windows_sys::Win32::Foundation::POINT) -> bool {
    if let Ok(mut events) = WHEEL.try_lock() {
        events.push((delta, Vec2::new(point.x as f32, point.y as f32)));
        true
    } else {
        false
    }
}
// Receive hover scrolling even when another application has keyboard focus and
// Windows' "scroll inactive windows" preference is disabled. Pass all other input on.
#[cfg(windows)]
unsafe extern "system" fn mouse_hook(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    if code >= 0 && wparam == WM_MOUSEWHEEL as usize {
        let event = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let pet = PET_WINDOW.load(std::sync::atomic::Ordering::Relaxed);
        if pet != 0
            && unsafe { WindowFromPoint(event.pt) } as isize == pet
            && queue_wheel((event.mouseData >> 16) as i16 as i32, event.pt)
        {
            return 1;
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}
#[cfg(windows)]
static CLOSE_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(windows)]
pub fn close_requested() -> bool {
    CLOSE_REQUESTED.swap(false, std::sync::atomic::Ordering::Relaxed)
}
#[cfg(windows)]
unsafe extern "system" fn window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    if message == WM_CLOSE {
        CLOSE_REQUESTED.store(true, std::sync::atomic::Ordering::Relaxed);
        return 0;
    }
    if message == WM_MOUSEWHEEL {
        let point = windows_sys::Win32::Foundation::POINT {
            x: lparam as i16 as i32,
            y: (lparam >> 16) as i16 as i32,
        };
        if unsafe { WindowFromPoint(point) } == hwnd
            && queue_wheel((wparam >> 16) as i16 as i32, point)
        {
            return 0;
        }
    }
    // A layered window never needs an opaque background erase between frames.
    if message == WM_ERASEBKGND {
        return 1;
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
#[cfg(windows)]
impl Drop for NativePet {
    fn drop(&mut self) {
        unsafe {
            PET_WINDOW.store(0, std::sync::atomic::Ordering::Relaxed);
            if !self.hook.is_null() {
                windows_sys::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(self.hook);
            }
            if !self.dc.is_null() {
                if !self.old_bitmap.is_null() {
                    windows_sys::Win32::Graphics::Gdi::SelectObject(self.dc, self.old_bitmap);
                }
                if !self.bitmap.is_null() {
                    windows_sys::Win32::Graphics::Gdi::DeleteObject(self.bitmap);
                }
                windows_sys::Win32::Graphics::Gdi::DeleteDC(self.dc);
            }
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
        }
    }
}
#[cfg(windows)]
pub fn setup(world: &mut World) {
    use windows_sys::Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*};
    let class: Vec<u16> = "LppVulkanLayeredPet\0".encode_utf16().collect();
    let title: Vec<u16> = "龙泡泡 · Rust Vulkan\0".encode_utf16().collect();
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.lpszClassName = class.as_ptr();
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED
                | if std::env::var_os("LPP_WINDOW_TEST").is_some() {
                    WS_EX_APPWINDOW
                } else {
                    WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
                },
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            320,
            320,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        assert!(
            !hwnd.is_null(),
            "Could not create transparent desktop window"
        );
        PET_WINDOW.store(hwnd as isize, std::sync::atomic::Ordering::Relaxed);
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), instance, 0);
        if hook.is_null() {
            warn!("Hover wheel hook unavailable; using window wheel messages");
        }
        world.insert_non_send_resource(NativePet {
            hwnd,
            hook,
            dc: std::ptr::null_mut(),
            bitmap: std::ptr::null_mut(),
            old_bitmap: std::ptr::null_mut(),
            bits: std::ptr::null_mut(),
            size: 0,
        });
    }
}
#[cfg(not(windows))]
pub fn setup() {}

#[derive(Resource, Default)]
pub struct DesktopFrame {
    pub rgba: Vec<u8>,
    pub serial: u64,
    pub size: u32,
}
#[cfg(any(target_os = "macos", test))]
impl DesktopFrame {
    /// UV coordinates in the composited pet window, with the origin at top left.
    pub fn hit_test(&self, uv: Vec2) -> bool {
        if !uv.is_finite() || uv.x < 0. || uv.y < 0. || uv.x >= 1. || uv.y >= 1. {
            return false;
        }
        let x = (uv.x * self.size as f32) as usize;
        let y = (uv.y * self.size as f32) as usize;
        self.rgba
            .get((y * self.size as usize + x) * 4 + 3)
            .is_some_and(|alpha| *alpha > 8)
    }
}
#[derive(Component)]
pub struct FrameSize(pub u32);
pub fn output_size(zoom: f32) -> u32 {
    (320. * pet_core::preferences::zoom(zoom)).round() as u32
}
#[cfg(test)]
fn unpack_rows(data: &[u8], size: u32) -> Option<Vec<u8>> {
    let row = size as usize * 4;
    let pitch = row.div_ceil(256) * 256;
    if data.len() != pitch * size as usize {
        return None;
    }
    Some(
        data.chunks_exact(pitch)
            .flat_map(|line| line[..row].iter().copied())
            .collect(),
    )
}
fn unpack_rows_into(data: &[u8], size: u32, output: &mut Vec<u8>) -> Option<(bool, Option<Vec2>)> {
    let row = size as usize * 4;
    let pitch = row.div_ceil(256) * 256;
    if size == 0 || data.len() != pitch * size as usize {
        return None;
    }
    output.resize(row * size as usize, 0);
    let mut has_alpha = false;
    let mut left = size;
    let mut right = 0;
    for (y, line) in data.chunks_exact(pitch).enumerate() {
        let line = &line[..row];
        output[y * row..(y + 1) * row].copy_from_slice(line);
        for (x, pixel) in line.as_chunks::<4>().0.iter().enumerate() {
            has_alpha |= pixel[3] != 0;
            if pixel[3] > 32 {
                left = left.min(x as u32);
                right = right.max(x as u32 + 1);
            }
        }
    }
    let insets = (left < right).then_some(Vec2::new(left as f32, (size - right) as f32));
    Some((has_alpha, insets))
}
#[cfg(test)]
fn side_insets(pixels: &[u8], size: u32) -> Option<Vec2> {
    if size == 0 || pixels.len() != size as usize * size as usize * 4 {
        return None;
    }
    let mut left = size;
    let mut right = 0;
    for (i, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
        // Ignore the almost transparent fringe left by filtering and temporal AA.
        if pixel[3] > 32 {
            let x = i as u32 % size;
            left = left.min(x);
            right = right.max(x + 1);
        }
    }
    (left < right).then_some(Vec2::new(left as f32, (size - right) as f32))
}
pub fn receive(
    event: On<ReadbackComplete>,
    sizes: Query<&FrameSize>,
    mut frame: ResMut<DesktopFrame>,
    mut rt: ResMut<crate::runtime::Runtime>,
) {
    let Ok(size) = sizes.get(event.entity) else {
        return;
    };
    let Some((has_alpha, insets)) = unpack_rows_into(&event.data, size.0, &mut frame.rgba) else {
        return;
    };
    if frame.serial == 0 && has_alpha {
        info!(
            "Readback {} bytes, max alpha {}",
            event.data.len(),
            event
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[3])
                .max()
                .unwrap_or(0)
        );
    }
    if has_alpha {
        if size.0 == output_size(rt.prefs.zoom)
            && let Some(insets) = insets
        {
            rt.desktop.side_insets = insets;
        }
        frame.size = size.0;
        frame.serial += 1;
    }
}

#[cfg(windows)]
pub fn present(
    frame: Res<DesktopFrame>,
    rt: Res<crate::runtime::Runtime>,
    mut native: NonSendMut<NativePet>,
    _main_thread: bevy::ecs::system::NonSendMarker,
    mut last: Local<(u64, u32)>,
    mut top: Local<Option<bool>>,
) {
    use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
    if rt.obscured {
        unsafe {
            ShowWindow(native.hwnd, SW_HIDE);
        }
        return;
    }
    unsafe {
        if IsWindowVisible(native.hwnd) == 0 {
            ShowWindow(native.hwnd, SW_SHOWNOACTIVATE);
        }
    }
    let size = output_size(rt.prefs.zoom);
    if frame.rgba.is_empty() || frame.size != size || *last == (frame.serial, size) {
        return;
    }
    {
        let hwnd = native.hwnd;
        unsafe {
            // Layered-window alpha hit testing already passes transparent pixels
            // through. Toggling WS_EX_TRANSPARENT from the previous cursor
            // position can lose the first click after moving onto the pet.
            let screen = GetDC(std::ptr::null_mut());
            if native.dc.is_null() {
                native.dc = CreateCompatibleDC(screen);
            }
            if native.dc.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen);
                warn!("Could not create layered-window drawing context");
                return;
            }
            if native.size != size || native.bitmap.is_null() || native.bits.is_null() {
                if !native.bitmap.is_null() {
                    SelectObject(native.dc, native.old_bitmap);
                    DeleteObject(native.bitmap);
                    native.bitmap = std::ptr::null_mut();
                    native.old_bitmap = std::ptr::null_mut();
                    native.bits = std::ptr::null_mut();
                }
                let mut info: BITMAPINFO = std::mem::zeroed();
                info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                info.bmiHeader.biWidth = size as i32;
                info.bmiHeader.biHeight = -(size as i32);
                info.bmiHeader.biPlanes = 1;
                info.bmiHeader.biBitCount = 32;
                info.bmiHeader.biCompression = BI_RGB;
                let mut bits = std::ptr::null_mut();
                native.bitmap = CreateDIBSection(
                    screen,
                    &info,
                    DIB_RGB_COLORS,
                    &mut bits,
                    std::ptr::null_mut(),
                    0,
                );
                if !native.bitmap.is_null() && !bits.is_null() {
                    native.old_bitmap = SelectObject(native.dc, native.bitmap);
                    native.bits = bits;
                    native.size = size;
                }
            }
            if !native.bitmap.is_null() && !native.bits.is_null() {
                let output =
                    std::slice::from_raw_parts_mut(native.bits.cast::<u8>(), frame.rgba.len());
                let lut = srgb_premultiply_lut();
                for (src, dst) in frame
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(output.as_chunks_mut::<4>().0.iter_mut())
                {
                    let alpha = src[3];
                    if alpha == 0 {
                        dst.fill(0);
                    } else if alpha == 255 {
                        dst.copy_from_slice(&[src[2], src[1], src[0], alpha]);
                    } else {
                        let offset = alpha as usize * 256;
                        dst.copy_from_slice(&[
                            lut[offset + src[2] as usize],
                            lut[offset + src[1] as usize],
                            lut[offset + src[0] as usize],
                            alpha,
                        ]);
                    }
                }
                let position = POINT {
                    x: rt.position.x.round() as i32,
                    y: rt.position.y.round() as i32,
                };
                let extent = SIZE {
                    cx: size as i32,
                    cy: size as i32,
                };
                let source = POINT { x: 0, y: 0 };
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                if UpdateLayeredWindow(
                    hwnd, screen, &position, &extent, native.dc, &source, 0, &blend, ULW_ALPHA,
                ) != 0
                {
                    *last = (frame.serial, size);
                    if *top != Some(rt.prefs.always_on_top) {
                        SetWindowPos(
                            hwnd,
                            if rt.prefs.always_on_top {
                                HWND_TOPMOST
                            } else {
                                HWND_NOTOPMOST
                            },
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        );
                        *top = Some(rt.prefs.always_on_top);
                    }
                } else {
                    warn!("Transparent frame presentation failed: {}", GetLastError());
                }
            }
            ReleaseDC(std::ptr::null_mut(), screen);
        }
    }
}

#[cfg(any(windows, test))]
fn srgb_premultiply_lut() -> &'static [u8] {
    static LUT: std::sync::OnceLock<Box<[u8]>> = std::sync::OnceLock::new();
    LUT.get_or_init(|| {
        let mut table = vec![0; 256 * 256].into_boxed_slice();
        for alpha_byte in 1..255 {
            let alpha = alpha_byte as f32 / 255.;
            for channel_byte in 0..=255 {
                let encoded = channel_byte as f32 / 255.;
                let linear = if encoded <= 0.04045 {
                    encoded / 12.92
                } else {
                    ((encoded + 0.055) / 1.055).powf(2.4)
                };
                let straight = (linear / alpha.max(1. / 255.)).clamp(0., 1.);
                let encoded = if straight <= 0.0031308 {
                    straight * 12.92
                } else {
                    1.055 * straight.powf(1. / 2.4) - 0.055
                };
                table[alpha_byte * 256 + channel_byte] = (encoded * alpha * 255.).round() as u8;
            }
        }
        table
    })
}

#[cfg(not(windows))]
pub fn present() {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alpha_hit_test_passes_through_empty_pixels_and_bounds() {
        let frame = DesktopFrame {
            rgba: vec![0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 8, 20, 20, 20, 64],
            serial: 1,
            size: 2,
        };
        assert!(!frame.hit_test(Vec2::new(0.1, 0.1)));
        assert!(frame.hit_test(Vec2::new(0.6, 0.1)));
        assert!(!frame.hit_test(Vec2::new(0.1, 0.6)));
        assert!(frame.hit_test(Vec2::new(0.6, 0.6)));
        assert!(!frame.hit_test(Vec2::new(-0.1, 0.5)));
        assert!(!frame.hit_test(Vec2::new(1., 0.5)));
        assert!(!frame.hit_test(Vec2::NAN));
        assert!(!DesktopFrame::default().hit_test(Vec2::ZERO));
    }

    #[test]
    fn silhouette_padding_follows_facing_and_zoom_without_transparent_fringe() {
        for (size, left, right) in [(320, 85, 260), (960, 255, 780), (320, 60, 235)] {
            let mut pixels = vec![0; size * size * 4];
            assert!(side_insets(&pixels, size as u32).is_none());
            pixels[3] = 12;
            for x in left..right {
                pixels[(size * (size / 2) + x) * 4 + 3] = 255;
            }
            assert_eq!(
                side_insets(&pixels, size as u32),
                Some(Vec2::new(left as f32, (size - right) as f32))
            );
        }
    }
    #[test]
    fn native_size_readback_strips_padding_without_blurring_or_shifting_rows() {
        for size in [64, 320, 374, 640, 960] {
            let row = size as usize * 4;
            let pitch = row.div_ceil(256) * 256;
            let mut data = vec![255; pitch * size as usize];
            for y in 0..size as usize {
                data[y * pitch..y * pitch + row].fill((y % 251) as u8);
            }
            let pixels = unpack_rows(&data, size).unwrap();
            assert_eq!(pixels.len(), row * size as usize);
            for (y, line) in pixels.chunks_exact(row).enumerate() {
                assert!(line.iter().all(|&v| v == (y % 251) as u8));
            }
            assert!(unpack_rows(&data[..data.len() - 1], size).is_none());
        }
        assert_eq!(output_size(3.), 960);
    }

    #[test]
    fn unpack_into_reuses_storage_and_computes_alpha_bounds_in_one_pass() {
        let size: usize = 64;
        let row = size * 4;
        let pitch = row.div_ceil(256) * 256;
        let mut data = vec![0; pitch * size];
        for y in 10..30 {
            for x in 20..40 {
                data[y * pitch + x * 4 + 3] = 255;
            }
        }
        let mut output = Vec::new();
        let (has_alpha, insets) = unpack_rows_into(&data, size as u32, &mut output).unwrap();
        assert!(has_alpha);
        assert_eq!(insets, Some(Vec2::new(20., 24.)));
        let capacity = output.capacity();
        let pointer = output.as_ptr();
        unpack_rows_into(&data, size as u32, &mut output).unwrap();
        assert_eq!(output.capacity(), capacity);
        assert_eq!(output.as_ptr(), pointer);
    }

    #[test]
    fn premultiplication_lookup_matches_the_previous_color_conversion() {
        let table = srgb_premultiply_lut();
        for alpha_byte in [1u8, 2, 16, 32, 64, 127, 128, 192, 254] {
            let alpha = alpha_byte as f32 / 255.;
            for channel_byte in 0..=255u8 {
                let encoded = channel_byte as f32 / 255.;
                let linear = if encoded <= 0.04045 {
                    encoded / 12.92
                } else {
                    ((encoded + 0.055) / 1.055).powf(2.4)
                };
                let straight = (linear / alpha.max(1. / 255.)).clamp(0., 1.);
                let encoded = if straight <= 0.0031308 {
                    straight * 12.92
                } else {
                    1.055 * straight.powf(1. / 2.4) - 0.055
                };
                let expected = (encoded * alpha * 255.).round() as u8;
                assert_eq!(
                    table[alpha_byte as usize * 256 + channel_byte as usize],
                    expected
                );
            }
        }
    }
}
