//! Reading a photo of the printed answer card (Phase 12, stage 2), in pure
//! Rust and without any AI: the four corner markers are found, the
//! perspective is corrected with a homography from the card's millimetres to
//! the photo's pixels, and each bubble's darkness is measured at the
//! positions [`CardLayout`] printed. The student checks the grid on screen
//! before it is graded.
//!
//! The card also carries the exam's number as a row of filled squares
//! (20 bits and a 4-bit check), so a photo of another exam's card is caught
//! and an upside-down photo is read the right way round.

use std::io::Cursor;

use image::{DynamicImage, GrayImage, ImageDecoder, ImageReader, imageops};

use crate::CardLayout;

/// Number bits on the card, then check bits.
pub const CODE_BITS: u32 = 20;
const CHECK_BITS: u32 = 4;
/// The photo is reduced to this long side before reading.
const WORK_SIDE: u32 = 1800;
/// Darkness from which a bubble counts as filled.
const FILLED: f32 = 0.45;
/// Darkness from which an empty-looking bubble is worth a second look.
const FAINT: f32 = 0.25;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OmrError {
    #[error("the photo could not be opened: {0}")]
    Image(String),
    #[error("the four corner markers were not found")]
    Markers,
}

/// The cells of the exam number, filled for 1 bits, low bit first.
pub fn code_cells(id: u32) -> Vec<bool> {
    let value = (id & ((1 << CODE_BITS) - 1)) | (check(id) << CODE_BITS);
    (0..CODE_BITS + CHECK_BITS)
        .map(|bit| value & (1 << bit) != 0)
        .collect()
}

/// XOR of the number's five nibbles, so a misread cell rarely passes.
fn check(id: u32) -> u32 {
    let id = id & ((1 << CODE_BITS) - 1);
    (0..CODE_BITS / 4).fold(0x5, |acc, n| acc ^ ((id >> (n * 4)) & 0xF))
}

/// One question as read from the photo.
#[derive(Debug, Clone, PartialEq)]
pub struct BubbleRead {
    /// The single filled mark, if there is exactly one.
    pub choice: Option<u8>,
    /// Two marks filled, or a faint one: the student should look.
    pub doubtful: bool,
    /// Darkness of each mark, `0.0` (paper) to `1.0` (marker).
    pub darkness: Vec<f32>,
}

/// A photo of an answer card, ready to read.
pub struct CardScan {
    gray: GrayImage,
    /// Card millimetres → photo pixels.
    homography: [f64; 9],
    white: f32,
    black: f32,
    layout: CardLayout,
    code: Option<u32>,
}

impl std::fmt::Debug for CardScan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardScan")
            .field("code", &self.code)
            .finish_non_exhaustive()
    }
}

impl CardScan {
    /// Opens a photo (PNG or JPEG, any orientation) of a card printed with
    /// `layout`'s geometry.
    pub fn open(bytes: &[u8], layout: &CardLayout) -> Result<Self, OmrError> {
        let gray = load(bytes)?;
        let corners = find_markers(&gray).ok_or(OmrError::Markers)?;
        // Page corners in millimetres (marker centres): TL, TR, BR, BL.
        let m = f64::from(layout.margin + layout.marker / 2.0);
        let page = [
            (m, m),
            (210.0 - m, m),
            (210.0 - m, 297.0 - m),
            (m, 297.0 - m),
        ];
        let mut scans = Vec::new();
        for turn in 0..4 {
            let image: Vec<(f64, f64)> = (0..4).map(|i| corners[(i + turn) % 4]).collect();
            let Some(homography) = solve_homography(&page, &image) else {
                continue;
            };
            let mut scan = CardScan {
                gray: gray.clone(),
                homography,
                white: 1.0,
                black: 0.0,
                layout: layout.clone(),
                code: None,
            };
            scan.calibrate();
            scan.code = scan.read_code();
            if scan.code.is_some() {
                return Ok(scan);
            }
            scans.push(scan);
        }
        // No code read: the first way round that stands upright.
        scans
            .into_iter()
            .find(CardScan::portrait)
            .ok_or(OmrError::Markers)
    }

    /// The exam number printed on the card, when its cells read cleanly.
    pub fn code(&self) -> Option<u32> {
        self.code
    }

