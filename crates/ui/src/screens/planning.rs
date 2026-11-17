//! Planning, for teachers (Phase W8): the classes of a classroom on a
//! calendar, from a plan's topics. The calendar is computed by the engine
//! without AI; here the teacher sets the class days, sees the classes and
//! edits each one. No business rules: every change goes to the engine.

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};
use egui::RichText;
use kg_engine::{
    Activity, ActivityKind, CalendarLabels, ClassSchedule, ClassTime, Command, Engine,
    HolidayCalendar, PlanId, ReleaseRule, ScheduleSpec, Session, SessionKind, SessionMode,
    SessionStatus,
};

use super::Actions;
use crate::app::KnowGiverApp;
use crate::{i18n, theme, widgets};

const WEEKDAYS: [Weekday; 6] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
];

/// What the screen keeps between frames.
#[derive(Debug, Default)]
pub struct PlanningUi {
    plan: Option<PlanId>,
    classroom: i64,
    form: Option<Form>,
    /// The teacher asked to change the days of an existing calendar.
    changing_days: bool,
    /// The class whose details are open, with its edit buffers.
    editing: Option<Editing>,
    error: Option<String>,
}

/// The class days and dates, as typed.
#[derive(Debug, Clone)]
struct Form {
    start: String,
    end: String,
    days: [bool; 6],
    time: String,
    minutes: u32,
    review_every: u32,
    mock_exam: bool,
    blocked: String,
}

impl Form {
    fn new() -> Self {
        let today = Engine::today();
        Self {
            start: i18n::date(today),
            end: i18n::date(today + chrono::Duration::days(120)),
            days: [true, false, true, false, false, false],
            time: "19:00".into(),
            minutes: 100,
            review_every: kg_engine::DEFAULT_REVIEW_EVERY,
            mock_exam: true,
            blocked: String::new(),
        }
    }

    fn from_spec(spec: &ScheduleSpec) -> Self {
        let mut form = Self::new();
        form.start = i18n::date(spec.start);
        form.end = i18n::date(spec.end);
        form.days = WEEKDAYS.map(|day| spec.classes.iter().any(|c| c.weekday == day));
        if let Some(first) = spec.classes.first() {
            form.time = first.start.format("%H:%M").to_string();
            form.minutes = first.minutes;
        }
        form.review_every = spec.review_every;
        form.mock_exam = spec.mock_exam;
        form.blocked = spec
            .blocked
            .iter()
            .map(|day| i18n::date(*day))
            .collect::<Vec<_>>()
            .join(", ");
        form
    }

    /// The spec, or the message for the first field that does not read.
    fn spec(&self) -> Result<ScheduleSpec, String> {
        let date = |text: &str| {
            NaiveDate::parse_from_str(text.trim(), &t!("format.date"))
                .map_err(|_| t!("planning.bad_date", value = text.trim()).into_owned())
        };
        let start = date(&self.start)?;
        let end = date(&self.end)?;
        if end < start {
            return Err(t!("planning.end_before_start").into_owned());
        }
        let time = NaiveTime::parse_from_str(self.time.trim(), "%H:%M")
            .map_err(|_| t!("planning.bad_time", value = self.time.trim()).into_owned())?;
        let classes: Vec<ClassTime> = WEEKDAYS
            .iter()
            .zip(self.days)
            .filter(|(_, on)| *on)
            .map(|(day, _)| ClassTime {
                weekday: *day,
                start: time,
                minutes: self.minutes.max(10),
            })
            .collect();
        if classes.is_empty() {
            return Err(t!("planning.no_days").into_owned());
        }
        let blocked = self
            .blocked
            .split([',', ';', '\n'])
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(date)
            .collect::<Result<_, _>>()?;
        Ok(ScheduleSpec {
            start,
            end,
            classes,
            blocked,
            holidays: HolidayCalendar::Brazil,
            review_every: self.review_every,
            mock_exam: self.mock_exam,
        })
    }
}

#[derive(Debug, Clone)]
struct Editing {
    key: String,
    objectives: String,
    materials: String,
    note: String,
    private_note: String,
    link: String,
    activities: Vec<Activity>,
    homework: Vec<Activity>,
}

impl Editing {
    fn of(session: &Session) -> Self {
        Self {
            key: session.key.clone(),
            objectives: session.plan.objectives.join("\n"),
            materials: session.plan.materials.join("\n"),
            note: session.plan.note.clone(),
            private_note: session.plan.private_note.clone(),
            link: match &session.plan.mode {
                SessionMode::Online(link) => link.clone(),
                _ => String::new(),
            },
            activities: session.plan.activities.clone(),
            homework: session.plan.homework.clone(),
        }
    }
}

