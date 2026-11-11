//! The client's brand as the panel sends it: names, colours, links, texts
//! and pictures. Everything is checked before it is kept, and it is kept as
//! files in `<data dir>/panel-brand/` so the reminder pop-up (a separate
//! process) reads the same brand. A new brand applies as soon as it arrives.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::AppPaths;

const FOLDER: &str = "panel-brand";
const BRAND_FILE: &str = "brand.json";
const MAX_PNG: usize = 2 * 1024 * 1024;
const MAX_SVG: usize = 512 * 1024;
const MAX_TEXTS: usize = 300;
const MAX_TEXT_LEN: usize = 2000;

/// The pictures the panel can send: its name in the panel's `images`, and
/// the file kept here.
pub const IMAGES: [(&str, &str); 5] = [
    ("icon", "icon.png"),
    ("cover", "cover.png"),
    ("engraving", "engraving.png"),
    ("mark", "mark.svg"),
    ("watermark", "watermark.svg"),
];

/// The brand as the panel sends it, already checked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelBrand {
    pub app_name: Option<String>,
    pub company_name: Option<String>,
    pub accent_light: Option<[u8; 3]>,
    pub accent_dark: Option<[u8; 3]>,
    pub highlight_light: Option<[u8; 3]>,
    pub highlight_dark: Option<[u8; 3]>,
    /// Credit and watermark text on handouts.
    pub company_color: Option<[u8; 3]>,
    pub support_url: Option<String>,
    pub contact_url: Option<String>,
    /// UI texts that replace the app's, by locale key (`demo.expired_body`).
    pub texts: BTreeMap<String, String>,
    /// Pictures by file name ([`IMAGES`]); kept as files, not in the JSON.
    #[serde(skip)]
    pub images: BTreeMap<String, Vec<u8>>,
}

impl PanelBrand {
    /// Reads the panel's brand JSON, dropping anything that is not safe to show.
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty() && v.chars().count() <= 60 && !v.contains(['\n', '\r']))
                .map(str::to_owned)
        };
        let url = |key: &str| {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| {
                    v.len() <= 500
                        && (v.starts_with("https://") || v.starts_with("mailto:"))
                        && !v.contains(char::is_whitespace)
                })
                .map(str::to_owned)
        };
        let color = |key: &str| {
            value
                .get("colors")
                .and_then(|c| c.get(key))
                .and_then(|v| v.as_str())
                .and_then(parse_hex)
        };
        Self {
            app_name: text("app_name"),
            company_name: text("company_name"),
            accent_light: color("accent_light"),
            accent_dark: color("accent_dark"),
            highlight_light: color("highlight_light"),
            highlight_dark: color("highlight_dark"),
            // Handout text in the company colour is left out of the reader, so
            // it must differ from the template's own colours (as in brand.toml).
            company_color: color("company").filter(|c| {
                ![
                    [0x2F, 0x6F, 0x68],
                    [0x9A, 0x6B, 0x1F],
                    [0x3B, 0x7A, 0x4A],
                    [0, 0, 0],
                ]
                .contains(c)
            }),
            support_url: url("support_url"),
            contact_url: url("contact_url"),
            texts: texts(value.get("texts")),
            images: BTreeMap::new(),
        }
    }

    /// A picture by file name (`icon.png`, `mark.svg`, …).
    pub fn image(&self, file: &str) -> Option<&[u8]> {
        self.images.get(file).map(Vec::as_slice)
    }
}

/// UI texts with plain keys and bounded values.
fn texts(value: Option<&serde_json::Value>) -> BTreeMap<String, String> {
    let Some(map) = value.and_then(|v| v.as_object()) else {
        return BTreeMap::new();
    };
    map.iter()
        .filter(|(key, _)| {
            (1..=100).contains(&key.len())
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
        })
        .filter_map(|(key, text)| {
            let text = text.as_str()?.trim();
            (!text.is_empty() && text.chars().count() <= MAX_TEXT_LEN)
                .then(|| (key.clone(), text.to_owned()))
        })
        .take(MAX_TEXTS)
        .collect()
}

pub(crate) fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// Where each picture is downloaded from: `https://`, or the panel's own
/// address (plain HTTP while developing on localhost). `None`: no picture.
pub(crate) fn image_urls(value: &serde_json::Value) -> Vec<(&'static str, Option<String>)> {
    IMAGES
        .iter()
        .map(|(name, file)| {
            let url = value
                .get("images")
                .and_then(|images| images.get(*name))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|url| {
                    url.len() <= 1000
                        && !url.contains(char::is_whitespace)
                        && (url.starts_with("https://")
                            || (!kg_brand::SERVER_URL.is_empty()
                                && url.starts_with(&format!("{}/", kg_brand::SERVER_URL))))
                })
                .map(str::to_owned);
            (*file, url)
        })
        .collect()
}

