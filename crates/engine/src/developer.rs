//! Developer mode (Phase W9): one production executable that the team points
//! at homologation or development with a developer token made in the panel.
//!
//! - The panel addresses stay compiled into the brand (`SERVER_URL_PROD`,
//!   `_HOM`, `_DEV`); only the choice between them moves to runtime, and only
//!   behind a token signed with the brand's developer key
//!   (`[developer] public_key`). No free address is ever accepted; a custom
//!   `localhost` port is offered for development only.
//! - Each environment has its own data folder inside the brand's
//!   (`<data_dir>/env/hom`, `<data_dir>/env/dev`), so production data is never
//!   opened, migrated or synced while another environment is on. The
//!   credential entry follows the data folder (engine `server::Vault`).
//! - The token is checked offline at start (signature, brand, expiry, device,
//!   environment) and online every hour (revocation); when it stops being
//!   valid the app goes back to production at the next start and says so.
//!
//! The choice is kept in `developer.json` in the brand's standard data
//! folder. A build with no developer key, or a student with no token,
//! behaves exactly as before.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use ring::signature::{ED25519, UnparsedPublicKey};
use serde::{Deserialize, Serialize};

use crate::AppPaths;

const DOMAIN: &[u8] = b"kg-dev-v1\0";
const FILE: &str = "developer.json";

/// Which panel and data folder the app uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Production,
    Homologation,
    Development,
}

impl Environment {
    pub const ALL: [Self; 3] = [Self::Production, Self::Homologation, Self::Development];

    /// The id the panel's tokens use (`hom`, `dev`); `prod` for production.
    pub fn id(self) -> &'static str {
        match self {
            Self::Production => "prod",
            Self::Homologation => "hom",
            Self::Development => "dev",
        }
    }

    /// The compiled panel address of this environment; empty when the brand
    /// has none.
    pub fn compiled_url(self) -> &'static str {
        match self {
            Self::Production => kg_brand::SERVER_URL,
            Self::Homologation => kg_brand::SERVER_URL_HOM,
            Self::Development => kg_brand::SERVER_URL_DEV,
        }
    }

    /// This environment's data folder inside the brand's standard one.
    pub fn data_dir(self, standard: &Path) -> PathBuf {
        match self {
            Self::Production => standard.to_path_buf(),
            other => standard.join("env").join(other.id()),
        }
    }
}

/// What `developer.json` keeps.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeveloperFile {
    /// The signed token from the panel; empty when there is none.
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub environment: Environment,
    /// A `localhost` port for development, instead of the compiled `dev_url`.
    #[serde(default)]
    pub port: Option<u16>,
    /// Set when the app went back to production on its own, until shown.
    #[serde(default)]
    pub ended: Option<String>,
}

impl DeveloperFile {
    fn path(standard: &Path) -> PathBuf {
        standard.join(FILE)
    }

    pub fn load(standard: &Path) -> Self {
        std::fs::read_to_string(Self::path(standard))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, standard: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(standard)?;
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(Self::path(standard), text)
    }
}

/// What a developer token says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeveloperClaims {
    pub v: u32,
    pub id: i64,
    pub brand: String,
    pub person: String,
    pub environments: Vec<String>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub device: Option<String>,
}

impl DeveloperClaims {
    pub fn opens(&self, environment: Environment) -> bool {
        environment == Environment::Production
            || self.environments.iter().any(|e| e == environment.id())
    }
}

/// Why a developer token is not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeveloperError {
    #[error("this build has no developer key")]
    NoKey,
    #[error("the developer token is not valid")]
    Invalid,
    #[error("the developer token's signature does not match")]
    Signature,
    #[error("the developer token is for another brand")]
    Brand,
    #[error("the developer token has expired")]
    Expired,
    #[error("the developer token is for another computer")]
    OtherDevice,
    #[error("the developer token does not open this environment")]
    Environment,
    #[error("the panel refused the developer code")]
    Refused,
}

