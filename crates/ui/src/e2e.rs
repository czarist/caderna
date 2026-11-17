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

/// A plan with one topic, as if the notice had been read.
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
        .insert_plan(&plan, "edital.pdf", "EDITAL", today)
        .unwrap();
}

#[test]
#[ignore = "needs the brand's panel and KG_E2E_CODE; see the module docs"]
fn a_new_student_gets_from_activation_to_a_lesson() {
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
        first_run_and_lesson(&mut harness);
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

fn first_run_and_lesson(harness: &mut App<'_>) {
    // 1. Language (the region comes with it).
    click(harness, &crate::i18n::language_name("pt-BR"));
    click(harness, &t!("setup.next"));

    // 2. Name.
    type_in_field(harness, "Ana");
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
    assert_ne!(harness.state().ui.screen, Screen::Setup);

    // The tour comes next; skip it.
    click(harness, &t!("tour.skip"));
    assert!(!engine(harness).needs_tour());
    harness.state_mut().clear_toasts();
    frames(harness);

    // Today offers the plan's first lesson.
    assert_eq!(harness.state().ui.screen, Screen::Today);
    click(harness, &t!("lesson.start"));
    assert_eq!(harness.state().ui.screen, Screen::Lesson);
    assert!(
        harness.state().ui.lesson_panel,
        "the transcript and chat open with the lesson"
    );
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
