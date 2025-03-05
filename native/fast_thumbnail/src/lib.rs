use rustler::nif;
use std::fs::File;
use std::io::{BufWriter, Cursor};

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgba};

use fast_image_resize::{images::Image as FirImage, pixels::PixelType, ResizeOptions, Resizer};

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// Performs a center-crop to square + resize to `width x width`.
/// Then:
///  - "base64": return a base64-encoded WebP (no file I/O)
///  - "webp":   write a new `"{path}.webp"` file
///  - "overwrite":   overwrite in the *original format* (JPEG→JPEG, PNG→PNG, etc.)
#[nif]
fn nif_create(path: String, width: u32, mode: String) -> Result<String, String> {
    let reader = ImageReader::open(&path).map_err(|e| e.to_string())?;
    let original_format = reader.format();
    let decoded_img = reader.decode().map_err(|e| e.to_string())?;

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
            let mut buf = Vec::new();
            resized_img
                .write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::WebP)
                .map_err(|e| e.to_string())?;

            // Base64-encode
            let b64 = STANDARD.encode(&buf);
            Ok(b64)
        }

        "webp" => {
            let new_path = format!("{}.webp", path);
            let file = File::create(&new_path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);

            resized_img
                .write_to(&mut writer, ImageOutputFormat::WebP)
                .map_err(|e| e.to_string())?;

            Ok(new_path)
        }

        "overwrite" => {
            let fmt = match original_format {
                Some(f) => f,
                None => return Err("Could not determine original format".to_string()),
            };

            // Overwrite the file in the same format
            resized_img
                .save_with_format(&path, fmt)
                .map_err(|e| e.to_string())?;

            Ok(path)
        }

        // Unknown mode
        _ => Err(format!("Unknown mode: {}", mode)),
    }
}

rustler::init!("Elixir.FastThumbnail");
