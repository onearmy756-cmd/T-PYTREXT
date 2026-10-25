//! tetea.rs — MITIHANI HALISI ya TETEA (maktaba.tetea.org) kwa ELIMU.
//!
//! Hupakua past papers za NECTA (CSEE, ACSEE, DSEE, FTNA/form_ii, GATCE, PSLE,
//! Darasa I/II/IV, QT + study-aids), huziorodhesha, hutoa maandishi
//! (`pdftotext`), hufanya OCR kwa zilizoskaniwa (`pdftoppm` + `tesseract`), na
//! huingiza maswali kwenye benki ya ELIMU (`edu_questions`) — kila swali
//! likihusishwa na karatasi yake HALISI kwa `source`.
//!
//! Endpoints (zote `/api/education/tetea/...`):
//!   GET  /status                  → idadi kwa exam + zana zilizopo
//!   POST /index/build             → changanua diski, jenga faharasa (DB)
//!   GET  /subjects                → masomo + idadi kwa exam
//!   GET  /papers                  → tafuta (exam, subject, year, kind, level, q)
//!   GET  /paper/:id               → metadata + maandishi (OCR ikihitajika)
//!   POST /paper/:id/extract       → gawanya maandishi → maswali → edu_questions
//!   POST /import                  → wingi: extract + insert kwa vichujio
//!   POST /fetch                   → pakua kutoka TETEA (nyuma, maendeleo)
//!   GET  /fetch/status            → maendeleo ya upakuaji
//!
//! Kanuni: hakuna kubuni. Karatasi isipopatikana au maandishi yasitoke, huripoti
//! kosa HALISI (hakuna maswali ya kutengeneza).

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Mutex, OnceLock};

// ============================================================
// TABLES
// ============================================================

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        "CREATE TABLE IF NOT EXISTS edu_tetea_papers (
            id TEXT PRIMARY KEY,
            rel TEXT NOT NULL UNIQUE,
            exam TEXT NOT NULL,
            subject TEXT NOT NULL,
            level TEXT NOT NULL,
            year INTEGER,
            paper TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'paper',
            bytes INTEGER NOT NULL DEFAULT 0,
            text_len INTEGER NOT NULL DEFAULT 0,
            imported INTEGER NOT NULL DEFAULT 0,
            attempts INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_edu_tetea_exam ON edu_tetea_papers(exam, subject, year)",
        "CREATE TABLE IF NOT EXISTS edu_tetea_imports (
            id TEXT PRIMARY KEY,
            rel TEXT NOT NULL,
            questions INTEGER NOT NULL DEFAULT 0,
            chars INTEGER NOT NULL DEFAULT 0,
            ocr INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
    ];
    for s in ddl {
        let _ = sqlx::query(s).execute(db).await;
    }
    // Bila safu hii, karatasi inayoshindwa (skana chakavu) huchaguliwa tena
    // kila mzunguko na uingizaji hukwama bila mwisho.
    let _ = sqlx::query(
        "ALTER TABLE edu_tetea_papers ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0",
    )
    .execute(db)
    .await;
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/education/tetea/status", get(status))
        .route("/api/education/tetea/index/build", post(index_build))
        .route("/api/education/tetea/subjects", get(subjects))
        .route("/api/education/tetea/papers", get(papers))
        .route("/api/education/tetea/paper/:id", get(paper_get))
        .route("/api/education/tetea/paper/:id/extract", post(paper_extract))
        .route("/api/education/tetea/import", post(bulk_import))
        .route("/api/education/tetea/fetch", post(fetch_start))
        .route("/api/education/tetea/fetch/status", get(fetch_status))
}

// ============================================================
// MSINGI
// ============================================================

/// Chanzo: maktaba.tetea.org (bure, NECTA).
const TETEA_BASE: &str = "https://maktaba.tetea.org";

/// Kurasa za TETEA zinazoorodhesha PDF (kila moja ina maelfu ya viungo).
const TETEA_PAGES: [&str; 8] = [
    "dsee-dtee",
    "form-1-2",
    "form-3-4",
    "form-5-6",
    "gatce",
    "qt",
    "standard-1-4",
    "standard-5-7",
];

/// PDF ndogo kuliko hii hutupwa (ukurasa wa kosa, si karatasi).
const MIN_PDF: i64 = 8000;
/// Maandishi machache kuliko haya = karatasi iliyoskaniwa (OCR inahitajika).
/// Kizingiti ni cha juu kwa makusudi: kurasa za boilerplate ya TETEA hutoa
/// herufi ~400, ambazo si karatasi.
const MIN_TEXT: usize = 900;

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Chini ya diski: `<FUNDI_DATA>/tetea`. PDF hukaa `past-papers/`, maandishi
/// yaliyotolewa huhifadhiwa `text/` (ili OCR isirudiwe kila mara).
fn root() -> PathBuf {
    crate::data_io::data_dir().join("tetea")
}

