use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use global_hotkey::GlobalHotKeyManager;
use tray_icon::TrayIcon;

use crate::catalog;
use crate::session::Session;
use crate::view::{Action, View};
use crate::{hotkey, macos, tray};

const WINDOW_Y_FRACTION: f32 = 0.18;

pub enum Command {
    Toggle,
    Show,
    Rescan,
}

pub struct OrbitApp {
    _hotkey: Option<GlobalHotKeyManager>,
    _tray: Option<TrayIcon>,
    cmd_rx: Receiver<Command>,
    session: Session,
    view: View,
    visible: bool,
    positioned: bool,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.text_cursor.blink = false;
        cc.egui_ctx.set_visuals(visuals);

        let mut session = Session::default();
        session.set_apps(catalog::scan());

        let (cmd_tx, cmd_rx) = mpsc::channel();
        let sender = CommandSender {
            tx: cmd_tx,
            ctx: cc.egui_ctx.clone(),
        };

        Self {
            _hotkey: hotkey::register(sender.clone()),
            _tray: tray::build(sender),
            cmd_rx,
            session,
            view: View::default(),
            visible: true,
            positioned: false,
        }
    }

    fn handle_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::Toggle | Command::Show if !self.visible => self.show(ctx),
            Command::Toggle => self.hide(ctx),
            Command::Show => {}
            Command::Rescan => self.session.set_apps(catalog::scan()),
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.visible = true;
        self.session.reset();
        macos::activate_app();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.position(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn position(&self, ctx: &egui::Context) -> bool {
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return false;
        };
        if monitor.x <= 0.0 || monitor.y <= 0.0 {
            return false;
        }
        let pos = egui::pos2(
            (monitor.x - crate::WINDOW_SIZE.x) / 2.0,
            monitor.y * WINDOW_Y_FRACTION,
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        true
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.visible = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        macos::hide_app();
    }
}

impl eframe::App for OrbitApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(command) = self.cmd_rx.try_recv() {
            self.handle_command(ctx, command);
        }

        if !self.positioned {
            self.positioned = self.position(ctx);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        match self.view.render(ui, &mut self.session) {
            Some(Action::Launch(row)) => {
                if let Some(entry) = self.session.entry(row) {
                    if let Err(err) = macos::open(&entry.path) {
                        eprintln!("orbit: failed to launch {}: {err}", entry.path.display());
                    }
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
