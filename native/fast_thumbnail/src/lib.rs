use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};

use fast_image_resize::{images::Image as FirImage, pixels::PixelType, ResizeOptions, Resizer};

use base64::{engine::general_purpose::STANDARD, Engine as _};

use libwebp_sys::WebPImageHint;
use webp::{Encoder as WebPEncoder, PixelLayout, WebPConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FitMode {
    Cover,
    Contain,
    Stretch,
    Width,
    Height,
}

impl FitMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "cover" => Ok(Self::Cover),
            "contain" => Ok(Self::Contain),
            "stretch" => Ok(Self::Stretch),
            "width" => Ok(Self::Width),
            "height" => Ok(Self::Height),
            _ => Err(format!("Unknown fit mode: {value}")),
        }
    }
}

/// Preserves the original NIF entry point and its square center-crop behavior.
#[rustler::nif(schedule = "DirtyCpu")]
fn nif_create(path: String, width: u32, mode: String) -> Result<String, String> {
    create_thumbnail(path, width, width, mode, FitMode::Cover, true)
}

/// Resizes an image according to the requested dimensions and fit mode.
#[rustler::nif(schedule = "DirtyCpu")]
fn nif_create_with_options(
    path: String,
    width: u32,
    height: u32,
    mode: String,
    fit: String,
    upscale: bool,
) -> Result<String, String> {
    let fit = FitMode::parse(&fit)?;

    create_thumbnail(path, width, height, mode, fit, upscale)
}

