//! Combined wallpapers: an image that leaves much of a monitor empty is shown
//! with other images in rows, like a mosaic, over a blurred copy of it.

use crate::config::{CollageLayout, CollageSettings};
use image::{DynamicImage, ImageEncoder, RgbImage, imageops::FilterType};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

/// Largest allowed space between images, in pixels.
pub const MAX_GAP: u32 = 32;
/// Most images in one combined wallpaper, including the first.
const MAX_IMAGES: usize = 6;
/// How many unplayed images are checked for companions.
const SCAN_LIMIT: usize = 30;
/// How many usable unplayed images the layout may choose from.
const POOL: usize = 8;
/// Images are never enlarged more than this, so small images do not blur.
const MAX_UPSCALE: f64 = 2.0;
/// The first image is combined only when, fitted alone, it covers less than
/// this share of the monitor.
const COMBINE_BELOW: f64 = 0.8;
/// A layout must cover at least this much more of the monitor than the first
/// image alone.
const MIN_GAIN: f64 = 0.15;
/// Score taken off for each extra image, so fewer images win a near tie.
const IMAGE_COST: f64 = 0.02;
/// Companions wider or taller than this many times their other side are
/// skipped, because they would become thin strips.
const MAX_SHAPE: f64 = 4.0;
/// Combined wallpapers kept on disk and remembered for Previous.
const KEEP: usize = 10;

/// Where one image is drawn on the combined wallpaper, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The images chosen to show next to the first one, and where every image
/// goes: `tiles[0]` is the first image, then the companions in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub companions: Vec<PathBuf>,
    pub tiles: Vec<Tile>,
    /// The share of the monitor covered by images, from 0 to 1.
    pub coverage: f64,
}

fn ratio((width, height): (u32, u32)) -> f64 {
    f64::from(width) / f64::from(height)
}

/// The share of `target` that `size` covers when fitted alone.
fn coverage_alone(size: (u32, u32), target: (u32, u32)) -> f64 {
    let (image, screen) = (ratio(size), ratio(target));
    image.min(screen) / image.max(screen)
}

/// Every way to split `count` images, in order, into one to three rows. Each entry lists the number of images per row.
fn row_splits(count: usize) -> Vec<Vec<usize>> {
    let mut splits = vec![vec![count]];
    for first in 1..count {
        splits.push(vec![first, count - first]);
        for second in 1..count - first {
            splits.push(vec![first, second, count - first - second]);
        }
    }
    splits
}

/// Arrange `sizes` in rows of `rows` images: every image in a row has the
/// same height and each row spans the monitor's width. When the rows are
/// taller than the monitor, everything is scaled down to fit. The result is
/// centered. Returns the covered share and the tiles, or `None` when an image
/// would be enlarged more than [`MAX_UPSCALE`] or there is no room.
fn layout(
    sizes: &[(u32, u32)],
    rows: &[usize],
    (width, height): (u32, u32),
    gap: u32,
) -> Option<(f64, Vec<Tile>)> {
    let (screen_width, screen_height, gap) = (f64::from(width), f64::from(height), f64::from(gap));
    let mut row_heights = Vec::with_capacity(rows.len());
    let mut start = 0;
    for &count in rows {
        let group = &sizes[start..start + count];
        let room = screen_width - gap * (count - 1) as f64;
        if room <= 0.0 {
            return None;
        }
        row_heights.push(room / group.iter().map(|&size| ratio(size)).sum::<f64>());
        start += count;
    }
    let row_gaps = gap * (rows.len() - 1) as f64;
    if row_gaps >= screen_height {
        return None;
    }
    let total: f64 = row_heights.iter().sum();
    let scale = if total + row_gaps > screen_height {
        (screen_height - row_gaps) / total
    } else {
        1.0
    };
    let mut y = (screen_height - (total * scale + row_gaps)) / 2.0;
    let mut tiles = Vec::with_capacity(sizes.len());
    let mut covered = 0.0;
    let mut start = 0;
    for (&count, &row_height) in rows.iter().zip(&row_heights) {
        let row_height = row_height * scale;
        let group = &sizes[start..start + count];
        let row_width = group
            .iter()
            .map(|&size| ratio(size) * row_height)
            .sum::<f64>()
            + gap * (count - 1) as f64;
        let mut x = (screen_width - row_width) / 2.0;
        for &size in group {
            if row_height > MAX_UPSCALE * f64::from(size.1) {
                return None;
            }
            let tile_width = ratio(size) * row_height;
            // Rounding both edges keeps neighbouring tiles from overlapping.
            let (left, right) = (x.round(), (x + tile_width).round());
            let (top, bottom) = (y.round(), (y + row_height).round());
            if right - left < 1.0 || bottom - top < 1.0 {
                return None;
            }
            tiles.push(Tile {
                x: left as u32,
                y: top as u32,
                width: (right - left) as u32,
                height: (bottom - top) as u32,
            });
            covered += tile_width * row_height;
            x += tile_width + gap;
        }
        y += row_height + gap;
        start += count;
    }
    Some((covered / (screen_width * screen_height), tiles))
}

