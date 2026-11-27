//! Goals, days off, the profile and the unlock sound (Phase 11).

use std::collections::BTreeSet;

use chrono::NaiveDate;
use kg_study::goals::{Level, StudyGoals, WeekProgress, heat_map, suggest_goals, week_start};
use kg_study::{PlanId, TopicStatus};

use crate::{Engine, EngineError};

/// Weeks shown on the profile's calendar.
pub const HEAT_WEEKS: u32 = 26;
/// Medals a student can pin to the profile.
pub const SHOWCASE_SIZE: usize = 6;

/// The student's profile: level, chosen medals, study calendar and history
/// per subject of the active plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub level: Level,
    pub minutes: u64,
    pub study_days: usize,
    /// Pinned challenge keys, in order.
    pub showcase: Vec<String>,
    /// Minutes per day, Monday first, [`HEAT_WEEKS`] weeks.
    pub heat: Vec<(NaiveDate, u32)>,
    pub subjects: Vec<SubjectHistory>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubjectHistory {
    pub name: String,
    pub done: usize,
    pub topics: usize,
    pub lessons: u32,
    pub right: u32,
    pub answered: u32,
}

/// This week and the last one against the goals.
#[derive(Debug, Clone, PartialEq)]
pub struct GoalStatus {
    pub goals: StudyGoals,
    pub this_week: WeekProgress,
    /// Share of right answers in the plan's quizzes of the last 30 days.
    pub recent_quiz: Option<u8>,
    /// Last week fell short of a goal: a gentle note, nothing more.
    pub missed_last_week: bool,
}

impl Engine {
    /// The active plan's goals, if the student set them.
    pub fn study_goals(&self) -> Option<StudyGoals> {
        let plan = self.active_plan()?.id;
        self.store.study_goals(plan).ok().flatten()
    }

    /// Goals worth proposing for the active plan, from the exam date and
    /// the last four weeks.
    pub fn suggested_goals(&self) -> Option<StudyGoals> {
        let plan = self.active_plan()?;
        let today = Self::today();
        let exam = plan.created_on + chrono::Duration::weeks(i64::from(plan.weeks));
        let weeks_left = u32::try_from((exam - today).num_weeks().max(0)).unwrap_or(0);
        let left = plan
            .topics()
            .filter(|(_, t)| t.status != TopicStatus::Done)
            .count();
        let since = week_start(today) - chrono::Duration::weeks(4);
        let millis: u64 = self
            .store
            .overall_progress()
            .map(|p| {
                p.study_millis
                    .iter()
                    .filter(|(day, _)| {
                        NaiveDate::parse_from_str(day, "%Y-%m-%d")
                            .is_ok_and(|d| d >= since && d < week_start(today))
                    })
                    .map(|(_, ms)| *ms)
                    .sum()
            })
            .unwrap_or(0);
        let recent = millis as f32 / 3_600_000.0 / 4.0;
        Some(suggest_goals(left, weeks_left, recent, Some(exam)))
    }

