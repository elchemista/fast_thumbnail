use rustler;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};

use fast_image_resize::{images::Image as FirImage, pixels::PixelType, ResizeOptions, Resizer};

use base64::{engine::general_purpose::STANDARD, Engine as _};

use libwebp_sys::WebPImageHint;
use webp::{Encoder as WebPEncoder, PixelLayout, WebPConfig};

/// Performs a center-crop to square + resize to `width x width`.
/// Then:
///  - "base64": return a base64-encoded optimized WebP (no file I/O)
///  - "webp":   write an optimized `"{path}.webp"` file
///  - "overwrite": overwrite in the *original* format (JPEG→JPEG, PNG→PNG, etc.)
#[rustler::nif(schedule = "DirtyCpu")]
fn nif_create(path: String, width: u32, mode: String) -> Result<String, String> {
    // Open the file, wrap in a BufReader, and have ImageReader guess the format by reading the header.
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = ImageReader::new(BufReader::new(file));
    let mut reader = reader.with_guessed_format().map_err(|e| e.to_string())?;

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

    let mut dst_image = FirImage::new(width, width, PixelType::U8x4);

    let min_side = src_w.min(src_h) as f64;
    let left = (src_w as f64 - min_side) / 2.0;
    let top = (src_h as f64 - min_side) / 2.0;

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().crop(left, top, min_side, min_side);

    resizer
        .resize(&src_image, &mut dst_image, &options)
        .map_err(|e| e.to_string())?;

    let resized_buf =
        ImageBuffer::<Rgba<u8>, _>::from_raw(width, width, dst_image.buffer().to_vec())
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

rustler::init!("Elixir.FastThumbnail");
