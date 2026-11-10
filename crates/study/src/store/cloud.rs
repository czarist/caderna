//! Plans shared through the brand's panel: the link between a local plan
//! and its course there, the topics' stable keys, and the progress kept in
//! the cloud (see [`crate::sync`]).

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};

use super::{Result, Store, StoreError, parse_date};
use crate::plan::{Difficulty, PlanId, TopicId};
use crate::sync::{
    LessonRecord, MistakeRecord, OverallProgress, PlanProgress, ProgressDelta, QuizRecord,
    ReviewRecord, TopicProgress,
};

/// A local plan's course in the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseLink {
    pub course_id: i64,
    /// The course version this computer has.
    pub version: i64,
    /// Received from a teacher (read-only here), not published from here.
    pub assigned: bool,
}

/// A course received from a teacher, ready to be kept as a local plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoursePlan {
    pub title: String,
    pub exam: String,
    /// `kg_ai::ExamPurpose` id.
    pub purpose: String,
    pub weeks: u32,
    pub syllabus_name: String,
    /// The exam syllabus, so handouts can still be written here.
    pub syllabus: String,
    pub subjects: Vec<CourseSubject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseSubject {
    pub name: String,
    pub topics: Vec<CourseTopic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseTopic {
    /// Stable key from the teacher (unique in the course).
    pub key: String,
    pub title: String,
    pub difficulty: Difficulty,
    pub week: u32,
}

impl Store {
    /// Creates the plan of a course, or changes it to the course's new
    /// version. Topics are matched by key, so a topic that stays keeps its
    /// progress, handout and everything made for it; topics the teacher
    /// removed go, with their progress.
    pub fn upsert_course_plan(
        &mut self,
        existing: Option<PlanId>,
        course: &CoursePlan,
        today: chrono::NaiveDate,
    ) -> Result<PlanId> {
        let weeks = course.weeks.clamp(1, 104);
        let subjects: Vec<&CourseSubject> = course
            .subjects
            .iter()
            .filter(|s| {
                !s.name.trim().is_empty() && s.topics.iter().any(|t| !t.title.trim().is_empty())
            })
            .collect();
        if subjects.is_empty() {
            return Err(StoreError::EmptyCourse);
        }
        let old_keys: HashMap<String, i64> = match existing {
            Some(plan) => self
                .topic_keys(plan)?
                .into_iter()
                .map(|(id, key)| (key, id.0))
                .collect(),
            None => HashMap::new(),
        };
        let tx = self.conn.transaction()?;
        let plan_id = match existing {
            Some(plan) => {
                let changed = tx.execute(
                    "UPDATE plans SET title = ?2, exam = ?3, weeks = ?4, purpose = ?5,
                            edital_name = ?6, edital_text = ?7
                     WHERE id = ?1",
                    params![
                        plan.0,
                        course.title.trim(),
                        course.exam.trim(),
                        weeks,
                        course.purpose,
                        course.syllabus_name,
                        course.syllabus
                    ],
                )?;
                if changed == 0 {
                    return Err(StoreError::PlanNotFound(plan));
                }
                plan
            }
            None => {
                tx.execute(
                    "INSERT INTO plans (title, exam, weeks, created_on, edital_name, edital_text, purpose)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        course.title.trim(),
                        course.exam.trim(),
                        weeks,
                        today.to_string(),
                        course.syllabus_name,
                        course.syllabus,
                        course.purpose
                    ],
                )?;
                PlanId(tx.last_insert_rowid())
            }
        };
        let old_subjects: HashMap<String, i64> = {
            let mut stmt = tx.prepare("SELECT name, id FROM subjects WHERE plan_id = ?1")?;
            let rows = stmt.query_map([plan_id.0], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let mut kept_topics = Vec::new();
        let mut kept_subjects = Vec::new();
        for (s_pos, subject) in subjects.iter().enumerate() {
            let name = subject.name.trim();
            let subject_id = match old_subjects.get(name) {
                Some(id) => {
                    tx.execute(
                        "UPDATE subjects SET position = ?2 WHERE id = ?1",
                        params![id, s_pos as i64],
                    )?;
                    *id
                }
                None => {
                    tx.execute(
                        "INSERT INTO subjects (plan_id, position, name) VALUES (?1, ?2, ?3)",
                        params![plan_id.0, s_pos as i64, name],
                    )?;
                    tx.last_insert_rowid()
                }
            };
            kept_subjects.push(subject_id);
            let topics = subject.topics.iter().filter(|t| !t.title.trim().is_empty());
            for (t_pos, topic) in topics.enumerate() {
                let week = topic.week.clamp(1, weeks);
                let topic_id = match old_keys.get(&topic.key) {
                    Some(id) => {
                        tx.execute(
                            "UPDATE topics SET subject_id = ?2, position = ?3, title = ?4,
                                    difficulty = ?5, week = ?6
                             WHERE id = ?1",
                            params![
                                id,
                                subject_id,
                                t_pos as i64,
                                topic.title.trim(),
                                topic.difficulty.as_str(),
                                week
                            ],
                        )?;
                        *id
                    }
                    None => {
                        tx.execute(
                            "INSERT INTO topics (subject_id, position, title, difficulty, status, week)
                             VALUES (?1, ?2, ?3, ?4, 'not_started', ?5)",
                            params![
                                subject_id,
                                t_pos as i64,
                                topic.title.trim(),
                                topic.difficulty.as_str(),
                                week
                            ],
                        )?;
                        let id = tx.last_insert_rowid();
                        tx.execute(
                            "INSERT INTO topic_keys (topic_id, key) VALUES (?1, ?2)",
                            params![id, topic.key],
                        )?;
                        id
                    }
                };
                kept_topics.push(topic_id);
            }
        }
        for id in old_keys.values().filter(|id| !kept_topics.contains(id)) {
            tx.execute("DELETE FROM topics WHERE id = ?1", [id])?;
        }
        for id in old_subjects
            .values()
            .filter(|id| !kept_subjects.contains(id))
        {
            tx.execute("DELETE FROM subjects WHERE id = ?1", [id])?;
        }
        tx.commit()?;
        Ok(plan_id)
    }

    pub fn plan_course(&self, plan: PlanId) -> Result<Option<CourseLink>> {
        Ok(self
            .conn
            .query_row(
                "SELECT course_id, version, assigned FROM plan_courses WHERE plan_id = ?1",
                [plan.0],
                |r| {
                    Ok(CourseLink {
                        course_id: r.get(0)?,
                        version: r.get(1)?,
                        assigned: r.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    /// Every plan linked to a course in the panel.
    pub fn course_links(&self) -> Result<Vec<(PlanId, CourseLink)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT plan_id, course_id, version, assigned FROM plan_courses")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                PlanId(r.get(0)?),
                CourseLink {
                    course_id: r.get(1)?,
                    version: r.get(2)?,
                    assigned: r.get(3)?,
                },
            ))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn set_plan_course(&mut self, plan: PlanId, link: CourseLink) -> Result<()> {
        self.conn.execute(
            "INSERT INTO plan_courses (plan_id, course_id, version, assigned)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(plan_id) DO UPDATE SET course_id = excluded.course_id,
                 version = excluded.version, assigned = excluded.assigned",
            params![plan.0, link.course_id, link.version, link.assigned],
        )?;
        Ok(())
    }

    /// The plan's topics in order with their stable keys: the key received
    /// with the course, or `t<id>` for a plan made on this computer.
    pub fn topic_keys(&self, plan: PlanId) -> Result<Vec<(TopicId, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, COALESCE(k.key, 't' || t.id)
             FROM topics t
             JOIN subjects s ON s.id = t.subject_id
             LEFT JOIN topic_keys k ON k.topic_id = t.id
             WHERE s.plan_id = ?1
             ORDER BY s.position, t.position",
        )?;
        let rows = stmt.query_map([plan.0], |r| Ok((TopicId(r.get(0)?), r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn set_topic_keys(&mut self, keys: &[(TopicId, String)]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (topic, key) in keys {
            tx.execute(
                "INSERT INTO topic_keys (topic_id, key) VALUES (?1, ?2)
                 ON CONFLICT(topic_id) DO UPDATE SET key = excluded.key",
                params![topic.0, key],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The plan's progress keyed by the topics' stable keys.
    pub fn plan_progress(&self, plan: PlanId) -> Result<PlanProgress> {
        let keys: HashMap<i64, String> = self
            .topic_keys(plan)?
            .into_iter()
            .map(|(id, key)| (id.0, key))
            .collect();
        let key = |id: i64| keys.get(&id).cloned().unwrap_or_default();
        let in_plan = "JOIN topics t ON t.id = x.topic_id
                       JOIN subjects s ON s.id = t.subject_id
                       WHERE s.plan_id = ?1";
        let mut progress = PlanProgress::default();

        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.status, t.repetitions, t.interval_days, t.ease, t.due, t.last_reviewed
             FROM topics t JOIN subjects s ON s.id = t.subject_id WHERE s.plan_id = ?1",
        )?;
        let rows = stmt.query_map([plan.0], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                TopicProgress {
                    status: r.get(1)?,
                    repetitions: r.get(2)?,
                    interval_days: r.get(3)?,
                    ease: r.get::<_, f64>(4)? as f32,
                    due: r.get(5)?,
                    last_reviewed: r.get(6)?,
                },
            ))
        })?;
        for row in rows {
            let (id, topic) = row?;
            progress.topics.insert(key(id), topic);
        }

        let mut stmt = self.conn.prepare(&format!(
            "SELECT x.topic_id, x.taken_on, x.correct, x.total FROM quiz_attempts x {in_plan}
             ORDER BY x.id"
        ))?;
        let rows = stmt.query_map([plan.0], |r| {
            Ok(QuizRecord {
                topic: key(r.get(0)?),
                taken_on: r.get(1)?,
                correct: r.get(2)?,
                total: r.get(3)?,
            })
        })?;
        progress.quizzes = rows.collect::<std::result::Result<_, _>>()?;

        let mut stmt = self.conn.prepare(&format!(
            "SELECT x.topic_id, x.taken_on, x.outline, x.board, x.summary FROM lessons x {in_plan}
             ORDER BY x.id"
        ))?;
        let rows = stmt.query_map([plan.0], |r| {
            let list = |text: String| serde_json::from_str(&text).unwrap_or_default();
            Ok(LessonRecord {
                topic: key(r.get(0)?),
                taken_on: r.get(1)?,
                outline: list(r.get(2)?),
                board: list(r.get(3)?),
                summary: r.get(4)?,
            })
        })?;
        progress.lessons = rows.collect::<std::result::Result<_, _>>()?;

        let mut stmt = self.conn.prepare(&format!(
            "SELECT x.topic_id, x.reviewed_on FROM review_log x {in_plan} ORDER BY x.id"
        ))?;
        let rows = stmt.query_map([plan.0], |r| {
            Ok(ReviewRecord {
                topic: key(r.get(0)?),
                reviewed_on: r.get(1)?,
            })
        })?;
        progress.reviews = rows.collect::<std::result::Result<_, _>>()?;

        let mut stmt = self.conn.prepare(&format!(
            "SELECT x.topic_id, x.prompt, x.options, x.correct, x.explanation, x.from_board,
                    x.chosen, x.missed_on, x.misses
             FROM mistakes x {in_plan} ORDER BY x.id"
        ))?;
        let rows = stmt.query_map([plan.0], |r| {
            Ok(MistakeRecord {
                topic: key(r.get(0)?),
                prompt: r.get(1)?,
                options: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(),
                correct: r.get(3)?,
                explanation: r.get(4)?,
                from_board: r.get(5)?,
                chosen: r.get(6)?,
                missed_on: r.get(7)?,
                misses: r.get(8)?,
            })
        })?;
        progress.mistakes = rows.collect::<std::result::Result<_, _>>()?;
        Ok(progress)
    }

    /// Adds what the panel had and this computer did not. Items for topics
    /// the plan no longer has are left out.
    pub fn apply_plan_progress(&mut self, plan: PlanId, delta: &ProgressDelta) -> Result<()> {
        if delta.is_empty() {
            return Ok(());
        }
        let ids: HashMap<String, i64> = self
            .topic_keys(plan)?
            .into_iter()
            .map(|(id, key)| (key, id.0))
            .collect();
        let tx = self.conn.transaction()?;
        for (key, topic) in &delta.topics {
            let Some(id) = ids.get(key) else { continue };
            if crate::plan::TopicStatus::parse(&topic.status).is_none() {
                continue;
            }
            tx.execute(
                "UPDATE topics SET status = ?2, repetitions = ?3, interval_days = ?4, ease = ?5,
                        due = ?6, last_reviewed = ?7
                 WHERE id = ?1",
                params![
                    id,
                    topic.status,
                    topic.repetitions,
                    topic.interval_days,
                    f64::from(topic.ease),
                    topic.due,
                    topic.last_reviewed
                ],
            )?;
        }
        for quiz in &delta.quizzes {
            let Some(id) = ids.get(&quiz.topic) else {
                continue;
            };
            parse_date("taken_on", &quiz.taken_on)?;
            tx.execute(
                "INSERT INTO quiz_attempts (topic_id, taken_on, correct, total)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, quiz.taken_on, quiz.correct, quiz.total],
            )?;
        }
        for lesson in &delta.lessons {
            let Some(id) = ids.get(&lesson.topic) else {
                continue;
            };
            parse_date("taken_on", &lesson.taken_on)?;
            let json =
                |items: &[String]| serde_json::to_string(items).unwrap_or_else(|_| "[]".into());
            tx.execute(
                "INSERT INTO lessons (topic_id, taken_on, outline, board, summary)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id,
                    lesson.taken_on,
                    json(&lesson.outline),
                    json(&lesson.board),
                    lesson.summary
                ],
            )?;
        }
        for review in &delta.reviews {
            let Some(id) = ids.get(&review.topic) else {
                continue;
            };
            parse_date("reviewed_on", &review.reviewed_on)?;
            tx.execute(
                "INSERT OR IGNORE INTO review_log (topic_id, reviewed_on) VALUES (?1, ?2)",
                params![id, review.reviewed_on],
            )?;
        }
        for mistake in &delta.mistakes {
            let Some(id) = ids.get(&mistake.topic) else {
                continue;
            };
            parse_date("missed_on", &mistake.missed_on)?;
            let options = serde_json::to_string(&mistake.options).unwrap_or_else(|_| "[]".into());
            tx.execute(
                "INSERT INTO mistakes
                   (topic_id, prompt, options, correct, explanation, from_board, chosen,
                    missed_on, misses)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(topic_id, prompt) DO NOTHING",
                params![
                    id,
                    mistake.prompt,
                    options,
                    mistake.correct,
                    mistake.explanation,
                    mistake.from_board,
                    mistake.chosen,
                    mistake.missed_on,
                    mistake.misses
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Study time per day.
    pub fn overall_progress(&self) -> Result<OverallProgress> {
        let mut stmt = self
            .conn
            .prepare("SELECT day, milliseconds FROM study_time ORDER BY day")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut overall = OverallProgress::default();
        for row in rows {
            let (day, millis) = row?;
            overall
                .study_millis
                .insert(day, u64::try_from(millis).unwrap_or(0));
        }
        Ok(overall)
    }

    /// Adds the study time counted on other computers.
    pub fn apply_overall_progress(&mut self, extra: &[(String, u64)]) -> Result<()> {
        for (day, millis) in extra {
            let day = parse_date("day", day)?;
            self.add_study_millis(day, *millis)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::plan::{Difficulty, NewPlan, NewSubject, NewTopic, TopicStatus};
    use crate::quiz::QuizAttempt;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn plan(store: &mut Store) -> crate::plan::StudyPlan {
        let new = NewPlan::new(
            "ENEM",
            "ENEM",
            4,
            vec![NewSubject {
                name: "Matemática".into(),
                topics: vec![
                    NewTopic {
                        title: "Funções".into(),
                        difficulty: Difficulty::Medium,
                        week: 1,
                    },
                    NewTopic {
                        title: "Geometria".into(),
                        difficulty: Difficulty::Hard,
                        week: 2,
                    },
                ],
            }],
        )
        .unwrap()
        .with_purpose("national_exam");
        store
            .insert_plan(&new, "enem.pdf", "texto", day(1))
            .unwrap()
    }

    #[test]
    fn progress_moves_to_another_computer_by_topic_key() {
        let mut teacher = Store::open_in_memory().unwrap();
        let source = plan(&mut teacher);
        assert_eq!(source.purpose, "national_exam");
        let keys = teacher.topic_keys(source.id).unwrap();
        assert_eq!(keys[0].1, format!("t{}", keys[0].0.0));

        // The student's first computer: the course with the teacher's keys.
        let mut first = Store::open_in_memory().unwrap();
        let on_first = plan(&mut first);
        let first_ids: Vec<TopicId> = on_first.topics().map(|(_, t)| t.id).collect();
        let received: Vec<(TopicId, String)> = first_ids
            .iter()
            .zip(keys.iter())
            .map(|(id, (_, key))| (*id, key.clone()))
            .collect();
        first.set_topic_keys(&received).unwrap();
        first
            .record_quiz(&QuizAttempt {
                topic_id: first_ids[1],
                taken_on: day(3),
                correct: 4,
                total: 5,
            })
            .unwrap();
        let mut topic = on_first.topics().nth(1).unwrap().1.clone();
        topic.status = TopicStatus::Reviewing;
        first.update_topic(&topic).unwrap();
        first.add_study_millis(day(3), 60_000).unwrap();
        let cloud = first.plan_progress(on_first.id).unwrap();
        assert!(cloud.topics.contains_key(&keys[1].1));

        // A new computer gets the same course and restores the progress.
        let mut second = Store::open_in_memory().unwrap();
        let on_second = plan(&mut second);
        let second_ids: Vec<TopicId> = on_second.topics().map(|(_, t)| t.id).collect();
        let received: Vec<(TopicId, String)> = second_ids
            .iter()
            .zip(keys.iter())
            .map(|(id, (_, key))| (*id, key.clone()))
            .collect();
        second.set_topic_keys(&received).unwrap();
        let local = second.plan_progress(on_second.id).unwrap();
        second
            .apply_plan_progress(on_second.id, &local.missing_from(&cloud, true))
            .unwrap();
        let restored = second.load_plan(on_second.id).unwrap();
        assert_eq!(
            restored.topics().nth(1).unwrap().1.status,
            TopicStatus::Reviewing
        );
        assert_eq!(second.quiz_stats(on_second.id).unwrap().correct, 4);
        let extra = second
            .overall_progress()
            .unwrap()
            .missing_from(&first.overall_progress().unwrap());
        second.apply_overall_progress(&extra).unwrap();
        assert_eq!(second.study_millis(day(3)).unwrap(), 60_000);

        // Syncing again adds nothing twice.
        let again = second.plan_progress(on_second.id).unwrap();
        assert!(again.missing_from(&cloud, false).is_empty());
    }

    fn course(topics: &[(&str, &str)]) -> CoursePlan {
        CoursePlan {
            title: "ENEM".into(),
            exam: "ENEM".into(),
            purpose: "national_exam".into(),
            weeks: 4,
            syllabus_name: "matriz.pdf".into(),
            syllabus: "Matriz de referência".into(),
            subjects: vec![CourseSubject {
                name: "Matemática".into(),
                topics: topics
                    .iter()
                    .map(|(key, title)| CourseTopic {
                        key: (*key).into(),
                        title: (*title).into(),
                        difficulty: Difficulty::Medium,
                        week: 9,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn a_new_course_version_keeps_the_progress_of_topics_that_stay() {
        let mut store = Store::open_in_memory().unwrap();
        let plan = store
            .upsert_course_plan(
                None,
                &course(&[("t1", "Funções"), ("t2", "Geometria")]),
                day(1),
            )
            .unwrap();
        let loaded = store.load_plan(plan).unwrap();
        assert_eq!(loaded.purpose, "national_exam");
        assert_eq!(
            loaded.topics().next().unwrap().1.week,
            4,
            "weeks are clamped"
        );
        assert_eq!(store.edital_text(plan).unwrap(), "Matriz de referência");
        let funcoes = loaded.topics().next().unwrap().1.clone();
        let mut studied = funcoes.clone();
        studied.status = TopicStatus::Reviewing;
        store.update_topic(&studied).unwrap();

        let again = store
            .upsert_course_plan(
                Some(plan),
                &course(&[("t3", "Estatística"), ("t1", "Funções afins")]),
                day(2),
            )
            .unwrap();
        assert_eq!(again, plan);
        let updated = store.load_plan(plan).unwrap();
        let titles: Vec<&str> = updated.topics().map(|(_, t)| t.title.as_str()).collect();
        assert_eq!(titles, ["Estatística", "Funções afins"]);
        let kept = updated.find_topic(funcoes.id).unwrap().1;
        assert_eq!(kept.status, TopicStatus::Reviewing);
        let keys: Vec<String> = store
            .topic_keys(plan)
            .unwrap()
            .into_iter()
            .map(|(_, k)| k)
            .collect();
        assert_eq!(keys, ["t3", "t1"]);
    }

    #[test]
    fn a_plan_keeps_its_course_link() {
        let mut store = Store::open_in_memory().unwrap();
        let stored = plan(&mut store);
        let link = CourseLink {
            course_id: 7,
            version: 2,
            assigned: true,
        };
        store.set_plan_course(stored.id, link).unwrap();
        assert_eq!(store.plan_course(stored.id).unwrap(), Some(link));
        assert_eq!(store.course_links().unwrap(), vec![(stored.id, link)]);
    }
}
