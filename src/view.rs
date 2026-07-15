use eframe::egui;

use crate::icons::IconCache;
use crate::launcher::Launcher;

const ROW_HEIGHT: f32 = 44.0;
const ROW_CORNER_RADIUS: f32 = 8.0;
const ICON_SIZE: f32 = 32.0;
const ICON_LEFT_PAD: f32 = 10.0;
const ICON_TEXT_GAP: f32 = 12.0;
const QUERY_TEXT_PX: f32 = 26.0;
const ROW_TEXT_PX: f32 = 17.0;

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(17, 17, 19, 180);
const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(64, 64, 84);
const ROW_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);
const ICON_PLACEHOLDER_BG: egui::Color32 = egui::Color32::from_gray(70);
const ROW_TEXT: egui::Color32 = egui::Color32::from_gray(215);
const ROW_TEXT_SELECTED: egui::Color32 = egui::Color32::WHITE;

pub enum Intent {
    Launch(usize),
    Dismiss,
}

#[derive(Default)]
struct Keys {
    up: bool,
    down: bool,
    enter: bool,
    escape: bool,
}

impl Keys {
    fn read(ctx: &egui::Context) -> Self {
        ctx.input_mut(|i| {
            let mut hit = |key| i.consume_key(egui::Modifiers::NONE, key);
            Keys {
                up: hit(egui::Key::ArrowUp),
                down: hit(egui::Key::ArrowDown),
                enter: hit(egui::Key::Enter),
                escape: hit(egui::Key::Escape),
            }
        })
    }
}

#[derive(Default)]
pub struct View {
    icons: IconCache,
    focus_requested: bool,
}

impl View {
    pub fn render(&mut self, ui: &mut egui::Ui, launcher: &mut Launcher) -> Option<Intent> {
        let ctx = ui.ctx().clone();

        // Keyboard input for one frame, consumed before any widget can claim it.
        let keys = Keys::read(&ctx);

        // Any selection change this frame means the selected row must be
        // scrolled back into view.
        let mut moved = false;
        if keys.down {
            launcher.select_next();
            moved = true;
        }
        if keys.up {
            launcher.select_prev();
            moved = true;
        }

        let mut clicked_row = None;
        let panel = egui::Frame::new()
            .fill(PANEL_BG)
            .corner_radius(egui::CornerRadius::same(crate::PANEL_CORNER_RADIUS))
            .inner_margin(egui::Margin::same(14));

        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            if self.draw_query_box(ui, launcher) {
                launcher.on_query_edited();
                moved = true;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            match launcher.result_count() {
                _ if launcher.is_scanning() => weak_label(ui, "Scanning applications…"),
                0 => weak_label(ui, "No matching apps"),
                count => self.draw_results(ui, &ctx, launcher, count, moved, &mut clicked_row),
            }
        });

        match () {
            _ if clicked_row.is_some() => clicked_row.map(Intent::Launch),
            _ if keys.enter => Some(Intent::Launch(launcher.selected())),
            _ if keys.escape => Some(Intent::Dismiss),
            _ => None,
        }
    }

    pub fn focus_query(&mut self) {
        self.focus_requested = false;
    }

    fn draw_query_box(&mut self, ui: &mut egui::Ui, launcher: &mut Launcher) -> bool {
        let response = ui.add(
            egui::TextEdit::singleline(&mut launcher.query)
                .font(egui::FontId::proportional(QUERY_TEXT_PX))
                .hint_text("Search apps…")
                .desired_width(f32::INFINITY)
                .frame(egui::Frame::NONE),
        );
        // Focus the query box on the first frame after a show.
        if !self.focus_requested {
            if !response.has_focus() {
                response.request_focus();
            }
            self.focus_requested = true;
        }
        response.changed()
    }

    fn draw_results(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        launcher: &Launcher,
        count: usize,
        moved: bool,
        clicked_row: &mut Option<usize>,
    ) {
        ui.spacing_mut().item_spacing.y = 2.0;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, ROW_HEIGHT, count, |ui, range| {
                for row in range {
                    if self.draw_row(ui, ctx, launcher, row, moved) {
                        *clicked_row = Some(row);
                    }
                }
            });
    }

    fn draw_row(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        launcher: &Launcher,
        row: usize,
        moved: bool,
    ) -> bool {
        let Some(entry) = launcher.entry(row) else {
            return false;
        };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ROW_HEIGHT),
            egui::Sense::click(),
        );
        let selected = row == launcher.selected();

        if let Some(bg) = row_background(selected, response.hovered()) {
            ui.painter().rect_filled(rect, ROW_CORNER_RADIUS, bg);
        }
        if selected && moved {
            ui.scroll_to_rect(rect, None);
        }

        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(
                rect.left() + ICON_LEFT_PAD + ICON_SIZE / 2.0,
                rect.center().y,
            ),
            egui::Vec2::splat(ICON_SIZE),
        );
        self.draw_icon(ui, ctx, &entry.path, icon_rect);

        ui.painter().text(
            egui::pos2(icon_rect.right() + ICON_TEXT_GAP, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            egui::FontId::proportional(ROW_TEXT_PX),
            if selected {
                ROW_TEXT_SELECTED
            } else {
                ROW_TEXT
            },
        );

        response.clicked()
    }

    fn draw_icon(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        path: &std::path::Path,
        rect: egui::Rect,
    ) {
        match self.icons.get(ctx, path) {
            Some(texture) => ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            ),
            None => ui
                .painter()
                .rect_filled(rect, ROW_CORNER_RADIUS - 2.0, ICON_PLACEHOLDER_BG),
        };
    }
}

fn row_background(selected: bool, hovered: bool) -> Option<egui::Color32> {
    match (selected, hovered) {
        (true, _) => Some(ROW_SELECTED_BG),
        (false, true) => Some(ROW_HOVER_BG),
        (false, false) => None,
    }
}

fn weak_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).weak());
}
