use eframe::egui;

use crate::catalog::{self, AppEntry};

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(24, 24, 27, 246);

pub struct OrbitApp {
    query: String,
    apps: Vec<AppEntry>,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self {
            query: String::new(),
            apps: catalog::scan(),
        }
    }
}

impl eframe::App for OrbitApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let panel = egui::Frame::new()
            .fill(PANEL_BG)
            .corner_radius(egui::CornerRadius::same(14))
            .inner_margin(egui::Margin::same(14));
        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .font(egui::FontId::proportional(26.0))
                    .hint_text("Search apps…")
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE),
            );
            response.request_focus();

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for app in &self.apps {
                        ui.label(&app.name);
                    }
                });
        });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}
