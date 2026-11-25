use mindswall_core::config::{Config, Folder};

#[test]
fn config_roundtrips_and_invalid_json_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let expected = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: false,
        }],
        interval_seconds: 60,
        shuffle: false,
        ..Config::default()
    };
    expected.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), expected);
    let updated = Config {
        shuffle: true,
        ..expected.clone()
    };
    updated.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), updated);
    std::fs::write(&path, "not json").unwrap();
    assert_eq!(Config::load(&path).unwrap(), Config::default());
}

#[test]
fn enabled_folders_are_deduplicated_and_respect_recursion() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(nested.join("one.jpg"), []).unwrap();
    std::fs::write(dir.path().join("two.png"), []).unwrap();
    let config = Config {
        folders: vec![
            Folder {
                path: dir.path().to_owned(),
                enabled: true,
                recursive: true,
            },
            Folder {
                path: nested,
                enabled: true,
                recursive: false,
            },
            Folder {
                path: dir.path().join("missing"),
                enabled: false,
                recursive: true,
            },
        ],
        ..Config::default()
    };
    assert_eq!(config.images().unwrap().len(), 2);
}
