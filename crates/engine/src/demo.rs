//! Demonstration builds: the brand (`[demo]` in `brand.toml`) can limit use
//! to some days from the first start, to a last date, or both.
//!
//! The first start and the latest day seen are kept in the database and in
//! a small file in the system's settings folder, and the earliest start and
//! latest day win: removing one of them, or turning the clock back, does not
//! restart the demonstration. When it ends the app shows only the "demo
//! ended" screen; plans and progress stay on the computer.

use std::path::PathBuf;

use chrono::{Days, NaiveDate};
use kg_study::Store;
use serde::{Deserialize, Serialize};

const CLOCK_KEY: &str = "demo_clock";
const CLOCK_FILE: &str = "demo.json";

/// Where this build stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DemoStatus {
    /// Not a demonstration build.
    #[default]
    Full,
    /// A demonstration with days left, counting today; `last_day` is the
    /// last day of use.
    Trial { days_left: u32, last_day: NaiveDate },
    /// The demonstration is over.
    Expired,
}

/// The demonstration's first start and the latest day it was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DemoClock {
    started: NaiveDate,
    last_seen: NaiveDate,
}

impl DemoClock {
    /// Joins two records: the earliest start and the latest day seen.
    fn merge(self, other: Self) -> Self {
        Self {
            started: self.started.min(other.started),
            last_seen: self.last_seen.max(other.last_seen),
        }
    }
}

/// The limits a brand sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DemoLimit {
    /// Days of use from the first start; 0 for none.
    days: u32,
    /// Last day of use.
    expires: Option<NaiveDate>,
}

impl DemoLimit {
    /// This build's limits, if it is a demonstration.
    pub(crate) fn of_brand() -> Option<Self> {
        if !kg_brand::DEMO {
            return None;
        }
        let expires = kg_brand::DEMO_EXPIRES
            .and_then(|(year, month, day)| NaiveDate::from_ymd_opt(year, month, day));
        Some(Self {
            days: kg_brand::DEMO_DAYS,
            expires,
        })
    }

    /// The status on `today`. A clock turned back counts as the latest day seen.
    pub(crate) fn status(self, clock: DemoClock, today: NaiveDate) -> DemoStatus {
        let now = today.max(clock.last_seen);
        let by_days = (self.days > 0)
            .then(|| {
                clock
                    .started
                    .checked_add_days(Days::new(u64::from(self.days) - 1))
            })
            .flatten();
        let last_day = match (by_days, self.expires) {
            (Some(a), Some(b)) => a.min(b),
            (Some(day), None) | (None, Some(day)) => day,
            // A demo without any limit cannot be built; treat it as over.
            (None, None) => return DemoStatus::Expired,
        };
        if now > last_day {
            return DemoStatus::Expired;
        }
        let left = (last_day - now).num_days() + 1;
        DemoStatus::Trial {
            days_left: u32::try_from(left).unwrap_or(u32::MAX),
            last_day,
        }
    }
}

/// The demonstration clock of this installation: read from the database and
/// `file` (see [`clock_file`]), moved to `today` and written back to both.
pub(crate) fn start(store: &mut Store, file: Option<PathBuf>, today: NaiveDate) -> DemoClock {
    let fresh = DemoClock {
        started: today,
        last_seen: today,
    };
    let saved = store
        .setting(CLOCK_KEY)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<DemoClock>(&json).ok());
    let kept = file
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|json| serde_json::from_str::<DemoClock>(&json).ok());
    let clock = [saved, kept]
        .into_iter()
        .flatten()
        .fold(fresh, DemoClock::merge);
    let clock = DemoClock {
        last_seen: clock.last_seen.max(today),
        ..clock
    };
    match serde_json::to_string(&clock) {
        Ok(json) => {
            if let Err(err) = store.set_setting(CLOCK_KEY, &json) {
                tracing::warn!(%err, "could not save the demo clock");
            }
            if let Some(path) = file {
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&path, &json));
                if let Err(err) = written {
                    tracing::warn!(%err, path = %path.display(), "could not save the demo clock");
                }
            }
        }
        Err(err) => tracing::warn!(%err, "could not encode the demo clock"),
    }
    clock
}

/// A copy of the clock outside the data folder (roaming settings on Windows).
pub(crate) fn clock_file() -> Option<PathBuf> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join(kg_brand::DATA_DIR).join(CLOCK_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn clock(started: &str, last_seen: &str) -> DemoClock {
        DemoClock {
            started: day(started),
            last_seen: day(last_seen),
        }
    }

    #[test]
    fn days_count_from_the_first_start_including_it() {
        let limit = DemoLimit {
            days: 14,
            expires: None,
        };
        let c = clock("2027-01-01", "2027-01-01");
        assert_eq!(
            limit.status(c, day("2027-01-01")),
            DemoStatus::Trial {
                days_left: 14,
                last_day: day("2027-01-14")
            }
        );
        assert!(matches!(
            limit.status(c, day("2027-01-14")),
            DemoStatus::Trial { days_left: 1, .. }
        ));
        assert_eq!(limit.status(c, day("2027-01-15")), DemoStatus::Expired);
    }

    #[test]
    fn the_earlier_of_days_and_date_ends_the_demo() {
        let limit = DemoLimit {
            days: 30,
            expires: Some(day("2027-01-10")),
        };
        let c = clock("2027-01-01", "2027-01-01");
        assert!(matches!(
            limit.status(c, day("2027-01-05")),
            DemoStatus::Trial { days_left: 6, .. }
        ));
        assert_eq!(limit.status(c, day("2027-01-11")), DemoStatus::Expired);
    }

    #[test]
    fn turning_the_clock_back_does_not_give_days_back() {
        let limit = DemoLimit {
            days: 7,
            expires: None,
        };
        let c = clock("2027-01-01", "2027-01-09");
        assert_eq!(limit.status(c, day("2027-01-02")), DemoStatus::Expired);
    }

    #[test]
    fn merged_records_keep_the_first_start_and_the_latest_day() {
        let a = clock("2027-01-05", "2027-01-06");
        let b = clock("2027-01-01", "2027-01-03");
        assert_eq!(a.merge(b), clock("2027-01-01", "2027-01-06"));
    }

    #[test]
    fn the_clock_is_kept_in_the_database() {
        let mut store = Store::open_in_memory().unwrap();
        let first = start(&mut store, None, day("2027-01-01"));
        let again = start(&mut store, None, day("2027-01-03"));
        assert_eq!(first, clock("2027-01-01", "2027-01-01"));
        assert_eq!(again, clock("2027-01-01", "2027-01-03"));
    }
}