fn tool_ok(bin: &str, arg: &str) -> bool {
    std::process::Command::new(bin)
        .arg(arg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `pdftotext` — hutoa maandishi ya PDF yenye tabaka la maandishi.
pub fn pdftotext_ok() -> bool {
    tool_ok("pdftotext", "-v")
}
/// `pdftoppm` — hupanga kurasa kuwa picha (kwa OCR).
pub fn pdftoppm_ok() -> bool {
    tool_ok("pdftoppm", "-v")
}
/// `tesseract` — OCR kwa karatasi zilizoskaniwa.
pub fn tesseract_ok() -> bool {
    tool_ok("tesseract", "--version")
}

// ============================================================
// KUCHANGANUA MAJINA
// ============================================================

/// Ngazi ya elimu kutoka aina ya mtihani wa NECTA.
/// Hii ni ramani HALISI ya mfumo wa Tanzania (si kubuni).
fn level_for(exam: &str, bucket: &str, name: &str) -> &'static str {
    match exam {
        "csee" | "form_ii" | "dsee" => "o-level",
        "acsee" | "gatce" => "a-level",
        "psle" | "std1" | "std2" | "std4" => "primary",
        "qt" => "college",
        // study-aids: ngazi iko kwenye sehemu ya pili ya njia (o-level/a-level/primary)
        _ => match bucket {
            "a-level" => "a-level",
            "primary" => "primary",
            "practicals" | "syllabi" => "o-level",
            _ => {
                let l = name.to_lowercase();
                if l.contains("f5") || l.contains("f6") || l.contains("s5") || l.contains("s6") {
                    "a-level"
                } else {
                    "o-level"
                }
            }
        },
    }
}

/// Mwaka wa mtihani: nambari ya mwisho ya tarakimu 4 katika jina la faili.
fn find_year(name: &str) -> Option<i64> {
    let re = regex::Regex::new(r"(19|20)\d{2}").ok()?;
    re.find_iter(name)
        .last()
        .and_then(|m| m.as_str().parse::<i64>().ok())
}

/// Namba ya karatasi ("Paper 1", "Accountancy 2 - F6 - 2009" → "2").
fn find_paper(name: &str) -> String {
    if let Ok(re) = regex::Regex::new(r"(?i)\bpaper\s*([1-9])") {
        if let Some(c) = re.captures(name) {
            return c[1].to_string();
        }
    }
    if let Ok(re) = regex::Regex::new(r"\s([1-9])\s*-\s*(?:F[1-6]|S[1-7]|\d{4})") {
        if let Some(c) = re.captures(name) {
            return c[1].to_string();
        }
    }
    String::new()
}

fn scan_pdfs(dir: &FsPath, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            scan_pdfs(&p, out);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| x.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

#[derive(Clone)]
struct PaperMeta {
    rel: String,
    exam: String,
    subject: String,
    level: String,
    year: Option<i64>,
    paper: String,
    kind: String,
}

fn paper_id(rel: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(rel.as_bytes());
    let d = h.finalize();
    d.iter().take(12).map(|b| format!("{b:02x}")).collect()
}

fn parse_paper(root: &FsPath, full: &FsPath) -> Option<PaperMeta> {
    let rel = full
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let name = full.file_name()?.to_string_lossy().to_string();
    let comps: Vec<&str> = rel.split('/').collect();
    let first = comps.first().copied().unwrap_or("other");
    let second = comps.get(1).copied().unwrap_or("general");

    // study-aids/ haina exam/subject kwa maana ya NECTA — tunaiweka kama "study-aids".
    let (exam, subject, bucket) = if first == "study-aids" {
        ("study-aids".to_string(), second.to_string(), second)
    } else {
        (first.to_string(), second.to_string(), first)
    };

    let kind = if name.to_lowercase().contains("solutions") {
        "solutions"
    } else if first == "study-aids" {
        "book"
    } else {
        "paper"
    };
    let level = level_for(&exam, bucket, &name).to_string();
    Some(PaperMeta {
        rel,
        exam,
        subject,
        level,
        year: find_year(&name),
        paper: find_paper(&name),
        kind: kind.to_string(),
    })
}

// ============================================================
// FAHARASA (DB)
// ============================================================

/// Changanua diski na ujaze faharasa. Hufanya upsert kwa `rel`.
pub async fn index_build(State(s): State<AppState>) -> Json<Value> {
    let r = root();
    if !r.is_dir() {
        return Json(json!({
            "ok": false,
            "error": format!("hakuna PDF bado: {} haipo. Tumia POST /api/education/tetea/fetch", r.display())
        }));
    }
    let scanned = tokio::task::spawn_blocking(move || {
        let mut files = Vec::new();
        scan_pdfs(&r, &mut files);
        files.sort();
        files
    })
    .await
    .unwrap_or_default();

    let r = root();
    let mut added = 0i64;
    let mut updated = 0i64;
    for f in &scanned {
        let Some(m) = parse_paper(&r, f) else { continue };
        let bytes = std::fs::metadata(f).map(|x| x.len() as i64).unwrap_or(0);
        let id = paper_id(&m.rel);
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM edu_tetea_papers WHERE rel = ?1")
                .bind(&m.rel)
                .fetch_optional(&s.db)
                .await
                .unwrap_or(None);
        let res = sqlx::query(
            "INSERT INTO edu_tetea_papers
             (id, rel, exam, subject, level, year, paper, kind, bytes, text_len, imported, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,0,?10)
             ON CONFLICT(rel) DO UPDATE SET
                exam = excluded.exam, subject = excluded.subject, level = excluded.level,
                year = excluded.year, paper = excluded.paper, kind = excluded.kind,
                bytes = excluded.bytes",
        )
        .bind(&id)
        .bind(&m.rel)
        .bind(&m.exam)
        .bind(&m.subject)
        .bind(&m.level)
        .bind(m.year)
        .bind(&m.paper)
        .bind(&m.kind)
        .bind(bytes)
        .bind(now())
        .execute(&s.db)
        .await;
        match res {
            Ok(_) if exists.is_some() => updated += 1,
            Ok(_) => added += 1,
            Err(_) => {}
        }
    }
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM edu_tetea_papers")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    Json(json!({"ok": true, "scanned": scanned.len(), "added": added, "updated": updated, "total": total.0}))
}

// ============================================================
// KUTOA MAANDISHI (pdftotext + OCR)
// ============================================================

fn text_cache(rel: &str) -> PathBuf {
    root().join("text").join(rel).with_extension("txt")
}

/// Safisha boilerplate ya TETEA (haiongezi thamani kwenye benki ya maswali).
/// Mstari wa "Find this and other free resources at:" huja mara mbili kwenye
/// kila ukurasa (mara moja bila URL), hivyo lazima uondolewe kwa kigezo cha
/// "free educational resources"/"find this and other", si URL pekee.
fn clean_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for line in raw.lines() {
        let l = line.trim();
        if l.is_empty() {
            out.push('\n');
            continue;
        }
        let low = l.to_lowercase();
        if low.contains("maktaba.tetea.org")
            || low.contains("camscanner")
            || low.contains("free educational resources")
            || low.contains("find this and other")
            || low.contains("prepared by: maria marco")
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    // Punguza mistari mitupu inayofuatana.
    let mut res = String::with_capacity(out.len());
    let mut blank = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        res.push_str(line);
        res.push('\n');
    }
    res.trim().to_string()
}

/// Soma PGM (P5) na ubinarishe kwa kizingiti. Hurejesha (data, w, h).
///
/// Skana za CamScanner zina kijivu hafifu (background ~200/255) ambacho
/// huchanganya tesseract. Kubinarisha kwa kizingiti ~170 hutoa maandishi
/// mara 20+ zaidi (kupimwa kwa karatasi halisi za TETEA).
fn binarize_pgm(path: &FsPath, thresh: u8) -> Option<(Vec<u8>, usize, usize)> {
    let raw = std::fs::read(path).ok()?;
    let mut it = raw.iter().copied().enumerate();
    // Magic "P5"
    let mut hdr = Vec::new();
    while hdr.len() < 2 {
        hdr.push(it.next()?.1);
    }
    if &hdr[..] != b"P5" {
        return None;
    }
    // Tokeni tatu za kichwa: w, h, maxval (zikiruka maoni ya '#').
    let mut vals = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    while vals.len() < 3 {
        let (_, b) = it.next()?;
        if b == b'#' {
            while let Some((_, c)) = it.next() {
                if c == b'\n' {
                    break;
                }
            }
        } else if b.is_ascii_whitespace() {
            if !cur.is_empty() {
                vals.push(String::from_utf8_lossy(&cur).to_string());
                cur.clear();
            }
        } else {
            cur.push(b);
        }
    }
    let w: usize = vals[0].parse().ok()?;
    let h: usize = vals[1].parse().ok()?;
    let maxv: u32 = vals[2].parse().ok()?;
    let start = it.next()?.0;
    let px = &raw[start..];
    if px.len() < w * h || maxv == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(w * h);
    for i in 0..(w * h) {
        let v = (px[i] as u32 * 255 / maxv).min(255) as u8;
        out.push(if v <= thresh { 0 } else { 255 });
    }
    Some((out, w, h))
}

/// OCR ya karatasi: hupaka kurasa kwa kijivu, hubinarisha, kisha hutumia tesseract.
fn ocr_pdf(pdf: &FsPath, max_pages: u32) -> Option<String> {
    if !(pdftoppm_ok() && tesseract_ok()) {
        return None;
    }
    let tmp = std::env::temp_dir().join(format!("edu_ocr_{}", uid()));
    std::fs::create_dir_all(&tmp).ok()?;
    let prefix = tmp.join("p");
    let out = std::process::Command::new("pdftoppm")
        .arg("-gray")
        .arg("-r")
        .arg("150")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg(max_pages.to_string())
        .arg(pdf)
        .arg(&prefix)
        .output()
        .ok()?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return None;
    }
    let mut pages: Vec<PathBuf> = std::fs::read_dir(&tmp)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("pgm"))
        .collect();
    pages.sort();
    let mut text = String::new();
    for (i, p) in pages.iter().enumerate() {
        // Binarisha kwa kizingiti; picha ya kijivu hafifu haitoi maandishi.
        // Jina LAZIMA liwe la kipekee kwa kila ukurasa — jina la pamoja
        // huandika juu ya ukurasa uliopita.
        let target = match binarize_pgm(p, 170) {
            Some((data, w, h)) => {
                let bp = tmp.join(format!("b{:03}.pgm", i));
                let mut buf = format!("P5\n{} {}\n255\n", w, h).into_bytes();
                buf.extend_from_slice(&data);
                if std::fs::write(&bp, buf).is_ok() {
                    bp
                } else {
                    p.clone()
                }
            }
            None => p.clone(),
        };
        // OMP_THREAD_LIMIT=1 ni LAZIMA: tesseract huruka thread 4+ kwa ukurasa
        // na hukwama (dakika 4+ kwa ukurasa mmoja) kwenye kontena yenye CPU 4.
        // Kwa thread moja, ukurasa huo huo hukamilika kwa ~3s.
        let mut cmd = std::process::Command::new("tesseract");
        cmd.arg(&target)
            .arg("-")
            .arg("--psm")
            .arg("6")
            .env("OMP_THREAD_LIMIT", "1");
        if let Ok(o) = cmd.output() {
            text.push_str(&String::from_utf8_lossy(&o.stdout));
            text.push_str("\n\n");
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let t = text.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// Toa maandishi ya karatasi. Huhifadhi matokeo (text/) ili kuepuka kazi mara mbili.
fn extract_text(rel: &str, allow_ocr: bool, max_pages: u32) -> Result<(String, bool), String> {
    let cached = text_cache(rel);
    if let Ok(t) = std::fs::read_to_string(&cached) {
        // Kashe fupi = boilerplate pekee; usiirukue, jaribu tena (inaweza kuwa OCR).
        if t.trim().len() >= MIN_TEXT {
            return Ok((t, false));
        }
    }
    let pdf = root().join(rel);
    if !pdf.is_file() {
        return Err(format!("karatasi haipo: {}", pdf.display()));
    }
    if !pdftotext_ok() {
        return Err("pdftotext haipo — sakinisha poppler-utils".into());
    }
    let mut text = String::new();
    let mut used_ocr = false;
    if let Ok(o) = std::process::Command::new("pdftotext")
        .arg("-layout")
        .arg(&pdf)
        .arg("-")
        .output()
    {
        text = clean_text(&String::from_utf8_lossy(&o.stdout));
    }
    if text.len() < MIN_TEXT {
        // Karatasi iliyoskaniwa (CamScanner) — maandishi hayapo, OCR inahitajika.
        if !allow_ocr {
            return Err(format!(
                "karatasi imeskaniwa (maandishi {} herufi). Tumia `ocr=1` kwa OCR.",
                text.len()
            ));
        }
        match ocr_pdf(&pdf, max_pages) {
            Some(o) => {
                let oc = clean_text(&o);
                if oc.len() > text.len() {
                    text = oc;
                    used_ocr = true;
                }
            }
            None => {
                if !(tesseract_ok() && pdftoppm_ok()) {
                    return Err("karatasi imeskaniwa; OCR haipo — sakinisha tesseract-ocr na poppler-utils".into());
                }
            }
        }
    } else if allow_ocr && seq_quality(&text) < 0.75 {
        // Karatasi ya safu mbili: tabaka la maandishi huchanganya namba za maswali.
        // OCR ya kurasa hutoa mpangilio safi — jarbu, na tumia ikizidi ubora.
        if pdftoppm_ok() && tesseract_ok() {
            if let Some(o) = ocr_pdf(&pdf, max_pages) {
                let oc = clean_text(&o);
                if oc.len() >= MIN_TEXT && score(&oc) > score(&text) {
                    text = oc;
                    used_ocr = true;
                }
            }
        }
    }
    if text.trim().is_empty() {
        return Err("maandishi hayakutoka (karatasi tupu au picha)".into());
    }
    // Baada ya juhudi zote, maandishi mafupi = boilerplate pekee (si karatasi).
    if text.len() < MIN_TEXT {
        return Err(format!(
            "maandishi yenye maana hayakutoka (herufi {} pekee) — karatasi huenda ni picha tupu au ubora mdogo",
            text.len()
        ));
    }
    // Kashe tu maandishi yenye maana; boilerplate pekee isihifadhiwe.
    if let Some(d) = cached.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&cached, &text);
    Ok((text, used_ocr))
}

// ============================================================
// KUGAWANYA MASWALI
// ============================================================

/// Alama za maswali: namba mwanzoni mwa mstari. Hurejesha
/// `(start, end_of_number, number, inafuatiwa_na_(a))`.
/// Karatasi za NECTA hupanga kila swali kuanza kwa `N.` na kisha sehemu `(a)`;
/// kizuizi cha MAELEKEZO hufanana lakini hakina `(a)`.
fn question_marks(text: &str) -> Vec<(usize, usize, i64, bool)> {
    let re = match regex::Regex::new(r"(?m)^\s*(\d{1,2})[.)]\s*") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let subre = regex::Regex::new(r"^\s*\(?([a-h]|[ivx]+)\)").ok();
    let mut out = Vec::new();
    for c in re.captures_iter(text) {
        let m = c.get(0).unwrap();
        let n: i64 = c[1].parse().unwrap_or(0);
        if n == 0 || n > 40 {
            continue;
        }
        let after = &text[m.end()..];
        // Sehemu `(a)` iko mstari huo huo, au kwenye mstari unaofuata ikiwa
        // namba ilikuwa peke yake kwenye mstari wake.
        let (line_rest, next_line) = match after.find('\n') {
            Some(i) => (&after[..i], after[i + 1..].lines().next().unwrap_or("")),
            None => (after, ""),
        };
        let is_sub = subre
            .as_ref()
            .is_some_and(|r| r.is_match(line_rest) || r.is_match(next_line));
        // Namba peke yake, au ikifuatiwa na (a)/(i), au herufi (maandishi ya swali).
        let bare = line_rest.trim().is_empty();
        let word = line_rest.chars().next().map(|ch| ch.is_alphabetic()).unwrap_or(false);
        if bare || is_sub || word {
            out.push((m.start(), m.end(), n, is_sub));
        }
    }
    out
}

/// Ubora wa mpangilio: uwiano wa namba zinazofuatana (+1). Karatasi zilizo
/// sawa hutoa 1,2,3,4...; karatasi za safu mbili zilizochanganywa hutoa
/// mpangilio uliovunjika (1,5,6,8...) — hapo OCR inahitajika.
fn seq_quality(text: &str) -> f64 {
    let marks = question_marks(text);
    if marks.len() < 3 {
        return 0.0;
    }
    let seq = marks.windows(2).filter(|w| w[1].2 == w[0].2 + 1).count();
    seq as f64 / (marks.len() - 1) as f64
}

/// Alama ya ubora wa uchimbaji: huchanganya usawa wa mpangilio wa namba na
/// idadi ya alama. Karatasi ya safu mbili hutoa alama nyingi lakini mpangilio
/// uliovunjika; OCR hutoa mpangilio safi. Kigezo cha pamoja huepuka
/// kubadilisha maandishi mazuri kwa OCR yenye alama chache.
fn score(text: &str) -> f64 {
    let n = question_marks(text).len() as f64;
    seq_quality(text) * 10.0 + n.min(60.0) * 0.1
}

/// Maelekezo ya karatasi hufanana na maswali kwa namba, na mengine hufuata
/// mwanzo halisi (kwa mfano "Write your Examination Number..."). Kichujio hiki
/// hukagua mwanzo wa mwili pekee ili swali halisi lisiondolewe.
fn is_instruction_body(body: &str) -> bool {
    let head: String = body.chars().take(180).collect::<String>().to_lowercase();
    const MARKERS: [&str; 20] = [
        "write your examination number",
        "write your examination no",
        "this paper consists of",
        "answer all questions",
        "answer any ",
        "time allowed",
        "read the following instructions",
        "do not open",
        "write in ink",
        "instructions to candidates",
        "turn over",
        "all answers should be written",
        // Mistari ya maelekezo ya karatasi za NECTA inayofuata orodha ya kwanza.
        "each question carries",
        "all writing must be",
        "communication devices",
        "unauthorised materials",
        "unauthorized materials",
        "not allowed in the examination room",
        "except drawings",
        "mobile phones",
    ];
    MARKERS.iter().any(|m| head.contains(m))
}

/// Chagua mwanzo wa maswali HALISI kati ya mifuatano ya namba.
///
/// Kizuizi cha maelekezo na maswali yenyewe hufanana kwa namba (1,2,3...),
/// hivyo hutofautishwa kwa uwepo wa sehemu `(a)`: maswali halisi huanza kwa
/// `N. (a) ...`, maelekezo huwa `N. This paper consists...`.
fn best_question_start(marks: &[(usize, usize, i64, bool)]) -> usize {
    let mut best = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    for (i, (_, _, n, _)) in marks.iter().enumerate() {
        if *n != 1 {
            continue;
        }
        // Mfululizo unaopanda kwa 1 kuanzia hapa.
        let mut len = 1usize;
        let mut subs = if marks[i].3 { 1usize } else { 0 };
        let mut expect = 2i64;
        for m in marks.iter().skip(i + 1) {
            if m.2 == expect {
                len += 1;
                if m.3 {
                    subs += 1;
                }
                expect += 1;
            }
        }
        let sub_frac = subs as f64 / len as f64;
        let score = len as f64 * (1.0 + sub_frac);
        if score > best_score {
            best_score = score;
            best = i;
        }
    }
    best
}

/// Gawanya maandishi ya karatasi → maswali. Hurejesha (swali, alama).
fn split_questions(text: &str) -> Vec<(String, i64)> {
    let marks = question_marks(text);
    if marks.is_empty() {
        return Vec::new();
    }
    let best_start = best_question_start(&marks);
    // Kata maswali: kila swali huanza kwenye alama na huisha kwenye alama inayofuata.
    let slice = &marks[best_start..];
    let mut out: Vec<(String, i64)> = Vec::new();
    for (k, (_, num_end, n, _)) in slice.iter().enumerate() {
        let end = slice.get(k + 1).map(|(s, _, _, _)| *s).unwrap_or(text.len());
        let body = text[*num_end..end].trim();
        if body.len() >= 25 && !is_instruction_body(body) {
            let parts = regex::Regex::new(r"(?m)^\s*\(?[a-h]\)")
                .map(|r| r.find_iter(body).count())
                .unwrap_or(0);
            let mk = if parts > 1 { parts as i64 } else { 1 };
            out.push((format!("({n}) {body}"), mk));
        }
    }
    out
}

/// Huingiza maswali kwenye `edu_questions`, yakihusishwa na karatasi (`source`).
async fn insert_questions(db: &SqlitePool, m: &PaperMeta, qs: &[(String, i64)]) -> (i64, i64) {
    let subject = pretty_subject(&m.subject);
    let source = format!("TETEA:{}", m.rel);
    let mut added = 0i64;
    let mut skipped = 0i64;
    for (q, mk) in qs {
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM edu_questions WHERE source = ?1 AND question = ?2 LIMIT 1")
                .bind(&source)
                .bind(q)
                .fetch_optional(db)
                .await
                .unwrap_or(None);
        if exists.is_some() {
            skipped += 1;
            continue;
        }
        let res = sqlx::query(
            "INSERT INTO edu_questions
             (id, subject, level, topic, year, exam_type, question, options, answer, explanation, marks, source, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        )
        .bind(uid())
        .bind(&subject)
        .bind(&m.level)
        .bind(m.paper.clone())
        .bind(m.year)
        .bind(format!("tetea-{}", m.exam))
        .bind(q)
        .bind(json!([]).to_string())
        .bind("") // karatasi ya maswali haina majibu (Solutions huja tofauti)
        .bind("")
        .bind(mk)
        .bind(&source)
        .bind(now())
        .execute(db)
        .await;
        if res.is_ok() {
            added += 1;
        }
    }
    (added, skipped)
}

/// `basic_math` → "Basic Math"; `addl_math` → "Additional Math".
fn pretty_subject(s: &str) -> String {
    match s {
        "addl_math" => return "Additional Math".into(),
        "basic_math" => return "Basic Math".into(),
        "kiislamu" => return "Islamic Knowledge".into(),
        _ => {}
    }
    s.split(['_', '-'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================
// HANDLERS
// ============================================================

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM edu_tetea_papers")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let by_exam = sqlx::query_as::<_, (String, i64)>(
        "SELECT exam, COUNT(*) FROM edu_tetea_papers GROUP BY exam ORDER BY COUNT(*) DESC",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let by_kind = sqlx::query_as::<_, (String, i64)>(
        "SELECT kind, COUNT(*) FROM edu_tetea_papers GROUP BY kind ORDER BY COUNT(*) DESC",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let imported: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(imported),0) FROM edu_tetea_papers")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let questions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM edu_questions WHERE source LIKE 'TETEA:%'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let dir = root();
    Json(json!({
        "ok": true,
        "source": TETEA_BASE,
        "root": dir.display().to_string(),
        "indexed": total.0,
        "papers_imported": imported.0,
        "questions_from_tetea": questions.0,
        "by_exam": by_exam.into_iter().map(|(e, n)| json!({"exam": e, "count": n})).collect::<Vec<_>>(),
        "by_kind": by_kind.into_iter().map(|(k, n)| json!({"kind": k, "count": n})).collect::<Vec<_>>(),
        "tools": {
            "pdftotext": pdftotext_ok(),
            "pdftoppm": pdftoppm_ok(),
            "tesseract": tesseract_ok(),
        },
    }))
}

pub async fn subjects(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT exam, subject, level, COUNT(*) FROM edu_tetea_papers
         WHERE kind != 'solutions'
         GROUP BY exam, subject, level ORDER BY exam, subject",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(exam, subject, level, n)| {
            json!({"exam": exam, "subject": subject, "label": pretty_subject(&subject), "level": level, "count": n})
        })
        .collect();
    Json(json!({"ok": true, "subjects": list}))
}

#[derive(Deserialize)]
pub struct PaperQuery {
    pub exam: Option<String>,
    pub subject: Option<String>,
    pub year: Option<i64>,
    pub kind: Option<String>,
    pub level: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn papers(State(s): State<AppState>, Query(q): Query<PaperQuery>) -> Json<Value> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);
    let like = q.q.as_ref().map(|t| format!("%{}%", t.trim()));
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            Option<i64>,
            String,
            String,
            i64,
            i64,
        ),
    >(
        "SELECT id, rel, exam, subject, year, paper, kind, bytes, imported
         FROM edu_tetea_papers
         WHERE (?1 IS NULL OR exam = ?1)
           AND (?2 IS NULL OR subject = ?2)
           AND (?3 IS NULL OR year = ?3)
           AND (?4 IS NULL OR kind = ?4)
           AND (?5 IS NULL OR level = ?5)
           AND (?6 IS NULL OR rel LIKE ?6)
         ORDER BY exam, subject, year DESC, kind
         LIMIT ?7 OFFSET ?8",
    )
    .bind(q.exam.as_deref())
    .bind(q.subject.as_deref())
    .bind(q.year)
    .bind(q.kind.as_deref())
    .bind(q.level.as_deref())
    .bind(like.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM edu_tetea_papers
         WHERE (?1 IS NULL OR exam = ?1) AND (?2 IS NULL OR subject = ?2)
           AND (?3 IS NULL OR year = ?3) AND (?4 IS NULL OR kind = ?4)
           AND (?5 IS NULL OR level = ?5) AND (?6 IS NULL OR rel LIKE ?6)",
    )
    .bind(q.exam.as_deref())
    .bind(q.subject.as_deref())
    .bind(q.year)
    .bind(q.kind.as_deref())
    .bind(q.level.as_deref())
    .bind(like.as_deref())
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, rel, exam, subject, year, paper, kind, bytes, imported)| {
            json!({
                "id": id, "rel": rel, "exam": exam, "subject": subject,
                "label": pretty_subject(&subject), "year": year, "paper": paper,
                "kind": kind, "bytes": bytes, "imported": imported,
                "url": format!("/api/education/tetea/paper/{id}"),
            })
        })
        .collect();
    Json(json!({"ok": true, "total": total.0, "limit": limit, "offset": offset, "papers": list}))
}

