//! Combined wallpapers: portrait images placed side by side on a landscape
//! monitor, over a blurred copy of the first image.

use crate::config::CollageSettings;
use image::{DynamicImage, GenericImageView, ImageEncoder, RgbImage, imageops::FilterType};
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
/// Most images shown next to the first one.
const MAX_COMPANIONS: usize = 5;
/// How many unplayed images are checked for companions.
const SCAN_LIMIT: usize = 30;
/// Combined wallpapers kept on disk and remembered for Previous.
const KEEP: usize = 10;

fn is_portrait((width, height): (u32, u32)) -> bool {
    width > 0 && height > width
}

/// Width of an image scaled to `height`, rounded up.
fn width_at((width, height): (u32, u32), target_height: u32) -> u32 {
    (u64::from(width) * u64::from(target_height)).div_ceil(u64::from(height.max(1))) as u32
}

/// Choose the images to show next to `first` on a `target` sized monitor.
///
/// Only a portrait image on a landscape monitor is combined. Candidates are
/// taken in order: landscape ones are skipped (they stay in the cycle), and
/// the search stops at the first portrait image that no longer fits. Returns
/// an empty list when nothing fits next to `first`.
pub fn plan(
    first: (u32, u32),
    candidates: impl IntoIterator<Item = (PathBuf, (u32, u32))>,
    target: (u32, u32),
    gap: u32,
) -> Vec<PathBuf> {
    let (target_width, target_height) = target;
    if target_width <= target_height || !is_portrait(first) {
        return Vec::new();
    }
    let mut used = width_at(first, target_height);
    let mut chosen = Vec::new();
    for (path, size) in candidates.into_iter().take(SCAN_LIMIT) {
        if !is_portrait(size) {
            continue;
        }
        let width = width_at(size, target_height);
        if used + gap + width > target_width {
            break;
        }
        used += gap + width;
        chosen.push(path);
        if chosen.len() == MAX_COMPANIONS {
            break;
        }
    }
    chosen
}

/// Draw `images` side by side at full height, centered on a `target` sized
/// canvas over a blurred, darkened copy of the first image, and save it as a
/// JPEG in `cache`. The same inputs reuse the existing file.
pub fn render(
    images: &[PathBuf],
    target: (u32, u32),
    gap: u32,
    cache: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let (width, height) = target;
    if images.is_empty()
        || width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 64_000_000
    {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid combined wallpaper").into(),
        );
    }
    let mut hash = DefaultHasher::new();
    for path in images {
        path.hash(&mut hash);
        let meta = fs::metadata(path)?;
        meta.len().hash(&mut hash);
        meta.modified()?.hash(&mut hash);
    }
    (width, height, gap).hash(&mut hash);
    let file = cache.join(format!("{:016x}.jpg", hash.finish()));
    if file.is_file() {
        return Ok(file);
    }

    let decoded = images
        .iter()
        .map(image::open)
        .collect::<Result<Vec<DynamicImage>, _>>()?;
    let mut canvas = background(&decoded[0], target);
    let scaled: Vec<RgbImage> = decoded
        .iter()
        .map(|image| {
            let size = width_at(image.dimensions(), height).min(width);
            image
                .resize_exact(size, height, FilterType::CatmullRom)
                .to_rgb8()
        })
        .collect();
    let total = scaled.iter().map(RgbImage::width).sum::<u32>()
        + gap * (scaled.len() as u32).saturating_sub(1);
    let mut x = i64::from(width.saturating_sub(total) / 2);
    for image in &scaled {
        image::imageops::overlay(&mut canvas, image, x, 0);
        x += i64::from(image.width() + gap);
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
    /// With `upcoming` images, a new combination is planned from them;
    /// without, the last combination shown for `first` is reused. Returns
    /// `None` when `first` should be shown alone. `size` is called only
    /// when a new combination is planned.
    pub fn combine(
        &self,
        target: &str,
        first: &Path,
        upcoming: &[PathBuf],
        size: impl FnOnce() -> Option<(u32, u32)>,
    ) -> Option<Combined> {
        if !self.enabled() {
            return None;
        }
        if upcoming.is_empty() {
            return self.remembered(target, first);
        }
        let first_size = crate::utilities::dimensions(first).ok()?;
        if !is_portrait(first_size) {
            return None;
        }
        let size = size()?;
        let candidates = upcoming
            .iter()
            .filter(|path| path.as_path() != first)
            .take(SCAN_LIMIT)
            .filter_map(|path| Some((path.clone(), crate::utilities::dimensions(path).ok()?)));
        let gap = self.settings.get().gap;
        let combined = plan(first_size, candidates, size, gap);
        if combined.is_empty() {
            return None;
        }
        let images: Vec<PathBuf> = std::iter::once(first.to_owned())
            .chain(combined.iter().cloned())
            .collect();
        let cache = self.cache.as_ref()?;
        let file = match render(&images, size, gap, cache) {
            Ok(file) => file,
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

    fn remember(&self, shown: Shown) {
        let mut list = self.shown.borrow_mut();
        list.retain(|entry| !(entry.target == shown.target && entry.first == shown.first));
        list.push_back(shown);
        while list.len() > KEEP {
            list.pop_front();
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
