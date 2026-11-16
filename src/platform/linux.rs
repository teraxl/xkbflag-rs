use crate::lang::Lang;
use crate::platform::TrayCallbacks;
use eframe::egui;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void};
use std::sync::{Arc, Mutex};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XkbStateRec {
    group: c_uchar,
    locked_group: c_uchar,
    base_group: u16,
    latched_group: u16,
    mods: c_uchar,
    base_mods: c_uchar,
    latched_mods: c_uchar,
    locked_mods: c_uchar,
    compat_state: c_uchar,
    grab_mods: c_uchar,
    compat_grab_mods: c_uchar,
    lookup_mods: c_uchar,
    compat_lookup_mods: c_uchar,
    ptr_buttons: u16,
    _pad: [u64; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XWindowAttributes {
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    depth: c_int,
    visual: *mut c_void,
    root: c_ulong,
    class: c_int,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: c_int,
    colormap: c_ulong,
    map_installed: c_int,
    map_state: c_int,
    all_event_masks: c_long,
    your_event_mask: c_long,
    do_not_propagate_mask: c_long,
    override_redirect: c_int,
    screen: *mut c_void,
    _pad: [u64; 8],
}

#[link(name = "X11")]
extern "C" {
    fn XInitThreads() -> c_int;
    fn XOpenDisplay(name: *const c_char) -> *mut c_void;
    fn XCloseDisplay(dpy: *mut c_void) -> c_int;
    fn XkbLockGroup(dpy: *mut c_void, device_spec: c_uint, group: c_uint) -> c_int;
    fn XkbGetState(dpy: *mut c_void, device_spec: c_uint, state_return: *mut XkbStateRec) -> c_int;
    fn XSync(dpy: *mut c_void, discard: c_int) -> c_int;
    fn XQueryPointer(
        dpy: *mut c_void,
        w: c_ulong,
        root: *mut c_ulong,
        child: *mut c_ulong,
        root_x: *mut c_int,
        root_y: *mut c_int,
        win_x: *mut c_int,
        win_y: *mut c_int,
        mask: *mut c_uint,
    ) -> c_int;
    fn XDefaultRootWindow(dpy: *mut c_void) -> c_ulong;
    fn XInternAtom(dpy: *mut c_void, atom_name: *const c_char, only_if_exists: c_int) -> c_ulong;
    fn XChangeProperty(
        dpy: *mut c_void,
        w: c_ulong,
        property: c_ulong,
        type_: c_ulong,
        format: c_int,
        mode: c_int,
        data: *const c_uchar,
        nelements: c_int,
    ) -> c_int;
    fn XSendEvent(
        dpy: *mut c_void,
        w: c_ulong,
        propagate: c_int,
        event_mask: c_long,
        event_send: *mut XEvent,
    ) -> c_int;
    fn XGetWindowProperty(
        dpy: *mut c_void,
        w: c_ulong,
        property: c_ulong,
        long_offset: c_long,
        long_length: c_long,
        delete: c_int,
        req_type: c_ulong,
        actual_type_return: *mut c_ulong,
        actual_format_return: *mut c_int,
        nitems_return: *mut c_ulong,
        bytes_after_return: *mut c_ulong,
        prop_return: *mut *mut c_uchar,
    ) -> c_int;
    fn XFree(data: *mut c_void) -> c_int;
    fn XGetWindowAttributes(dpy: *mut c_void, w: c_ulong, ret: *mut XWindowAttributes) -> c_int;
    fn XTranslateCoordinates(
        dpy: *mut c_void,
        src: c_ulong,
        dst: c_ulong,
        sx: c_int,
        sy: c_int,
        dx: *mut c_int,
        dy: *mut c_int,
        child: *mut c_ulong,
    ) -> c_int;
    fn XSetInputFocus(dpy: *mut c_void, focus: c_ulong, revert_to: c_int, time: c_ulong) -> c_int;
    fn XRaiseWindow(dpy: *mut c_void, w: c_ulong) -> c_int;
    fn XMoveWindow(dpy: *mut c_void, w: c_ulong, x: c_int, y: c_int) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XClientMessageEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut c_void,
    window: c_ulong,
    message_type: c_ulong,
    format: c_int,
    data: [c_long; 5],
}

#[repr(C)]
union XEvent {
    type_: c_int,
    xclient: XClientMessageEvent,
    pad: [c_long; 24],
}

pub fn init() {
    unsafe {
        XInitThreads();
    }
}

pub fn get_lang() -> Option<Lang> {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return None;
        }
        let mut st = XkbStateRec::default();
        let ok = XkbGetState(dpy, 0x0100, &mut st) == 0;
        XCloseDisplay(dpy);
        if ok {
            Some(Lang::from_group(st.group as u32))
        } else {
            None
        }
    }
}

pub fn set_lang(lang: Lang) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if !dpy.is_null() {
            XkbLockGroup(dpy, 0x0100, lang.to_group() as c_uint);
            XSync(dpy, 0);
            XCloseDisplay(dpy);
        }
    }
}

