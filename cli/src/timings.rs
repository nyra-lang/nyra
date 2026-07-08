//! Dev compile timings (Cargo-style `--timings`).

use std::time::{Duration, Instant};

use crate::ui::Ui;

#[derive(Default)]
pub struct BuildTimings {
    pub analysis: Duration,
    pub codegen: Duration,
    pub link: Duration,
}

impl BuildTimings {
    pub fn report(&self, ui: &Ui) {
        let compile = self.analysis + self.codegen;
        let total = compile + self.link;
        if total.as_millis() == 0 {
            return;
        }
        eprintln!(
            "{}",
            ui.finished(
                "timings",
                "",
                &format!(
                    "compile {:.2}s | link {:.2}s",
                    compile.as_secs_f64(),
                    self.link.as_secs_f64()
                )
            )
        );
    }
}

pub struct TimedStage {
    started: Instant,
}

impl TimedStage {
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
        }
    }

    pub fn elapsed(self) -> Duration {
        self.started.elapsed()
    }
}
