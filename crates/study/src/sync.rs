//! A student's progress as kept in the cloud (the brand's panel), so it
//! follows them to another computer.
//!
//! Progress on a plan received from a teacher is keyed by the topics' stable
//! keys (the teacher's topic ids), never by this computer's row ids. Two
//! computers may both have studied offline, so the panel's copy is merged
//! into the local one and the result sent back:
//!
//! - a topic takes the copy reviewed last (then the further status);
//! - quizzes, lessons and reviews are records: each one present on either
//!   side is kept once (as many copies as the side with most of them);
//! - the mistakes notebook comes from the panel only when the plan is
//!   restored on a new computer; afterwards this computer's notebook wins,
//!   so a mistake already cleared here does not come back;
//! - study time takes the longest of each day.
//!
//! Dates are ISO strings (`2026-11-10`).
//!
//! The work on a classroom's schedule (Phase W8: preparation and homework per
//! class) is not merged: it is computed again from the merged records and
//! the schedule before every upload, for the school's "Desempenho".

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::schedule::{ActivityKind, ClassSchedule, SessionKind, SessionStatus};

/// A topic's status and review schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicProgress {
    pub status: String,
    #[serde(default)]
    pub repetitions: u32,
    #[serde(default)]
    pub interval_days: u32,
    #[serde(default = "default_ease")]
    pub ease: f32,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub last_reviewed: Option<String>,
}

fn default_ease() -> f32 {
    2.5
}

impl TopicProgress {
    fn status_rank(&self) -> u8 {
        match self.status.as_str() {
            "studying" => 1,
            "reviewing" => 2,
            "done" => 3,
            _ => 0,
        }
    }

