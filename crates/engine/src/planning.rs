//! Lesson planning (Phase W8): a teacher's class schedule per plan and
//! classroom. Everything here works without AI and without the panel: the
//! calendar, the topics' distribution, editing, replanning, undo and the
//! calendar file. The AI's suggestions for each class arrive later through
//! the panel, when it is reachable.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kg_study::schedule::{
    ClassSchedule, ScheduleSpec, SessionKind, SessionStatus, TopicLoad, plan as plan_schedule,
    replan,
};
use kg_study::{PlanId, SavedSchedule};

use crate::{Engine, EngineError};

/// Words the calendar file needs, in the app's language (the UI passes them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarLabels {
    pub class: String,
    pub review: String,
    pub mock_exam: String,
    pub objectives: String,
    pub homework: String,
}

impl Engine {
    /// The saved schedule of `plan` for `classroom` (0: not tied to one).
    pub fn class_schedule(&self, plan: PlanId, classroom: i64) -> Option<SavedSchedule> {
        self.store
            .class_schedule(plan, classroom)
            .map_err(|err| tracing::warn!(%err, "could not read the class schedule"))
            .ok()
            .flatten()
    }

    /// Every schedule of a plan.
    pub fn class_schedules(&self, plan: PlanId) -> Vec<SavedSchedule> {
        self.store
            .class_schedules(plan)
            .map_err(|err| tracing::warn!(%err, "could not read the class schedules"))
            .unwrap_or_default()
    }

    /// Topic key → title, for showing a schedule.
    pub fn schedule_topic_titles(&self, plan: PlanId) -> HashMap<String, String> {
        let Ok(study) = self.store.load_plan(plan) else {
            return HashMap::new();
        };
        let titles: HashMap<_, _> = study
            .topics()
            .map(|(_, topic)| (topic.id, topic.title.clone()))
            .collect();
        self.store
            .topic_keys(plan)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, key)| titles.get(&id).map(|title| (key, title.clone())))
            .collect()
    }

    /// The plan's topics in order, with the class minutes each needs.
    fn topic_loads(&self, plan: PlanId) -> Result<Vec<TopicLoad>, EngineError> {
        let study = self.store.load_plan(plan)?;
        let difficulty: HashMap<_, _> = study
            .topics()
            .map(|(_, topic)| (topic.id, topic.difficulty))
            .collect();
        Ok(self
            .store
            .topic_keys(plan)?
            .into_iter()
            .filter_map(|(id, key)| {
                difficulty
                    .get(&id)
                    .map(|&level| TopicLoad::estimate(key, level, 100))
            })
            .collect())
    }

    pub(crate) fn generate_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
        spec: ScheduleSpec,
    ) -> Result<(), EngineError> {
        let topics = self.topic_loads(plan)?;
        let planned = plan_schedule(&spec, &topics);
        // A new calendar keeps the release rule the teacher chose before.
        let release = self
            .class_schedule(plan, classroom)
            .map(|saved| saved.schedule.release)
            .unwrap_or_default();
        let mut schedule = ClassSchedule::new(spec, planned);
        schedule.release = release;
        self.save_schedule(plan, classroom, &schedule)
    }

    /// Saves the teacher's edits as they are.
    pub(crate) fn save_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
        schedule: &ClassSchedule,
    ) -> Result<(), EngineError> {
        let now = chrono::Local::now().naive_local();
        self.store
            .save_class_schedule(plan, classroom, schedule, now)?;
        Ok(())
    }

    /// Plans again from today, keeping given and locked classes.
    pub(crate) fn replan_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
    ) -> Result<(), EngineError> {
        let Some(saved) = self.class_schedule(plan, classroom) else {
            return Ok(());
        };
        let topics = self.topic_loads(plan)?;
        let current = saved.schedule;
        let planned = replan(&current.spec, &topics, &current.sessions, Self::today());
        let schedule = ClassSchedule {
            sessions: planned.sessions,
            shortfall_minutes: planned.shortfall_minutes,
            unplaced: planned.unplaced,
            ..current
        };
        self.save_schedule(plan, classroom, &schedule)
    }

    pub(crate) fn undo_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
    ) -> Result<(), EngineError> {
        let now = chrono::Local::now().naive_local();
        self.store.undo_class_schedule(plan, classroom, now)?;
        Ok(())
    }

    pub(crate) fn delete_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
    ) -> Result<(), EngineError> {
        self.store.delete_class_schedule(plan, classroom)?;
        Ok(())
    }

    /// Writes the classes as an iCalendar file for any calendar app.
    pub(crate) fn export_schedule_calendar(
        &mut self,
        plan: PlanId,
        classroom: i64,
        path: PathBuf,
        labels: &CalendarLabels,
    ) -> Result<(), EngineError> {
        let Some(saved) = self.class_schedule(plan, classroom) else {
            return Ok(());
        };
        let titles = self.schedule_topic_titles(plan);
        let text = calendar_file(&saved.schedule, &titles, labels, plan, classroom);
        write_file(&path, &text)
    }
}

fn write_file(path: &Path, text: &str) -> Result<(), EngineError> {
    std::fs::write(path, text).map_err(|source| EngineError::Export {
        path: path.to_path_buf(),
        source,
    })
}