/// Resizes and then either returns base64 or writes the output image.
fn create_thumbnail(
    path: String,
    width: u32,
    height: u32,
    mode: String,
    fit: FitMode,
    upscale: bool,
) -> Result<String, String> {
    // Open the file, wrap in a BufReader, and have ImageReader guess the format by reading the header.
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = ImageReader::new(BufReader::new(file));
    let reader = reader.with_guessed_format().map_err(|e| e.to_string())?;

    // Check that we were able to detect a format
    let original_format = reader
        .format()
        .ok_or_else(|| "Could not guess the image format".to_string())?;

    // Optional: Return an error if the format is not one of your “supported” ones.
    match original_format {
        ImageFormat::Jpeg
        | ImageFormat::Png
        | ImageFormat::Gif
        | ImageFormat::WebP
        | ImageFormat::Bmp
        | ImageFormat::Ico
        | ImageFormat::Tiff => {}
        other => {
            return Err(format!("Unsupported image format: {:?}", other));
        }
    }

    // Decode the image now that we have a reader
    let decoded_img = reader.decode().map_err(|e| e.to_string())?;

    // Convert to RGBA8 for fast_image_resize
    let rgba_img = decoded_img.to_rgba8();
    let (src_w, src_h) = (rgba_img.width(), rgba_img.height());

    let src_image = FirImage::from_vec_u8(src_w, src_h, rgba_img.into_raw(), PixelType::U8x4)
        .map_err(|e| e.to_string())?;

    let (dst_w, dst_h) = calculate_dimensions(src_w, src_h, width, height, fit, upscale);
    let mut dst_image = FirImage::new(dst_w, dst_h, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = match fit {
        FitMode::Cover => ResizeOptions::new().fit_into_destination(None),
        _ => ResizeOptions::new(),
    };

    resizer
        .resize(&src_image, &mut dst_image, &options)
        .map_err(|e| e.to_string())?;

    let resized_buf =
        ImageBuffer::<Rgba<u8>, _>::from_raw(dst_w, dst_h, dst_image.buffer().to_vec())
            .ok_or_else(|| "Error constructing resized buffer".to_string())?;

    let resized_img = DynamicImage::ImageRgba8(resized_buf);

    match mode.as_str() {
        "base64" => {
            // Encode as a WebP and return in base64
            let webp_data = encode_webp_advanced(&resized_img, 75.0)?;
            let b64 = STANDARD.encode(webp_data);
            Ok(b64)
        }

        "webp" => {
            // Encode as a WebP, then write to a .webp file
            let webp_data = encode_webp_advanced(&resized_img, 75.0)?;
            let new_path = format!("{}.webp", path);
            let file = File::create(&new_path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            writer.write_all(&webp_data).map_err(|e| e.to_string())?;
            Ok(new_path)
        }

        "overwrite" => {
            // Overwrite the file in its original format (JPEG→JPEG, PNG→PNG, etc.).
            resized_img
                .save_with_format(&path, original_format)
                .map_err(|e| e.to_string())?;
            Ok(path)
        }

        _ => Err(format!("Unknown mode: {}", mode)),
    }
}

fn calculate_dimensions(
    src_w: u32,
    src_h: u32,
    target_w: u32,
    target_h: u32,
    fit: FitMode,
    upscale: bool,
) -> (u32, u32) {
    match fit {
        FitMode::Cover => {
            if upscale {
                (target_w, target_h)
            } else {
                contain_dimensions(target_w, target_h, src_w, src_h, false)
            }
        }
        FitMode::Contain => contain_dimensions(src_w, src_h, target_w, target_h, upscale),
        FitMode::Stretch => {
            if upscale {
                (target_w, target_h)
            } else {
                (target_w.min(src_w), target_h.min(src_h))
            }
        }
        FitMode::Width => width_dimensions(src_w, src_h, target_w, upscale),
        FitMode::Height => height_dimensions(src_w, src_h, target_h, upscale),
    }
}

fn contain_dimensions(src_w: u32, src_h: u32, max_w: u32, max_h: u32, upscale: bool) -> (u32, u32) {
    if !upscale && src_w <= max_w && src_h <= max_h {
        return (src_w, src_h);
    }

    if u64::from(max_w) * u64::from(src_h) <= u64::from(max_h) * u64::from(src_w) {
        width_dimensions(src_w, src_h, max_w, true)
    } else {
        height_dimensions(src_w, src_h, max_h, true)
    }
}

fn width_dimensions(src_w: u32, src_h: u32, target_w: u32, upscale: bool) -> (u32, u32) {
    let dst_w = if upscale {
        target_w
    } else {
        target_w.min(src_w)
    };

    (dst_w, scale_dimension(src_h, dst_w, src_w))
}

fn height_dimensions(src_w: u32, src_h: u32, target_h: u32, upscale: bool) -> (u32, u32) {
    let dst_h = if upscale {
        target_h
    } else {
        target_h.min(src_h)
    };

    (scale_dimension(src_w, dst_h, src_h), dst_h)
}

fn scale_dimension(dimension: u32, target: u32, source: u32) -> u32 {
    let numerator = u64::from(dimension) * u64::from(target);
    let rounded = (numerator + u64::from(source) / 2) / u64::from(source);

    rounded.clamp(1, u64::from(u32::MAX)) as u32
}

/// Encode a `DynamicImage` as a quality‐tuned WebP using the `webp` crate.
fn encode_webp_advanced(img: &DynamicImage, quality: f32) -> Result<Vec<u8>, String> {
    let rgba8 = img.to_rgba8();
    let (w, h) = rgba8.dimensions();

    let encoder = WebPEncoder::new(&rgba8, PixelLayout::Rgba, w, h);

    // Configure the WebP encoder.
    let mut config = WebPConfig::new().map_err(|_| "Could not create WebP config".to_string())?;
    config.method = 3;
    config.image_hint = WebPImageHint::WEBP_HINT_PHOTO;
    config.sns_strength = 70;
    config.filter_sharpness = 2;
    config.filter_strength = 25;
    config.quality = quality;

    let webp_data = encoder
        .encode_advanced(&config)
        .map_err(|e| format!("WebP encoding error: {:?}", e))?;

    Ok(webp_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::{calculate_dimensions, FitMode};

    #[test]
    fn scales_proportionally_from_width() {
        assert_eq!(
            calculate_dimensions(1000, 400, 100, 100, FitMode::Width, true),
            (100, 40)
        );
    }

    #[test]
    fn scales_proportionally_from_height() {
        assert_eq!(
            calculate_dimensions(1000, 400, 40, 40, FitMode::Height, true),
            (100, 40)
        );
    }

    #[test]
    fn contains_image_without_cropping() {
        assert_eq!(
            calculate_dimensions(1000, 400, 100, 100, FitMode::Contain, true),
            (100, 40)
        );
    }

    #[test]
    fn cover_and_stretch_use_exact_target_dimensions() {
        assert_eq!(
            calculate_dimensions(1000, 400, 300, 200, FitMode::Cover, true),
            (300, 200)
        );
        assert_eq!(
            calculate_dimensions(1000, 400, 300, 200, FitMode::Stretch, true),
            (300, 200)
        );
    }

    #[test]
    fn can_disable_upscaling() {
        assert_eq!(
            calculate_dimensions(1000, 400, 2000, 2000, FitMode::Width, false),
            (1000, 400)
        );
        assert_eq!(
            calculate_dimensions(1000, 400, 2000, 2000, FitMode::Contain, false),
            (1000, 400)
        );
        assert_eq!(
            calculate_dimensions(1000, 400, 2000, 1000, FitMode::Cover, false),
            (800, 400)
        );
    }
}

rustler::init!("Elixir.FastThumbnail");
