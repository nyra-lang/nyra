//! Watch `.ny` files and re-run check, build, or run on change.

use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::app::session::{build, run_file};
use crate::commands::check;
use crate::race::{effective_debug_symbols, prepare_opt_flags};

#[derive(Debug, Clone, Copy)]
pub enum WatchMode {
    Check,
    Build,
    Run,
}

impl WatchMode {
    fn label(self) -> &'static str {
        match self {
            WatchMode::Check => "check",
            WatchMode::Build => "build",
            WatchMode::Run => "run",
        }
    }
}

pub fn watch(path: &Path, mode: WatchMode) -> Result<(), String> {
    watch_with_opt(path, mode, &OptFlags::default())
}

pub fn watch_with_opt(path: &Path, mode: WatchMode, opt: &OptFlags) -> Result<(), String> {
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

    let race_note = if opt.race {
        " + --race"
    } else if opt.race_native {
        " + --race-native"
    } else {
        ""
    };
    eprintln!(
        "watch: {} — on change: `nyra {}{}` (Ctrl+C to stop)",
        root.display(),
        mode.label(),
        race_note
    );
    eprintln!("watch: watching .ny / .nyra / nyra.mod (ignores target/)");
    run_once(&target, mode, opt)?;

    let mut debounce = None::<std::time::Instant>;
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(event) => {
                if !matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                ) {
                    continue;
                }
                if event.paths.iter().any(|p| is_relevant_path(p, &root)) {
                    debounce = Some(std::time::Instant::now());
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if debounce.is_some_and(|t| t.elapsed() > Duration::from_millis(350)) {
            debounce = None;
            eprintln!("watch: change detected — {}{}", mode.label(), race_note);
            if let Err(e) = run_once(&target, mode, opt) {
                eprintln!("watch: {e}");
            }
        }
    }
    Ok(())
}

fn is_relevant_path(path: &Path, root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return is_source_name(path);
    };
    for component in rel.components() {
        let c = component.as_os_str();
        if c == "target" || c == ".git" || c == "node_modules" || c == ".nyra-cache" {
            return false;
        }
    }
    is_source_name(path)
}

fn is_source_name(path: &Path) -> bool {
    match path.file_name().and_then(|s| s.to_str()) {
        Some("nyra.mod") => true,
        Some(name) if name.ends_with(".ny") || name.ends_with(".nyra") => true,
        _ => false,
    }
}

fn run_once(path: &Path, mode: WatchMode, opt: &OptFlags) -> Result<(), String> {
    let stability = StabilityFlags::default();
    let target_args = TargetArgs::default();
    match mode {
        WatchMode::Check => check::check(path, &stability),
        WatchMode::Build => {
            prepare_opt_flags(opt, &target_args)?;
            build(
                path,
                None,
                opt,
                effective_debug_symbols(opt, false),
                false,
                false,
                &target_args,
                &stability,
                false,
                false,
                false,
            )
        }
        WatchMode::Run => {
            prepare_opt_flags(opt, &target_args)?;
            run_file(
                path,
                opt,
                &target_args,
                &stability,
                false,
                false,
                false,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_target_dir() {
        let root = PathBuf::from("/proj");
        assert!(!is_relevant_path(
            &root.join("target/debug/foo.ll"),
            &root
        ));
        assert!(is_relevant_path(&root.join("src/main.ny"), &root));
        assert!(is_relevant_path(&root.join("nyra.mod"), &root));
    }
}
