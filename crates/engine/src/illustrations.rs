//! Pictures for the lesson whiteboard, from Portuguese Wikipedia.
//!
//! The tutor writes "IMAGEM: <article title>" and the lesson opens with the
//! topic's title. Only that search term leaves the computer; results are
//! cached on disk so a picture is fetched once.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{Engine, Event};

const API: &str = "https://pt.wikipedia.org/w/api.php";

/// Width of the thumbnail asked from Wikimedia.
const THUMB_WIDTH: u32 = 480;
/// Largest picture accepted.
const MAX_BYTES: u64 = 3 * 1024 * 1024;
/// Longest search term sent.
const MAX_QUERY_CHARS: usize = 120;

/// A picture ready to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardImage {
    pub bytes: Arc<[u8]>,
    /// File extension of `bytes` ("jpg" or "png"), for the image loader.
    pub format: &'static str,
    /// Wikipedia article the picture illustrates.
    pub title: String,
    /// Article address, where the picture's author and license are listed.
    pub page_url: String,
    /// The picture file's author and license, from Wikimedia; `None` for
    /// pictures cached before credits were looked up, or when the lookup failed.
    pub credit: Option<ImageCredit>,
}

/// Who made a Wikimedia picture and under which license.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageCredit {
    /// Author as plain text; may be empty.
    pub author: String,
    /// Short license name, e.g. "CC BY-SA 4.0".
    pub license: String,
    /// File page on Wikimedia Commons (or the wiki).
    pub file_url: String,
    /// File title without its namespace, e.g. "Palacio do Planalto.jpg".
    pub file: String,
}

/// State of one search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Illustration {
    Loading,
    Ready(BoardImage),
    /// Nothing suitable, or the lookup failed: the board skips the entry.
    Missing,
}

/// Every search of this session, by normalized term.
#[derive(Default)]
pub(crate) struct Illustrations {
    found: HashMap<String, Illustration>,
}

/// Search term as the board shows it, used as the key.
pub fn illustration_key(query: &str) -> String {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect()
}

impl Engine {
    /// The picture for a board "IMAGEM:" entry, when one was looked up.
    pub fn illustration(&self, query: &str) -> Option<&Illustration> {
        self.illustrations.found.get(&illustration_key(query))
    }

    /// Starts looking up `query` unless it is known or pictures are off.
    pub(crate) fn request_illustration(&mut self, query: &str) {
        let key = illustration_key(query);
        if key.is_empty()
            || !self.settings.board_images
            || self.illustrations.found.contains_key(&key)
        {
            return;
        }
        self.illustrations
            .found
            .insert(key.clone(), Illustration::Loading);
        let cache = self.paths.data_dir.join("board-images");
        self.spawn(move || {
            let result = lookup(&cache, &key);
            if let Err(err) = &result {
                tracing::warn!(%err, query = %key, "board picture not found");
            }
            Event::Illustration {
                query: key,
                image: result.ok().flatten(),
            }
        });
    }

    pub(crate) fn on_illustration(&mut self, query: String, image: Option<BoardImage>) {
        let state = image.map_or(Illustration::Missing, Illustration::Ready);
        self.illustrations.found.insert(query, state);
    }
}

/// Pictures shown per topic in a handout: the topic's own, then sections.
const HANDOUT_PICTURES: usize = 3;

/// Looks up the handout's pictures and attaches them: one of each topic, and
/// those the sections ask for. Topics or sections without a fitting picture
/// are left as they are. Runs on a worker thread (it may use the network).
///
/// Handouts made with the bibliography labels also credit each picture's
/// author and license and list its Wikipedia article and file in the topic's
/// references. Older saved handouts keep their captions and references, so
/// their pages (and the highlights on them) do not move.
pub(crate) fn illustrate(document: &mut kg_handout::Document, cache: &Path) {
    let source = document.labels.picture_source.clone();
    let cite = Citer::new(&document.labels);
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut find = |query: &str| -> Option<(kg_handout::Picture, Vec<String>)> {
        let query = illustration_key(query);
        if query.is_empty() {
            return None;
        }
        let image = match lookup(cache, &query) {
            Ok(image) => image?,
            Err(err) => {
                tracing::warn!(%err, %query, "handout picture not found");
                return None;
            }
        };
        let file = format!("/img/{}.{}", files.len() + 1, image.format);
        files.push((file.clone(), image.bytes.to_vec()));
        let mut caption = if source.is_empty() {
            image.title.clone()
        } else {
            format!("{source} · {}", image.title)
        };
        let references = match &cite {
            Some(cite) => {
                if let Some(credit) = cite.credit_line(&image) {
                    caption = format!("{caption} · {credit}");
                }
                cite.references(&image)
            }
            None => Vec::new(),
        };
        Some((kg_handout::Picture { file, caption }, references))
    };
    for topic in &mut document.topics {
        let mut sources = Vec::new();
        let mut keep = |found: Option<(kg_handout::Picture, Vec<String>)>| {
            found.map(|(picture, references)| {
                sources.extend(references);
                picture
            })
        };
        topic.picture = keep(find(&topic.topic));
        let mut left = HANDOUT_PICTURES - usize::from(topic.picture.is_some());
        for section in &mut topic.sections {
            if left == 0 {
                break;
            }
            if !section.image.trim().is_empty() {
                section.picture = keep(find(&section.image));
                left -= usize::from(section.picture.is_some());
            }
        }
        for reference in sources {
            if !topic.references.contains(&reference) {
                topic.references.push(reference);
            }
        }
    }
    document.files = files;
}

