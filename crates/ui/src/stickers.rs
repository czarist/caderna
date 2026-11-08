//! Sticker art for the challenges: original SVGs in the app's engraving
//! style, on jade with the crescent moon, one per family
//! (`assets/stickers`, built by `tools/stickers.py`). Each family has its
//! own outline and motif, so it is known at a glance even small. The level
//! is drawn here: the rim's colour (bronze, silver, gold, jade) and one to
//! three dots. A sticker not won yet is the same art in grey, made from the
//! colour one so the two never drift apart.

use egui::{Image, Vec2};
use kg_engine::{ChallengeId, Family};

/// Rim colours by level: three levels each.
const RIMS: [&str; 4] = ["#B07A4A", "#B9C2C7", "#D4A932", "#7FD1BE"];
/// The colour the art leaves for the rim and dots.
const RIM_TOKEN: &str = "#RIM000";

fn sticker_art(name: &str, fallback: &'static str) -> &'static str {
    kg_brand::artwork("stickers", name)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or(fallback)
}

fn family_svg(family: Family) -> &'static str {
    match family {
        Family::Lessons => sticker_art(
            "lessons.svg",
            include_str!("../assets/stickers/lessons.svg"),
        ),
        Family::LessonDays => sticker_art(
            "lesson_days.svg",
            include_str!("../assets/stickers/lesson_days.svg"),
        ),
        Family::Quizzes => sticker_art(
            "quizzes.svg",
            include_str!("../assets/stickers/quizzes.svg"),
        ),
        Family::RightAnswers => sticker_art(
            "right_answers.svg",
            include_str!("../assets/stickers/right_answers.svg"),
        ),
        Family::PerfectQuizzes => sticker_art(
            "perfect_quizzes.svg",
            include_str!("../assets/stickers/perfect_quizzes.svg"),
        ),
        Family::HighQuizzes => sticker_art(
            "high_quizzes.svg",
            include_str!("../assets/stickers/high_quizzes.svg"),
        ),
        Family::Recoveries => sticker_art(
            "recoveries.svg",
            include_str!("../assets/stickers/recoveries.svg"),
        ),
        Family::CleanNotebook => sticker_art(
            "clean_notebook.svg",
            include_str!("../assets/stickers/clean_notebook.svg"),
        ),
        Family::Coverage => sticker_art(
            "coverage.svg",
            include_str!("../assets/stickers/coverage.svg"),
        ),
        Family::CoverageShare => sticker_art(
            "coverage_share.svg",
            include_str!("../assets/stickers/coverage_share.svg"),
        ),
        Family::Weeks => sticker_art("weeks.svg", include_str!("../assets/stickers/weeks.svg")),
        Family::PlanComplete => sticker_art(
            "plan_complete.svg",
            include_str!("../assets/stickers/plan_complete.svg"),
        ),
        Family::Mastered => sticker_art(
            "mastered.svg",
            include_str!("../assets/stickers/mastered.svg"),
        ),
        Family::SubjectsMastered => sticker_art(
            "subjects_mastered.svg",
            include_str!("../assets/stickers/subjects_mastered.svg"),
        ),
        Family::NoticeMastered => sticker_art(
            "notice_mastered.svg",
            include_str!("../assets/stickers/notice_mastered.svg"),
        ),
        Family::Reviews => sticker_art(
            "reviews.svg",
            include_str!("../assets/stickers/reviews.svg"),
        ),
        Family::PastExams => sticker_art(
            "past_exams.svg",
            include_str!("../assets/stickers/past_exams.svg"),
        ),
        Family::HandoutsRead => sticker_art(
            "handouts_read.svg",
            include_str!("../assets/stickers/handouts_read.svg"),
        ),
        Family::Highlights => sticker_art(
            "highlights.svg",
            include_str!("../assets/stickers/highlights.svg"),
        ),
        Family::HandoutsMade => sticker_art(
            "handouts_made.svg",
            include_str!("../assets/stickers/handouts_made.svg"),
        ),
        Family::Essays => sticker_art("essays.svg", include_str!("../assets/stickers/essays.svg")),
        Family::EssaysHigh => sticker_art(
            "essays_high.svg",
            include_str!("../assets/stickers/essays_high.svg"),
        ),
        Family::EssayTop => sticker_art(
            "essay_top.svg",
            include_str!("../assets/stickers/essay_top.svg"),
        ),
        Family::Mocks => sticker_art("mocks.svg", include_str!("../assets/stickers/mocks.svg")),
        Family::MocksHigh => sticker_art(
            "mocks_high.svg",
            include_str!("../assets/stickers/mocks_high.svg"),
        ),
        Family::MockTop => sticker_art(
            "mock_top.svg",
            include_str!("../assets/stickers/mock_top.svg"),
        ),
        Family::SubjectsStarted => sticker_art(
            "subjects_started.svg",
            include_str!("../assets/stickers/subjects_started.svg"),
        ),
        Family::Hours => sticker_art("hours.svg", include_str!("../assets/stickers/hours.svg")),
        Family::StudyDays => sticker_art(
            "study_days.svg",
            include_str!("../assets/stickers/study_days.svg"),
        ),
        Family::LongDay => sticker_art(
            "long_day.svg",
            include_str!("../assets/stickers/long_day.svg"),
        ),
        Family::Plans => sticker_art("plans.svg", include_str!("../assets/stickers/plans.svg")),
        Family::AllLessons => sticker_art(
            "all_lessons.svg",
            include_str!("../assets/stickers/all_lessons.svg"),
        ),
        Family::AllQuizzes => sticker_art(
            "all_quizzes.svg",
            include_str!("../assets/stickers/all_quizzes.svg"),
        ),
        Family::AllRightAnswers => sticker_art(
            "all_right_answers.svg",
            include_str!("../assets/stickers/all_right_answers.svg"),
        ),
        Family::AllMastered => sticker_art(
            "all_mastered.svg",
            include_str!("../assets/stickers/all_mastered.svg"),
        ),
        Family::Stickers => sticker_art(
            "stickers.svg",
            include_str!("../assets/stickers/stickers.svg"),
        ),
    }
}

