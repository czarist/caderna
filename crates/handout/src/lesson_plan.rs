//! Printable class schedule (Phase W8): the schedule as a table and one page
//! per class in the plan-of-class format, laid out with Typst like the
//! handouts. The caller formats every date, kind and status in the app's
//! language; this module only lays them out.

use serde::{Deserialize, Serialize};
use typst_layout::PagedDocument;

use crate::{HandoutError, HandoutWorld, join};

const TEMPLATE: &str = include_str!("../templates/lesson_plan.typ");

/// Fixed words in the PDF, from the app's translation files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LessonPlanLabels {
    pub schedule: String,
    pub lesson_plan: String,
    pub date: String,
    pub time: String,
    pub kind: String,
    pub topics: String,
    pub status: String,
    pub objectives: String,
    pub activities: String,
    pub materials: String,
    pub homework: String,
    pub link: String,
    pub note: String,
    /// Shown on a class with no plan yet.
    pub empty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonPlanActivity {
    pub kind: String,
    /// Already formatted, e.g. "15 min".
    pub minutes: String,
    pub text: String,
}

/// One class, every field already formatted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonPlanSession {
    pub weekday: String,
    pub date: String,
    pub time: String,
    pub minutes: String,
    pub kind: String,
    pub status: String,
    pub topics: Vec<String>,
    pub objectives: Vec<String>,
    pub activities: Vec<LessonPlanActivity>,
    pub materials: Vec<String>,
    pub homework: Vec<LessonPlanActivity>,
    /// The online class's link, or empty.
    pub link: String,
    /// The note to students (never the teacher's private note).
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonPlanDocument {
    pub title: String,
    pub subtitle: String,
    /// Typst language code for hyphenation, e.g. "pt" or "en".
    pub lang: String,
    pub labels: LessonPlanLabels,
    pub sessions: Vec<LessonPlanSession>,
}

fn layout(document: &LessonPlanDocument) -> Result<PagedDocument, HandoutError> {
    if document.sessions.is_empty() {
        return Err(HandoutError::Empty);
    }
    let data = serde_json::to_vec(document).map_err(|e| HandoutError::Layout(e.to_string()))?;
    let world = HandoutWorld::new(TEMPLATE, data, &[], &document.lang)?;
    typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|errors| HandoutError::Layout(join(&errors)))
}

/// Renders the schedule to PDF bytes.
pub fn render_lesson_plan_pdf(document: &LessonPlanDocument) -> Result<Vec<u8>, HandoutError> {
    let paged = layout(document)?;
    typst_pdf::pdf(&paged, &typst_pdf::PdfOptions::default())
        .map_err(|errors| HandoutError::Pdf(join(&errors)))
}

/// Pages of the laid-out schedule, for tests.
pub fn lesson_plan_page_count(document: &LessonPlanDocument) -> Result<usize, HandoutError> {
    Ok(layout(document)?.pages().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(date: &str, objectives: &[&str]) -> LessonPlanSession {
        LessonPlanSession {
            weekday: "Seg".into(),
            date: date.into(),
            time: "19:00".into(),
            minutes: "100 min".into(),
            kind: "Aula".into(),
            status: "Prevista".into(),
            topics: vec!["Funções #[x] $y$ *z*".into()],
            objectives: objectives.iter().map(|o| (*o).to_owned()).collect(),
            activities: vec![LessonPlanActivity {
                kind: "Explicação".into(),
                minutes: "30 min".into(),
                text: "Exposição dialogada".into(),
            }],
            materials: vec!["Apostila 1".into()],
            homework: Vec::new(),
            link: "https://meet.example.com/x".into(),
            note: "Tragam régua".into(),
        }
    }

    #[test]
    fn the_schedule_gets_a_table_and_a_page_per_class() {
        let document = LessonPlanDocument {
            title: "ENEM 2027".into(),
            subtitle: "Turma A".into(),
            lang: "pt".into(),
            labels: LessonPlanLabels::default(),
            sessions: vec![
                session("02/03/2026", &["Reconhecer funções"]),
                session("04/03/2026", &[]),
            ],
        };
        assert_eq!(lesson_plan_page_count(&document).unwrap(), 3);
        let pdf = render_lesson_plan_pdf(&document).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn an_empty_schedule_is_refused() {
        let document = LessonPlanDocument {
            title: String::new(),
            subtitle: String::new(),
            lang: "pt".into(),
            labels: LessonPlanLabels::default(),
            sessions: Vec::new(),
        };
        assert!(matches!(
            render_lesson_plan_pdf(&document),
            Err(HandoutError::Empty)
        ));
    }
}
