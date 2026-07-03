//! Watch `.ny` files and re-run check or build on change (in-process for fast dev loop).

use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::app::session::{build, run_file};
use crate::commands::check;
#[derive(Debug, Clone, Copy)]
pub enum WatchMode {
    Check,
    Build,
    Run,
}

pub fn watch(path: &Path, mode: WatchMode) -> Result<(), String> {
    let root = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    };
    let target = path.to_path_buf();

    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default(),
    )
    .map_err(|e| e.to_string())?;

    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    eprintln!(
        "watch: monitoring {} — mode {:?} (Ctrl+C to stop)",
        root.display(),
        mode
    );
    run_once(&target, mode)?;

    let mut debounce = None::<std::time::Instant>;
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(event) => {
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                ) {
                    debounce = Some(std::time::Instant::now());
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if debounce.is_some_and(|t| t.elapsed() > Duration::from_millis(300)) {
            debounce = None;
            eprintln!("watch: change detected");
            if let Err(e) = run_once(&target, mode) {
                eprintln!("watch: {e}");
            }
        }
    }
    Ok(())
}

fn run_once(path: &Path, mode: WatchMode) -> Result<(), String> {
    let stability = StabilityFlags::default();
    match mode {
        WatchMode::Check => check::check(path, &stability),
        WatchMode::Build => build(
            path,
            None,
            &OptFlags::default(),
            false,
            false,
            false,
            &TargetArgs::default(),
            &stability,
            false,
            false,
            false,
        ),
        WatchMode::Run => run_file(
            path,
            &OptFlags::default(),
            &TargetArgs::default(),
            &stability,
            false,
            false,
            false,
        ),
    }
}