#[derive(Deserialize)]
pub struct PaperGetQuery {
    pub ocr: Option<i64>,
    pub max_pages: Option<u32>,
    pub chars: Option<usize>,
}

pub async fn paper_get(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PaperGetQuery>,
) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, Option<i64>, String, String)>(
        "SELECT rel, exam, subject, level, year, paper, kind
         FROM edu_tetea_papers WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((rel, exam, subject, level, year, paper, kind)) = row else {
        return Json(json!({"ok": false, "error": "karatasi haipo kwenye faharasa. Tumia /tetea/index/build"}));
    };
    let ocr = q.ocr.unwrap_or(0) == 1;
    let max_pages = q.max_pages.unwrap_or(12).clamp(1, 60);
    let rel2 = rel.clone();
    let extracted = tokio::task::spawn_blocking(move || extract_text(&rel2, ocr, max_pages))
        .await
        .unwrap_or_else(|e| Err(e.to_string()));
    match extracted {
        Ok((text, used_ocr)) => {
            let cap = q.chars.unwrap_or(6000).min(200_000);
            let chars = text.len();
            let _ = sqlx::query("UPDATE edu_tetea_papers SET text_len = ?1 WHERE id = ?2")
                .bind(chars as i64)
                .bind(&id)
                .execute(&s.db)
                .await;
            let qs = split_questions(&text);
            Json(json!({
                "ok": true, "id": id, "rel": rel, "exam": exam, "subject": subject,
                "label": pretty_subject(&subject), "level": level, "year": year,
                "paper": paper, "kind": kind,
                "chars": chars, "ocr_used": used_ocr, "questions_detected": qs.len(),
                "text": text.chars().take(cap).collect::<String>(),
                "truncated": chars > cap,
            }))
        }
        Err(e) => Json(json!({"ok": false, "id": id, "rel": rel, "error": e})),
    }
}

