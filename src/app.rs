use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use global_hotkey::GlobalHotKeyManager;
use tray_icon::TrayIcon;
use tray_icon::menu::CheckMenuItem;

use crate::catalog::{self, AppEntry};
use crate::hotkey::register_hotkey;
use crate::icons::IconCache;
use crate::search::Search;
use crate::{macos, tray};

pub(crate) static IS_VISIBLE: AtomicBool = AtomicBool::new(true);

pub enum Request {
    Toggle,
    Show,
    Rescan,
    ToggleLogin,
}

#[derive(Clone)]
pub struct Commands {
    tx: Sender<Request>,
    ctx: egui::Context,
}

impl Commands {
    pub fn send(&self, request: Request) {
        let _ = self.tx.send(request);
        self.ctx.request_repaint();
    }
}

const ROW_HEIGHT: f32 = 44.0;
const ICON_SIZE: f32 = 32.0;

const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(24, 24, 27, 246);
const ROW_SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(64, 64, 84);
const ROW_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);

pub struct OrbitApp {
    _hotkey: Option<GlobalHotKeyManager>,
    _tray: Option<TrayIcon>,
    login_item: Option<CheckMenuItem>,
    cmd_rx: Receiver<Request>,
    apps: Vec<AppEntry>,
    search: Search,
    icons: IconCache,
    query: String,
    results: Vec<usize>,
    selected: usize,
    visible: bool,
    frames_since_show: u32,
    pending_activate: bool,
    positioned: bool,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let apps = catalog::scan();
        let mut search = Search::default();
        let results = search.filter("", &apps);

        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let commands = Commands {
            tx: cmd_tx,
            ctx: cc.egui_ctx.clone(),
        };
        let (tray, login_item) = tray::setup(commands.clone()).unzip();

        Self {
            _hotkey: register_hotkey(commands),
            _tray: tray,
            login_item,
            cmd_rx,
            apps,
            search,
            icons: IconCache::default(),
            query: String::new(),
            results,
            selected: 0,
            visible: true,
            frames_since_show: 0,
            pending_activate: true,
            positioned: false,
        }
    }

    fn refilter(&mut self) {
        self.results = self.search.filter(&self.query, &self.apps);
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.visible = true;
        IS_VISIBLE.store(true, Ordering::SeqCst);
        self.frames_since_show = 0;
        self.query.clear();
        self.selected = 0;
        self.refilter();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.position(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.pending_activate = true;
        ctx.request_repaint();
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.visible = false;
        IS_VISIBLE.store(false, Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        macos::hide_app();
    }

    fn position(&self, ctx: &egui::Context) -> bool {
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return false;
        };
        if monitor.x <= 0.0 || monitor.y <= 0.0 {
            return false;
        }
        let pos = egui::pos2((monitor.x - crate::WINDOW_SIZE.x) / 2.0, monitor.y * 0.18);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        true
    }

    fn launch(&mut self, ctx: &egui::Context, result_row: usize) {
        let Some(&app_idx) = self.results.get(result_row) else {
            return;
        };
        let path = &self.apps[app_idx].path;
        if let Err(err) = Command::new("open").arg(path).spawn() {
            eprintln!("orbit: failed to launch {}: {err}", path.display());
        }
        self.hide(ctx);
    }

    fn update_activation(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }
        if self.pending_activate && self.frames_since_show >= 1 {
            self.pending_activate = false;
            macos::activate_app();
        }
        if self.frames_since_show > 4 && ctx.input(|i| i.viewport().focused) == Some(false) {
            self.hide(ctx);
        } else {
            self.frames_since_show = self.frames_since_show.saturating_add(1);
            if self.frames_since_show <= 5 {
                ctx.request_repaint();
            }
        }
    }

    fn draw_result_row(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        row: usize,
        selection_moved: bool,
    ) -> bool {
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
        match self.icons.get(ctx, &self.apps[app_idx].path) {
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
            &self.apps[app_idx].name,
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

impl eframe::App for OrbitApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                Request::Toggle => {
                    if self.visible {
                        self.hide(ctx);
                    } else {
                        self.show(ctx);
                    }
                }
                Request::Show => {
                    if !self.visible {
                        self.show(ctx);
                    }
                }
                Request::Rescan => {
                    self.apps = catalog::scan();
                    self.refilter();
                }
                Request::ToggleLogin => {
                    let enabled = macos::set_login(!macos::login_enabled());
                    if let Some(item) = &self.login_item {
                        item.set_checked(enabled);
                    }
                }
            }
        }

        self.update_activation(ctx);

        if !self.positioned {
            self.positioned = self.position(ctx);
        }
    }

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
        let pressed_escape =
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

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
                        if self.draw_result_row(ui, &ctx, row, selection_moved) {
                            clicked_row = Some(row);
                        }
                    }
                });
        });

        if let Some(row) = clicked_row {
            self.launch(&ctx, row);
        } else if pressed_enter {
            self.launch(&ctx, self.selected);
        } else if pressed_escape {
            self.hide(&ctx);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.frames_since_show <= 1 {
            raw_input
                .events
                .retain(|e| !matches!(e, egui::Event::Text(t) if t == " " || t == "\u{a0}"));
        }
    }
}
