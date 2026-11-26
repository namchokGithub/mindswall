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
