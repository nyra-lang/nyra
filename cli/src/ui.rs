//! Colored, structured CLI output for `nyra pkg` and related commands.

use std::io::IsTerminal;
use std::sync::OnceLock;
use std::time::Duration;

use compiler::ColorChoice;

static COLOR: OnceLock<ColorChoice> = OnceLock::new();

pub fn set_cli_color(choice: ColorChoice) {
    let _ = COLOR.set(choice);
}

fn colors_on() -> bool {
    match COLOR.get().copied().unwrap_or(ColorChoice::Auto) {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
        }
    }
}

pub struct Ui {
    on: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self { on: colors_on() }
    }

    fn paint(&self, text: &str, code: &str) -> String {
        if self.on {
            format!("{code}{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn bold(&self, text: &str) -> String {
        self.paint(text, "\x1b[1m")
    }

    pub fn dim(&self, text: &str) -> String {
        self.paint(text, "\x1b[2m")
    }

    pub fn green(&self, text: &str) -> String {
        self.paint(text, "\x1b[38;5;42m")
    }

    pub fn cyan(&self, text: &str) -> String {
        self.paint(text, "\x1b[38;5;51m")
    }

    pub fn blue(&self, text: &str) -> String {
        self.paint(text, "\x1b[38;5;39m")
    }

    pub fn magenta(&self, text: &str) -> String {
        self.paint(text, "\x1b[38;5;141m")
    }

    pub fn yellow(&self, text: &str) -> String {
        self.paint(text, "\x1b[38;5;214m")
    }

    pub fn section(&self, title: &str, subtitle: &str) -> String {
        let sub = if subtitle == "." {
            "./".to_string()
        } else {
            subtitle.to_string()
        };
        format!(
            "\n{}  {}",
            self.bold(&self.magenta(title)),
            self.dim(&sub)
        )
    }

    pub fn success(&self, message: &str) -> String {
        format!("{}  {}", self.green("✔"), self.bold(message))
    }

    pub fn item(&self, name: &str) -> String {
        format!("  {}  {}", self.green("●"), self.bold(name))
    }

    pub fn field(&self, key: &str, value: &str) -> String {
        format!(
            "      {}  {}",
            self.dim(&format!("{key:<8}")),
            self.field_value(value)
        )
    }

    fn field_value(&self, value: &str) -> String {
        if value.starts_with('"') && value.ends_with('"') {
            self.cyan(value)
        } else if value.contains('/') || value.contains('\\') {
            self.blue(value)
        } else {
            self.bold(value)
        }
    }

    pub fn hint(&self, text: &str) -> String {
        format!("  {}  {}", self.dim("tip"), self.cyan(text))
    }

    pub fn cmd(&self, text: &str) -> String {
        self.cyan(text)
    }

    pub fn path(&self, text: &str) -> String {
        self.blue(text)
    }

    pub fn count(&self, n: usize, noun: &str) -> String {
        self.dim(&format!("{n} {noun}"))
    }

    /// Cargo-style `   Compiling foo.ny (/path/to/project)`.
    pub fn compiling(&self, label: &str, root: &str) -> String {
        format!(
            "   {} {} ({})",
            self.green("Compiling"),
            self.bold(label),
            self.blue(root)
        )
    }

    /// Cargo-style `    Finished `dev` profile […] target(s) in 1.23s`.
    pub fn finished(&self, profile: &str, profile_detail: &str, elapsed: &str) -> String {
        format!(
            "    {} `{}` profile{} target(s) in {}",
            self.green("Finished"),
            profile,
            profile_detail,
            self.bold(elapsed)
        )
    }
}

/// Human-readable build duration (matches Cargo's `in 1.23s` style).
pub fn format_build_elapsed(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs >= 0.005 {
        format!("{secs:.2}s")
    } else {
        format!("{}ms", d.as_millis())
    }
}

/// Profile bracket suffix for `Finished` lines (` [optimized]`, etc.).
pub fn build_profile_detail(release: bool, debug_symbols: bool) -> &'static str {
    match (release, debug_symbols) {
        (true, true) => " [optimized + debuginfo]",
        (true, false) => " [optimized]",
        (false, true) => " [unoptimized + debuginfo]",
        (false, false) => " [unoptimized]",
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}
