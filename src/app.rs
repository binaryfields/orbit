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
use crate::window::{Step, Window};
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
    window: Window,
    positioned: bool,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        macos::join_all_spaces();
        macos::add_vibrancy();

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
            window: Window::Activating { frames: 0 },
            positioned: false,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.window = Window::Activating { frames: 0 };
        IS_VISIBLE.store(true, Ordering::SeqCst);
        self.launcher.reset();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.position(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.window = Window::Hidden;
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
        let focused = ctx.input(|i| i.viewport().focused);
        match self.window.advance(focused) {
            Step::Idle => {}
            Step::Settle { activate } => {
                if activate {
                    macos::activate_app();
                }
                ctx.request_repaint();
            }
            Step::Dismiss => self.hide(ctx),
        }
    }
}

impl eframe::App for OrbitApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                Request::Toggle => {
                    if self.window.is_visible() {
                        self.hide(ctx);
                    } else {
                        self.show(ctx);
                    }
                }
                Request::Show => {
                    if !self.window.is_visible() {
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
        if self.window.just_shown() {
            raw_input
                .events
                .retain(|e| !matches!(e, egui::Event::Text(t) if t == " " || t == "\u{a0}"));
        }
    }
}
