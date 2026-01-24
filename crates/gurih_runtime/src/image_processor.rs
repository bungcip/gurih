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
    resized
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba, ImageFormat};

    #[test]
    fn test_resize_image_coverage() {
        // 1. Setup valid image
        let width = 10;
        let height = 10;
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        let mut valid_png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut valid_png), ImageFormat::Png).expect("Failed to create test image");

        // 2. Test successful resize (Happy Path)
        // Resize to 5x5
        let result = resize_image(&valid_png, "5x5");
        assert!(result.is_ok(), "Resize should succeed with valid input");
        let resized_bytes = result.unwrap();
        // Verify output is a valid image and dimensions are correct
        let resized_img = image::load_from_memory(&resized_bytes).expect("Output should be a valid image");
        assert_eq!(resized_img.width(), 5);
        assert_eq!(resized_img.height(), 5);

        // 3. Test Invalid Format String (Error Path)
        let err = resize_image(&valid_png, "500").unwrap_err();
        assert_eq!(err, "Invalid resize format. Expected WxH (e.g. 100x100)");

        // 4. Test Invalid Dimensions (Error Path)
        let err_width = resize_image(&valid_png, "abcx10").unwrap_err();
        assert_eq!(err_width, "Invalid width");

        let err_height = resize_image(&valid_png, "10xabc").unwrap_err();
        assert_eq!(err_height, "Invalid height");

        // 5. Test Invalid Image Data (Error Path)
        let invalid_data = vec![0u8; 10];
        let err_data = resize_image(&invalid_data, "10x10").unwrap_err();
        // The error message comes from image crate, usually "FormatError" or similar.
        // We just assert it returns an error and maybe check it's not empty.
        assert!(!err_data.is_empty());
    }
}
