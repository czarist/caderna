//! Lesson planning (Phase W8): a teacher's class schedule per plan and
//! classroom. Everything here works without AI and without the panel: the
//! calendar, the topics' distribution, editing, replanning, undo and the
//! calendar file. The AI's suggestions for each class arrive later through
//! the panel, when it is reachable.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kg_ai::{PlanningInput, SessionBrief, SessionDraft};
use kg_study::schedule::{
    Activity, ActivityKind, ClassSchedule, ScheduleSpec, Session, SessionKind, SessionMode,
    SessionPlan, SessionStatus, TopicLoad, plan as plan_schedule, replan,
};
use kg_study::{PlanId, SavedSchedule};

use crate::{Engine, EngineError, Event};

/// Classes asked of the model at once.
const BATCH: usize = 6;

/// A suggestion job for one schedule, while it runs.
#[derive(Debug, Default)]
pub(crate) struct PlanningState {
    pub(crate) suggesting: Option<(PlanId, i64)>,
    pub(crate) publishing: Option<(PlanId, i64)>,
    /// What moved in schedules the teacher republished, for a quiet note
    /// on the student's Today.
    pub(crate) changes: Vec<ScheduleNote>,
}

/// A republished schedule's changes, as the student's Today shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleNote {
    pub plan_title: String,
    pub changes: Vec<kg_study::schedule::ScheduleChange>,
}

/// A schedule as the panel sends it to a student.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct ScheduleDownload {
    pub(crate) course_id: i64,
    pub(crate) classroom_id: i64,
    pub(crate) version: i64,
    pub(crate) schedule: ClassSchedule,
}

/// The next class of one of the student's schedules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgendaItem {
    pub plan: PlanId,
    pub plan_title: String,
    pub session: Session,
    /// The session's topics, by title.
    pub topics: Vec<String>,
}

/// A reminder about a class of the student's schedule (Phase W8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassReminder {
    /// Shown once per id: plan, session and kind.
    pub id: String,
    pub kind: ClassReminderKind,
    pub plan_title: String,
    pub date: chrono::NaiveDate,
    pub start: chrono::NaiveTime,
    /// The online class's link.
    pub link: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassReminderKind {
    /// Tomorrow's class, with the previous class's homework still open.
    HomeworkOpen,
    /// An online class within the hour.
    OnlineSoon,
}

