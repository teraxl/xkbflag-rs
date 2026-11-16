use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use raw_window_handle::HasWindowHandle;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const RU_SVG: &[u8] = include_bytes!("../assets/flags/ru.svg");
const US_SVG: &[u8] = include_bytes!("../assets/flags/us.svg");

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct XkbStateRec {
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
struct XWindowAttributes {
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
        dpy: *mut c_void, w: c_ulong, root: *mut c_ulong, child: *mut c_ulong,
        root_x: *mut c_int, root_y: *mut c_int,
        win_x: *mut c_int, win_y: *mut c_int, mask: *mut c_uint,
    ) -> c_int;
    fn XDefaultRootWindow(dpy: *mut c_void) -> c_ulong;
    fn XInternAtom(dpy: *mut c_void, atom_name: *const c_char, only_if_exists: c_int) -> c_ulong;
    fn XChangeProperty(
        dpy: *mut c_void, w: c_ulong, property: c_ulong, type_: c_ulong,
        format: c_int, mode: c_int, data: *const c_uchar, nelements: c_int,
    ) -> c_int;
    fn XSendEvent(
        dpy: *mut c_void, w: c_ulong, propagate: c_int, event_mask: c_long, event_send: *mut XEvent,
    ) -> c_int;
    fn XGetWindowProperty(
        dpy: *mut c_void, w: c_ulong, property: c_ulong, long_offset: c_long,
        long_length: c_long, delete: c_int, req_type: c_ulong,
        actual_type_return: *mut c_ulong, actual_format_return: *mut c_int,
        nitems_return: *mut c_ulong, bytes_after_return: *mut c_ulong,
        prop_return: *mut *mut c_uchar,
    ) -> c_int;
    fn XFree(data: *mut c_void) -> c_int;
    fn XGetWindowAttributes(
        dpy: *mut c_void, w: c_ulong, window_attributes_return: *mut XWindowAttributes,
    ) -> c_int;
    fn XTranslateCoordinates(
        dpy: *mut c_void, src_w: c_ulong, dest_w: c_ulong,
        src_x: c_int, src_y: c_int, dest_x_return: *mut c_int,
        dest_y_return: *mut c_int, child_return: *mut c_ulong,
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

#[derive(Clone, Copy, PartialEq)]
enum Lang { Ru, En }

impl Lang {
    fn toggle(&mut self) { *self = if *self == Lang::Ru { Lang::En } else { Lang::Ru }; }
    fn set_xkb(&self) {
        let group: c_uint = if *self == Lang::Ru { 1 } else { 0 };
        unsafe {
            let dpy = XOpenDisplay(std::ptr::null());
            if !dpy.is_null() {
                XkbLockGroup(dpy, 0x0100, group);
                XSync(dpy, 0);
                XCloseDisplay(dpy);
            }
        }
    }
    fn from_system() -> Option<Self> {
        unsafe {
            let dpy = XOpenDisplay(std::ptr::null());
            if dpy.is_null() { return None; }
            let mut state = XkbStateRec::default();
            let res = XkbGetState(dpy, 0x0100, &mut state);
            XCloseDisplay(dpy);
            if res == 0 {
                Some(if state.group == 1 { Lang::Ru } else { Lang::En })
            } else { None }
        }
    }
}

fn cursor_pos() -> egui::Pos2 {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() { return egui::pos2(100.0, 100.0); }
        let root = XDefaultRootWindow(dpy);
        let (mut rx_, mut ry) = (0, 0);
        let (mut wx, mut wy) = (0, 0);
        let mut mask: c_uint = 0;
        let (mut rr, mut cr) = (0u64, 0u64);
        let ok = XQueryPointer(dpy, root, &mut rr, &mut cr, &mut rx_, &mut ry, &mut wx, &mut wy, &mut mask) != 0;
        XCloseDisplay(dpy);
        if ok { egui::pos2(rx_ as f32, ry as f32) } else { egui::pos2(100.0, 100.0) }
    }
}

fn is_left_button_pressed() -> bool {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() { return false; }
        let root = XDefaultRootWindow(dpy);
        let (mut rx_, mut ry) = (0, 0);
        let (mut wx, mut wy) = (0, 0);
        let mut mask: c_uint = 0;
        let (mut rr, mut cr) = (0u64, 0u64);
        let ok = XQueryPointer(dpy, root, &mut rr, &mut cr, &mut rx_, &mut ry, &mut wx, &mut wy, &mut mask) != 0;
        XCloseDisplay(dpy);
        ok && (mask & (1 << 8)) != 0
    }
}