/// Writes the picture credits and Wikipedia references of a handout, from
/// its translated templates.
struct Citer {
    article: String,
    image: String,
}

impl Citer {
    /// `None` for handouts saved before the templates existed.
    fn new(labels: &kg_handout::Labels) -> Option<Self> {
        (!labels.wikipedia_reference.trim().is_empty()).then(|| Self {
            article: labels.wikipedia_reference.clone(),
            image: labels.image_reference.clone(),
        })
    }

    /// "Author, license" for the caption.
    fn credit_line(&self, image: &BoardImage) -> Option<String> {
        let credit = image.credit.as_ref()?;
        let parts: Vec<&str> = [credit.author.as_str(), credit.license.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// The article, then the picture file when its credit is known.
    fn references(&self, image: &BoardImage) -> Vec<String> {
        let mut references = Vec::new();
        if !image.page_url.is_empty() {
            references.push(
                self.article
                    .replace("{title}", &abnt_title(&image.title))
                    .replace("{url}", &image.page_url),
            );
        }
        if let Some(credit) = &image.credit
            && !self.image.trim().is_empty()
            && !credit.file_url.is_empty()
        {
            let template = if credit.author.is_empty() {
                self.image.replace("{author}. ", "")
            } else {
                self.image.clone()
            };
            let license = if credit.license.is_empty() {
                "?"
            } else {
                credit.license.as_str()
            };
            references.push(
                template
                    .replace("{author}", &credit.author)
                    .replace("{file}", &credit.file)
                    .replace("{license}", license)
                    .replace("{url}", &credit.file_url),
            );
        }
        references
    }
}

/// A work listed by its title (ABNT): the first word in capitals.
fn abnt_title(title: &str) -> String {
    let title = title.trim();
    match title.split_once(' ') {
        Some((first, rest)) => format!("{} {rest}", first.to_uppercase()),
        None => title.to_uppercase(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedMeta {
    title: String,
    page_url: String,
    format: String,
    #[serde(default)]
    credit: Option<ImageCredit>,
}

/// Cached result, or a Wikipedia search. `Ok(None)` when nothing fits.
fn lookup(cache: &Path, query: &str) -> Result<Option<BoardImage>, String> {
    let stem = cache.join(format!("{:016x}", fnv(query)));
    let meta_path = stem.with_extension("json");
    if let Ok(json) = fs::read_to_string(&meta_path) {
        if json.trim() == "null" {
            return Ok(None);
        }
        if let Ok(meta) = serde_json::from_str::<CachedMeta>(&json)
            && let Some(format) = known_format(&meta.format)
            && let Ok(bytes) = fs::read(stem.with_extension(format))
        {
            return Ok(Some(BoardImage {
                bytes: bytes.into(),
                format,
                title: meta.title,
                page_url: meta.page_url,
                credit: meta.credit,
            }));
        }
    }
    let found = search(query)?;
    let _ = fs::create_dir_all(cache);
    match &found {
        Some(image) => {
            let meta = CachedMeta {
                title: image.title.clone(),
                page_url: image.page_url.clone(),
                format: image.format.to_owned(),
                credit: image.credit.clone(),
            };
            let _ = fs::write(stem.with_extension(image.format), &image.bytes);
            if let Ok(json) = serde_json::to_string(&meta) {
                let _ = fs::write(&meta_path, json);
            }
        }
        // Remember misses too, so the same term is not searched again.
        None => {
            let _ = fs::write(&meta_path, "null");
        }
    }
    Ok(found)
}

#[derive(Debug, Deserialize)]
struct SearchReply {
    #[serde(default)]
    query: Option<SearchPages>,
}

#[derive(Debug, Deserialize)]
struct SearchPages {
    #[serde(default)]
    pages: Vec<Page>,
}

#[derive(Debug, Deserialize)]
struct Page {
    title: String,
    #[serde(default)]
    index: u32,
    #[serde(default)]
    fullurl: String,
    #[serde(default)]
    thumbnail: Option<Thumbnail>,
    /// The picture's file name, without its namespace.
    #[serde(default)]
    pageimage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Thumbnail {
    source: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}

fn search(query: &str) -> Result<Option<BoardImage>, String> {
    let agent = agent();
    let mut response = agent
        .get(API)
        .header("User-Agent", kg_brand::user_agent("desktop study app"))
        .query("action", "query")
        .query("format", "json")
        .query("formatversion", "2")
        .query("generator", "search")
        .query("gsrsearch", query)
        .query("gsrnamespace", "0")
        .query("gsrlimit", "4")
        .query("prop", "pageimages|info")
        .query("piprop", "thumbnail|name")
        .query("pithumbsize", THUMB_WIDTH.to_string())
        .query("inprop", "url")
        .call()
        .map_err(|e| e.to_string())?;
    let reply: SearchReply = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_json()
        .map_err(|e| e.to_string())?;
    let mut pages = reply.query.map(|q| q.pages).unwrap_or_default();
    pages.sort_by_key(|p| p.index);
    let Some((page, thumb)) = pages
        .into_iter()
        .filter(|page| related(query, &page.title))
        .find_map(|page| {
            let source = page.thumbnail.as_ref()?.source.clone();
            Some((page, source))
        })
    else {
        return Ok(None);
    };
    let Some(format) = format_of(&thumb) else {
        return Ok(None);
    };
    let bytes = agent
        .get(&thumb)
        .header("User-Agent", kg_brand::user_agent("desktop study app"))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    // The picture is shown without its credit rather than not at all.
    let credit = page
        .pageimage
        .as_deref()
        .and_then(|file| match file_credit(&agent, file) {
            Ok(credit) => credit,
            Err(err) => {
                tracing::warn!(%err, file, "picture credit not found");
                None
            }
        });
    Ok(Some(BoardImage {
        bytes: bytes.into(),
        format,
        title: page.title,
        page_url: page.fullurl,
        credit,
    }))
}

#[derive(Debug, Deserialize)]
struct FileReply {
    #[serde(default)]
    query: Option<FilePages>,
}

#[derive(Debug, Deserialize)]
struct FilePages {
    #[serde(default)]
    pages: Vec<FilePage>,
}

#[derive(Debug, Deserialize)]
struct FilePage {
    #[serde(default)]
    imageinfo: Vec<ImageInfo>,
}

#[derive(Debug, Deserialize)]
struct ImageInfo {
    #[serde(default)]
    descriptionurl: String,
    #[serde(default)]
    extmetadata: HashMap<String, MetaValue>,
}

#[derive(Debug, Deserialize)]
struct MetaValue {
    #[serde(default)]
    value: serde_json::Value,
}

/// The author and license Wikimedia lists for a picture file.
fn file_credit(agent: &ureq::Agent, file: &str) -> Result<Option<ImageCredit>, String> {
    let reply: FileReply = agent
        .get(API)
        .header("User-Agent", kg_brand::user_agent("desktop study app"))
        .query("action", "query")
        .query("format", "json")
        .query("formatversion", "2")
        .query("titles", format!("File:{file}"))
        .query("prop", "imageinfo")
        .query("iiprop", "extmetadata|url")
        .query("iiextmetadatafilter", "Artist|LicenseShortName")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(256 * 1024)
        .read_json()
        .map_err(|e| e.to_string())?;
    Ok(credit_from(reply, file))
}

fn credit_from(reply: FileReply, file: &str) -> Option<ImageCredit> {
    let info = reply
        .query?
        .pages
        .into_iter()
        .find_map(|page| page.imageinfo.into_iter().next())?;
    let field = |name: &str| {
        info.extmetadata
            .get(name)
            .and_then(|meta| meta.value.as_str())
            .map(plain_text)
            .unwrap_or_default()
    };
    let license = field("LicenseShortName");
    let author = field("Artist");
    if license.is_empty() && author.is_empty() {
        return None;
    }
    Some(ImageCredit {
        author,
        license,
        file_url: info.descriptionurl,
        // Wiki file names use "_" for spaces.
        file: file.replace('_', " "),
    })
}

/// Longest author or license kept, in characters.
const MAX_CREDIT_CHARS: usize = 120;

/// Wikimedia's HTML fields as one line of plain text.
fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let line = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".");
    line.chars().take(MAX_CREDIT_CHARS).collect()
}

/// The article shares a meaningful word with the search, so a loose match
/// on an unrelated page is not shown. Words are compared by their first
/// letters, so "redes neurais" matches "rede neural".
fn related(query: &str, title: &str) -> bool {
    let words = |text: &str| -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 4 || w.chars().all(|c| c.is_ascii_digit()))
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase().chars().take(5).collect())
            .collect()
    };
    let title = words(title);
    words(query).iter().any(|w| title.contains(w))
}

fn format_of(url: &str) -> Option<&'static str> {
    // Thumbnail addresses carry tracking parameters after the file name.
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let lower = path.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("jpg")
    } else if lower.ends_with(".png") {
        Some("png")
    } else {
        None
    }
}

