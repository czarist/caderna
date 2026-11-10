//! The app's paid areas. A brand with a panel opens them per licence (the
//! panel's "Plano"); without a panel every module is open.

use crate::Command;

/// A part of the app a licence can open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    /// Voice lessons with the teacher and the whiteboard ("Videoaula").
    Lessons,
    /// Handouts, quizzes, reviews, mock exams, written answers and exam
    /// board material ("Criação de conteúdos").
    Content,
}

impl Module {
    /// The panel's id (`App\Enums\Module`).
    pub fn id(self) -> &'static str {
        match self {
            Self::Lessons => "lessons",
            Self::Content => "content",
        }
    }
}

impl Command {
    /// The module a command starts something in; `None` for commands every
    /// plan has (plan, notice, settings…) and for steps inside something
    /// already started.
    pub fn module(&self) -> Option<Module> {
        Some(match self {
            Self::StartLesson(_) | Self::ResumeLesson(_) => Module::Lessons,
            Self::StartQuiz(_)
            | Self::RetryMistakes(_)
            | Self::StartReview(_)
            | Self::ExportHandout(..)
            | Self::PrepareContent(_)
            | Self::ImportBoard { .. }
            | Self::ImportAnswerKey { .. }
            | Self::OpenReading(_)
            | Self::GenerateProposal(..)
            | Self::StartAttempt(..)
            | Self::GenerateMock(_)
            | Self::StartMock(_) => Module::Content,
            _ => return None,
        })
    }
}
