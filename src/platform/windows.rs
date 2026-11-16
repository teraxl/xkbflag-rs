use crate::lang::Lang;
use crate::platform::TrayCallbacks;
use eframe::egui;
use std::ffi::c_void;
use std::sync::Arc;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetAsyncKeyState, GetKeyboardLayout, HKL, KLF_ACTIVATE, VK_CONTROL,
    VK_LBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowThreadProcessId, PostMessageW, SetWindowLongPtrW, SetWindowPos,
    TranslateMessage, GWL_EXSTYLE, HWND_TOPMOST, MSG, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOSIZE, WM_INPUTLANGCHANGEREQUEST,
    WS_EX_TOOLWINDOW,
};

fn hwnd_from_u64(id: u64) -> HWND {
    HWND(id as *mut c_void)
}
fn hkl_from_lang_id(lang_id: u32) -> HKL {
    HKL(lang_id as usize as *mut c_void)
}

pub fn get_lang() -> Option<Lang> {
    unsafe {
        let hwnd = GetForegroundWindow();
        let tid = if hwnd.is_invalid() {
            0
        } else {
            GetWindowThreadProcessId(hwnd, None)
        };
        let hkl = GetKeyboardLayout(tid);
        let lang_id = (hkl.0 as usize) & 0xFFFF;
        match lang_id {
            0x0419 => Some(Lang::Ru),
            0x0409 => Some(Lang::En),
            _ => None,
        }
    }
}

pub fn set_lang(lang: Lang) {
    let lang_id: u32 = match lang {
        Lang::Ru => 0x0419,
        Lang::En => 0x0409,
    };
    let hkl = hkl_from_lang_id(lang_id);
    unsafe {
        let fg = GetForegroundWindow();
        if !fg.is_invalid() {
            let _ = PostMessageW(
                fg,
                WM_INPUTLANGCHANGEREQUEST,
                WPARAM(0),
                LPARAM(hkl.0 as isize),
            );
        } else {
            let _ = ActivateKeyboardLayout(hkl, KLF_ACTIVATE);
        }
    }
}

pub fn cursor_pos() -> egui::Pos2 {
    let mut p = POINT { x: 0, y: 0 };
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    egui::pos2(p.x as f32, p.y as f32)
}

pub fn is_left_button_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) & 0x8000u16 as i16) != 0 }
}

pub fn is_ctrl_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_CONTROL.0 as i32) & 0x8000u16 as i16) != 0 }
}

pub fn move_window(win_id: u64, x: i32, y: i32) {
    let hwnd = hwnd_from_u64(win_id);
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

pub fn set_skip_taskbar(win_id: u64) {
    let hwnd = hwnd_from_u64(win_id);
    unsafe {
        let cur = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new = cur | (WS_EX_TOOLWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new);
    }
}

pub fn focus_self(_win_id: u64) {}
pub fn focus_under(_win_id: u64) {}

pub fn window_id_from_raw(handle: &raw_window_handle::RawWindowHandle) -> Option<u64> {
    if let raw_window_handle::RawWindowHandle::Win32(w) = handle {
        Some(w.hwnd.get() as u64)
    } else {
        None
    }
}

pub fn spawn_tray(cb: TrayCallbacks) {
    let cb = Arc::new(cb);
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        use tray_icon::{
            menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
            TrayIconBuilder,
        };

        let icon = load_tray_icon_svg(include_bytes!("../../assets/icons/tray.svg"), 32);
        let menu = Menu::new();
        let toggle = MenuItem::new("Переключить язык", true, None);
        let follow = CheckMenuItem::new("Следовать за курсором", true, (cb.is_following)(), None);
        let quit = MenuItem::new("Выход", true, None);
        let _ = menu.append(&toggle);
        let _ = menu.append(&follow);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit);

        let _tray = match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip((cb.title)())
            .with_icon(icon)
            .build()
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!("tray build error: {e:?}");
                return;
            }
        };

        let toggle_id = toggle.id().clone();
        let follow_id = follow.id().clone();
        let quit_id = quit.id().clone();

        let mut msg = MSG::default();
        loop {
            while let Ok(ev) = MenuEvent::receiver().try_recv() {
                if ev.id == toggle_id {
                    (cb.toggle_lang)();
                } else if ev.id == follow_id {
                    (cb.toggle_follow)();
                    let _ = follow.set_checked((cb.is_following)());
                } else if ev.id == quit_id {
                    (cb.quit)();
                }
            }
            unsafe {
                let ret = GetMessageW(&mut msg, None, 0, 0);
                if ret.0 == 0 || ret.0 == -1 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

pub fn screen_size() -> (i32, i32, i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

pub fn try_single_instance() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        let _ = CreateMutexW(None, true, w!("Global\\xkbflag-rs-mutex"));
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

fn load_tray_icon_svg(bytes: &[u8], px: u32) -> tray_icon::Icon {
    use resvg::{tiny_skia, usvg};
    let opt = usvg::Options::default();
    let rtree = usvg::Tree::from_data(bytes, &opt).expect("svg parse");
    let size = rtree.size();
    let scale = px as f32 / size.width().max(size.height());
    let w = (size.width() * scale).round().max(1.0) as u32;
    let h = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).unwrap();
    resvg::render(
        &rtree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for c in pixmap.data().chunks_exact(4) {
        let a = c[3] as u32;
        let r = if a == 0 {
            0
        } else {
            (c[0] as u32 * 255 / a).min(255) as u8
        };
        let g = if a == 0 {
            0
        } else {
            (c[1] as u32 * 255 / a).min(255) as u8
        };
        let b = if a == 0 {
            0
        } else {
            (c[2] as u32 * 255 / a).min(255) as u8
        };
        rgba.extend_from_slice(&[r, g, b, a as u8]);
    }
    tray_icon::Icon::from_rgba(rgba, w, h).expect("tray icon")
}
