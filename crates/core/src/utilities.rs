use serde::{Deserialize, Serialize};
use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Any,
    Landscape,
    Portrait,
    Square,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    #[default]
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
    Span,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectRatio {
    #[default]
    Any,
    Wide,
    WideTen,
    Standard,
    Square,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageFilter {
    pub min_width: u32,
    pub min_height: u32,
    pub orientation: Orientation,
    pub aspect_ratio: AspectRatio,
}

pub fn matches_filter(width: u32, height: u32, filter: &ImageFilter) -> bool {
    width >= filter.min_width
        && height >= filter.min_height
        && match filter.orientation {
            Orientation::Any => true,
            Orientation::Landscape => width > height,
            Orientation::Portrait => height > width,
            Orientation::Square => width == height,
        }
        && match filter.aspect_ratio {
            AspectRatio::Any => true,
            ratio => {
                let (rw, rh): (u64, u64) = match ratio {
                    AspectRatio::Wide => (16, 9),
                    AspectRatio::WideTen => (16, 10),
                    AspectRatio::Standard => (4, 3),
                    AspectRatio::Square => (1, 1),
                    AspectRatio::Any => unreachable!(),
                };
                let actual = u64::from(width) * rh;
                let target = u64::from(height) * rw;
                target > 0 && actual.abs_diff(target) * 100 <= target * 5
            }
        }
}

pub fn dimensions(path: &Path) -> Result<(u32, u32), image::ImageError> {
    use std::sync::{Mutex, OnceLock};
    type Cached = (u64, std::time::SystemTime, (u32, u32));
    static CACHE: OnceLock<Mutex<std::collections::HashMap<PathBuf, Cached>>> = OnceLock::new();
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Ok(entries) = cache.lock()
        && let Some((length, time, dimensions)) = entries.get(path)
        && *length == metadata.len()
        && *time == modified
    {
        return Ok(*dimensions);
    }
    let dimensions = image::image_dimensions(path)?;
    if let Ok(mut entries) = cache.lock() {
        if entries.len() >= 10000 {
            entries.clear();
        }
        entries.insert(path.to_owned(), (metadata.len(), modified, dimensions));
    }
    Ok(dimensions)
}

#[derive(Serialize)]
pub struct ImageProperties {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u16,
    pub size: u64,
    pub created: Option<u64>,
    pub modified: Option<u64>,
}

pub fn image_properties(path: &Path) -> Result<ImageProperties, image::ImageError> {
    use image::ImageDecoder;
    use std::time::UNIX_EPOCH;
    let metadata = fs::metadata(path)?;
    let decoder = image::ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    let (width, height) = decoder.dimensions();
    let timestamp = |time: io::Result<std::time::SystemTime>| {
        time.ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|time| time.as_secs())
    };
    Ok(ImageProperties {
        width,
        height,
        bit_depth: decoder.color_type().bits_per_pixel(),
        size: metadata.len(),
        created: timestamp(metadata.created()),
        modified: timestamp(metadata.modified()),
    })
}

pub fn thumbnail(
    path: &Path,
    cache: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let meta = fs::metadata(path)?;
    let mut hash = DefaultHasher::new();
    path.hash(&mut hash);
    meta.len().hash(&mut hash);
    meta.modified()?.hash(&mut hash);
    let target = cache.join(format!("{:016x}.png", hash.finish()));
    if target.is_file() {
        return Ok(target);
    }
    fs::create_dir_all(cache)?;
    let img = image::open(path)?.thumbnail(240, 150);
    let mut file = tempfile::NamedTempFile::new_in(cache)?;
    img.write_to(&mut file, image::ImageFormat::Png)?;
    file.persist(&target)
        .map_err(|e| io::Error::other(e.error))?;
    Ok(target)
}
