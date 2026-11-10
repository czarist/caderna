//! Classes in the cloud, through the brand's panel.
//!
//! People enrolled in the panel's client area are teachers or students
//! ([`ClassRole`]). A teacher creates plans and content here and publishes a
//! plan to classrooms; the panel keeps it as a course. A student only uses
//! what their classrooms give them: their app downloads the courses as
//! read-only plans, with the teacher's handouts and quiz questions, and
//! keeps the student's progress in the panel so it follows them to another
//! computer (see [`kg_study::sync`]).
//!
//! Without a panel, or with a licence issued without an enrolment, the app
//! works as it always did ([`ClassRole::Owner`]).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use kg_study::sync::{OverallProgress, PlanProgress};
use kg_study::{
    CourseLink, CoursePlan, CourseSubject, CourseTopic, Difficulty, PlanId, StudyPlan, TopicId,
};
use serde::{Deserialize, Serialize};

use crate::server::{HttpPanel, PanelError};
use crate::{Command, Engine, EngineError, Event, Notice};

/// What the person using this computer does in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassRole {
    /// No classes: a licence without an enrolment, or a brand without a panel.
    #[default]
    Owner,
    /// Creates plans and content and publishes them to classrooms.
    Teacher,
    /// Uses what the classrooms give; creates no plans or subjects.
    Student,
}

impl ClassRole {
    /// Whether this person may create plans, subjects and content.
    pub fn creates_plans(self) -> bool {
        self != Self::Student
    }
}

impl Command {
    /// Commands that create or change plans and what they are made from;
    /// students only use what their classrooms give them.
    pub(crate) fn creates_plans(&self) -> bool {
        matches!(
            self,
            Self::ImportEdital(_)
                | Self::ClearEdital
                | Self::RevalidateEdital { .. }
                | Self::GeneratePlan(_)
                | Self::DeletePlan(_)
                | Self::PrepareContent(_)
                | Self::ImportBoard { .. }
                | Self::ImportAnswerKey { .. }
                | Self::DeleteBoardDocument(_)
                | Self::ReuseBoardDocument(_)
                | Self::SetBoardItem { .. }
                | Self::PublishPlan { .. }
                | Self::LoadClassrooms
        )
    }
}

/// A course as the panel lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CourseSummary {
    pub id: i64,
    pub title: String,
    pub version: i64,
    /// Sent to teachers only.
    #[serde(default)]
    pub classroom_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CourseContent {
    weeks: u32,
    #[serde(default)]
    syllabus_name: String,
    #[serde(default)]
    syllabus: String,
    subjects: Vec<WireSubject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireSubject {
    name: String,
    topics: Vec<WireTopic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireTopic {
    key: String,
    title: String,
    #[serde(default)]
    difficulty: String,
    #[serde(default = "first_week")]
    week: u32,
}

fn first_week() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireMaterial {
    topic: String,
    kind: String,
    content: String,
}

/// A whole course, downloaded because it is new or changed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CourseDownload {
    #[serde(flatten)]
    summary: CourseSummary,
    #[serde(default)]
    exam: String,
    #[serde(default)]
    purpose: String,
    content: CourseContent,
    #[serde(default)]
    materials: Vec<WireMaterial>,
}

/// A classroom as a teacher sees it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Classroom {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub course_ids: Vec<i64>,
    #[serde(default)]
    pub students: Vec<ClassStudent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ClassStudent {
    pub id: i64,
    pub name: String,
    /// Minutes studied in all.
    #[serde(default)]
    pub minutes: u64,
    #[serde(default)]
    pub courses: Vec<StudentCourse>,
}

/// How a student is doing on one course.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StudentCourse {
    pub course_id: i64,
    #[serde(default)]
    pub topics: u32,
    #[serde(default)]
    pub topics_done: u32,
    #[serde(default)]
    pub topics_started: u32,
    #[serde(default)]
    pub quiz_correct: u32,
    #[serde(default)]
    pub quiz_total: u32,
    #[serde(default)]
    pub lessons: u32,
    /// ISO date.
    #[serde(default)]
    pub last_activity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct ProgressDoc {
    course_id: Option<i64>,
    #[serde(default)]
    data: serde_json::Value,
}

/// What a student's sync brought from the panel.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SyncDownload {
    courses: Vec<CourseSummary>,
    fresh: Vec<CourseDownload>,
    progress: Vec<ProgressDoc>,
}