#[derive(Deserialize)]
pub struct ExtractBody {
    pub ocr: Option<i64>,
    pub max_pages: Option<u32>,
    pub subject: Option<String>,
    pub level: Option<String>,
    pub limit: Option<i64>,
}

pub async fn paper_extract(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<ExtractBody>,
) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, Option<i64>, String, String)>(
        "SELECT rel, exam, subject, level, year, paper, kind FROM edu_tetea_papers WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((rel, exam, subject, level, year, paper, kind)) = row else {
        return Json(json!({"ok": false, "error": "karatasi haipo kwenye faharasa"}));
    };
    let m = PaperMeta {
        rel: rel.clone(),
        exam,
        subject: b.subject.clone().unwrap_or(subject),
        level: b.level.clone().unwrap_or(level),
        year,
        paper,
        kind,
    };
    let ocr = b.ocr.unwrap_or(1) == 1;
    let max_pages = b.max_pages.unwrap_or(12).clamp(1, 60);
    let rel2 = rel.clone();
    let text = match tokio::task::spawn_blocking(move || extract_text(&rel2, ocr, max_pages)).await {
        Ok(Ok((t, _))) => t,
        Ok(Err(e)) => return Json(json!({"ok": false, "id": id, "error": e})),
        Err(e) => return Json(json!({"ok": false, "id": id, "error": e.to_string()})),
    };
    let mut qs = split_questions(&text);
    if let Some(l) = b.limit {
        qs.truncate(l.clamp(1, 200) as usize);
    }
    if qs.is_empty() {
        return Json(json!({
            "ok": false, "id": id, "rel": rel,
            "error": format!("hakuna maswali yaliyogundulika (herufi {})", text.len()),
            "chars": text.len()
        }));
    }
    let (added, skipped) = insert_questions(&s.db, &m, &qs).await;
    let _ = sqlx::query("UPDATE edu_tetea_papers SET imported = imported + ?1, text_len = ?2 WHERE id = ?3")
        .bind(added)
        .bind(text.len() as i64)
        .bind(&id)
        .execute(&s.db)
        .await;
    let _ = sqlx::query(
        "INSERT INTO edu_tetea_imports (id, rel, questions, chars, ocr, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(uid())
    .bind(&rel)
    .bind(added)
    .bind(text.len() as i64)
    .bind(if ocr { 1i64 } else { 0i64 })
    .bind(now())
    .execute(&s.db)
    .await;
    let sample: Vec<Value> = qs
        .iter()
        .take(3)
        .map(|(q, mk)| json!({"question": q.chars().take(280).collect::<String>(), "marks": mk}))
        .collect();
    Json(json!({
        "ok": true, "id": id, "rel": rel, "subject": m.subject, "level": m.level,
        "year": m.year, "detected": qs.len(), "added": added, "skipped_duplicates": skipped,
        "chars": text.len(), "sample": sample,
    }))
}

