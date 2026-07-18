use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use global_hotkey::GlobalHotKeyManager;
use tray_icon::TrayIcon;
use tray_icon::menu::CheckMenuItem;

use crate::hotkey::register_hotkey;
use crate::launcher::Launcher;
use crate::scanner::Scanner;
use crate::view::{Intent, View};
use crate::window::{Step, Window};
use crate::{macos, tray};

pub(crate) static IS_VISIBLE: AtomicBool = AtomicBool::new(true);

const WINDOW_Y_FRAC: f32 = 0.18;

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
    scanner: Scanner,
    cmd_rx: Receiver<Request>,
    launcher: Launcher,
    view: View,
    window: Window,
    positioned_for: Option<egui::Vec2>,
    last_activation_frame: Option<u64>,
}

impl OrbitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        // egui_taffy discards and re-runs the frame when its layout changes;
        // it needs at least one extra pass to settle.
        cc.egui_ctx.options_mut(|options| {
            options.max_passes = std::num::NonZeroUsize::new(2).unwrap();
        });

        macos::join_all_spaces();
        macos::add_vibrancy();

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
            scanner: Scanner::new(&cc.egui_ctx),
            cmd_rx,
            launcher: Launcher::default(),
            view: View::default(),
            window: Window::Activating { frames: 0 },
            positioned_for: None,
            last_activation_frame: None,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.window = Window::Activating { frames: 0 };
        IS_VISIBLE.store(true, Ordering::SeqCst);
        self.launcher.reset();
        self.view.focus_query();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.scanner.refresh(ctx);
        ctx.request_repaint();
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.window = Window::Hidden;
        IS_VISIBLE.store(false, Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        macos::hide_app();
    }

    fn set_visible(&mut self, ctx: &egui::Context, visible: bool) {
        match (self.window.is_visible(), visible) {
            (false, true) => self.show(ctx),
            (true, false) => self.hide(ctx),
            _ => {}
        }
    }

    fn reposition(&mut self, ctx: &egui::Context) {
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return;
        };
        if monitor.x <= 0.0 || monitor.y <= 0.0 || self.positioned_for == Some(monitor) {
            return;
        }
        let pos = egui::pos2(
            (monitor.x - crate::WINDOW_SIZE.x) / 2.0,
            monitor.y * WINDOW_Y_FRAC,
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        self.positioned_for = Some(monitor);
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

    fn handle(&mut self, ctx: &egui::Context, request: Request) {
        match request {
            Request::Toggle => self.set_visible(ctx, !self.window.is_visible()),
            Request::Show => self.set_visible(ctx, true),
            Request::Rescan => self.scanner.force(ctx),
            Request::ToggleLogin => {
                let enabled = macos::set_login(!macos::login_enabled());
                if let Some(item) = &self.login_item {
                    item.set_checked(enabled);
                }
            }
        }
    }

    fn update_activation(&mut self, ctx: &egui::Context) {
        // logic() re-runs on every layout pass; the window state machine
        // counts real frames, so advance it at most once per frame.
        let frame = ctx.cumulative_frame_nr();
        if self.last_activation_frame == Some(frame) {
            return;
        }
        self.last_activation_frame = Some(frame);

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
        if let Some(apps) = self.scanner.drain() {
            self.launcher.set_apps(apps);
        }

        while let Ok(request) = self.cmd_rx.try_recv() {
            self.handle(ctx, request);
        }

        self.update_activation(ctx);

        self.reposition(ctx);
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

    /// The global hotkey is Space-based, so the keystroke that summoned the
    /// window would otherwise land in the query box as a leading space.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.window.just_shown() {
            raw_input
                .events
                .retain(|e| !matches!(e, egui::Event::Text(t) if t == " " || t == "\u{a0}"));
        }
    }
}