/// A picture the app can show: a square PNG of a sensible size, or an SVG
/// without scripts or outside references.
pub(crate) fn image_ok(file: &str, bytes: &[u8]) -> bool {
    if file.ends_with(".png") {
        if bytes.len() > MAX_PNG || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return false;
        }
        return image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .is_ok_and(|img| img.width() == img.height() && (64..=2048).contains(&img.width()));
    }
    if bytes.len() > MAX_SVG {
        return false;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    lower.contains("<svg")
        && ![
            "<script",
            "javascript:",
            "<foreignobject",
            "href=\"http",
            "href='http",
            "href=\"//",
            "href='//",
            "url(http",
            "@import",
        ]
        .iter()
        .any(|bad| lower.contains(bad))
}

fn folder(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(FOLDER)
}

/// The brand kept from an earlier download, with its pictures.
pub fn load_panel_brand(paths: &AppPaths) -> Option<PanelBrand> {
    if !kg_brand::has_server() {
        return None;
    }
    let dir = folder(paths);
    let json = std::fs::read_to_string(dir.join(BRAND_FILE)).ok()?;
    let mut brand: PanelBrand = serde_json::from_str(&json).ok()?;
    for (_, file) in IMAGES {
        if let Ok(bytes) = std::fs::read(dir.join(file))
            && image_ok(file, &bytes)
        {
            brand.images.insert(file.to_owned(), bytes);
        }
    }
    Some(brand)
}

/// Keeps a downloaded brand; pictures it no longer has are removed. The
/// JSON is written last, so a half-written brand is never read.
pub(crate) fn save_panel_brand(paths: &AppPaths, brand: &PanelBrand) -> std::io::Result<()> {
    let dir = folder(paths);
    std::fs::create_dir_all(&dir)?;
    for (_, file) in IMAGES {
        match brand.images.get(file) {
            Some(bytes) => write(&dir, file, bytes)?,
            None => match std::fs::remove_file(dir.join(file)) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err),
                _ => {}
            },
        }
    }
    let json = serde_json::to_vec(brand).map_err(std::io::Error::other)?;
    write(&dir, BRAND_FILE, &json)
}

fn write(dir: &Path, file: &str, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = dir.join(format!("{file}.tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, dir.join(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(side: u32) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(side, side)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn only_safe_pictures_are_kept() {
        assert!(image_ok("icon.png", &png(256)));
        assert!(!image_ok("icon.png", &png(32)), "too small");
        assert!(!image_ok("icon.png", b"\x89PNG\r\n\x1a\nnot really"));
        assert!(!image_ok("icon.png", b"<svg/>"));
        assert!(image_ok(
            "mark.svg",
            br#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h1"/></svg>"#
        ));
        for bad in [
            r#"<svg><script>alert(1)</script></svg>"#,
            r#"<svg><image href="https://tracker.example/x.png"/></svg>"#,
            r#"<svg><foreignObject/></svg>"#,
            r#"<svg style="background:url(http://x)"/>"#,
            "no svg here",
        ] {
            assert!(!image_ok("mark.svg", bad.as_bytes()), "{bad}");
        }
    }

    #[test]
    fn texts_colours_and_picture_addresses_are_checked() {
        let brand = PanelBrand::from_json(&serde_json::json!({
            "colors": {"company": "#2F6F68"},
            "texts": {
                "demo.expired_body": "  Fale com a escola.  ",
                "Bad Key": "x",
                "empty": "",
                "long": "x".repeat(2001),
                "number": 3,
            },
        }));
        assert_eq!(brand.company_color, None, "the handout's own colour");
        assert_eq!(
            brand.texts,
            BTreeMap::from([(
                "demo.expired_body".to_owned(),
                "Fale com a escola.".to_owned()
            )])
        );
        let urls = image_urls(&serde_json::json!({"images": {
            "icon": "https://cdn.example/icon.png",
            "cover": "http://elsewhere.example/cover.png",
            "mark": null,
        }}));
        assert_eq!(
            urls[0],
            ("icon.png", Some("https://cdn.example/icon.png".into()))
        );
        assert_eq!(urls[1], ("cover.png", None));
        assert_eq!(urls[3], ("mark.svg", None));
    }

    #[test]
    fn a_kept_brand_comes_back_with_its_pictures_and_drops_removed_ones() {
        if !kg_brand::has_server() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path());
        let mut brand = PanelBrand::from_json(&serde_json::json!({"app_name": "Escola"}));
        brand.images.insert("icon.png".into(), png(128));
        brand.images.insert("mark.svg".into(), b"<svg/>".to_vec());
        save_panel_brand(&paths, &brand).unwrap();
        assert_eq!(load_panel_brand(&paths), Some(brand.clone()));

        brand.images.remove("mark.svg");
        save_panel_brand(&paths, &brand).unwrap();
        let loaded = load_panel_brand(&paths).unwrap();
        assert!(loaded.image("mark.svg").is_none());
        assert!(loaded.image("icon.png").is_some());
    }
}