    /// Each of `total` questions with `marks` bubbles.
    pub fn answers(&self, total: u32, marks: u32) -> Vec<BubbleRead> {
        let radius = self.layout.bubble / 2.0 * 0.55;
        (0..total)
            .map(|q| {
                let darkness: Vec<f32> = (0..marks)
                    .map(|mark| {
                        let [x, y] = self.layout.bubble_centre(q, mark);
                        self.darkness(f64::from(x), f64::from(y), f64::from(radius))
                    })
                    .collect();
                let filled: Vec<usize> = darkness
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| **d >= FILLED)
                    .map(|(i, _)| i)
                    .collect();
                let faint = darkness.iter().any(|d| (FAINT..FILLED).contains(d));
                BubbleRead {
                    choice: match filled.as_slice() {
                        [one] => u8::try_from(*one).ok(),
                        _ => None,
                    },
                    doubtful: filled.len() > 1 || faint,
                    darkness,
                }
            })
            .collect()
    }

    /// The page's height on the photo is longer than its width.
    fn portrait(&self) -> bool {
        let (ax, ay) = self.map(0.0, 0.0);
        let (bx, by) = self.map(210.0, 0.0);
        let (cx, cy) = self.map(0.0, 297.0);
        (cx - ax).hypot(cy - ay) > (bx - ax).hypot(by - ay)
    }

    /// Paper white from blank spots of the card, black from the markers.
    fn calibrate(&mut self) {
        let layout = &self.layout;
        let m = f64::from(layout.margin + layout.marker / 2.0);
        let r = f64::from(layout.marker) * 0.3;
        let blacks: Vec<f32> = [
            (m, m),
            (210.0 - m, m),
            (210.0 - m, 297.0 - m),
            (m, 297.0 - m),
        ]
        .iter()
        .map(|(x, y)| self.mean(*x, *y, r))
        .collect();
        // Margins beside the markers and the gap between columns.
        let whites: Vec<f32> = [
            (105.0, 4.0),
            (105.0, 293.0),
            (4.0, 148.5),
            (206.0, 148.5),
            (105.0, 30.0),
        ]
        .iter()
        .map(|(x, y)| self.mean(*x, *y, 1.5))
        .collect();
        self.black = blacks.iter().sum::<f32>() / blacks.len() as f32;
        self.white = whites.iter().copied().fold(f32::MIN, f32::max);
        if self.white - self.black < 0.1 {
            self.white = 1.0;
            self.black = 0.0;
        }
    }

    fn read_code(&self) -> Option<u32> {
        let layout = &self.layout;
        let half = f64::from(layout.code_cell) / 2.0;
        let cells: Vec<f32> = (0..CODE_BITS + CHECK_BITS)
            .map(|bit| {
                let x = f64::from(layout.code_left + bit as f32 * layout.code_pitch) + half;
                self.darkness(x, f64::from(layout.code_top) + half, half * 0.6)
            })
            .collect();
        // Every cell clearly filled or clearly empty.
        if cells.iter().any(|d| (0.3..0.6).contains(d)) || cells.iter().all(|d| *d < 0.3) {
            return None;
        }
        let value = cells
            .iter()
            .enumerate()
            .filter(|(_, d)| **d >= 0.6)
            .fold(0u32, |acc, (bit, _)| acc | (1 << bit));
        let id = value & ((1 << CODE_BITS) - 1);
        (value >> CODE_BITS == check(id)).then_some(id)
    }

    /// Darkness around a point, `0.0` paper to `1.0` marker.
    fn darkness(&self, x: f64, y: f64, radius: f64) -> f32 {
        let value = self.mean(x, y, radius);
        ((self.white - value) / (self.white - self.black)).clamp(0.0, 1.0)
    }

    /// Mean brightness (`0.0..=1.0`) of a disc in card millimetres.
    fn mean(&self, x: f64, y: f64, radius: f64) -> f32 {
        let steps = 6;
        let mut sum = 0.0;
        let mut count = 0.0;
        for i in -steps..=steps {
            for j in -steps..=steps {
                let dx = radius * f64::from(i) / f64::from(steps);
                let dy = radius * f64::from(j) / f64::from(steps);
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let (px, py) = self.map(x + dx, y + dy);
                if let Some(v) = self.pixel(px, py) {
                    sum += v;
                    count += 1.0;
                }
            }
        }
        if count > 0.0 { sum / count } else { 1.0 }
    }

    fn pixel(&self, x: f64, y: f64) -> Option<f32> {
        let (w, h) = self.gray.dimensions();
        if x < 0.0 || y < 0.0 || x >= f64::from(w) || y >= f64::from(h) {
            return None;
        }
        Some(f32::from(self.gray.get_pixel(x as u32, y as u32)[0]) / 255.0)
    }

    fn map(&self, x: f64, y: f64) -> (f64, f64) {
        let h = &self.homography;
        let w = h[6] * x + h[7] * y + h[8];
        (
            (h[0] * x + h[1] * y + h[2]) / w,
            (h[3] * x + h[4] * y + h[5]) / w,
        )
    }
}