fn set_x11_skip_taskbar(win_id: u64) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() { return; }
        let root = XDefaultRootWindow(dpy);
        let wm_state = XInternAtom(dpy, CString::new("_NET_WM_STATE").unwrap().as_ptr(), 0);
        let skip_taskbar = XInternAtom(dpy, CString::new("_NET_WM_STATE_SKIP_TASKBAR").unwrap().as_ptr(), 0);
        let skip_pager = XInternAtom(dpy, CString::new("_NET_WM_STATE_SKIP_PAGER").unwrap().as_ptr(), 0);
        let xa_atom = 4;
        let atoms = [skip_taskbar, skip_pager];
        XChangeProperty(dpy, win_id, wm_state, xa_atom, 32, 0, atoms.as_ptr() as *const c_uchar, 2);
        let mut ev = XEvent {
            xclient: XClientMessageEvent {
                type_: 33, serial: 0, send_event: 1, display: dpy, window: win_id,
                message_type: wm_state, format: 32,
                data: [1, skip_taskbar as c_long, skip_pager as c_long, 1, 0],
            },
        };
        XSendEvent(dpy, root, 0, (1 << 20) | (1 << 19), &mut ev);
        XSync(dpy, 0);
        XCloseDisplay(dpy);
    }
}

fn focus_self_window(self_win_id: u64) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() { return; }
        let root = XDefaultRootWindow(dpy);
        let active_win_atom = XInternAtom(dpy, CString::new("_NET_ACTIVE_WINDOW").unwrap().as_ptr(), 0);
        let mut ev = XEvent {
            xclient: XClientMessageEvent {
                type_: 33, serial: 0, send_event: 1, display: dpy, window: self_win_id,
                message_type: active_win_atom, format: 32, data: [1, 0, 0, 0, 0],
            },
        };
        XSendEvent(dpy, root, 0, (1 << 20) | (1 << 19), &mut ev);
        XRaiseWindow(dpy, self_win_id);
        XSetInputFocus(dpy, self_win_id, 1, 0);
        XSync(dpy, 0);
        XCloseDisplay(dpy);
    }
}

