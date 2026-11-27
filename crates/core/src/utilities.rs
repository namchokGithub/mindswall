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

/// A comparison key for a path. Windows paths compare case-insensitively and
/// treat `/` and `\` alike; other platforms compare paths exactly. Unpaired
/// UTF-16 surrogates are kept, so paths that are not valid Unicode still match.
#[cfg(windows)]
pub fn path_key(path: &Path) -> std::ffi::OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let mut key = Vec::new();
    for unit in char::decode_utf16(path.as_os_str().encode_wide()) {
        match unit {
            Ok('/') => key.push(u16::from(b'\\')),
            Ok(c) => {
                for lower in c.to_lowercase() {
                    let mut buffer = [0; 2];
                    key.extend_from_slice(lower.encode_utf16(&mut buffer));
                }
            }
            Err(error) => key.push(error.unpaired_surrogate()),
        }
    }
    std::ffi::OsString::from_wide(&key)
}

/// A comparison key for a path. Windows paths compare case-insensitively and
/// treat `/` and `\` alike; other platforms compare paths exactly.
#[cfg(not(windows))]
pub fn path_key(path: &Path) -> std::ffi::OsString {
    path.as_os_str().to_owned()
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

/// Delete the oldest files in `dir` until at most `max_files` remain and
/// their total size is at most `max_bytes`. Files whose names are in `keep`
/// are never deleted. Returns the number of deleted files.
pub fn prune_cache(
    dir: &Path,
    max_files: usize,
    max_bytes: u64,
    keep: &std::collections::HashSet<std::ffi::OsString>,
) -> io::Result<usize> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_file() {
            let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
            files.push((modified, metadata.len(), entry.path()));
        }
    }
    files.sort();
    let mut count = files.len();
    let mut total: u64 = files.iter().map(|(_, size, _)| size).sum();
    let mut deleted = 0;
    for (_, size, path) in files {
        if count <= max_files && total <= max_bytes {
            break;
        }
        if path.file_name().is_some_and(|name| keep.contains(name)) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {
                count -= 1;
                total -= size;
                deleted += 1;
            }
            Err(error) => tracing::warn!(path = ?path, %error, "could not delete cached file"),
        }
    }
    Ok(deleted)
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
    let img = open_scaled(path, (240, 150))?.thumbnail(240, 150);
    let mut file = tempfile::NamedTempFile::new_in(cache)?;
    // The encoder makes many small writes; buffer them.
    let mut writer = io::BufWriter::new(file.as_file_mut());
    img.write_to(&mut writer, image::ImageFormat::Png)?;
    io::Write::flush(&mut writer)?;
    drop(writer);
    drop(img);
    file.persist(&target)
        .map_err(|e| io::Error::other(e.error))?;
    trim_memory_soon();
    Ok(target)
}

/// This process's memory in MB, for timing logs.
#[derive(Debug, Clone, Copy)]
pub struct Memory {
    /// Physical memory in use now (the working set; Task Manager shows a
    /// similar number).
    pub working: u64,
    /// The highest working set so far.
    pub peak: u64,
    /// Private memory the process has committed. Unlike the working set, it
    /// does not drop when Windows trims the process, so a steady rise here
    /// points to memory that is never freed.
    pub commit: u64,
}

/// This process's memory; `None` where the system does not report it.
pub fn memory_mb() -> Option<Memory> {
    #[cfg(windows)]
    {
        use windows::Win32::System::{
            ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            Threading::GetCurrentProcess,
        };
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        let size = u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?;
        // SAFETY: the counters live on the stack for the call, `cb` is their
        // size, and the pseudo handle of the current process needs no closing.
        unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) }.ok()?;
        let mb = |bytes: usize| u64::try_from(bytes / (1024 * 1024)).unwrap_or(u64::MAX);
        Some(Memory {
            working: mb(counters.WorkingSetSize),
            peak: mb(counters.PeakWorkingSetSize),
            // For a process, the pagefile usage is its private commit charge.
            commit: mb(counters.PagefileUsage),
        })
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The current working set in MB, for timing logs.
pub fn ram_mb() -> Option<u64> {
    memory_mb().map(|memory| memory.working)
}

/// The highest working set so far in MB, for timing logs.
pub fn peak_ram_mb() -> Option<u64> {
    memory_mb().map(|memory| memory.peak)
}

/// The committed private memory in MB, for timing logs.
pub fn commit_mb() -> Option<u64> {
    memory_mb().map(|memory| memory.commit)
}

/// Log this process's memory with `event`, for example to compare it before
/// and after the window's WebView exists.
pub fn log_memory(event: &str) {
    let memory = memory_mb();
    tracing::info!(
        ram_mb = memory.map(|memory| memory.working),
        commit_mb = memory.map(|memory| memory.commit),
        "{event}"
    );
}