const ACTIVITY_KINDS: [ActivityKind; 9] = [
    ActivityKind::Explanation,
    ActivityKind::VoiceLesson,
    ActivityKind::HandoutReading,
    ActivityKind::Quiz,
    ActivityKind::SpokenReview,
    ActivityKind::WrittenAnswer,
    ActivityKind::MockExam,
    ActivityKind::Exercise,
    ActivityKind::Other,
];

fn activity_kind_name(kind: ActivityKind) -> String {
    match kind {
        ActivityKind::Explanation => t!("planning.activity_kind.explanation"),
        ActivityKind::VoiceLesson => t!("planning.activity_kind.voice_lesson"),
        ActivityKind::HandoutReading => t!("planning.activity_kind.handout_reading"),
        ActivityKind::Quiz => t!("planning.activity_kind.quiz"),
        ActivityKind::SpokenReview => t!("planning.activity_kind.spoken_review"),
        ActivityKind::WrittenAnswer => t!("planning.activity_kind.written_answer"),
        ActivityKind::MockExam => t!("planning.activity_kind.mock_exam"),
        ActivityKind::Exercise => t!("planning.activity_kind.exercise"),
        ActivityKind::Other => t!("planning.activity_kind.other"),
    }
    .into_owned()
}

/// A list of activities edited in place: kind, minutes, text; add, remove
/// and move up. Homework offers only what the student can do in the app.
fn activity_list(ui: &mut egui::Ui, id: &str, list: &mut Vec<Activity>, homework: bool) {
    let mut remove = None;
    let mut raise = None;
    let count = list.len();
    for (i, activity) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt((id, i))
                .width(150.0)
                .selected_text(activity_kind_name(activity.kind))
                .show_ui(ui, |ui| {
                    for kind in ACTIVITY_KINDS
                        .into_iter()
                        .filter(|k| !homework || k.works_as_homework())
                    {
                        ui.selectable_value(&mut activity.kind, kind, activity_kind_name(kind));
                    }
                });
            ui.add(
                egui::DragValue::new(&mut activity.minutes)
                    .range(1..=600)
                    .suffix(t!("planning.minutes_suffix")),
            );
            ui.add(egui::TextEdit::singleline(&mut activity.text).desired_width(300.0));
            if i > 0 && ui.small_button("↑").clicked() {
                raise = Some(i);
            }
            if ui.small_button(t!("planning.remove")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = raise {
        list.swap(i - 1, i);
    }
    if let Some(i) = remove.filter(|i| *i < count) {
        list.remove(i);
    }
    let label = if homework {
        t!("planning.add_homework")
    } else {
        t!("planning.add_activity")
    };
    if ui.small_button(label).clicked() {
        list.push(Activity {
            kind: if homework {
                ActivityKind::Quiz
            } else {
                ActivityKind::Explanation
            },
            minutes: 15,
            text: String::new(),
            topic: None,
        });
    }
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn show(app: &mut KnowGiverApp, ui: &mut egui::Ui) {
    let mut actions = Actions::default();
    let Ok(engine) = &app.engine else {
        return;
    };
    let state = &mut app.ui.planning;
    widgets::header(ui, &t!("planning.title"), &t!("planning.subtitle"));
    if !engine.role().creates_plans() {
        widgets::hint(ui, &t!("classes.teachers_only"));
        return;
    }
    let plans = engine.plans();
    if plans.is_empty() {
        widgets::hint(ui, &t!("planning.no_plans"));
        return;
    }
    let plan = state
        .plan
        .filter(|id| plans.iter().any(|p| p.id == *id))
        .or_else(|| engine.active_plan().map(|p| p.id))
        .unwrap_or(plans[0].id);
    state.plan = Some(plan);

    egui::ScrollArea::vertical().show(ui, |ui| {
        choose(ui, engine, state, plan);
        ui.add_space(12.0);
        let saved = engine.class_schedule(plan, state.classroom);
        if saved.is_none() && state.changing_days {
            state.changing_days = false;
        }
        match (&saved, state.changing_days) {
            (None, _) | (Some(_), true) => {
                let spec = saved.as_ref().map(|s| &s.schedule.spec);
                form_card(ui, state, spec, plan, saved.is_some(), &mut actions);
            }
            (Some(saved), false) => {
                toolbar(ui, engine, state, &saved.schedule, plan, &mut actions);
                ui.add_space(8.0);
                if let Some(version) = saved.published_version {
                    widgets::hint(ui, &t!("planning.published_version", version = version));
                }
                sessions(ui, engine, state, &saved.schedule, plan, &mut actions);
            }
        }
    });
    actions.run(app);
}

/// The plan and the classroom.
fn choose(ui: &mut egui::Ui, engine: &Engine, state: &mut PlanningUi, plan: PlanId) {
    ui.horizontal_wrapped(|ui| {
        ui.label(t!("planning.plan"));
        let title = engine
            .plans()
            .iter()
            .find(|p| p.id == plan)
            .map(|p| p.title.clone())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("planning_plan")
            .selected_text(title)
            .width(280.0)
            .show_ui(ui, |ui| {
                for p in engine.plans() {
                    if ui.selectable_label(p.id == plan, &p.title).clicked() {
                        state.plan = Some(p.id);
                        state.form = None;
                        state.changing_days = false;
                        state.editing = None;
                    }
                }
            });
        ui.add_space(12.0);
        ui.label(t!("planning.classroom"));
        let classrooms = engine.classrooms().unwrap_or_default();
        let name = classrooms
            .iter()
            .find(|c| c.id == state.classroom)
            .map_or_else(
                || t!("planning.no_classroom").into_owned(),
                |c| c.name.clone(),
            );
        egui::ComboBox::from_id_salt("planning_classroom")
            .selected_text(name)
            .width(220.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(state.classroom == 0, t!("planning.no_classroom"))
                    .clicked()
                {
                    state.classroom = 0;
                    state.form = None;
                    state.changing_days = false;
                }
                for classroom in classrooms {
                    if ui
                        .selectable_label(state.classroom == classroom.id, &classroom.name)
                        .clicked()
                    {
                        state.classroom = classroom.id;
                        state.form = None;
                        state.changing_days = false;
                    }
                }
            });
    });
}

/// Class days and dates; generates (or regenerates) the calendar.
fn form_card(
    ui: &mut egui::Ui,
    state: &mut PlanningUi,
    spec: Option<&ScheduleSpec>,
    plan: PlanId,
    has_schedule: bool,
    actions: &mut Actions,
) {
    let mut form = state
        .form
        .take()
        .unwrap_or_else(|| spec.map_or_else(Form::new, Form::from_spec));
    let mut keep = true;
    widgets::card(ui, |ui| {
        ui.label(RichText::new(t!("planning.form_title")).strong());
        widgets::hint(ui, &t!("planning.form_hint"));
        ui.add_space(8.0);
        egui::Grid::new("planning_form")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(t!("planning.start"));
                ui.add(egui::TextEdit::singleline(&mut form.start).desired_width(120.0));
                ui.end_row();
                ui.label(t!("planning.end"));
                ui.add(egui::TextEdit::singleline(&mut form.end).desired_width(120.0));
                ui.end_row();
                ui.label(t!("planning.days"));
                ui.horizontal_wrapped(|ui| {
                    for (on, day) in form.days.iter_mut().zip(WEEKDAYS) {
                        ui.checkbox(on, weekday_name(day));
                    }
                });
                ui.end_row();
                ui.label(t!("planning.time"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut form.time).desired_width(60.0));
                    ui.add(
                        egui::DragValue::new(&mut form.minutes)
                            .range(10..=600)
                            .suffix(t!("planning.minutes_suffix")),
                    );
                });
                ui.end_row();
                ui.label(t!("planning.review_every"));
                ui.add(egui::DragValue::new(&mut form.review_every).range(0..=30))
                    .on_hover_text(t!("planning.review_every_hint"));
                ui.end_row();
                ui.label(t!("planning.mock_exam"));
                ui.checkbox(&mut form.mock_exam, t!("planning.mock_exam_hint"));
                ui.end_row();
                ui.label(t!("planning.blocked"));
                ui.add(
                    egui::TextEdit::multiline(&mut form.blocked)
                        .desired_rows(2)
                        .desired_width(320.0)
                        .hint_text(t!("planning.blocked_hint")),
                );
                ui.end_row();
            });
        widgets::hint(ui, &t!("planning.holidays_note"));
        if let Some(error) = &state.error {
            ui.colored_label(theme::palette(ui.ctx()).danger, error);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let label = if has_schedule {
                t!("planning.regenerate")
            } else {
                t!("planning.generate")
            };
            if widgets::primary_button(ui, &label).clicked() {
                match form.spec() {
                    Ok(spec) => {
                        actions.send(Command::GenerateSchedule {
                            plan,
                            classroom: state.classroom,
                            spec: Box::new(spec),
                        });
                        state.error = None;
                        state.editing = None;
                        state.changing_days = false;
                        keep = false;
                    }
                    Err(message) => state.error = Some(message),
                }
            }
            if has_schedule && ui.button(t!("planning.cancel")).clicked() {
                state.error = None;
                state.changing_days = false;
                keep = false;
            }
        });
    });
    if keep {
        state.form = Some(form);
    }
}

