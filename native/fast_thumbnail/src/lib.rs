use image::ImageEncoder;
use rustler::nif;

use std::fs::File;
use std::io::BufWriter;

use image::io::Reader as ImageReader;
use image::{codecs::png::PngEncoder, ColorType, RgbaImage};

use fast_image_resize::{images::Image as FirImage, pixels::PixelType, Resizer};

/// NIF: Overwrites `path` with a center-cropped (width x width) thumbnail as PNG.
/// Returns `Ok(path)` on success or `Err(error_string)` on failure.
#[nif]
fn nif_create(path: String, width: u32) -> Result<String, String> {
    let dyn_img = ImageReader::open(&path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    // 2) Center-crop to a square
    let (orig_w, orig_h) = (dyn_img.width(), dyn_img.height());
    let min_side = orig_w.min(orig_h);

    let left = (orig_w - min_side) / 2;
    let top = (orig_h - min_side) / 2;
    let cropped = dyn_img.crop_imm(left, top, min_side, min_side);

    let cropped_rgba: RgbaImage = cropped.to_rgba8();

    let src_image = FirImage::from_vec_u8(
        min_side,
        min_side,
        cropped_rgba.into_raw(),
        PixelType::U8x4, // RGBA8
    )
    .map_err(|e| e.to_string())?;

    let mut dst_image = FirImage::new(width, width, PixelType::U8x4);

    let mut resizer = Resizer::new();

    resizer
        .resize(&src_image, &mut dst_image, None)
        .map_err(|e| e.to_string())?;

    let file_out = File::create(&path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file_out);

    PngEncoder::new(&mut writer)
        .write_image(
            dst_image.buffer(), // &[u8]
            width,
            width,
            ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;

    Ok(path)
}

rustler::init!("Elixir.FastThumbnail");
