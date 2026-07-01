use std::sync::atomic::Ordering;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::app::{Commands, IS_VISIBLE, Request};
use crate::macos;

/// Register Option+Space. The manager must live on the main thread and be
/// kept alive. The handler fires on the main run loop (Carbon), so it can
/// activate in-place — inside the user-event context macOS 14+ demands
/// before a background app may steal focus.
pub fn register_hotkey(commands: Commands) -> Option<GlobalHotKeyManager> {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("orbit: failed to create hotkey manager: {err}");
            return None;
        }
    };
    let hotkey = HotKey::new(Some(crate::HOTKEY_MODIFIERS), crate::HOTKEY_CODE);
    if let Err(err) = manager.register(hotkey) {
        eprintln!("orbit: failed to register Option+Space (already taken?): {err}");
        return None;
    }

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state() == HotKeyState::Pressed {
            if !IS_VISIBLE.load(Ordering::SeqCst) {
                // About to show: grab activation now (no-op if ever off-thread).
                macos::activate_app();
            }
            commands.send(Request::Toggle);
        }
    }));
    Some(manager)
}
