//! egui interface for Know Giver and its branded builds.
//!
//! Presentation only: all state lives in [`kg_engine::Engine`], and user
//! actions are sent to it as [`kg_engine::Command`]s. Every visible string
//! comes from the translation files in `locales/` through the internal `i18n` module.

// Embeds `locales/*.yml` and defines the lookup used by `t!`.
rust_i18n::i18n!("../../locales", fallback = "pt-BR");

/// `rust_i18n::t!` with the brand filled in (see [`brand`]): `%{app}` (product),
/// `%{company}` and `%{license}` (the mark's licence) in any translation. A
/// text the client set in its panel replaces the app's own.
macro_rules! t {
    ($key:expr $(, $name:ident = $value:expr)* $(,)?) => {{
        let key = $key;
        match crate::brand::text(::core::convert::AsRef::<str>::as_ref(&key)) {
            Some(text) => crate::brand::fill(
                text,
                &[
                    ("app", crate::brand::name().to_string()),
                    ("company", crate::brand::company().to_string()),
                    ("license", kg_brand::MARK_LICENSE.to_string()),
                    $((stringify!($name), ($value).to_string()),)*
                ],
            ),
            None => rust_i18n::t!(
                key,
                app = crate::brand::name(),
                company = crate::brand::company(),
                license = kg_brand::MARK_LICENSE
                $(, $name = $value)*
            ),
        }
    }};
}

mod accounts;
mod app;
mod art;
mod brand;
mod capture;
mod export;
mod i18n;
mod icon;
mod icons;
mod illustrations;
mod instance;
mod legal;
mod provider_logo;
mod reminder_popup;
mod reminders;
mod screens;
mod stickers;
mod teacher;
mod theme;
mod tour;
mod tray;
mod update_bar;
mod whiteboard;
mod widgets;
mod working;

#[cfg(test)]
mod snapshots;

pub use app::KnowGiverApp;
pub use icon::rgba as icon_rgba;
pub use instance::{SingleInstance, claim_instance};
pub use reminder_popup::run_reminder;
pub use reminders::REMINDER_FLAG;

pub use kg_engine::{AiConfig, AppPaths};

/// Stable identifier for eframe's settings storage. Not shown to the user.
const APP_ID: &str = kg_brand::INSTALL_SLUG;

/// Options passed by the installed app's launcher.
#[derive(Debug, Default, Clone, Copy)]
pub struct RunOptions {
    /// Start without showing the main window.
    pub start_hidden: bool,
    /// Enable reminders selected in the installer.
    pub enable_study_reminders: bool,
}

/// Opens Know Giver and runs until the user exits. `instance` comes from
/// [`claim_instance`]. `env_ai` supplies initial
/// AI settings from `.env`; `run_options` controls installer-selected behavior.
///
/// # Errors
///
/// Returns an error if eframe cannot start the native application.
pub fn run(env_ai: AiConfig, run_options: RunOptions, instance: SingleInstance) -> eframe::Result {
    // The client's brand from its panel, before the window gets its title and icon.
    brand::apply(kg_engine::load_panel_brand(&AppPaths::detect()).as_ref());
    i18n::set(i18n::DEFAULT_LOCALE);
    let title = brand::window_title();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_app_id(APP_ID)
            .with_icon(icon::app_icon())
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 520.0])
            .with_visible(!run_options.start_hidden),
        ..Default::default()
    };

    eframe::run_native(
        APP_ID,
        options,
        Box::new(move |cc| {
            let mut app = KnowGiverApp::with_run_options(cc, env_ai, run_options);
            app.attach_instance(instance.0, &cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
