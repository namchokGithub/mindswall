use mindswall_core::{collage, slideshow::Slideshow};
use std::path::PathBuf;

fn p(name: &str) -> PathBuf {
    PathBuf::from(name)
}

#[test]
fn portrait_images_fill_a_landscape_monitor_in_cycle_order() {
    let candidates = vec![
        (p("narrow.jpg"), (948, 2048)),
        (p("landscape.jpg"), (1920, 1080)),
        (p("wide-portrait.jpg"), (1170, 1650)),
        (p("too-many.jpg"), (948, 2048)),
    ];
    // 500 + 4 + 500 + 4 + 766 = 1774 fits in 1920; another 500 does not.
    assert_eq!(
        collage::plan((948, 2048), candidates, (1920, 1080), 4),
        vec![p("narrow.jpg"), p("wide-portrait.jpg")]
    );
}

#[test]
fn only_portrait_images_on_landscape_monitors_are_combined() {
    let candidates = || vec![(p("b.jpg"), (948, 2048))];
    assert!(collage::plan((1920, 1080), candidates(), (1920, 1080), 4).is_empty());
    assert!(collage::plan((948, 2048), candidates(), (1080, 1920), 4).is_empty());
    // 511 + 4 + 334 = 849 does not fit in 800.
    assert!(collage::plan((1170, 1650), candidates(), (800, 720), 4).is_empty());
    // A wider gap can leave no room for a second image: 710 + gap + 710.
    let wide = || vec![(p("c.jpg"), (1170, 1650))];
    assert_eq!(
        collage::plan((1170, 1650), wide(), (1430, 1000), 4).len(),
        1
    );
    assert!(collage::plan((1170, 1650), wide(), (1430, 1000), 32).is_empty());
}

#[test]
fn render_draws_a_monitor_sized_image_and_reuses_it() {
    let dir = tempfile::tempdir().unwrap();
    let images: Vec<PathBuf> = (0..3)
        .map(|index| {
            let path = dir.path().join(format!("{index}.png"));
            image::RgbImage::from_pixel(30, 60, image::Rgb([200, 40 * index as u8, 10]))
                .save(&path)
                .unwrap();
            path
        })
        .collect();
    let cache = dir.path().join("cache");
    let file = collage::render(&images, (400, 200), 4, &cache).unwrap();
    assert_eq!(image::image_dimensions(&file).unwrap(), (400, 200));
    assert_eq!(
        collage::render(&images, (400, 200), 4, &cache).unwrap(),
        file
    );
    assert_ne!(
        collage::render(&images, (400, 200), 8, &cache).unwrap(),
        file
    );
}

#[test]
fn combined_images_count_as_played_in_the_cycle() {
    let mut show = Slideshow::new(["a", "b", "c", "d", "e"].map(p).to_vec(), false).unwrap();
    let mut seen = Vec::new();
    let first = show
        .next_available_with(|path, upcoming| {
            seen = upcoming.to_vec();
            assert_eq!(path, p("a"));
            Ok::<_, ()>(vec![p("c"), p("d")])
        })
        .unwrap();
    assert_eq!(first, p("a"));
    assert_eq!(seen, ["b", "c", "d", "e"].map(p).to_vec());
    assert_eq!(show.next().to_owned(), p("b"));
    assert_eq!(show.position(), Some(4));
    assert_eq!(show.next().to_owned(), p("e"));
    // A new cycle starts with every image again.
    assert_eq!(show.next().to_owned(), p("a"));

    // Moving forward through history after Previous offers no upcoming images.
    show.previous();
    show.next_available_with(|_, upcoming| {
        assert!(upcoming.is_empty());
        Ok::<_, ()>(Vec::new())
    })
    .unwrap();
}

#[test]
fn collage_gap_is_validated() {
    let mut config = mindswall_core::config::Config::default();
    assert!(!config.collage.enabled);
    assert_eq!(config.collage.gap, 4);
    config.collage.gap = collage::MAX_GAP;
    assert!(config.validate().is_ok());
    config.collage.gap = collage::MAX_GAP + 1;
    assert!(config.validate().is_err());
}
