//! The brand's panel (`[server] url` in brand.toml): online licence
//! activation, a device token renewed in the background, the brand downloaded
//! from the panel and AI through it.
//!
//! The licence key is still checked offline first; the panel adds what an
//! offline key cannot: counting seats, signing a computer out and blocking
//! it. The app keeps working without the panel for [`OFFLINE_GRACE_DAYS`]
//! days after it last reached it. Study data is never touched.
//!
//! What the panel receives: the licence key, a hash of this computer's id,
//! the app version and the operating system (see the privacy policy). Never
//! the computer's name.

use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::panel_brand::{self, PanelBrand};
use crate::{EngineError, Event};

/// Days the app works without reaching the panel.
pub const OFFLINE_GRACE_DAYS: i64 = 14;
/// A token this close to expiring is renewed.
const RENEW_BEFORE: Duration = Duration::hours(24);
const CHECK_EVERY: StdDuration = StdDuration::from_secs(60 * 60);
const RETRY_AFTER_FAILURE: StdDuration = StdDuration::from_secs(5 * 60);

const SESSION_SETTING: &str = "panel_session";
const FALLBACK_ID_SETTING: &str = "panel_device_fallback";
/// Changing it gives every computer a new id, and so a new seat.
const DEVICE_SALT: &str = "kg-device-v1";

/// Why the panel refused this computer (its `error.code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    LicenseInvalid,
    LicenseRevoked,
    LicenseExpired,
    ClientInactive,
    NoSeatsLeft,
    /// Signed out in the panel; the student may activate again.
    DeviceRevoked,
    /// Blocked in the panel until it is unblocked there.
    DeviceBlocked,
}

impl Refusal {
    fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "license_invalid" => Self::LicenseInvalid,
            "license_revoked" => Self::LicenseRevoked,
            "license_expired" => Self::LicenseExpired,
            "client_inactive" => Self::ClientInactive,
            "no_seats_left" => Self::NoSeatsLeft,
            "device_revoked" => Self::DeviceRevoked,
            "device_blocked" => Self::DeviceBlocked,
            _ => return None,
        })
    }
}

/// Talking to the panel failed. Diagnostic text only; the UI uses locale keys.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PanelError {
    #[error("the panel could not be reached: {0}")]
    Unreachable(String),
    #[error("the panel did not accept the device token")]
    Unauthenticated,
    #[error("the panel refused this computer: {0:?}")]
    Refused(Refusal),
    #[error("unexpected answer from the panel ({status}): {detail}")]
    Unexpected { status: u16, detail: String },
}

/// What the student sees about the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelStatus {
    /// This brand has no panel.
    Off,
    /// No licence activated on this computer yet.
    NotActivated,
    /// Talking to the panel right now.
    Busy,
    Connected {
        last_ok: DateTime<Utc>,
    },
    /// The panel was not reached lately; the app works for `days_left` more days.
    Offline {
        last_ok: DateTime<Utc>,
        days_left: i64,
    },
    /// Too long without the panel: the app waits until it reaches it.
    OfflineTooLong {
        last_ok: DateTime<Utc>,
    },
    Refused(Refusal),
}

/// The panel's answer to an activation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Activation {
    pub token: String,
    pub token_expires_at: DateTime<Utc>,
}

/// The panel's API. A trait so the check-in logic can be tested without a network.
pub(crate) trait PanelApi {
    /// `renew`: an automatic renewal, which the panel refuses for a computer
    /// signed out there.
    fn activate(&self, key: &str, device: &str, renew: bool) -> Result<Activation, PanelError>;
    fn current(&self, token: &str) -> Result<(), PanelError>;
    /// `None` when the brand did not change since `etag`.
    fn brand(
        &self,
        token: &str,
        etag: Option<&str>,
    ) -> Result<Option<(serde_json::Value, Option<String>)>, PanelError>;
    fn sign_out(&self, token: &str) -> Result<(), PanelError>;
    /// A picture of the brand (no token: it may live on another host).
    fn download(&self, url: &str) -> Result<Vec<u8>, PanelError>;
}