fn focus_window_under_flag(self_win_id: u64) {
    unsafe {
        let dpy = XOpenDisplay(std::ptr::null());
        if dpy.is_null() { return; }
        let root = XDefaultRootWindow(dpy);
        let mut self_attrs = XWindowAttributes::default();
        if XGetWindowAttributes(dpy, self_win_id, &mut self_attrs) == 0 {
            XCloseDisplay(dpy);
            return;
        }
        let mut center_x: c_int = 0;
        let mut center_y: c_int = 0;
        let mut child_dummy: c_ulong = 0;
        XTranslateCoordinates(dpy, self_win_id, root, self_attrs.width / 2, self_attrs.height / 2, &mut center_x, &mut center_y, &mut child_dummy);

        let stacking_atom = XInternAtom(dpy, CString::new("_NET_CLIENT_LIST_STACKING").unwrap().as_ptr(), 0);
        let wm_state_atom = XInternAtom(dpy, CString::new("_NET_WM_STATE").unwrap().as_ptr(), 0);
        let hidden_atom = XInternAtom(dpy, CString::new("_NET_WM_STATE_HIDDEN").unwrap().as_ptr(), 0);
        let active_win_atom = XInternAtom(dpy, CString::new("_NET_ACTIVE_WINDOW").unwrap().as_ptr(), 0);

        let mut actual_type: c_ulong = 0;
        let mut actual_format: c_int = 0;
        let mut nitems: c_ulong = 0;
        let mut bytes_after: c_ulong = 0;
        let mut prop_ptr: *mut c_uchar = std::ptr::null_mut();

        let status = XGetWindowProperty(dpy, root, stacking_atom, 0, 1024, 0, 33, &mut actual_type, &mut actual_format, &mut nitems, &mut bytes_after, &mut prop_ptr);
        let mut target_win: Option<c_ulong> = None;

        if status == 0 && !prop_ptr.is_null() && nitems > 0 {
            let windows = std::slice::from_raw_parts(prop_ptr as *const c_ulong, nitems as usize);
            for &win in windows.iter().rev() {
                if win == self_win_id { continue; }
                let mut attrs = XWindowAttributes::default();
                if XGetWindowAttributes(dpy, win, &mut attrs) == 0 || attrs.map_state != 2 { continue; }

                let mut st_type: c_ulong = 0;
                let mut st_fmt: c_int = 0;
                let mut st_items: c_ulong = 0;
                let mut st_after: c_ulong = 0;
                let mut st_ptr: *mut c_uchar = std::ptr::null_mut();
                let mut is_hidden = false;
                if XGetWindowProperty(dpy, win, wm_state_atom, 0, 64, 0, 4, &mut st_type, &mut st_fmt, &mut st_items, &mut st_after, &mut st_ptr) == 0 && !st_ptr.is_null() {
                    let states = std::slice::from_raw_parts(st_ptr as *const c_ulong, st_items as usize);
                    if states.contains(&hidden_atom) { is_hidden = true; }
                    XFree(st_ptr as *mut c_void);
                }
                if is_hidden { continue; }

                let mut wx: c_int = 0;
                let mut wy: c_int = 0;
                XTranslateCoordinates(dpy, win, root, 0, 0, &mut wx, &mut wy, &mut child_dummy);
                if center_x >= wx && center_x <= wx + attrs.width && center_y >= wy && center_y <= wy + attrs.height {
                    target_win = Some(win);
                    break;
                }
            }
            XFree(prop_ptr as *mut c_void);
        }

        if let Some(win) = target_win {
            let mut ev = XEvent {
                xclient: XClientMessageEvent {
                    type_: 33, serial: 0, send_event: 1, display: dpy, window: win,
                    message_type: active_win_atom, format: 32, data: [2, 0, 0, 0, 0],
                },
            };
            XSendEvent(dpy, root, 0, (1 << 20) | (1 << 19), &mut ev);
            XSetInputFocus(dpy, win, 1, 0);
            XSync(dpy, 0);
        }
        XCloseDisplay(dpy);
    }
}

fn load_flag_svg(ctx: &egui::Context, name: &str, bytes: &[u8], px: u32) -> egui::TextureHandle {
    use resvg::{tiny_skia, usvg};
    let opt = usvg::Options::default();
    let rtree = usvg::Tree::from_data(bytes, &opt).expect("svg parse");
    let size = rtree.size();
    let scale = px as f32 / size.width().max(size.height());
    let w = (size.width() * scale).round().max(1.0) as u32;
    let h = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).unwrap();
    resvg::render(&rtree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for chunk in pixmap.data().chunks_exact(4) {
        let a = chunk[3] as u32;
        let r = if a == 0 { 0 } else { (chunk[0] as u32 * 255 / a).min(255) as u8 };
        let g = if a == 0 { 0 } else { (chunk[1] as u32 * 255 / a).min(255) as u8 };
        let b = if a == 0 { 0 } else { (chunk[2] as u32 * 255 / a).min(255) as u8 };
        rgba.extend_from_slice(&[r, g, b, a as u8]);
    }
    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture(name.to_string(), img, egui::TextureOptions::LINEAR)
}