/// Choose images to show with `first` on a `target` sized monitor and lay
/// them out in rows, like a mosaic.
///
/// `first` is combined only when, fitted alone, it leaves much of the
/// monitor empty. Companions are chosen from the first usable candidates
/// (no more than four times as wide as tall or the other way round), keeping
/// their order; images that are not chosen
/// stay in the cycle. The layout that covers the most of the monitor wins,
/// with a small cost per extra image. No image is enlarged more than twice.
/// Returns `None` when no layout covers clearly more than `first` alone.
pub fn plan(
    first: (u32, u32),
    candidates: impl IntoIterator<Item = (PathBuf, (u32, u32))>,
    target: (u32, u32),
    gap: u32,
) -> Option<Plan> {
    let valid = |(width, height): (u32, u32)| width > 0 && height > 0;
    if !valid(first) || !valid(target) {
        return None;
    }
    let alone = coverage_alone(first, target);
    if alone >= COMBINE_BELOW {
        return None;
    }
    let pool: Vec<(PathBuf, (u32, u32))> = candidates
        .into_iter()
        .take(SCAN_LIMIT)
        .filter(|(_, size)| valid(*size) && (1.0 / MAX_SHAPE..=MAX_SHAPE).contains(&ratio(*size)))
        .take(POOL)
        .collect();
    let mut best: Option<(f64, Vec<usize>, f64, Vec<Tile>)> = None;
    for mask in 1u32..(1 << pool.len()) {
        let chosen: Vec<usize> = (0..pool.len()).filter(|i| mask & (1 << i) != 0).collect();
        if chosen.len() + 1 > MAX_IMAGES {
            continue;
        }
        let sizes: Vec<(u32, u32)> = std::iter::once(first)
            .chain(chosen.iter().map(|&i| pool[i].1))
            .collect();
        for rows in row_splits(sizes.len()) {
            let Some((coverage, tiles)) = layout(&sizes, &rows, target, gap) else {
                continue;
            };
            let score = coverage - IMAGE_COST * chosen.len() as f64;
            if coverage >= alone + MIN_GAIN && best.as_ref().is_none_or(|best| score > best.0) {
                best = Some((score, chosen.clone(), coverage, tiles));
            }
        }
    }
    let (_, chosen, coverage, tiles) = best?;
    Some(Plan {
        companions: chosen.into_iter().map(|i| pool[i].0.clone()).collect(),
        tiles,
        coverage,
    })
}

/// The full-height width of an image on a monitor `target_height` tall, rounded up.
fn width_at((width, height): (u32, u32), target_height: u32) -> u32 {
    (u64::from(width) * u64::from(target_height)).div_ceil(u64::from(height.max(1))) as u32
}

/// The side-by-side layout: only a portrait image on a landscape monitor is
/// combined. Candidates are taken in order at full monitor height; images
/// that are not portrait are skipped (they stay in the cycle), and the search
/// stops at the first portrait image that no longer fits, with at most five
/// companions. The row is centered. Returns `None` when nothing fits next to
/// `first`.
pub fn plan_row(
    first: (u32, u32),
    candidates: impl IntoIterator<Item = (PathBuf, (u32, u32))>,
    target: (u32, u32),
    gap: u32,
) -> Option<Plan> {
    let portrait = |(width, height): (u32, u32)| width > 0 && height > width;
    let (target_width, target_height) = target;
    if target_width <= target_height || !portrait(first) {
        return None;
    }
    let mut widths = vec![width_at(first, target_height).min(target_width)];
    let mut used = widths[0];
    let mut companions = Vec::new();
    for (path, size) in candidates.into_iter().take(SCAN_LIMIT) {
        if !portrait(size) {
            continue;
        }
        let width = width_at(size, target_height);
        if used + gap + width > target_width {
            break;
        }
        used += gap + width;
        widths.push(width);
        companions.push(path);
        if companions.len() + 1 == MAX_IMAGES {
            break;
        }
    }
    if companions.is_empty() {
        return None;
    }
    let mut x = (target_width - used) / 2;
    let tiles = widths
        .iter()
        .map(|&width| {
            let tile = Tile {
                x,
                y: 0,
                width,
                height: target_height,
            };
            x += width + gap;
            tile
        })
        .collect();
    let covered: u64 = widths.iter().map(|&width| u64::from(width)).sum();
    Some(Plan {
        companions,
        tiles,
        coverage: covered as f64 / f64::from(target_width),
    })
}