/// Decodes, turns as the camera says, reduces and greys the photo.
fn load(bytes: &[u8]) -> Result<GrayImage, OmrError> {
    let err = |e: image::ImageError| OmrError::Image(e.to_string());
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| OmrError::Image(e.to_string()))?;
    let mut decoder = reader.into_decoder().map_err(err)?;
    let orientation = decoder.orientation().map_err(err)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(err)?;
    image.apply_orientation(orientation);
    let (w, h) = (image.width(), image.height());
    let long = w.max(h);
    let image = if long > WORK_SIDE {
        let scale = f64::from(WORK_SIDE) / f64::from(long);
        image.resize(
            (f64::from(w) * scale) as u32,
            (f64::from(h) * scale) as u32,
            imageops::FilterType::Triangle,
        )
    } else {
        image
    };
    Ok(image.to_luma8())
}

/// Otsu's threshold of the image's histogram.
fn otsu(gray: &GrayImage) -> u8 {
    let mut histogram = [0u64; 256];
    for p in gray.pixels() {
        histogram[usize::from(p[0])] += 1;
    }
    let total: u64 = histogram.iter().sum();
    let sum: f64 = histogram
        .iter()
        .enumerate()
        .map(|(i, n)| i as f64 * *n as f64)
        .sum();
    let (mut below, mut below_sum, mut best, mut threshold) = (0u64, 0.0, 0.0, 128u8);
    for (i, n) in histogram.iter().enumerate() {
        below += n;
        if below == 0 || below == total {
            continue;
        }
        below_sum += i as f64 * *n as f64;
        let above = total - below;
        let mean_below = below_sum / below as f64;
        let mean_above = (sum - below_sum) / above as f64;
        let between = below as f64 * above as f64 * (mean_below - mean_above).powi(2);
        if between > best {
            best = between;
            threshold = u8::try_from(i).unwrap_or(128);
        }
    }
    threshold
}

struct Blob {
    area: u32,
    min: (u32, u32),
    max: (u32, u32),
    sum: (f64, f64),
}

