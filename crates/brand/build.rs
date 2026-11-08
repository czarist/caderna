// Reads the brand (`KG_BRAND_DIR`, default `branding/default`) and writes
// its values as Rust constants for `src/lib.rs`.

use std::fmt::Write as _;
use std::path::PathBuf;

#[allow(dead_code)]
mod config {
    include!("config.rs");
}

use config::{load_brand, parse_color};

fn main() -> Result<(), String> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let root = manifest.join("..").join("..");
    println!("cargo:rerun-if-env-changed=KG_BRAND_DIR");
    println!("cargo:rerun-if-changed=config.rs");
    let brand = load_brand(&root)?;
    println!("cargo:rerun-if-changed={}", brand.dir.display());
    println!(
        "cargo:rerun-if-changed={}",
        brand.dir.join("brand.toml").display()
    );

    let f = &brand.file;
    let mut out = String::new();
    let mut text = |name: &str, value: &str| {
        let _ = writeln!(out, "pub const {name}: &str = {value:?};");
    };
    text("NAME", &f.app.name);
    text("SLUG", &f.app.slug);
    text("DATA_DIR", &f.app.data_dir);
    text("COMPANY", &f.company.name);
    text("COMPANY_URL", &f.company.url);
    text("COMPANY_URL_LABEL", &f.company.url_label);
    text("MARK_LICENSE", &f.company.mark_license);
    text("REPOSITORY", f.updates.repository.trim_end_matches('/'));
    text("DEMO_CONTACT_URL", &f.demo.contact_url);
    text("COMPANY_COLOR_HEX", &f.colors.company);

    let _ = writeln!(
        out,
        "pub const SHOW_CREDIT: bool = {};",
        f.company.show_credit
    );
    for (name, color) in [
        ("ACCENT_LIGHT", &f.colors.accent_light),
        ("ACCENT_DARK", &f.colors.accent_dark),
        ("HIGHLIGHT_LIGHT", &f.colors.highlight_light),
        ("HIGHLIGHT_DARK", &f.colors.highlight_dark),
        ("COMPANY_COLOR", &f.colors.company),
    ] {
        let _ = writeln!(
            out,
            "pub const {name}: [u8; 3] = {:?};",
            parse_color(color)?
        );
    }

    let a = &f.assets;
    for (name, asset) in [
        ("ICON_PNG", &a.icon_png),
        ("COVER_PNG", &a.cover_png),
        ("ENGRAVING_PNG", &a.engraving_png),
        ("MARK_SVG", &a.mark_svg),
        ("WATERMARK_SVG", &a.watermark_svg),
    ] {
        let path = brand.asset(asset);
        println!("cargo:rerun-if-changed={}", path.display());
        let path = path.display().to_string();
        let _ = writeln!(out, "pub const {name}: &[u8] = include_bytes!({path:?});");
    }

    let _ = writeln!(out, "pub const DEMO: bool = {};", f.demo.enabled);
    let days = if f.demo.enabled { f.demo.days } else { 0 };
    let _ = writeln!(out, "pub const DEMO_DAYS: u32 = {days};");
    let expires = if f.demo.enabled {
        brand.demo_expires()?
    } else {
        None
    };
    let _ = writeln!(
        out,
        "pub const DEMO_EXPIRES: Option<(i32, u32, u32)> = {expires:?};"
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?);
    std::fs::write(out_dir.join("brand.rs"), out).map_err(|e| e.to_string())
}
