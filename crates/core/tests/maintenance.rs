use mindswall_core::{
    config::{Config, Folder},
    history::{History, HistoryEntry},
    utilities::prune_cache,
};
use std::{collections::HashSet, ffi::OsString, fs, time::Duration};

#[test]
fn invalid_history_is_moved_aside_and_recording_works_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.json");
    fs::write(&path, "{ broken").unwrap();
    assert!(History::load(&path).unwrap().entries.is_empty());
    assert!(!path.exists());
    let backups: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("history.json.invalid-"))
        .collect();
    assert_eq!(backups.len(), 1);
    let mut history = History::default();
    history.push(HistoryEntry {
        path: "a.jpg".into(),
        timestamp: 1,
        monitor: None,
    });
    history.save(&path).unwrap();
    assert_eq!(History::load(&path).unwrap().entries.len(), 1);
}

#[test]
fn prune_cache_removes_oldest_files_and_keeps_protected_ones() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["1.png", "2.png", "3.png", "4.png"] {
        fs::write(dir.path().join(name), [0; 10]).unwrap();
        // Distinct modification times so the age order is deterministic.
        std::thread::sleep(Duration::from_millis(20));
    }
    let keep: HashSet<OsString> = [OsString::from("1.png")].into();
    assert_eq!(prune_cache(dir.path(), 2, u64::MAX, &keep).unwrap(), 2);
    let mut left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    left.sort();
    assert_eq!(left, ["1.png", "4.png"]);
    assert_eq!(prune_cache(dir.path(), 10, 10, &HashSet::new()).unwrap(), 1);
    assert_eq!(
        prune_cache(&dir.path().join("missing"), 0, 0, &HashSet::new()).unwrap(),
        0
    );
}

#[test]
fn many_exclusions_are_matched() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.jpg", "b.jpg", "c.jpg"] {
        fs::write(dir.path().join(name), []).unwrap();
    }
    let mut excluded: Vec<_> = (0..50_000)
        .map(|n| dir.path().join(format!("other-{n}.jpg")))
        .collect();
    excluded.push(dir.path().join("b.jpg"));
    let config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        excluded,
        ..Config::default()
    };
    assert_eq!(
        config.images().unwrap(),
        vec![dir.path().join("a.jpg"), dir.path().join("c.jpg")]
    );
}

#[cfg(windows)]
#[test]
fn exclusions_ignore_case_and_separator_style_on_windows() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Photo.JPG"), []).unwrap();
    let excluded = dir
        .path()
        .join("photo.jpg")
        .to_string_lossy()
        .to_uppercase()
        .replace('\\', "/");
    let config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        excluded: vec![excluded.into()],
        ..Config::default()
    };
    assert!(config.images().unwrap().is_empty());
}
