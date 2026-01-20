use image::imageops::FilterType;
use std::io::Cursor;

pub fn resize_image(data: &[u8], size_str: &str) -> Result<Vec<u8>, String> {
    let parts: Vec<&str> = size_str.split('x').collect();
    if parts.len() != 2 {
        return Err("Invalid resize format. Expected WxH (e.g. 100x100)".to_string());
    }

    let width: u32 = parts[0].parse().map_err(|_| "Invalid width".to_string())?;
    let height: u32 = parts[1].parse().map_err(|_| "Invalid height".to_string())?;

    let img = image::load_from_memory(data).map_err(|e| e.to_string())?;

    // Using resize_to_fill or resize? resize maintains aspect ratio and fits within.
    // resize_exact stretches.
    // resize_to_fill crops.
    // Usually "resize" implies fitting or filling. Let's assume `resize` (fit within box) for now,
    // or maybe `resize_to_fill` is better for thumbnails.
    // Given typical use cases (avatars), `resize_to_fill` is often preferred if they want square.
    // But if they ask for `resize`, `img.resize` is the safest default.

    let resized = img.resize(width, height, FilterType::Lanczos3);

    let mut out = Cursor::new(Vec::new());
    // Convert to PNG for consistency or keep original format?
    // image crate `load_from_memory` detects format.
    // `write_to` requires format.
    // We should probably preserve format if possible, but simplest is to standardize on PNG or JPEG.
    // Let's use PNG for lossless.
    resized.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;

    Ok(out.into_inner())
}
