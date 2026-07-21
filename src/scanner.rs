use std::time::{Duration, Instant};

use crate::app::{Command, CommandSender};
use crate::catalog;

const RESCAN_INTERVAL: Duration = Duration::from_secs(30);

pub struct ScanJob {
    sender: CommandSender,
    last_scan: Instant,
}

impl ScanJob {
    pub fn new(sender: CommandSender) -> Self {
        let mut job = ScanJob {
            sender,
            last_scan: Instant::now(),
        };
        job.rescan();
        job
    }

    pub fn rescan(&mut self) {
        self.last_scan = Instant::now();
        let sender = self.sender.clone();
        std::thread::spawn(move || sender.send(Command::Loaded(catalog::scan())));
    }

    pub fn rescan_if_stale(&mut self) {
        if self.last_scan.elapsed() >= RESCAN_INTERVAL {
            self.rescan();
        }
    }
}