    /// Whether `self` is further along than `other`.
    fn ahead_of(&self, other: &Self) -> bool {
        (
            self.last_reviewed.as_deref(),
            self.status_rank(),
            self.repetitions,
        ) > (
            other.last_reviewed.as_deref(),
            other.status_rank(),
            other.repetitions,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuizRecord {
    pub topic: String,
    pub taken_on: String,
    pub correct: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonRecord {
    pub topic: String,
    pub taken_on: String,
    #[serde(default)]
    pub outline: Vec<String>,
    #[serde(default)]
    pub board: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub topic: String,
    pub reviewed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MistakeRecord {
    pub topic: String,
    pub prompt: String,
    pub options: Vec<String>,
    pub correct: u32,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub from_board: bool,
    pub chosen: u32,
    pub missed_on: String,
    #[serde(default = "one")]
    pub misses: u32,
}

fn one() -> u32 {
    1
}

/// Progress on one plan received from a teacher.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanProgress {
    #[serde(default)]
    pub topics: BTreeMap<String, TopicProgress>,
    #[serde(default)]
    pub quizzes: Vec<QuizRecord>,
    #[serde(default)]
    pub lessons: Vec<LessonRecord>,
    #[serde(default)]
    pub reviews: Vec<ReviewRecord>,
    #[serde(default)]
    pub mistakes: Vec<MistakeRecord>,
    /// Session key → the student's work for that class, computed by
    /// [`PlanProgress::schedule_work`]; never merged.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub homework: BTreeMap<String, SessionWork>,
}

/// What a student did around one class of the schedule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWork {
    /// Studied one of the class's topics on or before its day.
    #[serde(default)]
    pub prepared: bool,
    /// Homework items done on or after the class's day.
    #[serde(default)]
    pub homework_done: u32,
    #[serde(default)]
    pub homework_total: u32,
}

/// What `remote` has that `local` lacks, for [`crate::Store::apply_plan_progress`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgressDelta {
    /// Topics where the remote copy is further along.
    pub topics: Vec<(String, TopicProgress)>,
    pub quizzes: Vec<QuizRecord>,
    pub lessons: Vec<LessonRecord>,
    pub reviews: Vec<ReviewRecord>,
    /// Only when restoring a plan on a new computer.
    pub mistakes: Vec<MistakeRecord>,
}

impl ProgressDelta {
    pub fn is_empty(&self) -> bool {
        self.topics.is_empty()
            && self.quizzes.is_empty()
            && self.lessons.is_empty()
            && self.reviews.is_empty()
            && self.mistakes.is_empty()
    }
}

impl PlanProgress {
    /// What to add to `self` (this computer) from `remote` (the panel).
    /// `restoring`: the plan has just arrived on this computer.
    pub fn missing_from(&self, remote: &Self, restoring: bool) -> ProgressDelta {
        let topics = remote
            .topics
            .iter()
            .filter(|(key, theirs)| {
                self.topics
                    .get(*key)
                    .is_none_or(|ours| theirs.ahead_of(ours))
            })
            .map(|(key, theirs)| (key.clone(), theirs.clone()))
            .collect();
        let mistakes = if restoring {
            remote
                .mistakes
                .iter()
                .filter(|m| {
                    !self
                        .mistakes
                        .iter()
                        .any(|ours| ours.topic == m.topic && ours.prompt == m.prompt)
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        ProgressDelta {
            topics,
            quizzes: missing_records(&self.quizzes, &remote.quizzes),
            lessons: missing_records(&self.lessons, &remote.lessons),
            reviews: missing_records(&self.reviews, &remote.reviews),
            mistakes,
        }
    }

    /// The student's work on every class of `schedule` up to `today`:
    /// preparation (any lesson, quiz or review on the class's topics on or
    /// before its day) and homework (a quiz for a quiz, a lesson for a voice
    /// lesson, any study of the topic for the rest, on or after its day).
    pub fn schedule_work(
        &self,
        schedule: &ClassSchedule,
        today: NaiveDate,
    ) -> BTreeMap<String, SessionWork> {
        let studied = |topic: &str| {
            self.lessons
                .iter()
                .map(|l| (l.topic.as_str(), l.taken_on.as_str()))
                .chain(
                    self.quizzes
                        .iter()
                        .map(|q| (q.topic.as_str(), q.taken_on.as_str())),
                )
                .chain(
                    self.reviews
                        .iter()
                        .map(|r| (r.topic.as_str(), r.reviewed_on.as_str())),
                )
                .filter(move |(t, _)| *t == topic)
                .map(|(_, day)| day)
                .collect::<Vec<_>>()
        };
        let mut work = BTreeMap::new();
        for session in &schedule.sessions {
            if session.date > today || session.status == SessionStatus::Cancelled {
                continue;
            }
            let day = session.date.format("%Y-%m-%d").to_string();
            let prepared = session.kind != SessionKind::MockExam
                && session.topics.iter().any(|topic| {
                    studied(topic)
                        .iter()
                        .any(|d| d.get(..10) <= Some(day.as_str()))
                });
            let mut done = 0;
            for item in &session.plan.homework {
                let topics: Vec<&str> = match &item.topic {
                    Some(topic) => vec![topic.as_str()],
                    None => session.topics.iter().map(String::as_str).collect(),
                };
                let after = |d: &str| d.get(..10) >= Some(day.as_str());
                let finished = topics.iter().any(|topic| match item.kind {
                    ActivityKind::Quiz => self
                        .quizzes
                        .iter()
                        .any(|q| q.topic == *topic && after(&q.taken_on)),
                    ActivityKind::VoiceLesson => self
                        .lessons
                        .iter()
                        .any(|l| l.topic == *topic && after(&l.taken_on)),
                    _ => studied(topic).iter().any(|d| after(d)),
                });
                if finished {
                    done += 1;
                }
            }
            let total = u32::try_from(session.plan.homework.len()).unwrap_or(u32::MAX);
            if prepared || total > 0 {
                work.insert(
                    session.key.clone(),
                    SessionWork {
                        prepared,
                        homework_done: done,
                        homework_total: total,
                    },
                );
            }
        }
        work
    }

    /// Answered questions and right answers, for the teacher's view.
    pub fn quiz_totals(&self) -> (u32, u32) {
        self.quizzes
            .iter()
            .fold((0, 0), |(c, t), q| (c + q.correct, t + q.total))
    }
}

/// Records of `remote` beyond those `local` already has, counting copies.
fn missing_records<T: PartialEq + Clone>(local: &[T], remote: &[T]) -> Vec<T> {
    let mut missing = Vec::new();
    for (i, record) in remote.iter().enumerate() {
        let wanted = remote[..=i].iter().filter(|r| *r == record).count();
        let have = local.iter().filter(|r| *r == record).count()
            + missing.iter().filter(|r| *r == record).count();
        if have < wanted {
            missing.push(record.clone());
        }
    }
    missing
}

/// Progress that belongs to no plan: study time and active days.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverallProgress {
    /// Milliseconds studied per day.
    #[serde(default)]
    pub study_millis: BTreeMap<String, u64>,
}

impl OverallProgress {
    /// Days where `remote` counted more time than `self`, with the extra time.
    pub fn missing_from(&self, remote: &Self) -> Vec<(String, u64)> {
        remote
            .study_millis
            .iter()
            .filter_map(|(day, theirs)| {
                let ours = self.study_millis.get(day).copied().unwrap_or(0);
                (*theirs > ours).then(|| (day.clone(), theirs - ours))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiz(topic: &str, day: &str, correct: u32) -> QuizRecord {
        QuizRecord {
            topic: topic.into(),
            taken_on: day.into(),
            correct,
            total: 5,
        }
    }

    #[test]
    fn schedule_work_counts_preparation_and_homework() {
        use crate::schedule::{Activity, ScheduleSpec, Session, SessionPlan};
        let day = |d: u32| NaiveDate::from_ymd_opt(2026, 3, d).unwrap_or_default();
        let session = |key: &str, date: NaiveDate, topic: &str, homework: Vec<Activity>| Session {
            key: key.into(),
            date,
            start: chrono::NaiveTime::from_hms_opt(19, 0, 0).unwrap_or_default(),
            minutes: 60,
            kind: SessionKind::Class,
            topics: vec![topic.into()],
            status: SessionStatus::Planned,
            locked: false,
            plan: SessionPlan {
                homework,
                ..SessionPlan::default()
            },
            covered: None,
        };
        let quiz_on = |topic: &str| Activity {
            kind: ActivityKind::Quiz,
            minutes: 15,
            text: "Quiz".into(),
            topic: Some(topic.into()),
        };
        let schedule = ClassSchedule::new(
            ScheduleSpec {
                start: day(1),
                end: day(31),
                classes: Vec::new(),
                blocked: Default::default(),
                holidays: crate::schedule::HolidayCalendar::None,
                review_every: 0,
                mock_exam: false,
            },
            crate::schedule::Schedule {
                sessions: vec![
                    session("s1", day(2), "a", vec![quiz_on("a")]),
                    session("s2", day(9), "b", vec![quiz_on("b")]),
                    session("s3", day(30), "c", vec![quiz_on("c")]),
                ],
                ..Default::default()
            },
        );
        let progress = PlanProgress {
            // Prepared s1 the day before; did s1's quiz after class; did
            // s2's quiz before its class (preparation, not homework).
            quizzes: vec![
                quiz("a", "2026-03-01", 3),
                quiz("a", "2026-03-04", 4),
                quiz("b", "2026-03-08", 4),
            ],
            ..PlanProgress::default()
        };
        let work = progress.schedule_work(&schedule, day(10));
        assert_eq!(
            work["s1"],
            SessionWork {
                prepared: true,
                homework_done: 1,
                homework_total: 1
            }
        );
        assert_eq!(
            work["s2"],
            SessionWork {
                prepared: true,
                homework_done: 0,
                homework_total: 1
            }
        );
        assert!(!work.contains_key("s3"), "future classes are left out");
    }

    fn topic(status: &str, last: Option<&str>) -> TopicProgress {
        TopicProgress {
            status: status.into(),
            repetitions: 0,
            interval_days: 0,
            ease: 2.5,
            due: None,
            last_reviewed: last.map(str::to_owned),
        }
    }

    #[test]
    fn records_from_both_computers_are_kept_once() {
        let local = PlanProgress {
            quizzes: vec![quiz("t1", "2026-11-01", 3), quiz("t1", "2026-11-01", 3)],
            ..PlanProgress::default()
        };
        let remote = PlanProgress {
            quizzes: vec![
                quiz("t1", "2026-11-01", 3),
                quiz("t1", "2026-11-01", 3),
                quiz("t1", "2026-11-01", 3),
                quiz("t2", "2026-11-02", 5),
            ],
            ..PlanProgress::default()
        };
        let delta = local.missing_from(&remote, false);
        assert_eq!(
            delta.quizzes,
            vec![quiz("t1", "2026-11-01", 3), quiz("t2", "2026-11-02", 5)]
        );
        assert!(remote.missing_from(&local, false).quizzes.is_empty());
    }

    #[test]
    fn the_topic_reviewed_last_wins() {
        let mut local = PlanProgress::default();
        local
            .topics
            .insert("t1".into(), topic("reviewing", Some("2026-11-05")));
        local.topics.insert("t2".into(), topic("done", None));
        let mut remote = PlanProgress::default();
        remote
            .topics
            .insert("t1".into(), topic("reviewing", Some("2026-11-08")));
        remote.topics.insert("t2".into(), topic("studying", None));
        remote.topics.insert("t3".into(), topic("studying", None));
        let delta = local.missing_from(&remote, false);
        let keys: Vec<&str> = delta.topics.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["t1", "t3"]);
    }

    #[test]
    fn mistakes_come_back_only_when_restoring() {
        let mistake = MistakeRecord {
            topic: "t1".into(),
            prompt: "Q".into(),
            options: vec!["a".into(), "b".into()],
            correct: 0,
            explanation: String::new(),
            from_board: false,
            chosen: 1,
            missed_on: "2026-11-01".into(),
            misses: 1,
        };
        let remote = PlanProgress {
            mistakes: vec![mistake.clone()],
            ..PlanProgress::default()
        };
        let local = PlanProgress::default();
        assert!(local.missing_from(&remote, false).mistakes.is_empty());
        assert_eq!(local.missing_from(&remote, true).mistakes, vec![mistake]);
    }

    #[test]
    fn study_time_takes_the_longest_day() {
        let mut local = OverallProgress::default();
        local.study_millis.insert("2026-11-01".into(), 1_000);
        let mut remote = OverallProgress::default();
        remote.study_millis.insert("2026-11-01".into(), 4_000);
        remote.study_millis.insert("2026-11-02".into(), 500);
        assert_eq!(
            local.missing_from(&remote),
            vec![("2026-11-01".into(), 3_000), ("2026-11-02".into(), 500)]
        );
    }
}
