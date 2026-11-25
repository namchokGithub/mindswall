use mindswall_core::editor::{Crop, EditOptions, render};

#[test]
fn edits_a_copy_and_keeps_original_intact() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.png");
    image::RgbImage::from_pixel(20, 10, image::Rgb([200, 120, 80]))
        .save(&original)
        .unwrap();
    let before = std::fs::read(&original).unwrap();
    let options = EditOptions {
        crop: Some(Crop {
            x: 2,
            y: 1,
            width: 10,
            height: 8,
        }),
        width: Some(5),
        height: Some(4),
        blur: 0.0,
        dim: 50,
    };
    let output = render(&original, &dir.path().join("cache"), &options).unwrap();
    assert_ne!(output, original);
    assert_eq!(std::fs::read(original).unwrap(), before);
    assert_eq!(image::image_dimensions(&output).unwrap(), (5, 4));
    let pixel = image::open(output).unwrap().to_rgb8().get_pixel(0, 0).0;
    assert!(pixel[0] < 200 && pixel[0] > 0);
}

#[test]
fn rejects_invalid_crop_and_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.png");
    image::RgbImage::new(20, 10).save(&original).unwrap();
    let options = EditOptions {
        crop: Some(Crop {
            x: 19,
            y: 0,
            width: 3,
            height: 2,
        }),
        ..EditOptions::default()
    };
    assert!(render(&original, dir.path(), &options).is_err());
}