/// The panel over HTTPS (HTTP only for localhost, checked in the brand).
pub(crate) struct HttpPanel {
    base: String,
    agent: ureq::Agent,
}

impl HttpPanel {
    pub(crate) fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(StdDuration::from_secs(15)))
            .timeout_global(Some(StdDuration::from_secs(30)))
            .user_agent(kg_brand::user_agent("licence"))
            .build()
            .into();
        Self {
            base: format!("{}/api/v1", kg_brand::SERVER_URL),
            agent,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.base)
    }
}

type Answer = (u16, Option<String>, String);

fn read(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Answer, PanelError> {
    let mut response = result.map_err(|e| PanelError::Unreachable(e.to_string()))?;
    let status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| PanelError::Unreachable(e.to_string()))?;
    Ok((status, etag, body))
}

/// The panel's error envelope (`{"error":{"code":…}}`) as a [`PanelError`].
fn failure(status: u16, body: &str) -> PanelError {
    if status == 401 {
        return PanelError::Unauthenticated;
    }
    let code = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/code")?.as_str().map(str::to_owned))
        .unwrap_or_default();
    match Refusal::from_code(&code) {
        Some(refusal) => PanelError::Refused(refusal),
        None => PanelError::Unexpected {
            status,
            detail: if code.is_empty() {
                body.chars().take(200).collect()
            } else {
                code
            },
        },
    }
}

fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

impl PanelApi for HttpPanel {
    fn activate(&self, key: &str, device: &str, renew: bool) -> Result<Activation, PanelError> {
        let body = serde_json::json!({
            "license_key": key,
            "device_id": device,
            "platform": platform(),
            "app_version": env!("CARGO_PKG_VERSION"),
            "renew": renew,
        });
        let (status, _, text) = read(
            self.agent
                .post(&self.url("activations"))
                .header("Accept", "application/json")
                .send_json(&body),
        )?;
        if status != 201 && status != 200 {
            return Err(failure(status, &text));
        }
        #[derive(Deserialize)]
        struct Envelope {
            data: Activation,
        }
        serde_json::from_str::<Envelope>(&text)
            .map(|e| e.data)
            .map_err(|e| PanelError::Unexpected {
                status,
                detail: e.to_string(),
            })
    }

    fn current(&self, token: &str) -> Result<(), PanelError> {
        let (status, _, text) = read(
            self.agent
                .get(&self.url("activations/current"))
                .header("Accept", "application/json")
                .header("Authorization", &format!("Bearer {token}"))
                .call(),
        )?;
        if status == 200 {
            Ok(())
        } else {
            Err(failure(status, &text))
        }
    }

    fn brand(
        &self,
        token: &str,
        etag: Option<&str>,
    ) -> Result<Option<(serde_json::Value, Option<String>)>, PanelError> {
        let mut request = self
            .agent
            .get(&self.url("brand"))
            .header("Accept", "application/json")
            .header("Authorization", &format!("Bearer {token}"));
        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }
        let (status, etag, text) = read(request.call())?;
        match status {
            304 => Ok(None),
            200 => {
                let value: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| PanelError::Unexpected {
                        status,
                        detail: e.to_string(),
                    })?;
                let data = value
                    .get("data")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                Ok(Some((data, etag)))
            }
            _ => Err(failure(status, &text)),
        }
    }

    fn sign_out(&self, token: &str) -> Result<(), PanelError> {
        let (status, _, text) = read(
            self.agent
                .delete(&self.url("activations/current"))
                .header("Accept", "application/json")
                .header("Authorization", &format!("Bearer {token}"))
                .call(),
        )?;
        if (200..300).contains(&status) || status == 401 {
            Ok(())
        } else {
            Err(failure(status, &text))
        }
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, PanelError> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| PanelError::Unreachable(e.to_string()))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(PanelError::Unexpected {
                status,
                detail: format!("picture {url}"),
            });
        }
        response
            .body_mut()
            .with_config()
            .limit(3 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| PanelError::Unreachable(e.to_string()))
    }
}

