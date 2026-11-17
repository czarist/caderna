//! Progress overview and export.

use egui::RichText;
use kg_engine::{Command, Engine, TopicStatus};

use super::Actions;
use crate::app::{KnowGiverApp, Screen};
use crate::icons::Icon;
use crate::{export, theme, widgets};

pub fn show(app: &mut KnowGiverApp, ui: &mut egui::Ui) {
    let mut actions = Actions::default();
    let Ok(engine) = &app.engine else {
        return;
    };
    widgets::header(ui, &t!("progress.title"), &t!("progress.subtitle"));

    let Some(plan) = engine.active_plan() else {
        if widgets::empty_state(
            ui,
            Icon::Progress,
            &t!("progress.title"),
            &t!("progress.empty"),
            Some(&t!("plan.go_to_edital")),
        ) {
            actions.go_to(Screen::Edital);
        }
        actions.run(app);
        return;
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        let today = Engine::today();
        let stats = engine.quiz_stats();
        let accuracy = stats
            .accuracy()
            .map_or_else(|| "–".to_owned(), |a| format!("{:.0}%", a * 100.0));
        ui.horizontal_wrapped(|ui| {
            tile(
                ui,
                &format!("{:.0}%", plan.progress() * 100.0),
                &t!("progress.tile.done"),
            );
            tile(
                ui,
                &plan.count_with_status(TopicStatus::Reviewing).to_string(),
                &t!("progress.tile.reviewing"),
            );
            tile(
                ui,
                &plan.due_topics(today).len().to_string(),
                &t!("progress.tile.due"),
            );
            tile(
                ui,
                &engine.lesson_count().to_string(),
                &t!("progress.tile.lessons"),
            );
            tile(ui, &accuracy, &t!("progress.tile.accuracy"));
        });

        ui.add_space(16.0);
        ui.label(RichText::new(t!("progress.by_subject")).strong());
        for subject in &plan.subjects {
            let done = subject
                .topics
                .iter()
                .filter(|t| t.status == TopicStatus::Done)
                .count();
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(260.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_width(260.0);
                        ui.add(egui::Label::new(&subject.name).truncate());
                    },
                );
                ui.add(
                    egui::ProgressBar::new(subject.progress())
                        .desired_width((ui.available_width() - 90.0).max(80.0))
                        .desired_height(8.0),
                );
                ui.label(
                    RichText::new(t!(
                        "progress.count",
                        done = done,
                        total = subject.topics.len()
                    ))
                    .small(),
                );
            });
        }

        if let Some(trend) = engine.mock_trend() {
            ui.add_space(16.0);
            mock_trend(ui, &trend);
        }

        ui.add_space(16.0);
        let p = theme::palette(ui.ctx());
        let export = egui::Button::image_and_text(
            Icon::Download.image(16.0, p.accent),
            t!("progress.export"),
        );
        if ui.add(export).clicked() {
            let default_name = format!("{}-{}.md", t!("export.file_prefix"), today);
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter(t!("progress.markdown"), &["md"])
                .save_file()
            {
                let mut content = export::markdown(plan, stats, engine.lesson_count(), today);
                content.push_str(&export::profile_section(
                    &engine.profile(),
                    engine.study_goals().as_ref(),
                    engine.streak(),
                ));
                actions.send(Command::ExportProgress { path, content });
            }
        }
    });
    actions.run(app);
}

/// Mock exams per subject: the average, the last sittings as a small row
/// of bars (oldest first) and the topics to study first.
fn mock_trend(ui: &mut egui::Ui, trend: &kg_engine::MockTrend) {
    let p = theme::palette(ui.ctx());
    ui.label(RichText::new(t!("progress.mocks", count = trend.sittings)).strong());
    for subject in &trend.subjects {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(260.0, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(260.0);
                    ui.add(egui::Label::new(&subject.subject).truncate());
                },
            );
            let recent = &subject.shares[subject.shares.len().saturating_sub(8)..];
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(10.0 * recent.len() as f32, 20.0),
                egui::Sense::hover(),
            );
            for (i, share) in recent.iter().enumerate() {
                let height = 2.0 + 18.0 * *share as f32;
                let x = rect.left() + 10.0 * i as f32;
                let bar = egui::Rect::from_min_max(
                    egui::pos2(x + 1.0, rect.bottom() - height),
                    egui::pos2(x + 8.0, rect.bottom()),
                );
                ui.painter().rect_filled(bar, 1.0, p.accent);
            }
            ui.label(
                RichText::new(t!(
                    "progress.mock_average",
                    percent = format!("{:.0}", subject.average * 100.0)
                ))
                .small(),
            );
            if let [.., before, last] = subject.shares[..] {
                let (word, color) = if last > before + 0.05 {
                    (t!("progress.mock_up"), p.success)
                } else if last + 0.05 < before {
                    (t!("progress.mock_down"), p.muted)
                } else {
                    (t!("progress.mock_steady"), p.muted)
                };
                ui.label(RichText::new(word).small().color(color));
            }
        });
    }
    if !trend.focus.is_empty() {
        ui.add_space(6.0);
        let topics: Vec<String> = trend
            .focus
            .iter()
            .map(|(title, missed)| {
                t!("progress.mock_focus_item", topic = title, count = missed).into_owned()
            })
            .collect();
        widgets::hint(ui, &t!("progress.mock_focus", topics = topics.join(" · ")));
    }
}

fn tile(ui: &mut egui::Ui, value: &str, label: &str) {
    let p = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(0)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_min_width(130.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(value).size(26.0).color(p.accent));
                ui.label(RichText::new(label).small().color(p.muted));
            });
        });
}
