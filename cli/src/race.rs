//! Concurrency race detectors: ThreadSanitizer (`--race`) and native lock-set (`--race-native`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::app::session::{compile_and_link, run_file};
use crate::target::TargetSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceDetector {
    /// Clang ThreadSanitizer (`-fsanitize=thread`).
    Tsan,
    /// Lightweight Nyra runtime (`stdlib/rt/rt_race.c`).
    Native,
}

impl RaceDetector {
    pub fn label(self) -> &'static str {
        match self {
            RaceDetector::Tsan => "ThreadSanitizer (TSan)",
            RaceDetector::Native => "native race runtime",
        }
    }

    pub fn flag(self) -> &'static str {
        match self {
            RaceDetector::Tsan => "--race",
            RaceDetector::Native => "--race-native",
        }
    }
}

/// Whether OptFlags requests any concurrency/memory sanitizer.
pub fn wants_race_or_sanitize(opt: &OptFlags) -> bool {
    opt.race || opt.race_native || opt.sanitize
}

/// Debug symbols should be on for usable sanitizer stacks.
pub fn effective_debug_symbols(opt: &OptFlags, requested: bool) -> bool {
    requested || wants_race_or_sanitize(opt)
}

pub fn apply_detector(opt: &mut OptFlags, detector: RaceDetector) {
    match detector {
        RaceDetector::Tsan => {
            opt.race = true;
            opt.race_native = false;
            opt.sanitize = false;
        }
        RaceDetector::Native => {
            opt.race_native = true;
            opt.race = false;
            opt.sanitize = false;
        }
    }
}

/// Preflight: TSan needs a host clang that accepts `-fsanitize=thread`.
pub fn ensure_detector_available(detector: RaceDetector, spec: &TargetSpec) -> Result<(), String> {
    if spec.is_wasm {
        return Err(format!(
            "{} is not available for wasm targets — race detectors need a native host build",
            detector.flag()
        ));
    }
    if spec.is_cross {
        return Err(format!(
            "{} is only supported for host builds (not cross-compile to {})",
            detector.flag(),
            spec.triple_for_codegen()
        ));
    }
    match detector {
        RaceDetector::Tsan => ensure_tsan_clang(),
        RaceDetector::Native => Ok(()),
    }
}

fn ensure_tsan_clang() -> Result<(), String> {
    let clang = crate::llvm_tools::find_clang();
    let status = Command::new(&clang)
        .args([
            "-fsanitize=thread",
            "-x",
            "c",
            "-",
            "-o",
            "/dev/null",
            "-c",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(b"int main(void){return 0;}\n");
            }
            child.wait()
        });
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err(format!(
            "--race: clang `{clang}` does not support ThreadSanitizer (`-fsanitize=thread`)\n\
             tip: use `nyra race --native` (Nyra lock-set runtime) or install a TSan-capable LLVM"
        )),
    }
}

pub fn print_race_banner(detector: RaceDetector) {
    eprintln!(
        "race: detector = {} ({})",
        detector.label(),
        detector.flag()
    );
    match detector {
        RaceDetector::Tsan => {
            eprintln!("race: linking with -fsanitize=thread (debug frames enabled)");
            eprintln!("race: run the binary to surface data races at runtime");
        }
        RaceDetector::Native => {
            eprintln!("race: linking stdlib/rt/rt_race.c (-DNYRA_RACE_NATIVE_BUILD)");
            eprintln!("race: call Race_init() / Race_track_* from stdlib/race.ny for tracking");
        }
    }
}

/// `nyra race` — build with a race detector (default TSan) and run.
pub fn race_command(
    path: &Path,
    native: bool,
    build_only: bool,
    args: &[String],
) -> Result<(), String> {
    let detector = if native {
        RaceDetector::Native
    } else {
        RaceDetector::Tsan
    };
    let mut opt = OptFlags::default();
    apply_detector(&mut opt, detector);
    let target_args = TargetArgs::default();
    let spec = target_args.resolve()?;
    ensure_detector_available(detector, &spec)?;
    print_race_banner(detector);

    let stability = StabilityFlags::default();
    let bin = compile_and_link(
        path,
        &opt,
        true, // always debug symbols for race
        false,
        false,
        &spec,
        None,
        &stability,
        false,
        false,
        false,
        None,
    )?;

    if build_only {
        println!("{}", bin.display());
        return Ok(());
    }

    eprintln!("race: running {}", bin.display());
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run {}: {e}", bin.display()))?;
    if !status.success() {
        // Non-zero may be a TSan abort — still surface as failure.
        return Err(format!(
            "program exited with status {} (race detector may have reported issues)",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

/// Shared preflight + banner when `build`/`run`/`test` pass `--race` / `--race-native`.
pub fn prepare_opt_flags(opt: &OptFlags, target_args: &TargetArgs) -> Result<(), String> {
    let detector = if opt.race {
        Some(RaceDetector::Tsan)
    } else if opt.race_native {
        Some(RaceDetector::Native)
    } else {
        None
    };
    let Some(detector) = detector else {
        return Ok(());
    };
    let spec = target_args.resolve()?;
    ensure_detector_available(detector, &spec)?;
    print_race_banner(detector);
    Ok(())
}

/// Convenience for tests / tooling: path used when checking clang (may be unused on skip).
#[allow(dead_code)]
pub fn race_artifact_hint(bin: &Path) -> PathBuf {
    bin.to_path_buf()
}

/// Re-export run_file after applying race flags (used by watch).
pub fn run_with_opt(
    path: &Path,
    opt: &OptFlags,
    target_args: &TargetArgs,
) -> Result<(), String> {
    prepare_opt_flags(opt, target_args)?;
    run_file(
        path,
        opt,
        target_args,
        &StabilityFlags::default(),
        false,
        false,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_flags() {
        assert_eq!(RaceDetector::Tsan.flag(), "--race");
        assert_eq!(RaceDetector::Native.flag(), "--race-native");
    }

    #[test]
    fn debug_symbols_follow_race() {
        let mut opt = OptFlags::default();
        assert!(!effective_debug_symbols(&opt, false));
        opt.race = true;
        assert!(effective_debug_symbols(&opt, false));
        opt.race = false;
        opt.race_native = true;
        assert!(effective_debug_symbols(&opt, false));
    }
}
