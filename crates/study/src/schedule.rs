//! A classroom's class schedule (Phase W8), computed in code without any AI:
//! the class days between two dates minus holidays and blocked days, the
//! plan's topics spread over them by weight, review sessions every few
//! classes, a mock exam before the end, and replanning that never moves a
//! session already given or locked.
//!
//! The AI only adds objectives, activities and homework to each session
//! afterwards, when the panel's AI is reachable; a session without them still
//! has its date, time and topics.

use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

use crate::plan::Difficulty;

/// Review session every this many class sessions, unless the teacher sets it.
pub const DEFAULT_REVIEW_EVERY: u32 = 6;

/// Holidays computed in code for a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HolidayCalendar {
    /// Brazil's national holidays, with Carnaval, Sexta-feira Santa and
    /// Corpus Christi from Easter. State and city holidays are blocked days.
    #[default]
    Brazil,
    /// No holidays; the teacher blocks days by hand.
    None,
}

/// One weekly class: on this weekday, at this time, for this long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassTime {
    pub weekday: Weekday,
    pub start: NaiveTime,
    pub minutes: u32,
}

/// What the teacher sets for a classroom's schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub start: NaiveDate,
    /// Last day with classes (the exam date or the end of the term).
    pub end: NaiveDate,
    pub classes: Vec<ClassTime>,
    /// State and city holidays, recess and any other day without class.
    #[serde(default)]
    pub blocked: BTreeSet<NaiveDate>,
    #[serde(default)]
    pub holidays: HolidayCalendar,
    /// A review session after every this many class sessions; 0 for none.
    #[serde(default = "default_review_every")]
    pub review_every: u32,
    /// The plan has an exam format: the last session becomes a mock exam.
    #[serde(default)]
    pub mock_exam: bool,
}

fn default_review_every() -> u32 {
    DEFAULT_REVIEW_EVERY
}

/// A topic to place, in the plan's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicLoad {
    /// The topic's stable key (the teacher's topic id).
    pub key: String,
    /// Class minutes the topic needs.
    pub minutes: u32,
}

impl TopicLoad {
    /// Minutes estimated from the difficulty, scaled by the notice's weight
    /// in percent (100 when the plan has none).
    pub fn estimate(key: impl Into<String>, difficulty: Difficulty, weight_percent: u32) -> Self {
        let base = match difficulty {
            Difficulty::Easy => 50,
            Difficulty::Medium => 80,
            Difficulty::Hard => 110,
        };
        Self {
            key: key.into(),
            minutes: (base * weight_percent.max(1)).div_ceil(100),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    #[default]
    Class,
    Review,
    MockExam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    #[default]
    Planned,
    Given,
    Cancelled,
}

/// One class on the calendar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Stable across replanning and republishing, so students' homework and
    /// progress stay attached to it.
    pub key: String,
    pub date: NaiveDate,
    pub start: NaiveTime,
    pub minutes: u32,
    pub kind: SessionKind,
    /// Topic keys, in order; a long topic appears in several sessions.
    pub topics: Vec<String>,
    #[serde(default)]
    pub status: SessionStatus,
    /// The teacher fixed it: replanning keeps it as it is.
    #[serde(default)]
    pub locked: bool,
    /// What happens in it: suggested by the AI when the panel's AI is
    /// reachable, then edited by the teacher. Empty until then.
    #[serde(default)]
    pub plan: SessionPlan,
}

/// Where a class happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "mode", content = "link")]
pub enum SessionMode {
    #[default]
    InPerson,
    /// Online, with an `https` link.
    Online(String),
    SelfStudy,
}

/// What a class activity is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Explanation,
    /// Opens the topic's voice lesson in the student's app.
    VoiceLesson,
    HandoutReading,
    Quiz,
    SpokenReview,
    WrittenAnswer,
    MockExam,
    Exercise,
    Other,
}

