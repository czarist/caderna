//! Storage of class schedules (Phase W8): one per plan and classroom, with a
//! short history for undo.

use chrono::NaiveDateTime;
use rusqlite::{OptionalExtension, params};

use super::discursive::{from_json, json, parse_time, time};
use super::{Result, Store};
use crate::plan::PlanId;
use crate::schedule::ClassSchedule;

/// Undo steps kept per schedule.
const HISTORY: i64 = 30;

/// A schedule as saved, with what the app needs around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSchedule {
    pub id: i64,
    pub plan: PlanId,
    /// The panel's classroom id; 0 for a schedule not tied to a classroom.
    pub classroom: i64,
    pub schedule: ClassSchedule,
    /// The version last published to the panel, if any.
    pub published_version: Option<i64>,
    pub updated_at: NaiveDateTime,
}

impl Store {
    /// The schedule of `plan` for `classroom`, if there is one.
    pub fn class_schedule(&self, plan: PlanId, classroom: i64) -> Result<Option<SavedSchedule>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, schedule, published_version, updated_at FROM class_schedules
                 WHERE plan_id = ?1 AND classroom_id = ?2",
                params![plan.0, classroom],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(id, text, published_version, updated)| {
            Ok(SavedSchedule {
                id,
                plan,
                classroom,
                schedule: from_json("class_schedules.schedule", &text)?,
                published_version,
                updated_at: parse_time("class_schedules.updated_at", &updated)?,
            })
        })
        .transpose()
    }

    /// Every schedule of a plan, by classroom.
    pub fn class_schedules(&self, plan: PlanId) -> Result<Vec<SavedSchedule>> {
        let mut stmt = self.conn.prepare(
            "SELECT classroom_id FROM class_schedules WHERE plan_id = ?1 ORDER BY classroom_id",
        )?;
        let rooms: Vec<i64> = stmt
            .query_map([plan.0], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        rooms
            .into_iter()
            .filter_map(|room| self.class_schedule(plan, room).transpose())
            .collect()
    }

    /// Saves a schedule; the version it replaces goes to the undo history.
    pub fn save_class_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
        schedule: &ClassSchedule,
        now: NaiveDateTime,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let previous: Option<(i64, String)> = tx
            .query_row(
                "SELECT id, schedule FROM class_schedules WHERE plan_id = ?1 AND classroom_id = ?2",
                params![plan.0, classroom],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let text = json(schedule)?;
        match previous {
            Some((id, old)) => {
                if old != text {
                    tx.execute(
                        "INSERT INTO class_schedule_history (schedule_id, schedule, saved_at)
                         VALUES (?1, ?2, ?3)",
                        params![id, old, time(now)],
                    )?;
                    tx.execute(
                        "DELETE FROM class_schedule_history WHERE schedule_id = ?1 AND id NOT IN
                           (SELECT id FROM class_schedule_history WHERE schedule_id = ?1
                            ORDER BY id DESC LIMIT ?2)",
                        params![id, HISTORY],
                    )?;
                }
                tx.execute(
                    "UPDATE class_schedules SET schedule = ?1, updated_at = ?2 WHERE id = ?3",
                    params![text, time(now), id],
                )?;
            }
            None => {
                tx.execute(
                    "INSERT INTO class_schedules (plan_id, classroom_id, schedule, updated_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![plan.0, classroom, text, time(now)],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Goes back to the version before the last change; `false` when there
    /// is nothing to undo.
    pub fn undo_class_schedule(
        &mut self,
        plan: PlanId,
        classroom: i64,
        now: NaiveDateTime,
    ) -> Result<bool> {
        let tx = self.conn.transaction()?;
        let last: Option<(i64, i64, String)> = tx
            .query_row(
                "SELECT h.id, s.id, h.schedule FROM class_schedule_history h
                 JOIN class_schedules s ON s.id = h.schedule_id
                 WHERE s.plan_id = ?1 AND s.classroom_id = ?2
                 ORDER BY h.id DESC LIMIT 1",
                params![plan.0, classroom],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((history_id, schedule_id, text)) = last else {
            return Ok(false);
        };
        tx.execute(
            "UPDATE class_schedules SET schedule = ?1, updated_at = ?2 WHERE id = ?3",
            params![text, time(now), schedule_id],
        )?;
        tx.execute(
            "DELETE FROM class_schedule_history WHERE id = ?1",
            [history_id],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Records the version the panel accepted.
    pub fn set_class_schedule_published(
        &mut self,
        plan: PlanId,
        classroom: i64,
        version: i64,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE class_schedules SET published_version = ?1
             WHERE plan_id = ?2 AND classroom_id = ?3",
            params![version, plan.0, classroom],
        )?;
        Ok(())
    }

    pub fn delete_class_schedule(&mut self, plan: PlanId, classroom: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM class_schedules WHERE plan_id = ?1 AND classroom_id = ?2",
            params![plan.0, classroom],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, Weekday};

    use super::*;
    use crate::schedule::{ClassTime, HolidayCalendar, ScheduleSpec, TopicLoad, plan};

    fn plan_id(store: &mut Store) -> PlanId {
        use crate::plan::{Difficulty, NewPlan, NewSubject, NewTopic};
        let new = NewPlan::new(
            "Plano",
            "ENEM",
            4,
            vec![NewSubject {
                name: "Matemática".into(),
                topics: vec![NewTopic {
                    title: "Funções".into(),
                    difficulty: Difficulty::Medium,
                    week: 1,
                }],
            }],
        )
        .expect("plan");
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).expect("date");
        store
            .insert_plan(&new, "edital.pdf", "texto", today)
            .expect("insert")
            .id
    }

    fn sample(minutes: u32) -> ClassSchedule {
        let spec = ScheduleSpec {
            start: NaiveDate::from_ymd_opt(2026, 3, 2).expect("date"),
            end: NaiveDate::from_ymd_opt(2026, 3, 31).expect("date"),
            classes: vec![ClassTime {
                weekday: Weekday::Mon,
                start: NaiveTime::from_hms_opt(19, 0, 0).expect("time"),
                minutes,
            }],
            blocked: Default::default(),
            holidays: HolidayCalendar::Brazil,
            review_every: 0,
            mock_exam: false,
        };
        let topics = [TopicLoad {
            key: "t1".into(),
            minutes: 60,
        }];
        ClassSchedule::new(spec.clone(), plan(&spec, &topics))
    }

    fn now(minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 1)
            .and_then(|d| d.and_hms_opt(10, minute, 0))
            .expect("time")
    }

    #[test]
    fn schedules_save_per_classroom_and_undo_steps_back() {
        let mut store = Store::open_in_memory().expect("store");
        let plan = plan_id(&mut store);
        assert!(store.class_schedule(plan, 7).expect("read").is_none());

        store
            .save_class_schedule(plan, 7, &sample(100), now(0))
            .expect("save");
        store
            .save_class_schedule(plan, 8, &sample(50), now(1))
            .expect("save");
        store
            .save_class_schedule(plan, 7, &sample(90), now(2))
            .expect("save");

        let saved = store.class_schedule(plan, 7).expect("read").expect("some");
        assert_eq!(saved.schedule.spec.classes[0].minutes, 90);
        assert_eq!(store.class_schedules(plan).expect("all").len(), 2);

        assert!(store.undo_class_schedule(plan, 7, now(3)).expect("undo"));
        let back = store.class_schedule(plan, 7).expect("read").expect("some");
        assert_eq!(back.schedule.spec.classes[0].minutes, 100);
        assert!(!store.undo_class_schedule(plan, 7, now(4)).expect("undo"));

        store
            .set_class_schedule_published(plan, 7, 3)
            .expect("publish");
        let published = store.class_schedule(plan, 7).expect("read").expect("some");
        assert_eq!(published.published_version, Some(3));

        store.delete_class_schedule(plan, 8).expect("delete");
        assert!(store.class_schedule(plan, 8).expect("read").is_none());
    }
}