/// Draw `images` at their `tiles` over a blurred, darkened copy of the first
/// image on a `target` sized canvas, and save it as a JPEG in `cache`. The
/// same inputs reuse the existing file.
pub fn render(
    images: &[PathBuf],
    tiles: &[Tile],
    target: (u32, u32),
    cache: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let (width, height) = target;
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "Invalid combined wallpaper");
    if images.is_empty()
        || images.len() != tiles.len()
        || width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 64_000_000
        || tiles.iter().any(|tile| {
            tile.width == 0
                || tile.height == 0
                || u64::from(tile.x) + u64::from(tile.width) > u64::from(width)
                || u64::from(tile.y) + u64::from(tile.height) > u64::from(height)
        })
    {
        return Err(invalid().into());
    }
    let mut hash = DefaultHasher::new();
    for path in images {
        path.hash(&mut hash);
        let meta = fs::metadata(path)?;
        meta.len().hash(&mut hash);
        meta.modified()?.hash(&mut hash);
    }
    (width, height, tiles).hash(&mut hash);
    let file = cache.join(format!("{:016x}.jpg", hash.finish()));
    if file.is_file() {
        return Ok(file);
    }

    // Each image is only needed at its tile's size.
    let decoded = images
        .iter()
        .zip(tiles)
        .map(|(path, tile)| crate::utilities::open_scaled(path, (tile.width, tile.height)))
        .collect::<Result<Vec<DynamicImage>, _>>()?;
    let mut canvas = background(&decoded[0], target);
    for (image, tile) in decoded.iter().zip(tiles) {
        let scaled = image
            .resize_exact(tile.width, tile.height, FilterType::CatmullRom)
            .to_rgb8();
        image::imageops::overlay(&mut canvas, &scaled, i64::from(tile.x), i64::from(tile.y));
    }

    fs::create_dir_all(cache)?;
    let mut temp = tempfile::NamedTempFile::new_in(cache)?;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut temp, 90).write_image(
        canvas.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )?;
    temp.persist(&file).map_err(|e| io::Error::other(e.error))?;
    Ok(file)
}

/// A blurred, darkened copy of `image` that covers `target`. Blurring a
/// small copy and scaling it up keeps this fast for large monitors.
fn background(image: &DynamicImage, (width, height): (u32, u32)) -> RgbImage {
    let small = image
        .resize_to_fill(
            (width / 16).max(1),
            (height / 16).max(1),
            FilterType::Triangle,
        )
        .blur(3.0);
    let mut canvas = small
        .resize_exact(width, height, FilterType::Triangle)
        .to_rgb8();
    for pixel in canvas.pixels_mut() {
        for channel in &mut pixel.0 {
            *channel = (u16::from(*channel) * 65 / 100) as u8;
        }
    }
    canvas
}

/// A combined wallpaper that was shown, so Previous and reapplying can show
/// the same combination again.
struct Shown {
    target: String,
    first: PathBuf,
    combined: Vec<PathBuf>,
    file: PathBuf,
}

/// The combined wallpaper to show for an image.
pub struct Combined {
    pub file: PathBuf,
    pub combined: Vec<PathBuf>,
}

/// Builds and remembers combined wallpapers for the slideshow service.
pub struct Collages {
    settings: Cell<CollageSettings>,
    span: Cell<bool>,
    cache: Option<PathBuf>,
    shown: RefCell<VecDeque<Shown>>,
}

impl Collages {
    pub fn new(config: &crate::config::Config) -> Self {
        Self {
            settings: Cell::new(config.collage),
            span: Cell::new(config.fit_mode == crate::utilities::FitMode::Span),
            cache: directories::ProjectDirs::from("", "", "MindsWall")
                .map(|dirs| dirs.cache_dir().join("collages")),
            shown: RefCell::new(VecDeque::new()),
        }
    }

    /// Like [`Collages::new`], with combined wallpapers kept in `cache`.
    pub fn with_cache(config: &crate::config::Config, cache: PathBuf) -> Self {
        Self {
            cache: Some(cache),
            ..Self::new(config)
        }
    }

    /// Use the settings of a reloaded configuration. Remembered combinations
    /// are dropped when the collage settings or the fit mode change.
    pub fn configure(&self, config: &crate::config::Config) {
        let span = config.fit_mode == crate::utilities::FitMode::Span;
        if config.collage != self.settings.get() || span != self.span.get() {
            self.shown.borrow_mut().clear();
        }
        self.settings.set(config.collage);
        self.span.set(span);
    }

    pub fn enabled(&self) -> bool {
        self.settings.get().enabled && self.cache.is_some()
    }

    /// Whether the fit mode spans one image across all monitors.
    pub fn span(&self) -> bool {
        self.span.get()
    }

