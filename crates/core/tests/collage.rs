use mindswall_core::{collage, slideshow::Slideshow};
use std::path::PathBuf;

fn p(name: &str) -> PathBuf {
    PathBuf::from(name)
}

/// Check what every plan must guarantee: tiles stay on the monitor, do not
/// overlap, keep each image's shape, never enlarge an image more than twice,
/// and cover clearly more than the first image alone.
fn assert_valid(plan: &collage::Plan, sizes: &[(u32, u32)], target: (u32, u32)) {
    assert_eq!(plan.tiles.len(), sizes.len());
    for (index, (tile, &(width, height))) in plan.tiles.iter().zip(sizes).enumerate() {
        assert!(tile.width > 0 && tile.height > 0);
        assert!(
            tile.x + tile.width <= target.0,
            "tile {index} leaves the monitor"
        );
        assert!(
            tile.y + tile.height <= target.1,
            "tile {index} leaves the monitor"
        );
        // Rounding may move each edge by half a pixel, so one side may be
        // off by up to two pixels from the other side times the image's shape.
        let shape = f64::from(width) / f64::from(height);
        let off_width = (f64::from(tile.width) - f64::from(tile.height) * shape).abs();
        let off_height = (f64::from(tile.height) - f64::from(tile.width) / shape).abs();
        assert!(
            off_width.min(off_height) <= 2.0,
            "tile {index} is distorted"
        );
        assert!(
            f64::from(tile.height) <= 2.0 * f64::from(height) + 1.0,
            "tile {index} is enlarged too much"
        );
        for other in &plan.tiles[index + 1..] {
            let apart = tile.x + tile.width <= other.x
                || other.x + other.width <= tile.x
                || tile.y + tile.height <= other.y
                || other.y + other.height <= tile.y;
            assert!(apart, "tiles overlap: {tile:?} {other:?}");
        }
    }
    assert!(plan.coverage > 0.0 && plan.coverage <= 1.0);
}

fn sizes_of(
    first: (u32, u32),
    plan: &collage::Plan,
    candidates: &[(PathBuf, (u32, u32))],
) -> Vec<(u32, u32)> {
    std::iter::once(first)
        .chain(
            plan.companions
                .iter()
                .map(|path| candidates.iter().find(|(c, _)| c == path).unwrap().1),
        )
        .collect()
}

#[test]
fn square_images_are_combined_to_fill_the_monitor() {
    let candidates = vec![
        (p("square.jpg"), (1200, 1200)),
        (p("portrait.jpg"), (1080, 1920)),
        (p("landscape.jpg"), (1600, 1200)),
    ];
    let first = (1200, 1200);
    let plan = collage::plan(first, candidates.clone(), (1920, 1080), 4).unwrap();
    assert!(!plan.companions.is_empty());
    // Alone, a square image covers 56% of a 16:9 monitor.
    assert!(plan.coverage >= 0.5625 + 0.15, "coverage {}", plan.coverage);
    assert_valid(&plan, &sizes_of(first, &plan, &candidates), (1920, 1080));
}

#[test]
fn portrait_images_still_combine_and_keep_their_order() {
    let candidates = vec![
        (p("a.jpg"), (948, 2048)),
        (p("b.jpg"), (1170, 1650)),
        (p("c.jpg"), (948, 2048)),
    ];
    let first = (948, 2048);
    let plan = collage::plan(first, candidates.clone(), (1920, 1080), 4).unwrap();
    let order: Vec<usize> = plan
        .companions
        .iter()
        .map(|path| candidates.iter().position(|(c, _)| c == path).unwrap())
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    assert_valid(&plan, &sizes_of(first, &plan, &candidates), (1920, 1080));
}

#[test]
fn images_that_already_fill_the_monitor_are_shown_alone() {
    let candidates = || vec![(p("b.jpg"), (948, 2048)), (p("c.jpg"), (1200, 1200))];
    assert!(collage::plan((1920, 1080), candidates(), (1920, 1080), 4).is_none());
    assert!(collage::plan((2560, 1440), candidates(), (1920, 1080), 4).is_none());
    // Nothing to combine with.
    assert!(collage::plan((1200, 1200), Vec::new(), (1920, 1080), 4).is_none());
}

