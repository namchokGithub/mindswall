use mindswall_core::config::Config;

#[test]
fn profile_switch_preserves_saved_settings() {
    let mut config = Config {
        interval_seconds: 60,
        ..Config::default()
    };
    config.save_profile("Work").unwrap();
    config.interval_seconds = 300;
    config.save_profile("Relax").unwrap();
    config.apply_profile("Work").unwrap();
    assert_eq!(config.interval_seconds, 60);
    assert_eq!(config.profiles.len(), 2);
    assert!(config.save_profile(" ").is_err());
    assert!(config.save_profile("work").is_err());
}

#[test]
fn active_profile_tracks_changes_and_can_be_updated() {
    let mut config = Config {
        interval_seconds: 60,
        ..Config::default()
    };
    assert!(!config.active_profile_changed());
    config.save_profile("Work").unwrap();
    assert!(!config.active_profile_changed());

    config.interval_seconds = 300;
    assert!(config.active_profile_changed());
    config.update_profile("Work").unwrap();
    assert!(!config.active_profile_changed());
    assert_eq!(config.profiles[0].settings.interval_seconds, 300);

    // Settings a profile does not store do not count as changes.
    config.hotkeys.next = "Ctrl+Alt+N".into();
    assert!(!config.active_profile_changed());

    config.shuffle = !config.shuffle;
    config.apply_profile("Work").unwrap();
    assert!(!config.active_profile_changed());
    assert!(config.update_profile("Missing").is_err());
}

#[test]
fn backup_validation_rejects_invalid_data_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let config = Config::default();
    config.save(&path).unwrap();
    assert!(
        Config::parse_backup(br#"{"version":1,"folders":[],"interval_seconds":0,"shuffle":true}"#)
            .is_err()
    );
    assert_eq!(Config::load(&path).unwrap(), config);
    let bytes = serde_json::to_vec(&config).unwrap();
    assert_eq!(Config::parse_backup(&bytes).unwrap(), config);
}

#[test]
fn rejects_duplicate_hotkeys() {
    let mut config = Config::default();
    config.hotkeys.next = "Ctrl+Alt+N".into();
    config.hotkeys.previous = "ctrl+alt+n".into();
    assert!(config.validate().is_err());
}
