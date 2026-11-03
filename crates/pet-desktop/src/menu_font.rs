//! Per-menu YaHei drawing; never changes the user's Windows system fonts.
use tray_icon::menu::{ContextMenu, Menu};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Controls::*, HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

struct Item {
    text: Vec<u16>,
    font: HFONT,
    scale: f32,
    submenu: bool,
}
struct StyledItem {
    menu: HMENU,
    index: u32,
    original: MENUITEMINFOW,
    // Stable address used only while TrackPopupMenu's synchronous loop runs.
    _item: Box<Item>,
}

pub fn show(menu: &Menu, hwnd: HWND) {
    unsafe {
        let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.;
        let face: Vec<u16> = "Microsoft YaHei\0".encode_utf16().collect();
        let font = CreateFontW(
            -(14. * scale).round() as i32,
            0,
            0,
            0,
            500,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            face.as_ptr(),
        );
        if font.is_null() {
            menu.show_context_menu_for_hwnd(hwnd as isize, None);
            return;
        }
        let mut items = vec![];
        style(menu.hpopupmenu() as HMENU, font, scale, &mut items);
        menu.show_context_menu_for_hwnd(hwnd as isize, None);
        for entry in &items {
            SetMenuItemInfoW(entry.menu, entry.index, 1, &entry.original);
        }
        DeleteObject(font);
    }
}
unsafe fn style(menu: HMENU, font: HFONT, scale: f32, items: &mut Vec<StyledItem>) {
    unsafe {
        for index in 0..GetMenuItemCount(menu).max(0) as u32 {
            let mut info: MENUITEMINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
            info.fMask = MIIM_FTYPE | MIIM_DATA | MIIM_SUBMENU;
            if GetMenuItemInfoW(menu, index, 1, &mut info) == 0 {
                continue;
            }
            if !info.hSubMenu.is_null() {
                style(info.hSubMenu, font, scale, items);
            }
            if info.fType & MFT_SEPARATOR != 0 {
                continue;
            }
            let len = GetMenuStringW(menu, index, std::ptr::null_mut(), 0, MF_BYPOSITION);
            let mut text = vec![0u16; len.max(0) as usize + 1];
            GetMenuStringW(
                menu,
                index,
                text.as_mut_ptr(),
                text.len() as i32,
                MF_BYPOSITION,
            );
            text.pop();
            let item = Box::new(Item {
                text,
                font,
                scale,
                submenu: !info.hSubMenu.is_null(),
            });
            let original = info;
            info.fMask = MIIM_FTYPE | MIIM_DATA;
            info.fType |= MFT_OWNERDRAW;
            info.dwItemData = (&*item as *const Item) as usize;
            if SetMenuItemInfoW(menu, index, 1, &info) != 0 {
                items.push(StyledItem {
                    menu,
                    index,
                    original,
                    _item: item,
                });
            }
        }
    }
}
/// Called only by our native pet window, which owns these popups.
pub unsafe fn handle(hwnd: HWND, message: u32, lparam: LPARAM) -> Option<LRESULT> {
    unsafe {
        if lparam == 0 {
            return None;
        }
        if message == WM_MEASUREITEM {
            let measure = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            if measure.CtlType != ODT_MENU || measure.itemData == 0 {
                return None;
            }
            let item = &*(measure.itemData as *const Item);
            let dc = GetDC(hwnd);
            let old = SelectObject(dc, item.font);
            let mut size: SIZE = std::mem::zeroed();
            GetTextExtentPoint32W(dc, item.text.as_ptr(), item.text.len() as i32, &mut size);
            SelectObject(dc, old);
            ReleaseDC(hwnd, dc);
            measure.itemWidth = (size.cx + (48. * item.scale) as i32) as u32;
            measure.itemHeight = (30. * item.scale).round() as u32;
            return Some(1);
        }
        if message == WM_DRAWITEM {
            let draw = &*(lparam as *const DRAWITEMSTRUCT);
            if draw.CtlType != ODT_MENU || draw.itemData == 0 {
                return None;
            }
            let item = &*(draw.itemData as *const Item);
            let selected = draw.itemState & ODS_SELECTED != 0;
            let bg = if selected {
                COLOR_HIGHLIGHT
            } else {
                COLOR_MENU
            };
            let fg = if draw.itemState & ODS_DISABLED != 0 {
                COLOR_GRAYTEXT
            } else if selected {
                COLOR_HIGHLIGHTTEXT
            } else {
                COLOR_MENUTEXT
            };
            FillRect(draw.hDC, &draw.rcItem, GetSysColorBrush(bg));
            let saved = SaveDC(draw.hDC);
            SelectObject(draw.hDC, item.font);
            SetBkMode(draw.hDC, TRANSPARENT as i32);
            SetTextColor(draw.hDC, GetSysColor(fg));
            let mut rect = draw.rcItem;
            rect.left += (16. * item.scale) as i32;
            rect.right -= (24. * item.scale) as i32;
            DrawTextW(
                draw.hDC,
                item.text.as_ptr(),
                item.text.len() as i32,
                &mut rect,
                DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
            );
            if item.submenu {
                rect.left = rect.right + (6. * item.scale) as i32;
                rect.right = draw.rcItem.right;
                let arrow = [0x203a_u16];
                DrawTextW(
                    draw.hDC,
                    arrow.as_ptr(),
                    1,
                    &mut rect,
                    DT_SINGLELINE | DT_VCENTER,
                );
            }
            RestoreDC(draw.hDC, saved);
            return Some(1);
        }
        None
    }
}
