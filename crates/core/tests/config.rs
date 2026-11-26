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
    std::fs::write(
        &path,
        r#"{"version":1,"folders":[],"interval_seconds":600,"shuffle":true}"#,
    )
    .unwrap();
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

#[test]
fn the_same_folder_spelled_differently_is_listed_once() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(dir.path().join("one.jpg"), []).unwrap();
    std::fs::write(nested.join("two.png"), []).unwrap();
    let folder = |path: std::path::PathBuf| Folder {
        path,
        enabled: true,
        recursive: true,
    };
    // Only Unix adds a link below.
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut folders = vec![
        folder(dir.path().to_owned()),
        // Another spelling of the same folder.
        folder(nested.join("..")),
    ];
    // A link to the folder, like a junction on Windows.
    let _links = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        let link = _links.path().join("link");
        std::os::unix::fs::symlink(dir.path(), &link).unwrap();
        folders.push(folder(link));
    }
    let config = Config {
        folders,
        ..Config::default()
    };
    assert_eq!(config.images().unwrap().len(), 2);
}

#[test]
fn recursive_collection_includes_nested_images_matching_filter() {
    use mindswall_core::utilities::Orientation;
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    image::RgbImage::new(20, 10)
        .save(dir.path().join("landscape.png"))
        .unwrap();
    image::RgbImage::new(10, 20)
        .save(nested.join("portrait.png"))
        .unwrap();
    let mut config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        ..Config::default()
    };
    config.filter.orientation = Orientation::Portrait;
    assert_eq!(config.images().unwrap(), vec![nested.join("portrait.png")]);
    config.folders[0].recursive = false;
    assert!(config.images().unwrap().is_empty());
}

#[test]
fn restoring_one_excluded_image_keeps_other_exclusions() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.jpg");
    let second = dir.path().join("second.jpg");
    std::fs::write(&first, []).unwrap();
    std::fs::write(&second, []).unwrap();
    let mut config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        excluded: vec![first.clone(), second.clone()],
        ..Config::default()
    };
    assert!(config.images().unwrap().is_empty());
    config.excluded.retain(|path| path != &first);
    assert_eq!(config.images().unwrap(), vec![first]);
    assert_eq!(config.excluded, vec![second]);
}

#[test]
fn theme_defaults_to_system_and_round_trips() {
    use mindswall_core::config::Theme;
    let config: Config =
        serde_json::from_str(r#"{"version":1,"folders":[],"interval_seconds":600,"shuffle":true}"#)
            .unwrap();
    assert_eq!(config.theme, Theme::System);
    let dark = Config {
        theme: Theme::Dark,
        ..Config::default()
    };
    let json = serde_json::to_string(&dark).unwrap();
    assert!(json.contains(r#""theme":"dark""#));
    assert_eq!(
        serde_json::from_str::<Config>(&json).unwrap().theme,
        Theme::Dark
    );
    assert!(
        serde_json::from_str::<Config>(
            r#"{"version":1,"folders":[],"interval_seconds":600,"shuffle":true,"theme":"blue"}"#
        )
        .is_err()
    );
}
