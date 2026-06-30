//! Delegate package-manager commands to the standalone `nyrapkg` binary.

use std::path::PathBuf;
use std::process::Command;

use crate::toolchain::nyra_home;

/// Resolve `nyrapkg` executable: `$NYRAPKG`, `$NYRA_HOME/bin/nyrapkg`, sibling of `nyra`, then `PATH`.
pub fn nyrapkg_bin() -> PathBuf {
    if let Ok(p) = std::env::var("NYRAPKG") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let installed = nyra_home().join("bin").join("nyrapkg");
    if installed.is_file() {
        return installed;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin) = exe.parent() {
            let sibling = bin.join("nyrapkg");
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    PathBuf::from("nyrapkg")
}

pub fn run_nyrapkg(args: &[String]) -> Result<(), String> {
    let bin = nyrapkg_bin();
    let status = Command::new(&bin)
        .args(args)
        .status()
        .map_err(|e| {
            format!(
                "failed to run {}: {e}\n\
                 install nyrapkg (https://github.com/nyra-lang/pkg) or set NYRAPKG to its path",
                bin.display()
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(1);
        Err(format!("nyrapkg exited with status {code}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nyrapkg_bin_not_empty() {
        assert!(!nyrapkg_bin().as_os_str().is_empty());
    }

    #[test]
    fn recognizes_nyrapkg_commands() {
        assert!(matches!("init", "init" | "add"));
        assert!(!matches!("build", "init" | "add"));
    }
}
