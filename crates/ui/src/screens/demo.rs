//! Demonstration builds: the days left in the sidebar and the screen shown
//! once the demonstration is over.

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

/// The only screen once the demonstration is over: study data stays on the
/// computer, and the brand's contact page is one click away.
pub fn expired(ui: &mut egui::Ui) {
    ui.add_space(24.0);
    ui.add(crate::icons::logo(72.0));
    ui.add_space(12.0);
    widgets::header(ui, &t!("demo.expired_title"), &t!("app.name"));
    ui.add_space(8.0);
    ui.label(t!("demo.expired_body"));
    if !kg_brand::DEMO_CONTACT_URL.is_empty() {
        ui.add_space(16.0);
        if widgets::primary_button(ui, &t!("demo.contact")).clicked() {
            crate::app::open_url(kg_brand::DEMO_CONTACT_URL);
        }
    }
}

/// Offline activation input shared by Settings and the expired screen.
pub fn activation(ui: &mut egui::Ui, engine: &kg_engine::Engine) -> Option<String> {
    if kg_brand::LICENSE_PUBLIC_KEY.is_empty() {
        return None;
    }
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
        if engine.demo() == kg_engine::DemoStatus::Full {
            ui.label(t!("licensing.active"));
        } else {
            ui.label(t!("licensing.expired"));
        }
    }
    widgets::hint(ui, &t!("licensing.offline"));
    let id = egui::Id::new("license_input");
    let mut input = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_default();
    ui.add(
        egui::TextEdit::multiline(&mut input)
            .hint_text(t!("licensing.placeholder"))
            .desired_width(480.0)
            .desired_rows(3)
            .char_limit(8192),
    );
    let activate = ui
        .add_enabled(
            !input.trim().is_empty(),
            egui::Button::new(t!("licensing.activate")),
        )
        .clicked();
    let token = activate.then(|| std::mem::take(&mut input));
    ui.ctx().data_mut(|d| d.insert_temp(id, input));
    token
}
