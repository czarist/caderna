//! Goals the student sets and the profile of their study (Phase 11): hours
//! and topics per week and a quiz score by a date, with values suggested
//! from the exam date and the student's pace; a level from study time and
//! results; and a streak that forgives a rest day per week and days off.
//!
//! Everything is computed in code from saved study; nothing here is an
//! online ranking or a comparison with other students.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};

/// The student's goals for one plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudyGoals {
    pub hours_per_week: f32,
    pub topics_per_week: u32,
    /// Share of right quiz answers wanted, in percent.
    pub quiz_percent: u8,
    /// When the quiz score should be reached; the exam date by default.
    pub quiz_by: Option<NaiveDate>,
}

/// One week measured against the goals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeekProgress {
    pub hours: f32,
    pub topics: u32,
}

impl WeekProgress {
    /// Both weekly goals reached.
    pub fn met(&self, goals: &StudyGoals) -> bool {
        self.hours + 0.01 >= goals.hours_per_week && self.topics >= goals.topics_per_week
    }
}

/// Realistic goals from what is left and how the student has been going:
/// the topics left spread over the weeks left, the recent weekly hours (or
/// an hour per topic for a new student), and 70 % right by the exam.
pub fn suggest_goals(
    topics_left: usize,
    weeks_left: u32,
    recent_hours_per_week: f32,
    exam: Option<NaiveDate>,
) -> StudyGoals {
    let weeks = weeks_left.max(1) as usize;
    let topics_per_week = u32::try_from(topics_left.div_ceil(weeks))
        .unwrap_or(u32::MAX)
        .clamp(1, 40);
    let by_topics = topics_per_week as f32;
    let hours = if recent_hours_per_week >= 1.0 {
        recent_hours_per_week.max(by_topics * 0.5)
    } else {
        by_topics
    };
    StudyGoals {
        hours_per_week: ((hours * 2.0).round() / 2.0).clamp(1.0, 40.0),
        topics_per_week,
        quiz_percent: 70,
        quiz_by: exam,
    }
}

/// Monday of the week holding `day`.
pub fn week_start(day: NaiveDate) -> NaiveDate {
    day - chrono::Duration::days(i64::from(day.weekday().num_days_from_monday()))
}

/// Days in a row with study ending today, forgiving: in each week (Monday
/// to Sunday) one day without study keeps the streak, and days marked off
/// (travel, illness) never break it. Today without study yet does not
/// break it either. `days` are the days with study, in any order.
pub fn forgiving_streak(days: &[NaiveDate], off: &BTreeSet<NaiveDate>, today: NaiveDate) -> u32 {
    let studied: BTreeSet<NaiveDate> = days.iter().copied().filter(|d| *d <= today).collect();
    let Some(&oldest) = studied.first() else {
        return 0;
    };
    let mut rested: HashSet<NaiveDate> = HashSet::new();
    let mut count = 0;
    let mut day = today;
    loop {
        if studied.contains(&day) {
            count += 1;
        } else if day == today || off.contains(&day) {
            // Today is not over; a day off is not a gap.
        } else if rested.insert(week_start(day)) {
            // This week's rest day.
        } else {
            break;
        }
        match day.checked_sub_days(Days::new(1)) {
            Some(previous) if previous >= oldest => day = previous,
            _ => break,
        }
    }
    count
}

/// The student's level from experience: level `n` needs `50·n·(n+1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub level: u32,
    pub experience: u64,
    /// Experience where this level started and where the next one starts.
    pub floor: u64,
    pub next: u64,
}

impl Level {
    /// Experience counts study and its results, never clicks: a point per
    /// minute studied, two per right quiz answer, ten per lesson and
    /// twenty-five per topic mastered.
    pub fn from_study(minutes: u64, right_answers: u64, lessons: u64, mastered: u64) -> Self {
        let experience = minutes + 2 * right_answers + 10 * lessons + 25 * mastered;
        let needed = |n: u64| 50 * n * (n + 1);
        let mut level = 0;
        while needed(level + 1) <= experience {
            level += 1;
        }
        Self {
            level: u32::try_from(level + 1).unwrap_or(u32::MAX),
            experience,
            floor: needed(level),
            next: needed(level + 1),
        }
    }

