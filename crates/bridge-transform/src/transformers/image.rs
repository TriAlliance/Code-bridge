//! Image format transformations

use crate::{Format, Result, TransformError, TransformOptions};
use image::{DynamicImage, ImageFormat as ImgFormat};

/// Transform image between formats
pub fn transform_image(
    data: &[u8],
    from: Format,
    to: Format,
    options: &TransformOptions,
) -> Result<Vec<u8>> {
    let mut img = image::load_from_memory(data)?;

    // Apply resize if specified
    if let Some((width, height)) = options.resize {
        img = if options.preserve_aspect {
            img.resize(width, height, image::imageops::FilterType::Lanczos3)
        } else {
            img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        };
    }

    // Convert to target format
    let output_format = format_to_image_format(to)?;
    let mut output = Vec::new();

    match to {
        Format::Jpeg => {
            let quality = options.quality.unwrap_or(85);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut output,
                quality,
            );
            img.write_with_encoder(encoder)?;
        }
        Format::Png => {
            img.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
        }
        Format::Gif => {
            img.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Gif)?;
        }
        Format::Bmp => {
            img.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Bmp)?;
        }
        Format::Ico => {
            img.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Ico)?;
        }
        Format::Tiff => {
            img.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Tiff)?;
        }
        Format::WebP => {
            let rgba = img.to_rgba8();
            let quality = options.quality.unwrap_or(85) as f32;
            let encoder = webp::Encoder::from_rgba(&rgba, rgba.width(), rgba.height());
            let encoded = encoder.encode(quality);
            output = encoded.to_vec();
        }
        _ => {
            return Err(TransformError::UnsupportedFormat(format!("{:?}", to)));
        }
    }

    Ok(output)
}

/// Convert our Format to image-rs ImageFormat
fn format_to_image_format(format: Format) -> Result<ImgFormat> {
    match format {
        Format::Png => Ok(ImgFormat::Png),
        Format::Jpeg => Ok(ImgFormat::Jpeg),
        Format::Gif => Ok(ImgFormat::Gif),
        Format::Bmp => Ok(ImgFormat::Bmp),
        Format::Ico => Ok(ImgFormat::Ico),
        Format::Tiff => Ok(ImgFormat::Tiff),
        Format::WebP => Ok(ImgFormat::WebP),
        _ => Err(TransformError::UnsupportedFormat(format!("{:?}", format))),
    }
}

/// Get image dimensions
pub fn get_dimensions(data: &[u8]) -> Result<(u32, u32)> {
    let img = image::load_from_memory(data)?;
    Ok((img.width(), img.height()))
}

/// Calculate optimal dimensions for resize
pub fn calculate_dimensions(
    original_width: u32,
    original_height: u32,
    max_width: u32,
    max_height: u32,
) -> (u32, u32) {
    let width_ratio = max_width as f64 / original_width as f64;
    let height_ratio = max_height as f64 / original_height as f64;
    let ratio = width_ratio.min(height_ratio);

    let new_width = (original_width as f64 * ratio).round() as u32;
    let new_height = (original_height as f64 * ratio).round() as u32;

    (new_width.max(1), new_height.max(1))
}

/// Apply blur effect
pub fn apply_blur(data: &[u8], sigma: f32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let blurred = img.blur(sigma);

    let mut output = Vec::new();
    blurred.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

/// Adjust brightness
pub fn adjust_brightness(data: &[u8], value: i32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let adjusted = img.brighten(value);

    let mut output = Vec::new();
    adjusted.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

/// Adjust contrast
pub fn adjust_contrast(data: &[u8], value: f32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let adjusted = img.adjust_contrast(value);

    let mut output = Vec::new();
    adjusted.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

/// Rotate image
pub fn rotate(data: &[u8], degrees: u32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;

    let rotated = match degrees {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    };

    let mut output = Vec::new();
    rotated.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

/// Flip image
pub fn flip(data: &[u8], horizontal: bool) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;

    let flipped = if horizontal {
        img.fliph()
    } else {
        img.flipv()
    };

    let mut output = Vec::new();
    flipped.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

/// Crop image
pub fn crop(data: &[u8], x: u32, y: u32, width: u32, height: u32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let cropped = img.crop_imm(x, y, width, height);

    let mut output = Vec::new();
    cropped.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

/// Convert to grayscale
pub fn grayscale(data: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    let gray = img.grayscale();

    let mut output = Vec::new();
    gray.write_to(&mut std::io::Cursor::new(&mut output), ImgFormat::Png)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions() {
        // Landscape image
        let (w, h) = calculate_dimensions(1920, 1080, 800, 600);
        assert!(w <= 800);
        assert!(h <= 600);

        // Portrait image
        let (w, h) = calculate_dimensions(1080, 1920, 800, 600);
        assert!(w <= 800);
        assert!(h <= 600);
    }
}