/// The engine's class state.
#[derive(Debug, Default)]
pub(crate) struct Classes {
    syncing: bool,
    publishing: Option<PlanId>,
    loading: bool,
    classrooms: Option<Vec<Classroom>>,
    links: HashMap<PlanId, CourseLink>,
    last_sync: Option<DateTime<Utc>>,
}

fn data<T: for<'de> Deserialize<'de>>(status: u16, text: &str) -> Result<T, PanelError> {
    #[derive(Deserialize)]
    struct Envelope<T> {
        data: T,
    }
    serde_json::from_str::<Envelope<T>>(text)
        .map(|e| e.data)
        .map_err(|e| PanelError::Unexpected {
            status,
            detail: e.to_string(),
        })
}

/// The courses, the changed ones in full, and the stored progress.
fn download(
    panel: &HttpPanel,
    token: &str,
    known: &HashMap<i64, i64>,
) -> Result<SyncDownload, PanelError> {
    let (status, text) = panel.call("GET", "courses", token, None)?;
    let courses: Vec<CourseSummary> = data(status, &text)?;
    let mut fresh = Vec::new();
    for course in &courses {
        if known.get(&course.id) == Some(&course.version) {
            continue;
        }
        let (status, text) = panel.call("GET", &format!("courses/{}", course.id), token, None)?;
        fresh.push(data(status, &text)?);
    }
    let (status, text) = panel.call("GET", "progress", token, None)?;
    let progress = data(status, &text)?;
    Ok(SyncDownload {
        courses,
        fresh,
        progress,
    })
}

fn upload(
    panel: &HttpPanel,
    token: &str,
    docs: &[(Option<i64>, serde_json::Value)],
) -> Result<(), PanelError> {
    for (course, data) in docs {
        let path = match course {
            Some(id) => format!("progress/courses/{id}"),
            None => "progress/overall".to_owned(),
        };
        panel.call(
            "PUT",
            &path,
            token,
            Some(&serde_json::json!({ "data": data })),
        )?;
    }
    Ok(())
}

fn publish(
    panel: &HttpPanel,
    token: &str,
    existing: Option<i64>,
    body: &serde_json::Value,
) -> Result<CourseSummary, PanelError> {
    let answer = match existing {
        Some(id) => match panel.call("PUT", &format!("courses/{id}"), token, Some(body)) {
            // Removed in the panel meanwhile: publish it again.
            Err(PanelError::Unexpected { status: 404, .. }) => {
                panel.call("POST", "courses", token, Some(body))
            }
            other => other,
        },
        None => panel.call("POST", "courses", token, Some(body)),
    }?;
    data(answer.0, &answer.1)
}