impl ActivityKind {
    /// Kinds the student can do alone and the app can tell were done.
    pub fn works_as_homework(self) -> bool {
        matches!(
            self,
            Self::VoiceLesson
                | Self::HandoutReading
                | Self::Quiz
                | Self::WrittenAnswer
                | Self::MockExam
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub kind: ActivityKind,
    pub minutes: u32,
    /// What to do, in the teacher's words.
    pub text: String,
    /// The topic it is about, when it opens something in the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// The teacher's plan for one class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionPlan {
    #[serde(default)]
    pub mode: SessionMode,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub activities: Vec<Activity>,
    #[serde(default)]
    pub materials: Vec<String>,
    /// Done by the students before the next class.
    #[serde(default)]
    pub homework: Vec<Activity>,
    /// Shown to the students.
    #[serde(default)]
    pub note: String,
    /// Only for the teacher; never published.
    #[serde(default)]
    pub private_note: String,
}

impl SessionPlan {
    pub fn is_empty(&self) -> bool {
        self.objectives.is_empty() && self.activities.is_empty() && self.homework.is_empty()
    }

    /// Minutes of the class activities.
    pub fn activity_minutes(&self) -> u32 {
        self.activities.iter().map(|a| a.minutes).sum()
    }
}

impl Session {
    fn kept_by_replan(&self) -> bool {
        self.locked || self.status == SessionStatus::Given
    }
}

/// When the students see a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseRule {
    /// Every topic from the start (the app's behaviour without a schedule).
    #[default]
    AllOpen,
    /// A topic opens on the day of its first class.
    AsClassesHappen,
}

/// A classroom's schedule as the teacher keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSchedule {
    pub spec: ScheduleSpec,
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub release: ReleaseRule,
    /// IANA time zone of the classroom.
    #[serde(default = "default_time_zone")]
    pub time_zone: String,
    /// Shortfall of the last planning, for the screen's warning.
    #[serde(default)]
    pub shortfall_minutes: u32,
    #[serde(default)]
    pub unplaced: Vec<String>,
}

fn default_time_zone() -> String {
    "America/Sao_Paulo".to_owned()
}

impl ClassSchedule {
    pub fn new(spec: ScheduleSpec, planned: Schedule) -> Self {
        Self {
            spec,
            sessions: planned.sessions,
            release: ReleaseRule::default(),
            time_zone: default_time_zone(),
            shortfall_minutes: planned.shortfall_minutes,
            unplaced: planned.unplaced,
        }
    }

    /// Topics the students may open on `today` under the release rule.
    pub fn topic_open(&self, topic: &str, today: NaiveDate) -> bool {
        match self.release {
            ReleaseRule::AllOpen => true,
            ReleaseRule::AsClassesHappen => {
                let first = self
                    .sessions
                    .iter()
                    .filter(|s| {
                        s.kind == SessionKind::Class && s.status != SessionStatus::Cancelled
                    })
                    .filter(|s| s.topics.iter().any(|t| t == topic))
                    .map(|s| s.date)
                    .min();
                // A topic with no class stays open: nothing says when to open it.
                first.is_none_or(|day| day <= today)
            }
        }
    }

    /// The next class on or after `today` that is not cancelled.
    pub fn next_session(&self, today: NaiveDate) -> Option<&Session> {
        self.sessions
            .iter()
            .filter(|s| s.date >= today && s.status == SessionStatus::Planned)
            .min_by_key(|s| (s.date, s.start))
    }

    /// The copy that goes to students: no private notes.
    pub fn for_students(&self) -> Self {
        let mut copy = self.clone();
        for session in &mut copy.sessions {
            session.plan.private_note.clear();
        }
        copy
    }
}

/// The result of planning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schedule {
    pub sessions: Vec<Session>,
    /// Class minutes the topics need beyond the sessions available; the
    /// teacher adds sessions, lengthens them or marks topics as self-study.
    pub shortfall_minutes: u32,
    /// Topics that got no session at all, in the plan's order.
    pub unplaced: Vec<String>,
}

/// Easter Sunday (Gregorian), by the anonymous computus.
pub fn easter(year: i32) -> Option<NaiveDate> {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, u32::try_from(month).ok()?, u32::try_from(day).ok()?)
}