fn toolbar(
    ui: &mut egui::Ui,
    engine: &Engine,
    state: &mut PlanningUi,
    schedule: &ClassSchedule,
    plan: PlanId,
    actions: &mut Actions,
) {
    let p = theme::palette(ui.ctx());
    let classroom = state.classroom;
    let classes = schedule
        .sessions
        .iter()
        .filter(|s| s.status != SessionStatus::Cancelled)
        .count();
    widgets::card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t!(
                    "planning.summary",
                    count = classes,
                    start = i18n::date(schedule.spec.start),
                    end = i18n::date(schedule.spec.end)
                ))
                .strong(),
            );
        });
        if schedule.shortfall_minutes > 0 {
            widgets::callout(ui, p.highlight, |ui| {
                ui.label(t!(
                    "planning.shortfall",
                    hours = schedule.shortfall_minutes.div_ceil(60),
                    topics = schedule.unplaced.len()
                ));
            });
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let empty = schedule
                .sessions
                .iter()
                .filter(|s| {
                    s.status == SessionStatus::Planned
                        && s.kind != SessionKind::MockExam
                        && s.plan.is_empty()
                })
                .count();
            if engine.suggesting_sessions() == Some((plan, classroom)) {
                ui.spinner();
                ui.label(t!("planning.suggesting"));
            } else if empty > 0
                && widgets::primary_button(ui, &t!("planning.suggest", count = empty))
                    .on_hover_text(t!("planning.suggest_hint"))
                    .clicked()
            {
                actions.send(Command::SuggestSessionPlans { plan, classroom });
            }
            if ui
                .button(t!("planning.replan"))
                .on_hover_text(t!("planning.replan_hint"))
                .clicked()
            {
                actions.send(Command::ReplanSchedule { plan, classroom });
                state.editing = None;
            }
            if classroom != 0 {
                if engine.publishing_schedule() == Some((plan, classroom)) {
                    ui.spinner();
                } else if ui
                    .button(t!("planning.publish"))
                    .on_hover_text(t!("planning.publish_hint"))
                    .clicked()
                {
                    actions.send(Command::PublishSchedule { plan, classroom });
                }
            }
            if ui.button(t!("planning.undo")).clicked() {
                actions.send(Command::UndoSchedule { plan, classroom });
                state.editing = None;
            }
            if ui.button(t!("planning.change_days")).clicked() {
                state.form = Some(Form::from_spec(&schedule.spec));
                state.changing_days = true;
            }
            if ui
                .button(t!("planning.export"))
                .on_hover_text(t!("planning.export_hint"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title(t!("planning.export"))
                    .set_file_name(format!("{}.ics", t!("planning.file_name")))
                    .add_filter(t!("planning.ics_filter"), &["ics"])
                    .save_file()
            {
                actions.send(Command::ExportScheduleCalendar {
                    plan,
                    classroom,
                    path,
                    labels: Box::new(CalendarLabels {
                        class: t!("planning.kind.class").into_owned(),
                        review: t!("planning.kind.review").into_owned(),
                        mock_exam: t!("planning.kind.mock_exam").into_owned(),
                        objectives: t!("planning.objectives").into_owned(),
                        homework: t!("planning.homework").into_owned(),
                    }),
                });
            }
            let mut gradual = schedule.release == ReleaseRule::AsClassesHappen;
            if ui
                .checkbox(&mut gradual, t!("planning.release_gradual"))
                .on_hover_text(t!("planning.release_hint"))
                .changed()
            {
                let mut changed = schedule.clone();
                changed.release = if gradual {
                    ReleaseRule::AsClassesHappen
                } else {
                    ReleaseRule::AllOpen
                };
                actions.send(Command::SaveSchedule {
                    plan,
                    classroom,
                    schedule: Box::new(changed),
                });
            }
            if ui.button(t!("planning.delete")).clicked() {
                actions.send(Command::DeleteSchedule { plan, classroom });
                state.editing = None;
            }
        });
    });
}

