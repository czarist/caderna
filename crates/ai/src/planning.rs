//! Lesson planning (Phase W8): the model suggests, for a batch of classes
//! already placed on the calendar by code, their objectives, activities with
//! minutes, materials and homework. The engine checks every answer.

use serde::Deserialize;

use crate::AiError;
use crate::client::{ChatMessage, Client};
use crate::config::Task;
use crate::prompts::{self, fill};
use crate::tasks::chat_json;

/// Activity kinds the model may use (the engine's `ActivityKind`).
pub const ACTIVITY_KINDS: [&str; 9] = [
    "explanation",
    "voice_lesson",
    "handout_reading",
    "quiz",
    "spoken_review",
    "written_answer",
    "mock_exam",
    "exercise",
    "other",
];

/// One class to plan; `topics` are numbers in [`PlanningInput::topics`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionBrief {
    pub number: usize,
    /// "class", "review" or "mock_exam".
    pub kind: String,
    pub minutes: u32,
    pub topics: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlanningInput {
    pub exam: String,
    /// Every topic of the plan, in order; numbered from 1 in the prompt.
    pub topics: Vec<String>,
    pub sessions: Vec<SessionBrief>,
    /// What the plan already has, one per line (handouts, quiz banks…).
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct ActivityDraft {
    pub kind: String,
    pub minutes: u32,
    pub text: String,
    /// Topic number, 0 for none.
    pub topic: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct SessionDraft {
    pub session: usize,
    pub objectives: Vec<String>,
    pub activities: Vec<ActivityDraft>,
    pub materials: Vec<String>,
    pub homework: Vec<ActivityDraft>,
}

#[derive(Deserialize)]
struct PlanningDraft {
    #[serde(default)]
    sessions: Vec<SessionDraft>,
}

/// Asks for the plan of each class in `input`.
pub fn suggest_session_plans(
    client: &Client,
    input: &PlanningInput,
) -> Result<Vec<SessionDraft>, AiError> {
    let lang = client.language();
    let topics: Vec<String> = input
        .topics
        .iter()
        .enumerate()
        .map(|(i, title)| format!("{}. {title}", i + 1))
        .collect();
    let sessions: Vec<String> = input
        .sessions
        .iter()
        .map(|s| {
            let numbers: Vec<String> = s.topics.iter().map(ToString::to_string).collect();
            format!(
                "{} {}: {} ({} min) — {}: {}",
                lang.pick("Aula", "Class"),
                s.number,
                s.kind,
                s.minutes,
                lang.pick("tópicos", "topics"),
                numbers.join(", ")
            )
        })
        .collect();
    let materials = if input.materials.is_empty() {
        lang.pick("(nenhum ainda)", "(none yet)").to_owned()
    } else {
        input
            .materials
            .iter()
            .map(|m| format!("- {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let system = fill(
        prompts::get(lang).lesson_planning,
        &[
            ("exam", input.exam.as_str()),
            ("topics", &topics.join("\n")),
            ("sessions", &sessions.join("\n")),
            ("materials", &materials),
            ("kinds", &ACTIVITY_KINDS.join(", ")),
        ],
    );
    let messages = [
        client.cultured(system),
        ChatMessage::user(lang.pick("Planeje as aulas.", "Plan the classes.")),
    ];
    let budget = 800 + 700 * input.sessions.len() as u32;
    let draft: PlanningDraft = chat_json(client, Task::StudyPlan, &messages, budget.min(12_000))?;
    Ok(draft.sessions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_read_partial_answers() {
        let draft: PlanningDraft = serde_json::from_str(
            r#"{"sessions": [{"session": 2, "objectives": ["Identificar funções"],
                "activities": [{"kind": "explanation", "minutes": 30, "text": "Exposição", "topic": 1}]}]}"#,
        )
        .unwrap();
        assert_eq!(draft.sessions[0].session, 2);
        assert_eq!(draft.sessions[0].activities[0].minutes, 30);
        assert!(draft.sessions[0].homework.is_empty());
    }
}
