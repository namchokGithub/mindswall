use mindswall_core::ipc::{PlaybackStatus, Response};

#[test]
fn legacy_and_structured_status_are_compatible() {
    let legacy: Response =
        serde_json::from_str(r#"{"ok":true,"message":"running: image.jpg"}"#).unwrap();
    assert!(legacy.status.is_none());
    let response = Response {
        ok: true,
        message: "running: image.jpg".into(),
        status: Some(PlaybackStatus {
            state: "running".into(),
            current: Some("nested/image.jpg".into()),
            position: Some(2),
            total: 50,
            combined: Vec::new(),
            display: None,
            monitors: Vec::new(),
        }),
    };
    let json = serde_json::to_string(&response).unwrap();
    let parsed: Response = serde_json::from_str(&json).unwrap();
    let status = parsed.status.unwrap();
    assert_eq!(
        status.current.unwrap(),
        std::path::PathBuf::from("nested/image.jpg")
    );
    assert_eq!(status.position, Some(2));
    assert_eq!(status.total, 50);
    assert_eq!(parsed.message, "running: image.jpg");
}

#[test]
fn per_monitor_commands_and_status_round_trip() {
    use mindswall_core::ipc::{Command, MonitorPlayback};
    let json = serde_json::to_string(&Command::NextMonitor("left".into())).unwrap();
    assert_eq!(json, r#"{"next_monitor":"left"}"#);
    assert!(matches!(
        serde_json::from_str(r#"{"previous_monitor":"left"}"#).unwrap(),
        Command::PreviousMonitor(monitor) if monitor == "left"
    ));

    // Older services send no monitor list.
    let old: PlaybackStatus =
        serde_json::from_str(r#"{"state":"running","current":null,"position":null,"total":0}"#)
            .unwrap();
    assert!(old.monitors.is_empty());
    let status = PlaybackStatus {
        monitors: vec![MonitorPlayback {
            monitor: "left".into(),
            current: Some("a.png".into()),
            position: Some(1),
            total: 2,
            combined: Vec::new(),
        }],
        ..old
    };
    let parsed: PlaybackStatus =
        serde_json::from_str(&serde_json::to_string(&status).unwrap()).unwrap();
    assert_eq!(parsed.monitors[0].monitor, "left");
    assert_eq!(parsed.monitors[0].total, 2);
}