/// Checks a token offline against `public_key` (base64 Ed25519), the
/// brand's slug, `now` and this computer's hashed id.
pub fn verify(
    token: &str,
    public_key: &str,
    brand: &str,
    device: &str,
    now: DateTime<Utc>,
) -> Result<DeveloperClaims, DeveloperError> {
    if public_key.trim().is_empty() {
        return Err(DeveloperError::NoKey);
    }
    let token = token.trim();
    if token.is_empty() || token.len() > 4096 {
        return Err(DeveloperError::Invalid);
    }
    let (payload, signature) = token.split_once('.').ok_or(DeveloperError::Invalid)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| DeveloperError::Invalid)?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| DeveloperError::Invalid)?;
    let key = STANDARD
        .decode(public_key.trim())
        .map_err(|_| DeveloperError::Invalid)?;
    if key.len() != 32 || signature.len() != 64 {
        return Err(DeveloperError::Invalid);
    }
    UnparsedPublicKey::new(&ED25519, key)
        .verify(&[DOMAIN, &payload].concat(), &signature)
        .map_err(|_| DeveloperError::Signature)?;
    let claims: DeveloperClaims =
        serde_json::from_slice(&payload).map_err(|_| DeveloperError::Invalid)?;
    if claims.v != 1 {
        return Err(DeveloperError::Invalid);
    }
    if claims.brand != brand {
        return Err(DeveloperError::Brand);
    }
    if claims.expires_at <= now {
        return Err(DeveloperError::Expired);
    }
    if claims.device.as_deref().is_some_and(|d| d != device) {
        return Err(DeveloperError::OtherDevice);
    }
    Ok(claims)
}

/// The environment this process runs in, decided once at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    pub environment: Environment,
    pub port: Option<u16>,
    pub claims: Option<DeveloperClaims>,
    /// The token in use, for the online check.
    pub token: String,
    /// Why the app went back to production on its own, if it did.
    pub ended: Option<DeveloperError>,
}

static ACTIVE: OnceLock<Active> = OnceLock::new();

/// The environment of this run (production unless a valid token chose
/// another one). `KG_DATA_DIR` runs keep production: tests and the second
/// copies of the live examples never switch.
pub fn active() -> &'static Active {
    ACTIVE.get_or_init(|| {
        if std::env::var_os("KG_DATA_DIR").is_some_and(|d| !d.is_empty()) {
            return production(None);
        }
        resolve(
            &DeveloperFile::load(&AppPaths::standard().data_dir),
            &crate::server::machine_hash(),
            Utc::now(),
        )
    })
}

fn production(ended: Option<DeveloperError>) -> Active {
    Active {
        environment: Environment::Production,
        port: None,
        claims: None,
        token: String::new(),
        ended,
    }
}

/// Decides the environment from the saved choice and its token.
pub(crate) fn resolve(file: &DeveloperFile, device: &str, now: DateTime<Utc>) -> Active {
    let claims = verify(
        &file.token,
        kg_brand::DEVELOPER_PUBLIC_KEY,
        kg_brand::SLUG,
        device,
        now,
    );
    if file.environment == Environment::Production {
        return Active {
            claims: claims.ok(),
            token: file.token.clone(),
            ..production(None)
        };
    }
    match claims {
        Ok(claims) if !file.environment.compiled_url().is_empty() => {
            if !claims.opens(file.environment) {
                return production(Some(DeveloperError::Environment));
            }
            Active {
                environment: file.environment,
                port: file
                    .port
                    .filter(|_| file.environment == Environment::Development),
                claims: Some(claims),
                token: file.token.clone(),
                ended: None,
            }
        }
        Ok(_) => production(Some(DeveloperError::Environment)),
        Err(err) => production(Some(err)),
    }
}

/// The panel this run talks to: the compiled address of the environment,
/// or `http://127.0.0.1:<port>` for development with a custom port.
pub fn panel_url() -> String {
    let active = active();
    match (active.environment, active.port) {
        (Environment::Development, Some(port)) => format!("http://127.0.0.1:{port}"),
        (environment, _) => environment.compiled_url().to_owned(),
    }
}

