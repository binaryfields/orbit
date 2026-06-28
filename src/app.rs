use std::cmp::Reverse;
use std::process::Command;

use eframe::egui;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::catalog::{self, AppEntry};

const ROW_HEIGHT: f32 = 44.0;
const ICON_SIZE: f32 = 32.0;

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(24, 24, 27, 246);
const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(64, 64, 84);
const ROW_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);

pub struct OrbitApp {
    query: String,
    apps: Vec<AppEntry>,
    matcher: Matcher,
    results: Vec<usize>,
    selected: usize,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.text_cursor.blink = false;
        cc.egui_ctx.set_visuals(visuals);
        let apps = catalog::scan();
        let mut matcher = Matcher::default();
        let results = Self::rank(&mut matcher, "", &apps);
        Self {
            query: String::new(),
            apps,
            matcher,
            results,
            selected: 0,
        }
    }

    fn update_results(&mut self) {
        self.results = Self::rank(&mut self.matcher, &self.query, &self.apps);
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

    fn rank(matcher: &mut Matcher, query: &str, apps: &[AppEntry]) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..apps.len()).collect();
        }

        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut buf = Vec::new();
        let mut hits: Vec<(u32, usize)> = apps
            .iter()
            .enumerate()
            .filter_map(|(i, app)| {
                pattern
                    .score(Utf32Str::new(&app.name, &mut buf), matcher)
                    .map(|score| (score, i))
            })
            .collect();

        hits.sort_by_key(|&(score, i)| (Reverse(score), &apps[i].name));
        hits.into_iter().map(|(_, i)| i).collect()
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
                    .hint_text("Search apps...")
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE),
            );
            if response.changed() {
                self.selected = 0;
                self.update_results();
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
                .show(ui, |ui| {
                    for row in 0..self.results.len() {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn apps(names: &[&str]) -> Vec<AppEntry> {
        names
            .iter()
            .map(|name| AppEntry {
                name: (*name).to_owned(),
                path: PathBuf::from(format!("/Applications/{name}.app")),
            })
            .collect()
    }

    fn rank(query: &str, apps: &[AppEntry]) -> Vec<usize> {
        OrbitApp::rank(&mut Matcher::default(), query, apps)
    }

    #[test]
    fn rank_blank_query() {
        let apps = apps(&["Books", "iTerm", "Xcode"]);
        assert_eq!(rank("   ", &apps), vec![0, 1, 2]);
    }

    #[test]
    fn rank_differing_scores() {
        let apps = apps(&["Chess", "System Settings"]);
        assert_eq!(rank("ss", &apps), vec![1, 0]);
    }

    #[test]
    fn rank_tied_scores() {
        let apps = apps(&["Sonoma", "Sierra"]);
        assert_eq!(rank("s", &apps), vec![1, 0]);
    }

    #[test]
    fn rank_unmatched_entries() {
        let apps = apps(&["Safari", "Notes", "Mail"]);
        assert_eq!(rank("saf", &apps), vec![0]);
    }
}
