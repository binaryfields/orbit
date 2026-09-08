use eframe::egui;

mod app;
mod catalog;
mod hotkey;
mod icons;
mod macos;
mod scanner;
mod session;
mod tray;
mod view;
mod window;

pub const WINDOW_SIZE: egui::Vec2 = egui::Vec2 { x: 640.0, y: 420.0 };
pub const PANEL_CORNER_RADIUS: u8 = 14;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false),
        event_loop_builder: Some(Box::new(macos::set_accessory_activation_policy)),
        ..Default::default()
    };
    eframe::run_native(
        "Orbit",
        options,
        Box::new(|cc| Ok(Box::new(app::OrbitApp::new(cc)))),
    )
}