/// Centres of the four corner markers, as they sit on the photo: top left,
/// top right, bottom right, bottom left.
fn find_markers(gray: &GrayImage) -> Option<[(f64, f64); 4]> {
    let (w, h) = gray.dimensions();
    let threshold = otsu(gray);
    let dark: Vec<bool> = gray.pixels().map(|p| p[0] <= threshold).collect();
    let mut seen = vec![false; dark.len()];
    let mut blobs = Vec::new();
    let mut stack = Vec::new();
    for start in 0..dark.len() {
        if !dark[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut blob = Blob {
            area: 0,
            min: (u32::MAX, u32::MAX),
            max: (0, 0),
            sum: (0.0, 0.0),
        };
        while let Some(i) = stack.pop() {
            let (x, y) = ((i as u32) % w, (i as u32) / w);
            blob.area += 1;
            blob.min = (blob.min.0.min(x), blob.min.1.min(y));
            blob.max = (blob.max.0.max(x), blob.max.1.max(y));
            blob.sum.0 += f64::from(x);
            blob.sum.1 += f64::from(y);
            let mut push = |nx: u32, ny: u32| {
                let j = (ny * w + nx) as usize;
                if dark[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                push(x - 1, y);
            }
            if x + 1 < w {
                push(x + 1, y);
            }
            if y > 0 {
                push(x, y - 1);
            }
            if y + 1 < h {
                push(x, y + 1);
            }
        }
        blobs.push(blob);
    }
    let smallest = (w.min(h) / 120).max(4);
    let squares: Vec<&Blob> = blobs
        .iter()
        .filter(|b| {
            let bw = b.max.0 - b.min.0 + 1;
            let bh = b.max.1 - b.min.1 + 1;
            let aspect = f64::from(bw) / f64::from(bh);
            let fill = f64::from(b.area) / (f64::from(bw) * f64::from(bh));
            b.min.0 > 0
                && b.min.1 > 0
                && b.max.0 + 1 < w
                && b.max.1 + 1 < h
                && bw >= smallest
                && bh >= smallest
                && bw < w / 6
                && bh < h / 6
                && (0.6..=1.6).contains(&aspect)
                && fill >= 0.8
        })
        .collect();
    let largest = squares.iter().map(|b| b.area).max()?;
    let markers: Vec<(f64, f64)> = squares
        .iter()
        .filter(|b| f64::from(b.area) >= f64::from(largest) * 0.4)
        .map(|b| (b.sum.0 / f64::from(b.area), b.sum.1 / f64::from(b.area)))
        .collect();
    if markers.len() < 4 {
        return None;
    }
    // The candidate nearest each corner of the photo.
    let (fw, fh) = (f64::from(w), f64::from(h));
    let mut found = [(0.0, 0.0); 4];
    for (slot, corner) in [(0.0, 0.0), (fw, 0.0), (fw, fh), (0.0, fh)]
        .iter()
        .enumerate()
    {
        found[slot] = *markers.iter().min_by(|a, b| {
            let da = (a.0 - corner.0).hypot(a.1 - corner.1);
            let db = (b.0 - corner.0).hypot(b.1 - corner.1);
            da.total_cmp(&db)
        })?;
    }
    let distinct = (0..4).all(|i| (i + 1..4).all(|j| found[i] != found[j]));
    distinct.then_some(found)
}

/// The homography taking `from[i]` to `to[i]` (four points each).
fn solve_homography(from: &[(f64, f64); 4], to: &[(f64, f64)]) -> Option<[f64; 9]> {
    let mut a = [[0.0f64; 9]; 8];
    for i in 0..4 {
        let (x, y) = from[i];
        let (u, v) = *to.get(i)?;
        a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }
    // Gaussian elimination with partial pivoting.
    for col in 0..8 {
        let pivot = (col..8).max_by(|&r, &s| a[r][col].abs().total_cmp(&a[s][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        for row in 0..8 {
            if row != col {
                let factor = a[row][col] / a[col][col];
                let pivot_row = a[col];
                for (cell, pivot) in a[row].iter_mut().zip(pivot_row).skip(col) {
                    *cell -= factor * pivot;
                }
            }
        }
    }
    let mut h = [0.0; 9];
    for (i, (value, row)) in h.iter_mut().zip(&a).enumerate() {
        *value = row[8] / row[i];
    }
    h[8] = 1.0;
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EssaySheet, ExamDocument, ExamLabels, ExamPageQuestion, ExamPageSection};

    const PX_PER_PT: f32 = 2.0;

    fn document(id: u32, total: u32) -> ExamDocument {
        let card = CardLayout::new(format!("KG-{id}")).with_id(id);
        ExamDocument {
            title: "Simulado".into(),
            subtitle: String::new(),
            lang: "pt".into(),
            labels: ExamLabels::default(),
            duration: "1 h".into(),
            instructions: Vec::new(),
            sections: vec![ExamPageSection {
                subject: "Matemática".into(),
                questions: (1..=total)
                    .map(|n| ExamPageQuestion {
                        number: n,
                        prompt: format!("Questão {n}"),
                        options: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
                    })
                    .collect(),
            }],
            marks: ["A", "B", "C", "D", "E"].map(String::from).to_vec(),
            total,
            essay: None::<EssaySheet>,
            card,
        }
    }

    /// The card page with `marks` filled in by pen.
    fn filled_card(document: &ExamDocument, marks: &[(u32, u32)]) -> GrayImage {
        let pages = crate::render_exam_pages(document, PX_PER_PT).unwrap();
        let card = pages.last().unwrap();
        let mut gray = GrayImage::from_fn(card.width, card.height, |x, y| {
            let i = ((y * card.width + x) * 4) as usize;
            let [r, g, b] = [card.rgba[i], card.rgba[i + 1], card.rgba[i + 2]];
            image::Luma([((u32::from(r) + u32::from(g) + u32::from(b)) / 3) as u8])
        });
        let px_per_mm = PX_PER_PT * 72.0 / 25.4;
        for &(q, mark) in marks {
            let [cx, cy] = document.card.bubble_centre(q, mark);
            let (cx, cy) = (cx * px_per_mm, cy * px_per_mm);
            let r = document.card.bubble / 2.0 * px_per_mm * 0.85;
            for y in (cy - r) as u32..=(cy + r) as u32 {
                for x in (cx - r) as u32..=(cx + r) as u32 {
                    if (x as f32 - cx).hypot(y as f32 - cy) <= r {
                        gray.put_pixel(x, y, image::Luma([40]));
                    }
                }
            }
        }
        gray
    }

    /// Photographs `card` on a grey table: shrunk, tilted in perspective.
    fn photograph(card: &GrayImage, turned: bool) -> Vec<u8> {
        let (cw, ch) = (f64::from(card.width()), f64::from(card.height()));
        let (w, h) = (1500u32, 1900u32);
        // Where the card's corners land in the photo.
        let mut corners = [
            (180.0, 150.0),
            (1290.0, 210.0),
            (1340.0, 1760.0),
            (130.0, 1700.0),
        ];
        if turned {
            corners.rotate_left(2);
        }
        let page = [(0.0, 0.0), (cw, 0.0), (cw, ch), (0.0, ch)];
        // Photo pixel → card pixel.
        let back = solve_homography(&corners, &page).unwrap();
        let photo = GrayImage::from_fn(w, h, |x, y| {
            let (x, y) = (f64::from(x), f64::from(y));
            let d = back[6] * x + back[7] * y + back[8];
            let (u, v) = (
                (back[0] * x + back[1] * y + back[2]) / d,
                (back[3] * x + back[4] * y + back[5]) / d,
            );
            if u >= 0.0 && v >= 0.0 && u < cw && v < ch {
                let p = card.get_pixel(u as u32, v as u32)[0];
                // A little shade across the sheet.
                image::Luma([(f64::from(p) * (0.85 + 0.1 * x / f64::from(w))) as u8])
            } else {
                image::Luma([70])
            }
        });
        let mut bytes = Vec::new();
        DynamicImage::ImageLuma8(photo)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn the_code_cells_carry_the_number_and_a_check() {
        let cells = code_cells(1234);
        assert_eq!(cells.len(), 24);
        let value = cells
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .fold(0u32, |acc, (bit, _)| acc | (1 << bit));
        assert_eq!(value & 0xFFFFF, 1234);
        assert_eq!(value >> 20, check(1234));
    }

    #[test]
    fn a_tilted_photo_of_a_filled_card_reads_back() {
        let document = document(4321, 45);
        let marks = [(0, 0), (1, 3), (2, 4), (39, 1), (40, 2), (44, 0), (44, 2)];
        let card = filled_card(&document, &marks);
        for turned in [false, true] {
            let scan = CardScan::open(&photograph(&card, turned), &document.card).unwrap();
            assert_eq!(scan.code(), Some(4321), "turned: {turned}");
            let answers = scan.answers(45, 5);
            assert_eq!(answers[0].choice, Some(0));
            assert_eq!(answers[1].choice, Some(3));
            assert_eq!(answers[2].choice, Some(4));
            assert_eq!(answers[39].choice, Some(1));
            assert_eq!(answers[40].choice, Some(2));
            assert_eq!(answers[3].choice, None, "left blank");
            assert!(!answers[3].doubtful);
            assert_eq!(answers[44].choice, None, "two marks");
            assert!(answers[44].doubtful);
            assert!(
                answers.iter().filter(|a| a.doubtful).count() == 1,
                "only the double mark is doubtful"
            );
        }
    }

    #[test]
    fn a_photo_without_markers_is_refused() {
        let blank = GrayImage::from_pixel(800, 1000, image::Luma([230]));
        let mut bytes = Vec::new();
        DynamicImage::ImageLuma8(blank)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        let layout = CardLayout::new("KG-1");
        assert!(matches!(
            CardScan::open(&bytes, &layout),
            Err(OmrError::Markers)
        ));
        assert!(matches!(
            CardScan::open(b"not an image", &layout),
            Err(OmrError::Image(_))
        ));
    }
}