/// What a background check found.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CheckIn {
    /// A new token, when the old one was renewed.
    pub renewed: Option<Activation>,
    /// The brand with its pictures, when it changed.
    pub brand: Option<BrandDownload>,
}

/// A changed brand, its pictures already checked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BrandDownload {
    pub json: serde_json::Value,
    pub etag: Option<String>,
    pub images: Vec<(&'static str, Vec<u8>)>,
}

/// Downloads the brand's pictures; one that fails or is not safe fails the
/// whole brand, which is tried again at the next check.
fn download_images(
    api: &dyn PanelApi,
    json: &serde_json::Value,
) -> Result<Vec<(&'static str, Vec<u8>)>, PanelError> {
    let mut images = Vec::new();
    for (file, url) in panel_brand::image_urls(json) {
        let Some(url) = url else { continue };
        let bytes = api.download(&url)?;
        if !panel_brand::image_ok(file, &bytes) {
            return Err(PanelError::Unexpected {
                status: 200,
                detail: format!("{file} is not a usable picture"),
            });
        }
        images.push((file, bytes));
    }
    Ok(images)
}

/// Confirms this computer with the panel, renewing the token when it is
/// missing, expiring or no longer accepted, then fetches the brand if it
/// changed. A computer signed out in the panel stays out (`renew`).
pub(crate) fn check_in(
    api: &dyn PanelApi,
    key: &str,
    device: &str,
    token: Option<(&str, DateTime<Utc>)>,
    etag: Option<&str>,
    now: DateTime<Utc>,
) -> Result<CheckIn, PanelError> {
    let usable = token.filter(|(_, expires)| *expires > now + RENEW_BEFORE);
    let mut renewed = None;
    let token = match usable {
        Some((token, _)) => match api.current(token) {
            Ok(()) => token.to_owned(),
            Err(PanelError::Unauthenticated) => {
                let fresh = api.activate(key, device, true)?;
                let token = fresh.token.clone();
                renewed = Some(fresh);
                token
            }
            Err(err) => return Err(err),
        },
        None => {
            let fresh = api.activate(key, device, true)?;
            let token = fresh.token.clone();
            renewed = Some(fresh);
            token
        }
    };
    let brand = match api.brand(&token, etag) {
        Ok(Some((json, etag))) => match download_images(api, &json) {
            Ok(images) => Some(BrandDownload { json, etag, images }),
            Err(err) => {
                tracing::warn!(%err, "could not download the brand's pictures");
                None
            }
        },
        Ok(None) => None,
        Err(err) => {
            tracing::warn!(%err, "could not download the brand from the panel");
            None
        }
    };
    Ok(CheckIn { renewed, brand })
}

/// This computer's id for the panel: the operating system's machine id,
/// hashed with the brand so it means nothing elsewhere. Without a machine id
/// a random one is kept in the database.
fn device_id(store: &mut kg_study::Store) -> String {
    let raw = machine_uid::get()
        .ok()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
        .or_else(|| {
            if let Ok(Some(saved)) = store.setting(FALLBACK_ID_SETTING) {
                return Some(saved);
            }
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).ok()?;
            let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            store.set_setting(FALLBACK_ID_SETTING, &id).ok()?;
            Some(id)
        })
        .unwrap_or_else(|| kg_brand::SLUG.to_owned());
    Sha256::digest(format!("{DEVICE_SALT}:{}:{raw}", kg_brand::SLUG))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Where the device token is kept: the operating system's credential store
/// (Windows Credential Manager, macOS Keychain, Linux kernel keyring), or
/// only memory. When it is lost the app renews it with the licence key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vault {
    System,
    Memory,
}

impl Vault {
    fn entry() -> Option<keyring::Entry> {
        keyring::Entry::new(&format!("{}-panel", kg_brand::SLUG), "device-token")
            .map_err(|err| tracing::warn!(%err, "credential store unavailable"))
            .ok()
    }

