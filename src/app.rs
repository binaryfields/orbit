use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use global_hotkey::GlobalHotKeyManager;
use tray_icon::TrayIcon;
use tray_icon::menu::CheckMenuItem;

use crate::catalog::AppEntry;
use crate::scanner::ScanJob;
use crate::session::Session;
use crate::view::{Action, View};
use crate::window::{Effect, WindowState};
use crate::{hotkey, macos, tray};

const WINDOW_Y_FRACTION: f32 = 0.18;

pub enum Command {
    Toggle,
    Show,
    Rescan,
    ToggleLogin,
    Loaded(Vec<AppEntry>),
}

pub struct OrbitApp {
    _hotkey: Option<GlobalHotKeyManager>,
    _tray: Option<TrayIcon>,
    login_item: Option<CheckMenuItem>,
    scan_job: ScanJob,
    cmd_rx: Receiver<Command>,
    session: Session,
    view: View,
    window_state: WindowState,
    last_monitor_size: Option<egui::Vec2>,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.text_cursor.blink = false;
        cc.egui_ctx.set_visuals(visuals);

        macos::configure_windows();

        let (cmd_tx, cmd_rx) = mpsc::channel();
        let sender = CommandSender {
            tx: cmd_tx,
            ctx: cc.egui_ctx.clone(),
        };
        let (tray, login_item) = tray::build(sender.clone()).unzip();

        Self {
            _hotkey: hotkey::register(sender.clone()),
            _tray: tray,
            login_item,
            scan_job: ScanJob::new(sender),
            cmd_rx,
            session: Session::default(),
            view: View::default(),
            window_state: WindowState::Activating { frames: 0 },
            last_monitor_size: None,
        }
    }

    fn handle_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::Toggle | Command::Show if !self.window_state.is_visible() => self.show(ctx),
            Command::Toggle => self.hide(ctx),
            Command::Show => {}
            Command::Rescan => self.scan_job.rescan(),
            Command::ToggleLogin => {
                let enabled = macos::set_start_at_login(!macos::starts_at_login());
                if let Some(item) = &self.login_item {
                    item.set_checked(enabled);
                }
            }
            Command::Loaded(apps) => self.session.set_apps(apps),
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.window_state = WindowState::Activating { frames: 0 };
        self.session.reset();
        self.view.focus_query();
        macos::activate_app();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.scan_job.rescan_if_stale();
        ctx.request_repaint();
    }

    fn update_activation(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.viewport().focused);
        match self.window_state.advance(focused) {
            Effect::Idle => {}
            Effect::Repaint { activate } => {
                if activate {
                    macos::activate_app();
                }
                ctx.request_repaint();
            }
            Effect::Dismiss => self.hide(ctx),
        }
    }

    fn reposition(&mut self, ctx: &egui::Context) {
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return;
        };
        if monitor.x <= 0.0 || monitor.y <= 0.0 || self.last_monitor_size == Some(monitor) {
            return;
        }
        let pos = egui::pos2(
            (monitor.x - crate::WINDOW_SIZE.x) / 2.0,
            monitor.y * WINDOW_Y_FRACTION,
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        self.last_monitor_size = Some(monitor);
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.window_state = WindowState::Hidden;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        macos::hide_app();
    }
}

impl eframe::App for OrbitApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(command) = self.cmd_rx.try_recv() {
            self.handle_command(ctx, command);
        }

        self.update_activation(ctx);

        self.reposition(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        match self.view.render(ui, &mut self.session) {
            Some(Action::Launch(row)) => {
                if let Some(entry) = self.session.entry(row) {
                    macos::open(&entry.path);
                    self.hide(&ctx);
                }
            }
            Some(Action::Dismiss) => self.hide(&ctx),
            None => {}
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.window_state.just_shown() {
            raw_input
                .events
                .retain(|e| !matches!(e, egui::Event::Text(t) if t == " " || t == "\u{a0}"));
        }
    }
}

#[derive(Clone)]
pub struct CommandSender {
    tx: Sender<Command>,
    ctx: egui::Context,
}

impl CommandSender {
    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
        self.ctx.request_repaint();
    }
}