    /// Progress from this level to the next, `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        let span = self.next.saturating_sub(self.floor).max(1);
        (self.experience.saturating_sub(self.floor) as f32 / span as f32).clamp(0.0, 1.0)
    }
}

/// Minutes per day for the last `weeks` weeks ending with the week of
/// `today`, Monday first, from the milliseconds kept per ISO day.
pub fn heat_map(
    millis: &BTreeMap<String, u64>,
    today: NaiveDate,
    weeks: u32,
) -> Vec<(NaiveDate, u32)> {
    let last_monday = week_start(today);
    let first = last_monday - chrono::Duration::weeks(i64::from(weeks.saturating_sub(1)));
    let mut days = Vec::new();
    let mut day = first;
    while day <= last_monday + chrono::Duration::days(6) {
        let minutes = millis
            .get(&day.format("%Y-%m-%d").to_string())
            .map_or(0, |ms| u32::try_from(ms / 60_000).unwrap_or(u32::MAX));
        days.push((day, minutes));
        let Some(next) = day.succ_opt() else { break };
        day = next;
    }
    days
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap_or_default()
    }

    #[test]
    fn the_streak_forgives_one_rest_day_a_week_and_days_off() {
        // Weeks of Mon 2 and Mon 9 March 2026: Thu 5 and Wed 11 are each
        // their week's rest day.
        let studied: Vec<NaiveDate> = [2, 3, 4, 6, 7, 8, 9, 10, 12]
            .iter()
            .map(|day| d(3, *day))
            .collect();
        let none = BTreeSet::new();
        assert_eq!(forgiving_streak(&studied, &none, d(3, 12)), 9);
        // Today (Fri 13) not studied yet: still 9.
        assert_eq!(forgiving_streak(&studied, &none, d(3, 13)), 9);
        // Sat 14: Fri 13 takes the week's rest, so Wed 11 breaks it and
        // only Thu 12 counts.
        assert_eq!(forgiving_streak(&studied, &none, d(3, 14)), 1);
        // Unless it was marked as a day off.
        let off: BTreeSet<NaiveDate> = [d(3, 13)].into();
        assert_eq!(forgiving_streak(&studied, &off, d(3, 14)), 9);
        assert_eq!(forgiving_streak(&[], &none, d(3, 14)), 0);
    }

    #[test]
    fn goals_are_suggested_from_what_is_left() {
        let exam = Some(d(6, 30));
        let goals = suggest_goals(30, 10, 0.0, exam);
        assert_eq!(goals.topics_per_week, 3);
        assert_eq!(goals.hours_per_week, 3.0);
        assert_eq!(goals.quiz_percent, 70);
        assert_eq!(goals.quiz_by, exam);
        // A student already doing 6.3 h a week keeps that pace.
        assert_eq!(suggest_goals(30, 10, 6.3, exam).hours_per_week, 6.5);
        // No weeks left still asks for something.
        assert_eq!(suggest_goals(5, 0, 0.0, None).topics_per_week, 5);
        let week = WeekProgress {
            hours: 3.0,
            topics: 3,
        };
        assert!(week.met(&goals));
        assert!(
            !WeekProgress {
                hours: 2.0,
                topics: 5
            }
            .met(&goals)
        );
    }

    #[test]
    fn levels_grow_with_study_and_results() {
        let start = Level::from_study(0, 0, 0, 0);
        assert_eq!((start.level, start.floor, start.next), (1, 0, 100));
        let later = Level::from_study(90, 10, 1, 0);
        assert_eq!(later.experience, 120);
        assert_eq!(later.level, 2);
        assert!((later.progress() - 0.1).abs() < 1e-6, "120 of 100..300");
    }

    #[test]
    fn the_heat_map_covers_whole_weeks() {
        let mut millis = BTreeMap::new();
        millis.insert("2026-03-11".to_owned(), 45 * 60_000);
        let days = heat_map(&millis, d(3, 11), 2);
        assert_eq!(days.len(), 14);
        assert_eq!(days[0].0, d(3, 2));
        assert_eq!(
            days.iter().find(|(day, _)| *day == d(3, 11)).map(|x| x.1),
            Some(45)
        );
    }
}
