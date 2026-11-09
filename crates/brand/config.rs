// The brand file (`brand.toml`) and its checks. Shared by the build scripts
// of `kg-brand` and of the app (`include!`), so both read the same brand.
//
// The brand folder comes from the `KG_BRAND_DIR` environment variable at
// build time; a relative path starts at the repository root. Without it the
// build uses `branding/default`.

use std::path::{Path, PathBuf};

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrandFile {
    pub app: AppSection,
    pub company: CompanySection,
    pub colors: ColorSection,
    pub assets: AssetSection,
    #[serde(default)]
    pub updates: UpdateSection,
    #[serde(default)]
    pub demo: DemoSection,
    #[serde(default)]
    pub legal: LegalSection,
    #[serde(default)]
    pub licensing: LicensingSection,
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub locale: LocaleSection,
}

/// UI languages a brand can offer (`locales/*.yml`).
pub const LANGUAGES: [&str; 5] = ["pt-BR", "en", "ja", "zh-Hans", "zh-Hant"];

/// Region ids as saved in settings (engine `Region`).
pub const REGIONS: [&str; 10] = ["br", "us", "eu", "uk", "jp", "tw", "hk", "sg", "cn", "other"];

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSection {
    /// The brand's panel (CADERNA-DASHBOARD), e.g. `https://painel.example.com`.
    /// With it the app activates licences online, downloads the brand and
    /// uses AI through the panel. Empty: offline licences only.
    #[serde(default)]
    pub url: String,
    /// The panel's model names for text tasks and for images (its AI aliases).
    #[serde(default)]
    pub text_model: String,
    #[serde(default)]
    pub vision_model: String,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocaleSection {
    /// UI languages offered, from [`LANGUAGES`]; empty offers all of them.
    #[serde(default)]
    pub languages: Vec<String>,
    /// Regions offered, from [`REGIONS`]; empty offers all of them.
    #[serde(default)]
    pub regions: Vec<String>,
}

impl LocaleSection {
    pub fn offers_language(&self, locale: &str) -> bool {
        self.languages.is_empty() || self.languages.iter().any(|l| l == locale)
    }

    pub fn offers_region(&self, region: &str) -> bool {
        self.regions.is_empty() || self.regions.iter().any(|r| r == region)
    }
}

/// `https://…`, or `http://` only for this computer (development).
pub fn server_url_ok(url: &str) -> bool {
    let local = ["http://localhost", "http://127.0.0.1", "http://[::1]"]
        .iter()
        .any(|prefix| {
            url.strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(':') || rest.starts_with('/'))
        });
    let https = url.strip_prefix("https://").is_some_and(|host| !host.is_empty());
    (local || https) && !url.ends_with('/') && !url.contains(char::is_whitespace)
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSection {
    /// Product name shown everywhere: window, sidebar, texts, installer.
    pub name: String,
    /// Lowercase id (`a-z`, `0-9`, `-`): window id, lock files, database.
    pub slug: String,
    /// Folder for the student's data and the installed program.
    pub data_dir: String,
    /// GUID that tells the Windows installer this product from others.
    pub installer_guid: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanySection {
    pub name: String,
    #[serde(default)]
    pub support_url: String,
    /// Site opened by the credits (`https://…`), or empty.
    #[serde(default)]
    pub url: String,
    /// The site as printed on handouts, e.g. `example.com`.
    #[serde(default)]
    pub url_label: String,
    /// "Made by …" in the sidebar, Settings, whiteboard and handouts.
    #[serde(default = "yes")]
    pub show_credit: bool,
    /// Licence of the mark when it is not the company's own, or empty.
    #[serde(default)]
    pub mark_license: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorSection {
    pub accent_light: String,
    pub accent_dark: String,
    pub highlight_light: String,
    pub highlight_dark: String,
    /// Credit and watermark text on handouts.
    pub company: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSection {
    /// Optional folders with the same filenames as the bundled art.
    #[serde(default)]
    pub teachers_dir: String,
    #[serde(default)]
    pub stickers_dir: String,
    /// App icon, square PNG with transparent corners (512 × 512).
    pub icon_png: String,
    /// Square cover art, corners filled (welcome and empty screens).
    pub cover_png: String,
    /// Artwork of the study reminder pop-up.
    pub engraving_png: String,
    /// Windows executable and installer icon.
    pub icon_ico: String,
    /// Company mark, white, single colour: tinted when drawn in the app.
    pub mark_svg: String,
    /// Company mark already faint and coloured, for the handout watermark.
    pub watermark_svg: String,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateSection {
    /// GitHub repository with releases (`https://github.com/owner/name`).
    /// Empty: no update check and no source-code link.
    #[serde(default)]
    pub repository: String,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DemoSection {
    #[serde(default)]
    pub enabled: bool,
    /// Days of use from the first start; 0 for no limit by days.
    #[serde(default)]
    pub days: u32,
    /// Last day of use, `YYYY-MM-DD`, whatever the first start; empty for none.
    #[serde(default)]
    pub expires: String,
    /// Page the "demo ended" screen opens (`https://…` or `mailto:…`).
    #[serde(default)]
    pub contact_url: String,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegalSection {
    /// The Markdown documents (terms, privacy and, for regions outside the
    /// public ones, partner access) for every language offered, relative to
    /// the brand folder.
    #[serde(default)]
    pub directory: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub licensor: String,
    #[serde(default)]
    pub contact: String,
    #[serde(default)]
    pub reviewed: bool,
    #[serde(default)]
    pub commercial_reviewed: bool,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicensingSection {
    /// Base64 Ed25519 public key. Private keys never belong in a brand.
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub required: bool,
}

/// A loaded brand: its folder and its file.
#[derive(Debug)]
pub struct Brand {
    pub dir: PathBuf,
    pub file: BrandFile,
}

impl Brand {
    /// Absolute path of an asset named in the brand file.
    pub fn asset(&self, relative: &str) -> PathBuf {
        let path = self.dir.join(relative);
        plain(path.canonicalize().unwrap_or(path))
    }

    /// The demo's last day as (year, month, day), if set.
    pub fn demo_expires(&self) -> Result<Option<(i32, u32, u32)>, String> {
        parse_date(&self.file.demo.expires)
    }
}

/// Reads the brand chosen by `KG_BRAND_DIR` and checks it. `repo_root` is
/// the repository root.
pub fn load_brand(repo_root: &Path) -> Result<Brand, String> {
    let dir = match std::env::var_os("KG_BRAND_DIR").filter(|d| !d.is_empty()) {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            if dir.is_absolute() {
                dir
            } else {
                repo_root.join(dir)
            }
        }
        None => repo_root.join("branding").join("default"),
    };
    let dir = plain(
        dir.canonicalize()
            .map_err(|e| format!("brand folder {} not found: {e}", dir.display()))?,
    );
    let path = dir.join("brand.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: BrandFile =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let brand = Brand { dir, file };
    brand.check()?;
    Ok(brand)
}

impl Brand {
    fn check(&self) -> Result<(), String> {
        let f = &self.file;
        let fail = |what: &str| Err(format!("brand.toml: {what}"));
        if f.app.name.trim().is_empty() || f.company.name.trim().is_empty() {
            return fail("app.name and company.name must not be empty");
        }
        if f.app.slug.is_empty()
            || !f
                .app
                .slug
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return fail("app.slug must use only a-z, 0-9 and -");
        }
        if f.app.data_dir.is_empty()
            || !f
                .app
                .data_dir
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return fail("app.data_dir must use only letters, digits, - and _");
        }
        if !is_guid(&f.app.installer_guid) {
            return fail("app.installer_guid must look like 6C1F3F7E-8E0B-4C55-9B7A-3B1E8A4D2F10");
        }
        for (name, url) in [
            ("company.url", &f.company.url),
            ("updates.repository", &f.updates.repository),
        ] {
            if !url.is_empty() && !url.starts_with("https://") {
                return fail(&format!("{name} must start with https:// or be empty"));
            }
        }
        if !f.updates.repository.is_empty()
            && (!f.updates.repository.starts_with("https://github.com/")
                || f.updates.repository.trim_end_matches('/').split('/').count() != 5)
        {
            return fail("updates.repository must be https://github.com/<owner>/<name> or empty");
        }
        let contact = &f.demo.contact_url;
        let support = &f.company.support_url;
        if !support.is_empty() && !support.starts_with("https://") && !support.starts_with("mailto:") {
            return fail("company.support_url must start with https:// or mailto:");
        }
        if !f.server.url.is_empty() && !server_url_ok(&f.server.url) {
            return fail("server.url must be https://host (http:// only for localhost), without a trailing /");
        }
        if !f.server.url.is_empty()
            && (f.server.text_model.trim().is_empty() || f.server.vision_model.trim().is_empty())
        {
            return fail("server.url needs text_model and vision_model (the panel's AI aliases)");
        }
        if !f.server.url.is_empty() && f.licensing.public_key.is_empty() {
            return fail("server.url needs licensing.public_key");
        }
        if let Some(bad) = f.locale.languages.iter().find(|l| !LANGUAGES.contains(&l.as_str())) {
            return fail(&format!("locale.languages: unknown language {bad:?}"));
        }
        if let Some(bad) = f.locale.regions.iter().find(|r| !REGIONS.contains(&r.as_str())) {
            return fail(&format!("locale.regions: unknown region {bad:?}"));
        }
        if f.licensing.required && f.licensing.public_key.is_empty() {
            return fail("licensing.required needs an Ed25519 public_key");
        }
        if !f.legal.directory.is_empty()
            && [f.legal.version.as_str(), f.legal.provider.as_str(), f.legal.licensor.as_str(), f.legal.contact.as_str()]
                .iter().any(|v| v.trim().is_empty())
        {
            return fail("legal overrides need version, provider, licensor and contact");
        }
        if !contact.is_empty() && !contact.starts_with("https://") && !contact.starts_with("mailto:")
        {
            return fail("demo.contact_url must start with https:// or mailto: or be empty");
        }
        if f.demo.enabled && f.demo.days == 0 && f.demo.expires.is_empty() {
            return fail("a demo needs demo.days, demo.expires or both");
        }
        self.demo_expires()?;
        let colors = [
            &f.colors.accent_light,
            &f.colors.accent_dark,
            &f.colors.highlight_light,
            &f.colors.highlight_dark,
            &f.colors.company,
        ];
        for color in colors {
            parse_color(color)?;
        }
        // Handout text in the company colour is left out of the reader, so
        // it must differ from the template's own colours (`handout.typ`).
        let company = parse_color(&f.colors.company)?;
        if [[0x2F, 0x6F, 0x68], [0x9A, 0x6B, 0x1F], [0x3B, 0x7A, 0x4A], [0, 0, 0]]
            .contains(&company)
        {
            return fail("colors.company must differ from the handout's own colours");
        }
        let a = &f.assets;
        for asset in [
            &a.icon_png,
            &a.cover_png,
            &a.engraving_png,
            &a.icon_ico,
            &a.mark_svg,
            &a.watermark_svg,
        ] {
            if !self.dir.join(asset).is_file() {
                return fail(&format!("asset {asset} not found next to brand.toml"));
            }
        }
        Ok(())
    }
}

/// `#RRGGBB` as bytes.
pub fn parse_color(text: &str) -> Result<[u8; 3], String> {
    let hex = text.strip_prefix('#').unwrap_or("");
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("brand.toml: colour {text:?} must be #RRGGBB"));
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string());
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

fn parse_date(text: &str) -> Result<Option<(i32, u32, u32)>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let bad = || format!("brand.toml: date {text:?} must be YYYY-MM-DD");
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(bad());
    }
    let year: i32 = parts[0].parse().map_err(|_| bad())?;
    let month: u32 = parts[1].parse().map_err(|_| bad())?;
    let day: u32 = parts[2].parse().map_err(|_| bad())?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(bad());
    }
    Ok(Some((year, month, day)))
}

/// Drops Windows' `\\?\` prefix, which resource compilers do not accept.
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

fn is_guid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, len)| g.len() == len && g.chars().all(|c| c.is_ascii_hexdigit()))
}
