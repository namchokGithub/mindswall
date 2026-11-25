use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn finds_supported_images_in_nested_folders() {
    let root = std::env::temp_dir().join(format!(
        "mindswall-scan-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("nested")).unwrap();
    for name in [
        "one.jpg",
        "nested/two.PNG",
        "nested/three.bmp",
        "four.JPEG",
        "ignore.txt",
    ] {
        fs::write(root.join(name), []).unwrap();
    }
    let mut images = mindswall_core::scanner::scan(std::slice::from_ref(&root), true).unwrap();
    images.sort();
    let mut expected: Vec<_> = ["one.jpg", "nested/two.PNG", "nested/three.bmp", "four.JPEG"]
        .iter()
        .map(|name| root.join(name))
        .collect();
    expected.sort();
    assert_eq!(images, expected);
    let flat = mindswall_core::scanner::scan(std::slice::from_ref(&root), false).unwrap();
    assert_eq!(flat.len(), 2);
    let combined =
        mindswall_core::scanner::scan(&[root.clone(), root.join("nested")], false).unwrap();
    assert_eq!(combined.len(), 4);
    assert!(mindswall_core::random_image(&[]).is_err());
    assert!(images.contains(&mindswall_core::random_image(&images).unwrap().to_owned()));
    assert!(mindswall_core::scanner::scan(&[root.join("missing")], true).is_err());
    fs::remove_dir_all(root).unwrap();
}
