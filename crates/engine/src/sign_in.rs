//! Signing in with email and password, for students and teachers of a brand
//! with a panel. The panel answers like a licence activation (device token
//! and the signed licence key, which the app keeps), so everything after the
//! sign-in is the same as with a typed key.
//!
//! After two failed attempts the panel asks for the code shown in a picture
//! it draws itself (a captcha); "forgot my password" always asks for it,
//! since it sends an email. Each picture works once.
//!
//! The password goes only to the panel, over HTTPS, and is never kept here.

use std::sync::Arc;

use base64::Engine as _;
use serde::Deserialize;

use crate::server::{Activation, HttpPanel, PanelError, Refusal, failure, platform};
use crate::{EngineError, Event};

/// Largest captcha picture accepted, in bytes.
const MAX_CAPTCHA_BYTES: usize = 256 * 1024;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// A picture with a code to type, from the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captcha {
    pub id: String,
    /// The PNG picture, shared so the UI can show it without copying.
    pub image_png: Arc<[u8]>,
}

/// The sign-in form's state in the engine.
#[derive(Debug, Default)]
pub(crate) struct SignIn {
    captcha: Option<Captcha>,
    /// The next attempt needs a captcha.
    needs_captcha: bool,
    loading_captcha: bool,
    /// The last captcha could not be loaded.
    captcha_failed: bool,
    resetting: bool,
    reset_sent: bool,
}

/// What the sign-in form shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignInView<'a> {
    pub captcha: Option<&'a Captcha>,
    pub needs_captcha: bool,
    pub loading_captcha: bool,
    pub captcha_failed: bool,
    /// A password-reset request is on its way.
    pub resetting: bool,
    /// The panel accepted a password-reset request.
    pub reset_sent: bool,
}

/// What the background calls send back to the engine.
#[derive(Debug)]
pub(crate) enum SignInEvent {
    SignedIn {
        result: Result<Activation, PanelError>,
        /// The panel says the next attempt needs a captcha.
        captcha_required: bool,
    },
    CaptchaLoaded(Result<Captcha, PanelError>),
    ResetRequested(Result<(), PanelError>),
}

/// An email the panel could hold: something before and after a single `@`.
pub fn plausible_email(email: &str) -> bool {
    let email = email.trim();
    email.len() <= 254
        && !email.contains(char::is_whitespace)
        && email.split_once('@').is_some_and(|(user, host)| {
            !user.is_empty() && host.contains('.') && !host.contains('@')
        })
}

impl crate::Engine {
    /// Typing a licence key (instead of signing in) is offered only in
    /// developer mode, development builds and brands without a panel. The
    /// panel refuses keys of enrolled people in production anyway.
    pub fn key_entry_allowed(&self) -> bool {
        !kg_brand::has_server()
            || cfg!(debug_assertions)
            || crate::developer::active().claims.is_some()
    }

