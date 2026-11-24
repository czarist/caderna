//! End-to-end check through the real screens: a new copy of the app is
//! activated, goes through the first run by clicking and typing as a person
//! would, opens a lesson from the Today screen, gets the tutor's first
//! answer and talks to it in the chat. Only the study plan is made by hand
//! (it needs an exam notice and the AI); everything else goes through the UI.
//!
//! With a brand that has a panel (the Caderna build `cargo test` uses here)
//! it needs that panel running and a test licence's access code:
//!
//! ```sh
//! KG_E2E_CODE=XXXX-XXXX-XXXX-XXXX cargo test -p kg-ui e2e -- --ignored --nocapture
//! ```
//!
//! The copy signs out at the end, freeing the seat. Without a GPU: frames
//! run, nothing is drawn to pixels. Voice is not covered.

use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use kg_engine::{AiConfig, AppPaths, Command, LessonPhase, PanelStatus, Speaker};
use kg_study::{Difficulty, NewPlan, NewSubject, NewTopic, Store};

use crate::app::{KnowGiverApp, Screen};

type App<'a> = Harness<'a, KnowGiverApp>;

fn frames(harness: &mut App<'_>) {
    harness.run_steps(6);
}

/// Runs frames until `done`, or fails after `limit`.
fn wait_for(harness: &mut App<'_>, what: &str, limit: Duration, done: impl Fn(&App<'_>) -> bool) {
    let start = Instant::now();
    while !done(harness) {
        assert!(
            start.elapsed() < limit,
            "timed out waiting for {what}; screen {:?}; panel {:?}; toasts {:?}; labels {:?}",
            harness.state().ui.screen,
            engine(harness).panel_status(),
            harness.state().toast_texts(),
            labels(harness)
        );
        harness.run_steps(2);
        std::thread::sleep(Duration::from_millis(100));
    }
    frames(harness);
}

/// Clicks the first widget with this label, like a pointer would.
fn click(harness: &mut App<'_>, label: &str) {
    let found = harness.query_all_by_label(label).next().is_some();
    assert!(
        found,
        "no {label:?} on screen {:?}; labels: {:?}",
        harness.state().ui.screen,
        labels(harness)
    );
    if let Some(node) = harness.query_all_by_label(label).next() {
        node.click();
    }
    frames(harness);
}

/// Clicks the first text field and types into it.
fn type_in_field(harness: &mut App<'_>, text: &str) {
    let found = harness
        .query_all_by(|n| matches!(n.role(), Role::TextInput | Role::MultilineTextInput))
        .next()
        .is_some();
    assert!(found, "no text field; labels: {:?}", labels(harness));
    if let Some(node) = harness
        .query_all_by(|n| matches!(n.role(), Role::TextInput | Role::MultilineTextInput))
        .next()
    {
        node.click();
    }
    frames(harness);
    if let Some(node) = harness
        .query_all_by(|n| matches!(n.role(), Role::TextInput | Role::MultilineTextInput))
        .next()
    {
        node.type_text(text);
    }
    frames(harness);
}

fn labels(harness: &App<'_>) -> Vec<String> {
    harness
        .query_all_by(|node| {
            node.label()
                .or_else(|| node.value())
                .is_some_and(|l| !l.is_empty())
        })
        .filter_map(|node| {
            let node = node.accesskit_node();
            node.label().or_else(|| node.value())
        })
        .take(60)
        .collect()
}

fn engine<'a>(harness: &'a App<'_>) -> &'a kg_engine::Engine {
    harness
        .state()
        .engine
        .as_ref()
        .unwrap_or_else(|err| panic!("engine did not start: {err}"))
}

/// A plan with one topic and no notice, as a classroom's course arrives.
fn seed_plan(paths: &AppPaths) {
    std::fs::create_dir_all(&paths.data_dir).unwrap();
    let mut store = Store::open(&paths.database()).unwrap();
    let plan = NewPlan::new(
        "Plano TJCE",
        "Tribunal de Justiça do Ceará",
        10,
        vec![NewSubject {
            name: "Língua Portuguesa".into(),
            topics: vec![NewTopic {
                title: "Emprego do sinal indicativo de crase".into(),
                difficulty: Difficulty::Medium,
                week: 1,
            }],
        }],
    )
    .unwrap();
    let today = chrono::Local::now().date_naive();
    store
        // No notice, like a plan from a classroom's course.
        .insert_plan(&plan, "", "", today)
        .unwrap();
}

#[test]
#[ignore = "needs the brand's panel and KG_E2E_CODE; see the module docs"]
fn a_new_student_gets_from_activation_to_a_lesson() {
    activated(|harness| {
        first_run(harness);
        handout_and_lesson(harness);
    });
}