pub fn cursor_pos() -> egui::Pos2 {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return egui::pos2(100.0, 100.0);
        }
        let root = XDefaultRootWindow(dpy);
        let (mut rx, mut ry) = (0, 0);
        let (mut wx, mut wy) = (0, 0);
        let mut mask: c_uint = 0;
        let (mut rr, mut cr) = (0u64, 0u64);
        let ok = XQueryPointer(
            dpy, root, &mut rr, &mut cr, &mut rx, &mut ry, &mut wx, &mut wy, &mut mask,
        ) != 0;
        XCloseDisplay(dpy);
        if ok {
            egui::pos2(rx as f32, ry as f32)
        } else {
            egui::pos2(100.0, 100.0)
        }
    }
}

pub fn is_left_button_pressed() -> bool {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return false;
        }
        let root = XDefaultRootWindow(dpy);
        let (mut rx, mut ry) = (0, 0);
        let (mut wx, mut wy) = (0, 0);
        let mut mask: c_uint = 0;
        let (mut rr, mut cr) = (0u64, 0u64);
        let ok = XQueryPointer(
            dpy, root, &mut rr, &mut cr, &mut rx, &mut ry, &mut wx, &mut wy, &mut mask,
        ) != 0;
        XCloseDisplay(dpy);
        ok && (mask & (1 << 8)) != 0
    }
}

pub fn move_window(win_id: u64, x: i32, y: i32) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if !dpy.is_null() {
            XMoveWindow(dpy, win_id, x, y);
            XSync(dpy, 0);
            XCloseDisplay(dpy);
        }
    }
}

pub fn set_skip_taskbar(win_id: u64) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return;
        }
        let root = XDefaultRootWindow(dpy);
        let wm_state = XInternAtom(dpy, CString::new("_NET_WM_STATE").unwrap().as_ptr(), 0);
        let st = XInternAtom(
            dpy,
            CString::new("_NET_WM_STATE_SKIP_TASKBAR").unwrap().as_ptr(),
            0,
        );
        let sp = XInternAtom(
            dpy,
            CString::new("_NET_WM_STATE_SKIP_PAGER").unwrap().as_ptr(),
            0,
        );
        let atoms = [st, sp];
        XChangeProperty(
            dpy,
            win_id,
            wm_state,
            4,
            32,
            0,
            atoms.as_ptr() as *const c_uchar,
            2,
        );
        let mut ev = XEvent {
            xclient: XClientMessageEvent {
                type_: 33,
                serial: 0,
                send_event: 1,
                display: dpy,
                window: win_id,
                message_type: wm_state,
                format: 32,
                data: [1, st as c_long, sp as c_long, 1, 0],
            },
        };
        XSendEvent(dpy, root, 0, (1 << 20) | (1 << 19), &mut ev);
        XSync(dpy, 0);
        XCloseDisplay(dpy);
    }
}

pub fn focus_self(win_id: u64) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return;
        }
        let root = XDefaultRootWindow(dpy);
        let active = XInternAtom(dpy, CString::new("_NET_ACTIVE_WINDOW").unwrap().as_ptr(), 0);
        let mut ev = XEvent {
            xclient: XClientMessageEvent {
                type_: 33,
                serial: 0,
                send_event: 1,
                display: dpy,
                window: win_id,
                message_type: active,
                format: 32,
                data: [1, 0, 0, 0, 0],
            },
        };
        XSendEvent(dpy, root, 0, (1 << 20) | (1 << 19), &mut ev);
        XRaiseWindow(dpy, win_id);
        XSetInputFocus(dpy, win_id, 1, 0);
        XSync(dpy, 0);
        XCloseDisplay(dpy);
    }
}