    pub fn sign_in_view(&self) -> SignInView<'_> {
        let s = &self.panel.sign_in;
        SignInView {
            captcha: s.captcha.as_ref(),
            needs_captcha: s.needs_captcha,
            loading_captcha: s.loading_captcha,
            captcha_failed: s.captcha_failed,
            resetting: s.resetting,
            reset_sent: s.reset_sent,
        }
    }

    /// Signs in with the panel. `captcha` is the code typed from the picture,
    /// used only when the panel asked for one.
    pub(crate) fn sign_in(
        &mut self,
        email: &str,
        password: &str,
        captcha: &str,
    ) -> Result<(), EngineError> {
        let email = email.trim().to_lowercase();
        if !kg_brand::has_server() || !plausible_email(&email) || password.is_empty() {
            return Ok(());
        }
        if self.panel.busy {
            return Ok(());
        }
        let captcha = if self.panel.sign_in.needs_captcha {
            // Each picture works once, right or wrong.
            match self.panel.sign_in.captcha.take() {
                Some(picture) if !captcha.trim().is_empty() => {
                    Some((picture.id, captcha.trim().to_owned()))
                }
                picture => {
                    self.panel.sign_in.captcha = picture;
                    if self.panel.sign_in.captcha.is_none() {
                        self.load_captcha();
                    }
                    return Ok(());
                }
            }
        } else {
            None
        };
        let mut body = serde_json::json!({
            "email": email,
            "password": password,
            "device_id": self.device(),
            "platform": platform(),
            "app_version": env!("CARGO_PKG_VERSION"),
        });
        if let Some((id, code)) = captcha {
            body["captcha_id"] = id.into();
            body["captcha_code"] = code.into();
        }
        self.panel.busy = true;
        self.panel.sign_in.reset_sent = false;
        self.spawn(move || {
            let (result, captcha_required) = post_sign_in(&HttpPanel::new(), &body);
            Event::SignIn(SignInEvent::SignedIn {
                result,
                captcha_required,
            })
        });
        Ok(())
    }

    /// Asks the panel for a new captcha picture.
    pub(crate) fn load_captcha(&mut self) {
        if !kg_brand::has_server() || self.panel.sign_in.loading_captcha {
            return;
        }
        self.panel.sign_in.loading_captcha = true;
        self.panel.sign_in.captcha_failed = false;
        self.spawn(|| Event::SignIn(SignInEvent::CaptchaLoaded(fetch_captcha(&HttpPanel::new()))));
    }

    /// Asks the panel to email a link for a new password. Needs a captcha.
    pub(crate) fn request_password_reset(
        &mut self,
        email: &str,
        captcha: &str,
    ) -> Result<(), EngineError> {
        let email = email.trim().to_lowercase();
        if !kg_brand::has_server()
            || !plausible_email(&email)
            || captcha.trim().is_empty()
            || self.panel.sign_in.resetting
        {
            return Ok(());
        }
        let Some(picture) = self.panel.sign_in.captcha.take() else {
            self.load_captcha();
            return Ok(());
        };
        let body = serde_json::json!({
            "email": email,
            "captcha_id": picture.id,
            "captcha_code": captcha.trim(),
        });
        self.panel.sign_in.resetting = true;
        self.panel.sign_in.reset_sent = false;
        self.spawn(move || {
            Event::SignIn(SignInEvent::ResetRequested(post_reset(
                &HttpPanel::new(),
                &body,
            )))
        });
        Ok(())
    }

    pub(crate) fn on_sign_in_event(&mut self, event: SignInEvent) -> Result<(), EngineError> {
        match event {
            SignInEvent::SignedIn {
                result,
                captcha_required,
            } => match result {
                Ok(activation) => {
                    let Some(key) = activation.license_key.clone() else {
                        self.panel.busy = false;
                        return Err(PanelError::Unexpected {
                            status: 201,
                            detail: "sign-in without a licence key".into(),
                        }
                        .into());
                    };
                    self.panel.sign_in = SignIn::default();
                    self.on_panel_activated(&key, Ok(activation))
                }
                Err(err) => {
                    self.panel.busy = false;
                    let asks = captcha_required
                        || matches!(
                            err,
                            PanelError::Refused(Refusal::CaptchaRequired | Refusal::CaptchaInvalid)
                        );
                    if asks {
                        self.panel.sign_in.needs_captcha = true;
                        self.panel.sign_in.captcha = None;
                        self.load_captcha();
                    }
                    // Asking for the picture is not an error to show.
                    if err == PanelError::Refused(Refusal::CaptchaRequired) {
                        return Ok(());
                    }
                    Err(err.into())
                }
            },
            SignInEvent::CaptchaLoaded(result) => {
                self.panel.sign_in.loading_captcha = false;
                match result {
                    Ok(captcha) => {
                        self.panel.sign_in.captcha = Some(captcha);
                        Ok(())
                    }
                    Err(err) => {
                        self.panel.sign_in.captcha_failed = true;
                        Err(err.into())
                    }
                }
            }
            SignInEvent::ResetRequested(result) => {
                self.panel.sign_in.resetting = false;
                match result {
                    Ok(()) => {
                        self.panel.sign_in.reset_sent = true;
                        Ok(())
                    }
                    Err(err) => {
                        // The picture was used; show a new one.
                        self.load_captcha();
                        Err(err.into())
                    }
                }
            }
        }
    }
}