    /// The combined wallpaper for `first` on the monitor named `target`.
    /// With `upcoming` images (possibly none), a new combination is planned
    /// from them; with `None`, the last combination shown for `first` is
    /// reused. Returns `None` when `first` should be shown alone. `size` is
    /// called only when a new combination is planned.
    pub fn combine(
        &self,
        target: &str,
        first: &Path,
        upcoming: Option<&[PathBuf]>,
        size: impl FnOnce() -> Option<(u32, u32)>,
    ) -> Option<Combined> {
        if !self.enabled() {
            return None;
        }
        let Some(upcoming) = upcoming else {
            return self.remembered(target, first);
        };
        let combined = self.combine_new(target, first, upcoming, size);
        if combined.is_none() {
            // `first` is shown alone now, so an older combination must not
            // be reported or shown again for it.
            self.forget(target, first);
        }
        combined
    }

    /// Plan, render, and remember a new combination; see [`Collages::combine`].
    fn combine_new(
        &self,
        target: &str,
        first: &Path,
        upcoming: &[PathBuf],
        size: impl FnOnce() -> Option<(u32, u32)>,
    ) -> Option<Combined> {
        let elapsed_ms = crate::utilities::elapsed_ms;
        let started = std::time::Instant::now();
        let first_size = crate::utilities::dimensions(first).ok()?;
        let size = size()?;
        let monitor_ms = elapsed_ms(started);
        let candidates: Vec<(PathBuf, (u32, u32))> = upcoming
            .iter()
            .filter(|path| path.as_path() != first)
            .take(SCAN_LIMIT)
            .filter_map(|path| Some((path.clone(), crate::utilities::dimensions(path).ok()?)))
            .collect();
        let sizes_ms = elapsed_ms(started) - monitor_ms;
        let settings = self.settings.get();
        let planned = match settings.layout {
            CollageLayout::Row => plan_row(first_size, candidates, size, settings.gap),
            CollageLayout::Mosaic => plan(first_size, candidates, size, settings.gap),
        };
        let plan_ms = elapsed_ms(started) - monitor_ms - sizes_ms;
        let Some(plan) = planned else {
            tracing::info!(
                target,
                monitor_ms,
                sizes_ms,
                plan_ms,
                "showing one image; nothing to combine"
            );
            return None;
        };
        let render_started = std::time::Instant::now();
        let combined = plan.companions;
        let images: Vec<PathBuf> = std::iter::once(first.to_owned())
            .chain(combined.iter().cloned())
            .collect();
        let cache = self.cache.as_ref()?;
        let file = match render(&images, &plan.tiles, size, cache) {
            Ok(file) => {
                tracing::info!(
                    target,
                    companions = combined.len(),
                    monitor_ms,
                    sizes_ms,
                    plan_ms,
                    render_ms = elapsed_ms(render_started),
                    ram_mb = crate::utilities::ram_mb(),
                    peak_mb = crate::utilities::peak_ram_mb(),
                    "combined wallpaper prepared"
                );
                file
            }
            Err(error) => {
                tracing::warn!(%error, "could not combine images; showing one image");
                return None;
            }
        };
        self.remember(Shown {
            target: target.to_owned(),
            first: first.to_owned(),
            combined: combined.clone(),
            file: file.clone(),
        });
        Some(Combined { file, combined })
    }

    /// The combination last shown for `first` on `target`, if its file still exists.
    pub fn remembered(&self, target: &str, first: &Path) -> Option<Combined> {
        if !self.enabled() {
            return None;
        }
        self.shown
            .borrow()
            .iter()
            .rev()
            .find(|shown| shown.target == target && shown.first == first && shown.file.is_file())
            .map(|shown| Combined {
                file: shown.file.clone(),
                combined: shown.combined.clone(),
            })
    }

    fn forget(&self, target: &str, first: &Path) {
        self.shown
            .borrow_mut()
            .retain(|entry| !(entry.target == target && entry.first == first));
    }

    fn remember(&self, shown: Shown) {
        let mut list = self.shown.borrow_mut();
        list.retain(|entry| !(entry.target == shown.target && entry.first == shown.first));
        list.push_back(shown);
        // Drop the oldest entries beyond KEEP, but never the newest entry of a
        // monitor: its file may be the wallpaper on screen right now.
        while list.len() > KEEP {
            let replaced = (0..list.len()).find(|&index| {
                list.iter()
                    .skip(index + 1)
                    .any(|later| later.target == list[index].target)
            });
            let Some(index) = replaced else {
                break;
            };
            list.remove(index);
        }
        let keep = list
            .iter()
            .filter_map(|entry| entry.file.file_name().map(ToOwned::to_owned))
            .collect();
        if let Some(cache) = &self.cache
            && let Err(error) = crate::utilities::prune_cache(cache, KEEP, u64::MAX, &keep)
        {
            tracing::warn!(%error, "could not clean up combined wallpapers");
        }
    }
}
