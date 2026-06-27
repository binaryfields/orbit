mod app;
mod catalog;

use eframe::egui;

pub const WINDOW_SIZE: egui::Vec2 = egui::Vec2 { x: 640.0, y: 420.0 };

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_SIZE)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Orbit",
        options,
        Box::new(|cc| Ok(Box::new(app::OrbitApp::new(cc)))),
    )
}
