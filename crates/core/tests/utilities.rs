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

#[test]
fn previews_are_downscaled_and_cached() {
    use mindswall_core::utilities::{PREVIEW_SIZE, preview};
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("previews");
    let large = dir.path().join("large.png");
    image::RgbImage::new(4000, 3000).save(&large).unwrap();
    let (file, cached) = preview(&large, &cache).unwrap();
    assert!(!cached);
    let (width, height) = image::image_dimensions(&file).unwrap();
    assert!(width <= PREVIEW_SIZE.0 && height <= PREVIEW_SIZE.1);
    assert_eq!((width, height), (1200, 900));
    // Using a cached preview marks it as recent, so cache cleaning keeps it.
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert_eq!(preview(&large, &cache).unwrap(), (file.clone(), true));
    let used = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert!(used > old + std::time::Duration::from_secs(60));

    // Small images keep their size.
    let small = dir.path().join("small.png");
    image::RgbImage::new(300, 200).save(&small).unwrap();
    let (file, _) = preview(&small, &cache).unwrap();
    assert_eq!(image::image_dimensions(&file).unwrap(), (300, 200));
}

#[test]
fn jpeg_images_decode_at_a_reduced_size_when_shown_small() {
    use mindswall_core::utilities::open_scaled;
    let dir = tempfile::tempdir().unwrap();
    let jpeg = dir.path().join("photo.jpg");
    image::RgbImage::from_pixel(4000, 3000, image::Rgb([120, 80, 40]))
        .save(&jpeg)
        .unwrap();
    // A thumbnail needs 200 × 150, so 1/8 (500 × 375) is enough.
    let small = open_scaled(&jpeg, (240, 150)).unwrap();
    assert_eq!((small.width(), small.height()), (500, 375));
    // A 1600 × 900 preview needs 1200 × 900, so 1/2 (2000 × 1500) is used.
    let preview = open_scaled(&jpeg, (1600, 900)).unwrap();
    assert_eq!((preview.width(), preview.height()), (2000, 1500));
    // The colour survives scaled decoding.
    let pixel = small.to_rgb8().get_pixel(250, 180).0;
    assert!(
        pixel
            .iter()
            .zip([120u8, 80, 40])
            .all(|(a, b)| a.abs_diff(b) <= 3),
        "{pixel:?}"
    );

    // Other formats, and requests at least as large as the image, are full size.
    let png = dir.path().join("picture.png");
    image::RgbImage::new(400, 300).save(&png).unwrap();
    let full = open_scaled(&png, (40, 30)).unwrap();
    assert_eq!((full.width(), full.height()), (400, 300));
    let full = open_scaled(&jpeg, (8000, 6000)).unwrap();
    assert_eq!((full.width(), full.height()), (4000, 3000));

    // A broken JPEG reports an error instead of panicking.
    let broken = dir.path().join("broken.jpg");
    std::fs::write(&broken, b"not a jpeg").unwrap();
    assert!(open_scaled(&broken, (240, 150)).is_err());
}

#[test]
fn resize_rgb_scales_to_the_exact_size_and_keeps_colours() {
    use mindswall_core::utilities::resize_rgb;
    let mut resizer = fast_image_resize::Resizer::new();
    let filter = fast_image_resize::FilterType::CatmullRom;
    let source = image::RgbImage::from_pixel(300, 200, image::Rgb([200, 100, 50]));

    let down = resize_rgb(&mut resizer, source.clone().into(), (75, 40), filter).unwrap();
    assert_eq!(down.dimensions(), (75, 40));
    let up = resize_rgb(&mut resizer, source.clone().into(), (600, 450), filter).unwrap();
    assert_eq!(up.dimensions(), (600, 450));
    for image in [&down, &up] {
        let pixel = image.get_pixel(image.width() / 2, image.height() / 2).0;
        assert!(
            pixel
                .iter()
                .zip([200u8, 100, 50])
                .all(|(a, b)| a.abs_diff(b) <= 1),
            "{pixel:?}"
        );
    }

    // Grayscale images become RGB; the same size is returned unchanged.
    let gray = image::GrayImage::from_pixel(10, 10, image::Luma([90]));
    let same = resize_rgb(&mut resizer, gray.into(), (10, 10), filter).unwrap();
    assert_eq!(same.get_pixel(5, 5).0, [90, 90, 90]);
}