    fn load(self) -> Option<String> {
        if self == Self::Memory {
            return None;
        }
        Self::entry()?.get_password().ok()
    }

    fn save(self, token: &str) {
        if self == Self::System
            && let Some(entry) = Self::entry()
            && let Err(err) = entry.set_password(token)
        {
            tracing::warn!(%err, "could not keep the device token in the credential store");
        }
    }

    fn clear(self) {
        if self == Self::System
            && let Some(entry) = Self::entry()
        {
            let _ = entry.delete_credential();
        }
    }
}

/// Saved between runs (no secrets: the token is in the [`Vault`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Session {
    token_expires_at: Option<DateTime<Utc>>,
    /// Last time the panel confirmed this computer.
    last_ok_at: Option<DateTime<Utc>>,
    refused: Option<Refusal>,
    brand_etag: Option<String>,
}

/// The engine's panel state.
#[derive(Debug)]
pub(crate) struct Panel {
    session: Session,
    token: Option<String>,
    vault: Vault,
    device: Option<String>,
    busy: bool,
    /// `None`: no background checks (tests, or nothing to check).
    next_check: Option<Instant>,
    /// The brand of this start (a newer download applies at the next one).
    brand: Option<PanelBrand>,
    /// A brand is kept on disk, so its ETag can be sent.
    brand_kept: bool,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            session: Session::default(),
            token: None,
            vault: Vault::Memory,
            device: None,
            busy: false,
            next_check: None,
            brand: None,
            brand_kept: false,
        }
    }
}

impl crate::Engine {
    /// Loads the panel state. `background`: check with the panel now and
    /// every hour (off in tests).
    pub(crate) fn start_panel(&mut self, vault: Vault, background: bool) {
        if !kg_brand::has_server() {
            return;
        }
        let session = self
            .store
            .setting(SESSION_SETTING)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        let brand = panel_brand::load_panel_brand(&self.paths);
        if let Some(brand) = &brand {
            kg_handout::set_brand(kg_handout::HandoutBrand {
                company: brand.company_name.clone(),
                color: brand.company_color,
                watermark_svg: brand.image("watermark.svg").map(<[u8]>::to_vec),
            });
        }
        self.panel = Panel {
            session,
            token: vault.load(),
            vault,
            device: None,
            busy: false,
            next_check: background.then(Instant::now),
            brand_kept: brand.is_some(),
            brand,
        };
    }

    /// Marks this computer as just confirmed by the panel, for tests of
    /// other modules on a brand with a panel.
    #[cfg(test)]
    pub(crate) fn confirm_panel_for_test(&mut self) {
        self.panel.session.last_ok_at = Some(Utc::now());
    }

    /// The brand downloaded from the panel at an earlier start.
    pub fn panel_brand(&self) -> Option<&PanelBrand> {
        self.panel.brand.as_ref()
    }

    pub fn panel_status(&self) -> PanelStatus {
        self.panel_status_at(Utc::now())
    }

    fn panel_status_at(&self, now: DateTime<Utc>) -> PanelStatus {
        if !kg_brand::has_server() {
            return PanelStatus::Off;
        }
        if self.panel.busy {
            return PanelStatus::Busy;
        }
        if let Some(refusal) = self.panel.session.refused {
            return PanelStatus::Refused(refusal);
        }
        let Some(last_ok) = self
            .panel
            .session
            .last_ok_at
            .filter(|_| self.license.is_some())
        else {
            return PanelStatus::NotActivated;
        };
        let days_left = OFFLINE_GRACE_DAYS - (now - last_ok).num_days();
        if days_left <= 0 {
            PanelStatus::OfflineTooLong { last_ok }
        } else if now - last_ok > Duration::from_std(CHECK_EVERY * 3).unwrap_or(Duration::hours(3))
        {
            PanelStatus::Offline { last_ok, days_left }
        } else {
            PanelStatus::Connected { last_ok }
        }
    }

