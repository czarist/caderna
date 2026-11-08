//! Offline Ed25519 licenses. The signature authenticates the exact JSON bytes.
//! Seats are a contractual limit; an offline app cannot count other computers.

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use chrono::NaiveDate;
use ring::signature::{ED25519, UnparsedPublicKey};
use serde::{Deserialize, Serialize};

const SETTING: &str = "signed_license";
const DOMAIN: &[u8] = b"kg-license-v1\0";

/// Signed, versioned claims issued by the software licensor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub version: u32,
    pub id: String,
    pub brand: String,
    pub customer: String,
    pub seats: u32,
    pub not_before: NaiveDate,
    pub expires: NaiveDate,
}

/// A verified license; construction always checks its signature and claims.
#[derive(Debug, Clone)]
pub struct License(Claims);

impl License {
    pub fn claims(&self) -> &Claims {
        &self.0
    }
    pub fn valid_on(&self, today: NaiveDate) -> bool {
        self.0.not_before <= today && today <= self.0.expires
    }
}

/// Errors are diagnostic only; the UI uses locale keys.
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("invalid license encoding or claims")]
    Invalid,
    #[error("license signature verification failed")]
    Signature,
    #[error("license belongs to a different brand")]
    Brand,
    #[error("license is outside its validity period")]
    Date,
    #[error("could not store license: {0}")]
    Store(#[from] kg_study::StoreError),
}

/// Bytes to sign with the issuer's private key. Includes protocol domain separation.
pub fn signing_message(payload: &[u8]) -> Vec<u8> {
    [DOMAIN, payload].concat()
}

/// Verifies a compact `base64url(payload).base64url(signature)` license.
pub fn verify(token: &str, public_key: &str, brand: &str) -> Result<License, LicenseError> {
    let token = token.trim();
    if token.len() > 8192 {
        return Err(LicenseError::Invalid);
    }
    let (payload, signature) = token.split_once('.').ok_or(LicenseError::Invalid)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| LicenseError::Invalid)?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| LicenseError::Invalid)?;
    let key = STANDARD
        .decode(public_key)
        .map_err(|_| LicenseError::Invalid)?;
    if key.len() != 32 || signature.len() != 64 {
        return Err(LicenseError::Invalid);
    }
    UnparsedPublicKey::new(&ED25519, key)
        .verify(&signing_message(&payload), &signature)
        .map_err(|_| LicenseError::Signature)?;
    let claims: Claims = serde_json::from_slice(&payload).map_err(|_| LicenseError::Invalid)?;
    if claims.version != 1
        || claims.id.trim().is_empty()
        || claims.customer.trim().is_empty()
        || claims.customer.len() > 256
        || claims.id.len() > 128
        || claims.seats == 0
        || claims.expires < claims.not_before
    {
        return Err(LicenseError::Invalid);
    }
    if claims.brand != brand {
        return Err(LicenseError::Brand);
    }
    Ok(License(claims))
}

impl crate::Engine {
    pub(crate) fn load_license(&mut self) {
        self.license = self
            .store
            .setting(SETTING)
            .ok()
            .flatten()
            .and_then(|token| verify(&token, kg_brand::LICENSE_PUBLIC_KEY, kg_brand::SLUG).ok());
    }

    /// The authenticated claims, including an expired license for renewal display.
    pub fn license(&self) -> Option<&License> {
        self.license.as_ref()
    }

    /// Activates only a valid license. Study records and settings remain untouched.
    pub fn activate_license(&mut self, token: &str) -> Result<(), LicenseError> {
        self.activate_checked(
            token,
            kg_brand::LICENSE_PUBLIC_KEY,
            kg_brand::SLUG,
            self.license_day(),
        )
    }

    fn activate_checked(
        &mut self,
        token: &str,
        key: &str,
        brand: &str,
        today: NaiveDate,
    ) -> Result<(), LicenseError> {
        let license = verify(token, key, brand)?;
        if !license.valid_on(today) {
            return Err(LicenseError::Date);
        }
        self.store.set_setting(SETTING, token.trim())?;
        self.license = Some(license);
        Ok(())
    }

