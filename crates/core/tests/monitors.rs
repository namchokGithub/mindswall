use mindswall_core::{
    config::{Config, Folder, MonitorFolders, MonitorName},
    monitors::{MonitorShows, SharedTarget},
};
use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

fn image(path: &Path) {
    image::RgbImage::new(4, 3).save(path).unwrap();
}

fn folder(path: &Path) -> Folder {
    Folder {
        path: path.to_owned(),
        enabled: true,
        recursive: true,
    }
}

fn config(main: &Path, left: &Path) -> Config {
    Config {
        folders: vec![folder(main)],
        shuffle: false,
        monitor_folders: vec![MonitorFolders {
            monitor: "left".into(),
            folders: vec![folder(left)],
        }],
        ..Config::default()
    }
}

type Applied = RefCell<Vec<(String, PathBuf)>>;

fn recorder(applied: &Applied) -> impl Fn(&str, &Path) -> Result<(), String> + '_ {
    |monitor, path| {
        applied
            .borrow_mut()
            .push((monitor.to_owned(), path.to_owned()));
        Ok(())
    }
}

#[test]
fn monitors_with_own_folders_leave_the_main_slideshow() {
    let main = tempfile::tempdir().unwrap();
    let left = tempfile::tempdir().unwrap();
    image(&main.path().join("main.png"));
    let a = left.path().join("a.png");
    let b = left.path().join("b.png");
    image(&a);
    image(&b);
    let config = config(main.path(), left.path());
    assert!(MonitorShows::is_configured(&config));

    let mut shows = MonitorShows::default();
    assert_eq!(shows.configure(&config), vec!["left".to_owned()]);
    assert_eq!(shows.shared_target(), SharedTarget::Desktop);

    shows.set_connected(vec!["left".into(), "right".into()]);
    assert_eq!(shows.active(), vec!["left".to_owned()]);
    assert_eq!(
        shows.shared_target(),
        SharedTarget::Monitors(vec!["right".into()])
    );

    let applied = Applied::default();
    shows.reapply(&shows.active(), &recorder(&applied));
    shows.advance(&recorder(&applied));
    assert_eq!(
        *applied.borrow(),
        vec![("left".into(), a.clone()), ("left".into(), b.clone())]
    );
    assert_eq!(shows.current("left"), Some(b.as_path()));

    // Reconfiguring keeps the cycle of an existing monitor slideshow.
    assert!(shows.configure(&config).is_empty());
    assert_eq!(shows.current("left"), Some(b.as_path()));
}

#[test]
fn disconnected_monitors_pause_and_resume_their_slideshow() {
    let main = tempfile::tempdir().unwrap();
    let left = tempfile::tempdir().unwrap();
    image(&main.path().join("main.png"));
    let a = left.path().join("a.png");
    image(&a);
    image(&left.path().join("b.png"));
    let config = config(main.path(), left.path());
    let mut shows = MonitorShows::default();
    shows.configure(&config);
    let applied = Applied::default();

    assert_eq!(
        shows.set_connected(vec!["left".into()]),
        vec!["left".to_owned()]
    );
    shows.advance(&recorder(&applied));
    assert_eq!(
        shows.set_connected(vec!["right".into()]),
        vec!["right".to_owned()]
    );
    assert_eq!(shows.shared_target(), SharedTarget::Desktop);
    shows.advance(&recorder(&applied));
    assert_eq!(applied.borrow().len(), 1);

    let added = shows.set_connected(vec!["left".into(), "right".into()]);
    assert_eq!(added, vec!["left".to_owned()]);
    shows.reapply(&added, &recorder(&applied));
    assert_eq!(applied.borrow().last(), Some(&("left".to_owned(), a)));
}

#[test]
fn monitors_without_images_fall_back_to_the_main_slideshow() {
    let main = tempfile::tempdir().unwrap();
    let left = tempfile::tempdir().unwrap();
    image(&main.path().join("main.png"));
    let mut config = config(main.path(), left.path());
    let mut shows = MonitorShows::default();
    assert!(shows.configure(&config).is_empty());
    shows.set_connected(vec!["left".into()]);
    assert_eq!(shows.shared_target(), SharedTarget::Desktop);

    config.monitor_folders[0].folders[0].enabled = false;
    assert!(!MonitorShows::is_configured(&config));
}