#[derive(Debug)]
pub(crate) enum PlanningEvent {
    Suggested {
        plan: PlanId,
        classroom: i64,
        /// Checked plans by session key.
        plans: Vec<(String, SessionPlan)>,
        /// Classes left empty: the model's answer did not pass the checks twice.
        failed: usize,
        error: Option<kg_ai::AiError>,
    },
    Published {
        plan: PlanId,
        classroom: i64,
        result: Result<i64, crate::server::PanelError>,
    },
    PdfExported(Result<PathBuf, EngineError>),
}

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
    /// The schedule whose classes the AI is planning now, if any.
    pub fn suggesting_sessions(&self) -> Option<(PlanId, i64)> {
        self.planning.suggesting
    }

    /// Asks the AI (through the panel on brands with one) for the plan of
    /// every class still empty. Classes already planned stay as they are.
    pub(crate) fn suggest_session_plans(
        &mut self,
        plan: PlanId,
        classroom: i64,
    ) -> Result<(), EngineError> {
        if self.planning.suggesting.is_some() {
            return Ok(());
        }
        // The AI comes from the panel: without its token there is none.
        if kg_brand::has_server() && self.panel_token().is_none() {
            return Err(EngineError::PanelOffline);
        }
        let Some(saved) = self.class_schedule(plan, classroom) else {
            return Ok(());
        };
        let study = self.store.load_plan(plan)?;
        let titles = self.schedule_topic_titles(plan);
        let ids = self.store.topic_keys(plan)?;
        let keys: Vec<String> = ids.iter().map(|(_, key)| key.clone()).collect();
        let number: HashMap<&str, usize> = keys
            .iter()
            .enumerate()
            .map(|(i, key)| (key.as_str(), i + 1))
            .collect();
        let language = self.prompt_language();
        let mut materials = Vec::new();
        for (id, key) in &ids {
            let title = titles.get(key).cloned().unwrap_or_else(|| key.clone());
            if self.store.handout(*id).ok().flatten().is_some() {
                materials.push(format!(
                    "{}{title}",
                    language.pick("Apostila: ", "Handout: ")
                ));
            }
            if self.store.quiz_bank(*id).ok().flatten().is_some() {
                materials.push(format!(
                    "{}{title}",
                    language.pick("Questões: ", "Questions: ")
                ));
            }
        }
        let briefs: Vec<(Session, SessionBrief)> = saved
            .schedule
            .sessions
            .iter()
            .filter(|s| {
                s.status == SessionStatus::Planned
                    && s.kind != SessionKind::MockExam
                    && s.plan.is_empty()
            })
            .enumerate()
            .map(|(i, session)| {
                let brief = SessionBrief {
                    number: i + 1,
                    kind: kind_id(session.kind).to_owned(),
                    minutes: session.minutes,
                    topics: session
                        .topics
                        .iter()
                        .filter_map(|t| number.get(t.as_str()).copied())
                        .collect(),
                };
                (session.clone(), brief)
            })
            .collect();
        if briefs.is_empty() {
            return Ok(());
        }
        let topics: Vec<String> = keys
            .iter()
            .map(|k| titles.get(k).cloned().unwrap_or_else(|| k.clone()))
            .collect();
        let client = self.ai_client();
        let exam = study.exam.clone();
        self.planning.suggesting = Some((plan, classroom));
        let spawned = self.spawn(move || {
            let (plans, failed, error) = suggest_in_batches(
                &briefs,
                |sessions| {
                    kg_ai::suggest_session_plans(
                        &client,
                        &PlanningInput {
                            exam: exam.clone(),
                            topics: topics.clone(),
                            sessions,
                            materials: materials.clone(),
                        },
                    )
                },
                &keys,
                &materials,
            );
            Event::Planning(PlanningEvent::Suggested {
                plan,
                classroom,
                plans,
                failed,
                error,
            })
        });
        if !spawned {
            self.planning.suggesting = None;
        }
        Ok(())
    }

    /// The schedule being sent to the panel, if any.
    pub fn publishing_schedule(&self) -> Option<(PlanId, i64)> {
        self.planning.publishing
    }

    /// Sends a classroom's schedule to the panel, for the school and the
    /// students. The plan must be published as a course first; the
    /// teacher's private notes never leave this computer.
    pub(crate) fn publish_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
    ) -> Result<(), EngineError> {
        if self.role() != crate::ClassRole::Teacher {
            return Err(EngineError::TeachersOnly);
        }
        if classroom == 0 {
            return Err(EngineError::ScheduleNeedsClassroom);
        }
        if self.planning.publishing.is_some() {
            return Ok(());
        }
        let course = self
            .course_link(plan)
            .filter(|link| !link.assigned)
            .map(|link| link.course_id)
            .ok_or(EngineError::ScheduleNeedsCourse)?;
        let token = self.panel_token().ok_or(EngineError::PanelOffline)?;
        let Some(saved) = self.class_schedule(plan, classroom) else {
            return Ok(());
        };
        let body = serde_json::json!({ "schedule": saved.schedule.for_students() });
        self.planning.publishing = Some((plan, classroom));
        self.spawn(move || {
            #[derive(serde::Deserialize)]
            struct Version {
                version: i64,
            }
            let path = format!("courses/{course}/schedules/{classroom}");
            let result = crate::server::HttpPanel::for_content()
                .call("PUT", &path, &token, Some(&body))
                .and_then(|(status, text)| crate::classes::data::<Version>(status, &text))
                .map(|v| v.version);
            Event::Planning(PlanningEvent::Published {
                plan,
                classroom,
                result,
            })
        });
        Ok(())
    }

    /// Keeps the schedules a student's sync brought, for the plans of their
    /// courses, and drops those no longer sent.
    pub(crate) fn apply_schedule_download(
        &mut self,
        schedules: &[ScheduleDownload],
    ) -> Result<(), EngineError> {
        let links = self.store.course_links()?;
        let plan_of = |course: i64| {
            links
                .iter()
                .find(|(_, link)| link.assigned && link.course_id == course)
                .map(|(plan, _)| *plan)
        };
        let now = chrono::Local::now().naive_local();
        let mut kept = std::collections::HashSet::new();
        for download in schedules {
            let Some(plan) = plan_of(download.course_id) else {
                continue;
            };
            kept.insert((plan, download.classroom_id));
            let current = self.class_schedule(plan, download.classroom_id);
            if current.as_ref().and_then(|s| s.published_version) == Some(download.version) {
                continue;
            }
            if let Some(current) = &current {
                let changes = download
                    .schedule
                    .changes_since(&current.schedule, now.date());
                if !changes.is_empty() {
                    let plan_title = self
                        .plans()
                        .iter()
                        .find(|p| p.id == plan)
                        .map(|p| p.title.clone())
                        .unwrap_or_default();
                    self.planning.changes.push(ScheduleNote {
                        plan_title,
                        changes,
                    });
                }
            }
            self.store
                .save_class_schedule(plan, download.classroom_id, &download.schedule, now)?;
            self.store.set_class_schedule_published(
                plan,
                download.classroom_id,
                download.version,
            )?;
        }
        for (plan, link) in &links {
            if !link.assigned {
                continue;
            }
            for saved in self.store.class_schedules(*plan)? {
                if !kept.contains(&(*plan, saved.classroom)) {
                    self.store.delete_class_schedule(*plan, saved.classroom)?;
                }
            }
        }
        Ok(())
    }

    /// Changes in republished schedules since the app started, oldest first.
    pub fn schedule_changes(&self) -> &[ScheduleNote] {
        &self.planning.changes
    }

    /// The student's next classes, one per schedule, soonest first.
    pub fn agenda(&self, today: chrono::NaiveDate) -> Vec<AgendaItem> {
        let mut items = Vec::new();
        for plan in self.plans() {
            if !self.course_link(plan.id).is_some_and(|link| link.assigned) {
                continue;
            }
            let titles = self.schedule_topic_titles(plan.id);
            for saved in self.class_schedules(plan.id) {
                if let Some(session) = saved.schedule.next_session(today) {
                    items.push(AgendaItem {
                        plan: plan.id,
                        plan_title: plan.title.clone(),
                        topics: session
                            .topics
                            .iter()
                            .map(|k| titles.get(k).cloned().unwrap_or_else(|| k.clone()))
                            .collect(),
                        session: session.clone(),
                    });
                }
            }
        }
        items.sort_by_key(|item| (item.session.date, item.session.start));
        items
    }

    /// Class reminders due at `now` for a student, as the settings allow:
    /// the day before a class while the last class's homework is still
    /// open, and in the hour before an online class.
    pub fn class_reminders(&self, now: chrono::NaiveDateTime) -> Vec<ClassReminder> {
        let settings = self.settings();
        if self.role() != crate::ClassRole::Student
            || !(settings.homework_reminders || settings.online_class_reminders)
        {
            return Vec::new();
        }
        let today = now.date();
        let mut reminders = Vec::new();
        for plan in self.plans() {
            if !self.course_link(plan.id).is_some_and(|link| link.assigned) {
                continue;
            }
            let progress = self.store.plan_progress(plan.id).ok();
            for saved in self.class_schedules(plan.id) {
                let schedule = &saved.schedule;
                let work = progress
                    .as_ref()
                    .map(|p| p.schedule_work(schedule, today))
                    .unwrap_or_default();
                let planned = schedule
                    .sessions
                    .iter()
                    .filter(|s| s.status == SessionStatus::Planned);
                for session in planned {
                    let id = |kind: &str| {
                        format!("{}:{}:{}:{kind}", plan.id.0, saved.classroom, session.key)
                    };
                    let starts = session.date.and_time(session.start);
                    if settings.online_class_reminders
                        && let SessionMode::Online(link) = &session.plan.mode
                        && starts > now
                        && starts - chrono::Duration::hours(1) <= now
                    {
                        reminders.push(ClassReminder {
                            id: id("online"),
                            kind: ClassReminderKind::OnlineSoon,
                            plan_title: plan.title.clone(),
                            date: session.date,
                            start: session.start,
                            link: Some(link.clone()),
                        });
                    }
                    if settings.homework_reminders && today.succ_opt() == Some(session.date) {
                        // The homework of the class before this one.
                        let open = schedule
                            .sessions
                            .iter()
                            .filter(|s| (s.date, s.start) < (session.date, session.start))
                            .filter(|s| !s.plan.homework.is_empty())
                            .max_by_key(|s| (s.date, s.start))
                            .and_then(|s| work.get(&s.key))
                            .is_some_and(|w| w.homework_done < w.homework_total);
                        if open {
                            reminders.push(ClassReminder {
                                id: id("homework"),
                                kind: ClassReminderKind::HomeworkOpen,
                                plan_title: plan.title.clone(),
                                date: session.date,
                                start: session.start,
                                link: None,
                            });
                        }
                    }
                }
            }
        }
        reminders
    }

    /// For a student, the day a topic opens when its classroom opens topics
    /// as the classes happen and its first class is still ahead.
    pub fn topic_opens_on(&self, topic: kg_study::TopicId) -> Option<chrono::NaiveDate> {
        if self.role() != crate::ClassRole::Student {
            return None;
        }
        let plan = self.store.plan_of_topic(topic).ok().flatten()?;
        let key = self
            .store
            .topic_keys(plan)
            .ok()?
            .into_iter()
            .find(|(id, _)| *id == topic)
            .map(|(_, key)| key)?;
        let today = Self::today();
        self.class_schedules(plan)
            .iter()
            .filter(|saved| !saved.schedule.topic_open(&key, today))
            .filter_map(|saved| {
                saved
                    .schedule
                    .sessions
                    .iter()
                    .filter(|s| {
                        s.kind == SessionKind::Class && s.status != SessionStatus::Cancelled
                    })
                    .filter(|s| s.topics.contains(&key))
                    .map(|s| s.date)
                    .min()
            })
            .min()
    }

    pub(crate) fn on_planning_event(&mut self, event: PlanningEvent) {
        match event {
            PlanningEvent::PdfExported(result) => match result {
                Ok(path) => self.notices.push(crate::Notice::Exported(path)),
                Err(err) => self.notices.push(crate::Notice::Error(err)),
            },
            PlanningEvent::Published {
                plan,
                classroom,
                result,
            } => {
                self.planning.publishing = None;
                match result {
                    Ok(version) => {
                        if let Err(err) = self
                            .store
                            .set_class_schedule_published(plan, classroom, version)
                        {
                            self.notices.push(crate::Notice::Error(err.into()));
                        }
                        self.notices.push(crate::Notice::SchedulePublished);
                    }
                    Err(err) => self.notices.push(crate::Notice::Error(err.into())),
                }
            }
            PlanningEvent::Suggested {
                plan,
                classroom,
                plans,
                failed,
                error,
            } => {
                self.planning.suggesting = None;
                // Applied to the schedule as it is now: classes the teacher
                // planned or changed meanwhile keep what they have.
                if let Some(saved) = self.class_schedule(plan, classroom) {
                    let mut schedule = saved.schedule;
                    let mut applied = 0;
                    for (key, suggestion) in plans {
                        if let Some(session) = schedule
                            .sessions
                            .iter_mut()
                            .find(|s| s.key == key && s.plan.is_empty())
                        {
                            let keep = std::mem::take(&mut session.plan);
                            session.plan = SessionPlan {
                                mode: keep.mode,
                                note: keep.note,
                                private_note: keep.private_note,
                                ..suggestion
                            };
                            applied += 1;
                        }
                    }
                    if applied > 0
                        && let Err(err) = self.save_schedule(plan, classroom, &schedule)
                    {
                        self.notices.push(crate::Notice::Error(err));
                    }
                    self.notices
                        .push(crate::Notice::SessionsPlanned { applied, failed });
                }
                if let Some(err) = error.filter(|_| failed > 0) {
                    self.notices.push(crate::Notice::Error(err.into()));
                }
            }
        }
    }

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

