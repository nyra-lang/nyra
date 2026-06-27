//! `nyra pkg prune` removes unused imports and prefixes unused locals.

mod common;

use common::{nyra_bin, workspace_root};
use std::path::{Path, PathBuf};
use std::process::Command;

fn copy_prune_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    let src = workspace_root().join("tests/fixtures/prune_unused");
    copy_dir_all(&src, &root).expect("copy prune fixture");
    (dir, root)
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

#[test]
fn pkg_prune_check_reports_unused() {
    let (_guard, dir) = copy_prune_fixture();
    let output = Command::new(nyra_bin())
        .args(["pkg", "prune", "--check"])
        .current_dir(&dir)
        .output()
        .expect("nyra pkg prune --check");
    assert!(
        !output.status.success(),
        "expected check to fail when unused code exists"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unused code found") || stderr.contains("prune check"),
        "stderr={stderr}"
    );
}

#[test]
fn pkg_prune_applies_fixes() {
    let (_guard, dir) = copy_prune_fixture();
    let main = dir.join("main.ny");

    let output = Command::new(nyra_bin())
        .args(["pkg", "prune"])
        .current_dir(&dir)
        .output()
        .expect("nyra pkg prune");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let pruned = std::fs::read_to_string(&main).expect("read pruned main.ny");
    assert!(!pruned.contains("import \"src/unused.ny\""));
    assert!(pruned.contains("let _dead = 99"));
}
