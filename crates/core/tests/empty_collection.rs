use mindswall_core::{
    config::{Config, Folder},
    playback::Playback,
};
use std::{cell::RefCell, path::PathBuf, time::Instant};

#[test]
fn disabling_last_folder_restores_default_and_reenabling_resumes() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("image.jpg");
    std::fs::write(&image, []).unwrap();
    let mut config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        shuffle: false,
        ..Config::default()
    };
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    let seen = RefCell::new(Vec::<PathBuf>::new());
    let apply = |path: &std::path::Path| {
        seen.borrow_mut().push(path.to_owned());
        Ok(())
    };
    let fallback = || Ok(PathBuf::from("windows-default.jpg"));
    config.folders[0].enabled = false;
    playback
        .reload(&config, Instant::now(), &apply, &fallback)
        .unwrap();
    assert!(playback.show.is_none());
    assert!(playback.timer.is_paused());
    assert_eq!(
        seen.borrow().as_slice(),
        &[PathBuf::from("windows-default.jpg")]
    );
    config.folders[0].enabled = true;
    playback
        .reload(&config, Instant::now(), &apply, &fallback)
        .unwrap();
    assert!(playback.show.is_some());
    assert!(!playback.timer.is_paused());
    assert_eq!(seen.borrow().last().unwrap(), &image);
}

#[test]
fn empty_enabled_folder_does_not_restore_default() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        ..Config::default()
    };
    std::fs::write(dir.path().join("a.jpg"), []).unwrap();
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    config.folders[0].path = dir.path().join("empty");
    std::fs::create_dir(&config.folders[0].path).unwrap();
    let result = playback.reload(&config, Instant::now(), &|_| Ok(()), &|| {
        panic!("must not restore default")
    });
    assert!(result.is_err());
    assert!(playback.show.is_some());
}

#[test]
fn paused_slideshow_remains_paused_after_disabling_and_enabling() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), []).unwrap();
    let mut config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        ..Config::default()
    };
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    playback.timer.pause(Instant::now());
    config.folders[0].enabled = false;
    playback
        .reload(&config, Instant::now(), &|_| Ok(()), &|| {
            Ok("default.jpg".into())
        })
        .unwrap();
    config.folders[0].enabled = true;
    playback
        .reload(&config, Instant::now(), &|_| Ok(()), &|| {
            Ok("default.jpg".into())
        })
        .unwrap();
    assert!(playback.timer.is_paused());
}

#[test]
fn failed_default_change_still_stops_old_slideshow() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), []).unwrap();
    let mut config = Config {
        folders: vec![Folder {
            path: dir.path().to_owned(),
            enabled: true,
            recursive: true,
        }],
        ..Config::default()
    };
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    config.folders[0].enabled = false;
    assert!(
        playback
            .reload(
                &config,
                Instant::now(),
                &|_| Err("cannot set wallpaper".into()),
                &|| Ok("default.jpg".into())
            )
            .is_err()
    );
    assert!(playback.show.is_none());
    assert!(playback.timer.is_paused());
}
