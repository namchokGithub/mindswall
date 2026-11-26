use mindswall_core::{
    config::{Config, Folder},
    playback::Playback,
    scanner,
    slideshow::Slideshow,
    utilities::FitMode,
};
use std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn lenient_scan_skips_missing_folder_and_keeps_others() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.jpg"), []).unwrap();
    let report = scanner::scan_lenient(&[dir.path().join("missing"), dir.path().to_owned()], true);
    assert_eq!(report.images, vec![dir.path().join("a.jpg")]);
    assert_eq!(report.skipped.len(), 1);
}

#[test]
fn collection_ignores_unavailable_enabled_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.jpg"), []).unwrap();
    let config = Config {
        folders: vec![
            Folder {
                path: dir.path().join("offline"),
                enabled: true,
                recursive: true,
            },
            Folder {
                path: dir.path().to_owned(),
                enabled: true,
                recursive: true,
            },
        ],
        ..Config::default()
    };
    assert_eq!(config.images().unwrap(), vec![dir.path().join("a.jpg")]);
}

#[test]
fn invalid_config_is_moved_aside_before_defaults_are_used() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{ not json").unwrap();
    assert_eq!(Config::load(&path).unwrap(), Config::default());
    assert!(!path.exists());
    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("config.json.invalid-"))
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = std::fs::read_to_string(dir.path().join(&backups[0])).unwrap();
    assert_eq!(backup, "{ not json");
    Config::default().save(&path).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join(&backups[0])).unwrap(),
        "{ not json"
    );
}

#[test]
fn failing_image_is_skipped_and_removed() {
    let mut show = Slideshow::new(paths(&["a", "b", "c"]), false).unwrap();
    let applied = show
        .next_available(|path| {
            if path == Path::new("a") {
                Err("missing")
            } else {
                Ok(())
            }
        })
        .unwrap();
    assert_eq!(applied, PathBuf::from("b"));
    assert_eq!(show.total(), 2);
    assert!(!show.contains(Path::new("a")));
    assert_eq!(show.next(), PathBuf::from("c"));
    assert_eq!(show.next(), PathBuf::from("b"));
}

#[test]
fn slideshow_is_unchanged_when_no_image_can_be_applied() {
    let mut show = Slideshow::new(paths(&["a", "b"]), false).unwrap();
    show.next();
    assert!(show.next_available(|_| Err("offline")).is_err());
    assert_eq!(show.total(), 2);
    assert_eq!(show.current().unwrap(), PathBuf::from("a"));
}

#[test]
fn previous_skips_earlier_image_that_fails() {
    let mut show = Slideshow::new(paths(&["a", "b", "c"]), false).unwrap();
    show.next();
    show.next();
    show.next();
    let applied = show
        .previous_available(|path| {
            if path == Path::new("b") {
                Err("missing")
            } else {
                Ok(())
            }
        })
        .unwrap()
        .unwrap();
    assert_eq!(applied, PathBuf::from("a"));
    assert!(show.previous_available(|_| Ok::<(), ()>(())).is_none());
    assert_eq!(show.next(), PathBuf::from("c"));
}

#[test]
fn previous_reports_failure_without_changing_history() {
    let mut show = Slideshow::new(paths(&["a", "b"]), false).unwrap();
    show.next();
    show.next();
    assert!(
        show.previous_available(|_| Err("missing"))
            .unwrap()
            .is_err()
    );
    assert_eq!(show.current().unwrap(), PathBuf::from("b"));
    assert_eq!(show.total(), 2);
}

#[test]
fn update_keeps_cycle_without_repeats() {
    let mut show = Slideshow::new(paths(&["a", "b", "c", "d"]), true).unwrap();
    let mut seen = vec![show.next().to_owned(), show.next().to_owned()];
    let removed = paths(&["a", "b", "c", "d"])
        .into_iter()
        .find(|image| !seen.contains(image))
        .unwrap();
    let mut images: Vec<PathBuf> = paths(&["a", "b", "c", "d", "e"])
        .into_iter()
        .filter(|image| image != &removed)
        .collect();
    images.sort();
    show.update(images, true).unwrap();
    assert_eq!(show.total(), 4);
    assert_eq!(show.current().unwrap(), seen[1]);
    seen.push(show.next().to_owned());
    seen.push(show.next().to_owned());
    let cycle: HashSet<_> = seen.iter().collect();
    assert_eq!(cycle.len(), 4);
    assert!(cycle.contains(&PathBuf::from("e")));
    assert!(!cycle.contains(&removed));
}

#[test]
fn update_to_sequential_orders_remaining_images() {
    let mut show = Slideshow::new(paths(&["a", "b", "c", "d"]), false).unwrap();
    assert_eq!(show.next(), PathBuf::from("a"));
    show.update(paths(&["a", "b", "c", "d"]), true).unwrap();
    show.update(paths(&["a", "b", "c", "d"]), false).unwrap();
    assert_eq!(show.next(), PathBuf::from("b"));
    assert_eq!(show.next(), PathBuf::from("c"));
    assert_eq!(show.next(), PathBuf::from("d"));
}

fn folder_config(dir: &Path) -> Config {
    Config {
        folders: vec![Folder {
            path: dir.to_owned(),
            enabled: true,
            recursive: true,
        }],
        shuffle: false,
        ..Config::default()
    }
}

#[test]
fn reload_keeps_wallpaper_when_collection_still_contains_it() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.jpg", "b.jpg", "c.jpg"] {
        std::fs::write(dir.path().join(name), []).unwrap();
    }
    let mut config = folder_config(dir.path());
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    playback.show.as_mut().unwrap().next();
    let seen = RefCell::new(Vec::<PathBuf>::new());
    let apply = |path: &Path| {
        seen.borrow_mut().push(path.to_owned());
        Ok(())
    };
    let fallback = || Ok(PathBuf::from("default.jpg"));

    config.interval_seconds = 120;
    config.shuffle = true;
    playback
        .reload(&config, Instant::now(), &apply, &fallback)
        .unwrap();
    assert!(seen.borrow().is_empty());
    assert_eq!(playback.timer.interval(), Duration::from_secs(120));

    config.fit_mode = FitMode::Center;
    playback
        .reload(&config, Instant::now(), &apply, &fallback)
        .unwrap();
    assert_eq!(seen.borrow().as_slice(), &[dir.path().join("a.jpg")]);

    config.excluded.push(dir.path().join("a.jpg"));
    playback
        .reload(&config, Instant::now(), &apply, &fallback)
        .unwrap();
    assert_eq!(seen.borrow().len(), 2);
    assert_ne!(seen.borrow()[1], dir.path().join("a.jpg"));
}

#[test]
fn reload_keeps_pause_when_interval_changes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.jpg"), []).unwrap();
    let mut config = folder_config(dir.path());
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    playback.show.as_mut().unwrap().next();
    playback.timer.pause(Instant::now());
    config.interval_seconds = 300;
    playback
        .reload(&config, Instant::now(), &|_| Ok(()), &|| {
            Ok("default.jpg".into())
        })
        .unwrap();
    assert!(playback.timer.is_paused());
    assert_eq!(playback.timer.interval(), Duration::from_secs(300));
}