/// `POST /sessions`: the activation, or the error and whether the next
/// attempt needs a captcha.
fn post_sign_in(
    panel: &HttpPanel,
    body: &serde_json::Value,
) -> (Result<Activation, PanelError>, bool) {
    let (status, text) = match panel.post_open("sessions", body) {
        Ok(answer) => answer,
        Err(err) => return (Err(err), false),
    };
    if status == 200 || status == 201 {
        #[derive(Deserialize)]
        struct Envelope {
            data: Activation,
        }
        let result = serde_json::from_str::<Envelope>(&text)
            .map(|e| e.data)
            .map_err(|e| PanelError::Unexpected {
                status,
                detail: e.to_string(),
            });
        return (result, false);
    }
    (Err(failure(status, &text)), captcha_flag(&text))
}

/// `error.captcha_required` in the panel's error envelope.
fn captcha_flag(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/captcha_required")?.as_bool())
        .unwrap_or(false)
}

/// `POST /captchas`.
fn fetch_captcha(panel: &HttpPanel) -> Result<Captcha, PanelError> {
    let (status, text) = panel.post_open("captchas", &serde_json::json!({}))?;
    if status != 200 && status != 201 {
        return Err(failure(status, &text));
    }
    parse_captcha(status, &text)
}

fn parse_captcha(status: u16, text: &str) -> Result<Captcha, PanelError> {
    #[derive(Deserialize)]
    struct Data {
        id: String,
        image_png: String,
    }
    #[derive(Deserialize)]
    struct Envelope {
        data: Data,
    }
    let unexpected = |detail: String| PanelError::Unexpected { status, detail };
    let data = serde_json::from_str::<Envelope>(text)
        .map_err(|e| unexpected(e.to_string()))?
        .data;
    if data.id.is_empty() || data.id.len() > 128 || data.image_png.len() > MAX_CAPTCHA_BYTES * 2 {
        return Err(unexpected("captcha out of bounds".into()));
    }
    let png = base64::engine::general_purpose::STANDARD
        .decode(data.image_png.trim())
        .map_err(|e| unexpected(e.to_string()))?;
    if png.len() > MAX_CAPTCHA_BYTES || !png.starts_with(PNG_SIGNATURE) {
        return Err(unexpected("captcha is not a PNG".into()));
    }
    Ok(Captcha {
        id: data.id,
        image_png: png.into(),
    })
}

/// `POST /password-resets`.
fn post_reset(panel: &HttpPanel, body: &serde_json::Value) -> Result<(), PanelError> {
    let (status, text) = panel.post_open("password-resets", body)?;
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(failure(status, &text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_checked_loosely() {
        assert!(plausible_email("ana@escola.com.br"));
        assert!(plausible_email("  Ana.Souza+1@escola.com "));
        assert!(!plausible_email("ana"));
        assert!(!plausible_email("@escola.com"));
        assert!(!plausible_email("ana@escola"));
        assert!(!plausible_email("ana souza@escola.com"));
        assert!(!plausible_email("ana@@escola.com"));
    }

    #[test]
    fn a_wrong_password_is_a_refusal_with_the_captcha_flag() {
        let body =
            r#"{"error":{"code":"invalid_credentials","message":"x","captcha_required":true}}"#;
        assert_eq!(
            failure(401, body),
            PanelError::Refused(Refusal::InvalidCredentials)
        );
        assert!(captcha_flag(body));
        assert!(!captcha_flag(
            r#"{"error":{"code":"invalid_credentials","message":"x"}}"#
        ));
        // A device token the panel no longer accepts is still Unauthenticated.
        assert_eq!(
            failure(401, r#"{"message":"Unauthenticated."}"#),
            PanelError::Unauthenticated
        );
        assert_eq!(
            failure(429, r#"{"message":"Too Many Attempts."}"#),
            PanelError::Refused(Refusal::TooManyAttempts)
        );
    }

    #[test]
    fn captcha_pictures_must_be_png() {
        let png = base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nrest");
        let ok = format!(
            r#"{{"data":{{"id":"abc","image_png":"{png}","expires_at":"2026-11-23T10:00:00Z"}}}}"#
        );
        let captcha = parse_captcha(201, &ok).expect("a PNG captcha");
        assert_eq!(captcha.id, "abc");
        assert!(captcha.image_png.starts_with(PNG_SIGNATURE));

        let gif = base64::engine::general_purpose::STANDARD.encode(b"GIF89a....");
        let bad = format!(r#"{{"data":{{"id":"abc","image_png":"{gif}"}}}}"#);
        assert!(parse_captcha(201, &bad).is_err());
        assert!(parse_captcha(201, r#"{"data":{"id":"","image_png":""}}"#).is_err());
    }
}