#[derive(Deserialize)]
pub struct BulkBody {
    pub exam: Option<String>,
    pub subject: Option<String>,
    pub year: Option<i64>,
    pub kind: Option<String>,
    pub level: Option<String>,
    pub limit: Option<i64>,
    pub ocr: Option<i64>,
    pub max_pages: Option<u32>,
    /// Ruka karatasi hizi mwanzoni (kwa kuendesha sehemu sambamba).
    pub offset: Option<i64>,
    /// Ingiza upya hata karatasi zilizoingizwa tayari (default: ruka).
    pub force: Option<i64>,
}

/// Wingi: toa maandishi + ingiza maswali kwa karatasi nyingi.
pub async fn bulk_import(State(s): State<AppState>, Json(b): Json<BulkBody>) -> Json<Value> {
    let limit = b.limit.unwrap_or(10).clamp(1, 500);
    let offset = b.offset.unwrap_or(0).max(0);
    let ocr = b.ocr.unwrap_or(1) == 1;
    let max_pages = b.max_pages.unwrap_or(12).clamp(1, 60);
    // Kwa default tunachukua karatasi za maswali PEKEE (solutions hazina maswali mapya).
    let kind = b.kind.clone().unwrap_or_else(|| "paper".into());
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<i64>, String)>(
        "SELECT id, rel, exam, subject, level, year, paper FROM edu_tetea_papers
         WHERE (?1 IS NULL OR exam = ?1) AND (?2 IS NULL OR subject = ?2)
           AND (?3 IS NULL OR year = ?3) AND (?4 IS NULL OR kind = ?4)
           AND (?5 IS NULL OR level = ?5)
           AND (?8 = 1 OR imported = 0)
           AND (?8 = 1 OR attempts < 2)
         ORDER BY exam, subject, year DESC LIMIT ?6 OFFSET ?7",
    )
    .bind(b.exam.as_deref())
    .bind(b.subject.as_deref())
    .bind(b.year)
    .bind(kind.as_str())
    .bind(b.level.as_deref())
    .bind(limit)
    .bind(offset)
    .bind(b.force.unwrap_or(0))
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return Json(json!({"ok": false, "error": "hakuna karatasi zilizolingana. Tumia /tetea/index/build kwanza."}));
    }
    let mut results = Vec::new();
    let mut total_added = 0i64;
    let mut total_chars = 0usize;
    let mut total_skipped = 0i64;
    let mut ocr_papers = 0i64;
    let mut failures = 0i64;
    for (id, rel, exam, subject, level, year, paper) in rows {
        let m = PaperMeta {
            rel: rel.clone(),
            exam,
            subject,
            level,
            year,
            paper,
            kind: "paper".into(),
        };
        let rel2 = rel.clone();
        let id_attempt = id.clone();
        // Kila jaribio huhesabiwa — la sivyo karatasi inayoshindwa hurudiwa milele.
        let _ = sqlx::query("UPDATE edu_tetea_papers SET attempts = attempts + 1 WHERE id = ?1")
            .bind(&id_attempt)
            .execute(&s.db)
            .await;
        let (text, ocr_used) =
            match tokio::task::spawn_blocking(move || extract_text(&rel2, ocr, max_pages)).await {
                Ok(Ok((t, o))) => (t, o),
                Ok(Err(e)) => {
                    failures += 1;
                    results.push(json!({"id": id, "rel": rel, "ok": false, "error": e}));
                    continue;
                }
                Err(e) => {
                    failures += 1;
                    results.push(json!({"id": id, "rel": rel, "ok": false, "error": e.to_string()}));
                    continue;
                }
            };
        let qs = split_questions(&text);
        if qs.is_empty() {
            failures += 1;
            results.push(json!({"id": id, "rel": rel, "ok": false, "error": "hakuna maswali yaliyogundulika"}));
            continue;
        }
        let (added, skipped) = insert_questions(&s.db, &m, &qs).await;
        total_added += added;
        total_chars += text.len();
        if ocr_used {
            ocr_papers += 1;
        }
        total_skipped += skipped;
        let _ = sqlx::query("UPDATE edu_tetea_papers SET imported = imported + ?1, text_len = ?2 WHERE id = ?3")
            .bind(added)
            .bind(text.len() as i64)
            .bind(&id)
            .execute(&s.db)
            .await;
        results.push(json!({
            "id": id, "rel": rel, "ok": true, "detected": qs.len(),
            "added": added, "skipped": skipped, "ocr": ocr_used, "chars": text.len(),
        }));
    }
    Json(json!({
        "ok": true, "papers": results.len(), "questions_added": total_added,
        "skipped_duplicates": total_skipped, "ocr_papers": ocr_papers,
        "chars_processed": total_chars, "failures": failures, "results": results,
    }))
}