/// The days without class of `calendar` in `year`.
pub fn holidays(calendar: HolidayCalendar, year: i32) -> BTreeSet<NaiveDate> {
    let mut days = BTreeSet::new();
    if calendar == HolidayCalendar::None {
        return days;
    }
    // Confraternização, Tiradentes, Trabalho, Independência, Aparecida,
    // Finados, República, Consciência Negra (national since 2024), Natal.
    let fixed = [
        (1, 1),
        (4, 21),
        (5, 1),
        (9, 7),
        (10, 12),
        (11, 2),
        (11, 15),
        (11, 20),
        (12, 25),
    ];
    days.extend(
        fixed
            .iter()
            .filter_map(|&(m, d)| NaiveDate::from_ymd_opt(year, m, d)),
    );
    if let Some(easter) = easter(year) {
        // Carnaval (Monday and Tuesday), Sexta-feira Santa, Corpus Christi.
        for offset in [-48, -47, -2, 60] {
            days.insert(easter + Duration::days(offset));
        }
    }
    days
}

/// The class slots of `spec`: every weekly class between start and end,
/// minus holidays and blocked days, in date and time order.
pub fn calendar(spec: &ScheduleSpec) -> Vec<(NaiveDate, NaiveTime, u32)> {
    let mut slots = Vec::new();
    if spec.end < spec.start {
        return slots;
    }
    let mut off: BTreeSet<NaiveDate> = spec.blocked.clone();
    for year in spec.start.year()..=spec.end.year() {
        off.extend(holidays(spec.holidays, year));
    }
    let mut day = spec.start;
    while day <= spec.end {
        if !off.contains(&day) {
            for class in spec.classes.iter().filter(|c| c.weekday == day.weekday()) {
                if class.minutes > 0 {
                    slots.push((day, class.start, class.minutes));
                }
            }
        }
        let Some(next) = day.succ_opt() else { break };
        day = next;
    }
    slots.sort();
    slots
}

/// Plans the whole schedule from scratch.
pub fn plan(spec: &ScheduleSpec, topics: &[TopicLoad]) -> Schedule {
    fill_from(calendar(spec), spec, topics, &mut KeyMaker::new(&[]), 0)
}

/// Plans again after a change (a session cancelled or moved, a topic that
/// took longer, new blocked days), from `today` on. Given and locked
/// sessions stay exactly as they are, their topics count as covered, and
/// the other sessions keep their keys in order.
pub fn replan(
    spec: &ScheduleSpec,
    topics: &[TopicLoad],
    previous: &[Session],
    today: NaiveDate,
) -> Schedule {
    let kept: Vec<Session> = previous
        .iter()
        .filter(|s| s.kept_by_replan())
        .cloned()
        .collect();
    let reusable: Vec<String> = previous
        .iter()
        .filter(|s| !s.kept_by_replan() && s.status != SessionStatus::Cancelled)
        .map(|s| s.key.clone())
        .collect();
    let mut keys = KeyMaker::new(previous);
    keys.reuse = reusable.into_iter().rev().collect();

    let covered: BTreeSet<&str> = kept
        .iter()
        .filter(|s| s.kind == SessionKind::Class)
        .flat_map(|s| s.topics.iter().map(String::as_str))
        .collect();
    let left: Vec<TopicLoad> = topics
        .iter()
        .filter(|t| !covered.contains(t.key.as_str()))
        .cloned()
        .collect();

    let taken: BTreeSet<(NaiveDate, NaiveTime)> = kept.iter().map(|s| (s.date, s.start)).collect();
    let slots: Vec<_> = calendar(spec)
        .into_iter()
        .filter(|(date, start, _)| *date >= today && !taken.contains(&(*date, *start)))
        .collect();
    let since_review = kept
        .iter()
        .rev()
        .take_while(|s| s.kind != SessionKind::Review)
        .filter(|s| s.kind == SessionKind::Class)
        .count();
    let mut schedule = fill_from(slots, spec, &left, &mut keys, since_review);
    // A session that kept its key and its topics keeps the teacher's plan.
    for session in &mut schedule.sessions {
        if let Some(old) = previous
            .iter()
            .find(|old| old.key == session.key && old.topics == session.topics)
        {
            session.plan = old.plan.clone();
        }
    }
    schedule.sessions.extend(kept);
    schedule.sessions.sort_by_key(|s| (s.date, s.start));
    schedule
}

