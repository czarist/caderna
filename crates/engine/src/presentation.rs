//! Developer tools for showing the app (Phase W9): the student or teacher
//! view whatever the licence says, and presentation mode, which shows the
//! app with the brand of a client registered in the panel, so a school can
//! see its own look before it signs. Both need a valid developer token and
//! last only for this run; nothing is saved.

use crate::developer::{self, DeveloperFile};
use crate::panel_brand::PanelBrand;
use crate::server::{HttpPanel, PanelError};
use crate::{Engine, Event, Notice, UseRole};

/// A client with a brand in the panel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PresentationClient {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Default)]
pub(crate) struct PresentationState {
    clients: Option<Vec<PresentationClient>>,
    busy: bool,
    /// The client shown and its brand.
    showing: Option<(i64, PanelBrand)>,
    /// The view forced from the developer area.
    view: Option<UseRole>,
}

#[derive(Debug)]
pub(crate) enum PresentationEvent {
    Clients(Result<Vec<PresentationClient>, PanelError>),
    Brand(i64, Result<PanelBrand, PanelError>),
}

impl Engine {
    /// The saved developer token, when it is valid on this computer.
    fn developer_token(&self) -> Option<String> {
        if kg_brand::DEVELOPER_PUBLIC_KEY.is_empty() || !kg_brand::has_server() {
            return None;
        }
        let file = DeveloperFile::load(&crate::AppPaths::standard().data_dir);
        developer::verify(
            &file.token,
            kg_brand::DEVELOPER_PUBLIC_KEY,
            kg_brand::SLUG,
            &crate::server::machine_hash(),
            chrono::Utc::now(),
        )
        .ok()
        .map(|_| file.token)
    }

    fn developer_body(token: String) -> serde_json::Value {
        let device = crate::server::machine_hash();
        serde_json::json!({
            "token": token,
            "device": (!device.is_empty()).then_some(device),
            "app_version": env!("CARGO_PKG_VERSION"),
        })
    }

    /// The clients with a brand, once listed.
    pub fn presentation_clients(&self) -> Option<&[PresentationClient]> {
        self.presentation.clients.as_deref()
    }

    /// The panel is being asked for clients or a brand.
    pub fn presentation_busy(&self) -> bool {
        self.presentation.busy
    }

    /// The client whose brand is shown, in presentation mode.
    pub fn presenting(&self) -> Option<i64> {
        self.presentation.showing.as_ref().map(|(id, _)| *id)
    }

    /// The view forced from the developer area, if any.
    pub fn developer_view(&self) -> Option<UseRole> {
        self.presentation.view
    }

    /// The brand shown while presenting, over the licence's own.
    pub(crate) fn presentation_brand(&self) -> Option<&PanelBrand> {
        self.presentation.showing.as_ref().map(|(_, brand)| brand)
    }

    pub(crate) fn developer_list_clients(&mut self) {
        if self.presentation.busy {
            return;
        }
        let Some(token) = self.developer_token() else {
            return;
        };
        self.presentation.busy = true;
        self.spawn(move || {
            let result = HttpPanel::for_developer()
                .call(
                    "POST",
                    "developer/clients",
                    "",
                    Some(&Self::developer_body(token)),
                )
                .and_then(|(status, text)| crate::classes::data(status, &text));
            Event::Presentation(PresentationEvent::Clients(result))
        });
    }

    /// Shows `client`'s brand, or with `None` goes back to the licence's.
    pub(crate) fn developer_present(&mut self, client: Option<i64>) {
        let Some(client) = client else {
            if self.presentation.showing.take().is_some() {
                self.restore_panel_brand();
            }
            return;
        };
        if self.presentation.busy {
            return;
        }
        let Some(token) = self.developer_token() else {
            return;
        };
        self.presentation.busy = true;
        self.spawn(move || {
            let panel = HttpPanel::for_developer();
            let result = panel
                .call(
                    "POST",
                    &format!("developer/clients/{client}/brand"),
                    "",
                    Some(&Self::developer_body(token)),
                )
                .and_then(|(status, text)| crate::classes::data::<serde_json::Value>(status, &text))
                .and_then(|json| {
                    let mut brand = PanelBrand::from_json(&json);
                    for (file, bytes) in crate::server::download_images(&panel, &json)? {
                        brand.images.insert(file.to_owned(), bytes);
                    }
                    Ok(brand)
                });
            Event::Presentation(PresentationEvent::Brand(client, result))
        });
    }

    /// Forces the student or teacher view (`None`: the licence's again).
    pub(crate) fn developer_set_view(&mut self, view: Option<UseRole>) {
        if view.is_some() && self.developer_token().is_none() {
            return;
        }
        self.presentation.view = view;
    }

    pub(crate) fn on_presentation_event(&mut self, event: PresentationEvent) {
        self.presentation.busy = false;
        match event {
            PresentationEvent::Clients(Ok(clients)) => self.presentation.clients = Some(clients),
            PresentationEvent::Brand(client, Ok(brand)) => {
                crate::server::set_handout_brand(&brand);
                self.presentation.showing = Some((client, brand));
                self.mark_brand_changed();
            }
            PresentationEvent::Clients(Err(err)) | PresentationEvent::Brand(_, Err(err)) => {
                tracing::warn!(%err, "presentation: the panel did not answer");
                self.notices
                    .push(Notice::Error(crate::EngineError::Panel(err)));
            }
        }
    }
}