pub fn focus_under(win_id: u64) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return;
        }
        let root = XDefaultRootWindow(dpy);
        let mut attrs = XWindowAttributes::default();
        if XGetWindowAttributes(dpy, win_id, &mut attrs) == 0 {
            XCloseDisplay(dpy);
            return;
        }
        let (mut cx, mut cy) = (0, 0);
        let mut ch: c_ulong = 0;
        XTranslateCoordinates(
            dpy,
            win_id,
            root,
            attrs.width / 2,
            attrs.height / 2,
            &mut cx,
            &mut cy,
            &mut ch,
        );

        let stacking = XInternAtom(
            dpy,
            CString::new("_NET_CLIENT_LIST_STACKING").unwrap().as_ptr(),
            0,
        );
        let wm_state = XInternAtom(dpy, CString::new("_NET_WM_STATE").unwrap().as_ptr(), 0);
        let hidden = XInternAtom(
            dpy,
            CString::new("_NET_WM_STATE_HIDDEN").unwrap().as_ptr(),
            0,
        );
        let active = XInternAtom(dpy, CString::new("_NET_ACTIVE_WINDOW").unwrap().as_ptr(), 0);

        let mut at: c_ulong = 0;
        let mut af: c_int = 0;
        let mut ni: c_ulong = 0;
        let mut ba: c_ulong = 0;
        let mut pp: *mut c_uchar = std::ptr::null_mut();
        let st = XGetWindowProperty(
            dpy, root, stacking, 0, 1024, 0, 33, &mut at, &mut af, &mut ni, &mut ba, &mut pp,
        );
        let mut target: Option<c_ulong> = None;
        if st == 0 && !pp.is_null() && ni > 0 {
            let wins = std::slice::from_raw_parts(pp as *const c_ulong, ni as usize);
            for &w in wins.iter().rev() {
                if w == win_id {
                    continue;
                }
                let mut a = XWindowAttributes::default();
                if XGetWindowAttributes(dpy, w, &mut a) == 0 || a.map_state != 2 {
                    continue;
                }
                let mut t2: c_ulong = 0;
                let mut f2: c_int = 0;
                let mut i2: c_ulong = 0;
                let mut b2: c_ulong = 0;
                let mut p2: *mut c_uchar = std::ptr::null_mut();
                let mut is_h = false;
                if XGetWindowProperty(
                    dpy, w, wm_state, 0, 64, 0, 4, &mut t2, &mut f2, &mut i2, &mut b2, &mut p2,
                ) == 0
                    && !p2.is_null()
                {
                    let s = std::slice::from_raw_parts(p2 as *const c_ulong, i2 as usize);
                    if s.contains(&hidden) {
                        is_h = true;
                    }
                    XFree(p2 as *mut c_void);
                }
                if is_h {
                    continue;
                }
                let (mut wx, mut wy) = (0, 0);
                XTranslateCoordinates(dpy, w, root, 0, 0, &mut wx, &mut wy, &mut ch);
                if cx >= wx && cx <= wx + a.width && cy >= wy && cy <= wy + a.height {
                    target = Some(w);
                    break;
                }
            }
            XFree(pp as *mut c_void);
        }
        if let Some(w) = target {
            let mut ev = XEvent {
                xclient: XClientMessageEvent {
                    type_: 33,
                    serial: 0,
                    send_event: 1,
                    display: dpy,
                    window: w,
                    message_type: active,
                    format: 32,
                    data: [2, 0, 0, 0, 0],
                },
            };
            XSendEvent(dpy, root, 0, (1 << 20) | (1 << 19), &mut ev);
            XSetInputFocus(dpy, w, 1, 0);
            XSync(dpy, 0);
        }
        XCloseDisplay(dpy);
    }
}

pub fn window_id_from_raw(handle: &raw_window_handle::RawWindowHandle) -> Option<u64> {
    if let raw_window_handle::RawWindowHandle::Xlib(x) = handle {
        Some(x.window)
    } else {
        None
    }
}

struct LinuxTray {
    cb: Arc<TrayCallbacks>,
}

impl ksni::Tray for LinuxTray {
    fn id(&self) -> String {
        "xkbflag-rs".into()
    }
    fn title(&self) -> String {
        (self.cb.title)()
    }
    fn icon_name(&self) -> String {
        "input-keyboard".into()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        (self.cb.toggle_lang)();
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, StandardItem};
        let following = (self.cb.is_following)();
        vec![
            StandardItem {
                label: "Переключить язык".into(),
                activate: Box::new(|this: &mut Self| (this.cb.toggle_lang)()),
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Следовать за курсором".into(),
                checked: following,
                activate: Box::new(|this: &mut Self| (this.cb.toggle_follow)()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Выход".into(),
                activate: Box::new(|this: &mut Self| (this.cb.quit)()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn_tray(cb: TrayCallbacks) {
    let cb = Arc::new(cb);
    std::thread::spawn(move || {
        let service = ksni::TrayService::new(LinuxTray { cb });
        let _ = service.run();
    });
}

pub fn is_ctrl_pressed() -> bool {
    false
}

pub fn screen_size() -> (i32, i32, i32, i32) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() {
            return (0, 0, 1920, 1080);
        }
        let screen = XDefaultScreen(dpy);
        let w = XDisplayWidth(dpy, screen);
        let h = XDisplayHeight(dpy, screen);
        XCloseDisplay(dpy);
        (0, 0, w, h)
    }
}

pub fn try_single_instance() -> bool {
    use std::os::unix::io::AsRawFd;
    let path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| format!("{}/xkbflag-rs.lock", d))
        .unwrap_or_else(|_| "/tmp/xkbflag-rs.lock".to_string());
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
    {
        Ok(f) => f,
        Err(_) => return true,
    };
    let fd = file.as_raw_fd();
    let rc = unsafe { libc_flock(fd, 2) }; // LOCK_EX = 2
    if rc != 0 {
        return false;
    }
    std::mem::forget(file); // держим fd открытым на всё время работы
    true
}

extern "C" {
    #[link_name = "flock"]
    fn libc_flock(fd: c_int, op: c_int) -> c_int;
}