    /// The panel lets this computer use the app (licence dates are checked apart).
    pub(crate) fn panel_allows_use(&self) -> bool {
        matches!(
            self.panel_status(),
            PanelStatus::Connected { .. } | PanelStatus::Offline { .. } | PanelStatus::Busy
        ) && self.panel.session.last_ok_at.is_some()
    }

    /// AI through the panel, with the device token; `base` keeps the
    /// student's other AI preferences.
    pub(crate) fn panel_ai(&self, base: &kg_ai::AiConfig) -> kg_ai::AiConfig {
        let text = kg_brand::SERVER_TEXT_MODEL.to_owned();
        kg_ai::AiConfig {
            base_url: format!("{}/api/v1/ai", kg_brand::SERVER_URL),
            api_key: self.panel.token.clone().unwrap_or_default(),
            models: kg_ai::ModelSet {
                study_plan: text.clone(),
                tutor: text.clone(),
                quiz: text.clone(),
                correction: text.clone(),
                handout: text.clone(),
                verifier: text,
                vision: kg_brand::SERVER_VISION_MODEL.to_owned(),
                fallback: String::new(),
            },
            fast_responses: base.fast_responses,
        }
    }

    fn device(&mut self) -> String {
        if let Some(device) = &self.panel.device {
            return device.clone();
        }
        let device = device_id(&mut self.store);
        self.panel.device = Some(device.clone());
        device
    }

    fn save_session(&mut self) {
        match serde_json::to_string(&self.panel.session) {
            Ok(json) => {
                if let Err(err) = self.store.set_setting(SESSION_SETTING, &json) {
                    tracing::warn!(%err, "could not save the panel session");
                }
            }
            Err(err) => tracing::warn!(%err, "could not encode the panel session"),
        }
    }

    fn keep_token(&mut self, activation: &Activation) {
        self.panel.vault.save(&activation.token);
        self.panel.token = Some(activation.token.clone());
        self.panel.session.token_expires_at = Some(activation.token_expires_at);
    }

    fn drop_token(&mut self) {
        self.panel.vault.clear();
        self.panel.token = None;
        self.panel.session.token_expires_at = None;
    }

    /// Activates a licence key with the panel; it is kept only once the
    /// panel accepts this computer.
    pub(crate) fn activate_online(&mut self, key: &str) -> Result<(), EngineError> {
        let key = key.trim().to_owned();
        let license = crate::license::verify(&key, kg_brand::LICENSE_PUBLIC_KEY, kg_brand::SLUG)?;
        if !license.valid_on(self.license_day()) {
            return Err(crate::license::LicenseError::Date.into());
        }
        if self.panel.busy {
            return Ok(());
        }
        let device = self.device();
        self.panel.busy = true;
        self.spawn(move || {
            let result = HttpPanel::new().activate(&key, &device, false);
            Event::PanelActivated { key, result }
        });
        Ok(())
    }

    /// Activates this computer again with the licence key already kept.
    pub(crate) fn reactivate_device(&mut self) -> Result<(), EngineError> {
        let key = self
            .license_key()
            .ok_or(crate::license::LicenseError::Invalid)?;
        self.activate_online(&key)
    }

    /// Frees this computer's seat in the panel and forgets the licence here,
    /// so it can be used on another computer. Study data stays.
    pub(crate) fn sign_out_device(&mut self) {
        if self.panel.busy {
            return;
        }
        let Some(token) = self.panel.token.clone() else {
            self.forget_activation();
            return;
        };
        self.panel.busy = true;
        self.spawn(move || Event::PanelSignedOut(HttpPanel::new().sign_out(&token)));
    }

    fn forget_activation(&mut self) {
        self.drop_token();
        self.panel.session = Session::default();
        self.save_session();
        self.forget_license();
    }

    pub(crate) fn on_panel_activated(
        &mut self,
        key: &str,
        result: Result<Activation, PanelError>,
    ) -> Result<(), EngineError> {
        self.panel.busy = false;
        let activation = result?;
        self.store_license(key)?;
        self.keep_token(&activation);
        self.panel.session.last_ok_at = Some(Utc::now());
        self.panel.session.refused = None;
        self.save_session();
        // Fetch the brand right away.
        self.panel.next_check = Some(Instant::now());
        Ok(())
    }