/// Copies the production database into `target`'s data folder (one way),
/// with `VACUUM INTO`, so it works while production is open.
pub(crate) fn copy_production(
    store: &kg_study::Store,
    target: Environment,
) -> Result<(), crate::EngineError> {
    let dir = target.data_dir(&AppPaths::standard().data_dir);
    if target == Environment::Production {
        return Ok(());
    }
    remove_environment(target)?;
    std::fs::create_dir_all(&dir).map_err(|source| crate::EngineError::Export {
        path: dir.clone(),
        source,
    })?;
    store.copy_into(&AppPaths::new(&dir).database())?;
    Ok(())
}

/// Removes an environment's data folder; never production's.
pub(crate) fn remove_environment(environment: Environment) -> Result<(), crate::EngineError> {
    if environment == Environment::Production {
        return Ok(());
    }
    let dir = environment.data_dir(&AppPaths::standard().data_dir);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(crate::EngineError::Export { path: dir, source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn keys() -> (Ed25519KeyPair, String) {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public = STANDARD.encode(pair.public_key().as_ref());
        (pair, public)
    }

    fn token(pair: &Ed25519KeyPair, claims: serde_json::Value) -> String {
        let payload = serde_json::to_vec(&claims).unwrap();
        let signature = pair.sign(&[DOMAIN, &payload].concat());
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&payload),
            URL_SAFE_NO_PAD.encode(signature.as_ref())
        )
    }

    fn claims(brand: &str, expires: &str, device: Option<&str>) -> serde_json::Value {
        serde_json::json!({
            "v": 1, "id": 7, "brand": brand, "person": "Ana",
            "environments": ["hom"], "expires_at": expires, "device": device
        })
    }

    #[test]
    fn tokens_are_checked_offline() {
        let (pair, public) = keys();
        let now: DateTime<Utc> = "2026-11-17T12:00:00Z".parse().unwrap();
        let good = token(&pair, claims("caderna", "2026-12-01T00:00:00Z", None));
        let ok = verify(&good, &public, "caderna", "abc", now).unwrap();
        assert!(ok.opens(Environment::Homologation));
        assert!(!ok.opens(Environment::Development));

        let (other, _) = keys();
        let forged = token(&other, claims("caderna", "2026-12-01T00:00:00Z", None));
        assert_eq!(
            verify(&forged, &public, "caderna", "abc", now),
            Err(DeveloperError::Signature)
        );
        assert_eq!(
            verify(&good, &public, "outra", "abc", now),
            Err(DeveloperError::Brand)
        );
        let old = token(&pair, claims("caderna", "2026-11-01T00:00:00Z", None));
        assert_eq!(
            verify(&old, &public, "caderna", "abc", now),
            Err(DeveloperError::Expired)
        );
        let bound = token(
            &pair,
            claims("caderna", "2026-12-01T00:00:00Z", Some("xyz")),
        );
        assert_eq!(
            verify(&bound, &public, "caderna", "abc", now),
            Err(DeveloperError::OtherDevice)
        );
        assert!(verify(&bound, &public, "caderna", "xyz", now).is_ok());
        assert_eq!(
            verify("nonsense", &public, "caderna", "abc", now),
            Err(DeveloperError::Invalid)
        );
        assert_eq!(
            verify(&good, "", "caderna", "abc", now),
            Err(DeveloperError::NoKey)
        );
    }

    #[test]
    fn environments_have_their_own_folders() {
        let standard = Path::new("/data/Caderna");
        assert_eq!(Environment::Production.data_dir(standard), standard);
        assert_eq!(
            Environment::Homologation.data_dir(standard),
            standard.join("env").join("hom")
        );
        assert_eq!(
            Environment::Development.data_dir(standard),
            standard.join("env").join("dev")
        );
    }

    #[test]
    fn without_a_valid_token_the_app_stays_in_production() {
        let file = DeveloperFile {
            token: "not a token".into(),
            environment: Environment::Homologation,
            port: Some(9000),
            ended: None,
        };
        let active = resolve(&file, "abc", Utc::now());
        assert_eq!(active.environment, Environment::Production);
        assert!(active.ended.is_some(), "and says why");
        assert_eq!(active.port, None);
        let none = resolve(&DeveloperFile::default(), "abc", Utc::now());
        assert_eq!(none.environment, Environment::Production);
        assert_eq!(none.ended, None);
    }
}
