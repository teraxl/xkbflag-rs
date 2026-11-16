#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod lang;
mod platform;

use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use lang::Lang;
use raw_window_handle::HasWindowHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const RU_SVG: &[u8] = include_bytes!("../assets/flags/ru.svg");
const US_SVG: &[u8] = include_bytes!("../assets/flags/us.svg");
const FLAG_ASPECT: f32 = 96.0 / 64.0;

fn load_flag_svg(ctx: &egui::Context, name: &str, bytes: &[u8], px: u32) -> egui::TextureHandle {
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
    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture(name.to_string(), img, egui::TextureOptions::LINEAR)
}

struct App {
    lang: Arc<Mutex<Lang>>,
    follow: Arc<AtomicBool>,
    ctrl_held: Arc<AtomicBool>,
    size: f32,
    last_size: f32,
    rx: Receiver<()>,
    win_id: Option<u64>,
    configured: bool,
    dragging: bool,
    drag_off: egui::Vec2,
    tex_ru: egui::TextureHandle,
    tex_us: egui::TextureHandle,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        rx: Receiver<()>,
        lang: Arc<Mutex<Lang>>,
        follow: Arc<AtomicBool>,
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
            follow,
            ctrl_held,
            size: 48.0,
            last_size: 48.0,
            rx,
            win_id: None,
            configured: false,
            dragging: false,
            drag_off: egui::vec2(0.0, 0.0),
            tex_ru,
            tex_us,
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.configured {
            if let Ok(h) = frame.window_handle() {
                self.win_id = platform::window_id_from_raw(&h.as_raw());
                if let Some(id) = self.win_id {
                    platform::set_skip_taskbar(id);
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            self.configured = true;
        }

        let follow_enabled = self.follow.load(Ordering::Relaxed);
        let ctrl = platform::is_ctrl_pressed() || self.ctrl_held.load(Ordering::Relaxed);
        let following = follow_enabled && !ctrl;

        if self.dragging {
            if !platform::is_left_button_pressed() {
                self.dragging = false;
                if let Some(id) = self.win_id {
                    platform::focus_under(id);
                }
            } else {
                #[cfg(windows)]
                {
                    let c = platform::cursor_pos();
                    if let Some(id) = self.win_id {
                        platform::move_window(
                            id,
                            (c.x - self.drag_off.x) as i32,
                            (c.y - self.drag_off.y) as i32,
                        );
                    }
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        while self.rx.try_recv().is_ok() {
            if let Ok(mut l) = self.lang.lock() {
                l.toggle();
                platform::set_lang(*l);
            }
        }

        if (self.size - self.last_size).abs() > f32::EPSILON {
            let w = self.size;
            let h = self.size / FLAG_ASPECT;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
            self.last_size = self.size;
        }

        let current_lang = *self.lang.lock().unwrap();
        let tex = if current_lang == Lang::Ru {
            &self.tex_ru
        } else {
            &self.tex_us
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let avail = ui.available_size();
                let (rect, resp) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());

                let draw = if rect.width() / rect.height() > FLAG_ASPECT {
                    let w = rect.height() * FLAG_ASPECT;
                    egui::Rect::from_center_size(rect.center(), egui::vec2(w, rect.height()))
                } else {
                    let h = rect.width() / FLAG_ASPECT;
                    egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), h))
                };

                ui.painter().image(
                    tex.id(),
                    draw,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                ui.painter().rect_stroke(
                    draw,
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
                    self.dragging = true;
                    if let Some(id) = self.win_id {
                        platform::focus_self(id);
                    }
                    #[cfg(target_os = "linux")]
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    #[cfg(windows)]
                    {
                        let c = platform::cursor_pos();
                        let outer_min = ctx.input(|i| {
                            i.viewport()
                                .outer_rect
                                .map(|r| r.min)
                                .unwrap_or(egui::Pos2::ZERO)
                        });
                        self.drag_off = c - outer_min;
                    }
                }
                if !following && resp.clicked() {
                    if let Ok(mut l) = self.lang.lock() {
                        l.toggle();
                        platform::set_lang(*l);
                    }
                    if let Some(id) = self.win_id {
                        platform::focus_under(id);
                    }
                }
            });

        if following {
            if let Some(id) = self.win_id {
                let c = platform::cursor_pos();
                let (sx, sy, sw, sh) = platform::screen_size();
                let fw = self.size;
                let fh = self.size / FLAG_ASPECT;
                let mut x = c.x + 50.0;
                let mut y = c.y + 50.0;
                let min_x = sx as f32 + 4.0;
                let min_y = sy as f32 + 4.0;
                let max_x = (sx + sw) as f32 - fw - 4.0;
                let max_y = (sy + sh) as f32 - fh - 4.0;
                if x > max_x {
                    x = max_x;
                }
                if y > max_y {
                    y = max_y;
                }
                if x < min_x {
                    x = min_x;
                }
                if y < min_y {
                    y = min_y;
                }
                platform::move_window(id, x as i32, y as i32);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

fn main() {
    if !platform::try_single_instance() {
        std::process::exit(0);
    }

    platform::init();

    let (tx, rx): (Sender<()>, Receiver<()>) = unbounded();
    let lang = Arc::new(Mutex::new(platform::get_lang().unwrap_or(Lang::En)));
    let follow = Arc::new(AtomicBool::new(false));
    let ctrl_held = Arc::new(AtomicBool::new(false));

    let lang_sync = Arc::clone(&lang);
    std::thread::spawn(move || loop {
        if let Some(sys) = platform::get_lang() {
            if let Ok(mut l) = lang_sync.lock() {
                if *l != sys {
                    *l = sys;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
    });

    #[cfg(target_os = "linux")]
    {
        let ctrl_flag = Arc::clone(&ctrl_held);
        let lang_keys = Arc::clone(&lang);
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
                        if let Ok(mut l) = lang_keys.lock() {
                            l.toggle();
                            platform::set_lang(*l);
                        }
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
    }

    let tx_tray = tx.clone();
    let lang_tray = Arc::clone(&lang);
    let follow_tray = Arc::clone(&follow);
    let cb = platform::TrayCallbacks {
        toggle_lang: Box::new(move || {
            let _ = tx_tray.send(());
        }),
        toggle_follow: {
            let f = Arc::clone(&follow_tray);
            Box::new(move || {
                let p = f.load(Ordering::Relaxed);
                f.store(!p, Ordering::Relaxed);
            })
        },
        is_following: {
            let f = Arc::clone(&follow_tray);
            Box::new(move || f.load(Ordering::Relaxed))
        },
        quit: Box::new(|| std::process::exit(0)),
        title: {
            let l = Arc::clone(&lang_tray);
            Box::new(move || match *l.lock().unwrap() {
                Lang::Ru => "Раскладка: RU".into(),
                Lang::En => "Раскладка: EN".into(),
            })
        },
    };
    platform::spawn_tray(cb);

    let c = platform::cursor_pos();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_inner_size([48.0, 32.0])
            .with_position([c.x + 16.0, c.y + 16.0])
            .with_taskbar(false)
            .with_resizable(false),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "xkbflag-rs",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, rx, lang, follow, ctrl_held)))),
    );
}