// ============================================================
// KUPakUA KUTOKA TETEA
// ============================================================

static FETCH: OnceLock<Mutex<Value>> = OnceLock::new();

fn fetch_state() -> &'static Mutex<Value> {
    FETCH.get_or_init(|| {
        Mutex::new(json!({"running": false, "done": 0, "total": 0, "ok": 0, "fail": 0}))
    })
}

pub async fn fetch_status() -> Json<Value> {
    let v = fetch_state()
        .lock()
        .map(|g| g.clone())
        .unwrap_or(json!({"running": false}));
    Json(json!({"ok": true, "state": v}))
}

#[derive(Deserialize)]
pub struct FetchBody {
    /// Pakua tu PDF zinazolingana na kichujio hiki (chaguo).
    pub exam: Option<String>,
    pub limit: Option<i64>,
    /// Changanua na ujaze faharasa baada ya kupakua.
    pub index: Option<bool>,
}

pub async fn fetch_start(Json(b): Json<FetchBody>) -> Json<Value> {
    {
        let st = fetch_state().lock().map(|g| g.clone()).unwrap_or(json!({}));
        if st["running"] == true {
            return Json(json!({"ok": false, "error": "upakuaji unaendelea tayari", "state": st}));
        }
    }
    {
        let mut st = fetch_state().lock().unwrap();
        *st = json!({"running": true, "phase": "orodha", "done": 0, "total": 0, "ok": 0, "fail": 0, "started_at": now()});
    }
    let exam = b.exam.clone();
    let limit = b.limit.unwrap_or(0).max(0) as usize;
    tokio::spawn(async move {
        let urls = match collect_urls().await {
            Ok(u) => u,
            Err(e) => {
                if let Ok(mut st) = fetch_state().lock() {
                    *st = json!({"running": false, "error": e});
                }
                return;
            }
        };
        let mut urls: Vec<String> = urls
            .into_iter()
            .filter(|u| match &exam {
                Some(e) if !e.is_empty() => u.contains(&format!("/past-papers/{e}/")),
                _ => true,
            })
            .collect();
        urls.sort();
        urls.dedup();
        if limit > 0 {
            urls.truncate(limit);
        }
        if let Ok(mut st) = fetch_state().lock() {
            st["phase"] = json!("kupakua");
            st["total"] = json!(urls.len());
        }
        download_all(urls).await;
        if let Ok(mut st) = fetch_state().lock() {
            st["running"] = json!(false);
            st["phase"] = json!("imekamilika");
            st["finished_at"] = json!(now());
        }
    });
    Json(json!({"ok": true, "message": "upakuaji umeanza; fuatilia GET /api/education/tetea/fetch/status"}))
}

