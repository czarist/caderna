//! The brand as shown: the build's (`kg-brand`), with what the brand's panel
//! sent at an earlier start laid over it. Set once at startup, so names and
//! colours never change in the middle of a session.

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
