use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::app::{Command, CommandSender};

pub fn register(sender: CommandSender) -> Option<GlobalHotKeyManager> {
    let manager = GlobalHotKeyManager::new()
        .inspect_err(|err| eprintln!("orbit: failed to create hotkey manager: {err}"))
        .ok()?;
    let hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
    if let Err(err) = manager.register(hotkey) {
        eprintln!("orbit: failed to register Option+Space (already taken?): {err}");
        return None;
    }

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state() == HotKeyState::Pressed {
            sender.send(Command::Toggle);
        }
    }));
    Some(manager)
}