struct App {
    lang: Arc<Mutex<Lang>>,
    follow_cursor: Arc<AtomicBool>,
    ctrl_held: Arc<AtomicBool>,
    size: f32,
    last_size: f32,
    rx: Receiver<()>,
    x11_configured: bool,
    x11_win_id: Option<u64>,
    is_dragging: bool,
    tex_ru: egui::TextureHandle,
    tex_us: egui::TextureHandle,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        rx: Receiver<()>,
        lang: Arc<Mutex<Lang>>,
        follow_cursor: Arc<AtomicBool>,
        ctrl_held: Arc<AtomicBool>,
    ) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.window_fill = egui::Color32::TRANSPARENT;
        style.visuals.panel_fill = egui::Color32::TRANSPARENT;
        cc.egui_ctx.set_style(style);

        let tex_ru = load_flag_svg(&cc.egui_ctx, "flag_ru", RU_SVG, 256);
        let tex_us = load_flag_svg(&cc.egui_ctx, "flag_us", US_SVG, 256);

        Self {
            lang,
            follow_cursor,
            ctrl_held,
            size: 48.0,
            last_size: 48.0,
            rx,
            x11_configured: false,
            x11_win_id: None,
            is_dragging: false,
            tex_ru,
            tex_us,
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] { [0.0, 0.0, 0.0, 0.0] }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.x11_configured {
            if let Ok(handle) = frame.window_handle() {
                if let raw_window_handle::RawWindowHandle::Xlib(xlib) = handle.as_raw() {
                    self.x11_win_id = Some(xlib.window);
                    set_x11_skip_taskbar(xlib.window);
                    self.x11_configured = true;
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        }

        let follow_enabled = self.follow_cursor.load(Ordering::Relaxed);
        let ctrl = self.ctrl_held.load(Ordering::Relaxed);
        // В режиме следования Ctrl временно отключает follow и разрешает клик/драг по флагу.
        let following = follow_enabled && !ctrl;

        if self.is_dragging {
            if !is_left_button_pressed() {
                self.is_dragging = false;
                if let Some(win_id) = self.x11_win_id {
                    focus_window_under_flag(win_id);
                }
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        while self.rx.try_recv().is_ok() {
            if let Ok(mut l) = self.lang.lock() {
                l.toggle();
                l.set_xkb();
            }
        }

        if (self.size - self.last_size).abs() > f32::EPSILON {
            let w = self.size;
            let h = self.size * 2.0 / 3.0;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
            self.last_size = self.size;
        }

        let current_lang = *self.lang.lock().unwrap();
        let tex = if current_lang == Lang::Ru { &self.tex_ru } else { &self.tex_us };

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0_f32, egui::Color32::from_black_alpha(120)),
                );

                if resp.hovered() {
                    let scroll = ctx.input(|i| i.raw_scroll_delta.y);
                    if scroll != 0.0 {
                        self.size = (self.size + scroll * 0.5).clamp(16.0, 400.0);
                    }
                }

                if !following && resp.drag_started() {
                    self.is_dragging = true;
                    if let Some(win_id) = self.x11_win_id {
                        focus_self_window(win_id);
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                // клик работает всегда, когда follow отключён (в т.ч. на время Ctrl)
                if !following && resp.clicked() {
                    if let Ok(mut l) = self.lang.lock() {
                        l.toggle();
                        l.set_xkb();
                    }
                    if let Some(win_id) = self.x11_win_id {
                        focus_window_under_flag(win_id);
                    }
                }
            });

        if following {
            if let Some(win_id) = self.x11_win_id {
                let c = cursor_pos();
                let target_x = (c.x + 50.0) as c_int;
                let target_y = (c.y + 50.0) as c_int;
                unsafe {
                    let dpy = XOpenDisplay(std::ptr::null());
                    if !dpy.is_null() {
                        XMoveWindow(dpy, win_id, target_x, target_y);
                        XSync(dpy, 0);
                        XCloseDisplay(dpy);
                    }
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

struct Tray {
    tx: Sender<()>,
    lang: Arc<Mutex<Lang>>,
    follow_cursor: Arc<AtomicBool>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String { "langflag".into() }
    fn title(&self) -> String {
        let l = *self.lang.lock().unwrap();
        if l == Lang::Ru { "Раскладка: RU".into() } else { "Раскладка: EN".into() }
    }
    fn icon_name(&self) -> String { "input-keyboard".into() }
    fn activate(&mut self, _x: i32, _y: i32) { let _ = self.tx.send(()); }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, StandardItem};
        let is_following = self.follow_cursor.load(Ordering::Relaxed);
        vec![
            StandardItem {
                label: "Переключить язык".into(),
                activate: Box::new(|this: &mut Self| { let _ = this.tx.send(()); }),
                ..Default::default()
            }.into(),
            CheckmarkItem {
                label: "Следовать за курсором".into(),
                checked: is_following,
                activate: Box::new(|this: &mut Self| {
                    let prev = this.follow_cursor.load(Ordering::Relaxed);
                    this.follow_cursor.store(!prev, Ordering::Relaxed);
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Выход".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }.into(),
        ]
    }
}

fn main() {
    unsafe { XInitThreads(); }

    let (tx, rx): (Sender<()>, Receiver<()>) = unbounded();
    let initial_lang = Lang::from_system().unwrap_or(Lang::En);
    let lang = Arc::new(Mutex::new(initial_lang));
    let follow_cursor = Arc::new(AtomicBool::new(false));
    let ctrl_held = Arc::new(AtomicBool::new(false));

    let lang_sync = Arc::clone(&lang);
    std::thread::spawn(move || {
        unsafe {
            let dpy = XOpenDisplay(std::ptr::null());
            if !dpy.is_null() {
                let mut state = XkbStateRec::default();
                loop {
                    if XkbGetState(dpy, 0x0100, &mut state) == 0 {
                        let sys_lang = if state.group == 1 { Lang::Ru } else { Lang::En };
                        if let Ok(mut l) = lang_sync.lock() {
                            if *l != sys_lang { *l = sys_lang; }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(80));
                }
            }
        }
    });

    let tx_keys = tx.clone();
    let ctrl_flag = Arc::clone(&ctrl_held);
    std::thread::spawn(move || {
        let mut last_alt = false;
        let mut last_shift = false;
        let mut last_ctrl = false;
        let _ = rdev::listen(move |ev| match ev.event_type {
            rdev::EventType::KeyPress(k) => match k {
                rdev::Key::Alt => last_alt = true,
                rdev::Key::ShiftLeft | rdev::Key::ShiftRight => last_shift = true,
                rdev::Key::ControlLeft | rdev::Key::ControlRight => {
                    last_ctrl = true;
                    ctrl_flag.store(true, Ordering::Relaxed);
                }
                rdev::Key::KeyL | rdev::Key::KeyR
                    if (last_alt && last_shift) || (last_ctrl && last_shift) =>
                {
                    let _ = tx_keys.send(());
                }
                _ => {}
            },
            rdev::EventType::KeyRelease(k) => match k {
                rdev::Key::Alt => last_alt = false,
                rdev::Key::ShiftLeft | rdev::Key::ShiftRight => last_shift = false,
                rdev::Key::ControlLeft | rdev::Key::ControlRight => {
                    last_ctrl = false;
                    ctrl_flag.store(false, Ordering::Relaxed);
                }
                _ => {}
            },
            _ => {}
        });
    });

    let tx_tray = tx.clone();
    let lang_tray = Arc::clone(&lang);
    let follow_tray = Arc::clone(&follow_cursor);
    std::thread::spawn(move || {
        let service = ksni::TrayService::new(Tray {
            tx: tx_tray,
            lang: lang_tray,
            follow_cursor: follow_tray,
        });
        let _ = service.run();
    });

    let c = cursor_pos();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size([48.0, 32.0])
            .with_position([c.x + 16.0, c.y + 16.0])
            .with_taskbar(false)
            .with_resizable(false),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "langflag",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, rx, lang, follow_cursor, ctrl_held)))),
    );
}
