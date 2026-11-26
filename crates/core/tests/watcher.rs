use mindswall_core::{
    config::{Config, Folder},
    playback::Playback,
};
use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

fn image(path: &Path) {
    image::RgbImage::new(4, 3).save(path).unwrap();
}

fn config(root: &Path, recursive: bool) -> Config {
    Config {
        folders: vec![Folder {
            path: root.to_owned(),
            enabled: true,
            recursive,
        }],
        shuffle: false,
        ..Config::default()
    }
}

fn changed(paths: &[PathBuf]) -> HashSet<PathBuf> {
    paths.iter().cloned().collect()
}

#[test]
fn added_and_removed_files_update_the_collection() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    image(&a);
    image(&b);
    let config = config(dir.path(), true);
    let current = config.images().unwrap();
    fs::remove_file(&a).unwrap();
    let c = dir.path().join("c.png");
    image(&c);
    let updated = config.apply_changes(&current, &changed(&[a, c.clone()]));
    assert_eq!(updated, vec![b, c]);
}

#[test]
fn files_still_being_written_and_other_files_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let partial = dir.path().join("partial.jpg");
    fs::write(&partial, []).unwrap();
    let text = dir.path().join("notes.txt");
    fs::write(&text, "x").unwrap();
    let config = config(dir.path(), true);
    assert!(
        config
            .apply_changes(&[], &changed(&[partial, text]))
            .is_empty()
    );
}

#[test]
fn folder_events_respect_recursion_exclusions_and_removal() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let inside = nested.join("inside.png");
    let excluded = nested.join("excluded.png");
    image(&inside);
    image(&excluded);

    let mut recursive = config(dir.path(), true);
    recursive.excluded.push(excluded);
    let added = recursive.apply_changes(&[], &changed(std::slice::from_ref(&nested)));
    assert_eq!(added, vec![inside.clone()]);

    let flat = config(dir.path(), false);
    assert!(
        flat.apply_changes(&[], &changed(std::slice::from_ref(&nested)))
            .is_empty()
    );

    fs::remove_dir_all(&nested).unwrap();
    assert!(
        recursive
            .apply_changes(&added, &changed(std::slice::from_ref(&nested)))
            .is_empty()
    );
}

#[test]
fn file_changes_keep_the_current_wallpaper_unless_it_was_removed() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    image(&a);
    image(&b);
    let config = config(dir.path(), true);
    let mut playback = Playback::new(&config, Instant::now()).unwrap();
    assert_eq!(playback.show.as_mut().unwrap().next(), a);
    let applied = RefCell::new(Vec::<PathBuf>::new());
    let apply = |path: &Path| {
        applied.borrow_mut().push(path.to_owned());
        Ok(())
    };

    let c = dir.path().join("c.png");
    image(&c);
    assert!(
        playback
            .apply_file_changes(
                &config,
                &changed(std::slice::from_ref(&c)),
                Instant::now(),
                &apply
            )
            .unwrap()
    );
    assert!(applied.borrow().is_empty());
    assert_eq!(playback.show.as_ref().unwrap().total(), 3);

    assert!(
        !playback
            .apply_file_changes(
                &config,
                &changed(std::slice::from_ref(&c)),
                Instant::now(),
                &apply
            )
            .unwrap()
    );

    fs::remove_file(&a).unwrap();
    assert!(
        playback
            .apply_file_changes(
                &config,
                &changed(std::slice::from_ref(&a)),
                Instant::now(),
                &apply
            )
            .unwrap()
    );
    assert_eq!(applied.borrow().len(), 1);
    assert_ne!(applied.borrow()[0], a);
    assert_eq!(playback.show.as_ref().unwrap().total(), 2);
}
