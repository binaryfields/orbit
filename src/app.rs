use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use global_hotkey::GlobalHotKeyManager;
use tray_icon::TrayIcon;
use tray_icon::menu::CheckMenuItem;

use crate::catalog;
use crate::hotkey::register_hotkey;
use crate::launcher::Launcher;
use crate::view::{Intent, View};
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

pub struct OrbitApp {
    _hotkey: Option<GlobalHotKeyManager>,
    _tray: Option<TrayIcon>,
    login_item: Option<CheckMenuItem>,
    cmd_rx: Receiver<Request>,
    launcher: Launcher,
    view: View,
    visible: bool,
    frames_since_show: u32,
    pending_activate: bool,
    positioned: bool,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let mut launcher = Launcher::default();
        launcher.set_apps(catalog::scan());

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
            launcher,
            view: View::default(),
            visible: true,
            frames_since_show: 0,
            pending_activate: true,
            positioned: false,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.visible = true;
        IS_VISIBLE.store(true, Ordering::SeqCst);
        self.frames_since_show = 0;
        self.launcher.reset();
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

    fn launch(&mut self, ctx: &egui::Context, row: usize) {
        let Some(entry) = self.launcher.entry(row) else {
            return;
        };
        if let Err(err) = macos::open(&entry.path) {
            eprintln!("orbit: failed to launch {}: {err}", entry.path.display());
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
                    self.launcher.set_apps(catalog::scan());
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
        match self.view.render(ui, &mut self.launcher) {
            Some(Intent::Launch(row)) => self.launch(&ctx, row),
            Some(Intent::Dismiss) => self.hide(&ctx),
            None => {}
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