/// Asks `ask` for the classes in batches; a class whose answer fails the
/// checks is asked once more, then left empty.
pub(crate) fn suggest_in_batches(
    briefs: &[(Session, SessionBrief)],
    mut ask: impl FnMut(Vec<SessionBrief>) -> Result<Vec<SessionDraft>, kg_ai::AiError>,
    keys: &[String],
    materials: &[String],
) -> (Vec<(String, SessionPlan)>, usize, Option<kg_ai::AiError>) {
    let mut plans = Vec::new();
    let mut failed = 0;
    let mut error = None;
    for batch in briefs.chunks(BATCH) {
        let mut left: Vec<&(Session, SessionBrief)> = batch.iter().collect();
        for _attempt in 0..2 {
            if left.is_empty() {
                break;
            }
            match ask(left.iter().map(|(_, b)| b.clone()).collect()) {
                Ok(drafts) => left.retain(|(session, brief)| {
                    let checked = drafts
                        .iter()
                        .find(|d| d.session == brief.number)
                        .and_then(|d| checked_plan(d, session, keys, materials));
                    match checked {
                        Some(plan) => {
                            plans.push((session.key.clone(), plan));
                            false
                        }
                        None => true,
                    }
                }),
                Err(err) => {
                    tracing::warn!(%err, "lesson planning request failed");
                    error = Some(err);
                }
            }
        }
        failed += left.len();
    }
    (plans, failed, error)
}

