use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::app::{Commands, Request};

const SHOW_ID: &str = "show";
const RESCAN_ID: &str = "rescan";

pub fn setup(commands: Commands) -> Option<TrayIcon> {
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
            SHOW_ID => Request::Show,
            RESCAN_ID => Request::Rescan,
            _ => return,
        };
        commands.send(cmd);
    }));

    match TrayIconBuilder::new()
        .with_icon(tray_icon())
        .with_icon_as_template(true)
        .with_tooltip("Orbit")
        .with_menu(Box::new(menu))
        .build()
    {
        Ok(tray) => Some(tray),
        Err(err) => {
            eprintln!("orbit: failed to create menu bar item: {err}");
            None
        }
    }
}

fn tray_icon() -> Icon {
    let png = include_bytes!("../assets/tray.png");
    let mut reader = png::Decoder::new(png.as_slice())
        .read_info()
        .expect("tray.png is a valid PNG");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("decode tray.png");
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).expect("valid tray icon")
}
