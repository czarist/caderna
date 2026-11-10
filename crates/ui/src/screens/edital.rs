//! Edital import, plan generation and the list of saved plans.

use egui::RichText;
use kg_engine::{Command, Edital, Engine, ExamPurpose, ImportState, PlanPrefs, PlanSummary};

use super::Actions;
use crate::app::{KnowGiverApp, Screen};
use crate::{i18n, theme, widgets, working};

pub fn show(app: &mut KnowGiverApp, ui: &mut egui::Ui) {
    let mut actions = Actions::default();
    let Ok(engine) = &app.engine else {
        return;
    };
    let state = &mut app.ui;
    widgets::header(ui, &t!("edital.title"), &t!("edital.subtitle"));

    let creates_plans = engine.role().creates_plans();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if !creates_plans {
            student_card(ui, engine, &mut actions);
        } else {
            match engine.import_state() {
                ImportState::Extracting { file } => {
                    widgets::card(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(t!("edital.extracting", file = file));
                        });
                    });
                }
                ImportState::Ready(edital) => match engine.generating_since() {
                    Some(started) => {
                        let detail = t!("edital.generating", seconds = started.elapsed().as_secs());
                        let note = if state.prepare_all {
                            t!("edital.generating_then_prepare")
                        } else {
                            t!("edital.generating_note")
                        };
                        working::panel(
                            ui,
                            engine,
                            &working::Working {
                                title: &t!("edital.generating_title"),
                                detail: &detail,
                                note: &note,
                                progress: None,
                            },
                            |_| {},
                        );
                    }
                    None => ready_card(
                        ui,
                        engine,
                        edital,
                        &mut state.prefs,
                        &mut state.prepare_all,
                        &mut state.purpose_chosen,
                        &mut actions,
                    ),
                },
                ImportState::Empty | ImportState::Failed => {
                    widgets::card(ui, |ui| {
                        widgets::hint(ui, &t!("edital.empty"));
                        ui.add_space(4.0);
                        if widgets::primary_button(ui, &t!("edital.select_pdf")).clicked()
                            && let Some(path) = pick_pdf()
                        {
                            actions.send(Command::ImportEdital(path));
                        }
                    });
                }
            }
        }

        ui.add_space(20.0);
        ui.label(RichText::new(t!("edital.your_plans")).strong().size(18.0));
        if engine.plans().is_empty() {
            widgets::hint(ui, &t!("edital.no_plans"));
        }
        let active = engine.active_plan().map(|p| p.id);
        for summary in engine.plans() {
            plan_row(
                ui,
                engine,
                summary,
                active == Some(summary.id),
                &mut state.confirm_delete,
                &mut actions,
            );
        }
    });
    actions.run(app);
}

fn ready_card(
    ui: &mut egui::Ui,
    engine: &Engine,
    edital: &Edital,
    prefs: &mut PlanPrefs,
    prepare_all: &mut bool,
    purpose_chosen: &mut bool,
    actions: &mut Actions,
) {
    widgets::card(ui, |ui| {
        ui.label(RichText::new(&edital.file_name).strong());
        widgets::hint(
            ui,
            &t!(
                "edital.summary",
                pages = edital.pages,
                chars = edital.text.char_count()
            ),
        );
        egui::CollapsingHeader::new(t!("edital.preview"))
            .id_salt("edital_preview")
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        let preview: String = edital.text.as_str().chars().take(3_000).collect();
                        ui.label(RichText::new(preview).small());
                    });
            });

        ui.add_space(8.0);
        purpose_picker(ui, &mut prefs.purpose, purpose_chosen);
        ui.add_space(8.0);
        let roles = engine.edital_roles();
        let choose_role = roles.len() >= 2;
        if choose_role {
            role_picker(ui, roles, &mut prefs.role);
            ui.add_space(8.0);
        } else if let [only] = roles
            && prefs.role.trim().is_empty()
        {
            prefs.role.clone_from(&only.name);
        }
        egui::Grid::new("plan_prefs")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label(t!("edital.weeks"));
                ui.add(
                    egui::DragValue::new(&mut prefs.weeks)
                        .range(1..=104)
                        .speed(0.2),
                );
                ui.end_row();

                ui.label(t!("edital.hours"));
                ui.add(
                    egui::DragValue::new(&mut prefs.hours_per_week)
                        .range(1..=80)
                        .speed(0.2),
                );
                ui.end_row();

                if !choose_role {
                    ui.label(t!("edital.role"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut prefs.role)
                                .hint_text(t!("edital.role_hint"))
                                .desired_width(360.0),
                        );
                        if engine.finding_roles() {
                            ui.spinner();
                            widgets::hint(ui, &t!("edital.finding_roles"));
                        }
                    });
                    ui.end_row();
                }
            });

        ui.add_space(8.0);
        ui.checkbox(prepare_all, t!("edital.prepare_all"));
        widgets::hint(ui, &t!("edital.prepare_all_hint"));
        ui.add_space(8.0);
        let role_chosen = !choose_role || roles.iter().any(|r| r.name == prefs.role);
        ui.horizontal(|ui| {
            let generate = ui
                .add_enabled_ui(role_chosen && *purpose_chosen, |ui| {
                    widgets::primary_button(ui, &t!("edital.generate"))
                })
                .inner;
            if !*purpose_chosen {
                widgets::hint(ui, &t!("edital.pick_purpose_first"));
            } else if !role_chosen {
                widgets::hint(ui, &t!("edital.pick_role_first"));
            }
            if generate.clicked() {
                *purpose_chosen = false;
                actions.send(Command::GeneratePlan(prefs.clone()));
                if *prepare_all {
                    actions.send(Command::PrepareContent(super::handout::prepare_request()));
                }
            }
            if ui.button(t!("edital.other_file")).clicked()
                && let Some(path) = pick_pdf()
            {
                actions.send(Command::ImportEdital(path));
            }
        });
    });
}

