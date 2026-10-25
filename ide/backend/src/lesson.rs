//! lesson.rs — AI MWALIMU: mwalimu ana-upload notes (PDF/Word/picha/maandishi),
//! AI inatengeneza somo kamili (maelezo + mifano + uchambuzi + maswali) na
//! sauti (TTS) ya kila sehemu.
//!
//! Kila kitu hapa ni HALISI:
//!   - kutoa maandishi: `pdftotext` (PDF), `tesseract` (picha/OCR), ZIP+deflate
//!     (DOCX), au maandishi mbichi (TXT/MD);
//!   - somo: LLM iliyochaguliwa kwenye DB (`ai_config::load_with_db`);
//!   - sauti: `espeak-ng` (ina Kiswahili) → WAV, kisha `ffmpeg` → MP3 kama ipo.
//!
//! Zana isiyopo HAIBUNIWI: hurudisha `available: false` pamoja na jinsi ya
//! kuisakinisha (KANUNI ya grounding ya mradi).
//!
//! Endpoints (zote `/api/education/lesson*`):
//!   POST /upload        → faili (bytes) → maandishi → somo + sauti
//!   GET  /lessons       → masomo yaliyohifadhiwa
//!   GET  /:id           → somo moja
//!   POST /:id/audio     → zalisha sauti ya sehemu zote
//!   POST /:id/ask       → mwanafunzi anauliza swali kuhusu somo

use crate::AppState;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::{Path as FsPath, PathBuf};
use std::process::{Command, Stdio};

// ============================================================
// TABLES
// ============================================================

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS edu_lessons (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT '',
            language TEXT NOT NULL DEFAULT 'Kiswahili',
            level TEXT NOT NULL DEFAULT 'sekondari',
            minutes INTEGER NOT NULL DEFAULT 10,
            source_type TEXT NOT NULL DEFAULT 'pdf',
            source_name TEXT NOT NULL DEFAULT '',
            chars INTEGER NOT NULL DEFAULT 0,
            transcript TEXT NOT NULL DEFAULT '',
            lesson TEXT NOT NULL DEFAULT '{}',
            audio TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

// ============================================================
// ROUTER
// ============================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/education/lesson/upload", post(lesson_upload))
        .route("/api/education/lesson/lessons", get(lessons_list))
        .route("/api/education/lesson/:id", get(lesson_get))
        .route("/api/education/lesson/:id/audio", post(lesson_audio))
        .route("/api/education/lesson/:id/ask", post(lesson_ask))
        .route("/api/education/lesson/media/:name", get(lesson_media_serve))
        // Notes huja kama bytes mbichi (kama `scribe_transcribe`) — hakuna
        // multipart crate. PDF ya shule hupita 2MB ya axum default.
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
}

