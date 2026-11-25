use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn scan_and_errors_are_reported_through_cli() {
    let root = std::env::temp_dir().join(format!(
        "mindswall-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("nested/picture.PNG"), []).unwrap();
    let binary = env!("CARGO_BIN_EXE_mindswall");
    let output = Command::new(binary)
        .args(["scan", "--no-recursive"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let output = Command::new(binary)
        .arg("scan")
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("picture.PNG"));
    let output = Command::new(binary)
        .arg("scan")
        .arg(root.join("missing"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing"));
    let output = Command::new(binary)
        .arg("set")
        .arg(root.join("missing.jpg"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    fs::remove_dir_all(root).unwrap();
}
