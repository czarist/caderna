//! Live check of lesson planning (Phase W8) against the brand's panel: a
//! teacher's code activates a temporary data folder, a small plan is made
//! there, the calendar is built in code (no AI), then the AI (through the
//! panel's endpoint) plans the empty classes and the engine checks them.
//!
//! ```sh
//! cargo run -p kg-engine --example planning_live -- <teacher code>
//! ```

use std::time::{Duration, Instant};

use chrono::{NaiveDate, NaiveTime, Weekday};
use kg_engine::{
    AiConfig, AppPaths, ClassTime, Command, Engine, HolidayCalendar, Notice, PanelStatus,
    ScheduleSpec,
};
use kg_study::{Difficulty, NewPlan, NewSubject, NewTopic, Store};

fn wait(engine: &mut Engine, seconds: u64, done: impl Fn(&Engine) -> bool) -> Vec<Notice> {
    let start = Instant::now();
    while !done(engine) && start.elapsed() < Duration::from_secs(seconds) {
        engine.poll();
        std::thread::sleep(Duration::from_millis(100));
    }
    engine.take_notices()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = std::env::args()
        .nth(1)
        .ok_or("usage: planning_live <teacher code>")?;
    let dir = tempfile::tempdir()?;
    let paths = AppPaths::new(dir.path());
    let topic = |title: &str, difficulty| NewTopic {
        title: title.into(),
        difficulty,
        week: 1,
    };
    {
        let mut store = Store::open(&paths.database())?;
        let plan = NewPlan::new(
            "ENEM 2027 — Matemática",
            "ENEM",
            8,
            vec![NewSubject {
                name: "Matemática".into(),
                topics: vec![
                    topic("Razão e proporção", Difficulty::Easy),
                    topic("Porcentagem", Difficulty::Easy),
                    topic("Funções do 1º grau", Difficulty::Medium),
                    topic("Funções do 2º grau", Difficulty::Hard),
                    topic("Estatística básica", Difficulty::Medium),
                ],
            }],
        )?;
        store.insert_plan(&plan, "enem.pdf", "Matriz de referência", Engine::today())?;
    }

    let mut engine = Engine::new(paths, AiConfig::default())?;
    engine.handle(Command::ActivateLicense(code))?;
    for notice in wait(&mut engine, 60, |e| {
        matches!(e.panel_status(), PanelStatus::Connected { .. })
    }) {
        println!("  notice: {notice:?}");
    }
    println!(
        "panel: {:?}, role {:?}",
        engine.panel_status(),
        engine.role()
    );

    let plan = engine.plans().first().ok_or("no plan")?.id;
    let spec = ScheduleSpec {
        start: NaiveDate::from_ymd_opt(2027, 3, 1).ok_or("date")?,
        end: NaiveDate::from_ymd_opt(2027, 4, 30).ok_or("date")?,
        classes: vec![ClassTime {
            weekday: Weekday::Tue,
            start: NaiveTime::from_hms_opt(14, 0, 0).ok_or("time")?,
            minutes: 90,
        }],
        blocked: Default::default(),
        holidays: HolidayCalendar::Brazil,
        review_every: 3,
        mock_exam: true,
    };
    engine.handle(Command::GenerateSchedule {
        plan,
        classroom: 0,
        spec: Box::new(spec),
    })?;
    let saved = engine.class_schedule(plan, 0).ok_or("no schedule")?;
    println!(
        "calendar (no AI): {} classes",
        saved.schedule.sessions.len()
    );

    engine.handle(Command::SuggestSessionPlans { plan, classroom: 0 })?;
    for notice in wait(&mut engine, 300, |e| e.suggesting_sessions().is_none()) {
        println!("  notice: {notice:?}");
    }
    let titles = engine.schedule_topic_titles(plan);
    let saved = engine.class_schedule(plan, 0).ok_or("no schedule")?;
    for session in &saved.schedule.sessions {
        let topics: Vec<&str> = session
            .topics
            .iter()
            .map(|k| titles.get(k).map_or(k.as_str(), String::as_str))
            .collect();
        println!(
            "\n{} {:?} {} min — {}",
            session.date,
            session.kind,
            session.minutes,
            topics.join(", ")
        );
        for objective in &session.plan.objectives {
            println!("  objetivo: {objective}");
        }
        for activity in &session.plan.activities {
            println!(
                "  {:?} {} min: {}",
                activity.kind, activity.minutes, activity.text
            );
        }
        for homework in &session.plan.homework {
            println!(
                "  tarefa {:?} {} min: {}",
                homework.kind, homework.minutes, homework.text
            );
        }
    }
    Ok(())
}