/// Vuta kurasa za TETEA na ukusanye viungo vyote vya PDF.
async fn collect_urls() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("MTECHOS-ELIMU/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let re = regex::Regex::new(r#"href="([^"]+\.pdf)""#).map_err(|e| e.to_string())?;
    let mut out: Vec<String> = Vec::new();
    for p in TETEA_PAGES {
        let url = format!("{TETEA_BASE}/resources/{p}/");
        let html = match client.get(&url).send().await {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => return Err(format!("{url}: {e}")),
        };
        for c in re.captures_iter(&html) {
            let mut u = c[1].to_string();
            u = u.replace("http://maktaba.tetea.org", TETEA_BASE);
            u = u.replace("&amp;", "&");
            if u.starts_with("http") && u.to_lowercase().ends_with(".pdf") {
                out.push(u);
            }
        }
    }
    Ok(out)
}

fn dest_for(url: &str) -> PathBuf {
    let rel = url
        .trim_start_matches("https://maktaba.tetea.org/")
        .trim_start_matches("http://maktaba.tetea.org/")
        .split('?')
        .next()
        .unwrap_or("")
        .to_string();
    let rel = urlencoding_decode(&rel);
    let mut p = root();
    for part in rel.split('/') {
        let clean: String = part
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || " ._-()&,".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let c = clean.trim().trim_matches('.').to_string();
        p.push(if c.is_empty() { "_".to_string() } else { c });
    }
    p
}