// ============================================================
// HELPERS
// ============================================================

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn lessons_dir() -> PathBuf {
    let d = crate::data_io::data_dir().join("lessons");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Je, binary ipo kwenye PATH? (uthibitisho halisi, si dhana)
fn tool_ok(bin: &str, arg: &str) -> bool {
    Command::new(bin)
        .arg(arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn pdftotext_ok() -> bool {
    tool_ok("pdftotext", "-v")
}
pub fn pdftoppm_ok() -> bool {
    tool_ok("pdftoppm", "-v")
}
pub fn tesseract_ok() -> bool {
    tool_ok("tesseract", "--version")
}
pub fn espeak_ok() -> bool {
    tool_ok("espeak-ng", "--version") || tool_ok("espeak", "--version")
}
pub fn ffmpeg_ok() -> bool {
    crate::nyi_media::ffmpeg_available()
}

/// Hali ya zana — kwa UI na kwa LLM (bila kubuni).
pub fn tools_status() -> Value {
    json!({
        "pdftotext": pdftotext_ok(),
        "pdftoppm": pdftoppm_ok(),
        "tesseract": tesseract_ok(),
        "espeak": espeak_ok(),
        "ffmpeg": ffmpeg_ok(),
        "install": {
            "pdf": "apt-get install -y poppler-utils",
            "ocr": "apt-get install -y tesseract-ocr tesseract-ocr-swa",
            "tts": "apt-get install -y espeak-ng",
            "audio": "apt-get install -y ffmpeg"
        }
    })
}

// ============================================================
// KUTOA MAANDISHI KUTOKA FAILI
// ============================================================

/// Aina ya faili kutoka jina lake (au `application/octet-stream`).
pub fn kind_of(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "pdf",
        "docx" => "docx",
        "txt" | "md" | "csv" | "text" => "text",
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" => "image",
        _ => "unknown",
    }
}

/// Tafuta sehemu ya ZIP inayoitwa `want` na uitoe (bila crate ya `zip`).
/// Hushughulikia method 0 (stored) na 8 (deflate) — ndiyo inayotumika kwenye DOCX.
fn unzip_entry(data: &[u8], want: &str) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let mut i = 0usize;
    while i + 30 <= data.len() {
        if &data[i..i + 4] != b"PK\x03\x04" {
            break;
        }
        let method = u16::from_le_bytes([data[i + 8], data[i + 9]]);
        let comp_size = u32::from_le_bytes([
            data[i + 18],
            data[i + 19],
            data[i + 20],
            data[i + 21],
        ]) as usize;
        let name_len = u16::from_le_bytes([data[i + 26], data[i + 27]]) as usize;
        let extra_len = u16::from_le_bytes([data[i + 28], data[i + 29]]) as usize;
        let name_start = i + 30;
        if name_start + name_len > data.len() {
            break;
        }
        let name = String::from_utf8_lossy(&data[name_start..name_start + name_len]).to_string();
        let data_start = name_start + name_len + extra_len;
        // comp_size == 0 hutumia data descriptor (streaming) — hatuipitishi.
        if comp_size == 0 || data_start + comp_size > data.len() {
            break;
        }
        if name == want {
            let comp = &data[data_start..data_start + comp_size];
            return match method {
                0 => Some(comp.to_vec()),
                8 => {
                    let mut out = Vec::new();
                    let mut d = flate2::read::DeflateDecoder::new(comp);
                    d.read_to_end(&mut out).ok()?;
                    Some(out)
                }
                _ => None,
            };
        }
        i = data_start + comp_size;
    }
    None
}

/// Toa maandishi kutoka `word/document.xml`: `<w:t>` ni maudhui, `</w:p>` ni
/// mwisho wa aya. Huzuia XML nyingine yote (styling, metadata).
fn docx_xml_to_text(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(p) = rest.find("</w:p>") {
        out.push_str(&extract_wt(&rest[..p]));
        out.push('\n');
        rest = &rest[p + 6..];
    }
    out.push_str(&extract_wt(rest));
    out.trim().to_string()
}

/// Toa maandishi ya `<w:t>` pekee kutoka kipande cha XML.
fn extract_wt(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(s) = rest.find("<w:t") {
        let after = &rest[s..];
        let Some(gt) = after.find('>') else { break };
        let tail = &rest[s + gt + 1..];
        let Some(e) = tail.find("</w:t>") else { break };
        out.push_str(&unescape_xml(&tail[..e]));
        rest = &tail[e + 6..];
    }
    out
}

/// Entities za kawaida za XML.
fn unescape_xml(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// OCR ya picha moja kwa tesseract.
fn ocr_image(path: &FsPath, lang: &str) -> Option<String> {
    if !tesseract_ok() {
        return None;
    }
    let mut cmd = Command::new("tesseract");
    cmd.arg(path)
        .arg("-")
        .arg("-l")
        .arg(lang)
        .arg("--psm")
        .arg("6")
        // OMP_THREAD_LIMIT=1: tesseract huruka thread 4+ na hukwama kwenye
        // kontena yenye CPU chache (iliyopimwa kwenye tetea.rs).
        .env("OMP_THREAD_LIMIT", "1");
    let o = cmd.output().ok()?;
    let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// OCR ya PDF: kurasa → PGM kijivu → tesseract.
fn ocr_pdf(pdf: &FsPath, lang: &str, max_pages: u32) -> Option<String> {
    if !(pdftoppm_ok() && tesseract_ok()) {
        return None;
    }
    let tmp = std::env::temp_dir().join(format!("edu_lesson_ocr_{}", uid()));
    std::fs::create_dir_all(&tmp).ok()?;
    let prefix = tmp.join("p");
    let out = Command::new("pdftoppm")
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
    for p in &pages {
        if let Some(t) = ocr_image(p, lang) {
            text.push_str(&t);
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

/// Lugha ya OCR/TTS kutoka lugha ya somo.
fn lang_code(language: &str) -> &'static str {
    let l = language.to_lowercase();
    if l.starts_with("ki") || l.contains("swahili") || l == "sw" {
        "sw"
    } else {
        "en"
    }
}

/// Toa maandishi ya faili. Hurudisha (maandishi, aina).
pub fn extract_text(name: &str, bytes: &[u8], language: &str, max_pages: u32) -> Result<(String, String), String> {
    let kind = kind_of(name);
    let ocr_lang = lang_code(language);
    match kind {
        "text" => {
            let t = String::from_utf8_lossy(bytes).trim().to_string();
            if t.is_empty() {
                Err("faili la maandishi ni tupu".into())
            } else {
                Ok((t, "text".into()))
            }
        }
        "pdf" => {
            let tmp = std::env::temp_dir().join(format!("edu_lesson_{}.pdf", uid()));
            std::fs::write(&tmp, bytes).map_err(|e| format!("haikuweza kuandika PDF: {e}"))?;
            let mut text = String::new();
            if pdftotext_ok() {
                if let Ok(o) = Command::new("pdftotext")
                    .arg("-layout")
                    .arg("-f")
                    .arg("1")
                    .arg("-l")
                    .arg(max_pages.to_string())
                    .arg(&tmp)
                    .arg("-")
                    .output()
                {
                    text = String::from_utf8_lossy(&o.stdout).trim().to_string();
                }
            }
            // PDF iliyoskaniwa haina tabaka la maandishi → OCR.
            if text.chars().filter(|c| !c.is_whitespace()).count() < 40 {
                if let Some(o) = ocr_pdf(&tmp, ocr_lang, max_pages) {
                    text = o;
                }
            }
            let _ = std::fs::remove_file(&tmp);
            if text.trim().is_empty() {
                Err("hakuna maandishi yaliyopatikana kwenye PDF. Kama ni skani, sakinisha poppler-utils + tesseract-ocr.".into())
            } else {
                Ok((text, "pdf".into()))
            }
        }
        "docx" => {
            let xml = unzip_entry(bytes, "word/document.xml")
                .ok_or("haikuweza kusoma DOCX (word/document.xml haipo)")?;
            let text = docx_xml_to_text(&String::from_utf8_lossy(&xml));
            if text.trim().is_empty() {
                Err("DOCX haina maandishi".into())
            } else {
                Ok((text, "docx".into()))
            }
        }
        "image" => {
            if !tesseract_ok() {
                return Err("OCR haipo — sakinisha: apt-get install -y tesseract-ocr tesseract-ocr-swa".into());
            }
            let ext = name.rsplit('.').next().unwrap_or("png");
            let tmp = std::env::temp_dir().join(format!("edu_lesson_{}.{}", uid(), ext));
            std::fs::write(&tmp, bytes).map_err(|e| format!("haikuweza kuandika picha: {e}"))?;
            let t = ocr_image(&tmp, ocr_lang);
            let _ = std::fs::remove_file(&tmp);
            match t {
                Some(t) => Ok((t, "image".into())),
                None => Err("OCR haikupata maandishi kwenye picha".into()),
            }
        }
        _ => Err(format!(
            "aina ya faili '{name}' haitumiki. Tumia PDF, DOCX, TXT, au picha (PNG/JPG)."
        )),
    }
}

// ============================================================
// SAUTI (TTS)
// ============================================================

/// Zalisha sauti ya maandishi → faili. Hurudisha jina la faili + urefu wa bytes.
fn synthesize(text: &str, language: &str, voice: Option<&str>, speed: i64) -> Result<(String, u64), String> {
    if !espeak_ok() {
        return Err("TTS haipo — sakinisha: apt-get install -y espeak-ng".into());
    }
    let bin = if tool_ok("espeak-ng", "--version") { "espeak-ng" } else { "espeak" };
    let lang = voice.map(|v| v.to_string()).unwrap_or_else(|| lang_code(language).to_string());
    let stamp = chrono::Utc::now().timestamp_millis();
    let wav = lessons_dir().join(format!("tts_{stamp}_{}.wav", &uid()[..8]));
    let status = Command::new(bin)
        .arg("-v")
        .arg(&lang)
        .arg("-s")
        .arg(speed.clamp(80, 300).to_string())
        .arg("-w")
        .arg(&wav)
        .arg(text)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() && wav.is_file() => {}
        _ => return Err("espeak-ng ilishindwa kutengeneza sauti".into()),
    }
    // MP3 ni nyepesi kwa browser — kama ffmpeg ipo.
    if ffmpeg_ok() {
        let mp3 = wav.with_extension("mp3");
        let ok = Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-i"])
            .arg(&wav)
            .args(["-codec:a", "libmp3lame", "-qscale:a", "6"])
            .arg(&mp3)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok && mp3.is_file() {
            let _ = std::fs::remove_file(&wav);
            let name = mp3.file_name().unwrap().to_string_lossy().to_string();
            let bytes = std::fs::metadata(&mp3).map(|m| m.len()).unwrap_or(0);
            return Ok((name, bytes));
        }
    }
    let name = wav.file_name().unwrap().to_string_lossy().to_string();
    let bytes = std::fs::metadata(&wav).map(|m| m.len()).unwrap_or(0);
    Ok((name, bytes))
}

/// Maandishi yanayosomeka kwa sauti kutoka sehemu ya somo.
fn spoken_text(section: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(k) = section["kichwa"].as_str() {
        parts.push(k.trim_end_matches('.').to_string());
    }
    for key in ["maelezo", "mfano", "uchambuzi"] {
        if let Some(v) = section[key].as_str() {
            if !v.trim().is_empty() {
                parts.push(v.trim().to_string());
            }
        }
    }
    parts.join(". ")
}

// ============================================================
// HANDLERS
// ============================================================

#[derive(Deserialize)]
pub struct UploadQuery {
    pub name: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub language: Option<String>,
    pub level: Option<String>,
    pub minutes: Option<i64>,
    pub voice: Option<String>,
    pub speed: Option<i64>,
    pub max_pages: Option<i64>,
    #[serde(default)]
    pub audio: Option<i64>,
}

pub async fn lesson_upload(
    State(s): State<AppState>,
    Query(q): Query<UploadQuery>,
    body: Bytes,
) -> Json<Value> {
    if body.is_empty() {
        return Json(json!({"ok": false, "error": "hakuna faili iliyotumwa"}));
    }
    let name = q
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "notes.pdf".into());
    let language = q.language.clone().unwrap_or_else(|| "Kiswahili".into());
    let level = q.level.clone().unwrap_or_else(|| "sekondari".into());
    let minutes = q.minutes.unwrap_or(10).clamp(3, 60);
    let max_pages = q.max_pages.unwrap_or(20).clamp(1, 80);

    // 1) KUTOA MAANDISHI
    let bytes = body.to_vec();
    let name2 = name.clone();
    let lang2 = language.clone();
    let extracted = tokio::task::spawn_blocking(move || {
        extract_text(&name2, &bytes, &lang2, max_pages as u32)
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()));

    let (text, source_type) = match extracted {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({"ok": false, "error": e, "tools": tools_status()}));
        }
    };

    // 2) KUTENGENEZA SOMO (LLM)
    let Some(lesson) = generate_lesson(&s.db, &text, &language, &level, minutes, q.subject.as_deref()).await
    else {
        return Json(json!({
            "ok": false,
            "error": "LLM haikujibu. Angalia model iliyochaguliwa kwenye /api/ai/test.",
            "chars": text.chars().count()
        }));
    };

    let title = q
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .or_else(|| lesson["mada"].as_str().map(|m| m.to_string()))
        .unwrap_or_else(|| format!("Somo — {}", name));

    let id = uid();

    // 3) SAUTI (TTS) — huzalishwa kwa kila sehemu kama mtumiaji ameomba.
    let mut audio = json!({});
    if q.audio.unwrap_or(1) == 1 {
        audio = generate_audio(&lesson, &language, q.voice.as_deref(), q.speed.unwrap_or(150));
    }

    let chars = text.chars().count() as i64;
    let res = sqlx::query(
        "INSERT INTO edu_lessons
         (id, title, language, level, minutes, source_type, source_name, chars, transcript, lesson, audio, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
    )
    .bind(&id)
    .bind(&title)
    .bind(&language)
    .bind(&level)
    .bind(minutes)
    .bind(&source_type)
    .bind(&name)
    .bind(chars)
    .bind(text.chars().take(20000).collect::<String>())
    .bind(lesson.to_string())
    .bind(audio.to_string())
    .bind(now())
    .execute(&s.db)
    .await;

    match res {
        Ok(_) => Json(json!({
            "ok": true, "id": id, "title": title,
            "language": language, "level": level, "minutes": minutes,
            "source_type": source_type, "source_name": name, "chars": chars,
            "lesson": lesson, "audio": audio
        })),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// Piga LLM itengeneze somo kamili. Hurudisha `None` kama LLM haikujibu.
async fn generate_lesson(
    db: &SqlitePool,
    text: &str,
    language: &str,
    level: &str,
    minutes: i64,
    subject: Option<&str>,
) -> Option<Value> {
    let sections = (minutes / 2).clamp(3, 8);
    let subject = subject.unwrap_or("");
    let prompt = format!(
        "Wewe ni mwalimu mahiri wa Tanzania. Tengeneza somo kamili la dakika {minutes} \
         kwa lugha ya {language}, kiwango cha {level}, kulingana na NOTES hizi.\n\
         Somo liwe na sehemu {sections}. Kila sehemu LAZIMA iwe na maelezo (maelezo ya \
         kina), mfano (mfano wa kueleweka wa maisha halisi), na uchambuzi (kwa nini ni \
         muhimu na linahusiana vipi na maisha).\n\
         Somo: {subject}\n\nNOTES:\n{text}\n\n\
         Rudisha JSON PEKEE (bila maelezo mengine):\n\
         {{\"mada\":\"kichwa cha somo\",\
         \"utangulizi\":\"utangulizi wa kuvutia\",\
         \"sehemu\":[{{\"kichwa\":\"1. jina la sehemu\",\"maelezo\":\"maelezo ya kina\",\
         \"mfano\":\"mfano halisi\",\"uchambuzi\":\"kwa nini ni muhimu\"}}],\
         \"hitimisho\":\"muhtasari wa mwisho\",\
         \"maswali_ya_mazoezi\":[\"swali 1\",\"swali 2\"],\
         \"msamiati\":[{{\"neno\":\"neno\",\"maana\":\"maana yake\"}}]}}",
        minutes = minutes,
        language = language,
        level = level,
        sections = sections,
        subject = subject,
        text = text.chars().take(14000).collect::<String>()
    );
    let raw = crate::education::llm_long(db, &prompt, 600).await?;
    let v = crate::education::extract_json(&raw)?;
    // Somo lisilo na sehemu haliwezi kutumika.
    if v["sehemu"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
        return Some(json!({ "raw": raw }));
    }
    Some(v)
}

/// Zalisha sauti kwa kila sehemu ya somo. Hurudisha ramani ya faharasa → sauti.
fn generate_audio(lesson: &Value, language: &str, voice: Option<&str>, speed: i64) -> Value {
    let mut out = json!({});
    if !espeak_ok() {
        return json!({
            "available": false,
            "error": "TTS haipo — sakinisha: apt-get install -y espeak-ng",
            "sections": {}
        });
    }
    let mut sections = serde_json::Map::new();
    let mut intro = Value::Null;
    let mut concl = Value::Null;

    if let Some(t) = lesson["utangulizi"].as_str().filter(|t| !t.trim().is_empty()) {
        if let Ok((f, b)) = synthesize(t, language, voice, speed) {
            intro = json!({"file": f, "url": format!("/api/education/lesson/media/{f}"), "bytes": b});
        }
    }
    if let Some(arr) = lesson["sehemu"].as_array() {
        for (i, sec) in arr.iter().enumerate() {
            let t = spoken_text(sec);
            if t.trim().is_empty() {
                continue;
            }
            if let Ok((f, b)) = synthesize(&t, language, voice, speed) {
                sections.insert(
                    i.to_string(),
                    json!({"file": f, "url": format!("/api/education/lesson/media/{f}"), "bytes": b}),
                );
            }
        }
    }
    if let Some(t) = lesson["hitimisho"].as_str().filter(|t| !t.trim().is_empty()) {
        if let Ok((f, b)) = synthesize(t, language, voice, speed) {
            concl = json!({"file": f, "url": format!("/api/education/lesson/media/{f}"), "bytes": b});
        }
    }
    out["available"] = json!(true);
    out["utangulizi"] = intro;
    out["sehemu"] = Value::Object(sections);
    out["hitimisho"] = concl;
    out
}

/// Hudumia sauti ya somo. Jina la faili linazuiliwa (hakuna `..`).
pub async fn lesson_media_serve(Path(name): Path<String>) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return (StatusCode::BAD_REQUEST, "jina la faili si halali").into_response();
    }
    let path = lessons_dir().join(&name);
    let Ok(bytes) = std::fs::read(&path) else {
        return (StatusCode::NOT_FOUND, "faili haipo").into_response();
    };
    let ct = match path.extension().and_then(|e| e.to_str()) {
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    };
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(axum::body::Body::from(bytes))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "kosa").into_response())
}

