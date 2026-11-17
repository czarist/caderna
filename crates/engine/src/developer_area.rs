//! The engine side of the developer area (Phase W9): exchanging a code for a
//! token, choosing the environment for the next start, resetting an
//! environment's data, and the hourly revocation check.

use std::time::{Duration, Instant};

use crate::developer::{self, DeveloperError, DeveloperFile, Environment};
use crate::server::HttpPanel;
use crate::{AppPaths, Engine, EngineError, Event, Notice};

/// How often a token in use is checked with the panel.
const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

#[derive(Debug)]
pub(crate) enum DeveloperEvent {
    Exchanged(Result<String, DeveloperError>),
    /// The panel's answer about the token in use: still valid or not.
    Checked(Option<bool>),
}

/// What the developer area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperStatus {
    /// The environment of this run.
    pub environment: Environment,
    pub port: Option<u16>,
    /// The saved token's claims, when it is valid.
    pub claims: Option<developer::DeveloperClaims>,
    /// The environment saved for the next start.
    pub next: Environment,
    /// Environments with data of their own on this computer.
    pub with_data: Vec<Environment>,
    /// Why the last run went back to production, if it did.
    pub ended: Option<DeveloperError>,
    pub exchanging: bool,
}

#[derive(Debug, Default)]
pub(crate) struct DeveloperState {
    exchanging: bool,
    checking: bool,
    next_check: Option<Instant>,
}

impl Engine {
    fn standard_dir() -> std::path::PathBuf {
        AppPaths::standard().data_dir
    }

    /// The developer area's state; `None` when the build has no developer
    /// key (the area does not exist there).
    pub fn developer_status(&self) -> Option<DeveloperStatus> {
        if kg_brand::DEVELOPER_PUBLIC_KEY.is_empty() || !kg_brand::has_server() {
            return None;
        }
        let active = developer::active();
        let file = DeveloperFile::load(&Self::standard_dir());
        let claims = developer::verify(
            &file.token,
            kg_brand::DEVELOPER_PUBLIC_KEY,
            kg_brand::SLUG,
            &crate::server::machine_hash(),
            chrono::Utc::now(),
        )
        .ok();
        let with_data = [Environment::Homologation, Environment::Development]
            .into_iter()
            .filter(|e| {
                AppPaths::new(e.data_dir(&Self::standard_dir()))
                    .database()
                    .exists()
            })
            .collect();
        Some(DeveloperStatus {
            environment: active.environment,
            port: active.port,
            claims,
            next: file.environment,
            with_data,
            ended: active.ended,
            exchanging: self.developer.exchanging,
        })
    }

    pub(crate) fn developer_code(&mut self, code: String) {
        if self.developer.exchanging || !kg_brand::has_server() {
            return;
        }
        let device = crate::server::machine_hash();
        self.developer.exchanging = true;
        self.spawn(move || {
            let body = serde_json::json!({
                "code": code.trim(),
                "device": (!device.is_empty()).then_some(device),
                "app_version": env!("CARGO_PKG_VERSION"),
            });
            let result = HttpPanel::for_developer()
                .call("POST", "developer/exchange", "", Some(&body))
                .map_err(|_| DeveloperError::Refused)
                .and_then(|(status, text)| {
                    #[derive(serde::Deserialize)]
                    struct Data {
                        token: String,
                    }
                    if status != 200 {
                        return Err(DeveloperError::Refused);
                    }
                    crate::classes::data::<Data>(status, &text)
                        .map(|d| d.token)
                        .map_err(|_| DeveloperError::Invalid)
                });
            Event::Developer(DeveloperEvent::Exchanged(result))
        });
    }