/// A course's plan as the store keeps it.
fn course_plan(course: &CourseDownload) -> CoursePlan {
    CoursePlan {
        title: course.summary.title.clone(),
        exam: course.exam.clone(),
        purpose: kg_ai::ExamPurpose::from_id(&course.purpose).id().to_owned(),
        weeks: course.content.weeks,
        syllabus_name: course.content.syllabus_name.clone(),
        syllabus: course.content.syllabus.clone(),
        subjects: course
            .content
            .subjects
            .iter()
            .map(|s| CourseSubject {
                name: s.name.clone(),
                topics: s
                    .topics
                    .iter()
                    .map(|t| CourseTopic {
                        key: t.key.clone(),
                        title: t.title.clone(),
                        difficulty: Difficulty::parse_lenient(&t.difficulty),
                        week: t.week,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// What a teacher sends for a plan: its subjects and topics with their
/// keys, the syllabus, and the handouts and quiz questions made so far.
fn course_body(
    plan: &StudyPlan,
    keys: &HashMap<TopicId, String>,
    syllabus: &str,
    language: &str,
    materials: Vec<WireMaterial>,
    classrooms: &[i64],
) -> serde_json::Value {
    let content = CourseContent {
        weeks: plan.weeks,
        syllabus_name: String::new(),
        syllabus: syllabus.to_owned(),
        subjects: plan
            .subjects
            .iter()
            .map(|s| WireSubject {
                name: s.name.clone(),
                topics: s
                    .topics
                    .iter()
                    .map(|t| WireTopic {
                        key: keys
                            .get(&t.id)
                            .cloned()
                            .unwrap_or_else(|| format!("t{}", t.id.0)),
                        title: t.title.clone(),
                        difficulty: t.difficulty.as_str().to_owned(),
                        week: t.week,
                    })
                    .collect(),
            })
            .collect(),
    };
    serde_json::json!({
        "title": plan.title,
        "exam": plan.exam,
        "purpose": kg_ai::ExamPurpose::from_id(&plan.purpose).id(),
        "language": language,
        "content": content,
        "materials": materials,
        "classroom_ids": classrooms,
    })
}

/// Largest syllabus sent with a course, in characters.
const SYLLABUS_CHARS: usize = 300_000;

impl Engine {
    pub fn classrooms(&self) -> Option<&[Classroom]> {
        self.classes.classrooms.as_deref()
    }

    pub fn loading_classrooms(&self) -> bool {
        self.classes.loading
    }

    /// The plan being published, if any.
    pub fn publishing(&self) -> Option<PlanId> {
        self.classes.publishing
    }

    pub fn syncing_classes(&self) -> bool {
        self.classes.syncing
    }

    /// When this computer last brought its courses and progress up to date.
    pub fn last_class_sync(&self) -> Option<DateTime<Utc>> {
        self.classes.last_sync
    }

    /// A plan's course in the panel: received from a teacher (`assigned`)
    /// or published from here.
    pub fn course_link(&self, plan: PlanId) -> Option<CourseLink> {
        self.classes.links.get(&plan).copied()
    }

    pub(crate) fn refresh_course_links(&mut self) {
        match self.store.course_links() {
            Ok(links) => self.classes.links = links.into_iter().collect(),
            Err(err) => tracing::warn!(%err, "could not read the course links"),
        }
    }

    /// After every good check with the panel: students bring their courses
    /// and progress up to date.
    pub(crate) fn after_panel_check(&mut self) {
        if self.role() == ClassRole::Student {
            self.start_class_sync();
        }
    }

    fn start_class_sync(&mut self) {
        if self.classes.syncing {
            return;
        }
        let Some(token) = self.panel_token() else {
            return;
        };
        let known: HashMap<i64, i64> = self
            .classes
            .links
            .values()
            .filter(|link| link.assigned)
            .map(|link| (link.course_id, link.version))
            .collect();
        self.classes.syncing = true;
        self.spawn(move || {
            Event::ClassesDownloaded(download(&HttpPanel::for_content(), &token, &known))
        });
    }

    pub(crate) fn on_classes_downloaded(
        &mut self,
        result: Result<SyncDownload, PanelError>,
    ) -> Result<(), EngineError> {
        self.classes.syncing = false;
        let download = match result {
            Ok(download) => download,
            Err(err) => {
                tracing::info!(%err, "class sync failed; trying again at the next check");
                return Ok(());
            }
        };
        let uploads = self.apply_class_download(&download)?;
        self.classes.last_sync = Some(Utc::now());
        if !download.fresh.is_empty() {
            self.notices
                .push(Notice::CoursesUpdated(download.fresh.len()));
        }
        if !uploads.is_empty()
            && let Some(token) = self.panel_token()
        {
            self.spawn(move || {
                Event::ProgressUploaded(upload(&HttpPanel::for_content(), &token, &uploads))
            });
        }
        Ok(())
    }

    /// Applies a sync on this computer; returns the progress to send back.
    pub(crate) fn apply_class_download(
        &mut self,
        download: &SyncDownload,
    ) -> Result<Vec<(Option<i64>, serde_json::Value)>, EngineError> {
        // Courses taken away from the student's classrooms leave this
        // computer; their progress stays in the panel.
        let listed: HashSet<i64> = download.courses.iter().map(|c| c.id).collect();
        for (plan, link) in self.store.course_links()? {
            if link.assigned && !listed.contains(&link.course_id) {
                tracing::info!(
                    course = link.course_id,
                    "course removed from the classrooms"
                );
                self.delete_plan(plan)?;
            }
        }
        self.refresh_course_links();

        let mut restoring = HashSet::new();
        for course in &download.fresh {
            let existing = self
                .classes
                .links
                .iter()
                .find(|(_, link)| link.assigned && link.course_id == course.summary.id)
                .map(|(plan, _)| *plan);
            let plan =
                self.store
                    .upsert_course_plan(existing, &course_plan(course), Self::today())?;
            self.store.set_plan_course(
                plan,
                CourseLink {
                    course_id: course.summary.id,
                    version: course.summary.version,
                    assigned: true,
                },
            )?;
            self.save_course_materials(plan, &course.materials)?;
            if existing.is_none() {
                restoring.insert(plan);
            }
            tracing::info!(
                course = course.summary.id,
                version = course.summary.version,
                "course received"
            );
        }
        self.refresh_course_links();

        let mut remote: HashMap<Option<i64>, serde_json::Value> = download
            .progress
            .iter()
            .map(|doc| (doc.course_id, doc.data.clone()))
            .collect();
        let mut uploads = Vec::new();
        let links: Vec<(PlanId, CourseLink)> = self
            .classes
            .links
            .iter()
            .filter(|(_, link)| link.assigned)
            .map(|(plan, link)| (*plan, *link))
            .collect();
        for (plan, link) in links {
            let theirs: PlanProgress = remote
                .remove(&Some(link.course_id))
                .and_then(|value| serde_json::from_value(value).ok())
                .unwrap_or_default();
            let ours = self.store.plan_progress(plan)?;
            let delta = ours.missing_from(&theirs, restoring.contains(&plan));
            self.store.apply_plan_progress(plan, &delta)?;
            let merged = self.store.plan_progress(plan)?;
            if merged != theirs
                && let Ok(value) = serde_json::to_value(&merged)
            {
                uploads.push((Some(link.course_id), value));
            }
        }
        let theirs: OverallProgress = remote
            .remove(&None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        let extra = self.store.overall_progress()?.missing_from(&theirs);
        self.store.apply_overall_progress(&extra)?;
        let merged = self.store.overall_progress()?;
        if merged != theirs
            && let Ok(value) = serde_json::to_value(&merged)
        {
            uploads.push((None, value));
        }
        self.reload_plans();
        Ok(uploads)
    }

    /// The teacher's handouts and quiz questions, kept so the student's app
    /// uses them as they are.
    fn save_course_materials(
        &mut self,
        plan: PlanId,
        materials: &[WireMaterial],
    ) -> Result<(), EngineError> {
        let topics: HashMap<String, TopicId> = self
            .store
            .topic_keys(plan)?
            .into_iter()
            .map(|(id, key)| (key, id))
            .collect();
        for material in materials {
            let Some(topic) = topics.get(&material.topic) else {
                continue;
            };
            match material.kind.as_str() {
                "handout" => {
                    if let Some(json) = crate::handouts::teacher_handout(&material.content) {
                        self.store.save_handout(*topic, &json, Self::today())?;
                    }
                }
                "quiz_bank" => {
                    let usable =
                        serde_json::from_str::<Vec<kg_ai::QuestionDraft>>(&material.content)
                            .is_ok_and(|questions| {
                                !questions.is_empty()
                                    && questions.iter().all(|q| q.correct < q.options.len())
                            });
                    if usable {
                        self.store
                            .save_quiz_bank(*topic, &material.content, Self::today())?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn on_progress_uploaded(&mut self, result: Result<(), PanelError>) {
        if let Err(err) = result {
            tracing::info!(%err, "progress not sent; trying again at the next check");
        }
    }

    /// Publishes a plan to classrooms (or updates the course it already is).
    pub(crate) fn publish_plan(
        &mut self,
        plan: PlanId,
        classrooms: Vec<i64>,
    ) -> Result<(), EngineError> {
        if self.role() != ClassRole::Teacher {
            return Err(EngineError::TeachersOnly);
        }
        if self.classes.publishing.is_some() {
            return Ok(());
        }
        let token = self.panel_token().ok_or(EngineError::PanelOffline)?;
        let study_plan = self.store.load_plan(plan)?;
        let keys: HashMap<TopicId, String> = self.store.topic_keys(plan)?.into_iter().collect();
        let mut materials = Vec::new();
        for (_, topic) in study_plan.topics() {
            let key = keys
                .get(&topic.id)
                .cloned()
                .unwrap_or_else(|| format!("t{}", topic.id.0));
            if let Some(content) = self
                .store
                .handout(topic.id)?
                .as_deref()
                .and_then(crate::handouts::handout_content)
            {
                materials.push(WireMaterial {
                    topic: key.clone(),
                    kind: "handout".into(),
                    content,
                });
            }
            if let Some(content) = self.store.quiz_bank(topic.id)? {
                materials.push(WireMaterial {
                    topic: key,
                    kind: "quiz_bank".into(),
                    content,
                });
            }
        }
        let syllabus: String = self
            .store
            .edital_text(plan)?
            .chars()
            .take(SYLLABUS_CHARS)
            .collect();
        let body = course_body(
            &study_plan,
            &keys,
            &syllabus,
            &self.settings.locale,
            materials,
            &classrooms,
        );
        let existing = self
            .course_link(plan)
            .filter(|link| !link.assigned)
            .map(|link| link.course_id);
        self.classes.publishing = Some(plan);
        let count = classrooms.len();
        self.spawn(move || Event::PlanPublished {
            plan,
            classrooms: count,
            result: publish(&HttpPanel::for_content(), &token, existing, &body),
        });
        Ok(())
    }

    pub(crate) fn on_plan_published(
        &mut self,
        plan: PlanId,
        classrooms: usize,
        result: Result<CourseSummary, PanelError>,
    ) -> Result<(), EngineError> {
        self.classes.publishing = None;
        let course = result?;
        self.store.set_plan_course(
            plan,
            CourseLink {
                course_id: course.id,
                version: course.version,
                assigned: false,
            },
        )?;
        self.refresh_course_links();
        self.notices.push(Notice::PlanPublished { classrooms });
        self.load_classrooms()
    }

    /// Asks the panel for the school's classrooms and how students do.
    pub(crate) fn load_classrooms(&mut self) -> Result<(), EngineError> {
        if self.role() != ClassRole::Teacher {
            return Err(EngineError::TeachersOnly);
        }
        if self.classes.loading {
            return Ok(());
        }
        let token = self.panel_token().ok_or(EngineError::PanelOffline)?;
        self.classes.loading = true;
        self.spawn(move || {
            let panel = HttpPanel::new();
            Event::ClassroomsLoaded(
                panel
                    .call("GET", "classrooms", &token, None)
                    .and_then(|(status, text)| data(status, &text)),
            )
        });
        Ok(())
    }

    pub(crate) fn on_classrooms_loaded(
        &mut self,
        result: Result<Vec<Classroom>, PanelError>,
    ) -> Result<(), EngineError> {
        self.classes.loading = false;
        self.classes.classrooms = Some(result?);
        Ok(())
    }

    /// Brings courses and progress up to date now.
    pub(crate) fn sync_classes_now(&mut self) {
        self.check_panel_now();
    }

    /// Test hook: applies a sync as if the panel had sent it.
    #[cfg(test)]
    pub(crate) fn apply_class_json(
        &mut self,
        courses: serde_json::Value,
        fresh: serde_json::Value,
        progress: serde_json::Value,
    ) -> Result<Vec<(Option<i64>, serde_json::Value)>, EngineError> {
        let download = SyncDownload {
            courses: serde_json::from_value(courses).map_err(|e| PanelError::Unexpected {
                status: 0,
                detail: e.to_string(),
            })?,
            fresh: serde_json::from_value(fresh).map_err(|e| PanelError::Unexpected {
                status: 0,
                detail: e.to_string(),
            })?,
            progress: serde_json::from_value(progress).map_err(|e| PanelError::Unexpected {
                status: 0,
                detail: e.to_string(),
            })?,
        };
        self.apply_class_download(&download)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_published_plan_becomes_the_course_body_the_panel_checks() {
        let plan = StudyPlan {
            id: PlanId(1),
            title: "ENEM 2027".into(),
            exam: "ENEM".into(),
            weeks: 8,
            created_on: chrono::NaiveDate::default(),
            purpose: "national_exam".into(),
            subjects: vec![kg_study::Subject {
                id: kg_study::SubjectId(1),
                name: "Matemática".into(),
                topics: vec![kg_study::Topic {
                    id: TopicId(7),
                    title: "Funções".into(),
                    difficulty: Difficulty::Hard,
                    status: kg_study::TopicStatus::Done,
                    week: 2,
                    review: kg_study::ReviewState::default(),
                }],
            }],
        };
        let body = course_body(
            &plan,
            &HashMap::new(),
            "Matriz",
            "pt-BR",
            vec![WireMaterial {
                topic: "t7".into(),
                kind: "handout".into(),
                content: "{}".into(),
            }],
            &[3],
        );
        assert_eq!(body["purpose"], "national_exam");
        assert_eq!(body["content"]["subjects"][0]["topics"][0]["key"], "t7");
        assert_eq!(
            body["content"]["subjects"][0]["topics"][0]["difficulty"],
            "hard"
        );
        assert_eq!(body["content"]["syllabus"], "Matriz");
        assert_eq!(body["classroom_ids"][0], 3);
        // The student's progress never travels with the course.
        assert!(!body.to_string().contains("done"));
    }
}
