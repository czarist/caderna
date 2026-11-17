//! Live check of a classroom end to end against the brand's panel, with two
//! copies of the app on this computer: the teacher publishes a plan and its
//! class schedule to a classroom; the student's copy syncs and gets the
//! course and the agenda.
//!
//! ```sh
//! cargo run -p kg-engine --example classes_live -- <teacher code> <student code> [classroom name]
//! ```
//!
//! The student must belong to that classroom in the client area.

use std::time::{Duration, Instant};

use chrono::{NaiveTime, Weekday};
use kg_engine::{
    AiConfig, AppPaths, ClassTime, Command, Engine, HolidayCalendar, PanelStatus, ReleaseRule,
    ScheduleSpec,
};
use kg_study::{Difficulty, NewPlan, NewSubject, NewTopic, Store};

fn wait(engine: &mut Engine, seconds: u64, done: impl Fn(&Engine) -> bool) {
    let start = Instant::now();
    while !done(engine) && start.elapsed() < Duration::from_secs(seconds) {
        engine.poll();
        std::thread::sleep(Duration::from_millis(100));
    }
    for notice in engine.take_notices() {
        println!("  notice: {notice:?}");
    }
}

fn connected(engine: &Engine) -> bool {
    matches!(engine.panel_status(), PanelStatus::Connected { .. })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // RUST_LOG=info shows the engine's panel and sync log.
    tracing_subscriber::fmt::init();
    let mut args = std::env::args().skip(1);
    let usage = "usage: classes_live <teacher code> <student code> [classroom]";
    let teacher_code = args.next().ok_or(usage)?;
    let student_code = args.next().ok_or(usage)?;
    let room_name = args.next().unwrap_or_else(|| "Turma Teste".to_owned());

    // Teacher's copy, with a small plan of its own.
    let teacher_dir = tempfile::tempdir()?;
    let paths = AppPaths::new(teacher_dir.path());
    {
        let mut store = Store::open(&paths.database())?;
        let topic = |title: &str| NewTopic {
            title: title.into(),
            difficulty: Difficulty::Medium,
            week: 1,
        };
        let plan = NewPlan::new(
            "Curso ao vivo — Matemática",
            "ENEM",
            4,
            vec![NewSubject {
                name: "Matemática".into(),
                topics: vec![topic("Porcentagem"), topic("Funções"), topic("Estatística")],
            }],
        )?;
        store.insert_plan(&plan, "enem.pdf", "Matriz", Engine::today())?;
    }
    let mut teacher = Engine::new(paths, AiConfig::default())?;
    teacher.handle(Command::ActivateLicense(teacher_code))?;
    wait(&mut teacher, 60, connected);
    println!("teacher: {:?} {:?}", teacher.panel_status(), teacher.role());
    teacher.handle(Command::LoadClassrooms)?;
    wait(&mut teacher, 30, |e| !e.loading_classrooms());
    let room = teacher
        .classrooms()
        .unwrap_or_default()
        .iter()
        .find(|c| c.name == room_name)
        .map(|c| c.id)
        .ok_or("classroom not found")?;
    let plan = teacher.plans().first().ok_or("no plan")?.id;
    teacher.handle(Command::PublishPlan {
        plan,
        classrooms: vec![room],
    })?;
    wait(&mut teacher, 60, |e| e.publishing().is_none());
    println!("course: {:?}", teacher.course_link(plan));

    let today = Engine::today();
    let seven_pm = NaiveTime::from_hms_opt(19, 0, 0).ok_or("time")?;
    let spec = ScheduleSpec {
        start: today,
        end: today + chrono::Duration::days(60),
        classes: [Weekday::Mon, Weekday::Wed, Weekday::Fri]
            .map(|weekday| ClassTime {
                weekday,
                start: seven_pm,
                minutes: 60,
            })
            .to_vec(),
        blocked: Default::default(),
        holidays: HolidayCalendar::Brazil,
        review_every: 0,
        mock_exam: false,
    };
    teacher.handle(Command::GenerateSchedule {
        plan,
        classroom: room,
        spec: Box::new(spec),
    })?;
    let mut schedule = teacher
        .class_schedule(plan, room)
        .ok_or("schedule")?
        .schedule;
    schedule.release = ReleaseRule::AsClassesHappen;
    schedule.sessions[0].plan.note = "Tragam calculadora".into();
    schedule.sessions[0].plan.private_note = "não deve sair do computador".into();
    teacher.handle(Command::SaveSchedule {
        plan,
        classroom: room,
        schedule: Box::new(schedule),
    })?;
    teacher.handle(Command::PublishSchedule {
        plan,
        classroom: room,
    })?;
    wait(&mut teacher, 60, |e| e.publishing_schedule().is_none());
    println!(
        "schedule published: version {:?}",
        teacher
            .class_schedule(plan, room)
            .and_then(|s| s.published_version)
    );

    // Student's copy, in its own data folder.
    let student_dir = tempfile::tempdir()?;
    let mut student = Engine::new(AppPaths::new(student_dir.path()), AiConfig::default())?;
    student.handle(Command::ActivateLicense(student_code))?;
    wait(&mut student, 60, connected);
    println!("student: {:?} {:?}", student.panel_status(), student.role());
    wait(&mut student, 60, |e| !e.agenda(Engine::today()).is_empty());
    println!(
        "student sync: last {:?}, plans {}, schedules {}",
        student.last_class_sync(),
        student.plans().len(),
        student
            .plans()
            .iter()
            .map(|p| student.class_schedules(p.id).len())
            .sum::<usize>()
    );
    for item in student.agenda(Engine::today()) {
        println!(
            "agenda: {} {} {} — {} — note {:?}, private {:?}",
            item.plan_title,
            item.session.date,
            item.session.start,
            item.topics.join(", "),
            item.session.plan.note,
            item.session.plan.private_note
        );
    }
    if let Some(plan) = student.active_plan() {
        for (_, topic) in plan.topics() {
            println!(
                "  {}: opens {:?}",
                topic.title,
                student.topic_opens_on(topic.id)
            );
        }
    }

    for engine in [&mut teacher, &mut student] {
        engine.handle(Command::SignOutDevice)?;
        wait(engine, 30, |e| {
            e.panel_status() == PanelStatus::NotActivated
        });
    }
    Ok(())
}
