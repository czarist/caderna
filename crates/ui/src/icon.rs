//! The app icon: the owner's engraving of Thoth as a baboon under the
//! crescent moon, on a jade tile (built by `tools/icon.py`).

/// The icon, 512 × 512 with rounded corners.
pub const ICON_PNG: &[u8] = kg_brand::ICON_PNG;

/// Icon pixels as straight (not premultiplied) RGBA, `size × size`.
/// Falls back to a plain jade square if the image cannot be decoded.
pub fn rgba(size: u32) -> Vec<u8> {
    render(size).unwrap_or_else(|| {
        [0x3F, 0x8F, 0x7C, 0xFF]
            .into_iter()
            .cycle()
            .take((size * size * 4) as usize)
            .collect()
    })
}

fn render(size: u32) -> Option<Vec<u8>> {
    let image = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?;
    let resized = image.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    Some(resized.into_rgba8().into_raw())
}

pub fn app_icon() -> egui::IconData {
    let size = 128;
    egui::IconData {
        rgba: rgba(size),
        width: size,
        height: size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brand_icon_decodes_with_transparent_corners() {
        let size = 64;
        let px = render(size).unwrap();
        assert_eq!(px.len(), (size * size * 4) as usize);
        // brand.toml asks for transparent corners; resampling may leave a trace.
        assert!(px[3] < 16, "corner alpha {}", px[3]);
        // The middle of the tile is opaque.
        let i = ((size / 2 * size + size / 2) * 4) as usize;
        assert_eq!(px[i + 3], 0xFF);
    }
}
