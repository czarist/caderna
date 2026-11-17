//! The hidden developer area (Phase W9): opened by clicking the version in
//! Settings → About seven times or with `--developer`. Only for the team:
//! it needs a developer code made by an admin in the panel. Nothing here is
//! mentioned in the tour or the README for students.

use egui::RichText;
use kg_engine::{Command, DeveloperError, Environment};

use super::Actions;
use crate::app::KnowGiverApp;
use crate::{i18n, widgets};

/// Clicks on the version that open the area.
pub const CLICKS: u32 = 7;

#[derive(Debug, Clone, Default)]
struct AreaState {
    open: bool,
    code: String,
    next: Option<Environment>,
    port: String,
    copy: bool,
    fresh: bool,
    clicks: u32,
}

fn state_id() -> egui::Id {
    egui::Id::new("developer_area")
}

/// Counts a click on the version; the seventh opens the area.
pub fn version_clicked(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let state: &mut AreaState = d.get_temp_mut_or_default(state_id());
        state.clicks += 1;
        if state.clicks >= CLICKS {
            state.clicks = 0;
            state.open = true;
        }
    });
}

fn environment_name(environment: Environment) -> String {
    match environment {
        Environment::Production => t!("developer.production"),
        Environment::Homologation => t!("developer.homologation"),
        Environment::Development => t!("developer.development"),
    }
    .into_owned()
}

pub fn error_text(err: DeveloperError) -> String {
    match err {
        DeveloperError::NoKey => t!("developer.error.no_key"),
        DeveloperError::Invalid | DeveloperError::Signature | DeveloperError::Brand => {
            t!("developer.error.invalid")
        }
        DeveloperError::Expired => t!("developer.error.expired"),
        DeveloperError::OtherDevice => t!("developer.error.other_device"),
        DeveloperError::Environment => t!("developer.error.environment"),
        DeveloperError::Refused => t!("developer.error.refused"),
    }
    .into_owned()
}

/// Starts the app again with the same arguments (but `--developer`) and
/// closes this one; the new run opens the chosen environment's data folder.
pub fn restart(ctx: &egui::Context) {
    match std::env::current_exe() {
        Ok(exe) => {
            let args: Vec<String> = std::env::args()
                .skip(1)
                .filter(|a| a != "--developer")
                .collect();
            if let Err(err) = std::process::Command::new(exe).args(args).spawn() {
                tracing::warn!(%err, "could not restart the app");
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        Err(err) => tracing::warn!(%err, "could not find the app to restart"),
    }
}

/// `--developer` on the command line opens the area once.
static FLAG_SEEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn window(app: &mut KnowGiverApp, ctx: &egui::Context) {
    let mut state: AreaState = ctx.data(|d| d.get_temp(state_id())).unwrap_or_default();
    if !FLAG_SEEN.swap(true, std::sync::atomic::Ordering::Relaxed)
        && std::env::args().any(|a| a == "--developer")
    {
        state.open = true;
    }
    if !state.open {
        ctx.data_mut(|d| d.insert_temp(state_id(), state));
        return;
    }
    let Ok(engine) = &app.engine else {
        return;
    };
    let mut actions = Actions::default();
    let mut open = true;
    egui::Window::new(t!("developer.title"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .show(ctx, |ui| {
            let Some(status) = engine.developer_status() else {
                widgets::hint(ui, &t!("developer.unavailable"));
                return;
            };
            widgets::hint(
                ui,
                &t!(
                    "developer.now",
                    environment = environment_name(status.environment)
                ),
            );
            if let Some(err) = status.ended {
                widgets::hint(ui, &t!("developer.ended", reason = error_text(err)));
            }
            ui.add_space(6.0);
            let Some(claims) = status.claims.clone() else {
                ui.label(t!("developer.code_hint"));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.code)
                            .hint_text("DEV-XXXX-XXXX-XXXX")
                            .desired_width(200.0),
                    );
                    let ready = state.code.trim().len() >= 8 && !status.exchanging;
                    if ui
                        .add_enabled(ready, egui::Button::new(t!("developer.use_code")))
                        .clicked()
                    {
                        actions.send(Command::DeveloperCode(state.code.trim().to_owned()));
                        state.code.clear();
                    }
                    if status.exchanging {
                        ui.spinner();
                    }
                });
                return;
            };
            ui.label(
                RichText::new(t!(
                    "developer.token",
                    person = claims.person,
                    date = i18n::date(claims.expires_at.date_naive())
                ))
                .strong(),
            );
            ui.add_space(6.0);
            let next = state.next.get_or_insert(status.next);
            ui.label(t!("developer.environment"));
            for environment in Environment::ALL {
                if !claims.opens(environment) || environment.compiled_url().is_empty() {
                    continue;
                }
                ui.radio_value(
                    next,
                    environment,
                    format!(
                        "{} · {}",
                        environment_name(environment),
                        environment.compiled_url()
                    ),
                );
            }
            let next = *next;
            if next == Environment::Development {
                ui.horizontal(|ui| {
                    ui.label(t!("developer.port"));
                    ui.add(
                        egui::TextEdit::singleline(&mut state.port)
                            .hint_text("8080")
                            .desired_width(70.0),
                    );
                });
            }
            if next != Environment::Production && next != status.environment {
                let has_data = status.with_data.contains(&next);
                if status.environment == Environment::Production {
                    ui.checkbox(&mut state.copy, t!("developer.copy"));
                }
                if has_data && !state.copy {
                    ui.checkbox(&mut state.fresh, t!("developer.fresh"));
                }
            }
            let port: Option<u16> = state.port.trim().parse().ok();
            ui.horizontal(|ui| {
                if ui.button(t!("developer.switch")).clicked() {
                    // The app starts again once the choice is saved.
                    actions.send(Command::DeveloperSwitch {
                        environment: next,
                        port,
                        copy: state.copy,
                        fresh: state.fresh,
                    });
                }
                if ui.button(t!("developer.leave")).clicked() {
                    actions.send(Command::DeveloperLeave);
                }
            });
            widgets::hint(ui, &t!("developer.switch_hint"));
            ui.add_space(8.0);
            ui.label(RichText::new(t!("developer.tools")).strong());
            ui.horizontal_wrapped(|ui| {
                for environment in &status.with_data {
                    if *environment != status.environment
                        && ui
                            .small_button(t!(
                                "developer.reset",
                                environment = environment_name(*environment)
                            ))
                            .clicked()
                    {
                        actions.send(Command::DeveloperReset(*environment));
                    }
                }
                if ui.small_button(t!("developer.open_folder")).clicked() {
                    open_folder(&engine.paths().data_dir);
                }
                if ui.small_button(t!("developer.check_now")).clicked() {
                    actions.send(Command::SyncClasses);
                }
            });
            let modules: Vec<&str> = kg_engine::Module::ALL
                .iter()
                .filter(|m| engine.module_enabled(**m))
                .map(|m| m.id())
                .collect();
            widgets::hint(ui, &t!("developer.modules", modules = modules.join(", ")));
        });
    state.open = open;
    ctx.data_mut(|d| d.insert_temp(state_id(), state));
    actions.run(app);
}

fn open_folder(path: &std::path::Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(err) = std::process::Command::new(program).arg(path).spawn() {
        tracing::warn!(%err, "could not open the data folder");
    }
}
