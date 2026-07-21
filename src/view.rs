use eframe::egui;

use crate::icons::IconCache;
use crate::session::Session;

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(24, 24, 27, 246);
const QUERY_TEXT_PX: f32 = 26.0;
const ROW_HEIGHT: f32 = 44.0;
const ROW_CORNER_RADIUS: f32 = 8.0;
const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(64, 64, 84);
const ROW_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);
const ICON_LEFT_PADDING: f32 = 10.0;
const ICON_SIZE: f32 = 32.0;
const ICON_PLACEHOLDER_BG: egui::Color32 = egui::Color32::from_gray(70);
const ROW_TEXT_SELECTED: egui::Color32 = egui::Color32::WHITE;
const ROW_TEXT: egui::Color32 = egui::Color32::from_gray(215);
const ICON_TEXT_GAP: f32 = 12.0;
const ROW_TEXT_PX: f32 = 17.0;

pub enum Action {
    Launch(usize),
    Dismiss,
}

pub struct View {
    icons: IconCache,
    focus_pending: bool,
    scroll_pending: bool,
}

impl View {
    pub fn render(&mut self, ui: &mut egui::Ui, session: &mut Session) -> Option<Action> {
        let ctx = ui.ctx().clone();

        let (up, down, enter, escape) = ctx.input_mut(|i| {
            let mut hit = |key| i.consume_key(egui::Modifiers::NONE, key);
            (
                hit(egui::Key::ArrowUp),
                hit(egui::Key::ArrowDown),
                hit(egui::Key::Enter),
                hit(egui::Key::Escape),
            )
        });

        if down {
            session.select_next();
        }
        if up {
            session.select_prev();
        }
        self.scroll_pending |= up || down;

        let mut clicked_row = None;
        let panel = egui::Frame::new()
            .fill(PANEL_BG)
            .corner_radius(egui::CornerRadius::same(14))
            .inner_margin(egui::Margin::same(14));

        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            if self.draw_query_box(ui, session) {
                session.query_changed();
                self.scroll_pending = true;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            if session.is_scanning() {
                ui.label(egui::RichText::new("Scanning applications...").weak());
            } else if session.result_count() == 0 {
                ui.label(egui::RichText::new("No matching apps").weak());
            } else {
                ui.spacing_mut().item_spacing.y = 2.0;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for row in 0..session.result_count() {
                            if self.draw_row(ui, session, row) {
                                clicked_row = Some(row);
                            }
                        }
                    });
            }
        });

        self.scroll_pending = false;

        let launch = clicked_row.or(enter.then(|| session.selected()));
        launch
            .map(Action::Launch)
            .or(escape.then_some(Action::Dismiss))
    }

    fn draw_query_box(&mut self, ui: &mut egui::Ui, session: &mut Session) -> bool {
        let response = ui.add(
            egui::TextEdit::singleline(&mut session.query)
                .font(egui::FontId::proportional(QUERY_TEXT_PX))
                .hint_text("Search apps...")
                .desired_width(f32::INFINITY)
                .frame(egui::Frame::NONE),
        );
        if std::mem::take(&mut self.focus_pending) {
            response.request_focus();
        }
        response.changed()
    }

    fn draw_row(&mut self, ui: &mut egui::Ui, session: &Session, row: usize) -> bool {
        let Some(entry) = session.entry(row) else {
            return false;
        };
        let size = egui::vec2(ui.available_width(), ROW_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let selected = row == session.selected();
        if selected && self.scroll_pending {
            ui.scroll_to_rect(rect, None);
        }
        if !ui.is_rect_visible(rect) {
            return false;
        }

        let painter = ui.painter();
        if selected {
            painter.rect_filled(rect, ROW_CORNER_RADIUS, ROW_SELECTED_BG);
        } else if response.hovered() {
            painter.rect_filled(rect, ROW_CORNER_RADIUS, ROW_HOVER_BG);
        }

        let icon = egui::Rect::from_min_size(
            rect.left_center() + egui::vec2(ICON_LEFT_PADDING, -ICON_SIZE / 2.0),
            egui::Vec2::splat(ICON_SIZE),
        );
        match self.icons.get_or_load(ui.ctx(), &entry.path) {
            Some(texture) => egui::Image::new((texture.id(), icon.size())).paint_at(ui, icon),
            None => {
                painter.rect_filled(icon, ROW_CORNER_RADIUS - 2.0, ICON_PLACEHOLDER_BG);
            }
        }

        let color = if selected {
            ROW_TEXT_SELECTED
        } else {
            ROW_TEXT
        };
        let pos = icon.right_center() + egui::vec2(ICON_TEXT_GAP, 0.0);
        let font = egui::FontId::proportional(ROW_TEXT_PX);
        painter.text(pos, egui::Align2::LEFT_CENTER, &entry.name, font, color);
        response.clicked()
    }

    pub fn focus_query(&mut self) {
        self.focus_pending = true;
    }
}

impl Default for View {
    fn default() -> Self {
        Self {
            icons: IconCache::default(),
            focus_pending: true,
            scroll_pending: false,
        }
    }
}