#[test]
fn small_images_are_not_enlarged_more_than_twice() {
    // A 200 px square may be drawn at most 400 px tall, which cannot cover
    // clearly more of a 1080 px monitor than the image fitted alone.
    let small = || {
        (0..8)
            .map(|i| (p(&format!("{i}.jpg")), (200, 200)))
            .collect::<Vec<_>>()
    };
    assert!(collage::plan((200, 200), small(), (1920, 1080), 4).is_none());
    // Large companions do not help a small first image either way: every
    // tile in a plan respects the limit.
    let mixed = vec![(p("big.jpg"), (2000, 2000)), (p("big2.jpg"), (1500, 2000))];
    if let Some(plan) = collage::plan((400, 400), mixed.clone(), (1920, 1080), 4) {
        assert_valid(&plan, &sizes_of((400, 400), &plan, &mixed), (1920, 1080));
    }
}

#[test]
fn very_long_images_are_not_used_as_companions() {
    let candidates = vec![(p("strip.jpg"), (4000, 500)), (p("tower.jpg"), (300, 2000))];
    assert!(collage::plan((1200, 1200), candidates, (1920, 1080), 4).is_none());
}

#[test]
fn landscape_images_stack_on_a_portrait_monitor() {
    let candidates = vec![(p("b.jpg"), (1920, 1080)), (p("c.jpg"), (1920, 1080))];
    let first = (1920, 1080);
    let plan = collage::plan(first, candidates.clone(), (1080, 1920), 4).unwrap();
    assert!(plan.tiles.iter().all(|tile| tile.x == 0 || tile.x < 10));
    assert_valid(&plan, &sizes_of(first, &plan, &candidates), (1080, 1920));
}

#[test]
fn gaps_separate_tiles() {
    let candidates = vec![(p("b.jpg"), (1200, 1200)), (p("c.jpg"), (1200, 1200))];
    let plan = collage::plan((1200, 1200), candidates, (1920, 1080), 32).unwrap();
    for (index, tile) in plan.tiles.iter().enumerate() {
        for other in &plan.tiles[index + 1..] {
            let horizontal = other
                .x
                .saturating_sub(tile.x + tile.width)
                .max(tile.x.saturating_sub(other.x + other.width));
            let vertical = other
                .y
                .saturating_sub(tile.y + tile.height)
                .max(tile.y.saturating_sub(other.y + other.height));
            assert!(horizontal.max(vertical) >= 31, "{tile:?} {other:?}");
        }
    }
}

#[test]
fn random_sizes_always_give_valid_layouts() {
    // A fixed linear congruential generator keeps the test repeatable.
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = |low: u32, high: u32| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        low + ((seed >> 33) as u32) % (high - low + 1)
    };
    let targets = [
        (1920, 1080),
        (2560, 1440),
        (3440, 1440),
        (1080, 1920),
        (5760, 1080),
        (1280, 1024),
    ];
    let mut combined = 0;
    for round in 0..400 {
        let target = targets[round % targets.len()];
        let first = (next(50, 4000), next(50, 4000));
        let candidates: Vec<(PathBuf, (u32, u32))> = (0..next(0, 12))
            .map(|i| {
                (
                    p(&format!("{round}-{i}.jpg")),
                    (next(50, 4000), next(50, 4000)),
                )
            })
            .collect();
        let gap = next(0, collage::MAX_GAP);
        if let Some(plan) = collage::plan(first, candidates.clone(), target, gap) {
            combined += 1;
            assert!(plan.companions.len() <= 5);
            assert_valid(&plan, &sizes_of(first, &plan, &candidates), target);
        }
    }
    assert!(combined > 50, "only {combined} layouts were combined");
}

#[test]
fn render_draws_a_monitor_sized_image_and_reuses_it() {
    let dir = tempfile::tempdir().unwrap();
    let images: Vec<PathBuf> = (0..3)
        .map(|index| {
            let path = dir.path().join(format!("{index}.png"));
            image::RgbImage::from_pixel(150, 150, image::Rgb([200, 40 * index as u8, 10]))
                .save(&path)
                .unwrap();
            path
        })
        .collect();
    let sizes = [(150, 150); 3];
    let plan = collage::plan(
        sizes[0],
        images[1..].iter().cloned().zip(sizes[1..].iter().copied()),
        (400, 200),
        4,
    )
    .unwrap();
    let used: Vec<PathBuf> = std::iter::once(images[0].clone())
        .chain(plan.companions.clone())
        .collect();
    let cache = dir.path().join("cache");
    let file = collage::render(&used, &plan.tiles, (400, 200), &cache).unwrap();
    assert_eq!(image::image_dimensions(&file).unwrap(), (400, 200));
    assert_eq!(
        collage::render(&used, &plan.tiles, (400, 200), &cache).unwrap(),
        file
    );
    // Tiles that leave the canvas are refused.
    let mut outside = plan.tiles.clone();
    outside[0].x = 399;
    assert!(collage::render(&used, &outside, (400, 200), &cache).is_err());
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
