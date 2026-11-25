use mindswall_core::utilities::{AspectRatio, ImageFilter, Orientation, matches_filter, thumbnail};

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
