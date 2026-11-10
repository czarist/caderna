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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