/// Give memory that is no longer used back to Windows: compact the process
/// heap and trim the working set. Pages still in use come back when touched.
/// Does nothing on other systems.
pub fn trim_memory() {
    #[cfg(windows)]
    {
        use windows::Win32::System::{
            Memory::{GetProcessHeap, HEAP_FLAGS, HeapCompact},
            Threading::{GetCurrentProcess, SetProcessWorkingSetSize},
        };
        let before = memory_mb();
        // SAFETY: both calls take the pseudo handle of this process or its
        // default heap, which need no closing; `usize::MAX` for both sizes
        // asks Windows to trim the working set.
        unsafe {
            if let Ok(heap) = GetProcessHeap() {
                HeapCompact(heap, HEAP_FLAGS(0));
            }
            if let Err(error) =
                SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX)
            {
                tracing::debug!(%error, "could not trim the working set");
            }
        }
        let after = memory_mb();
        tracing::info!(
            before_mb = before.map(|memory| memory.working),
            ram_mb = after.map(|memory| memory.working),
            commit_mb = after.map(|memory| memory.commit),
            "memory returned to Windows"
        );
    }
}

/// How long [`trim_memory_soon`] waits after the last request.
const TRIM_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// Call [`trim_memory`] once image work has been quiet for two seconds, so
/// a burst of thumbnails trims once, after the last one.
pub fn trim_memory_soon() {
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };
    static SCHEDULED: AtomicBool = AtomicBool::new(false);
    static LAST: Mutex<Option<std::time::Instant>> = Mutex::new(None);
    let now = std::time::Instant::now();
    if let Ok(mut last) = LAST.lock() {
        *last = Some(now);
    }
    if SCHEDULED.swap(true, Ordering::AcqRel) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("trim-memory".into())
        .spawn(|| {
            loop {
                let last = LAST.lock().ok().and_then(|last| *last);
                let wait = last
                    .map(|last| {
                        (last + TRIM_DELAY).saturating_duration_since(std::time::Instant::now())
                    })
                    .unwrap_or_default();
                if wait.is_zero() {
                    break;
                }
                std::thread::sleep(wait);
            }
            SCHEDULED.store(false, Ordering::Release);
            trim_memory();
        });
    if spawned.is_err() {
        SCHEDULED.store(false, Ordering::Release);
    }
}

/// Resize `image` to exactly `width` × `height` RGB pixels with a
/// convolution `filter`. Unlike `image`'s resize, this works on the 8-bit
/// pixels directly instead of a full-size floating point copy, which needs far
/// less memory for large images and is faster.
pub fn resize_rgb(
    resizer: &mut fast_image_resize::Resizer,
    image: image::DynamicImage,
    (width, height): (u32, u32),
    filter: fast_image_resize::FilterType,
) -> Result<image::RgbImage, Box<dyn std::error::Error + Send + Sync>> {
    use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions, images::Image};
    let source = image.into_rgb8();
    if source.dimensions() == (width, height) {
        return Ok(source);
    }
    let (source_width, source_height) = source.dimensions();
    let source = Image::from_vec_u8(
        source_width,
        source_height,
        source.into_raw(),
        PixelType::U8x3,
    )?;
    let mut target = Image::new(width, height, PixelType::U8x3);
    resizer.resize(
        &source,
        &mut target,
        &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter)),
    )?;
    drop(source);
    image::RgbImage::from_raw(width, height, target.into_vec())
        .ok_or_else(|| io::Error::other("resized image has the wrong size").into())
}

/// Open an image that will be shown no larger than `max` (it is fitted inside
/// it, keeping its shape). JPEG files are decoded at 1/2, 1/4, or 1/8 size
/// when that is still large enough, which needs far less memory and time than
/// decoding every pixel; other files, and JPEG files the scaling decoder
/// cannot read, are opened normally. The result can be larger than `max`, so
/// resize it afterwards.
pub fn open_scaled(path: &Path, max: (u32, u32)) -> image::ImageResult<image::DynamicImage> {
    if matches!(
        image::ImageFormat::from_path(path),
        Ok(image::ImageFormat::Jpeg)
    ) && let Some(image) = open_jpeg_scaled(path, max)
    {
        return Ok(image);
    }
    image::open(path)
}

