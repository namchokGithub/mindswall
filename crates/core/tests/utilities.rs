use mindswall_core::utilities::{AspectRatio, ImageFilter, Orientation, matches_filter, thumbnail};

#[test]
fn image_properties_report_actual_dimensions_bit_depth_and_file_size() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("picture.png");
    image::RgbImage::new(40, 25).save(&path).unwrap();
    let info = mindswall_core::utilities::image_properties(&path).unwrap();
    assert_eq!((info.width, info.height), (40, 25));
    assert_eq!(info.bit_depth, 24);
    assert_eq!(info.size, std::fs::metadata(&path).unwrap().len());
    assert!(mindswall_core::utilities::image_properties(&dir.path().join("missing.png")).is_err());
}

#[test]
fn filters_dimensions_and_orientation() {
    let filter = ImageFilter {
        min_width: 1920,
        min_height: 1080,
        orientation: Orientation::Landscape,
        ..ImageFilter::default()
    };
    assert!(matches_filter(2560, 1440, &filter));
    assert!(!matches_filter(1920, 900, &filter));
    assert!(!matches_filter(1920, 2560, &filter));
    let ratio = ImageFilter {
        aspect_ratio: AspectRatio::Wide,
        ..ImageFilter::default()
    };
    assert!(matches_filter(1920, 1080, &ratio));
    assert!(!matches_filter(1600, 1200, &ratio));
}

#[test]
fn creates_thumbnail_and_reuses_cache() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("image.png");
    image::RgbImage::new(400, 200).save(&image).unwrap();
    let cache = dir.path().join("cache");
    let thumb = thumbnail(&image, &cache).unwrap();
    assert!(thumb.exists());
    let (width, height) = image::image_dimensions(&thumb).unwrap();
    assert_eq!((width, height), (240, 120));
    assert_eq!(thumbnail(&image, &cache).unwrap(), thumb);
}

#[test]
fn dimensions_refresh_after_image_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("image.png");
    image::RgbImage::new(20, 10).save(&path).unwrap();
    assert_eq!(
        mindswall_core::utilities::dimensions(&path).unwrap(),
        (20, 10)
    );
    image::RgbImage::new(150, 80).save(&path).unwrap();
    assert_eq!(
        mindswall_core::utilities::dimensions(&path).unwrap(),
        (150, 80)
    );
}

#[test]
fn tail_lines_reads_only_the_last_lines() {
    use mindswall_core::utilities::tail_lines;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");
    assert!(tail_lines(&path, 100).unwrap().is_empty());

    // Long enough to need several chunks.
    let text: String = (1..=5000)
        .map(|n| format!("line {n} {}\n", "x".repeat(40)))
        .collect();
    std::fs::write(&path, &text).unwrap();
    let lines = tail_lines(&path, 100).unwrap();
    assert_eq!(lines.len(), 100);
    assert!(lines[0].starts_with("line 4901 "));
    assert!(lines[99].starts_with("line 5000 "));

    std::fs::write(&path, "first\nsecond\nlast without newline").unwrap();
    assert_eq!(
        tail_lines(&path, 100).unwrap(),
        ["first", "second", "last without newline"]
    );
    assert_eq!(tail_lines(&path, 1).unwrap(), ["last without newline"]);
}