fn known_format(format: &str) -> Option<&'static str> {
    match format {
        "jpg" => Some("jpg"),
        "png" => Some("png"),
        _ => None,
    }
}

/// Stable file name for a search term.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handout_gets_pictures_from_the_cache() {
        let cache = tempfile::tempdir().unwrap();
        let put = |query: &str, meta: &str, bytes: Option<&[u8]>| {
            let stem = cache.path().join(format!("{:016x}", fnv(query)));
            fs::write(stem.with_extension("json"), meta).unwrap();
            if let Some(bytes) = bytes {
                fs::write(stem.with_extension("png"), bytes).unwrap();
            }
        };
        put(
            "Crase",
            r#"{"title":"Crase","page_url":"https://pt.wikipedia.org/wiki/Crase","format":"png"}"#,
            Some(b"png-bytes"),
        );
        put("Escola", "null", None);
        let mut document = kg_handout::Document {
            title: String::new(),
            subtitle: String::new(),
            generated: String::new(),
            lang: "pt".into(),
            labels: kg_handout::Labels {
                picture_source: "Imagem: Wikipédia".into(),
                ..Default::default()
            },
            topics: vec![kg_handout::TopicHandout {
                topic: "Crase".into(),
                sections: vec![kg_handout::Section {
                    heading: "Uso".into(),
                    image: "Escola".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            files: Vec::new(),
            marks: Vec::new(),
        };
        illustrate(&mut document, cache.path());
        let topic = &document.topics[0];
        let picture = topic.picture.as_ref().unwrap();
        assert_eq!(picture.caption, "Imagem: Wikipédia · Crase");
        assert_eq!(
            document.files,
            [(picture.file.clone(), b"png-bytes".to_vec())]
        );
        // Known to have no picture: the section stays as it is.
        assert_eq!(topic.sections[0].picture, None);
    }

    #[test]
    fn new_handouts_credit_pictures_and_cite_their_wikipedia_sources() {
        let cache = tempfile::tempdir().unwrap();
        let stem = cache
            .path()
            .join(format!("{:016x}", fnv("Palácio do Planalto")));
        fs::write(stem.with_extension("jpg"), b"jpg-bytes").unwrap();
        fs::write(
            stem.with_extension("json"),
            r#"{"title":"Palácio do Planalto","page_url":"https://pt.wikipedia.org/wiki/Pal%C3%A1cio_do_Planalto","format":"jpg",
               "credit":{"author":"Ana Lima","license":"CC BY-SA 4.0","file_url":"https://commons.wikimedia.org/wiki/File:Planalto.jpg","file":"Planalto.jpg"}}"#,
        )
        .unwrap();
        let mut document = kg_handout::Document {
            title: String::new(),
            subtitle: String::new(),
            generated: String::new(),
            lang: "pt".into(),
            labels: kg_handout::Labels {
                picture_source: "Imagem: Wikipédia".into(),
                wikipedia_reference:
                    "{title}. In: WIKIPÉDIA. Disponível em: {url}. Acesso em: 11 nov. 2026.".into(),
                image_reference: "{author}. {file}. Licença {license}. Disponível em: {url}."
                    .into(),
                ..Default::default()
            },
            topics: vec![kg_handout::TopicHandout {
                topic: "Palácio do Planalto".into(),
                references: vec!["BRASIL. Constituição (1988).".into()],
                ..Default::default()
            }],
            files: Vec::new(),
            marks: Vec::new(),
        };
        illustrate(&mut document, cache.path());
        // Opening it again does not list the sources twice.
        illustrate(&mut document, cache.path());
        let topic = &document.topics[0];
        assert_eq!(
            topic.picture.as_ref().unwrap().caption,
            "Imagem: Wikipédia · Palácio do Planalto · Ana Lima, CC BY-SA 4.0"
        );
        assert_eq!(
            topic.references,
            [
                "BRASIL. Constituição (1988).",
                "PALÁCIO do Planalto. In: WIKIPÉDIA. Disponível em: https://pt.wikipedia.org/wiki/Pal%C3%A1cio_do_Planalto. Acesso em: 11 nov. 2026.",
                "Ana Lima. Planalto.jpg. Licença CC BY-SA 4.0. Disponível em: https://commons.wikimedia.org/wiki/File:Planalto.jpg.",
            ]
        );
    }

    #[test]
    fn reads_the_author_and_license_wikimedia_lists() {
        let reply: FileReply = serde_json::from_str(
            r#"{"query":{"pages":[{"title":"Ficheiro:X.jpg","imageinfo":[{
                "descriptionurl":"https://commons.wikimedia.org/wiki/File:X.jpg",
                "extmetadata":{"Artist":{"value":"<a href=\"//commons.wikimedia.org/wiki/User:Ana\">Ana &amp; Bia</a> , Caio\n"},
                               "LicenseShortName":{"value":"CC BY-SA 4.0"}}}]}]}}"#,
        )
        .unwrap();
        assert_eq!(
            credit_from(reply, "X_1.jpg"),
            Some(ImageCredit {
                author: "Ana & Bia, Caio".into(),
                license: "CC BY-SA 4.0".into(),
                file_url: "https://commons.wikimedia.org/wiki/File:X.jpg".into(),
                file: "X 1.jpg".into(),
            })
        );
        let empty: FileReply =
            serde_json::from_str(r#"{"query":{"pages":[{"imageinfo":[{"extmetadata":{}}]}]}}"#)
                .unwrap();
        assert_eq!(credit_from(empty, "Y.jpg"), None);
        assert_eq!(abnt_title("Crase"), "CRASE");
    }

    #[test]
    fn keeps_only_related_articles_and_known_formats() {
        assert!(related("Palácio do Planalto", "Palácio do Planalto"));
        assert!(related(
            "Constituição de 1988",
            "Constituição brasileira de 1988"
        ));
        assert!(!related("Lei de Responsabilidade Fiscal", "Futebol"));
        assert!(related(
            "Redes neurais artificiais",
            "Rede neural artificial"
        ));
        assert_eq!(
            format_of("https://upload.wikimedia.org/a/b/480px-X.jpg?utm_source=pt.wikipedia.org"),
            Some("jpg")
        );
        assert_eq!(format_of("https://x/y/400px-Foo.JPG"), Some("jpg"));
        assert_eq!(format_of("https://x/y/400px-Foo.svg.png"), Some("png"));
        assert_eq!(format_of("https://x/y/400px-Foo.gif"), None);
        assert_eq!(
            illustration_key("  Palácio   do Planalto "),
            "Palácio do Planalto"
        );
    }

    #[test]
    #[ignore = "reaches pt.wikipedia.org; run with --ignored"]
    fn live_search_finds_pictures() {
        for query in ["Palácio do Planalto", "Redes neurais artificiais"] {
            let image = search(query).unwrap().unwrap();
            assert!(image.bytes.len() > 1_000, "{query}");
            let magic: &[u8] = if image.format == "png" {
                b"\x89PNG"
            } else {
                b"\xFF\xD8"
            };
            assert!(image.bytes.starts_with(magic), "{query}");
            let credit = image.credit.as_ref().expect("Wikimedia lists a license");
            assert!(!credit.license.is_empty(), "{query}");
            println!("  credit: {credit:?}");
            println!(
                "{query}: {} ({}, {} bytes)",
                image.title,
                image.format,
                image.bytes.len()
            );
        }
        assert_eq!(search("xqzv kkkwpq zzzt").unwrap(), None);
    }

    #[test]
    fn cached_results_are_read_back_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let stem = dir.path().join(format!("{:016x}", fnv("Brasília")));
        std::fs::write(stem.with_extension("png"), [1u8, 2, 3]).unwrap();
        std::fs::write(
            stem.with_extension("json"),
            r#"{"title":"Brasília","page_url":"https://pt.wikipedia.org/wiki/Bras%C3%ADlia","format":"png"}"#,
        )
        .unwrap();
        let image = lookup(dir.path(), "Brasília").unwrap().unwrap();
        assert_eq!(&*image.bytes, &[1, 2, 3]);
        assert_eq!(image.title, "Brasília");
        std::fs::write(
            dir.path()
                .join(format!("{:016x}", fnv("Nada")))
                .with_extension("json"),
            "null",
        )
        .unwrap();
        assert_eq!(lookup(dir.path(), "Nada").unwrap(), None);
    }
}