fn open_jpeg_scaled(
    path: &Path,
    (max_width, max_height): (u32, u32),
) -> Option<image::DynamicImage> {
    use jpeg_decoder::{Decoder, PixelFormat};
    let file = fs::File::open(path).ok()?;
    let mut decoder = Decoder::new(io::BufReader::new(file));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let (width, height) = (f64::from(info.width), f64::from(info.height));
    // The fitted size; the decoder picks the smallest scale that reaches it.
    let factor = (f64::from(max_width) / width)
        .min(f64::from(max_height) / height)
        .min(1.0);
    let side = |length: f64| (length * factor).ceil().clamp(1.0, f64::from(u16::MAX)) as u16;
    decoder.scale(side(width), side(height)).ok()?;
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    match info.pixel_format {
        PixelFormat::RGB24 => {
            image::RgbImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageRgb8)
        }
        PixelFormat::L8 => {
            image::GrayImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageLuma8)
        }
        // CMYK and 16-bit images are rare; the regular decoder handles them.
        _ => None,
    }
}

/// Milliseconds since `started`, for timing logs.
pub fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Largest preview the app shows for a wallpaper, in pixels.
pub const PREVIEW_SIZE: (u32, u32) = (1600, 900);

/// A JPEG copy of `path` that fits in [`PREVIEW_SIZE`] (smaller images keep
/// their size), cached in `cache` by path, size, and modification time. A
/// cached copy gets a new modification time each time it is used. Returns the
/// file and whether it was already cached.
pub fn preview(
    path: &Path,
    cache: &Path,
) -> Result<(PathBuf, bool), Box<dyn std::error::Error + Send + Sync>> {
    use image::ImageEncoder;
    let started = std::time::Instant::now();
    let meta = fs::metadata(path)?;
    let mut hash = DefaultHasher::new();
    path.hash(&mut hash);
    meta.len().hash(&mut hash);
    meta.modified()?.hash(&mut hash);
    PREVIEW_SIZE.hash(&mut hash);
    let target = cache.join(format!("{:016x}.jpg", hash.finish()));
    if target.is_file() {
        // Mark it as recently used, so cleaning the cache (oldest first)
        // removes previews that are no longer shown, not this one.
        let touched = fs::File::options()
            .write(true)
            .open(&target)
            .and_then(|file| file.set_modified(std::time::SystemTime::now()));
        if let Err(error) = touched {
            tracing::debug!(path = ?target, %error, "could not mark a cached preview as used");
        }
        tracing::info!(path = ?path, ms = elapsed_ms(started), "preview read from cache");
        return Ok((target, true));
    }
    fs::create_dir_all(cache)?;
    let image = open_scaled(path, PREVIEW_SIZE)?;
    let decode_ms = elapsed_ms(started);
    let (width, height) = PREVIEW_SIZE;
    let image = if image.width() > width || image.height() > height {
        image.thumbnail(width, height)
    } else {
        image
    };
    let rgb = image.to_rgb8();
    let mut file = tempfile::NamedTempFile::new_in(cache)?;
    // The encoder makes many small writes; buffer them.
    let mut writer = io::BufWriter::new(file.as_file_mut());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 85).write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    io::Write::flush(&mut writer)?;
    drop(writer);
    file.persist(&target)
        .map_err(|e| io::Error::other(e.error))?;
    tracing::info!(
        path = ?path,
        bytes = meta.len(),
        decode_ms,
        ms = elapsed_ms(started),
        ram_mb = ram_mb(),
        peak_mb = peak_ram_mb(),
        commit_mb = commit_mb(),
        "preview created"
    );
    trim_memory_soon();
    Ok((target, false))
}

/// The last `count` lines of a text file, oldest first. Only the end of the
/// file is read (at most 4 MB), so a large log stays cheap. Invalid UTF-8 is
/// replaced. A missing file has no lines.
pub fn tail_lines(path: &Path, count: usize) -> io::Result<Vec<String>> {
    use std::io::{Read, Seek, SeekFrom};
    const CHUNK: u64 = 16 * 1024;
    const LIMIT: u64 = 4 * 1024 * 1024;
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let length = file.metadata()?.len();
    let mut start = length;
    let mut buffer = Vec::new();
    // One extra line break is needed to know that the oldest kept line is whole.
    while start > 0
        && length - start < LIMIT
        && buffer.iter().filter(|&&byte| byte == b'\n').count() <= count
    {
        let size = CHUNK.min(start);
        start -= size;
        file.seek(SeekFrom::Start(start))?;
        let mut chunk = vec![0; size as usize];
        file.read_exact(&mut chunk)?;
        chunk.extend_from_slice(&buffer);
        buffer = chunk;
    }
    let text = String::from_utf8_lossy(&buffer);
    let mut lines: Vec<&str> = text.lines().collect();
    if start > 0 && !lines.is_empty() {
        // The first line starts before the part that was read.
        lines.remove(0);
    }
    let skip = lines.len().saturating_sub(count);
    Ok(lines[skip..]
        .iter()
        .map(|line| (*line).to_owned())
        .collect())
}