/// Students do not create plans: their classrooms give them.
fn student_card(ui: &mut egui::Ui, engine: &Engine, actions: &mut Actions) {
    widgets::card(ui, |ui| {
        ui.label(RichText::new(t!("classes.student_title")).strong());
        widgets::hint(ui, &t!("classes.student_note"));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if engine.syncing_classes() {
                ui.spinner();
                widgets::hint(ui, &t!("classes.syncing"));
            } else if ui.button(t!("classes.sync_now")).clicked() {
                actions.send(Command::SyncClasses);
            }
            if let Some(at) = engine.last_class_sync() {
                widgets::hint(
                    ui,
                    &t!(
                        "classes.last_sync",
                        date = i18n::date(at.with_timezone(&chrono::Local).date_naive())
                    ),
                );
            }
        });
    });
}

/// What the plan prepares for; the prompts adapt to it. Asked every time.
fn purpose_picker(ui: &mut egui::Ui, purpose: &mut ExamPurpose, chosen: &mut bool) {
    ui.label(RichText::new(t!("edital.purpose_title")).strong());
    widgets::hint(ui, &t!("edital.purpose_hint"));
    ui.horizontal_wrapped(|ui| {
        for option in ExamPurpose::ALL {
            let selected = *chosen && *purpose == option;
            if ui
                .selectable_label(selected, i18n::purpose_name(option))
                .clicked()
            {
                *purpose = option;
                *chosen = true;
            }
        }
    });
}

/// One option per role; the plan covers only the chosen one.
fn role_picker(ui: &mut egui::Ui, roles: &[kg_engine::Role], chosen: &mut String) {
    let p = theme::palette(ui.ctx());
    ui.label(RichText::new(t!("edital.roles_title", count = roles.len())).strong());
    widgets::hint(ui, &t!("edital.roles_hint"));
    ui.add_space(4.0);
    for role in roles {
        let selected = *chosen == role.name;
        let label = match role.number {
            Some(n) => format!("{}  ·  {}", t!("edital.role_number", number = n), role.name),
            None => role.name.clone(),
        };
        let text = if selected {
            RichText::new(label).strong().color(p.text)
        } else {
            RichText::new(label).color(p.text)
        };
        if ui.add(egui::RadioButton::new(selected, text)).clicked() {
            chosen.clone_from(&role.name);
        }
    }
}

fn plan_row(
    ui: &mut egui::Ui,
    engine: &Engine,
    summary: &PlanSummary,
    is_active: bool,
    confirm_delete: &mut Option<kg_engine::PlanId>,
    actions: &mut Actions,
) {
    let revalidating = engine.revalidating();
    let creates_plans = engine.role().creates_plans();
    let course = engine.course_link(summary.id);
    let p = theme::palette(ui.ctx());
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&summary.title).strong());
                    if is_active {
                        widgets::chip(ui, &t!("edital.active"), p.accent);
                    }
                    match course {
                        Some(link) if link.assigned => {
                            widgets::chip(ui, &t!("classes.from_teacher"), p.muted);
                        }
                        Some(_) => widgets::chip(ui, &t!("classes.published_short"), p.muted),
                        None => {}
                    }
                });
                if !summary.exam.is_empty() {
                    widgets::hint(ui, &summary.exam);
                }
                ui.label(
                    RichText::new(t!(
                        "edital.plan_meta",
                        date = i18n::date(summary.created_on),
                        done = summary.done_count,
                        total = summary.topic_count
                    ))
                    .small()
                    .color(p.muted),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if creates_plans && ui.button(t!("edital.delete")).clicked() {
                    *confirm_delete = Some(summary.id);
                }
                if ui.button(t!("edital.open")).clicked() {
                    actions.send(Command::SelectPlan(summary.id));
                    actions.go_to(Screen::Plan);
                }
                if !creates_plans {
                    // Students use the plan as their teacher made it.
                } else if revalidating == Some(summary.id) {
                    ui.spinner();
                } else if ui
                    .add_enabled(
                        revalidating.is_none(),
                        egui::Button::new(t!("edital.revalidate")),
                    )
                    .on_hover_text(t!("edital.revalidate_hint"))
                    .clicked()
                    && let Some(path) = pick_pdf()
                {
                    actions.send(Command::RevalidateEdital {
                        plan: summary.id,
                        path,
                    });
                }
            });
        });
        if *confirm_delete == Some(summary.id) {
            widgets::callout(ui, p.danger, |ui| {
                ui.label(t!("edital.confirm_delete"));
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t!("edital.delete")).color(p.danger))
                        .clicked()
                    {
                        actions.send(Command::DeletePlan(summary.id));
                        *confirm_delete = None;
                    }
                    if ui.button(t!("common.cancel")).clicked() {
                        *confirm_delete = None;
                    }
                });
            });
        }
    });
}

fn pick_pdf() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title(t!("edital.select_pdf"))
        .add_filter(t!("edital.pdf_filter"), &["pdf"])
        .pick_file()
}