/// Fills `slots` in order: a mock exam in the last one when asked, a review
/// after every `review_every` classes, and the topics' minutes poured into
/// the class sessions (a topic may run over several).
fn fill_from(
    mut slots: Vec<(NaiveDate, NaiveTime, u32)>,
    spec: &ScheduleSpec,
    topics: &[TopicLoad],
    keys: &mut KeyMaker,
    mut since_review: usize,
) -> Schedule {
    let mock = if spec.mock_exam && slots.len() > 1 {
        slots.pop()
    } else {
        None
    };
    let mut sessions = Vec::new();
    let mut queue = topics.iter().map(|t| (t.key.as_str(), t.minutes.max(1)));
    let mut current = queue.next();
    let every = spec.review_every as usize;

    for (date, start, minutes) in slots {
        let session = |kind, topics| Session {
            key: String::new(),
            date,
            start,
            minutes,
            kind,
            topics,
            status: SessionStatus::Planned,
            locked: false,
            plan: SessionPlan::default(),
        };
        if current.is_none() {
            break;
        }
        if every > 0 && since_review >= every {
            sessions.push(session(SessionKind::Review, Vec::new()));
            since_review = 0;
            continue;
        }
        let mut room = minutes;
        let mut here = Vec::new();
        while room > 0 {
            let Some((key, need)) = current else { break };
            here.push(key.to_owned());
            if need <= room {
                room -= need;
                current = queue.next();
            } else {
                current = Some((key, need - room));
                room = 0;
            }
        }
        sessions.push(session(SessionKind::Class, here));
        since_review += 1;
    }

    // Reviews cover the topics of the classes since the previous review.
    let mut recent: Vec<String> = Vec::new();
    for session in &mut sessions {
        match session.kind {
            SessionKind::Class => {
                for topic in &session.topics {
                    if !recent.contains(topic) {
                        recent.push(topic.clone());
                    }
                }
            }
            SessionKind::Review => session.topics = std::mem::take(&mut recent),
            SessionKind::MockExam => {}
        }
    }

    let mut shortfall = 0;
    let mut unplaced = Vec::new();
    if let Some((key, need)) = current {
        shortfall += need;
        let partly_placed = sessions.iter().any(|s| s.topics.iter().any(|t| t == key));
        if !partly_placed {
            unplaced.push(key.to_owned());
        }
        for (key, need) in queue {
            shortfall += need;
            unplaced.push(key.to_owned());
        }
    }

    if let Some((date, start, minutes)) = mock {
        let mut all: Vec<String> = Vec::new();
        for topic in sessions.iter().flat_map(|s| s.topics.iter()) {
            if !all.contains(topic) {
                all.push(topic.clone());
            }
        }
        sessions.push(Session {
            key: String::new(),
            date,
            start,
            minutes,
            kind: SessionKind::MockExam,
            topics: all,
            status: SessionStatus::Planned,
            locked: false,
            plan: SessionPlan::default(),
        });
    }

    for session in &mut sessions {
        session.key = keys.next();
    }
    Schedule {
        sessions,
        shortfall_minutes: shortfall,
        unplaced,
    }
}

/// Session keys: reused from the previous schedule first (in order), then
/// new ones above every key ever used, so a key never means two sessions.
struct KeyMaker {
    reuse: Vec<String>,
    next: u32,
}

