use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

pub static TOGGLE_REQUESTED: AtomicUsize = AtomicUsize::new(0);

pub fn register(ctx: &egui::Context) -> Option<GlobalHotKeyManager> {
    let manager = GlobalHotKeyManager::new()
        .inspect_err(|err| eprintln!("orbit: failed to create hotkey manager: {err}"))
        .ok()?;
    let hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
    if let Err(err) = manager.register(hotkey) {
        eprintln!("orbit: failed to register Option+Space (already taken?): {err}");
        return None;
    }

    let ctx = ctx.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state() == HotKeyState::Pressed {
            TOGGLE_REQUESTED.fetch_add(1, Ordering::SeqCst);
            ctx.request_repaint();
        }
    }));
    Some(manager)
}
