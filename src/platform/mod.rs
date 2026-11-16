use crate::lang::Lang;
use eframe::egui;

#[cfg(target_os = "linux")]
pub fn init() {
    imp::init()
}

#[cfg(windows)]
pub fn init() {}

pub fn get_lang() -> Option<Lang> {
    imp::get_lang()
}
pub fn set_lang(lang: Lang) {
    imp::set_lang(lang)
}
pub fn cursor_pos() -> egui::Pos2 {
    imp::cursor_pos()
}
pub fn move_window(win_id: u64, x: i32, y: i32) {
    imp::move_window(win_id, x, y)
}
pub fn is_left_button_pressed() -> bool {
    imp::is_left_button_pressed()
}
pub fn is_ctrl_pressed() -> bool {
    imp::is_ctrl_pressed()
}
pub fn set_skip_taskbar(win_id: u64) {
    imp::set_skip_taskbar(win_id)
}
pub fn focus_self(win_id: u64) {
    imp::focus_self(win_id)
}
pub fn focus_under(win_id: u64) {
    imp::focus_under(win_id)
}
pub fn window_id_from_raw(handle: &raw_window_handle::RawWindowHandle) -> Option<u64> {
    imp::window_id_from_raw(handle)
}

pub fn screen_size() -> (i32, i32, i32, i32) {
    imp::screen_size()
}

pub struct TrayCallbacks {
    pub toggle_lang: Box<dyn Fn() + Send + Sync>,
    pub toggle_follow: Box<dyn Fn() + Send + Sync>,
    pub is_following: Box<dyn Fn() -> bool + Send + Sync>,
    pub quit: Box<dyn Fn() + Send + Sync>,
    pub title: Box<dyn Fn() -> String + Send + Sync>,
}

pub fn spawn_tray(cb: TrayCallbacks) {
    imp::spawn_tray(cb)
}

pub fn try_single_instance() -> bool {
    imp::try_single_instance()
}

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

#[cfg(windows)]
#[path = "windows.rs"]
mod imp;
