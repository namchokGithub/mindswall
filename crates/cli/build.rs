//! Gives `mindswall.exe` the MindsWall icon and file details on Windows.

fn main() {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest.join("../../src-tauri/icons/icon.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    // The resource compiler comes with the Windows build tools, so resources
    // are added only when building on Windows for Windows; builds elsewhere,
    // including cross builds, go without the icon.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || !cfg!(windows) {
        return;
    }
    let mut resource = tauri_winres::WindowsResource::new();
    resource
        .set_icon(&icon.to_string_lossy())
        .set("ProductName", "MindsWall")
        .set("FileDescription", "MindsWall command-line tool")
        .set("OriginalFilename", "mindswall.exe")
        .set("LegalCopyright", "© 2026 Namchok Singhachai. MIT License.");
    resource
        .compile()
        .expect("failed to add the icon to mindswall.exe");
}
