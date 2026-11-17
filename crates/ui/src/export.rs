//! Progress report as Markdown, in the current UI language.

use chrono::NaiveDate;
use kg_engine::{QuizStats, StudyPlan, TopicStatus};

use crate::i18n;

/// The profile and goals (Phase 11), appended to the progress report.
pub fn profile_section(
    profile: &kg_engine::Profile,
    goals: Option<&kg_engine::StudyGoals>,
    streak: u32,
) -> String {
    let mut out = format!("\n## {}\n", t!("profile.tab"));
    out.push_str(&format!(
        "- {} ({})\n",
        t!("profile.level", level = profile.level.level),
        t!(
            "profile.experience",
            current = profile.level.experience,
            next = profile.level.next
        )
    ));
    out.push_str(&format!(
        "- {}\n",
        t!(
            "profile.totals",
            hours = profile.minutes / 60,
            days = profile.study_days
        )
    ));
    out.push_str(&format!("- {}\n", t!("profile.streak", days = streak)));
    if let Some(goals) = goals {
        out.push_str(&format!(
            "- {}\n",
            t!(
                "export.goals",
                hours = goals.hours_per_week,
                topics = goals.topics_per_week,
                quiz = goals.quiz_percent
            )
        ));
    }
    out
}

pub fn markdown(plan: &StudyPlan, stats: QuizStats, lessons: u32, today: NaiveDate) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    line(format!("# {}: {}", t!("export.title"), plan.title));
    if !plan.exam.is_empty() {
        line(format!("_{}_", plan.exam));
    }
    line(String::new());
    line(t!("export.generated", date = i18n::date(today)).into_owned());
    line(String::new());
    line(format!("## {}", t!("export.summary")));
    line(format!(
        "- {}",
        t!(
            "export.topics_done",
            done = plan.count_with_status(TopicStatus::Done),
            total = plan.topic_count()
        )
    ));
    line(format!(
        "- {}",
        t!("export.due", count = plan.due_topics(today).len())
    ));
    line(format!("- {}", t!("export.lessons", count = lessons)));
    if let Some(accuracy) = stats.accuracy() {
        line(format!(
            "- {}",
            t!(
                "export.quizzes",
                attempts = stats.attempts,
                accuracy = format!("{:.0}", accuracy * 100.0)
            )
        ));
    }

    for subject in &plan.subjects {
        line(String::new());
        line(format!(
            "## {} ({:.0}%)",
            subject.name,
            subject.progress() * 100.0
        ));
        line(String::new());
        line(format!(
            "| {} | {} | {} | {} |",
            t!("export.col.topic"),
            t!("export.col.week"),
            t!("export.col.status"),
            t!("export.col.next_review")
        ));
        line("|---|---|---|---|".into());
        for topic in &subject.topics {
            let due = topic.review.due.map(i18n::date).unwrap_or_default();
            line(format!(
                "| {} | {} | {} | {} |",
                topic.title.replace('|', "/"),
                topic.week,
                i18n::status(topic.status),
                due
            ));
        }
    }
    out
}
