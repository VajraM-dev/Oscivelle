//! Pictures ("plates") a look can show: --image files and AI-made ones. Each is centre-cropped and
//! resized to PLATE_W x PLATE_H (3:2), RGBA8 sRGB, ready for Engine::set_plates.
use crate::types::{PLATE_H, PLATE_W};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Plates per plan (texture array layers).
pub const MAX: usize = 8;

fn fit(img: image::DynamicImage) -> Vec<u8> {
    img.resize_to_fill(PLATE_W, PLATE_H, image::imageops::FilterType::Lanczos3).to_rgba8().into_raw()
}

pub fn load(path: &Path) -> Result<Vec<u8>> {
    Ok(fit(image::open(path).with_context(|| format!("reading image {}", path.display()))?))
}

/// The plan's plates; a missing or broken file becomes a blank (transparent) plate, with a warning.
pub fn load_all(paths: &[PathBuf]) -> Vec<Vec<u8>> {
    paths
        .iter()
        .take(MAX)
        .map(|p| {
            load(p).unwrap_or_else(|e| {
                eprintln!("warning: {e:#}; that picture stays blank");
                vec![0; (PLATE_W * PLATE_H * 4) as usize]
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_any_image_to_the_plate_size() {
        let red = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(30, 10, image::Rgb([255, 0, 0])));
        let px = fit(red);
        assert_eq!(px.len(), (PLATE_W * PLATE_H * 4) as usize);
        assert_eq!(&px[..4], &[255, 0, 0, 255]);
        assert_eq!(load_all(&[PathBuf::from("no/such.png")])[0].len(), px.len());
    }
}
