//! CLI integration tests.
//!
//! These tests must never change the real desktop wallpaper, read or write the
//! real user configuration, or send control commands to a running app. They
//! therefore avoid `run`, `config show`, and the control commands (`next`,
//! `previous`, `pause`, `resume`, `status`), and only run `random` and `set`
//! with inputs that fail before any wallpaper is applied.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// A unique temporary directory that is removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "mindswall-cli-{name}-{}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn mindswall() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mindswall"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn scan_and_errors_are_reported_through_cli() {
    let root = TempDir::new("scan");
    let root = root.path();
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("nested/picture.PNG"), []).unwrap();
    let output = mindswall()
        .args(["scan", "--no-recursive"])
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let output = mindswall().arg("scan").arg(root).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("picture.PNG"));
    let output = mindswall()
        .arg("scan")
        .arg(root.join("missing"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing"));
    let output = mindswall()
        .arg("set")
        .arg(root.join("missing.jpg"))
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn help_lists_all_commands() {
    let output = mindswall().arg("--help").output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for command in [
        "scan", "random", "set", "run", "next", "previous", "pause", "resume", "status", "config",
    ] {
        assert!(
            help.lines()
                .any(|line| line.split_whitespace().next() == Some(command)),
            "help does not list `{command}`:\n{help}"
        );
    }
}

#[test]
fn recursive_and_no_recursive_conflict() {
    let root = TempDir::new("conflict");
    for command in ["scan", "random"] {
        let output = mindswall()
            .args([command, "--recursive", "--no-recursive"])
            .arg(root.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{command} accepted both flags");
        let message = stderr(&output);
        assert!(
            message.contains("--recursive") && message.contains("--no-recursive"),
            "unexpected stderr for {command}: {message}"
        );
    }
}

#[test]
fn random_fails_for_missing_folder() {
    let root = TempDir::new("random-missing");
    let missing = root.path().join("missing-folder");
    let output = mindswall().arg("random").arg(&missing).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let message = stderr(&output);
    assert!(
        message.contains("not a readable folder") && message.contains("missing-folder"),
        "unexpected stderr: {message}"
    );
}

#[test]
fn random_fails_without_supported_images() {
    let root = TempDir::new("random-empty");
    fs::write(root.path().join("notes.txt"), "not an image").unwrap();
    fs::write(root.path().join("picture.gif"), []).unwrap();
    let output = mindswall().arg("random").arg(root.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let message = stderr(&output);
    assert!(
        message.contains("no supported images found"),
        "unexpected stderr: {message}"
    );
}

#[test]
fn set_fails_for_missing_file() {
    let root = TempDir::new("set-missing");
    let output = mindswall()
        .arg("set")
        .arg(root.path().join("missing.jpg"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let message = stderr(&output);
    if cfg!(windows) {
        assert!(
            message.contains("not a readable file") && message.contains("missing.jpg"),
            "unexpected stderr: {message}"
        );
    } else {
        assert!(
            message.contains("wallpaper setting is supported only on Windows"),
            "unexpected stderr: {message}"
        );
    }
}