fn sessions(
    ui: &mut egui::Ui,
    engine: &Engine,
    state: &mut PlanningUi,
    schedule: &ClassSchedule,
    plan: PlanId,
    actions: &mut Actions,
) {
    let p = theme::palette(ui.ctx());
    let titles = engine.schedule_topic_titles(plan);
    let today = Engine::today();
    let classroom = state.classroom;
    for (index, session) in schedule.sessions.iter().enumerate() {
        let open = state.editing.as_ref().is_some_and(|e| e.key == session.key);
        widgets::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let when = format!(
                    "{} {} · {} · {}",
                    weekday_name(session.date.weekday()),
                    i18n::date(session.date),
                    session.start.format("%H:%M"),
                    t!("planning.minutes", count = session.minutes)
                );
                let mut text = RichText::new(when).strong();
                if session.status == SessionStatus::Cancelled {
                    text = text.strikethrough().color(p.muted);
                }
                ui.label(text);
                widgets::chip(ui, &kind_name(session.kind), p.accent);
                match session.status {
                    SessionStatus::Given => widgets::chip(ui, &t!("planning.given"), p.success),
                    SessionStatus::Cancelled => {
                        widgets::chip(ui, &t!("planning.cancelled"), p.muted);
                    }
                    SessionStatus::Planned if session.date < today => {
                        widgets::chip(ui, &t!("planning.past"), p.highlight);
                    }
                    SessionStatus::Planned => {}
                }
                if session.locked {
                    widgets::chip(ui, &t!("planning.locked"), p.muted);
                }
            });
            let topics: Vec<&str> = session
                .topics
                .iter()
                .map(|key| titles.get(key).map_or(key.as_str(), String::as_str))
                .collect();
            if !topics.is_empty() {
                ui.label(topics.join(" · "));
            }
            if !session.plan.objectives.is_empty() && !open {
                widgets::hint(ui, &session.plan.objectives.join(" · "));
            }
            ui.horizontal_wrapped(|ui| {
                let mut edit = |change: &dyn Fn(&mut Session)| {
                    let mut changed = schedule.clone();
                    change(&mut changed.sessions[index]);
                    actions.send(Command::SaveSchedule {
                        plan,
                        classroom,
                        schedule: Box::new(changed),
                    });
                };
                if session.status != SessionStatus::Given
                    && ui.small_button(t!("planning.mark_given")).clicked()
                {
                    edit(&|s| s.status = SessionStatus::Given);
                }
                if session.status == SessionStatus::Planned
                    && ui.small_button(t!("planning.cancel_class")).clicked()
                {
                    edit(&|s| s.status = SessionStatus::Cancelled);
                }
                if session.status != SessionStatus::Planned
                    && ui.small_button(t!("planning.reopen")).clicked()
                {
                    edit(&|s| s.status = SessionStatus::Planned);
                }
                let lock = if session.locked {
                    t!("planning.unlock")
                } else {
                    t!("planning.lock")
                };
                if ui.small_button(lock).clicked() {
                    edit(&|s| s.locked = !s.locked);
                }
                let details = if open {
                    t!("planning.close_details")
                } else {
                    t!("planning.details")
                };
                if ui.small_button(details).clicked() {
                    state.editing = (!open).then(|| Editing::of(session));
                }
            });
            if open && let Some(editing) = state.editing.as_mut() {
                details(ui, editing, schedule, index, plan, classroom, actions);
            }
        });
        ui.add_space(6.0);
    }
}