impl KeyMaker {
    fn new(previous: &[Session]) -> Self {
        let highest = previous
            .iter()
            .filter_map(|s| s.key.strip_prefix('s')?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        Self {
            reuse: Vec::new(),
            next: highest + 1,
        }
    }

    fn next(&mut self) -> String {
        if let Some(key) = self.reuse.pop() {
            return key;
        }
        let key = format!("s{}", self.next);
        self.next += 1;
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("valid time")
    }

    fn mon_wed(start: NaiveDate, end: NaiveDate) -> ScheduleSpec {
        ScheduleSpec {
            start,
            end,
            classes: vec![
                ClassTime {
                    weekday: Weekday::Mon,
                    start: t(19, 0),
                    minutes: 100,
                },
                ClassTime {
                    weekday: Weekday::Wed,
                    start: t(19, 0),
                    minutes: 100,
                },
            ],
            blocked: BTreeSet::new(),
            holidays: HolidayCalendar::Brazil,
            review_every: 0,
            mock_exam: false,
        }
    }

    fn topics(minutes: &[u32]) -> Vec<TopicLoad> {
        minutes
            .iter()
            .enumerate()
            .map(|(i, &m)| TopicLoad {
                key: format!("t{}", i + 1),
                minutes: m,
            })
            .collect()
    }

    #[test]
    fn easter_matches_known_years() {
        assert_eq!(easter(2024), Some(d(2024, 3, 31)));
        assert_eq!(easter(2025), Some(d(2025, 4, 20)));
        assert_eq!(easter(2026), Some(d(2026, 4, 5)));
        assert_eq!(easter(2027), Some(d(2027, 3, 28)));
        assert_eq!(easter(2030), Some(d(2030, 4, 21)));
    }

    #[test]
    fn brazilian_holidays_include_the_movable_ones() {
        let days = holidays(HolidayCalendar::Brazil, 2026);
        // Carnaval 16–17 Feb, Sexta-feira Santa 3 Apr, Corpus Christi 4 Jun.
        for day in [
            d(2026, 2, 16),
            d(2026, 2, 17),
            d(2026, 4, 3),
            d(2026, 6, 4),
            d(2026, 11, 20),
            d(2026, 9, 7),
        ] {
            assert!(days.contains(&day), "{day}");
        }
        assert_eq!(days.len(), 13);
        assert!(holidays(HolidayCalendar::None, 2026).is_empty());
    }

    #[test]
    fn the_calendar_skips_holidays_and_blocked_days() {
        // Mondays and Wednesdays of February 2026: Carnaval Monday 16 is off,
        // and the teacher blocks Wednesday 4.
        let mut spec = mon_wed(d(2026, 2, 1), d(2026, 2, 28));
        spec.blocked.insert(d(2026, 2, 4));
        let dates: Vec<_> = calendar(&spec).iter().map(|s| s.0).collect();
        assert_eq!(
            dates,
            [
                d(2026, 2, 2),
                d(2026, 2, 9),
                d(2026, 2, 11),
                d(2026, 2, 18),
                d(2026, 2, 23),
                d(2026, 2, 25)
            ]
        );
        let backwards = mon_wed(d(2026, 3, 1), d(2026, 2, 1));
        assert!(calendar(&backwards).is_empty());
    }

    #[test]
    fn topics_fill_the_sessions_in_order_and_long_ones_span_several() {
        let spec = mon_wed(d(2026, 3, 2), d(2026, 3, 31));
        let schedule = plan(&spec, &topics(&[60, 60, 150, 30]));
        let topics: Vec<_> = schedule.sessions.iter().map(|s| s.topics.clone()).collect();
        assert_eq!(
            topics,
            [vec!["t1", "t2"], vec!["t2", "t3"], vec!["t3", "t4"],]
        );
        assert_eq!(schedule.shortfall_minutes, 0);
        assert!(schedule.unplaced.is_empty());
        let keys: Vec<_> = schedule.sessions.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["s1", "s2", "s3"]);
    }

    #[test]
    fn too_many_topics_report_the_shortfall_up_front() {
        // Two sessions of 100 minutes for 320 minutes of topics.
        let spec = mon_wed(d(2026, 3, 2), d(2026, 3, 4));
        let schedule = plan(&spec, &topics(&[80, 80, 80, 80]));
        assert_eq!(schedule.sessions.len(), 2);
        assert_eq!(schedule.shortfall_minutes, 120);
        assert_eq!(schedule.unplaced, ["t4"]);
    }

