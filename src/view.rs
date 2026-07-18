use std::path::Path;

use eframe::egui;
use egui_taffy::taffy::prelude::{auto, length, percent};
use egui_taffy::{TaffyContainerUi, Tui, TuiBuilderLogic, taffy, tid, tui};

use crate::icons::IconCache;
use crate::launcher::Launcher;

const ROW_HEIGHT: f32 = 44.0;
const ROW_CORNER_RADIUS: f32 = 8.0;
const ROW_GAP: f32 = 2.0;
const ICON_SIZE: f32 = 32.0;
const ICON_LEFT_PAD: f32 = 10.0;
const ICON_TEXT_GAP: f32 = 12.0;
const QUERY_TEXT_PX: f32 = 26.0;
const ROW_TEXT_PX: f32 = 17.0;
const SEPARATOR_MARGIN_TOP: f32 = 11.0;
const SEPARATOR_MARGIN_BOTTOM: f32 = 7.0;

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
    // Any selection change means the selected row must be scrolled back
    // into view. The flag lives on self because input only exists in the
    // first pass of a frame, while a layout change discards that pass and
    // renders in a later one.
    scroll_pending: bool,
}

impl View {
    pub fn render(&mut self, ui: &mut egui::Ui, launcher: &mut Launcher) -> Option<Intent> {
        let ctx = ui.ctx().clone();

        // Keyboard input for one frame, consumed before any widget can claim it.
        let keys = Keys::read(&ctx);

        if keys.down {
            launcher.select_next();
            self.scroll_pending = true;
        }
        if keys.up {
            launcher.select_prev();
            self.scroll_pending = true;
        }

        let mut clicked_row = None;
        let panel = egui::Frame::new()
            .fill(PANEL_BG)
            .corner_radius(egui::CornerRadius::same(crate::PANEL_CORNER_RADIUS))
            .inner_margin(egui::Margin::same(14));

        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            tui(ui, ui.id().with("view"))
                .reserve_available_space()
                .style(column_style())
                .show(|tui| {
                    apply_base_style(tui);

                    if self.draw_query_box(tui, launcher) {
                        launcher.on_query_edited();
                        self.scroll_pending = true;
                    }

                    tui.mut_style(|style| {
                        style.margin.top = length(SEPARATOR_MARGIN_TOP);
                        style.margin.bottom = length(SEPARATOR_MARGIN_BOTTOM);
                    })
                    .separator();

                    match launcher.result_count() {
                        _ if launcher.is_scanning() => weak_label(tui, "Scanning applications…"),
                        0 => weak_label(tui, "No matching apps"),
                        count => self.draw_results(tui, &ctx, launcher, count, &mut clicked_row),
                    }
                });
        });

        // A scroll target set during a discarded pass is thrown away with it,
        // so only consume the flag once a pass survives.
        if !ctx.will_discard() {
            self.scroll_pending = false;
        }

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

    fn draw_query_box(&mut self, tui: &mut Tui, launcher: &mut Launcher) -> bool {
        let response = tui.ui_add(
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
        tui: &mut Tui,
        ctx: &egui::Context,
        launcher: &Launcher,
        count: usize,
        clicked_row: &mut Option<usize>,
    ) {
        tui.id("results").style(results_style()).add(|tui| {
            for row in 0..count {
                if self.draw_row(tui, ctx, launcher, row) {
                    *clicked_row = Some(row);
                }
            }
        });
    }

    fn draw_row(
        &mut self,
        tui: &mut Tui,
        ctx: &egui::Context,
        launcher: &Launcher,
        row: usize,
    ) -> bool {
        let Some(entry) = launcher.entry(row) else {
            return false;
        };
        let selected = row == launcher.selected();
        let scroll_pending = self.scroll_pending;
        let icons = &mut self.icons;

        tui.id(tid(("row", row)))
            .style(row_style())
            .add_with_background_ui(
                |ui, container| paint_row_bg(ui, container, selected),
                |tui, background| {
                    // Must be issued inside the scroll container's content; the
                    // surrounding ScrollArea only honours targets set here.
                    if selected && scroll_pending {
                        background.scroll_to_me(None);
                    }

                    draw_icon(tui, ctx, icons, &entry.path);

                    let color = if selected {
                        ROW_TEXT_SELECTED
                    } else {
                        ROW_TEXT
                    };
                    tui.mut_style(|style| style.min_size.width = length(0.0_f32))
                        .colored_label(color, egui::RichText::new(&entry.name).size(ROW_TEXT_PX));
                },
            )
            .background
            .clicked()
    }
}

fn column_style() -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Column,
        align_items: Some(taffy::AlignItems::Stretch),
        size: percent(1.0_f32),
        max_size: percent(1.0_f32),
        ..Default::default()
    }
}

fn results_style() -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Column,
        align_items: Some(taffy::AlignItems::Stretch),
        flex_grow: 1.0,
        gap: length(ROW_GAP),
        min_size: taffy::Size {
            width: auto(),
            height: length(0.0_f32),
        },
        overflow: taffy::Point {
            x: taffy::Overflow::Visible,
            y: taffy::Overflow::Scroll,
        },
        ..Default::default()
    }
}

fn row_style() -> taffy::Style {
    taffy::Style {
        flex_direction: taffy::FlexDirection::Row,
        align_items: Some(taffy::AlignItems::Center),
        gap: length(ICON_TEXT_GAP),
        padding: taffy::Rect {
            left: length(ICON_LEFT_PAD),
            ..taffy::Rect::zero()
        },
        size: taffy::Size {
            width: auto(),
            height: length(ROW_HEIGHT),
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}

fn icon_style() -> taffy::Style {
    taffy::Style {
        size: taffy::Size {
            width: length(ICON_SIZE),
            height: length(ICON_SIZE),
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}

fn paint_row_bg(ui: &mut egui::Ui, container: &TaffyContainerUi, selected: bool) -> egui::Response {
    let rect = container.full_container();
    let response = ui.interact(rect, ui.id().with("bg"), egui::Sense::click());
    if let Some(bg) = row_background(selected, response.hovered()) {
        ui.painter().rect_filled(rect, ROW_CORNER_RADIUS, bg);
    }
    response
}

fn draw_icon(tui: &mut Tui, ctx: &egui::Context, icons: &mut IconCache, path: &Path) {
    let icon = tui.style(icon_style());
    match icons.get(ctx, path) {
        Some(texture) => {
            icon.ui_add(egui::Image::from_texture((
                texture.id(),
                egui::Vec2::splat(ICON_SIZE),
            )));
        }
        None => {
            icon.ui(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(ICON_SIZE), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, ROW_CORNER_RADIUS - 2.0, ICON_PLACEHOLDER_BG);
            });
        }
    }
}

fn row_background(selected: bool, hovered: bool) -> Option<egui::Color32> {
    match (selected, hovered) {
        (true, _) => Some(ROW_SELECTED_BG),
        (false, true) => Some(ROW_HOVER_BG),
        (false, false) => None,
    }
}

fn apply_base_style(tui: &mut Tui) {
    let style = tui.egui_style_mut();
    style.wrap_mode = Some(egui::TextWrapMode::Extend);
    style.interaction.selectable_labels = false;
}

fn weak_label(tui: &mut Tui, text: &str) {
    tui.label(egui::RichText::new(text).weak());
}
