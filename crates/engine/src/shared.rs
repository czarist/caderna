//! The shared base in the brand's panel: material the AI made once (a
//! study plan for a syllabus, a topic's handout or quiz questions) that
//! every school reuses instead of asking the AI again, to save tokens.
//!
//! Before asking the AI the app looks up a key; when the panel has it, the
//! AI is not called. When it does not, the app shares what the AI made. The
//! key is a SHA-256 of everything the AI was asked (kind, prompt text,
//! language, purpose, exam culture and the syllabus or the topic), so a
//! changed prompt or another language never reuses old material.
//!
//! Only AI output is shared: no student data, no board material a school
//! uploaded and no handout a school changed. Any failure here is logged and
//! the app asks the AI as usual.

use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Bump to stop reusing everything shared so far.
const SHARE_VERSION: &str = "kg-shared-v1";

/// What a piece of shared material is, for the panel's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedKind {
    Plan,
    Handout,
    QuizBank,
}

impl SharedKind {
    fn id(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Handout => "handout",
            Self::QuizBank => "quiz_bank",
        }
    }
}

/// The shared base as the active plan sees it: the panel and the plan's
/// language, purpose and exam culture.
#[derive(Debug, Clone)]
pub(crate) struct SharedBase {
    base: String,
    token: String,
    language: kg_ai::PromptLanguage,
    purpose: kg_ai::ExamPurpose,
    culture: String,
    agent: ureq::Agent,
}

#[derive(Serialize)]
struct Contribution<'a> {
    kind: &'static str,
    language: &'static str,
    purpose: &'static str,
    subject: &'a str,
    topic: &'a str,
    content: &'a str,
}

impl SharedBase {
    pub(crate) fn new(
        token: String,
        language: kg_ai::PromptLanguage,
        purpose: kg_ai::ExamPurpose,
        culture: &str,
    ) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(kg_brand::user_agent("shared"))
            .build()
            .into();
        Self {
            base: format!("{}/api/v1/shared-materials", kg_brand::SERVER_URL),
            token,
            language,
            purpose,
            culture: culture.to_owned(),
            agent,
        }
    }

    pub(crate) fn language(&self) -> kg_ai::PromptLanguage {
        self.language
    }

    /// The key of a request. `prompt` is the prompt text the AI gets, so
    /// changing it stops reuse; `parts` are the request's own inputs.
    pub(crate) fn key(&self, kind: SharedKind, prompt: &str, parts: &[&str]) -> String {
        let mut hasher = Sha256::new();
        for part in [
            SHARE_VERSION,
            kind.id(),
            self.language.code(),
            self.purpose.id(),
            &self.culture,
            prompt,
        ]
        .into_iter()
        .chain(parts.iter().copied())
        {
            hasher.update(part.as_bytes());
            hasher.update([0x1f]);
        }
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Whether the exam's name belongs in a topic's key: public-service and
    /// professional exams follow their own notice, so a topic of one exam
    /// is not reused for another; school-leaving and entrance exams share
    /// the same secondary-school content.
    pub(crate) fn exam_matters(&self) -> bool {
        matches!(
            self.purpose,
            kg_ai::ExamPurpose::CivilService | kg_ai::ExamPurpose::Professional
        )
    }

    /// The shared material under `key`, if the panel has it.
    pub(crate) fn fetch(&self, key: &str) -> Option<String> {
        let response = self
            .agent
            .get(&format!("{}/{key}", self.base))
            .header("Accept", "application/json")
            .header("Authorization", &format!("Bearer {}", self.token))
            .call();
        let mut response = match response {
            Ok(response) => response,
            Err(err) => {
                tracing::info!(%err, "shared base not reached");
                return None;
            }
        };
        if response.status().as_u16() != 200 {
            return None;
        }
        let body = response.body_mut().read_to_string().ok()?;
        let value: serde_json::Value = serde_json::from_str(&body).ok()?;
        let content = value.pointer("/data/content")?.as_str()?.to_owned();
        tracing::info!("reused material from the shared base");
        Some(content)
    }

    /// Shares what the AI made. Best effort: a failure only means the next
    /// school asks the AI too.
    pub(crate) fn give(
        &self,
        key: &str,
        kind: SharedKind,
        subject: &str,
        topic: &str,
        content: &str,
    ) {
        let body = Contribution {
            kind: kind.id(),
            language: self.language.code(),
            purpose: self.purpose.id(),
            subject: truncate(subject, 255),
            topic: truncate(topic, 255),
            content,
        };
        let result = self
            .agent
            .put(&format!("{}/{key}", self.base))
            .header("Accept", "application/json")
            .header("Authorization", &format!("Bearer {}", self.token))
            .send_json(&body);
        match result {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                tracing::info!(
                    status = response.status().as_u16(),
                    "shared base refused the material"
                );
            }
            Err(err) => tracing::info!(%err, "could not share the material"),
        }
    }
}

