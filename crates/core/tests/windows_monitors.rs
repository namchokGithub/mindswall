#![cfg(windows)]

#[test]
#[ignore = "requires an interactive Windows desktop"]
fn enumerate_monitors_on_interactive_desktop() {
    let monitors = mindswall_core::wallpaper::list_monitors().expect("enumerate monitors");
    assert!(
        !monitors.is_empty(),
        "expected at least one connected display"
    );
    assert!(
        monitors
            .iter()
            .all(|monitor| !monitor.id.is_empty() && monitor.width > 0 && monitor.height > 0)
    );
}