/// The class's plan, edited in place.
fn details(
    ui: &mut egui::Ui,
    editing: &mut Editing,
    schedule: &ClassSchedule,
    index: usize,
    plan: PlanId,
    classroom: i64,
    actions: &mut Actions,
) {
    ui.add_space(6.0);
    egui::Grid::new(("planning_details", index))
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label(t!("planning.objectives"));
            ui.add(
                egui::TextEdit::multiline(&mut editing.objectives)
                    .desired_rows(3)
                    .desired_width(420.0)
                    .hint_text(t!("planning.one_per_line")),
            );
            ui.end_row();
            ui.label(t!("planning.materials"));
            ui.add(
                egui::TextEdit::multiline(&mut editing.materials)
                    .desired_rows(2)
                    .desired_width(420.0)
                    .hint_text(t!("planning.one_per_line")),
            );
            ui.end_row();
            ui.label(t!("planning.online_link"));
            ui.add(
                egui::TextEdit::singleline(&mut editing.link)
                    .desired_width(420.0)
                    .hint_text("https://"),
            );
            ui.end_row();
            ui.label(t!("planning.note"));
            ui.add(
                egui::TextEdit::multiline(&mut editing.note)
                    .desired_rows(2)
                    .desired_width(420.0)
                    .hint_text(t!("planning.note_hint")),
            );
            ui.end_row();
            ui.label(t!("planning.private_note"));
            ui.add(
                egui::TextEdit::multiline(&mut editing.private_note)
                    .desired_rows(2)
                    .desired_width(420.0)
                    .hint_text(t!("planning.private_note_hint")),
            );
            ui.end_row();
        });
    let session = &schedule.sessions[index];
    ui.add_space(6.0);
    ui.label(RichText::new(t!("planning.activities")).strong());
    activity_list(ui, "planning_activity", &mut editing.activities, false);
    let used: u32 = editing.activities.iter().map(|a| a.minutes).sum();
    if used > session.minutes {
        widgets::hint(
            ui,
            &t!(
                "planning.minutes_over",
                used = used,
                length = session.minutes
            ),
        );
    }
    ui.add_space(6.0);
    ui.label(RichText::new(t!("planning.homework")).strong());
    activity_list(ui, "planning_homework", &mut editing.homework, true);
    let link = editing.link.trim();
    let link_ok = link.is_empty() || link.starts_with("https://");
    if !link_ok {
        widgets::hint(ui, &t!("planning.link_https"));
    }
    if ui
        .add_enabled(link_ok, egui::Button::new(t!("planning.save")))
        .clicked()
    {
        let mut changed = schedule.clone();
        let target = &mut changed.sessions[index].plan;
        target.objectives = lines(&editing.objectives);
        target.materials = lines(&editing.materials);
        target.note = editing.note.trim().to_owned();
        target.private_note = editing.private_note.trim().to_owned();
        let filled = |list: &[Activity]| {
            list.iter()
                .filter(|a| !a.text.trim().is_empty())
                .map(|a| Activity {
                    text: a.text.trim().to_owned(),
                    ..a.clone()
                })
                .collect::<Vec<_>>()
        };
        target.activities = filled(&editing.activities);
        target.homework = filled(&editing.homework);
        target.mode = if link.is_empty() {
            match target.mode {
                SessionMode::Online(_) => SessionMode::InPerson,
                ref other => other.clone(),
            }
        } else {
            SessionMode::Online(link.to_owned())
        };
        actions.send(Command::SaveSchedule {
            plan,
            classroom,
            schedule: Box::new(changed),
        });
    }
}

