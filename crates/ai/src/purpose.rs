//! What a study plan prepares for, chosen by the teacher when the plan is
//! created: a national exam (ENEM), a university entrance exam (vestibular),
//! a public-service exam (concurso), a professional exam (OAB, certifications)
//! or something else. Teaching tasks get the purpose's note after the exam
//! culture's, so the plan, handouts, quizzes and lessons aim at that kind of
//! exam. The syllabus and past exams still win over it.

use crate::config::PromptLanguage;

/// The kind of exam a plan prepares for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExamPurpose {
    /// ENEM or another national school-leaving exam.
    NationalExam,
    /// A university's own entrance exam (vestibular).
    UniversityEntrance,
    /// A public-service selection (concurso público).
    CivilService,
    /// A professional or certification exam (OAB, councils, IT certifications).
    Professional,
    /// Anything else: school tests, courses, general study.
    #[default]
    General,
}

impl ExamPurpose {
    pub const ALL: [Self; 5] = [
        Self::NationalExam,
        Self::UniversityEntrance,
        Self::CivilService,
        Self::Professional,
        Self::General,
    ];

    /// Stable id, kept in the database and sent to the panel.
    pub fn id(self) -> &'static str {
        match self {
            Self::NationalExam => "national_exam",
            Self::UniversityEntrance => "university_entrance",
            Self::CivilService => "civil_service",
            Self::Professional => "professional",
            Self::General => "general",
        }
    }

    /// The purpose with this id; unknown ids are [`Self::General`].
    pub fn from_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|p| p.id() == id.trim())
            .unwrap_or_default()
    }

    /// The note for the model, in the prompts' language.
    pub fn note(self, language: PromptLanguage) -> &'static str {
        let (pt, en) = match self {
            Self::NationalExam => (NATIONAL_PT, NATIONAL_EN),
            Self::UniversityEntrance => (ENTRANCE_PT, ENTRANCE_EN),
            Self::CivilService => (CIVIL_PT, CIVIL_EN),
            Self::Professional => (PROFESSIONAL_PT, PROFESSIONAL_EN),
            Self::General => (GENERAL_PT, GENERAL_EN),
        };
        language.pick(pt, en)
    }
}

const NATIONAL_PT: &str = "\
Finalidade do estudo: exame nacional do ensino médio (como o ENEM). Siga-a quando o programa da prova não disser outra coisa:
- Conteúdo do ensino médio cobrado por competências e habilidades, de forma interdisciplinar e contextualizada (textos, gráficos, situações do cotidiano).
- Questões longas com texto-base; treine leitura, interpretação e a escolha entre alternativas parecidas. Na nota pela TRI a coerência das respostas conta: acertar as fáceis importa.
- Redação, quando houver: dissertativo-argumentativa com repertório sociocultural e proposta de intervenção.";

const NATIONAL_EN: &str = "\
Purpose of study: a national school-leaving exam (such as Brazil's ENEM). Follow it unless the exam syllabus says otherwise:
- Secondary-school content assessed through competencies and skills, across subjects and in context (texts, charts, everyday situations).
- Long items built on a source text; train reading, interpretation and choosing between close options. Consistency counts in the score (item response theory): getting the easy items right matters.
- The essay, when there is one: argumentative, with cultural references and an intervention proposal.";

const ENTRANCE_PT: &str = "\
Finalidade do estudo: vestibular (prova de ingresso de uma universidade). Siga-a quando o programa da prova não disser outra coisa:
- Conteúdo do ensino médio com a profundidade e o estilo da universidade indicada no programa; listas de leituras obrigatórias contam.
- Primeira fase objetiva; segunda fase com questões discursivas por matéria, que pedem resolução escrita e justificativa.
- Dê exemplos de questões no estilo da instituição e mostre como montar uma resposta dissertativa completa.";

const ENTRANCE_EN: &str = "\
Purpose of study: a university entrance exam (vestibular). Follow it unless the exam syllabus says otherwise:
- Secondary-school content at the depth and in the style of the university the syllabus names; required reading lists count.
- A multiple-choice first stage; a second stage of written questions per subject that ask for worked solutions and reasons.
- Give examples in the institution's style and show how to build a complete written answer.";

const CIVIL_PT: &str = "\
Finalidade do estudo: concurso público. Siga-a quando o programa da prova não disser outra coisa:
- O edital manda: estude só o que ele lista, na ordem de peso das matérias, para o cargo escolhido.
- Letra da lei, súmulas e jurisprudência atualizadas onde couber; a banca organizadora define o estilo (certo ou errado, múltipla escolha) e as pegadinhas.
- Priorize o que mais cai e treine com questões no formato da banca.";

const CIVIL_EN: &str = "\
Purpose of study: a public-service selection exam (concurso). Follow it unless the exam syllabus says otherwise:
- The official notice rules: study only what it lists, weighted by the subjects' scores, for the chosen position.
- The letter of the law, binding precedents and current case law where relevant; the exam board sets the style (true/false, multiple choice) and the traps.
- Put first what is asked most and practise with items in the board's format.";

const PROFESSIONAL_PT: &str = "\
Finalidade do estudo: exame profissional ou de certificação (como o Exame de Ordem da OAB ou provas de conselhos e certificações). Siga-a quando o programa da prova não disser outra coisa:
- Foco na prática da profissão: casos concretos, normas da área, ética profissional e os procedimentos que o exame cobra.
- Se houver fase prática (peça, estudo de caso, laboratório), ensine a estrutura esperada e os erros que eliminam.
- Use a nota mínima de aprovação como meta e priorize os temas de maior incidência.";

const PROFESSIONAL_EN: &str = "\
Purpose of study: a professional or certification exam (such as a bar exam, a professional council's exam or an IT certification). Follow it unless the exam syllabus says otherwise:
- Focus on practising the profession: concrete cases, the field's rules, professional ethics and the procedures the exam asks for.
- When there is a practical stage (a legal brief, a case study, a lab), teach the expected structure and the mistakes that fail a candidate.
- Aim at the pass mark and put the most frequent themes first.";

const GENERAL_PT: &str = "\
Finalidade do estudo: estudo geral ou prova sem formato definido. Siga o programa da prova e equilibre teoria, exemplos e exercícios.";

const GENERAL_EN: &str = "\
Purpose of study: general study or an exam without a set format. Follow the exam syllabus and balance theory, examples and exercises.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_every_purpose_has_both_notes() {
        for purpose in ExamPurpose::ALL {
            assert_eq!(ExamPurpose::from_id(purpose.id()), purpose);
            let pt = purpose.note(PromptLanguage::Portuguese);
            let en = purpose.note(PromptLanguage::English);
            assert!(pt.starts_with("Finalidade do estudo"), "{purpose:?}");
            assert!(en.starts_with("Purpose of study"), "{purpose:?}");
            assert_eq!(purpose.note(PromptLanguage::Japanese), en);
        }
        assert_eq!(ExamPurpose::from_id("unknown"), ExamPurpose::General);
    }
}
