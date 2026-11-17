//! The brand as shown: the build's (`kg-brand`), with what the brand's panel
//! sent laid over it: names, colours, links, pictures and texts. The brand
//! kept at an earlier start applies when the app opens; a newer one from the
//! panel replaces it as soon as it arrives ([`refresh`]).

use std::borrow::Cow;
use std::sync::RwLock;

use kg_engine::PanelBrand;

/// The brands are leaked so the text and picture getters stay `'static`.
/// A brand changes only when the client edits it in the panel, so a session
/// leaks one or two at most.
static PANEL: RwLock<Option<&'static PanelBrand>> = RwLock::new(None);

/// Presentation mode (developer area): a client's brand is on screen, so
/// nothing shows that this is a development run.
static PRESENTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_presenting(on: bool) {
    PRESENTING.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Changes every time another brand is laid over the build's, so the
/// taskbar and tray icons follow it.
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

fn presenting() -> bool {
    PRESENTING.load(std::sync::atomic::Ordering::Relaxed)
}

/// Lays the panel's brand over the build's (`None`: the build's again).
/// Returns whether it changed.
pub fn apply(brand: Option<&PanelBrand>) -> bool {
    let Some(brand) = brand else {
        if panel().is_none() {
            return false;
        }
        match PANEL.write() {
            Ok(mut current) => *current = None,
            Err(poisoned) => *poisoned.into_inner() = None,
        }
        GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return true;
    };
    if panel() == Some(brand) {
        return false;
    }
    let brand: &'static PanelBrand = Box::leak(Box::new(brand.clone()));
    match PANEL.write() {
        Ok(mut current) => *current = Some(brand),
        Err(poisoned) => *poisoned.into_inner() = Some(brand),
    }
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    true
}

/// Shows a brand that arrived during the session: colours, pictures, the
/// window's title and icon. Texts and names are read on every frame.
pub fn refresh(ctx: &egui::Context, brand: Option<&PanelBrand>) {
    if !apply(brand) {
        return;
    }
    crate::theme::apply(ctx);
    for uri in crate::art::BRAND_URIS {
        ctx.forget_image(uri);
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title()));
    ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(
        crate::icon::app_icon(),
    ))));
}

fn panel() -> Option<&'static PanelBrand> {
    match PANEL.read() {
        Ok(current) => *current,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// The channel this run shows: the developer-mode environment when one is
/// on (Phase W9), otherwise the build's channel.
pub fn channel_id() -> &'static str {
    match kg_engine::developer::active().environment {
        kg_engine::Environment::Production => kg_brand::CHANNEL,
        other => other.id(),
    }
}

/// The build's channel in the UI language ("HOMOLOGAÇÃO", "DESENVOLVIMENTO");
/// `None` in production and for brands without a panel.
pub fn channel_label() -> Option<String> {
    if presenting() {
        return None;
    }
    match channel_id() {
        "hom" => Some(t!("channel.hom").into_owned()),
        "dev" => Some(t!("channel.dev").into_owned()),
        _ => None,
    }
}

/// The window's title: the product name, and the channel outside production.
pub fn window_title() -> String {
    let name = t!("app.name");
    match channel_label() {
        Some(label) => format!("{name} — {label}"),
        None => name.into_owned(),
    }
}

/// Product name shown to students.
pub fn name() -> &'static str {
    panel()
        .and_then(|b| b.app_name.as_deref())
        .unwrap_or(kg_brand::NAME)
}

pub fn company() -> &'static str {
    panel()
        .and_then(|b| b.company_name.as_deref())
        .unwrap_or(kg_brand::COMPANY)
}

pub fn support_url() -> &'static str {
    panel()
        .and_then(|b| b.support_url.as_deref())
        .unwrap_or(kg_brand::SUPPORT_URL)
}

/// Where to get a licence (the "demo ended" and activation screens).
pub fn contact_url() -> &'static str {
    panel()
        .and_then(|b| b.contact_url.as_deref())
        .unwrap_or(kg_brand::DEMO_CONTACT_URL)
}

/// A picture of the brand by file name (`icon.png`, `cover.png`,
/// `engraving.png`, `mark.svg`), or the build's own.
pub fn image(file: &str, fallback: &'static [u8]) -> &'static [u8] {
    panel().and_then(|b| b.image(file)).unwrap_or(fallback)
}

/// The client's text for a locale key. The panel's texts are written in the
/// brand's main language, so other languages keep the app's own texts.
pub fn text(key: &str) -> Option<&'static str> {
    let main = kg_brand::LANGUAGES.first()?;
    if &*rust_i18n::locale() != *main {
        return None;
    }
    panel()?.texts.get(key).map(String::as_str)
}

/// Fills `%{name}` placeholders, as the translation files do.
pub fn fill(text: &str, values: &[(&str, String)]) -> Cow<'static, str> {
    let mut out = text.to_owned();
    for (name, value) in values {
        out = out.replace(&format!("%{{{name}}}"), value);
    }
    Cow::Owned(out)
}

/// Accent and highlight colours for the light and dark themes.
pub struct Colors {
    pub accent_light: [u8; 3],
    pub accent_dark: [u8; 3],
    pub highlight_light: [u8; 3],
    pub highlight_dark: [u8; 3],
}

pub fn colors() -> Colors {
    let p = panel();
    let pick = |value: Option<[u8; 3]>, fallback: [u8; 3]| value.unwrap_or(fallback);
    Colors {
        accent_light: pick(p.and_then(|b| b.accent_light), kg_brand::ACCENT_LIGHT),
        accent_dark: pick(p.and_then(|b| b.accent_dark), kg_brand::ACCENT_DARK),
        highlight_light: pick(p.and_then(|b| b.highlight_light), kg_brand::HIGHLIGHT_LIGHT),
        highlight_dark: pick(p.and_then(|b| b.highlight_dark), kg_brand::HIGHLIGHT_DARK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_texts_fill_the_same_placeholders_as_the_translations() {
        let text = fill(
            "Fale com %{company} sobre o %{app}: %{days} dias.",
            &[
                ("app", "Escola Aprova".into()),
                ("company", "Escola".into()),
                ("days", 3.to_string()),
            ],
        );
        assert_eq!(text, "Fale com Escola sobre o Escola Aprova: 3 dias.");
    }

    #[test]
    fn without_a_panel_brand_the_build_brand_shows() {
        // Tests never apply a panel brand (it would leak into other tests).
        assert_eq!(name(), kg_brand::NAME);
        assert_eq!(image("icon.png", kg_brand::ICON_PNG), kg_brand::ICON_PNG);
        assert_eq!(text("app.name"), None);
    }
}