fn kind_id(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::Class => "class",
        SessionKind::Review => "review",
        SessionKind::MockExam => "mock_exam",
    }
}

fn activity_kind(id: &str) -> Option<ActivityKind> {
    Some(match id {
        "explanation" => ActivityKind::Explanation,
        "voice_lesson" => ActivityKind::VoiceLesson,
        "handout_reading" => ActivityKind::HandoutReading,
        "quiz" => ActivityKind::Quiz,
        "spoken_review" => ActivityKind::SpokenReview,
        "written_answer" => ActivityKind::WrittenAnswer,
        "mock_exam" => ActivityKind::MockExam,
        "exercise" => ActivityKind::Exercise,
        "other" => ActivityKind::Other,
        _ => return None,
    })
}

/// The model's plan for `session`, if it passes the checks: objectives
/// present, known activity kinds, topics of that class, minutes within the
/// class, homework the app can follow. Materials that are not in the plan
/// are dropped rather than failing the class.
pub(crate) fn checked_plan(
    draft: &SessionDraft,
    session: &Session,
    keys: &[String],
    materials: &[String],
) -> Option<SessionPlan> {
    let objectives: Vec<String> = draft
        .objectives
        .iter()
        .map(|o| o.trim().to_owned())
        .filter(|o| !o.is_empty())
        .collect();
    if objectives.is_empty() || draft.activities.is_empty() {
        return None;
    }
    let topic = |number: usize| -> Option<Option<String>> {
        if number == 0 {
            return Some(None);
        }
        let key = keys.get(number - 1)?;
        session.topics.contains(key).then(|| Some(key.clone()))
    };
    let activity = |a: &kg_ai::ActivityDraft| -> Option<Activity> {
        let text = a.text.trim();
        if text.is_empty() || a.minutes == 0 {
            return None;
        }
        Some(Activity {
            kind: activity_kind(a.kind.trim())?,
            minutes: a.minutes,
            text: text.to_owned(),
            topic: topic(a.topic)?,
        })
    };
    let activities: Vec<Activity> = draft
        .activities
        .iter()
        .map(activity)
        .collect::<Option<_>>()?;
    if activities.iter().map(|a| a.minutes).sum::<u32>() > session.minutes {
        return None;
    }
    let homework: Vec<Activity> = draft.homework.iter().map(activity).collect::<Option<_>>()?;
    if homework.iter().any(|a| !a.kind.works_as_homework()) {
        return None;
    }
    Some(SessionPlan {
        objectives,
        activities,
        materials: draft
            .materials
            .iter()
            .map(|m| m.trim())
            .filter(|m| materials.iter().any(|known| known == m))
            .map(str::to_owned)
            .collect(),
        homework,
        ..SessionPlan::default()
    })
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, Weekday};

    use super::*;
    use kg_study::schedule::{ClassTime, HolidayCalendar};

    fn session(topics: &[&str], minutes: u32) -> Session {
        Session {
            key: "s1".into(),
            date: NaiveDate::from_ymd_opt(2026, 3, 2).expect("date"),
            start: NaiveTime::from_hms_opt(19, 0, 0).expect("time"),
            minutes,
            kind: SessionKind::Class,
            topics: topics.iter().map(|t| (*t).to_owned()).collect(),
            status: SessionStatus::Planned,
            locked: false,
            covered: None,
            plan: SessionPlan::default(),
        }
    }

    fn draft(json: &str) -> SessionDraft {
        serde_json::from_str(json).expect("draft")
    }

    #[test]
    fn the_models_class_plan_is_checked_in_code() {
        let keys = ["t1".to_owned(), "t2".to_owned(), "t3".to_owned()];
        let materials = ["Apostila: Funções".to_owned()];
        let class = session(&["t1", "t2"], 100);
        let good = draft(
            r#"{"session": 1, "objectives": ["Identificar funções"],
                "activities": [{"kind": "explanation", "minutes": 40, "text": "Exposição", "topic": 1},
                               {"kind": "exercise", "minutes": 50, "text": "Lista", "topic": 0}],
                "materials": ["Apostila: Funções", "Livro inventado"],
                "homework": [{"kind": "quiz", "minutes": 15, "text": "Quiz de funções", "topic": 1}]}"#,
        );
        let plan = checked_plan(&good, &class, &keys, &materials).expect("passes");
        assert_eq!(plan.activity_minutes(), 90);
        assert_eq!(
            plan.materials,
            ["Apostila: Funções"],
            "unknown material dropped"
        );
        assert_eq!(plan.activities[0].topic.as_deref(), Some("t1"));

        // A topic of another class, too many minutes, an unknown kind, homework
        // the app cannot follow, no objectives: each fails.
        for bad in [
            r#"{"objectives": ["x"], "activities": [{"kind": "explanation", "minutes": 10, "text": "a", "topic": 3}]}"#,
            r#"{"objectives": ["x"], "activities": [{"kind": "explanation", "minutes": 101, "text": "a"}]}"#,
            r#"{"objectives": ["x"], "activities": [{"kind": "dance", "minutes": 10, "text": "a"}]}"#,
            r#"{"objectives": ["x"], "activities": [{"kind": "quiz", "minutes": 10, "text": "a"}], "homework": [{"kind": "exercise", "minutes": 10, "text": "b"}]}"#,
            r#"{"objectives": [" "], "activities": [{"kind": "quiz", "minutes": 10, "text": "a"}]}"#,
        ] {
            assert!(
                checked_plan(&draft(bad), &class, &keys, &materials).is_none(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_class_failing_the_checks_is_asked_once_more_then_left_empty() {
        let keys = ["t1".to_owned()];
        let briefs: Vec<(Session, SessionBrief)> = (1..=2)
            .map(|n| {
                let mut s = session(&["t1"], 60);
                s.key = format!("s{n}");
                let brief = SessionBrief {
                    number: n,
                    kind: "class".into(),
                    minutes: 60,
                    topics: vec![1],
                };
                (s, brief)
            })
            .collect();
        let mut calls = Vec::new();
        let (plans, failed, error) = suggest_in_batches(
            &briefs,
            |sessions| {
                calls.push(sessions.len());
                // Class 1 is always fine; class 2 always runs over its time.
                Ok(sessions
                    .iter()
                    .map(|b| SessionDraft {
                        session: b.number,
                        objectives: vec!["Aplicar".into()],
                        activities: vec![kg_ai::ActivityDraft {
                            kind: "exercise".into(),
                            minutes: if b.number == 1 { 50 } else { 90 },
                            text: "Exercícios".into(),
                            topic: 1,
                        }],
                        ..SessionDraft::default()
                    })
                    .collect())
            },
            &keys,
            &[],
        );
        assert_eq!(calls, [2, 1], "only the failed class is asked again");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].0, "s1");
        assert_eq!(failed, 1);
        assert!(error.is_none());
    }

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
