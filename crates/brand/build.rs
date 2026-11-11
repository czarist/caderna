// Reads the brand (`KG_BRAND_DIR`, default `branding/default`) and writes
// its values as Rust constants for `src/lib.rs`.

use std::fmt::Write as _;
use std::path::PathBuf;

#[allow(dead_code)]
mod config {
    include!("config.rs");
}

use config::{Channel, load_brand, parse_color};

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
    // Brands with a panel build per channel (KG_CHANNEL): production,
    // homologation or development, each installed as a program of its own.
    println!("cargo:rerun-if-env-changed=KG_CHANNEL");
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let channel = if f.server.has_panel() {
        Channel::parse(std::env::var("KG_CHANNEL").ok().as_deref(), release)?
    } else {
        Channel::Prod
    };
    let suffix = channel.suffix();
    let joined = |base: &str, sep: &str, part: &str| {
        if part.is_empty() {
            base.to_owned()
        } else {
            format!("{base}{sep}{part}")
        }
    };
    let mut out = String::new();
    let mut text = |name: &str, value: &str| {
        let _ = writeln!(out, "pub const {name}: &str = {value:?};");
    };
    text("NAME", &f.app.name);
    // The licence's brand: the same in every channel.
    text("SLUG", &f.app.slug);
    text("CHANNEL", channel.id());
    text(
        "INSTALL_NAME",
        &joined(&f.app.name, " ", &suffix.to_uppercase()),
    );
    text(
        "INSTALL_SLUG",
        &joined(&f.app.slug, "-", &suffix.to_lowercase()),
    );
    text("DATA_DIR", &joined(&f.app.data_dir, "-", suffix));
    text("EXECUTABLE", &joined(&f.app.executable, "-", suffix));
    text(
        "INSTALLER_GUID",
        &channel.installer_guid(&f.app.installer_guid),
    );
    text(
        "ICON_PNG_PATH",
        &brand.asset(&f.assets.icon_png).to_string_lossy(),
    );
    text(
        "ICON_ICO_PATH",
        &brand.asset(&f.assets.icon_ico).to_string_lossy(),
    );
    text("COMPANY", &f.company.name);
    text("COMPANY_URL", &f.company.url);
    text("COMPANY_URL_LABEL", &f.company.url_label);
    text("SUPPORT_URL", &f.company.support_url);
    text("LICENSE_PUBLIC_KEY", &f.licensing.public_key);
    text(
        "TERMS_VERSION",
        if f.legal.version.is_empty() {
            "2026-11-08.1"
        } else {
            &f.legal.version
        },
    );
    // Contact every legal text names; docs/legal names Trentin's site.
    text(
        "LEGAL_CONTACT",
        if f.legal.contact.is_empty() {
            "trentin.software"
        } else {
            &f.legal.contact
        },
    );
    text("MARK_LICENSE", &f.company.mark_license);
    // Only production updates itself: the releases there are production's.
    text(
        "REPOSITORY",
        if channel == Channel::Prod {
            f.updates.repository.trim_end_matches('/')
        } else {
            ""
        },
    );
    // Development builds talk to dev_url (or KG_SERVER_URL from the shell or
    // the repository's .env); release builds to url (or KG_SERVER_URL from the
    // shell or CI), never to a file on the student's computer.
    println!("cargo:rerun-if-env-changed=KG_SERVER_URL");
    let mut server_override = std::env::var("KG_SERVER_URL").ok();
    if !release && server_override.is_none() {
        let dotenv = root.join(".env");
        println!("cargo:rerun-if-changed={}", dotenv.display());
        server_override = std::fs::read_to_string(&dotenv).ok().and_then(|text| {
            text.lines().find_map(|line| {
                let value = line.trim().strip_prefix("KG_SERVER_URL=")?;
                Some(value.trim().trim_matches(['"', '\'']).to_owned())
            })
        });
    }
    let server_url = f.server.url_for(channel, server_override.as_deref())?;
    text("SERVER_URL", &server_url);
    text("SERVER_TEXT_MODEL", f.server.text_model.trim());
    text("SERVER_VISION_MODEL", f.server.vision_model.trim());
    text("DEMO_CONTACT_URL", &f.demo.contact_url);
    text("COMPANY_COLOR_HEX", &f.colors.company);
    let offered = |all: &[&str], chosen: &[String]| -> Vec<String> {
        all.iter()
            .filter(|item| chosen.is_empty() || chosen.iter().any(|c| c == *item))
            .map(|item| (*item).to_owned())
            .collect()
    };
    let _ = writeln!(
        out,
        "pub const LANGUAGES: &[&str] = &{:?};",
        offered(&config::LANGUAGES, &f.locale.languages)
    );
    let _ = writeln!(
        out,
        "pub const REGIONS: &[&str] = &{:?};",
        offered(&config::REGIONS, &f.locale.regions)
    );
    let _ = writeln!(
        out,
        "pub const LICENSE_REQUIRED: bool = {};",
        f.licensing.required
    );
    let _ = writeln!(
        out,
        "pub const LEGAL_REVIEWED: bool = {};",
        f.legal.reviewed
    );
    let _ = writeln!(
        out,
        "pub const COMMERCIAL_REVIEWED: bool = {};",
        f.legal.commercial_reviewed
    );

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
    out.push_str("pub fn legal_document(name: &str) -> &'static str { match name {\n");
    // Only the texts the brand can show: its languages, and the partner terms
    // only when it offers a region outside the public ones.
    let partners = f.locale.offers_region("cn") || f.locale.offers_region("other");
    for kind in ["terms-of-use", "privacy-policy", "partner-access"] {
        if kind == "partner-access" && !partners {
            continue;
        }
        for locale in config::LANGUAGES {
            if !f.locale.offers_language(locale) {
                continue;
            }
            let name = format!("{kind}.{locale}");
            let folder = if f.legal.directory.is_empty() {
                root.join("docs/legal")
            } else {
                brand.dir.join(&f.legal.directory)
            };
            let path = folder.join(format!("{name}.md"));
            println!("cargo:rerun-if-changed={}", path.display());
            let mut body =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            for (key, value) in [
                ("app", f.app.name.as_str()),
                ("company", f.company.name.as_str()),
                ("provider", f.legal.provider.as_str()),
                ("licensor", f.legal.licensor.as_str()),
                ("contact", f.legal.contact.as_str()),
                ("data_dir", f.app.data_dir.as_str()),
                ("version", f.legal.version.as_str()),
            ] {
                body = body.replace(&format!("{{{{{key}}}}}"), value);
            }
            if body.contains("{{") {
                return Err(format!(
                    "unresolved legal placeholder in {}",
                    path.display()
                ));
            }
            let _ = writeln!(out, "{name:?} => {body:?},");
        }
    }
    out.push_str("_ => \"\", } }\n");
    out.push_str(
        "#[allow(clippy::match_single_binding)]\npub fn artwork(kind: &str, name: &str) -> Option<&'static [u8]> { match (kind, name) {\n",
    );
    for (kind, directory) in [("teachers", &a.teachers_dir), ("stickers", &a.stickers_dir)] {
        if directory.is_empty() {
            continue;
        }
        let folder = brand.dir.join(directory);
        println!("cargo:rerun-if-changed={}", folder.display());
        for path in artwork_files(&folder)? {
            let name = path
                .strip_prefix(&folder)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let path = path.canonicalize().map_err(|e| e.to_string())?;
            let path = path.to_string_lossy();
            let _ = writeln!(
                out,
                "({kind:?}, {name:?}) => Some(include_bytes!({path:?})),"
            );
        }
    }
    out.push_str("_ => None, } }\n");
    std::fs::write(out_dir.join("brand.rs"), out).map_err(|e| e.to_string())
}

fn artwork_files(folder: &std::path::Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            files.extend(artwork_files(&entry.path())?);
        } else if kind.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}
