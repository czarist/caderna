//! Demonstration builds: the days left in the sidebar and the screen shown
//! once the demonstration is over.

use egui::RichText;
use kg_engine::{Command, PanelStatus, Refusal};

use crate::{theme, widgets};

/// Days left, in the sidebar; the last day of use on hover.
pub fn badge(ui: &mut egui::Ui, days_left: u32, last_day: chrono::NaiveDate) {
    let p = theme::palette(ui.ctx());
    let text = if days_left <= 1 {
        t!("demo.badge_last")
    } else {
        t!("demo.badge", count = days_left)
    };
    let date = last_day.format("%Y-%m-%d").to_string();
    ui.label(egui::RichText::new(text).small().color(p.highlight))
        .on_hover_text(t!("demo.hint", date = date));
}

/// The only screen once the demonstration is over, or, with the brand's
/// panel, until this computer is activated: study data stays on the
/// computer, and the brand's contact page is one click away.
pub fn expired(ui: &mut egui::Ui, engine: Option<&kg_engine::Engine>) {
    ui.add_space(24.0);
    ui.add(crate::icons::logo(72.0));
    ui.add_space(12.0);
    let (title, body, contact) = if kg_brand::has_server() {
        let first = engine.is_none_or(|e| e.license().is_none());
        (
            t!("panel.activate_title"),
            if first {
                t!("panel.activate_body")
            } else {
                t!("panel.locked_body")
            },
            t!("panel.get_license"),
        )
    } else {
        (
            t!("demo.expired_title"),
            t!("demo.expired_body"),
            t!("demo.contact"),
        )
    };
    widgets::header(ui, &title, &t!("app.name"));
    ui.add_space(8.0);
    ui.label(body);
    let url = crate::brand::contact_url();
    if !url.is_empty() {
        ui.add_space(16.0);
        if widgets::primary_button(ui, &contact).clicked() {
            crate::app::open_url(url);
        }
    }
}

/// Licence activation shared by Settings and the locked screen: offline, or
/// with the brand's panel and its status.
pub fn activation(ui: &mut egui::Ui, engine: &kg_engine::Engine) -> Option<Command> {
    if kg_brand::LICENSE_PUBLIC_KEY.is_empty() {
        return None;
    }
    let mut command = None;
    ui.add_space(16.0);
    ui.label(t!("licensing.title"));
    if let Some(license) = engine.license() {
        let claims = license.claims();
        ui.label(t!(
            "licensing.details",
            customer = claims.customer.as_str(),
            seats = claims.seats,
            expires = claims.expires.to_string()
        ));
        if kg_brand::has_server() {
            panel_status(ui, engine, &mut command);
        } else if engine.demo() == kg_engine::DemoStatus::Full {
            ui.label(t!("licensing.active"));
        } else {
            ui.label(t!("licensing.expired"));
        }
    } else if kg_brand::has_server() {
        panel_status(ui, engine, &mut command);
    }
    if kg_brand::has_server() {
        widgets::hint(ui, &t!("panel.hint"));
    } else {
        widgets::hint(ui, &t!("licensing.offline"));
    }
    let id = egui::Id::new("license_input");
    let mut input = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_default();
    // With a panel the student types the short access code; the long signed
    // key still works there and is the only kind without a panel.
    let placeholder = if kg_brand::has_server() {
        t!("panel.placeholder")
    } else {
        t!("licensing.placeholder")
    };
    ui.add(
        egui::TextEdit::multiline(&mut input)
            .hint_text(placeholder)
            .desired_width(480.0)
            .desired_rows(3)
            .char_limit(8192),
    );
    let busy = engine.panel_status() == PanelStatus::Busy;
    let activate = ui
        .add_enabled(
            !input.trim().is_empty() && !busy,
            egui::Button::new(t!("licensing.activate")),
        )
        .clicked();
    if activate {
        command = Some(Command::ActivateLicense(std::mem::take(&mut input)));
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, input));
    command
}

