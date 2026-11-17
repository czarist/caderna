//! The student's profile (Phase 11), a tab of the challenges screen: level
//! and experience from study and results, the medals they pinned, their
//! weekly goals, days off that keep the streak, a calendar of study days
//! and the history per subject. Everything comes from the engine.

use chrono::{Datelike, NaiveDate};
use egui::RichText;
use kg_engine::{ChallengeId, Command, Engine, Profile, SHOWCASE_SIZE, StudyGoals};

use super::Actions;
use crate::{i18n, stickers, theme, widgets};

/// Which tab of the challenges screen is open.
pub fn tab_id() -> egui::Id {
    egui::Id::new("challenges_profile_tab")
}

/// Opens the profile tab next time the challenges screen shows.
pub fn open(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(tab_id(), true));
}

pub fn show(ui: &mut egui::Ui, engine: &Engine, actions: &mut Actions) {
    let profile = engine.profile();
    level_card(ui, &profile);
    ui.add_space(12.0);
    showcase(ui, &profile, actions);
    ui.add_space(12.0);
    goals(ui, engine, actions);
    ui.add_space(12.0);
    days_off(ui, engine, actions);
    ui.add_space(12.0);
    heat_map(ui, &profile);
    if !profile.subjects.is_empty() {
        ui.add_space(12.0);
        subjects(ui, &profile);
    }
}

fn level_card(ui: &mut egui::Ui, profile: &Profile) {
    let p = theme::palette(ui.ctx());
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t!("profile.level", level = profile.level.level))
                    .family(theme::heading_family())
                    .size(26.0),
            );
            ui.add_space(12.0);
            widgets::progress_line(
                ui,
                220.0,
                profile.level.progress(),
                &t!(
                    "profile.experience",
                    current = profile.level.experience,
                    next = profile.level.next
                ),
                p.accent,
            );
        });
        widgets::hint(
            ui,
            &t!(
                "profile.totals",
                hours = profile.minutes / 60,
                days = profile.study_days
            ),
        );
        widgets::hint(ui, &t!("profile.experience_hint"));
    });
}

fn showcase(ui: &mut egui::Ui, profile: &Profile, actions: &mut Actions) {
    ui.label(RichText::new(t!("profile.showcase")).strong());
    if profile.showcase.is_empty() {
        widgets::hint(ui, &t!("profile.showcase_empty"));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for key in &profile.showcase {
            let Some(id) = ChallengeId::from_key(key) else {
                continue;
            };
            ui.vertical(|ui| {
                ui.set_width(96.0);
                ui.add(stickers::image(ui.ctx(), id, 72.0, true))
                    .on_hover_text(i18n::challenge_caption(id));
                ui.add(egui::Label::new(RichText::new(i18n::challenge_name(id)).small()).wrap());
                if ui.small_button(t!("profile.unpin")).clicked() {
                    let keys = profile
                        .showcase
                        .iter()
                        .filter(|k| *k != key)
                        .cloned()
                        .collect();
                    actions.send(Command::SetShowcase(keys));
                }
            });
        }
    });
    widgets::hint(ui, &t!("profile.showcase_hint", count = SHOWCASE_SIZE));
}

/// The goals being edited, kept between frames.
#[derive(Debug, Clone, PartialEq)]
struct GoalDraft {
    hours: f32,
    topics: u32,
    quiz: u8,
    by: String,
}

impl GoalDraft {
    fn of(goals: &StudyGoals) -> Self {
        Self {
            hours: goals.hours_per_week,
            topics: goals.topics_per_week,
            quiz: goals.quiz_percent,
            by: goals.quiz_by.map(i18n::date).unwrap_or_default(),
        }
    }
}

