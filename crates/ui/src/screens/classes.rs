//! Classes, for teachers: publish a plan to classrooms and see how each
//! student is doing. Classrooms and students are organised in the panel's
//! client area; this screen only reads them.

use egui::RichText;
use kg_engine::{ClassRole, Classroom, Command, Engine, PlanId};

use super::Actions;
use crate::app::KnowGiverApp;
use crate::{i18n, theme, widgets};

pub fn show(app: &mut KnowGiverApp, ui: &mut egui::Ui) {
    let mut actions = Actions::default();
    let Ok(engine) = &app.engine else {
        return;
    };
    let state = &mut app.ui;
    widgets::header(ui, &t!("classes.title"), &t!("classes.subtitle"));
    if engine.role() != ClassRole::Teacher {
        widgets::hint(ui, &t!("classes.teachers_only"));
        return;
    }
    // Asked once per visit; "Atualizar" asks again.
    if !state.classes_requested && !engine.loading_classrooms() {
        state.classes_requested = true;
        actions.send(Command::LoadClassrooms);
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        publish_card(ui, engine, state, &mut actions);
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("classes.classrooms")).strong().size(18.0));
            if engine.loading_classrooms() {
                ui.spinner();
            } else if ui.button(t!("classes.refresh")).clicked() {
                actions.send(Command::LoadClassrooms);
            }
        });
        match engine.classrooms() {
            None if !engine.loading_classrooms() => widgets::hint(ui, &t!("classes.not_loaded")),
            None => {}
            Some([]) => widgets::hint(ui, &t!("classes.no_classrooms")),
            Some(classrooms) => {
                for classroom in classrooms {
                    classroom_card(ui, engine, classroom);
                }
            }
        }
    });
    actions.run(app);
}