pub async fn lessons_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, i64, String)>(
        "SELECT id, title, language, level, minutes, source_type, chars, created_at
         FROM edu_lessons ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let lessons: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, language, level, minutes, source_type, chars, created_at)| {
            json!({
                "id": id, "title": title, "language": language, "level": level,
                "minutes": minutes, "source_type": source_type, "chars": chars,
                "created_at": created_at
            })
        })
        .collect();
    Json(json!({"ok": true, "lessons": lessons, "tools": tools_status()}))
}

pub async fn lesson_get(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64, String, String, i64, String, String, String, String)>(
        "SELECT id, title, language, level, minutes, source_type, source_name, chars,
                transcript, lesson, audio, created_at FROM edu_lessons WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((id, title, language, level, minutes, source_type, source_name, chars, transcript, lesson, audio, created_at)) = row else {
        return Json(json!({"ok": false, "error": "somo haipo"}));
    };
    Json(json!({
        "ok": true, "id": id, "title": title, "language": language, "level": level,
        "minutes": minutes, "source_type": source_type, "source_name": source_name,
        "chars": chars, "transcript": transcript,
        "lesson": serde_json::from_str::<Value>(&lesson).unwrap_or(json!({})),
        "audio": serde_json::from_str::<Value>(&audio).unwrap_or(json!({})),
        "created_at": created_at
    }))
}