    pub(crate) fn developer_switch(
        &mut self,
        environment: Environment,
        port: Option<u16>,
        copy: bool,
        fresh: bool,
    ) -> Result<(), EngineError> {
        let standard = Self::standard_dir();
        let mut file = DeveloperFile::load(&standard);
        if environment != Environment::Production {
            let claims = developer::verify(
                &file.token,
                kg_brand::DEVELOPER_PUBLIC_KEY,
                kg_brand::SLUG,
                &crate::server::machine_hash(),
                chrono::Utc::now(),
            )?;
            if !claims.opens(environment) || environment.compiled_url().is_empty() {
                return Err(DeveloperError::Environment.into());
            }
            let current = developer::active().environment;
            if environment != current {
                if copy && current == Environment::Production {
                    developer::copy_production(&self.store, environment)?;
                } else if fresh {
                    developer::remove_environment(environment)?;
                }
            }
        }
        file.environment = environment;
        file.port = port.filter(|_| environment == Environment::Development);
        file.ended = None;
        file.save(&standard).map_err(|source| EngineError::Export {
            path: standard.clone(),
            source,
        })?;
        self.notices.push(Notice::DeveloperSwitched);
        Ok(())
    }

    pub(crate) fn developer_reset(&mut self, environment: Environment) -> Result<(), EngineError> {
        if environment == developer::active().environment {
            return Ok(());
        }
        developer::remove_environment(environment)
    }

    pub(crate) fn developer_leave(&mut self) -> Result<(), EngineError> {
        let standard = Self::standard_dir();
        DeveloperFile::default()
            .save(&standard)
            .map_err(|source| EngineError::Export {
                path: standard,
                source,
            })?;
        self.notices.push(Notice::DeveloperSwitched);
        Ok(())
    }

    /// Checks the token in use with the panel at start and every hour.
    pub(crate) fn tick_developer(&mut self) {
        let active = developer::active();
        if active.environment == Environment::Production || self.developer.checking {
            return;
        }
        let due = self
            .developer
            .next_check
            .is_none_or(|at| Instant::now() >= at);
        if !due {
            return;
        }
        self.developer.next_check = Some(Instant::now() + CHECK_EVERY);
        self.developer.checking = true;
        let token = active.token.clone();
        let environment = active.environment.id();
        let device = crate::server::machine_hash();
        self.spawn(move || {
            let body = serde_json::json!({
                "token": token,
                "environment": environment,
                "device": (!device.is_empty()).then_some(device),
                "app_version": env!("CARGO_PKG_VERSION"),
            });
            #[derive(serde::Deserialize)]
            struct Data {
                active: bool,
            }
            let answer = HttpPanel::for_developer()
                .call("POST", "developer/check", "", Some(&body))
                .ok()
                .and_then(|(status, text)| crate::classes::data::<Data>(status, &text).ok())
                .map(|d| d.active);
            Event::Developer(DeveloperEvent::Checked(answer))
        });
    }

    pub(crate) fn on_developer_event(&mut self, event: DeveloperEvent) {
        match event {
            DeveloperEvent::Exchanged(result) => {
                self.developer.exchanging = false;
                match result.and_then(|token| {
                    developer::verify(
                        &token,
                        kg_brand::DEVELOPER_PUBLIC_KEY,
                        kg_brand::SLUG,
                        &crate::server::machine_hash(),
                        chrono::Utc::now(),
                    )
                    .map(|_| token)
                }) {
                    Ok(token) => {
                        let standard = Self::standard_dir();
                        let mut file = DeveloperFile::load(&standard);
                        file.token = token;
                        match file.save(&standard) {
                            Ok(()) => self.notices.push(Notice::DeveloperTokenSaved),
                            Err(source) => self.notices.push(Notice::Error(EngineError::Export {
                                path: standard,
                                source,
                            })),
                        }
                    }
                    Err(err) => self.notices.push(Notice::Error(err.into())),
                }
            }
            DeveloperEvent::Checked(answer) => {
                self.developer.checking = false;
                // No answer (offline): keep going; the offline check holds.
                if answer == Some(false) {
                    let standard = Self::standard_dir();
                    let file = DeveloperFile {
                        ended: Some("revoked".into()),
                        ..DeveloperFile::default()
                    };
                    if let Err(err) = file.save(&standard) {
                        tracing::warn!(%err, "could not leave developer mode");
                    }
                    tracing::info!("developer token no longer valid; production at next start");
                    self.notices.push(Notice::DeveloperEnded);
                }
            }
        }
    }
}
