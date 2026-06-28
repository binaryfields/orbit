use std::process::Command;

use eframe::egui;

use crate::catalog::{self, AppEntry};
use crate::search::Search;

const ROW_HEIGHT: f32 = 44.0;
const ICON_SIZE: f32 = 32.0;

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(24, 24, 27, 246);
const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(64, 64, 84);
const ROW_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);

pub struct OrbitApp {
    query: String,
    apps: Vec<AppEntry>,
    search: Search,
    results: Vec<usize>,
    selected: usize,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let apps = catalog::scan();
        let mut search = Search::default();
        let results = search.filter("", &apps);
        Self {
            query: String::new(),
            apps,
            search,
            results,
            selected: 0,
        }
    }

    fn refilter(&mut self) {
        self.results = self.search.filter(&self.query, &self.apps);
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }

    fn launch(&self, result_row: usize) {
        let Some(&app_idx) = self.results.get(result_row) else {
            return;
        };
        let path = &self.apps[app_idx].path;
        if let Err(err) = Command::new("open").arg(path).spawn() {
            eprintln!("orbit: failed to launch {}: {err}", path.display());
        }
    }
}

impl eframe::App for OrbitApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        let mut selection_moved = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                && self.selected + 1 < self.results.len()
            {
                self.selected += 1;
                selection_moved = true;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) && self.selected > 0 {
                self.selected -= 1;
                selection_moved = true;
            }
        });

        let pressed_enter =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));

        let panel = egui::Frame::new()
            .fill(PANEL_BG)
            .corner_radius(egui::CornerRadius::same(14))
            .inner_margin(egui::Margin::same(14));
        let mut clicked_row = None;

        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .font(egui::FontId::proportional(26.0))
                    .hint_text("Search apps…")
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE),
            );
            if response.changed() {
                self.selected = 0;
                self.refilter();
                selection_moved = true;
            }
            response.request_focus();

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            if self.results.is_empty() {
                ui.label(egui::RichText::new("No matching apps").weak());
                return;
            }

            ui.spacing_mut().item_spacing.y = 2.0;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(ui, ROW_HEIGHT, self.results.len(), |ui, range| {
                    for row in range {
                        let app_idx = self.results[row];
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), ROW_HEIGHT),
                            egui::Sense::click(),
                        );
                        let is_selected = row == self.selected;

                        if is_selected {
                            ui.painter().rect_filled(rect, 8.0, ROW_SELECTED_BG);
                            if selection_moved {
                                ui.scroll_to_rect(rect, None);
                            }
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 8.0, ROW_HOVER_BG);
                        }

                        let icon_rect = egui::Rect::from_center_size(
                            egui::pos2(rect.left() + 10.0 + ICON_SIZE / 2.0, rect.center().y),
                            egui::vec2(ICON_SIZE, ICON_SIZE),
                        );
                        ui.painter()
                            .rect_filled(icon_rect, 6.0, egui::Color32::from_gray(70));

                        ui.painter().text(
                            egui::pos2(icon_rect.right() + 12.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &self.apps[app_idx].name,
                            egui::FontId::proportional(17.0),
                            if is_selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_gray(215)
                            },
                        );

                        if response.clicked() {
                            clicked_row = Some(row);
                        }
                    }
                });
        });

        if let Some(row) = clicked_row {
            self.launch(row);
        } else if pressed_enter {
            self.launch(self.selected);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}