    #[test]
    fn reviews_come_every_few_classes_and_a_mock_exam_closes() {
        let mut spec = mon_wed(d(2026, 3, 2), d(2026, 4, 30));
        spec.review_every = 2;
        spec.mock_exam = true;
        let schedule = plan(&spec, &topics(&[100, 100, 100, 100]));
        let kinds: Vec<_> = schedule.sessions.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [
                SessionKind::Class,
                SessionKind::Class,
                SessionKind::Review,
                SessionKind::Class,
                SessionKind::Class,
                SessionKind::MockExam,
            ]
        );
        assert_eq!(schedule.sessions[2].topics, ["t1", "t2"]);
        assert_eq!(schedule.sessions[5].topics, ["t1", "t2", "t3", "t4"]);
        // The mock exam takes the last class day before the end.
        assert_eq!(schedule.sessions[5].date, d(2026, 4, 29));
    }

    #[test]
    fn replanning_keeps_given_and_locked_sessions_and_their_keys() {
        let spec = mon_wed(d(2026, 3, 2), d(2026, 4, 30));
        let first = plan(&spec, &topics(&[100, 100, 100, 100]));
        let mut previous = first.sessions.clone();
        previous[0].status = SessionStatus::Given;
        previous[1].status = SessionStatus::Cancelled;
        previous[2].locked = true;
        previous[2].date = d(2026, 3, 20);

        let today = d(2026, 3, 5);
        let again = replan(&spec, &topics(&[100, 100, 100, 100]), &previous, today);

        assert_eq!(again.sessions[0], previous[0], "given stays");
        assert!(again.sessions.contains(&previous[2]), "locked stays");
        let planned: Vec<_> = again
            .sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Planned && !s.locked)
            .collect();
        // t2 (cancelled) and t4 still need sessions, from today on; t1 and
        // t3 are covered by the kept ones.
        let topics: Vec<_> = planned.iter().map(|s| s.topics.clone()).collect();
        assert_eq!(topics, [vec!["t2"], vec!["t4"]]);
        assert!(planned.iter().all(|s| s.date >= today));
        // The fourth session's key is reused; the next one is new, never s2.
        let keys: Vec<_> = planned.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["s4", "s5"]);
        let mut sorted = again.sessions.clone();
        sorted.sort_by_key(|s| (s.date, s.start));
        assert_eq!(sorted, again.sessions);
    }

    #[test]
    fn topics_open_as_classes_happen_and_students_never_see_private_notes() {
        let spec = mon_wed(d(2026, 3, 2), d(2026, 3, 31));
        let mut schedule = ClassSchedule::new(spec.clone(), plan(&spec, &topics(&[100, 100, 100])));
        schedule.sessions[0].plan.private_note = "turma agitada".into();
        schedule.sessions[0].plan.note = "tragam calculadora".into();

        assert!(
            schedule.topic_open("t3", d(2026, 3, 1)),
            "all open by default"
        );
        schedule.release = ReleaseRule::AsClassesHappen;
        // t2 has its first class on Wednesday 4 March.
        assert!(!schedule.topic_open("t2", d(2026, 3, 3)));
        assert!(schedule.topic_open("t2", d(2026, 3, 4)));
        assert!(schedule.topic_open("no-class", d(2026, 3, 1)));

        assert_eq!(
            schedule.next_session(d(2026, 3, 3)).map(|s| s.date),
            Some(d(2026, 3, 4))
        );
        schedule.sessions[1].status = SessionStatus::Cancelled;
        assert_eq!(
            schedule.next_session(d(2026, 3, 3)).map(|s| s.date),
            Some(d(2026, 3, 9))
        );

        let shared = schedule.for_students();
        assert!(shared.sessions[0].plan.private_note.is_empty());
        assert_eq!(shared.sessions[0].plan.note, "tragam calculadora");
    }

    #[test]
    fn replanning_keeps_the_plan_of_a_session_with_the_same_topics() {
        let spec = mon_wed(d(2026, 3, 2), d(2026, 3, 31));
        let loads = topics(&[100, 100]);
        let mut previous = plan(&spec, &loads).sessions;
        previous[1].plan.objectives = vec!["Resolver equações".into()];
        let again = replan(&spec, &loads, &previous, d(2026, 3, 1));
        assert_eq!(again.sessions[1].plan.objectives, ["Resolver equações"]);
    }

    #[test]
    fn estimates_follow_difficulty_and_weight() {
        assert_eq!(TopicLoad::estimate("a", Difficulty::Easy, 100).minutes, 50);
        assert_eq!(TopicLoad::estimate("a", Difficulty::Hard, 200).minutes, 220);
        assert_eq!(TopicLoad::estimate("a", Difficulty::Medium, 0).minutes, 1);
    }

    #[test]
    fn the_spec_reads_with_defaults() {
        let spec: ScheduleSpec = serde_json::from_str(
            r#"{"start":"2026-03-02","end":"2026-06-30","classes":[{"weekday":"Mon","start":"19:00:00","minutes":100}]}"#,
        )
        .expect("spec");
        assert_eq!(spec.review_every, DEFAULT_REVIEW_EVERY);
        assert_eq!(spec.holidays, HolidayCalendar::Brazil);
        assert!(!spec.mock_exam);
    }
}