    pub(crate) fn on_panel_signed_out(
        &mut self,
        result: Result<(), PanelError>,
    ) -> Result<(), EngineError> {
        self.panel.busy = false;
        result?;
        self.forget_activation();
        self.panel.next_check = None;
        Ok(())
    }

    /// Starts a background check when one is due. Call once per frame.
    pub(crate) fn tick_panel(&mut self) {
        let due = self.panel.next_check.is_some_and(|at| Instant::now() >= at);
        if !due || self.panel.busy || self.panel.session.refused.is_some() {
            return;
        }
        let Some(key) = self.license_key().filter(|_| self.license.is_some()) else {
            self.panel.next_check = None;
            return;
        };
        let device = self.device();
        let token = self.panel.token.clone();
        let expires = self.panel.session.token_expires_at;
        let etag = self
            .panel
            .session
            .brand_etag
            .clone()
            .filter(|_| self.panel.brand_kept);
        self.panel.busy = true;
        self.panel.next_check = None;
        self.spawn(move || {
            let token = token.as_deref().zip(expires);
            Event::PanelChecked(check_in(
                &HttpPanel::new(),
                &key,
                &device,
                token,
                etag.as_deref(),
                Utc::now(),
            ))
        });
    }

    pub(crate) fn on_panel_checked(&mut self, result: Result<CheckIn, PanelError>) {
        self.panel.busy = false;
        self.apply_check(result, Utc::now());
    }

