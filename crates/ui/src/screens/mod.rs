//! One module per screen. Each reads engine state and collects user actions,
//! which run after drawing; none of them contain business rules.

pub mod board;
pub mod challenges;
pub mod classes;
pub mod demo;
pub mod developer;
pub mod discursive;
pub mod edital;
pub mod handout;
pub mod lesson;
pub mod mock;
pub mod plan;
pub mod planning;
pub mod profile;
pub mod progress;
pub mod quiz;
pub mod reader;
pub mod review;
pub mod settings;
pub mod setup;
pub mod today;
pub mod welcome;

use kg_engine::{Command, Engine, StudyPlan};

use crate::app::{KnowGiverApp, Screen};

/// What the user asked for while a screen was drawn.
#[derive(Debug, Default)]
pub struct Actions {
    commands: Vec<Command>,
    go_to: Option<Screen>,
}

impl Actions {
    pub fn send(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub fn go_to(&mut self, screen: Screen) {
        self.go_to = Some(screen);
    }

    pub fn run(self, app: &mut KnowGiverApp) {
        for command in self.commands {
            app.dispatch(command);
        }
        if let Some(screen) = self.go_to {
            app.go_to(screen);
        }
    }
}

/// Study week of the plan we are in today (1-based).
pub fn current_week(plan: &StudyPlan) -> u32 {
    let days = (Engine::today() - plan.created_on).num_days().max(0);
    ((days / 7) as u32 + 1).clamp(1, plan.weeks.max(1))
}

/// What a topic's lesson button does: continue an unfinished lesson, look
/// back at a taken one, or start one.
pub fn lesson_action(engine: &Engine, topic: &kg_engine::Topic) -> (String, Command) {
    if engine.has_lesson_draft(topic.id) {
        (
            t!("plan.lesson_continue").into_owned(),
            Command::ResumeLesson(topic.id),
        )
    } else if topic.lesson_taken() {
        (
            t!("plan.lesson_again").into_owned(),
            Command::OpenRecap(topic.id),
        )
    } else {
        (
            t!("plan.lesson").into_owned(),
            Command::StartLesson(topic.id),
        )
    }
}

/// The three lesson types, the suggested one marked with its reason.
pub fn lesson_type_menu(
    ui: &mut egui::Ui,
    engine: &Engine,
    topic: kg_engine::TopicId,
    actions: &mut Actions,
) {
    let (suggested, reason) = engine.suggested_lesson_length(topic);
    ui.menu_button("▾", |ui| {
        ui.label(
            egui::RichText::new(t!("lesson_type.choose"))
                .small()
                .strong(),
        );
        for length in kg_engine::LessonLength::ALL {
            let mut text = crate::i18n::lesson_length(length);
            if length == suggested {
                text = format!("{text} · {}", t!("lesson_type.suggested"));
            }
            let button = ui
                .button(text)
                .on_hover_text(crate::i18n::lesson_length_hint(length));
            if button.clicked() {
                actions.send(Command::StartLessonAs { topic, length });
                actions.go_to(Screen::Lesson);
                ui.close();
            }
        }
        if let Some(why) = crate::i18n::length_reason(reason) {
            ui.label(egui::RichText::new(why).small());
        }
    })
    .response
    .on_hover_text(t!("lesson_type.menu_hint"));
}

/// A small button running [`lesson_action`]; opens the lesson screen.
pub fn lesson_button(
    ui: &mut egui::Ui,
    engine: &Engine,
    topic: &kg_engine::Topic,
    actions: &mut Actions,
) {
    let (label, command) = lesson_action(engine, topic);
    if ui.small_button(label).clicked() {
        actions.send(command);
        actions.go_to(Screen::Lesson);
    }
    // A new lesson: the student may pick its type (Phase 10).
    if matches!(lesson_action(engine, topic).1, Command::StartLesson(_)) {
        lesson_type_menu(ui, engine, topic.id, actions);
    }
}