fn goals(ui: &mut egui::Ui, engine: &Engine, actions: &mut Actions) {
    let Some(suggested) = engine.suggested_goals() else {
        return;
    };
    let saved = engine.study_goals();
    let id = egui::Id::new("profile_goal_draft");
    let mut draft: GoalDraft = ui
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| GoalDraft::of(saved.as_ref().unwrap_or(&suggested)));
    widgets::card(ui, |ui| {
        ui.label(RichText::new(t!("profile.goals")).strong());
        widgets::hint(ui, &t!("profile.goals_hint"));
        egui::Grid::new("profile_goals")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(t!("profile.hours_per_week"));
                ui.add(
                    egui::DragValue::new(&mut draft.hours)
                        .range(0.5..=60.0)
                        .speed(0.5)
                        .suffix(" h"),
                );
                ui.end_row();
                ui.label(t!("profile.topics_per_week"));
                ui.add(egui::DragValue::new(&mut draft.topics).range(1..=60));
                ui.end_row();
                ui.label(t!("profile.quiz_target"));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut draft.quiz)
                            .range(10..=100)
                            .suffix("%"),
                    );
                    ui.label(t!("profile.quiz_by"));
                    ui.add(egui::TextEdit::singleline(&mut draft.by).desired_width(100.0));
                });
                ui.end_row();
            });
        widgets::hint(
            ui,
            &t!(
                "profile.suggested",
                hours = suggested.hours_per_week,
                topics = suggested.topics_per_week
            ),
        );
        let by = draft.by.trim();
        let date = (!by.is_empty())
            .then(|| NaiveDate::parse_from_str(by, &t!("format.date")).ok())
            .flatten();
        let date_ok = by.is_empty() || date.is_some();
        if !date_ok {
            widgets::hint(ui, &t!("planning.bad_date", value = by));
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(date_ok, egui::Button::new(t!("profile.save_goals")))
                .clicked()
            {
                actions.send(Command::SaveGoals(Some(StudyGoals {
                    hours_per_week: draft.hours,
                    topics_per_week: draft.topics,
                    quiz_percent: draft.quiz,
                    quiz_by: date,
                })));
            }
            if ui.button(t!("profile.use_suggestion")).clicked() {
                draft = GoalDraft::of(&suggested);
            }
            if saved.is_some() && ui.button(t!("profile.clear_goals")).clicked() {
                actions.send(Command::SaveGoals(None));
                draft = GoalDraft::of(&suggested);
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(id, draft));
}

fn days_off(ui: &mut egui::Ui, engine: &Engine, actions: &mut Actions) {
    let today = Engine::today();
    let off = engine.days_off();
    ui.label(RichText::new(t!("profile.streak", days = engine.streak())).strong());
    widgets::hint(ui, &t!("profile.streak_hint"));
    ui.horizontal_wrapped(|ui| {
        for (label, day) in [
            (t!("profile.today_off"), today),
            (
                t!("profile.yesterday_off"),
                today.pred_opt().unwrap_or(today),
            ),
        ] {
            if !off.contains(&day) && ui.small_button(label).clicked() {
                actions.send(Command::SetDayOff { day, off: true });
            }
        }
    });
    if !off.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t!("profile.days_off")).small());
            for day in off.iter().take(10) {
                if ui
                    .small_button(format!("{} ×", i18n::date(*day)))
                    .on_hover_text(t!("profile.remove_day_off"))
                    .clicked()
                {
                    actions.send(Command::SetDayOff {
                        day: *day,
                        off: false,
                    });
                }
            }
        });
    }
}

/// A grid of study days: a column per week, Monday at the top, darker for
/// more minutes.
fn heat_map(ui: &mut egui::Ui, profile: &Profile) {
    let p = theme::palette(ui.ctx());
    ui.label(RichText::new(t!("profile.calendar")).strong());
    let cell = 12.0;
    let gap = 3.0;
    let weeks = profile.heat.len().div_ceil(7);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(weeks as f32 * (cell + gap), 7.0 * (cell + gap)),
        egui::Sense::hover(),
    );
    let mut hovered = None;
    for (i, (day, minutes)) in profile.heat.iter().enumerate() {
        let week = i / 7;
        let weekday = day.weekday().num_days_from_monday() as usize;
        let min = rect.min + egui::vec2(week as f32 * (cell + gap), weekday as f32 * (cell + gap));
        let square = egui::Rect::from_min_size(min, egui::vec2(cell, cell));
        let strength = match minutes {
            0 => 0.0,
            1..=29 => 0.35,
            30..=89 => 0.6,
            90..=179 => 0.8,
            _ => 1.0,
        };
        let color = if strength == 0.0 {
            p.border
        } else {
            p.accent.gamma_multiply(strength)
        };
        ui.painter().rect_filled(square, 2.0, color);
        if response.hover_pos().is_some_and(|pos| square.contains(pos)) {
            hovered = Some((*day, *minutes));
        }
    }
    if let Some((day, minutes)) = hovered {
        response.on_hover_text(t!(
            "profile.day_minutes",
            date = i18n::date(day),
            minutes = minutes
        ));
    }
}

fn subjects(ui: &mut egui::Ui, profile: &Profile) {
    ui.label(RichText::new(t!("profile.subjects")).strong());
    egui::Grid::new("profile_subjects")
        .num_columns(4)
        .striped(true)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new(t!("mock.subject")).small());
            ui.label(RichText::new(t!("profile.mastered")).small());
            ui.label(RichText::new(t!("profile.lessons")).small());
            ui.label(RichText::new(t!("profile.quiz_accuracy")).small());
            ui.end_row();
            for subject in &profile.subjects {
                ui.label(&subject.name);
                ui.label(format!("{} / {}", subject.done, subject.topics));
                ui.label(subject.lessons.to_string());
                ui.label(
                    (subject.right * 100)
                        .checked_div(subject.answered)
                        .map_or_else(|| "–".to_owned(), |share| format!("{share}%")),
                );
                ui.end_row();
            }
        });
}