#[derive(Deserialize)]
pub struct AudioBody {
    pub voice: Option<String>,
    pub speed: Option<i64>,
}

pub async fn lesson_audio(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<AudioBody>,
) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT lesson, language, title FROM edu_lessons WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((lesson, language, title)) = row else {
        return Json(json!({"ok": false, "error": "somo haipo"}));
    };
    let lesson: Value = serde_json::from_str(&lesson).unwrap_or(json!({}));
    let speed = b.speed.unwrap_or(150);
    let voice = b.voice.clone();
    let lang = language.clone();
    let audio = tokio::task::spawn_blocking(move || {
        generate_audio(&lesson, &lang, voice.as_deref(), speed)
    })
    .await
    .unwrap_or_else(|e| json!({"available": false, "error": e.to_string()}));

    let _ = sqlx::query("UPDATE edu_lessons SET audio = ?1 WHERE id = ?2")
        .bind(audio.to_string())
        .bind(&id)
        .execute(&s.db)
        .await;
    Json(json!({"ok": true, "id": id, "title": title, "audio": audio}))
}

#[derive(Deserialize)]
pub struct AskBody {
    pub question: String,
}

/// Mwanafunzi anauliza swali kuhusu somo — LLM hujibu kwa muktadha wa somo.
pub async fn lesson_ask(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<AskBody>,
) -> Json<Value> {
    if b.question.trim().is_empty() {
        return Json(json!({"ok": false, "error": "swali linahitajika"}));
    }
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT lesson, language, title FROM edu_lessons WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((lesson, language, title)) = row else {
        return Json(json!({"ok": false, "error": "somo haipo"}));
    };
    let lesson: Value = serde_json::from_str(&lesson).unwrap_or(json!({}));
    let context = lesson.to_string();
    let prompt = format!(
        "Wewe ni mwalimu wa AI. Jibu swali la mwanafunzi kwa lugha ya {language}, \
         kwa kutumia somo hili PEKEE kama muktadha. Jibu kwa ufupi na kwa mifano.\n\n\
         SOMO: {context}\n\nSWALI LA MWANAFUNZI: {q}\n\nJibu:",
        language = language,
        context = context.chars().take(8000).collect::<String>(),
        q = b.question.trim()
    );
    let Some(ans) = crate::education::llm_long(&s.db, &prompt, 180).await else {
        return Json(json!({"ok": false, "error": "LLM haikujibu"}));
    };
    Json(json!({"ok": true, "id": id, "title": title, "question": b.question, "answer": ans}))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_of_hutambua_aina() {
        assert_eq!(kind_of("notes.pdf"), "pdf");
        assert_eq!(kind_of("NOTES.PDF"), "pdf");
        assert_eq!(kind_of("somo.docx"), "docx");
        assert_eq!(kind_of("notes.txt"), "text");
        assert_eq!(kind_of("ukurasa.JPG"), "image");
        assert_eq!(kind_of("faili.xyz"), "unknown");
    }

    #[test]
    fn lang_code_hutambua_kiswahili() {
        assert_eq!(lang_code("Kiswahili"), "sw");
        assert_eq!(lang_code("sw"), "sw");
        assert_eq!(lang_code("English"), "en");
    }

    #[test]
    fn maandishi_mbichi_hutolewa() {
        let (t, k) = extract_text("notes.txt", b"  Mzunguko wa maji  ", "Kiswahili", 5).unwrap();
        assert_eq!(t, "Mzunguko wa maji");
        assert_eq!(k, "text");
    }

    #[test]
    fn faili_tupu_hurudisha_kosa() {
        assert!(extract_text("notes.txt", b"   ", "Kiswahili", 5).is_err());
    }

    #[test]
    fn aina_isiyotumika_hurudisha_kosa_la_wazi() {
        let e = extract_text("faili.xyz", b"abc", "Kiswahili", 5).unwrap_err();
        assert!(e.contains("haifai") || e.contains("haitumiki"));
    }

    #[test]
    fn docx_hutolewa_kwa_kweli() {
        // DOCX ndogo ya kweli: ZIP yenye word/document.xml (stored, method 0).
        let xml = r#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Mzunguko wa maji</w:t></w:r></w:p><w:p><w:r><w:t>Mvua na umimimaji</w:t></w:r></w:p></w:body></w:document>"#;
        let mut zip: Vec<u8> = Vec::new();
        let name = b"word/document.xml";
        zip.extend_from_slice(b"PK\x03\x04");
        zip.extend_from_slice(&[20, 0]); // version
        zip.extend_from_slice(&[0, 0]); // flags
        zip.extend_from_slice(&[0, 0]); // method = stored
        zip.extend_from_slice(&[0; 4]); // time/date
        zip.extend_from_slice(&[0; 4]); // crc (haijasomwa)
        zip.extend_from_slice(&(xml.len() as u32).to_le_bytes()); // comp size
        zip.extend_from_slice(&(xml.len() as u32).to_le_bytes()); // uncomp size
        zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes()); // extra len
        zip.extend_from_slice(name);
        zip.extend_from_slice(xml.as_bytes());

        let (t, k) = extract_text("somo.docx", &zip, "Kiswahili", 5).unwrap();
        assert_eq!(k, "docx");
        assert!(t.contains("Mzunguko wa maji"), "text: {t}");
        assert!(t.contains("Mvua na umimimaji"), "text: {t}");
    }

    #[test]
    fn spoken_text_huunganisha_sehemu() {
        let sec = json!({
            "kichwa": "1. Mvua",
            "maelezo": "Maji hupata joto.",
            "mfano": "Kama sufuria.",
            "uchambuzi": "Ni muhimu."
        });
        let t = spoken_text(&sec);
        assert!(t.starts_with("1. Mvua."), "text: {t}");
        assert!(t.contains("Maji hupata joto."));
        assert!(t.contains("Kama sufuria."));
    }

    #[test]
    fn audio_hurudisha_available_false_bila_espeak() {
        // Bila espeak kwenye PATH, haturudishi sauti za kubuni.
        if !espeak_ok() {
            let a = generate_audio(&json!({"sehemu": []}), "Kiswahili", None, 150);
            assert_eq!(a["available"], false);
        }
    }

    #[test]
    fn tools_status_ina_funguo_zote() {
        let s = tools_status();
        for k in ["pdftotext", "pdftoppm", "tesseract", "espeak", "ffmpeg"] {
            assert!(s.get(k).is_some(), "kikosekana: {k}");
        }
    }
}