/// iCalendar (RFC 5545) text: one event per class not cancelled, in the
/// classroom's time zone, with the topics, objectives and homework.
pub(crate) fn calendar_file(
    schedule: &ClassSchedule,
    titles: &HashMap<String, String>,
    labels: &CalendarLabels,
    plan: PlanId,
    classroom: i64,
) -> String {
    let mut out = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n");
    let _ = write!(
        out,
        "PRODID:-//{}//{}//PT\r\nCALSCALE:GREGORIAN\r\n",
        escape(kg_brand::COMPANY),
        escape(kg_brand::NAME)
    );
    for session in schedule
        .sessions
        .iter()
        .filter(|s| s.status != SessionStatus::Cancelled)
    {
        let topics: Vec<&str> = session
            .topics
            .iter()
            .map(|key| titles.get(key).map_or(key.as_str(), String::as_str))
            .collect();
        let kind = match session.kind {
            SessionKind::Class => &labels.class,
            SessionKind::Review => &labels.review,
            SessionKind::MockExam => &labels.mock_exam,
        };
        let start = session.date.and_time(session.start);
        let end = start + chrono::Duration::minutes(i64::from(session.minutes));
        let mut description = topics.join(", ");
        if !session.plan.objectives.is_empty() {
            let _ = write!(
                description,
                "\n\n{}:\n- {}",
                labels.objectives,
                session.plan.objectives.join("\n- ")
            );
        }
        if !session.plan.homework.is_empty() {
            let homework: Vec<&str> = session
                .plan
                .homework
                .iter()
                .map(|a| a.text.as_str())
                .collect();
            let _ = write!(
                description,
                "\n\n{}:\n- {}",
                labels.homework,
                homework.join("\n- ")
            );
        }
        if !session.plan.note.is_empty() {
            let _ = write!(description, "\n\n{}", session.plan.note);
        }
        let summary = if topics.is_empty() {
            kind.clone()
        } else {
            format!("{kind}: {}", topics.join(", "))
        };
        let _ = write!(
            out,
            "BEGIN:VEVENT\r\nUID:{}-{}-{}@{}\r\nDTSTAMP:{}\r\nDTSTART;TZID={tz}:{}\r\nDTEND;TZID={tz}:{}\r\nSUMMARY:{}\r\nDESCRIPTION:{}\r\n",
            plan.0,
            classroom,
            session.key,
            kg_brand::SLUG,
            chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
            start.format("%Y%m%dT%H%M%S"),
            end.format("%Y%m%dT%H%M%S"),
            escape(&summary),
            escape(&description),
            tz = schedule.time_zone,
        );
        if let kg_study::schedule::SessionMode::Online(link) = &session.plan.mode {
            let _ = write!(out, "URL:{}\r\n", escape(link));
        }
        out.push_str("END:VEVENT\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

/// RFC 5545 text escaping.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, Weekday};

    use super::*;
    use kg_study::schedule::{ClassTime, HolidayCalendar};

    #[test]
    fn the_calendar_file_lists_classes_with_topics_and_skips_cancelled_ones() {
        let spec = ScheduleSpec {
            start: NaiveDate::from_ymd_opt(2026, 3, 2).expect("date"),
            end: NaiveDate::from_ymd_opt(2026, 3, 9).expect("date"),
            classes: vec![ClassTime {
                weekday: Weekday::Mon,
                start: NaiveTime::from_hms_opt(19, 0, 0).expect("time"),
                minutes: 100,
            }],
            blocked: Default::default(),
            holidays: HolidayCalendar::Brazil,
            review_every: 0,
            mock_exam: false,
        };
        let topics = [
            TopicLoad {
                key: "t1".into(),
                minutes: 100,
            },
            TopicLoad {
                key: "t2".into(),
                minutes: 100,
            },
        ];
        let mut schedule = ClassSchedule::new(spec.clone(), plan_schedule(&spec, &topics));
        schedule.sessions[0].plan.objectives = vec!["Resolver; somar, dividir".into()];
        schedule.sessions[1].status = SessionStatus::Cancelled;
        let titles = HashMap::from([("t1".to_owned(), "Funções".to_owned())]);
        let labels = CalendarLabels {
            class: "Aula".into(),
            review: "Revisão".into(),
            mock_exam: "Simulado".into(),
            objectives: "Objetivos".into(),
            homework: "Tarefa".into(),
        };
        let text = calendar_file(&schedule, &titles, &labels, PlanId(3), 7);
        assert!(text.starts_with("BEGIN:VCALENDAR\r\n"));
        assert_eq!(text.matches("BEGIN:VEVENT").count(), 1);
        assert!(text.contains("DTSTART;TZID=America/Sao_Paulo:20260302T190000"));
        assert!(text.contains("DTEND;TZID=America/Sao_Paulo:20260302T204000"));
        assert!(text.contains("SUMMARY:Aula: Funções"));
        assert!(text.contains("Resolver\\; somar\\, dividir"));
        assert!(text.contains("UID:3-7-s1@"));
        assert!(text.ends_with("END:VCALENDAR\r\n"));
    }
}