/// Ufafanuzi wa `%XX` bila crate ya ziada.
fn urlencoding_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let h = (b[i + 1] as char).to_digit(16);
            let l = (b[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (h, l) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

async fn download_all(urls: Vec<String>) {
    use futures::StreamExt;
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("MTECHOS-ELIMU/1.0")
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let total = urls.len();
    let mut stream = futures::stream::iter(urls.into_iter().map(|u| {
        let c = client.clone();
        async move {
            let dst = dest_for(&u);
            if let Ok(m) = std::fs::metadata(&dst) {
                if dst.is_file() && m.len() as i64 >= MIN_PDF {
                    return "skip";
                }
            }
            if let Some(d) = dst.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let resp = match c.get(&u).send().await {
                Ok(r) => r,
                Err(_) => return "fail",
            };
            if !resp.status().is_success() {
                return "fail";
            }
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(_) => return "fail",
            };
            if !bytes.starts_with(b"%PDF") || (bytes.len() as i64) < MIN_PDF {
                return "fail";
            }
            let tmp = dst.with_extension("pdf.part");
            if std::fs::write(&tmp, &bytes).is_err() {
                return "fail";
            }
            if std::fs::rename(&tmp, &dst).is_err() {
                return "fail";
            }
            "ok"
        }
    }))
    .buffer_unordered(6);
    let mut done = 0usize;
    while let Some(r) = stream.next().await {
        done += 1;
        if let Ok(mut st) = fetch_state().lock() {
            st["done"] = json!(done);
            st["total"] = json!(total);
            let k = if r == "ok" {
                "ok"
            } else if r == "skip" {
                "skipped"
            } else {
                "fail"
            };
            let cur = st[k].as_i64().unwrap_or(0) + 1;
            st[k] = json!(cur);
        }
    }
}

// ============================================================
// VIPIMO
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_for_hufuata_mfumo_wa_tanzania() {
        assert_eq!(level_for("csee", "csee", "BasicMath-F4-2019.pdf"), "o-level");
        assert_eq!(level_for("form_ii", "form_ii", "Math-F2-2018.pdf"), "o-level");
        assert_eq!(
            level_for("acsee", "acsee", "Accountancy 1 - F6 - 2009.pdf"),
            "a-level"
        );
        assert_eq!(level_for("psle", "psle", "Kiarabu-S6-ZEC-2018.pdf"), "primary");
        assert_eq!(level_for("std4", "std4", "English-S4-2015.pdf"), "primary");
        assert_eq!(level_for("qt", "qt", "x.pdf"), "college");
        assert_eq!(
            level_for("study-aids", "a-level", "Syllabus - Physics - F5-F6 - 2017.pdf"),
            "a-level"
        );
        assert_eq!(
            level_for("study-aids", "primary", "Syllabus - Awali - 2016.pdf"),
            "primary"
        );
    }

    #[test]
    fn find_year_huchukua_mwaka_wa_mwisho() {
        assert_eq!(find_year("BasicMath-F4-2019.pdf"), Some(2019));
        assert_eq!(find_year("Additional Mathematics - F4 - 2015.pdf"), Some(2015));
        assert_eq!(find_year("Physics-PracticalGuidelines-2021.pdf"), Some(2021));
        assert_eq!(find_year("hakuna mwaka.pdf"), None);
    }

    #[test]
    fn find_paper_hutambua_namba_ya_karatasi() {
        assert_eq!(find_paper("Accountancy 2 - F6 - 2009.pdf"), "2");
        assert_eq!(find_paper("Agriculture 1 - F4 - 1993.pdf"), "1");
        assert_eq!(find_paper("Paper 3 - 2018.pdf"), "3");
        assert_eq!(find_paper("BasicMath-F4-2019.pdf"), "");
    }

    #[test]
    fn split_questions_huondoa_maelekezo_yaliyobaki() {
        // "Write your Examination Number..." hufuata mwanzo halisi lakini si swali.
        let text = "1. This paper consists of two questions.\n\
                    Instructions to candidates\n\
                    2. Write your Examination Number on every page of your answer booklet.\n\
                    3. Time allowed: 3 hours.\n\
                    1. (a) Explain the water cycle.\n(b) State two uses of water.\n";
        let qs = split_questions(text);
        assert_eq!(qs.len(), 1, "swali moja halisi: {qs:?}");
        assert!(qs[0].0.contains("water cycle"), "{:?}", qs[0].0);
        assert!(is_instruction_body(
            "Write your Examination Number on every page of your answer booklet(s)."
        ));
        assert!(!is_instruction_body("(a) Explain the water cycle."));
    }

    #[test]
    fn is_instruction_body_hukamata_maelekezo_ya_necta() {
        // Mistari hii hufuata maelekezo ya kwanza kwenye karatasi za NECTA.
        assert!(is_instruction_body(
            "(3) Each question carries twenty five (25) marks."
        ));
        assert!(is_instruction_body(
            "(4) All writing must be in blue or black ink, except drawings which must be in pencil."
        ));
        assert!(is_instruction_body(
            "(5) Communication devices and any unauthorised materials are not allowed in the examination room."
        ));
        assert!(is_instruction_body(
            "(6) Mobile phones are not allowed in the examination room."
        ));
        // Maswali halisi hayakamatwi.
        assert!(!is_instruction_body(
            "(a) Describe the process of photosynthesis in green plants."
        ));
        assert!(!is_instruction_body(
            "(b) State four factors that affect the rate of transpiration."
        ));
    }

    #[test]
    fn binarize_pgm_hubadilisha_kijivu_hafifu() {
        // Picha ya 2x2: 100 (wino), 200 (kijivu hafifu), 210, 255 (background).
        let mut buf = b"P5\n2 2\n255\n".to_vec();
        buf.extend_from_slice(&[100u8, 200, 210, 255]);
        let p = std::env::temp_dir().join(format!("tetea_pgm_test_{}.pgm", uid()));
        std::fs::write(&p, &buf).unwrap();
        let (data, w, h) = binarize_pgm(&p, 170).expect("PGM isomwe");
        assert_eq!((w, h), (2, 2));
        assert_eq!(data, vec![0u8, 255, 255, 255], "kizingiti 170 kikitumika");
        // Kizingiti cha chini hukamata kijivu hafifu pia.
        let (d2, _, _) = binarize_pgm(&p, 220).unwrap();
        assert_eq!(d2, vec![0u8, 0, 0, 255]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn clean_text_huondoa_boilerplate_ya_tetea() {
        let raw = "Find this and other free resources at: http://maktaba.tetea.org\n\
                   THE UNITED REPUBLIC OF TANZANIA\n\
                   Scanned by CamScanner\n\
                   Find this and other free resources at:\n\
                   Prepared by: Maria Marco for TETEA\n\
                   Swali halisi hapa\n\n\n\n\nSwali la pili";
        let t = clean_text(raw);
        assert!(!t.contains("maktaba.tetea.org"));
        assert!(!t.contains("CamScanner"));
        assert!(!t.contains("Find this and other"), "mstari wa boilerplate bila URL uondolewe");
        assert!(!t.contains("Maria Marco"));
        assert!(t.contains("THE UNITED REPUBLIC OF TANZANIA"));
        assert!(t.contains("Swali halisi hapa"));
        assert!(t.contains("Swali la pili"));
        assert!(!t.contains("\n\n\n"), "mistari mitupu ipunguzwe");
    }

    #[test]
    fn split_questions_hugawanya_karatasi_halisi() {
        let text = "INSTRUCTIONS\n1. This paper consists of 2 questions.\n2. Answer all questions.\n\
                    1. The accounting cycle starts with source documents.\n\
                    (a) Identify five stages.\n(b) List five source documents.\n\
                    2. Wabongo Ltd maintains a combined rent account.\n\
                    (a) Prepare the account.\n";
        let qs = split_questions(text);
        assert_eq!(qs.len(), 2, "maswali mawili yanatarajiwa: {qs:?}");
        assert!(qs[0].0.contains("accounting cycle"), "{:?}", qs[0].0);
        assert!(qs[0].0.starts_with("(1) "), "namba ihifadhiwe: {}", qs[0].0);
        assert_eq!(qs[0].1, 2, "vijisehemu (a)(b) = alama 2");
        assert!(qs[1].0.contains("Wabongo"));
    }

    #[test]
    fn split_questions_huruka_maelekezo_na_kuchukua_maswali_halisi() {
        // Maelekezo yanahesabiwa 1..3; maswali halisi yanaanza tena kwa 1.
        let text = "Instructions\n1. This paper consists of three questions.\n\
                    2. Answer all questions.\n3. Write your number.\n\
                    SECTION A\n\
                    1. (a) Find the value of x in the equation.\n(b) Find the value of y.\n\
                    2. (a) Solve the given quadratic equation.\n\
                    3. (a) Draw the graph of the function.\n";
        let qs = split_questions(text);
        assert_eq!(qs.len(), 3, "maswali 3 halisi: {qs:?}");
        assert!(qs[0].0.contains("Find the value of x"), "{:?}", qs[0].0);
        assert!(qs[2].0.contains("Draw the graph"), "{:?}", qs[2].0);
        assert!(
            !qs.iter().any(|(q, _)| q.contains("consists of three")),
            "maelekezo yasichukuliwe: {qs:?}"
        );
    }

    #[test]
    fn seq_quality_hupima_mpangilio() {
        let nzuri = "1. (a) moja\n2. (a) mbili\n3. (a) tatu\n4. (a) nne\n";
        let mbovu = "1. (a) moja\n5. (a) tano\n6. (a) sita\n8. (a) nane\n";
        assert!(seq_quality(nzuri) > 0.9, "{}", seq_quality(nzuri));
        assert!(seq_quality(mbovu) < 0.5, "{}", seq_quality(mbovu));
        assert_eq!(seq_quality("hakuna namba"), 0.0);
    }

    #[test]
    fn split_questions_hupuuza_maandishi_bila_maswali() {
        assert!(split_questions("Hakuna maswali hapa kabisa.").is_empty());
    }

    #[test]
    fn paper_id_ni_imara_na_tofauti() {
        assert_eq!(
            paper_id("csee/basic_math/x.pdf"),
            paper_id("csee/basic_math/x.pdf")
        );
        assert_ne!(
            paper_id("csee/basic_math/x.pdf"),
            paper_id("csee/basic_math/y.pdf")
        );
        assert_eq!(paper_id("a.pdf").len(), 24);
    }

    #[test]
    fn pretty_subject_hupanga_majina() {
        assert_eq!(pretty_subject("basic_math"), "Basic Math");
        assert_eq!(pretty_subject("addl_math"), "Additional Math");
        assert_eq!(pretty_subject("computer"), "Computer");
        assert_eq!(pretty_subject("kiislamu"), "Islamic Knowledge");
    }

    #[test]
    fn dest_for_hupanga_njia_salama() {
        let p = dest_for(
            "https://maktaba.tetea.org/past-papers/csee/basic_math/BasicMath%20-%20F4%20-%202019.pdf",
        );
        let s = p.to_string_lossy();
        assert!(
            s.ends_with("past-papers/csee/basic_math/BasicMath - F4 - 2019.pdf"),
            "{s}"
        );
        assert!(!s.contains(".."));
    }

    #[test]
    fn urlencoding_decode_hufafanua() {
        assert_eq!(
            urlencoding_decode("BasicMath%20-%20F4.pdf"),
            "BasicMath - F4.pdf"
        );
        assert_eq!(urlencoding_decode("hakuna%25kitu"), "hakuna%kitu");
    }

    #[test]
    fn parse_paper_hupata_exam_subject_na_ngazi() {
        let root = std::path::Path::new("/r");
        let m = parse_paper(
            root,
            std::path::Path::new("/r/csee/basic_math/BasicMath-F4-2019.pdf"),
        )
        .unwrap();
        assert_eq!(m.exam, "csee");
        assert_eq!(m.subject, "basic_math");
        assert_eq!(m.level, "o-level");
        assert_eq!(m.year, Some(2019));
        assert_eq!(m.kind, "paper");
        assert_eq!(m.rel, "csee/basic_math/BasicMath-F4-2019.pdf");

        let s = parse_paper(
            root,
            std::path::Path::new("/r/acsee/accountancy/Accountancy 2 - F6 - 2009-Solutions.pdf"),
        )
        .unwrap();
        assert_eq!(s.kind, "solutions");
        assert_eq!(s.paper, "2");
        assert_eq!(s.level, "a-level");
    }
}