/// Developer mode (Phase W9): the student and teacher views, and
/// presentation mode with the brand of a client registered in the panel.
/// Needs `KG_E2E_DEV_CODE`, a developer code made in the panel; the token is
/// saved in the build's standard data folder, whose `developer.json` is put
/// back as it was afterwards.
#[test]
#[ignore = "needs the brand's panel, KG_E2E_CODE and KG_E2E_DEV_CODE; see the module docs"]
fn a_developer_switches_views_and_presents_a_clients_brand() {
    let code = std::env::var("KG_E2E_DEV_CODE").expect("KG_E2E_DEV_CODE: a developer code");
    let file = AppPaths::standard().data_dir.join("developer.json");
    let saved = std::fs::read(&file).ok();
    let result = std::panic::catch_unwind(|| {
        activated(|harness| {
            first_run(harness);
            developer_tools(harness, &code);
        });
    });
    match saved {
        Some(bytes) => std::fs::write(&file, bytes).unwrap(),
        None => {
            let _ = std::fs::remove_file(&file);
        }
    }
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

/// A new copy of the app, activated with `KG_E2E_CODE` when the brand has a
/// panel; signs out afterwards, whatever `body` did.
fn activated(body: impl FnOnce(&mut App<'_>)) {
    // The app's log, on the test's output (RUST_LOG, default info).
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,wgpu=warn,naga=warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let data = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(data.path());
    seed_plan(&paths);
    crate::i18n::set("pt-BR");
    let mut harness = Harness::builder()
        .with_size([1280.0, 900.0])
        .build_eframe(|cc| KnowGiverApp::with_paths(cc, paths.clone(), AiConfig::default()));
    frames(&mut harness);
    assert_eq!(harness.state().ui.screen, Screen::Setup);

    // 0. A brand with a panel opens on the licence: type the code, activate.
    if kg_brand::has_server() {
        let code = std::env::var("KG_E2E_CODE").expect("KG_E2E_CODE: a test licence's access code");
        assert!(
            harness
                .query_all_by_label(&t!("licensing.activate"))
                .next()
                .is_some(),
            "a new copy asks for the licence first"
        );
        type_in_field(&mut harness, &code);
        click(&mut harness, &t!("licensing.activate"));
        wait_for(
            &mut harness,
            "the activation",
            Duration::from_secs(30),
            |h| matches!(engine(h).panel_status(), PanelStatus::Connected { .. }),
        );
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        body(&mut harness);
    }));
    // Free the seat whatever happened.
    if kg_brand::has_server() {
        harness.state_mut().dispatch(Command::SignOutDevice);
        wait_for(&mut harness, "the sign-out", Duration::from_secs(20), |h| {
            !matches!(
                engine(h).panel_status(),
                PanelStatus::Busy | PanelStatus::Connected { .. }
            )
        });
    }
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

fn first_run(harness: &mut App<'_>) {
    // 1. Language (the region comes with it).
    click(harness, &crate::i18n::language_name("pt-BR"));
    click(harness, &t!("setup.next"));

    // 2. Name, and student or teacher (the test licence has no enrolment,
    //    so the choice is free).
    type_in_field(harness, "Ana");
    click(harness, &t!("setup.role.student"));
    click(harness, &t!("setup.next"));

    // 3. Teacher.
    click(
        harness,
        &crate::i18n::teacher_name(kg_engine::Teacher::ALL[0]),
    );
    click(harness, &t!("setup.next"));

    // 4. AI: the brand's panel provides it.
    click(harness, &t!("setup.next"));

    // 5. Terms.
    click(harness, &t!("setup.terms.agree"));
    click(harness, &t!("setup.finish"));
    assert!(!engine(harness).needs_setup(), "the first run was saved");
    assert_eq!(engine(harness).settings().student_name, "Ana");
    assert_eq!(
        engine(harness).settings().role,
        Some(kg_engine::UseRole::Student)
    );
    assert_eq!(engine(harness).view_role(), kg_engine::UseRole::Student);
    assert!(
        harness
            .query_all_by_label(&t!("nav.planning"))
            .next()
            .is_none(),
        "no lesson planning in the student view"
    );
    assert_ne!(harness.state().ui.screen, Screen::Setup);

    // The tour comes next; skip it.
    click(harness, &t!("tour.skip"));
    assert!(!engine(harness).needs_tour());
    harness.state_mut().clear_toasts();
    frames(harness);
}

fn handout_and_lesson(harness: &mut App<'_>) {
    // The topic's handout, from the study plan.
    assert_eq!(harness.state().ui.screen, Screen::Today);
    click(harness, &t!("nav.plan"));
    assert_eq!(harness.state().ui.screen, Screen::Plan);
    click(harness, &t!("plan.handout"));
    assert!(
        engine(harness).handout_job().is_some(),
        "the handout started; toasts {:?}",
        harness.state().toast_texts()
    );
    wait_for(harness, "the handout", Duration::from_secs(900), |h| {
        engine(h).handout_job().is_none()
    });
    eprintln!(
        "after the handout: toasts {:?}",
        harness.state().toast_texts()
    );
    assert_eq!(
        engine(harness).handouts_cached(),
        1,
        "the handout was written; toasts {:?}",
        harness.state().toast_texts()
    );
    harness.state_mut().clear_toasts();
    click(harness, &t!("nav.today"));

    // Today offers the plan's first lesson.
    assert_eq!(harness.state().ui.screen, Screen::Today);
    click(harness, &t!("lesson.start"));
    assert_eq!(harness.state().ui.screen, Screen::Lesson);
    // The transcript and chat stay open for the whole lesson.
    assert!(
        harness
            .query_all_by(|n| n.role() == Role::TextInput)
            .next()
            .is_some(),
        "the chat field is on screen"
    );

    // The tutor's first answer, from the AI through the panel.
    let tutor_turns = |h: &App<'_>| {
        engine(h).lesson().map_or(0, |l| {
            l.turns
                .iter()
                .filter(|t| t.speaker == Speaker::Tutor)
                .count()
        })
    };
    wait_for(
        harness,
        "the tutor's first answer",
        Duration::from_secs(120),
        |h| {
            tutor_turns(h) >= 1
                && engine(h)
                    .lesson()
                    .is_some_and(|l| !matches!(l.phase, LessonPhase::Thinking))
        },
    );
    let first = engine(harness)
        .lesson()
        .and_then(|l| l.turns.last())
        .map(|t| t.text.clone())
        .unwrap_or_default();
    eprintln!("tutor: {first}");
    assert!(!first.trim().is_empty());

    // Typed chat: ask something and get an answer.
    type_in_field(harness, "Quando a crase é obrigatória?");
    click(harness, &t!("lesson.send"));
    assert!(
        engine(harness).lesson().is_some_and(|l| l
            .turns
            .iter()
            .any(|t| t.speaker == Speaker::Student && t.text.contains("crase"))),
        "the typed question is in the transcript"
    );
    wait_for(
        harness,
        "the tutor's reply to the chat",
        Duration::from_secs(120),
        |h| {
            tutor_turns(h) >= 2
                && engine(h)
                    .lesson()
                    .is_some_and(|l| !matches!(l.phase, LessonPhase::Thinking))
        },
    );
    let reply = engine(harness)
        .lesson()
        .and_then(|l| l.turns.last())
        .map(|t| t.text.clone())
        .unwrap_or_default();
    eprintln!("tutor: {reply}");
    let start: String = reply.chars().take(30).collect();
    assert!(
        harness
            .query_all_by(|n| {
                n.label()
                    .or_else(|| n.value())
                    .is_some_and(|l| l.contains(start.as_str()))
            })
            .next()
            .is_some(),
        "the reply shows in the chat panel; labels {:?}",
        labels(harness)
    );
}

