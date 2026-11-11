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

/// Licences, the brand and AI go through the brand's panel ([`SERVER_URL`]).
pub fn has_server() -> bool {
    !SERVER_URL.is_empty()
}

/// The brand offers this UI language (`pt-BR`, `en`, …).
pub fn offers_language(locale: &str) -> bool {
    LANGUAGES.contains(&locale)
}

/// The brand offers this region (`br`, `us`, …, as saved in settings).
pub fn offers_region(region: &str) -> bool {
    REGIONS.contains(&region)
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
        // A demonstration needs a limit; the values differ per brand.
        let limited = std::hint::black_box(DEMO_DAYS) != 0 || DEMO_EXPIRES.is_some();
        assert!(!DEMO || limited);
        assert!(user_agent("test").starts_with(DATA_DIR));
        assert!(!LANGUAGES.is_empty() && !REGIONS.is_empty());
        for locale in LANGUAGES {
            assert!(!legal_document(&format!("terms-of-use.{locale}")).is_empty());
            assert!(!legal_document(&format!("privacy-policy.{locale}")).is_empty());
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod config {
    include!("../config.rs");

    #[test]
    fn each_channel_talks_to_its_own_panel() {
        let server = ServerSection {
            url: "https://painel.example.com".into(),
            hom_url: "https://hom.painel.example.com".into(),
            dev_url: "http://localhost:8080".into(),
            text_model: "t".into(),
            vision_model: "v".into(),
        };
        assert_eq!(
            server.url_for(Channel::Prod, None).as_deref(),
            Ok("https://painel.example.com")
        );
        assert_eq!(
            server.url_for(Channel::Hom, None).as_deref(),
            Ok("https://hom.painel.example.com")
        );
        assert_eq!(
            server.url_for(Channel::Dev, None).as_deref(),
            Ok("http://localhost:8080")
        );
        // KG_SERVER_URL wins, without a trailing slash, and must be https.
        assert_eq!(
            server
                .url_for(Channel::Prod, Some("https://outro.example.com/"))
                .as_deref(),
            Ok("https://outro.example.com")
        );
        assert!(
            server
                .url_for(Channel::Prod, Some("http://evil.example.com"))
                .is_err()
        );

        let dev_only = ServerSection {
            url: String::new(),
            hom_url: String::new(),
            ..server
        };
        assert!(
            dev_only.url_for(Channel::Prod, None).is_err(),
            "never ship localhost"
        );
        assert!(dev_only.url_for(Channel::Hom, None).is_err());
        assert_eq!(
            ServerSection::default()
                .url_for(Channel::Prod, None)
                .as_deref(),
            Ok("")
        );
    }

    #[test]
    fn channels_install_apart() {
        assert_eq!(Channel::parse(None, true), Ok(Channel::Prod));
        assert_eq!(Channel::parse(None, false), Ok(Channel::Dev));
        assert_eq!(Channel::parse(Some(" hom "), true), Ok(Channel::Hom));
        assert!(Channel::parse(Some("staging"), true).is_err());
        let guid = "F104D038-F0C6-44A0-9C76-2FC42659D335";
        assert_eq!(Channel::Prod.installer_guid(guid), guid);
        let hom = Channel::Hom.installer_guid(guid);
        let dev = Channel::Dev.installer_guid(guid);
        assert!(hom != guid && dev != guid && hom != dev);
        assert!(is_guid(&hom) && is_guid(&dev));
        assert!(hom.starts_with("F104D038-F0C6-44A0-9C76-"));
        // Stable between builds.
        assert_eq!(hom, Channel::Hom.installer_guid(guid));
    }
}