/// Removes every `<!--{name}-->…<!--/{name}-->` block.
fn strip(svg: &str, name: &str) -> String {
    let (open, close) = (format!("<!--{name}-->"), format!("<!--/{name}-->"));
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        match rest[start..].find(&close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The family art with this challenge's level: a coloured rim and dots, or
/// none when the family has a single challenge.
fn level_svg(id: ChallengeId) -> String {
    let svg = family_svg(id.family());
    let levels = id.family().challenges().count();
    if levels <= 1 {
        let svg = strip(svg, "rim");
        return ["dots1", "dots2", "dots3"]
            .iter()
            .fold(svg, |svg, dots| strip(&svg, dots));
    }
    let level = id.level().max(1) as usize;
    let rim = RIMS[((level - 1) / 3).min(RIMS.len() - 1)];
    let keep = format!("dots{}", (level - 1) % 3 + 1);
    ["dots1", "dots2", "dots3"]
        .iter()
        .filter(|dots| **dots != keep)
        .fold(svg.to_owned(), |svg, dots| strip(&svg, dots))
        .replace(RIM_TOKEN, rim)
}

fn uri(id: ChallengeId, won: bool) -> String {
    let state = if won { "won" } else { "locked" };
    format!("bytes://sticker-{}-{state}.svg", id.key())
}

/// Makes both versions of a sticker loadable by URI, once per context.
fn register(ctx: &egui::Context, id: ChallengeId) {
    let flag = egui::Id::new(("sticker_registered", id.key()));
    if ctx.data(|d| d.get_temp::<bool>(flag)).unwrap_or(false) {
        return;
    }
    let svg = level_svg(id);
    ctx.include_bytes(uri(id, false), greyed(&svg).into_bytes());
    ctx.include_bytes(uri(id, true), svg.into_bytes());
    ctx.data_mut(|d| d.insert_temp(flag, true));
}

/// The sticker at `size` points: in colour when won, grey otherwise.
pub fn image(ctx: &egui::Context, id: ChallengeId, size: f32, won: bool) -> Image<'static> {
    register(ctx, id);
    let image = Image::new(uri(id, won)).fit_to_exact_size(Vec2::splat(size));
    if won {
        image
    } else {
        image.tint(egui::Color32::from_white_alpha(170))
    }
}

/// The same SVG with every `#rrggbb` colour turned into a light grey of
/// similar lightness: a faded, colourless sticker.
fn greyed(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find('#') {
        out.push_str(&rest[..at]);
        let hex = rest.get(at + 1..at + 7).filter(|h| {
            h.chars().all(|c| c.is_ascii_hexdigit())
                && !rest[at + 7..].starts_with(|c: char| c.is_ascii_alphanumeric())
        });
        match hex.and_then(grey_of) {
            Some(grey) => {
                out.push_str(&grey);
                rest = &rest[at + 7..];
            }
            None => {
                out.push('#');
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn grey_of(hex: &str) -> Option<String> {
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    let (r, g, b) = (channel(0)?, channel(2)?, channel(4)?);
    let lightness = 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
    // Pulled toward a light grey: less contrast, so it reads as "not yet".
    let grey = (lightness * 0.55 + 190.0 * 0.45).round().clamp(0.0, 255.0) as u8;
    Some(format!("#{grey:02X}{grey:02X}{grey:02X}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draws(svg: &str) -> resvg::tiny_skia::Pixmap {
        let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &Default::default()).unwrap();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(48, 48).unwrap();
        let scale = 48.0 / tree.size().width();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        pixmap
    }

    #[test]
    fn every_sticker_draws_in_colour_and_in_grey() {
        for id in ChallengeId::all() {
            let svg = level_svg(id);
            assert!(!svg.contains(RIM_TOKEN), "{} keeps the rim token", id.key());
            assert!(
                draws(&svg).pixels().iter().any(|p| p.alpha() > 0),
                "{} is empty",
                id.key()
            );
            for pixel in draws(&greyed(&svg)).pixels() {
                let c = pixel.demultiply();
                assert!(
                    c.red().abs_diff(c.green()) <= 2 && c.green().abs_diff(c.blue()) <= 2,
                    "{} keeps colour when locked",
                    id.key()
                );
            }
        }
    }

    #[test]
    fn levels_differ_by_rim_and_dots() {
        let first = ChallengeId::from_key("first_step").unwrap();
        let fourth = ChallengeId::from_key("lessons_10").unwrap();
        let single = ChallengeId::from_key("clean_notebook").unwrap();
        assert!(level_svg(first).contains(RIMS[0]));
        assert!(level_svg(fourth).contains(RIMS[1]));
        // Level 1 keeps one dot, level 4 one dot in silver, level 6 three.
        assert!(level_svg(first).contains("<!--dots1-->"));
        assert!(!level_svg(first).contains("<!--dots2-->"));
        let sixth = ChallengeId::from_key("lessons_20").unwrap();
        assert!(level_svg(sixth).contains("<!--dots3-->"));
        assert!(!level_svg(single).contains("<!--rim-->"));
        assert!(!level_svg(single).contains("<!--dots"));
    }

    #[test]
    fn greying_changes_only_colours() {
        let svg = r##"<rect fill="#2F6F68" id="a1"/><use href="#a1"/>"##;
        assert_eq!(
            greyed(svg),
            r##"<rect fill="#888888" id="a1"/><use href="#a1"/>"##
        );
    }
}