    fn apply_check(&mut self, result: Result<CheckIn, PanelError>, now: DateTime<Utc>) {
        match result {
            Ok(check) => {
                if let Some(activation) = &check.renewed {
                    self.keep_token(activation);
                }
                if let Some(download) = check.brand {
                    let mut brand = PanelBrand::from_json(&download.json);
                    for (file, bytes) in download.images {
                        brand.images.insert(file.to_owned(), bytes);
                    }
                    match panel_brand::save_panel_brand(&self.paths, &brand) {
                        Ok(()) => {
                            self.panel.session.brand_etag = download.etag;
                            self.panel.brand_kept = true;
                        }
                        Err(err) => tracing::warn!(%err, "could not keep the panel brand"),
                    }
                }
                self.panel.session.last_ok_at = Some(now);
                self.panel.session.refused = None;
                self.panel.next_check = Some(Instant::now() + CHECK_EVERY);
            }
            Err(PanelError::Refused(refusal)) => {
                tracing::warn!(?refusal, "the panel refused this computer");
                self.drop_token();
                self.panel.session.refused = Some(refusal);
                self.panel.next_check = None;
            }
            Err(err) => {
                tracing::info!(%err, "panel check failed; trying again later");
                self.panel.next_check = Some(Instant::now() + RETRY_AFTER_FAILURE);
            }
        }
        self.save_session();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A scripted panel that records the calls it gets.
    #[derive(Default)]
    struct FakePanel {
        current: Option<Result<(), PanelError>>,
        activate: Option<Result<Activation, PanelError>>,
        calls: RefCell<Vec<String>>,
    }

    impl PanelApi for FakePanel {
        fn activate(&self, _: &str, _: &str, renew: bool) -> Result<Activation, PanelError> {
            self.calls
                .borrow_mut()
                .push(format!("activate renew={renew}"));
            self.activate
                .clone()
                .unwrap_or(Err(PanelError::Unreachable("none".into())))
        }
        fn current(&self, token: &str) -> Result<(), PanelError> {
            self.calls.borrow_mut().push(format!("current {token}"));
            self.current.clone().unwrap_or(Ok(()))
        }
        fn brand(
            &self,
            _: &str,
            etag: Option<&str>,
        ) -> Result<Option<(serde_json::Value, Option<String>)>, PanelError> {
            self.calls.borrow_mut().push("brand".into());
            Ok((etag != Some("v1")).then(|| {
                (
                    serde_json::json!({
                        "app_name": "Escola",
                        "colors": {"accent_light": "#112233"},
                        "images": {"mark": "https://cdn.example/mark.svg", "icon": null},
                    }),
                    Some("v1".into()),
                )
            }))
        }
        fn sign_out(&self, _: &str) -> Result<(), PanelError> {
            Ok(())
        }
        fn download(&self, url: &str) -> Result<Vec<u8>, PanelError> {
            self.calls.borrow_mut().push(format!("download {url}"));
            Ok(b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec())
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-09T12:00:00Z")
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_default()
    }

    fn fresh_token(days: i64) -> Activation {
        Activation {
            token: "new".into(),
            token_expires_at: now() + Duration::days(days),
        }
    }

    #[test]
    fn a_valid_token_is_only_confirmed_and_the_brand_fetched_when_it_changed() {
        let panel = FakePanel::default();
        let check = check_in(
            &panel,
            "key",
            "dev",
            Some(("old", now() + Duration::days(5))),
            Some("v1"),
            now(),
        )
        .unwrap();
        assert_eq!(check.renewed, None);
        assert_eq!(check.brand, None);
        assert_eq!(*panel.calls.borrow(), ["current old", "brand"]);
    }

    #[test]
    fn a_missing_expiring_or_rejected_token_is_renewed_silently() {
        for token in [None, Some(("old", now() + Duration::hours(3)))] {
            let panel = FakePanel {
                activate: Some(Ok(fresh_token(7))),
                ..FakePanel::default()
            };
            let check = check_in(&panel, "key", "dev", token, None, now()).unwrap();
            assert_eq!(check.renewed, Some(fresh_token(7)));
            let brand = check.brand.unwrap();
            assert_eq!(brand.images.len(), 1);
            assert_eq!(brand.images[0].0, "mark.svg");
            assert_eq!(panel.calls.borrow()[0], "activate renew=true");
            assert!(
                panel
                    .calls
                    .borrow()
                    .contains(&"download https://cdn.example/mark.svg".to_owned())
            );
        }
        let panel = FakePanel {
            current: Some(Err(PanelError::Unauthenticated)),
            activate: Some(Ok(fresh_token(7))),
            ..FakePanel::default()
        };
        let check = check_in(
            &panel,
            "key",
            "dev",
            Some(("old", now() + Duration::days(5))),
            None,
            now(),
        )
        .unwrap();
        assert_eq!(check.renewed, Some(fresh_token(7)));
        assert_eq!(
            panel.calls.borrow()[..2],
            ["current old", "activate renew=true"]
        );
    }

    #[test]
    fn refusals_and_network_failures_come_back_as_they_are() {
        let panel = FakePanel {
            current: Some(Err(PanelError::Unauthenticated)),
            activate: Some(Err(PanelError::Refused(Refusal::DeviceRevoked))),
            ..FakePanel::default()
        };
        let token = Some(("old", now() + Duration::days(5)));
        assert_eq!(
            check_in(&panel, "key", "dev", token, None, now()),
            Err(PanelError::Refused(Refusal::DeviceRevoked))
        );
        let panel = FakePanel {
            current: Some(Err(PanelError::Unreachable("offline".into()))),
            ..FakePanel::default()
        };
        assert!(matches!(
            check_in(&panel, "key", "dev", token, None, now()),
            Err(PanelError::Unreachable(_))
        ));
    }

    #[test]
    fn panel_errors_map_to_refusals() {
        assert_eq!(failure(401, ""), PanelError::Unauthenticated);
        assert_eq!(
            failure(403, r#"{"error":{"code":"device_blocked","message":"x"}}"#),
            PanelError::Refused(Refusal::DeviceBlocked)
        );
        assert_eq!(
            failure(409, r#"{"error":{"code":"no_seats_left"}}"#),
            PanelError::Refused(Refusal::NoSeatsLeft)
        );
        assert_eq!(
            failure(422, r#"{"error":{"code":"validation_failed"}}"#),
            PanelError::Unexpected {
                status: 422,
                detail: "validation_failed".into()
            }
        );
    }

    #[test]
    fn the_brand_keeps_only_safe_values() {
        let brand = PanelBrand::from_json(&serde_json::json!({
            "app_name": "Escola Aprova",
            "company_name": "x".repeat(61),
            "colors": {"accent_light": "#52613C", "accent_dark": "red", "highlight_light": "#12345"},
            "support_url": "https://escola.example/ajuda",
            "contact_url": "javascript:alert(1)",
        }));
        assert_eq!(brand.app_name.as_deref(), Some("Escola Aprova"));
        assert_eq!(brand.company_name, None);
        assert_eq!(brand.accent_light, Some([0x52, 0x61, 0x3C]));
        assert_eq!(brand.accent_dark, None);
        assert_eq!(brand.highlight_light, None);
        assert_eq!(
            brand.support_url.as_deref(),
            Some("https://escola.example/ajuda")
        );
        assert_eq!(brand.contact_url, None);
    }

    #[test]
    fn the_device_id_is_a_stable_hash_without_the_raw_machine_id() {
        let mut store = kg_study::Store::open_in_memory().unwrap();
        let first = device_id(&mut store);
        assert_eq!(first, device_id(&mut store));
        assert_eq!(first.len(), 64);
        if let Ok(raw) = machine_uid::get() {
            assert!(!first.contains(raw.trim()));
        }
    }

    fn engine(dir: &std::path::Path) -> crate::Engine {
        crate::Engine::in_memory(crate::AppPaths::new(dir), crate::AiConfig::default()).unwrap()
    }

    #[test]
    fn checks_keep_the_app_open_for_fourteen_days_offline_and_refusals_close_it() {
        if !kg_brand::has_server() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(dir.path());
        engine.panel.session.last_ok_at = Some(now());
        engine.license = Some(crate::license::tests::sample_license());
        assert_eq!(
            engine.panel_status_at(now()),
            PanelStatus::Connected { last_ok: now() }
        );
        assert_eq!(
            engine.panel_status_at(now() + Duration::days(3)),
            PanelStatus::Offline {
                last_ok: now(),
                days_left: 11
            }
        );
        assert_eq!(
            engine.panel_status_at(now() + Duration::days(14)),
            PanelStatus::OfflineTooLong { last_ok: now() }
        );

        engine.panel.token = Some("old".into());
        engine.apply_check(Err(PanelError::Unreachable("offline".into())), now());
        assert_eq!(engine.panel.session.refused, None);
        assert_eq!(engine.panel.token.as_deref(), Some("old"));

        engine.apply_check(
            Ok(CheckIn {
                renewed: Some(fresh_token(7)),
                brand: Some(BrandDownload {
                    json: serde_json::json!({"app_name": "Escola"}),
                    etag: Some("v2".into()),
                    images: vec![("mark.svg", b"<svg/>".to_vec())],
                }),
            }),
            now(),
        );
        assert_eq!(engine.panel.token.as_deref(), Some("new"));
        assert_eq!(engine.panel.session.brand_etag.as_deref(), Some("v2"));
        // Kept for the next start, not applied to this one.
        assert_eq!(engine.panel_brand(), None);
        let kept = panel_brand::load_panel_brand(&engine.paths).unwrap();
        assert_eq!(kept.app_name.as_deref(), Some("Escola"));
        assert!(kept.image("mark.svg").is_some());
        let ai = engine.panel_ai(&crate::AiConfig::default());
        assert_eq!(ai.api_key, "new");
        assert_eq!(ai.models.vision, kg_brand::SERVER_VISION_MODEL);
        assert!(ai.base_url.ends_with("/api/v1/ai"));

        engine.apply_check(Err(PanelError::Refused(Refusal::DeviceBlocked)), now());
        assert_eq!(
            engine.panel_status_at(now()),
            PanelStatus::Refused(Refusal::DeviceBlocked)
        );
        assert_eq!(engine.panel.token, None);
        assert!(!engine.panel_allows_use());
    }
}