fn truncate(text: &str, chars: usize) -> &str {
    match text.char_indices().nth(chars) {
        Some((at, _)) => &text[..at],
        None => text,
    }
}

impl crate::Engine {
    /// The shared base for the active plan: only with a brand panel that
    /// confirmed this computer.
    pub(crate) fn shared_base(&self) -> Option<SharedBase> {
        let token = self.panel_token()?;
        let purpose = self
            .active
            .as_ref()
            .map(|plan| kg_ai::ExamPurpose::from_id(&plan.purpose))
            .unwrap_or_default();
        Some(self.shared_base_for(token, purpose))
    }

    /// The shared base for a plan not created yet.
    pub(crate) fn shared_base_with(&self, purpose: kg_ai::ExamPurpose) -> Option<SharedBase> {
        let token = self.panel_token()?;
        Some(self.shared_base_for(token, purpose))
    }

    fn shared_base_for(&self, token: String, purpose: kg_ai::ExamPurpose) -> SharedBase {
        let culture = self.settings.region.map_or("", crate::settings::Region::id);
        SharedBase::new(token, self.prompt_language(), purpose, culture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(purpose: kg_ai::ExamPurpose) -> SharedBase {
        SharedBase::new(
            "token".into(),
            kg_ai::PromptLanguage::Portuguese,
            purpose,
            "brazil",
        )
    }

    #[test]
    fn equal_requests_share_a_key_and_any_change_makes_another() {
        let enem = base(kg_ai::ExamPurpose::NationalExam);
        let key = enem.key(SharedKind::Handout, "prompt", &["Matemática", "Funções"]);
        assert_eq!(key.len(), 64);
        assert_eq!(
            key,
            enem.key(SharedKind::Handout, "prompt", &["Matemática", "Funções"])
        );
        assert_ne!(
            key,
            enem.key(SharedKind::QuizBank, "prompt", &["Matemática", "Funções"])
        );
        assert_ne!(
            key,
            enem.key(
                SharedKind::Handout,
                "new prompt",
                &["Matemática", "Funções"]
            )
        );
        assert_ne!(
            key,
            base(kg_ai::ExamPurpose::UniversityEntrance).key(
                SharedKind::Handout,
                "prompt",
                &["Matemática", "Funções"]
            )
        );
        // Parts are separated, so moving text between them changes the key.
        assert_ne!(
            enem.key(SharedKind::Handout, "p", &["ab", "c"]),
            enem.key(SharedKind::Handout, "p", &["a", "bc"])
        );
    }

    #[test]
    fn only_exams_with_their_own_notice_key_by_exam_name() {
        assert!(!base(kg_ai::ExamPurpose::NationalExam).exam_matters());
        assert!(base(kg_ai::ExamPurpose::CivilService).exam_matters());
        assert_eq!(truncate("ação", 2), "aç");
    }
}