    pub(crate) fn license_day(&self) -> NaiveDate {
        self.demo
            .map_or_else(Self::today, |(_, clock)| clock.effective_day(Self::today()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn signed() -> (String, String, Claims) {
        signed_between(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        )
    }

    fn signed_between(not_before: NaiveDate, expires: NaiveDate) -> (String, String, Claims) {
        let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
        let claims = Claims {
            version: 1,
            id: "test-1".into(),
            brand: "client".into(),
            customer: "School".into(),
            seats: 5,
            not_before,
            expires,
        };
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&payload),
            URL_SAFE_NO_PAD.encode(key.sign(&signing_message(&payload)).as_ref())
        );
        (token, STANDARD.encode(key.public_key().as_ref()), claims)
    }

    #[test]
    fn authentic_license_has_inclusive_dates_and_brand_binding() {
        let (token, key, claims) = signed();
        let license = verify(&token, &key, "client").unwrap();
        assert!(license.valid_on(claims.not_before));
        assert!(license.valid_on(claims.expires));
        assert!(!license.valid_on(claims.expires.succ_opt().unwrap()));
        assert!(!license.valid_on(claims.not_before.pred_opt().unwrap()));
        assert!(matches!(
            verify(&token, &key, "other"),
            Err(LicenseError::Brand)
        ));
    }

    #[test]
    fn changed_claims_wrong_key_and_oversized_input_are_rejected() {
        let (token, key, mut claims) = signed();
        claims.seats = 500;
        let (_, signature) = token.split_once('.').unwrap();
        let changed = format!(
            "{}.{signature}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        );
        assert!(matches!(
            verify(&changed, &key, "client"),
            Err(LicenseError::Signature)
        ));
        assert!(verify(&token, &STANDARD.encode([0; 32]), "client").is_err());
        assert!(verify(&"a".repeat(8193), &key, "client").is_err());
        assert!(verify("not a license", &key, "client").is_err());
    }

    #[test]
    fn a_license_turns_an_ended_demo_into_the_full_app_keeping_study_data() {
        use crate::DemoStatus;
        let dir = tempfile::tempdir().unwrap();
        let mut engine =
            crate::Engine::in_memory(crate::AppPaths::new(dir.path()), crate::AiConfig::default())
                .unwrap();
        engine
            .store
            .set_setting("student_note", "keep my progress")
            .unwrap();
        engine.demo = Some(crate::demo::ended());
        assert_eq!(engine.demo(), DemoStatus::Expired);

        let today = engine.license_day();
        let (token, key, _) = signed_between(
            today.pred_opt().unwrap(),
            today.checked_add_days(chrono::Days::new(30)).unwrap(),
        );
        engine
            .activate_checked(&token, &key, "client", today)
            .unwrap();
        assert_eq!(engine.demo(), DemoStatus::Full);
        assert_eq!(
            engine.store.setting("student_note").unwrap().as_deref(),
            Some("keep my progress")
        );

        // When the accepted license lapses the app stops again; data stays.
        let (old, _, _) = signed_between(
            today.checked_sub_days(chrono::Days::new(60)).unwrap(),
            today.checked_sub_days(chrono::Days::new(30)).unwrap(),
        );
        engine.license = Some(verify(&old, &key, "client").unwrap());
        assert_eq!(engine.demo(), DemoStatus::Expired);
        assert_eq!(
            engine.store.setting("student_note").unwrap().as_deref(),
            Some("keep my progress")
        );
    }

    #[test]
    fn activation_and_rejected_replacement_preserve_study_and_settings() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine =
            crate::Engine::in_memory(crate::AppPaths::new(dir.path()), crate::AiConfig::default())
                .unwrap();
        engine
            .store
            .set_setting("student_note", "keep my progress")
            .unwrap();
        let (token, key, claims) = signed();
        engine
            .activate_checked(&token, &key, "client", claims.not_before)
            .unwrap();
        assert_eq!(
            engine.store.setting("student_note").unwrap().as_deref(),
            Some("keep my progress")
        );
        assert_eq!(engine.license().unwrap().claims().id, "test-1");
        assert!(
            engine
                .activate_checked("bad", &key, "client", claims.not_before)
                .is_err()
        );
        assert!(
            engine
                .activate_checked(&token, &key, "other", claims.not_before)
                .is_err()
        );
        assert!(
            engine
                .activate_checked(&token, &key, "client", claims.expires.succ_opt().unwrap())
                .is_err()
        );
        assert_eq!(
            engine.store.setting(SETTING).unwrap().as_deref(),
            Some(token.as_str())
        );
        assert_eq!(engine.license().unwrap().claims().id, "test-1");
    }
}
