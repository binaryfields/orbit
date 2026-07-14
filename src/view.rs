use eframe::egui;

use crate::icons::IconCache;
use crate::launcher::Launcher;

const ROW_HEIGHT: f32 = 44.0;
const ICON_SIZE: f32 = 32.0;

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(17, 17, 19, 180);
const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(64, 64, 84);
const ROW_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);

pub enum Intent {
    Launch(usize),
    Dismiss,
}

#[derive(Default)]
pub struct View {
    icons: IconCache,
}

impl View {
    pub fn render(&mut self, ui: &mut egui::Ui, launcher: &mut Launcher) -> Option<Intent> {
        let ctx = ui.ctx().clone();

        let mut selection_moved = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                launcher.select_next();
                selection_moved = true;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                launcher.select_prev();
                selection_moved = true;
            }
        });
        let pressed_enter =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let pressed_escape =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

        let panel = egui::Frame::new()
            .fill(PANEL_BG)
            .corner_radius(egui::CornerRadius::same(crate::PANEL_CORNER_RADIUS))
            .inner_margin(egui::Margin::same(14));
        let mut clicked_row = None;

        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut launcher.query)
                    .font(egui::FontId::proportional(26.0))
                    .hint_text("Search apps…")
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE),
            );
            if response.changed() {
                launcher.on_query_edited();
                selection_moved = true;
            }
            response.request_focus();

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            if launcher.result_count() == 0 {
                ui.label(egui::RichText::new("No matching apps").weak());
                return;
            }

            ui.spacing_mut().item_spacing.y = 2.0;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(ui, ROW_HEIGHT, launcher.result_count(), |ui, range| {
                    for row in range {
                        if self.draw_result_row(ui, &ctx, launcher, row, selection_moved) {
                            clicked_row = Some(row);
                        }
                    }
                });
        });

        if let Some(row) = clicked_row {
            Some(Intent::Launch(row))
        } else if pressed_enter {
            Some(Intent::Launch(launcher.selected()))
        } else if pressed_escape {
            Some(Intent::Dismiss)
        } else {
            None
        }
    }

    fn draw_result_row(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        launcher: &Launcher,
        row: usize,
        selection_moved: bool,
    ) -> bool {
        let Some(entry) = launcher.entry(row) else {
            return false;
        };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ROW_HEIGHT),
            egui::Sense::click(),
        );
        let is_selected = row == launcher.selected();

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
        match self.icons.get(ctx, &entry.path) {
            Some(texture) => {
                ui.painter().image(
                    texture.id(),
                    icon_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => {
                ui.painter()
                    .rect_filled(icon_rect, 6.0, egui::Color32::from_gray(70));
            }
        }

        ui.painter().text(
            egui::pos2(icon_rect.right() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            egui::FontId::proportional(17.0),
            if is_selected {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_gray(215)
            },
        );

        response.clicked()
    }
}
