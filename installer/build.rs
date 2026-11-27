//! Embeds the MindsWall program into the setup and sets up the setup window.
//!
//! Set `MINDSWALL_APP_EXE` to the release `mindswall-desktop.exe` when
//! building a setup to ship. Without it the setup is built with nothing to
//! install, which is enough for checks and tests; it then refuses to install.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=MINDSWALL_APP_EXE");
    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("MindsWall.exe");
    match std::env::var_os("MINDSWALL_APP_EXE") {
        Some(app) => {
            let app = PathBuf::from(app);
            println!("cargo:rerun-if-changed={}", app.display());
            std::fs::copy(&app, &out).unwrap_or_else(|error| {
                panic!(
                    "could not read MINDSWALL_APP_EXE {}: {error}",
                    app.display()
                )
            });
        }
        None => std::fs::write(&out, []).expect("could not write the empty payload"),
    }
    // The setup asks for no elevation, so Windows does not treat a program
    // named "setup" or "uninstall" as one that needs administrator rights.
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows-app-manifest.xml"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run the Tauri build script");
}