#[test]
fn folder_changes_update_monitor_slideshows() {
    let main = tempfile::tempdir().unwrap();
    let left = tempfile::tempdir().unwrap();
    image(&main.path().join("main.png"));
    let a = left.path().join("a.png");
    let b = left.path().join("b.png");
    image(&a);
    image(&b);
    let config = config(main.path(), left.path());
    let mut shows = MonitorShows::default();
    shows.configure(&config);
    shows.set_connected(vec!["left".into()]);
    let applied = Applied::default();
    shows.advance(&recorder(&applied));
    assert_eq!(shows.current("left"), Some(a.as_path()));

    fs::remove_file(&a).unwrap();
    let changed: HashSet<PathBuf> = [a.clone()].into_iter().collect();
    assert!(shows.apply_file_changes(&config, &changed, &recorder(&applied)));
    assert_eq!(shows.current("left"), Some(b.as_path()));
    assert!(!shows.apply_file_changes(&config, &changed, &recorder(&applied)));
}

#[test]
fn one_monitor_moves_without_the_others() {
    let main = tempfile::tempdir().unwrap();
    let left = tempfile::tempdir().unwrap();
    image(&main.path().join("main.png"));
    let a = left.path().join("a.png");
    let b = left.path().join("b.png");
    image(&a);
    image(&b);
    let config = config(main.path(), left.path());
    let mut shows = MonitorShows::default();
    shows.configure(&config);
    let applied = Applied::default();
    let next = |monitor: &str, path: &Path, _: Option<&[PathBuf]>| {
        recorder(&applied)(monitor, path).map(|()| Vec::new())
    };

    // Not connected yet.
    assert!(shows.advance_one("left", &next).is_err());
    shows.set_connected(vec!["left".into(), "right".into()]);
    // The right monitor shares the main slideshow.
    assert!(shows.advance_one("right", &next).is_err());
    assert!(shows.previous_one("left", &recorder(&applied)).is_err());

    assert_eq!(shows.advance_one("left", &next), Ok(a.clone()));
    assert_eq!(shows.advance_one("left", &next), Ok(b.clone()));
    assert_eq!(
        shows.previous_one("left", &recorder(&applied)),
        Ok(a.clone())
    );
    assert_eq!(shows.current("left"), Some(a.as_path()));
    assert!(
        applied
            .borrow()
            .iter()
            .all(|(monitor, _)| monitor == "left")
    );
    assert_eq!(shows.show("left").unwrap().position(), Some(1));
}

#[test]
fn monitor_folders_are_validated() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = config(dir.path(), dir.path());
    assert!(config.validate().is_ok());
    config.monitor_folders.push(MonitorFolders {
        monitor: "LEFT".into(),
        folders: Vec::new(),
    });
    assert!(config.validate().is_err());
    config.monitor_folders[1].monitor = " ".into();
    assert!(config.validate().is_err());
}

#[test]
fn monitor_names_are_validated() {
    let name = |monitor: &str, name: &str| MonitorName {
        monitor: monitor.into(),
        name: name.into(),
    };
    let mut config = Config {
        monitor_names: vec![name("left", "Desk"), name("right", "Wall")],
        ..Config::default()
    };
    assert!(config.validate().is_ok());

    for invalid in [
        name("LEFT", "Duplicate monitor"),
        name("other", " "),
        name("other", " Padded"),
        name("other", &"x".repeat(41)),
        name(" ", "No monitor"),
    ] {
        config.monitor_names.push(invalid);
        assert!(config.validate().is_err());
        config.monitor_names.pop();
    }
    config.monitor_names.push(name("other", &"จ".repeat(40)));
    assert!(config.validate().is_ok());
}

#[test]
fn monitor_names_default_to_empty_in_older_configs() {
    let config: Config =
        serde_json::from_str(r#"{"version":1,"folders":[],"interval_seconds":600,"shuffle":true}"#)
            .unwrap();
    assert!(config.monitor_names.is_empty());
}
