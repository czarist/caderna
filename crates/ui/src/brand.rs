//! The brand as shown: the build's (`kg-brand`), with what the brand's panel
//! sent at an earlier start laid over it: names, colours, links, pictures and
//! texts. Set once at startup, so nothing changes in the middle of a session.

use std::borrow::Cow;
use std::sync::OnceLock;

use kg_engine::PanelBrand;

static PANEL: OnceLock<PanelBrand> = OnceLock::new();

/// Lays the panel's brand over the build's. Only the first call counts.
pub fn apply(brand: Option<&PanelBrand>) {
    if let Some(brand) = brand {
        let _ = PANEL.set(brand.clone());
    }
}

fn panel() -> Option<&'static PanelBrand> {
    PANEL.get()
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