fn developer_tools(harness: &mut App<'_>, code: &str) {
    let strip = crate::brand::channel_label();
    // The hidden area: seven clicks on the version in Settings.
    click(harness, &t!("nav.settings"));
    let version = t!("app.version", version = env!("CARGO_PKG_VERSION")).into_owned();
    // The sidebar shows the version too; the one in Settings is the last.
    if let Some(node) = harness.query_all_by_label(&version).last() {
        node.scroll_to_me();
    }
    frames(harness);
    for _ in 0..crate::screens::developer::CLICKS {
        if let Some(node) = harness.query_all_by_label(&version).last() {
            node.click();
        }
        frames(harness);
    }
    assert!(
        harness
            .query_all_by_label(&t!("developer.use_code"))
            .next()
            .is_some(),
        "the developer area opened; screen {:?}; labels {:?}",
        harness.state().ui.screen,
        labels(harness)
    );
    type_in_field_labeled(harness, "DEV-XXXX-XXXX-XXXX", code);
    click(harness, &t!("developer.use_code"));
    wait_for(
        harness,
        "the developer token",
        Duration::from_secs(30),
        |h| {
            engine(h)
                .developer_status()
                .is_some_and(|s| s.claims.is_some())
        },
    );

    // The teacher view, then the student view, whatever the licence says.
    click(harness, &t!("developer.view_teacher"));
    assert_eq!(engine(harness).view_role(), kg_engine::UseRole::Teacher);
    assert!(
        harness
            .query_all_by_label(&t!("nav.planning"))
            .next()
            .is_some(),
        "lesson planning in the teacher view"
    );
    click(harness, &t!("developer.view_student"));
    assert_eq!(engine(harness).view_role(), kg_engine::UseRole::Student);
    assert!(
        harness
            .query_all_by_label(&t!("nav.planning"))
            .next()
            .is_none(),
        "no lesson planning in the student view"
    );
    click(harness, &t!("developer.view_licence"));
    assert_eq!(engine(harness).developer_view(), None);

    // Presentation mode: the panel's clients, then one client's brand.
    click(harness, &t!("developer.clients_load"));
    wait_for(harness, "the clients", Duration::from_secs(30), |h| {
        engine(h).presentation_clients().is_some()
    });
    let clients = engine(harness).presentation_clients().unwrap().to_vec();
    eprintln!("clients: {clients:?}");
    let client = clients
        .iter()
        .find(|c| c.name.starts_with("Escola Teste"))
        .expect("the test client has a brand")
        .clone();
    // The list starts on the first client (sorted by name): the test one.
    assert_eq!(clients[0].id, client.id, "the test client comes first");
    click(harness, &t!("developer.present"));
    wait_for(
        harness,
        "the client's brand",
        Duration::from_secs(60),
        |h| engine(h).presenting() == Some(client.id),
    );
    frames(harness);
    let brand = engine(harness).panel_brand().cloned().unwrap();
    assert_eq!(brand.app_name.as_deref(), Some("Escola Teste"));
    for file in [
        "icon.png",
        "cover.png",
        "engraving.png",
        "mark.svg",
        "watermark.svg",
    ] {
        assert!(brand.image(file).is_some(), "{file} came with the brand");
    }
    assert_eq!(
        crate::brand::name(),
        "Escola Teste",
        "the UI shows the client's name"
    );
    assert_eq!(
        crate::brand::channel_label(),
        None,
        "nothing shows it is a development run"
    );

    click(harness, &t!("developer.present_stop"));
    assert_eq!(engine(harness).presenting(), None);
    frames(harness);
    assert_eq!(
        crate::brand::channel_label(),
        strip,
        "the channel strip is back"
    );

    // The same views from Settings; the teacher checks lessons and quizzes.
    // The developer window still covers the top of the screen, so these
    // buttons are pressed through accessibility, not the pointer.
    click(harness, &t!("nav.settings"));
    let press = |harness: &mut App<'_>, label: &str| {
        if let Some(node) = harness.query_all_by_label(label).next() {
            node.click_accesskit();
        }
        frames(harness);
    };
    press(harness, &t!("settings.developer.teacher"));
    assert_eq!(
        engine(harness).developer_view(),
        Some(kg_engine::UseRole::Teacher)
    );
    for label in [t!("nav.lesson_check"), t!("nav.quiz_check")] {
        assert!(
            harness.query_all_by_label(&label).next().is_some(),
            "{label} in the teacher's sidebar"
        );
    }
    press(harness, &t!("settings.developer.student"));
    assert_eq!(
        engine(harness).developer_view(),
        Some(kg_engine::UseRole::Student)
    );
    assert!(
        harness
            .query_all_by_label(&t!("nav.lesson_check"))
            .next()
            .is_none()
            && harness
                .query_all_by_label(&t!("nav.lesson"))
                .next()
                .is_some(),
        "the student takes lessons"
    );
    press(harness, &t!("settings.developer.licence"));
    assert_eq!(engine(harness).developer_view(), None);
}

/// Types into the text field showing `hint`.
fn type_in_field_labeled(harness: &mut App<'_>, hint: &str, text: &str) {
    let field = |h: &App<'_>| {
        h.query_all_by(|n| {
            matches!(n.role(), Role::TextInput)
                && (n.placeholder().is_some_and(|p| p == hint)
                    || n.label().is_some_and(|l| l == hint))
        })
        .next()
        .is_some()
    };
    if !field(harness) {
        // Fall back to the last text field on screen (the area's).
        let last = harness
            .query_all_by(|n| matches!(n.role(), Role::TextInput))
            .last();
        let Some(node) = last else {
            panic!("no field for {hint:?}; labels {:?}", labels(harness));
        };
        node.click();
    } else if let Some(node) = harness
        .query_all_by(|n| {
            matches!(n.role(), Role::TextInput)
                && (n.placeholder().is_some_and(|p| p == hint)
                    || n.label().is_some_and(|l| l == hint))
        })
        .next()
    {
        node.click();
    }
    frames(harness);
    if let Some(node) = harness
        .query_all_by(|n| matches!(n.role(), Role::TextInput) && n.is_focused())
        .next()
    {
        node.type_text(text);
    }
    frames(harness);
}