fn weekday_name(day: Weekday) -> String {
    match day {
        Weekday::Mon => t!("planning.weekday.mon"),
        Weekday::Tue => t!("planning.weekday.tue"),
        Weekday::Wed => t!("planning.weekday.wed"),
        Weekday::Thu => t!("planning.weekday.thu"),
        Weekday::Fri => t!("planning.weekday.fri"),
        Weekday::Sat => t!("planning.weekday.sat"),
        Weekday::Sun => t!("planning.weekday.sun"),
    }
    .into_owned()
}

fn kind_name(kind: SessionKind) -> String {
    match kind {
        SessionKind::Class => t!("planning.kind.class"),
        SessionKind::Review => t!("planning.kind.review"),
        SessionKind::MockExam => t!("planning.kind.mock_exam"),
    }
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_reads_dates_days_and_blocked_days() {
        crate::i18n::set("pt-BR");
        let mut form = Form::new();
        form.start = "02/03/2026".into();
        form.end = "30/06/2026".into();
        form.days = [true, false, true, false, false, false];
        form.blocked = "06/04/2026, 20/04/2026".into();
        let spec = form.spec().expect("spec");
        assert_eq!(spec.classes.len(), 2);
        assert_eq!(spec.classes[1].weekday, Weekday::Wed);
        assert_eq!(spec.blocked.len(), 2);
        assert_eq!(Form::from_spec(&spec).blocked, "06/04/2026, 20/04/2026");

        form.end = "01/01/2026".into();
        assert!(form.spec().is_err());
        form.end = "30/06/2026".into();
        form.days = [false; 6];
        assert!(form.spec().is_err());
        form.days[0] = true;
        form.time = "7pm".into();
        assert!(form.spec().is_err());
    }
}