    /// The goals and how this week goes, for Today.
    pub fn goal_status(&self) -> Option<GoalStatus> {
        let plan = self.active_plan()?.id;
        let goals = self.study_goals()?;
        let today = Self::today();
        let this = week_start(today);
        let last = this - chrono::Duration::weeks(1);
        let this_week = self.week_progress(plan, this);
        let previous = self.week_progress(plan, last);
        // A goal set this week says nothing about the last one.
        let set_before = self
            .store
            .study_goals_set_on(plan)
            .ok()
            .flatten()
            .is_some_and(|day| day < this);
        let since = (today - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let recent_quiz = self.store.plan_progress(plan).ok().and_then(|p| {
            let (right, total) = p
                .quizzes
                .iter()
                .filter(|q| q.taken_on.as_str() >= since.as_str())
                .fold((0u32, 0u32), |(r, t), q| (r + q.correct, t + q.total));
            (total > 0).then(|| u8::try_from(right * 100 / total).unwrap_or(100))
        });
        Some(GoalStatus {
            missed_last_week: set_before && !previous.met(&goals),
            goals,
            this_week,
            recent_quiz,
        })
    }

    /// Hours studied (all plans) and topics of `plan` studied in the week
    /// starting on `monday`.
    fn week_progress(&self, plan: PlanId, monday: NaiveDate) -> WeekProgress {
        let end = monday + chrono::Duration::days(7);
        let in_week = |day: &str| {
            day.get(..10)
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .is_some_and(|d| d >= monday && d < end)
        };
        let millis: u64 = self
            .store
            .overall_progress()
            .map(|p| {
                p.study_millis
                    .iter()
                    .filter(|(day, _)| in_week(day))
                    .map(|(_, ms)| *ms)
                    .sum()
            })
            .unwrap_or(0);
        let topics = self
            .store
            .topics_studied_between(plan, monday, end)
            .unwrap_or(0);
        WeekProgress {
            hours: millis as f32 / 3_600_000.0,
            topics,
        }
    }

    pub(crate) fn save_goals(&mut self, goals: Option<StudyGoals>) -> Result<(), EngineError> {
        let Some(plan) = self.active_plan().map(|p| p.id) else {
            return Ok(());
        };
        match goals {
            Some(goals) => self.store.save_study_goals(plan, &goals, Self::today())?,
            None => self.store.delete_study_goals(plan)?,
        }
        Ok(())
    }

    /// Days marked off (travel, illness), newest first.
    pub fn days_off(&self) -> Vec<NaiveDate> {
        let mut days: Vec<NaiveDate> = self
            .store
            .days_off()
            .unwrap_or_default()
            .into_iter()
            .collect();
        days.reverse();
        days
    }

    pub(crate) fn set_day_off(&mut self, day: NaiveDate, off: bool) -> Result<(), EngineError> {
        self.store.set_day_off(day, off)?;
        self.refresh_activity();
        Ok(())
    }

    pub(crate) fn days_off_set(&self) -> BTreeSet<NaiveDate> {
        self.store.days_off().unwrap_or_default()
    }

    pub(crate) fn set_showcase(&mut self, keys: Vec<String>) -> Result<(), EngineError> {
        let mut unique: Vec<String> = Vec::new();
        for key in keys {
            if !unique.contains(&key) && unique.len() < SHOWCASE_SIZE {
                unique.push(key);
            }
        }
        self.store.set_showcase(&unique)?;
        Ok(())
    }

    /// The student's profile.
    pub fn profile(&self) -> Profile {
        let today = Self::today();
        let overall = self.store.overall_progress().unwrap_or_default();
        let minutes = overall.study_millis.values().sum::<u64>() / 60_000;
        let study_days = overall.study_millis.values().filter(|ms| **ms > 0).count();
        let (mut right, mut lessons, mut mastered) = (0u64, 0u64, 0u64);
        for plan in self.plans() {
            right += u64::from(self.store.quiz_stats(plan.id).map_or(0, |s| s.correct));
            lessons += u64::from(self.store.lesson_count(plan.id).unwrap_or(0));
            if let Ok(study) = self.store.load_plan(plan.id) {
                mastered += study
                    .topics()
                    .filter(|(_, t)| t.status == TopicStatus::Done)
                    .count() as u64;
            }
        }
        let mut subjects = Vec::new();
        if let Some(plan) = self.active_plan() {
            let history = self.store.subject_history(plan.id).unwrap_or_default();
            for subject in &plan.subjects {
                let (lessons, right, answered) = history
                    .iter()
                    .find(|(id, ..)| *id == subject.id)
                    .map_or((0, 0, 0), |(_, l, r, a)| (*l, *r, *a));
                subjects.push(SubjectHistory {
                    name: subject.name.clone(),
                    done: subject
                        .topics
                        .iter()
                        .filter(|t| t.status == TopicStatus::Done)
                        .count(),
                    topics: subject.topics.len(),
                    lessons,
                    right,
                    answered,
                });
            }
        }
        Profile {
            level: Level::from_study(minutes, right, lessons, mastered),
            minutes,
            study_days,
            showcase: self.store.showcase().unwrap_or_default(),
            heat: heat_map(&overall.study_millis, today, HEAT_WEEKS),
            subjects,
        }
    }
}
