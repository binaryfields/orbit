use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::app::{Command, CommandSender};

const SHOW_ID: &str = "show";
const RESCAN_ID: &str = "rescan";

pub fn build(sender: CommandSender) -> Option<TrayIcon> {
    let menu = Menu::new();
    let show = MenuItem::with_id(SHOW_ID, "Show Orbit", true, None);
    let rescan = MenuItem::with_id(RESCAN_ID, "Rescan Applications", true, None);
    let quit = PredefinedMenuItem::quit(Some("Quit Orbit"));
    if let Err(err) = menu.append_items(&[&show, &rescan, &PredefinedMenuItem::separator(), &quit])
    {
        eprintln!("orbit: failed to build tray menu: {err}");
        return None;
    }

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let cmd = match event.id.0.as_str() {
            SHOW_ID => Command::Show,
            RESCAN_ID => Command::Rescan,
            _ => return,
        };
        sender.send(cmd);
    }));

    let tray = TrayIconBuilder::new()
        .with_icon(icon())
        .with_icon_as_template(true)
        .with_tooltip("Orbit")
        .with_menu(Box::new(menu))
        .build()
        .inspect_err(|err| eprintln!("orbit: failed to create menu bar item: {err}"))
        .ok()?;
    Some(tray)
}

fn icon() -> Icon {
    let png = eframe::icon_data::from_png_bytes(include_bytes!("../assets/tray.png"))
        .expect("tray.png is a valid PNG");
    Icon::from_rgba(png.rgba, png.width, png.height).expect("valid tray icon")
}
