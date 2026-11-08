//! The brand a build carries: product and company names, colours, pictures,
//! where updates come from and, for demonstrations, how long it may be used.
//!
//! Chosen at compile time from a `brand.toml` folder: the `KG_BRAND_DIR`
//! environment variable, or `branding/default` (Know Giver) without it.
//! See `branding/README.md`.

include!(concat!(env!("OUT_DIR"), "/brand.rs"));

/// Updates and the source-code link come from a GitHub repository.
pub fn has_repository() -> bool {
    !REPOSITORY.is_empty()
}

/// The credits ("Made by …") can open the company's site.
pub fn has_company_url() -> bool {
    !COMPANY_URL.is_empty()
}

/// HTTP user agent for requests the app makes, e.g. `"update check"`.
pub fn user_agent(purpose: &str) -> String {
    let site = if has_repository() {
        REPOSITORY
    } else {
        COMPANY_URL
    };
    format!(
        "{}/{} ({purpose}; {site})",
        DATA_DIR,
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_is_complete() {
        assert!(!NAME.is_empty() && !COMPANY.is_empty());
        assert!(
            SLUG.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        );
        assert!(ICON_PNG.starts_with(b"\x89PNG"));
        assert!(COVER_PNG.starts_with(b"\x89PNG"));
        assert!(ENGRAVING_PNG.starts_with(b"\x89PNG"));
        assert!(std::str::from_utf8(MARK_SVG).is_ok_and(|s| s.contains("<svg")));
        assert!(std::str::from_utf8(WATERMARK_SVG).is_ok_and(|s| s.contains("<svg")));
        if DEMO {
            assert!(DEMO_DAYS != 0 || DEMO_EXPIRES.is_some());
        }
        assert!(user_agent("test").starts_with(DATA_DIR));
    }
}
