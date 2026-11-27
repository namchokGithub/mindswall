use image::{GenericImageView, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditOptions {
    pub crop: Option<Crop>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub blur: f32,
    pub dim: u8,
}

pub fn render(
    source: &Path,
    cache: &Path,
    options: &EditOptions,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    if !options.blur.is_finite() || !(0.0..=20.0).contains(&options.blur) || options.dim > 90 {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid blur or dim value").into(),
        );
    }
    match (options.width, options.height) {
        (None, None) => (),
        (Some(w), Some(h))
            if (1..=8192).contains(&w)
                && (1..=8192).contains(&h)
                && u64::from(w) * u64::from(h) <= 32_000_000 => {}
        _ => {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid output dimensions").into(),
            );
        }
    }
    let mut img = image::open(source)?;
    if let Some(crop) = &options.crop {
        let (w, h) = img.dimensions();
        if crop.width == 0
            || crop.height == 0
            || crop.x.checked_add(crop.width).is_none_or(|v| v > w)
            || crop.y.checked_add(crop.height).is_none_or(|v| v > h)
        {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "Crop is outside the image").into(),
            );
        }
        img = img.crop_imm(crop.x, crop.y, crop.width, crop.height);
    }
    if let (Some(w), Some(h)) = (options.width, options.height) {
        img = img.resize_exact(w, h, FilterType::Lanczos3);
    }
    if options.blur > 0.0 {
        img = img.blur(options.blur);
    }
    let mut output = img.to_rgba8();
    for pixel in output.pixels_mut() {
        for color in &mut pixel.0[..3] {
            *color = (u32::from(*color) * u32::from(100 - options.dim) / 100) as u8;
        }
    }
    fs::create_dir_all(cache)?;
    let mut file = tempfile::Builder::new()
        .prefix("mindswall-")
        .suffix(".png")
        .tempfile_in(cache)?;
    // The encoder makes many small writes; buffer them.
    let mut writer = std::io::BufWriter::new(file.as_file_mut());
    output.write_to(&mut writer, image::ImageFormat::Png)?;
    std::io::Write::flush(&mut writer)?;
    drop(writer);
    let (_, path) = file.keep().map_err(|e| e.error)?;
    Ok(path)
}