/// What the panel says about this computer, with the action that fits.
fn panel_status(ui: &mut egui::Ui, engine: &kg_engine::Engine, command: &mut Option<Command>) {
    let p = theme::palette(ui.ctx());
    let date = |at: chrono::DateTime<chrono::Utc>| {
        crate::i18n::date(at.with_timezone(&chrono::Local).date_naive())
    };
    match engine.panel_status() {
        PanelStatus::Off | PanelStatus::NotActivated => {}
        PanelStatus::Busy => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!("panel.busy"));
            });
        }
        PanelStatus::Connected { last_ok } => {
            ui.label(RichText::new(t!("panel.connected", date = date(last_ok))).color(p.success));
            sign_out(ui, command);
        }
        PanelStatus::Offline { last_ok, days_left } => {
            ui.label(
                RichText::new(t!("panel.offline", date = date(last_ok), days = days_left))
                    .color(p.warning),
            );
            sign_out(ui, command);
        }
        PanelStatus::OfflineTooLong { last_ok } => {
            ui.label(
                RichText::new(t!(
                    "panel.offline_too_long",
                    date = date(last_ok),
                    days = kg_engine::OFFLINE_GRACE_DAYS
                ))
                .color(p.danger),
            );
            if ui.button(t!("panel.retry")).clicked() {
                *command = Some(Command::ReactivateDevice);
            }
        }
        PanelStatus::Refused(refusal) => {
            ui.label(RichText::new(refusal_message(refusal)).color(p.danger));
            if refusal == Refusal::BillingPaused {
                ui.label(t!("panel.billing_paused_hint"));
                let support = crate::brand::support_url();
                let url = if support.is_empty() {
                    crate::brand::contact_url()
                } else {
                    support
                };
                if !url.is_empty()
                    && widgets::primary_button(ui, &t!("licensing.support")).clicked()
                {
                    crate::app::open_url(url);
                }
            }
            let again = match refusal {
                Refusal::DeviceRevoked => Some(t!("panel.reactivate")),
                Refusal::LicenseInvalid => None,
                _ => Some(t!("panel.retry")),
            };
            if engine.license().is_some()
                && let Some(label) = again
                && ui.button(label).clicked()
            {
                *command = Some(Command::ReactivateDevice);
            }
        }
    }
}

fn sign_out(ui: &mut egui::Ui, command: &mut Option<Command>) {
    if ui
        .button(t!("panel.sign_out"))
        .on_hover_text(t!("panel.sign_out_hint"))
        .clicked()
    {
        *command = Some(Command::SignOutDevice);
    }
}

/// A screen the licence's plan does not open: what is missing and who to ask.
pub fn module_locked(ui: &mut egui::Ui, module: kg_engine::Module) {
    ui.add_space(24.0);
    widgets::header(ui, &t!("modules.locked_title"), &t!("app.name"));
    ui.add_space(8.0);
    ui.label(t!("modules.locked_body", module = module_name(module)));
    let url = crate::brand::contact_url();
    if !url.is_empty() {
        ui.add_space(16.0);
        if widgets::primary_button(ui, &t!("modules.contact")).clicked() {
            crate::app::open_url(url);
        }
    }
}

/// A module's name, as the panel calls it.
pub fn module_name(module: kg_engine::Module) -> String {
    match module {
        kg_engine::Module::Lessons => t!("modules.lessons"),
        kg_engine::Module::Content => t!("modules.content"),
    }
    .into_owned()
}

/// Why the panel refused this computer, for the student.
pub fn refusal_message(refusal: Refusal) -> String {
    match refusal {
        Refusal::LicenseInvalid => t!("panel.refused.license_invalid"),
        Refusal::LicenseRevoked => t!("panel.refused.license_revoked"),
        Refusal::LicenseExpired => t!("panel.refused.license_expired"),
        Refusal::ClientInactive => t!("panel.refused.client_inactive"),
        Refusal::NoSeatsLeft => t!("panel.refused.no_seats_left"),
        Refusal::DeviceRevoked => t!("panel.refused.device_revoked"),
        Refusal::DeviceBlocked => t!("panel.refused.device_blocked"),
        Refusal::BillingPaused => t!("panel.refused.billing_paused"),
    }
    .into_owned()
}