/// The plan to publish and the classrooms that receive it.
fn publish_card(
    ui: &mut egui::Ui,
    engine: &Engine,
    state: &mut crate::app::UiState,
    actions: &mut Actions,
) {
    let p = theme::palette(ui.ctx());
    widgets::card(ui, |ui| {
        ui.label(RichText::new(t!("classes.publish_title")).strong());
        widgets::hint(ui, &t!("classes.publish_hint"));
        ui.add_space(6.0);
        let own: Vec<_> = engine
            .plans()
            .iter()
            .filter(|plan| {
                engine
                    .course_link(plan.id)
                    .is_none_or(|link| !link.assigned)
            })
            .collect();
        if own.is_empty() {
            widgets::hint(ui, &t!("classes.no_plans"));
            return;
        }
        let chosen: PlanId = state
            .publish_plan
            .filter(|id| own.iter().any(|plan| plan.id == *id))
            .or_else(|| engine.active_plan().map(|plan| plan.id))
            .filter(|id| own.iter().any(|plan| plan.id == *id))
            .unwrap_or(own[0].id);
        let title = own
            .iter()
            .find(|plan| plan.id == chosen)
            .map(|plan| plan.title.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("publish_plan")
            .selected_text(title)
            .width(360.0)
            .show_ui(ui, |ui| {
                for plan in &own {
                    if ui
                        .selectable_label(plan.id == chosen, &plan.title)
                        .clicked()
                    {
                        state.publish_plan = Some(plan.id);
                    }
                }
            });
        if let Some(link) = engine.course_link(chosen) {
            widgets::chip(
                ui,
                &t!("classes.published", version = link.version),
                p.accent,
            );
            // The classrooms that already have it, the first time it is shown.
            if state.publish_to.is_empty()
                && let Some(classrooms) = engine.classrooms()
            {
                state.publish_to = classrooms
                    .iter()
                    .filter(|c| c.course_ids.contains(&link.course_id))
                    .map(|c| c.id)
                    .collect();
            }
        }
        ui.add_space(6.0);
        match engine.classrooms() {
            Some(classrooms) if !classrooms.is_empty() => {
                ui.horizontal_wrapped(|ui| {
                    for classroom in classrooms {
                        let mut on = state.publish_to.contains(&classroom.id);
                        if ui.checkbox(&mut on, &classroom.name).changed() {
                            if on {
                                state.publish_to.insert(classroom.id);
                            } else {
                                state.publish_to.remove(&classroom.id);
                            }
                        }
                    }
                });
            }
            _ => widgets::hint(ui, &t!("classes.no_classrooms")),
        }
        ui.add_space(6.0);
        widgets::hint(ui, &t!("classes.publish_note"));
        ui.horizontal(|ui| {
            if engine.publishing().is_some() {
                ui.spinner();
                widgets::hint(ui, &t!("classes.publishing"));
            } else if widgets::primary_button(ui, &t!("classes.publish")).clicked() {
                let mut classrooms: Vec<i64> = state.publish_to.iter().copied().collect();
                classrooms.sort_unstable();
                actions.send(Command::PublishPlan {
                    plan: chosen,
                    classrooms,
                });
            }
        });
    });
}

fn classroom_card(ui: &mut egui::Ui, engine: &Engine, classroom: &Classroom) {
    let p = theme::palette(ui.ctx());
    widgets::card(ui, |ui| {
        egui::CollapsingHeader::new(
            RichText::new(t!(
                "classes.classroom_header",
                name = classroom.name.as_str(),
                count = classroom.students.len()
            ))
            .strong(),
        )
        .id_salt(("classroom", classroom.id))
        .default_open(true)
        .show(ui, |ui| {
            if classroom.students.is_empty() {
                widgets::hint(ui, &t!("classes.no_students"));
                return;
            }
            if classroom.course_ids.is_empty() {
                widgets::hint(ui, &t!("classes.no_courses"));
            }
            egui::Grid::new(("classroom_grid", classroom.id))
                .num_columns(7)
                .striped(true)
                .spacing([14.0, 6.0])
                .show(ui, |ui| {
                    for heading in [
                        t!("classes.col.student"),
                        t!("classes.col.course"),
                        t!("classes.col.topics"),
                        t!("classes.col.quizzes"),
                        t!("classes.col.lessons"),
                        t!("classes.col.time"),
                        t!("classes.col.last"),
                    ] {
                        ui.label(RichText::new(heading).small().color(p.muted));
                    }
                    ui.end_row();
                    for student in &classroom.students {
                        let time = t!(
                            "classes.time",
                            hours = student.minutes / 60,
                            minutes = student.minutes % 60
                        );
                        if student.courses.is_empty() {
                            ui.label(&student.name);
                            ui.label("—");
                            ui.label("—");
                            ui.label("—");
                            ui.label("—");
                            ui.label(time.as_ref());
                            ui.label("—");
                            ui.end_row();
                        }
                        for course in &student.courses {
                            ui.label(&student.name);
                            ui.label(course_title(engine, course.course_id));
                            ui.label(format!("{} / {}", course.topics_done, course.topics));
                            if let Some(percent) =
                                (100 * course.quiz_correct).checked_div(course.quiz_total)
                            {
                                ui.label(format!(
                                    "{percent}% ({}/{})",
                                    course.quiz_correct, course.quiz_total
                                ));
                            } else {
                                ui.label("—");
                            }
                            ui.label(course.lessons.to_string());
                            ui.label(time.as_ref());
                            ui.label(
                                course
                                    .last_activity
                                    .as_deref()
                                    .and_then(|day| day.parse().ok())
                                    .map_or_else(|| "—".to_owned(), i18n::date),
                            );
                            ui.end_row();
                        }
                    }
                });
        });
    });
}

/// The title of a course published from this computer, or its number.
fn course_title(engine: &Engine, course: i64) -> String {
    engine
        .plans()
        .iter()
        .find(|plan| {
            engine
                .course_link(plan.id)
                .is_some_and(|link| link.course_id == course)
        })
        .map_or_else(
            || t!("classes.course_number", number = course).into_owned(),
            |plan| plan.title.clone(),
        )
}
