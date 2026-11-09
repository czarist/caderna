//! The app icon's engraving as a cover plate, like the animal engraving on a
//! programming book cover: Thoth, god of writing and knowledge, as a baboon.

use egui::{Image, ImageSource, vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art {
    /// Thoth as a baboon under the crescent moon, on a jade ground, square.
    Mark,
}

impl Art {
    fn source(self) -> ImageSource<'static> {
        match self {
            Self::Mark => brand_image(
                "cover.png",
                crate::brand::image("cover.png", kg_brand::COVER_PNG),
            ),
        }
    }

    /// Width over height of the artwork.
    pub fn aspect(self) -> f32 {
        match self {
            Self::Mark => 1.0,
        }
    }

    /// The artwork at `height` points.
    pub fn image(self, height: f32) -> Image<'static> {
        Image::new(self.source()).fit_to_exact_size(vec2(height * self.aspect(), height))
    }
}

/// A picture of the build's brand (`kg-brand`); `name` gives the loader its
/// format, e.g. "icon.png" or "mark.svg".
pub fn brand_image(name: &'static str, bytes: &'static [u8]) -> ImageSource<'static> {
    let uri = match name {
        "icon.png" => "bytes://brand/icon.png",
        "cover.png" => "bytes://brand/cover.png",
        "engraving.png" => "bytes://brand/engraving.png",
        _ => "bytes://brand/mark.svg",
    };
    ImageSource::Bytes {
        uri: std::borrow::Cow::Borrowed(uri),
        bytes: egui::load::Bytes::Static(bytes),
    }
}
