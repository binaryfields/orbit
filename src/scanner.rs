use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::catalog::{self, AppEntry};

const RESCAN_INTERVAL: Duration = Duration::from_secs(30);

pub struct Scanner {
    tx: mpsc::Sender<Vec<AppEntry>>,
    rx: mpsc::Receiver<Vec<AppEntry>>,
    last: Instant,
}

impl Scanner {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let scanner = Scanner {
            tx,
            rx,
            last: Instant::now(),
        };
        scanner.spawn(ctx);
        scanner
    }

    fn spawn(&self, ctx: &egui::Context) {
        let (ctx, tx) = (ctx.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let apps = catalog::scan();
            if tx.send(apps).is_ok() {
                ctx.request_repaint();
            }
        });
    }

    pub fn force(&mut self, ctx: &egui::Context) {
        self.last = Instant::now();
        self.spawn(ctx);
    }

    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.last.elapsed() >= RESCAN_INTERVAL {
            self.force(ctx);
        }
    }

    pub fn drain(&self) -> Option<Vec<AppEntry>> {
        self.rx.try_iter().last()
    }
}
