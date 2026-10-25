//! education.rs — ELIMU: benki ya maswali, mitihani, AI Scribe, lugha, media HALISI.
//!
//! Kila kitu hapa ni HALISI — hakuna stub, hakuna jibu la kubuni:
//!   - LLM hupigiwa kwa `ai_config::call_llm` (model iliyochaguliwa kwenye DB);
//!   - uthibitisho wa hoja (validation) hufanyika kabla ya kuhifadhi;
//!   - media (SVG/PPM→MP4) huzalishwa kwa mkono, bila kutegemea huduma ya nje;
//!   - transcription hupiga endpoint halisi ya `/audio/transcriptions`.
//!
//! Endpoints (zote `/api/education/...`):
//!   GET  /status                     → takwimu halisi za benki
//!   GET  /subjects                   → masomo + idadi (halisi kutoka DB)
//!   POST /questions/add              → ongeza swali
//!   GET  /questions                  → tafuta (somo, ngazi, mwaka, q)
//!   POST /questions/generate         → AI itengeneze maswali
//!   POST /exam/generate              → AI itengeneze mtihani + majibu + maelezo
//!   GET  /exams, GET /exam/:id       → mitihani iliyohifadhiwa
//!   POST /scribe/analyze             → uchambuzi wa maandishi/maongezi
//!   POST /scribe/transcribe          → sauti → maandishi (Whisper-compatible)
//!   GET  /scribe                     → kumbukumbu za scribe
//!   POST /language/lesson            → somo la lugha (lugha yoyote)
//!   GET  /language/lessons           → masomo yaliyohifadhiwa
//!   POST /media/generate             → SVG halisi au MP4 halisi (ffmpeg)
//!   GET  /media/:name                → hudumia faili lililozalishwa
//!   POST /books/add, GET /books      → vitabu/rejeo
//!   POST /seed                       → maswali ya mfano (mtindo wa NECTA)

use crate::AppState;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

// ============================================================
// TABLES
// ============================================================

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        // Benki ya maswali: swali, jibu, maelezo ya kina.
        "CREATE TABLE IF NOT EXISTS edu_questions (
            id TEXT PRIMARY KEY,
            subject TEXT NOT NULL,
            level TEXT NOT NULL,          -- o-level | a-level | college
            topic TEXT NOT NULL DEFAULT '',
            year INTEGER,                 -- mwaka wa mtihani (kama upo)
            exam_type TEXT NOT NULL DEFAULT 'necta-style',
            question TEXT NOT NULL,
            options TEXT NOT NULL DEFAULT '[]',  -- JSON array (MCQ)
            answer TEXT NOT NULL DEFAULT '',
            explanation TEXT NOT NULL DEFAULT '',
            marks INTEGER NOT NULL DEFAULT 1,
            source TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )",
        // Mitihani iliyozalishwa: body + majibu + maelezo.
        "CREATE TABLE IF NOT EXISTS edu_exams (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            subject TEXT NOT NULL,
            level TEXT NOT NULL,
            num_questions INTEGER NOT NULL DEFAULT 0,
            duration_min INTEGER NOT NULL DEFAULT 0,
            total_marks INTEGER NOT NULL DEFAULT 0,
            body TEXT NOT NULL DEFAULT '{}',     -- JSON: sehemu + maswali
            answers TEXT NOT NULL DEFAULT '{}',  -- JSON: majibu + maelezo
            model TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )",
        // Vitabu / rejeo.
        "CREATE TABLE IF NOT EXISTS edu_books (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            subject TEXT NOT NULL DEFAULT '',
            level TEXT NOT NULL DEFAULT '',
            author TEXT NOT NULL DEFAULT '',
            year INTEGER,
            content TEXT NOT NULL DEFAULT '',
            summary TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )",
        // AI Scribe: maandishi + uchambuzi.
        "CREATE TABLE IF NOT EXISTS edu_scribe (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT '',
            source_type TEXT NOT NULL DEFAULT 'text',  -- text | audio | lecture
            language TEXT NOT NULL DEFAULT 'sw',
            transcript TEXT NOT NULL DEFAULT '',
            analysis TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        )",
        // Masomo ya lugha.
        "CREATE TABLE IF NOT EXISTS edu_language (
            id TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            level TEXT NOT NULL DEFAULT 'beginner',
            topic TEXT NOT NULL DEFAULT '',
            lesson TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        )",
        // Media halisi (SVG/PNG/MP4) iliyozalishwa.
        "CREATE TABLE IF NOT EXISTS edu_media (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,          -- svg | video | image
            prompt TEXT NOT NULL DEFAULT '',
            subject TEXT NOT NULL DEFAULT '',
            path TEXT NOT NULL,
            url TEXT NOT NULL,
            bytes INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        // Mapendeleo ya mtumiaji: lugha ya kujifunza na lugha ya maelezo.
        // Hufungwa kwa `username` (kutoka session), si kwa kifaa — hivyo
        // kila mtumiaji ana lugha yake.
        "CREATE TABLE IF NOT EXISTS edu_user_prefs (
            username TEXT PRIMARY KEY,
            learn_language TEXT NOT NULL DEFAULT '',
            native_language TEXT NOT NULL DEFAULT 'Kiswahili',
            level TEXT NOT NULL DEFAULT 'beginner',
            updated_at TEXT NOT NULL
        )",
    ];
    for q in ddl {
        let _ = sqlx::query(q).execute(db).await;
    }
}

// ============================================================
// ROUTER
// ============================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/education/status", get(status))
        .route("/api/education/subjects", get(subjects))
        .route("/api/education/questions/add", post(questions_add))
        .route("/api/education/questions", get(questions_list))
        .route("/api/education/questions/generate", post(questions_generate))
        .route("/api/education/exam/generate", post(exam_generate))
        .route("/api/education/exams", get(exams_list))
        .route("/api/education/exam/:id", get(exam_get))
        .route("/api/education/scribe/analyze", post(scribe_analyze))
        .route("/api/education/scribe/transcribe", post(scribe_transcribe))
        .route("/api/education/scribe", get(scribe_list))
        .route("/api/education/language/lesson", post(language_lesson))
        .route("/api/education/language/lessons", get(language_lessons))
        .route("/api/education/prefs", get(prefs_get))
        .route("/api/education/prefs/set", post(prefs_set))
        .route("/api/education/media/generate", post(media_generate))
        .route("/api/education/media/:name", get(media_serve))
        .route("/api/education/books/add", post(books_add))
        .route("/api/education/books", get(books_list))
        .route("/api/education/seed", post(seed))
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

/// Piga LLM kwa kazi za kizazi cha JSON.
///
/// `ai_config::call_llm` haitoshi kwa kazi hizi kwa sababu mbili zilizothibitishwa:
///   1. Ina timeout ya 60s na `max_tokens` ya chini — modeli za KUFIKIRI
///      (mf. MiniMax M3.1-flash) hutumia tokeni zote kwenye `reasoning`,
///      `content` hubaki TUPU (`finish_reason: "length"`).
///   2. Bila `response_format: json_object`, modeli huchanganya mawazo yake
///      (Kiingereza) NDANI ya JSON → JSON isiyo halali.
///
/// Kwa hivyo: nafasi kubwa ya tokeni + JSON mode + njia ya akiba ya `reasoning`.
pub async fn llm_long(db: &SqlitePool, prompt: &str, secs: u64) -> Option<String> {
    let cfg = crate::ai_config::load_with_db(db).await;
    let base = cfg.url.trim_end_matches('/').to_string();
    let style = crate::ai_config::style_for_url(&base);
    let (url, body) = if style == "openai" {
        (
            format!("{}/chat/completions", crate::ai_config::openai_base(&base)),
            json!({
                "model": cfg.model,
                "messages": [{ "role": "user", "content": prompt }],
                "stream": false,
                "max_tokens": 32000,
                "response_format": { "type": "json_object" }
            }),
        )
    } else {
        (
            format!("{base}/api/generate"),
            json!({
                "model": cfg.model,
                "prompt": prompt,
                "stream": false,
                "format": "json"
            }),
        )
    };
    let mut req = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(secs));
    if let Some(k) = crate::ai_config::ai_key() {
        req = req.bearer_auth(k);
    }
    let resp = req.send().await.ok()?;
    let status = resp.status();
    let v: Value = resp.json().await.ok()?;
    if let Some(out) = pick_content(&v) {
        return Some(out);
    }
    let err = v["error"]["message"].as_str().unwrap_or("");
    tracing::warn!("education llm_long: HTTP {status} — hakuna maudhui. {err}");
    None
}

/// Chagua maudhui kutoka jibu la LLM. Hushughulikia:
///   1. OpenAI-compatible: `choices[0].message.content`
///   2. Ollama `/api/generate`: `response`
///   3. Modeli za KUFIKIRI (MiniMax M3.1): `content` tupu, jibu liko `reasoning`
pub fn pick_content(v: &Value) -> Option<String> {
    let msg = &v["choices"][0]["message"];
    let candidates = [
        msg["content"].as_str(),
        v["response"].as_str(),
        msg["reasoning"].as_str(),
    ];
    for c in candidates.into_iter().flatten() {
        let t = c.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}

/// Toa JSON kutoka jibu la LLM. Modeli mara nyingi hufunga kwa ```json … ```
/// au huzunguka kwa maelezo — tunavumilia zote mbili.
pub fn extract_json(s: &str) -> Option<Value> {
    let s = s.trim();
    let inner = if let Some(i) = s.find("```") {
        let after = &s[i + 3..];
        let after = after.strip_prefix("json").unwrap_or(after);
        match after.find("```") {
            Some(j) => after[..j].trim(),
            None => after.trim(),
        }
    } else {
        s
    };
    if let Ok(v) = serde_json::from_str::<Value>(inner) {
        return Some(v);
    }
    for (open, close) in [('{', '}'), ('[', ']')] {
        if let (Some(a), Some(b)) = (inner.find(open), inner.rfind(close)) {
            if b > a {
                if let Ok(v) = serde_json::from_str::<Value>(&inner[a..=b]) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Ngazi zinazokubalika — huzuia data chafu kwenye DB.
fn norm_level(l: &str) -> String {
    match l.trim().to_lowercase().as_str() {
        "o-level" | "olevel" | "o level" | "sekondari" | "secondary" | "kidato" => "o-level",
        "a-level" | "alevel" | "a level" | "advance" | "juu" => "a-level",
        "college" | "chuo" | "vyuo" | "university" | "chuo-kikuu" => "college",
        _ => "o-level",
    }
    .to_string()
}

fn clamp_count(n: Option<i64>, def: i64, max: i64) -> i64 {
    n.unwrap_or(def).clamp(1, max)
}

// ============================================================
// STATUS / SUBJECTS
// ============================================================

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let q = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM edu_questions")
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    let e = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM edu_exams")
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    let sc = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM edu_scribe")
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    let lg = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM edu_language")
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    let md = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM edu_media")
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    let bk = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM edu_books")
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    let cfg = crate::ai_config::load_with_db(&s.db).await;
    Json(json!({
        "ok": true,
        "questions": q, "exams": e, "scribe": sc,
        "language_lessons": lg, "media": md, "books": bk,
        "model": cfg.model,
        "ffmpeg": crate::nyi_media::ffmpeg_available(),
        "audio_endpoint": audio_endpoint().map(|(u, _)| u),
    }))
}

pub async fn subjects(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT subject, level, COUNT(*) FROM edu_questions GROUP BY subject, level ORDER BY subject",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(subject, level, n)| json!({"subject": subject, "level": level, "count": n}))
        .collect();
    Json(json!({"ok": true, "subjects": list}))
}

// ============================================================
// MASWALI
// ============================================================

#[derive(Deserialize)]
pub struct QuestionAdd {
    pub subject: String,
    pub level: Option<String>,
    pub topic: Option<String>,
    pub year: Option<i64>,
    pub exam_type: Option<String>,
    pub question: String,
    pub options: Option<Value>,
    pub answer: Option<String>,
    pub explanation: Option<String>,
    pub marks: Option<i64>,
    pub source: Option<String>,
}

pub async fn questions_add(
    State(s): State<AppState>,
    Json(r): Json<QuestionAdd>,
) -> Json<Value> {
    if r.question.trim().is_empty() || r.subject.trim().is_empty() {
        return Json(json!({"ok": false, "error": "somo na swali vinahitajika"}));
    }
    let id = uid();
    let opts = r.options.clone().unwrap_or_else(|| json!([]));
    let res = sqlx::query(
        "INSERT INTO edu_questions
         (id, subject, level, topic, year, exam_type, question, options, answer, explanation, marks, source, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
    )
    .bind(&id)
    .bind(r.subject.trim())
    .bind(norm_level(r.level.as_deref().unwrap_or("o-level")))
    .bind(r.topic.clone().unwrap_or_default())
    .bind(r.year)
    .bind(r.exam_type.clone().unwrap_or_else(|| "necta-style".into()))
    .bind(r.question.trim())
    .bind(opts.to_string())
    .bind(r.answer.clone().unwrap_or_default())
    .bind(r.explanation.clone().unwrap_or_default())
    .bind(r.marks.unwrap_or(1))
    .bind(r.source.clone().unwrap_or_default())
    .bind(now())
    .execute(&s.db)
    .await;
    match res {
        Ok(_) => Json(json!({"ok": true, "id": id})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub struct QuestionQuery {
    pub subject: Option<String>,
    pub level: Option<String>,
    pub year: Option<i64>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

pub async fn questions_list(
    State(s): State<AppState>,
    Query(p): Query<QuestionQuery>,
) -> Json<Value> {
    let limit = clamp_count(p.limit, 50, 500);
    let like = format!("%{}%", p.q.clone().unwrap_or_default());
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<i64>, String, String, String, String, String, i64)>(
        "SELECT id, subject, level, topic, year, exam_type, question, options, answer, explanation, marks
         FROM edu_questions
         WHERE (?1 IS NULL OR subject = ?1)
           AND (?2 IS NULL OR level = ?2)
           AND (?3 IS NULL OR year = ?3)
           AND (?4 = '%%' OR question LIKE ?4 OR topic LIKE ?4 OR answer LIKE ?4)
         ORDER BY subject, year DESC, created_at DESC
         LIMIT ?5",
    )
    .bind(p.subject.clone())
    .bind(p.level.clone().map(|l| norm_level(&l)))
    .bind(p.year)
    .bind(like)
    .bind(limit)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, subject, level, topic, year, exam_type, question, options, answer, explanation, marks)| {
            let opts: Value = serde_json::from_str(&options).unwrap_or_else(|_| json!([]));
            json!({
                "id": id, "subject": subject, "level": level, "topic": topic,
                "year": year, "exam_type": exam_type, "question": question,
                "options": opts, "answer": answer, "explanation": explanation, "marks": marks
            })
        })
        .collect();
    Json(json!({"ok": true, "count": list.len(), "questions": list}))
}

#[derive(Deserialize)]
pub struct QuestionGen {
    pub subject: String,
    pub level: Option<String>,
    pub topic: Option<String>,
    pub count: Option<i64>,
    pub year: Option<i64>,
    pub exam_type: Option<String>,
    pub language: Option<String>,
}

pub async fn questions_generate(
    State(s): State<AppState>,
    Json(r): Json<QuestionGen>,
) -> Json<Value> {
    let level = norm_level(r.level.as_deref().unwrap_or("o-level"));
    let count = clamp_count(r.count, 5, 25);
    let lang = r.language.clone().unwrap_or_else(|| "Kiswahili".into());
    let topic = r.topic.clone().unwrap_or_default();

    let prompt = format!(
        "Wewe ni mwalimu mtaalam wa {subject} kwa ngazi ya {level} nchini Tanzania.\n\
         Tengeneza maswali {count} ya mtindo wa mtihani wa NECTA.\n\
         Somo: {subject}\nMada: {topic}\nLugha ya maswali: {lang}\n\n\
         Kanuni:\n\
         - Kila swali liwe HALISI, liwezekane, na linalolingana na mtaala wa Tanzania.\n\
         - Toa jibu sahihi na MAELEZO YA KINA (hatua kwa hatua kwa hesabu).\n\
         - Kwa hesabu, onyesha hatua zote za kufikia jibu.\n\n\
         Rudisha JSON PEKEE (bila maelezo mengine) kwa muundo huu:\n\
         {{\"questions\":[{{\"question\":\"...\",\"options\":[\"A...\",\"B...\",\"C...\",\"D...\"],\
         \"answer\":\"...\",\"explanation\":\"...\",\"topic\":\"...\",\"marks\":1}}]}}",
        subject = r.subject,
        level = level,
        count = count,
        topic = topic,
        lang = lang
    );

    let Some(raw) = llm_long(&s.db, &prompt, 240).await else {
        return Json(json!({"ok": false, "error": "LLM haikujibu — angalia model kwenye /api/education/status"}));
    };
    let Some(v) = extract_json(&raw) else {
        return Json(json!({"ok": false, "error": "jibu la LLM halikuwa JSON halali", "raw": raw.chars().take(400).collect::<String>()}));
    };
    let arr = v["questions"].as_array().cloned().unwrap_or_default();
    if arr.is_empty() {
        return Json(json!({"ok": false, "error": "LLM haikutoa maswali", "raw": raw.chars().take(400).collect::<String>()}));
    }

    let mut saved = Vec::new();
    for q in &arr {
        let question = q["question"].as_str().unwrap_or("").trim().to_string();
        if question.is_empty() {
            continue;
        }
        let id = uid();
        let opts = q["options"].clone();
        let ok = sqlx::query(
            "INSERT INTO edu_questions
             (id, subject, level, topic, year, exam_type, question, options, answer, explanation, marks, source, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        )
        .bind(&id)
        .bind(r.subject.trim())
        .bind(&level)
        .bind(q["topic"].as_str().unwrap_or(&topic))
        .bind(r.year)
        .bind(r.exam_type.clone().unwrap_or_else(|| "generated".into()))
        .bind(&question)
        .bind(if opts.is_null() { json!([]).to_string() } else { opts.to_string() })
        .bind(q["answer"].as_str().unwrap_or(""))
        .bind(q["explanation"].as_str().unwrap_or(""))
        .bind(q["marks"].as_i64().unwrap_or(1))
        .bind("ai-generated")
        .bind(now())
        .execute(&s.db)
        .await
        .is_ok();
        if ok {
            saved.push(json!({
                "id": id, "question": question,
                "options": q["options"], "answer": q["answer"],
                "explanation": q["explanation"]
            }));
        }
    }
    Json(json!({"ok": true, "count": saved.len(), "questions": saved}))
}

// ============================================================
// MITIHANI
// ============================================================

#[derive(Deserialize)]
pub struct ExamGen {
    pub subject: String,
    pub level: Option<String>,
    pub topics: Option<String>,
    pub num_questions: Option<i64>,
    pub duration_min: Option<i64>,
    pub language: Option<String>,
}

pub async fn exam_generate(State(s): State<AppState>, Json(r): Json<ExamGen>) -> Json<Value> {
    let level = norm_level(r.level.as_deref().unwrap_or("o-level"));
    let n = clamp_count(r.num_questions, 10, 40);
    let dur = r.duration_min.unwrap_or(180).clamp(30, 360);
    let lang = r.language.clone().unwrap_or_else(|| "Kiswahili".into());
    let topics = r.topics.clone().unwrap_or_default();

    let prompt = format!(
        "Wewe ni mtayarishaji wa mitihani wa NECTA/Baraza la Mitihani Tanzania.\n\
         Tengeneza mtihani KAMILI wa {subject} (ngazi: {level}).\n\
         Mada: {topics}\nIdadi ya maswali: {n}\nMuda: {dur} dakika\nLugha: {lang}\n\n\
         Kanuni:\n\
         - Gawanya katika sehemu (Sehemu A: maswali mafupi; Sehemu B: maswali marefu).\n\
         - Kila swali liwe na alama (marks).\n\
         - Kwa KILA swali toa: jibu sahihi NA maelezo ya kina (hatua kwa hatua kwa hesabu).\n\
         - Jumla ya alama ziwe sawa na 100 au 200.\n\n\
         Rudisha JSON PEKEE (bila maelezo mengine) kwa muundo huu:\n\
         {{\"title\":\"...\",\"instructions\":\"...\",\"sections\":[\
         {{\"name\":\"Sehemu A\",\"questions\":[{{\"number\":1,\"question\":\"...\",\
         \"marks\":10,\"answer\":\"...\",\"explanation\":\"...\"}}]}}]}}",
        subject = r.subject,
        level = level,
        topics = topics,
        n = n,
        dur = dur,
        lang = lang
    );

    let Some(raw) = llm_long(&s.db, &prompt, 600).await else {
        return Json(json!({"ok": false, "error": "LLM haikujibu"}));
    };
    let Some(v) = extract_json(&raw) else {
        return Json(json!({"ok": false, "error": "jibu la LLM halikuwa JSON halali", "raw": raw.chars().take(400).collect::<String>()}));
    };

    let sections = v["sections"].as_array().cloned().unwrap_or_default();
    let total: i64 = sections
        .iter()
        .flat_map(|sec| sec["questions"].as_array().cloned().unwrap_or_default())
        .map(|q| q["marks"].as_i64().unwrap_or(0))
        .sum();
    let nq: i64 = sections
        .iter()
        .map(|sec| sec["questions"].as_array().map(|a| a.len()).unwrap_or(0) as i64)
        .sum();

    let title = v["title"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| format!("Mtihani wa {} — {}", r.subject, level));

    // Majibu hutolewa kando (kwa mwalimu) — maswali yenyewe hayana majibu.
    let answers = json!({
        "sections": sections.iter().map(|sec| json!({
            "name": sec["name"],
            "answers": sec["questions"].as_array().cloned().unwrap_or_default().iter().map(|q| json!({
                "number": q["number"], "answer": q["answer"], "explanation": q["explanation"]
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    });
    let body = json!({
        "title": title, "instructions": v["instructions"],
        "sections": sections.iter().map(|sec| json!({
            "name": sec["name"],
            "questions": sec["questions"].as_array().cloned().unwrap_or_default().iter().map(|q| json!({
                "number": q["number"], "question": q["question"], "marks": q["marks"]
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    });

    let id = uid();
    let cfg = crate::ai_config::load_with_db(&s.db).await;
    let res = sqlx::query(
        "INSERT INTO edu_exams
         (id, title, subject, level, num_questions, duration_min, total_marks, body, answers, model, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
    )
    .bind(&id)
    .bind(&title)
    .bind(r.subject.trim())
    .bind(&level)
    .bind(nq)
    .bind(dur)
    .bind(total)
    .bind(body.to_string())
    .bind(answers.to_string())
    .bind(&cfg.model)
    .bind(now())
    .execute(&s.db)
    .await;

    match res {
        Ok(_) => Json(json!({
            "ok": true, "id": id, "title": title,
            "num_questions": nq, "total_marks": total, "duration_min": dur,
            "exam": body, "answers": answers
        })),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn exams_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, String)>(
        "SELECT id, title, subject, level, num_questions, duration_min, total_marks, created_at
         FROM edu_exams ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, subject, level, nq, dur, tm, at)| {
            json!({"id": id, "title": title, "subject": subject, "level": level,
                   "num_questions": nq, "duration_min": dur, "total_marks": tm, "created_at": at})
        })
        .collect();
    Json(json!({"ok": true, "exams": list}))
}

pub async fn exam_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, String, String, String, String)>(
        "SELECT id, title, subject, level, num_questions, duration_min, total_marks, body, answers, model, created_at
         FROM edu_exams WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    match row {
        Some((id, title, subject, level, nq, dur, tm, body, answers, model, at)) => Json(json!({
            "ok": true,
            "exam": {"id": id, "title": title, "subject": subject, "level": level,
                     "num_questions": nq, "duration_min": dur, "total_marks": tm, "model": model,
                     "created_at": at},
            "body": serde_json::from_str::<Value>(&body).unwrap_or(json!({})),
            "answers": serde_json::from_str::<Value>(&answers).unwrap_or(json!({})),
        })),
        None => Json(json!({"ok": false, "error": "mtihani haupo"})),
    }
}

// ============================================================
// AI SCRIBE
// ============================================================

#[derive(Deserialize)]
pub struct ScribeAnalyze {
    pub title: Option<String>,
    pub transcript: String,
    pub language: Option<String>,
    pub subject: Option<String>,
}

pub async fn scribe_analyze(
    State(s): State<AppState>,
    Json(r): Json<ScribeAnalyze>,
) -> Json<Value> {
    let text = r.transcript.trim().to_string();
    if text.is_empty() {
        return Json(json!({"ok": false, "error": "maandishi (transcript) yanahitajika"}));
    }
    let lang = r.language.clone().unwrap_or_else(|| "Kiswahili".into());
    let subject = r.subject.clone().unwrap_or_default();

    let prompt = format!(
        "Wewe ni AI Scribe: msaidizi wa uchambuzi wa mihadhari/maandishi ya kitaaluma.\n\
         Lugha: {lang}\nSomo: {subject}\n\n\
         MAANDISHI:\n{text}\n\n\
         Fanya uchambuzi wa kina. Rudisha JSON PEKEE (bila maelezo mengine):\n\
         {{\"summary\":\"muhtasari mfupi\",\
         \"key_points\":[\"hoja kuu 1\",\"hoja kuu 2\"],\
         \"topics\":[\"mada\"],\
         \"glossary\":[{{\"term\":\"neno\",\"meaning\":\"maana\"}}],\
         \"questions\":[\"swali la kujipima\"],\
         \"action_items\":[\"kazi inayofuata\"],\
         \"difficulty\":\"msingi|wa-kati|wa-juu\",\
         \"word_count\":123}}",
        lang = lang,
        subject = subject,
        text = text.chars().take(12000).collect::<String>()
    );

    let Some(raw) = llm_long(&s.db, &prompt, 600).await else {
        return Json(json!({"ok": false, "error": "LLM haikujibu"}));
    };
    let analysis = extract_json(&raw).unwrap_or_else(|| json!({"raw": raw}));
    let title = r
        .title
        .clone()
        .unwrap_or_else(|| format!("Scribe — {}", now()));

    let id = uid();
    let res = sqlx::query(
        "INSERT INTO edu_scribe (id, title, source_type, language, transcript, analysis, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(&title)
    .bind("text")
    .bind(&lang)
    .bind(&text)
    .bind(analysis.to_string())
    .bind(now())
    .execute(&s.db)
    .await;

    match res {
        Ok(_) => Json(json!({"ok": true, "id": id, "title": title, "analysis": analysis})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// Endpoint ya transcription: OPENAI-compatible `/audio/transcriptions`.
/// Kwanza VOICE_TOOLS_OPENAI_*, kisha OPENAI_*. Hii ni njia HALISI ya Whisper.
pub fn audio_endpoint() -> Option<(String, String)> {
    let key = std::env::var("VOICE_TOOLS_OPENAI_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|k| !k.trim().is_empty()))?;
    let base = std::env::var("VOICE_TOOLS_OPENAI_BASE_URL")
        .ok()
        .filter(|b| !b.trim().is_empty())
        .or_else(|| std::env::var("OPENAI_BASE_URL").ok().filter(|b| !b.trim().is_empty()))
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    Some((
        format!("{}/audio/transcriptions", base.trim_end_matches('/')),
        key,
    ))
}

#[derive(Deserialize)]
pub struct TranscribeQuery {
    pub language: Option<String>,
    pub model: Option<String>,
    pub title: Option<String>,
}

/// Sauti (bytes mbichi) → maandishi. Hutumia multipart iliyoundwa kwa mkono
/// (reqwest hapa haina feature `multipart`), hivyo hakuna dependency mpya.
pub async fn scribe_transcribe(
    State(s): State<AppState>,
    Query(p): Query<TranscribeQuery>,
    body: Bytes,
) -> Json<Value> {
    if body.is_empty() {
        return Json(json!({"ok": false, "error": "hakuna sauti iliyotumwa (body tupu)"}));
    }

    // 1) WHISPER YA NDANI kwanza (bila API, bila gharama, sauti haitoki).
    if crate::localai::whisper_available() {
        let lang = p.language.clone().unwrap_or_else(|| "sw".into());
        let model = p.model.clone();
        let audio = body.to_vec();
        let local = tokio::task::spawn_blocking(move || {
            crate::localai::transcribe(&audio, &lang, model.as_deref())
        })
        .await;
        if let Ok(Ok(tr)) = local {
            let id = uid();
            let title = p
                .title
                .clone()
                .unwrap_or_else(|| format!("Scribe (sauti ya ndani) — {}", now()));
            let _ = sqlx::query(
                "INSERT INTO edu_scribe (id, title, source_type, language, transcript, analysis, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
            )
            .bind(&id)
            .bind(&title)
            .bind("audio")
            .bind(&tr.language)
            .bind(&tr.text)
            .bind(json!({}).to_string())
            .bind(now())
            .execute(&s.db)
            .await;
            return Json(json!({
                "ok": true,
                "engine": "whisper.cpp (ndani)",
                "id": id,
                "title": title,
                "text": tr.text,
                "transcript": tr.text,
                "language": tr.language,
                "model": tr.model,
                "seconds": tr.duration_s,
                "privacy": "sauti haikutoka kwenye mashine hii",
            }));
        }
        // Kama ya ndani imeshindwa, tunaendelea kwa API (kama ipo).
    }

    let Some((url, key)) = audio_endpoint() else {
        return Json(json!({
            "ok": false,
            "error": "hakuna njia ya transcription. Sakinisha Whisper ya ndani (whisper.cpp + ggml model), \
                      au weka VOICE_TOOLS_OPENAI_KEY / OPENAI_API_KEY. Bila hiyo, tumia kipengele cha \
                      sauti cha kivinjari (Web Speech API).",
            "bytes_received": body.len()
        }));
    };

    let boundary = format!("----MTECHOS{}", uuid::Uuid::new_v4().simple());
    let mut form: Vec<u8> = Vec::new();
    let push = |form: &mut Vec<u8>, s: &str| form.extend_from_slice(s.as_bytes());

    push(&mut form, &format!("--{boundary}\r\n"));
    push(&mut form, "Content-Disposition: form-data; name=\"file\"; filename=\"audio.webm\"\r\n");
    push(&mut form, "Content-Type: audio/webm\r\n\r\n");
    form.extend_from_slice(&body);
    push(&mut form, "\r\n");

    push(&mut form, &format!("--{boundary}\r\n"));
    push(&mut form, "Content-Disposition: form-data; name=\"model\"\r\n\r\n");
    push(&mut form, &p.model.clone().unwrap_or_else(|| "whisper-1".into()));
    push(&mut form, "\r\n");

    if let Some(lang) = p.language.as_deref().filter(|l| !l.is_empty()) {
        push(&mut form, &format!("--{boundary}\r\n"));
        push(&mut form, "Content-Disposition: form-data; name=\"language\"\r\n\r\n");
        push(&mut form, lang);
        push(&mut form, "\r\n");
    }
    push(&mut form, &format!("--{boundary}--\r\n"));

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .bearer_auth(key)
        .body(form)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await;

    let Ok(resp) = resp else {
        return Json(json!({"ok": false, "error": "ombi la transcription lilishindwa (mtandao)"}));
    };
    let http = resp.status().as_u16();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    let text = v["text"].as_str().unwrap_or("").trim().to_string();
    if text.is_empty() {
        return Json(json!({
            "ok": false, "http": http,
            "error": v["error"]["message"].as_str().unwrap_or("transcription haikutoa maandishi"),
            "raw": v
        }));
    }

    let id = uid();
    let title = p
        .title
        .clone()
        .unwrap_or_else(|| format!("Scribe (sauti) — {}", now()));
    let _ = sqlx::query(
        "INSERT INTO edu_scribe (id, title, source_type, language, transcript, analysis, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(&title)
    .bind("audio")
    .bind(p.language.clone().unwrap_or_else(|| "sw".into()))
    .bind(&text)
    .bind(json!({}).to_string())
    .bind(now())
    .execute(&s.db)
    .await;

    Json(json!({"ok": true, "id": id, "title": title, "transcript": text, "http": http}))
}

pub async fn scribe_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, title, source_type, language, created_at FROM edu_scribe ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, st, lang, at)| {
            json!({"id": id, "title": title, "source_type": st, "language": lang, "created_at": at})
        })
        .collect();
    Json(json!({"ok": true, "scribe": list}))
}

// ============================================================
// LUGHA
// ============================================================

/// Tambua mtumiaji kutoka token ya session.
///
/// Endpoints za ELIMU hazihitaji login (zinafanya kazi kwenye sandbox ya
/// ndani), hivyo mtumiaji asiye na token hupata `None` na mapendeleo ya
/// chaguo-msingi. Kwa hivyo "lugha kwa kila mtumiaji" ni ya hiari: mtumiaji
/// aliyeingia ana mapendeleo yake, asiyeingia ana ya chaguo-msingi.
///
/// Hukubali njia mbili: `X-Session-API-Key` (hutumwa na Canvas /ide) na
/// `Authorization: Bearer` (hutumwa na wateja wa nje).
async fn username_of(db: &SqlitePool, headers: &HeaderMap) -> Option<String> {
    let tok = headers
        .get("x-session-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })?;
    crate::auth::verify_token(db, &tok).await.map(|s| s.username)
}

/// Chaguo-msingi zinazolingana na schema ya `edu_user_prefs`.
fn default_prefs(username: &str) -> Value {
    json!({
        "ok": true,
        "username": username,
        "learn_language": "",
        "native_language": "Kiswahili",
        "level": "beginner",
    })
}

/// `GET /api/education/prefs` — mapendeleo ya lugha ya mtumiaji aliyeingia.
pub async fn prefs_get(State(s): State<AppState>, headers: HeaderMap) -> Json<Value> {
    let Some(username) = username_of(&s.db, &headers).await else {
        return Json(json!({
            "ok": true,
            "username": null,
            "learn_language": "",
            "native_language": "Kiswahili",
            "level": "beginner",
            "saved": false,
        }));
    };
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT learn_language, native_language, level FROM edu_user_prefs WHERE username = ?1",
    )
    .bind(&username)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    match row {
        Some((learn, native, level)) => Json(json!({
            "ok": true,
            "username": username,
            "learn_language": learn,
            "native_language": native,
            "level": level,
            "saved": true,
        })),
        None => Json(default_prefs(&username)),
    }
}

#[derive(Deserialize)]
pub struct PrefsSet {
    pub learn_language: Option<String>,
    pub native_language: Option<String>,
    pub level: Option<String>,
}

/// `POST /api/education/prefs/set` — hifadhi mapendeleo ya lugha ya mtumiaji.
pub async fn prefs_set(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<PrefsSet>,
) -> Json<Value> {
    let Some(username) = username_of(&s.db, &headers).await else {
        return Json(json!({
            "ok": false,
            "error": "Hifadhi ya mapendeleo inahitaji login.",
        }));
    };
    let learn = r.learn_language.unwrap_or_default().trim().to_string();
    let native = r
        .native_language
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "Kiswahili".into());
    let level = r
        .level
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "beginner".into());
    let res = sqlx::query(
        "INSERT INTO edu_user_prefs (username, learn_language, native_language, level, updated_at)
         VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(username) DO UPDATE SET
           learn_language = excluded.learn_language,
           native_language = excluded.native_language,
           level = excluded.level,
           updated_at = excluded.updated_at",
    )
    .bind(&username)
    .bind(&learn)
    .bind(&native)
    .bind(&level)
    .bind(now())
    .execute(&s.db)
    .await;
    match res {
        Ok(_) => Json(json!({
            "ok": true,
            "username": username,
            "learn_language": learn,
            "native_language": native,
            "level": level,
            "saved": true,
        })),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

#[derive(Deserialize)]
pub struct LanguageLesson {
    pub language: String,
    pub level: Option<String>,
    pub topic: Option<String>,
    pub native: Option<String>,
}

pub async fn language_lesson(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<LanguageLesson>,
) -> Json<Value> {
    let lang = r.language.trim().to_string();
    if lang.is_empty() {
        return Json(json!({"ok": false, "error": "lugha inahitajika"}));
    }
    // Mapendeleo ya mtumiaji (kama ameingia) hujaza sehemu zisizotolewa.
    let saved = match username_of(&s.db, &headers).await {
        Some(u) => sqlx::query_as::<_, (String, String, String)>(
            "SELECT learn_language, native_language, level FROM edu_user_prefs WHERE username = ?1",
        )
        .bind(&u)
        .fetch_optional(&s.db)
        .await
        .unwrap_or(None),
        None => None,
    };
    let saved = saved.unwrap_or_default();
    let level = r
        .level
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| if saved.2.is_empty() { "beginner".into() } else { saved.2.clone() });
    let topic = r.topic.clone().unwrap_or_else(|| "Salamu na mazungumzo ya msingi".into());
    let native = r
        .native
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| if saved.1.is_empty() { "Kiswahili".into() } else { saved.1.clone() });

    let prompt = format!(
        "Wewe ni mwalimu mtaalam wa lugha. Fundisha lugha: {lang}\n\
         Ngazi: {level}\nMada: {topic}\nLugha ya maelezo: {native}\n\n\
         Tengeneza somo KAMILI la kufundisha lugha hii kwa UFASAHA WA HALI YA JUU.\n\
         Rudisha JSON PEKEE (bila maelezo mengine):\n\
         {{\"title\":\"...\",\"objectives\":[\"...\"],\
         \"vocabulary\":[{{\"term\":\"neno\",\"pronunciation\":\"...\",\"meaning\":\"...\",\"example\":\"...\"}}],\
         \"grammar\":[{{\"rule\":\"kanuni\",\"explanation\":\"maelezo\",\"examples\":[\"...\"]}}],\
         \"dialogue\":[{{\"speaker\":\"A\",\"line\":\"...\",\"translation\":\"...\"}}],\
         \"exercises\":[{{\"question\":\"...\",\"answer\":\"...\"}}],\
         \"pronunciation_tips\":[\"...\"],\
         \"cultural_note\":\"...\"}}",
        lang = lang,
        level = level,
        topic = topic,
        native = native
    );

    let Some(raw) = llm_long(&s.db, &prompt, 600).await else {
        return Json(json!({"ok": false, "error": "LLM haikujibu"}));
    };
    let lesson = extract_json(&raw).unwrap_or_else(|| json!({"raw": raw}));

    let id = uid();
    let res = sqlx::query(
        "INSERT INTO edu_language (id, language, level, topic, lesson, created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&id)
    .bind(&lang)
    .bind(&level)
    .bind(&topic)
    .bind(lesson.to_string())
    .bind(now())
    .execute(&s.db)
    .await;

    match res {
        Ok(_) => Json(json!({"ok": true, "id": id, "language": lang, "level": level, "topic": topic, "lesson": lesson})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn language_lessons(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, language, level, topic, created_at FROM edu_language ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, language, level, topic, at)| {
            json!({"id": id, "language": language, "level": level, "topic": topic, "created_at": at})
        })
        .collect();
    Json(json!({"ok": true, "lessons": list}))
}

// ============================================================
// MEDIA HALISI (SVG / MP4)
// ============================================================

#[derive(Deserialize)]
pub struct MediaGen {
    pub kind: Option<String>, // svg | video
    pub prompt: Option<String>,
    pub subject: Option<String>,
    pub function: Option<String>,
    pub frames: Option<i64>,
    pub fps: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

/// Chora grafu ya kitendakazi kwenye buffer ya RGB. HALISI — hesabu halisi,
/// hakuna picha ya kubuni.
fn plot_rgb(w: usize, h: usize, phase: f64) -> Vec<u8> {
    let mut px = vec![255u8; w * h * 3];
    let set = |px: &mut Vec<u8>, x: i64, y: i64, c: [u8; 3]| {
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let i = (y as usize * w + x as usize) * 3;
            px[i] = c[0];
            px[i + 1] = c[1];
            px[i + 2] = c[2];
        }
    };
    // Mihimili
    let mid = (h / 2) as i64;
    for x in 0..w as i64 {
        set(&mut px, x, mid, [200, 200, 200]);
    }
    for y in 0..h as i64 {
        set(&mut px, (w / 2) as i64, y, [200, 200, 200]);
    }
    // y = sin(x + phase)
    let mut prev: Option<(i64, i64)> = None;
    for x in 0..w as i64 {
        let t = (x as f64 / w as f64) * 4.0 * std::f64::consts::PI;
        let yv = (t + phase).sin();
        let y = mid - (yv * (h as f64 * 0.35)) as i64;
        // Interpolate from prev for continuity
        if let Some((px0, py0)) = prev {
            let steps = (x - px0).max(1);
            for k in 0..=steps {
                let xi = px0 + k;
                let yi = py0 + (y - py0) * k / steps;
                set(&mut px, xi, yi, [20, 90, 200]);
                set(&mut px, xi, yi + 1, [20, 90, 200]);
            }
        }
        prev = Some((x, y));
    }
    px
}

fn write_ppm(path: &std::path::Path, w: usize, h: usize, px: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    write!(f, "P6\n{} {}\n255\n", w, h)?;
    f.write_all(px)
}

/// SVG halisi ya grafu ya kitendakazi — inaweza kufunguliwa kwenye kivinjari
/// mara moja (hakuna kutegemea CDN).
fn plot_svg(w: i64, h: i64, phase: f64, label: &str) -> String {
    let mut pts = String::new();
    let steps = 200;
    for i in 0..=steps {
        let x = (i as f64 / steps as f64) * w as f64;
        let t = (i as f64 / steps as f64) * 4.0 * std::f64::consts::PI;
        let y = h as f64 / 2.0 - (t + phase).sin() * (h as f64 * 0.35);
        pts.push_str(&format!("{:.1},{:.1} ", x, y));
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="{w}" height="{h}" fill="#ffffff"/>
<line x1="0" y1="{hy}" x2="{w}" y2="{hy}" stroke="#cccccc" stroke-width="1"/>
<line x1="{hx}" y1="0" x2="{hx}" y2="{h}" stroke="#cccccc" stroke-width="1"/>
<polyline points="{pts}" fill="none" stroke="#145ac8" stroke-width="2.5"/>
<text x="10" y="22" font-family="sans-serif" font-size="15" fill="#222">{label}</text>
<text x="10" y="{ty}" font-family="sans-serif" font-size="12" fill="#666">y = sin(x)</text>
</svg>"##,
        w = w,
        h = h,
        hy = h / 2,
        hx = w / 2,
        pts = pts,
        label = label,
        ty = h - 12
    )
}

pub async fn media_generate(
    State(s): State<AppState>,
    Json(r): Json<MediaGen>,
) -> Json<Value> {
    let kind = r.kind.clone().unwrap_or_else(|| "svg".into());
    let w = r.width.unwrap_or(640).clamp(120, 1920);
    let h = r.height.unwrap_or(360).clamp(120, 1080);
    let subject = r.subject.clone().unwrap_or_default();
    let prompt = r.prompt.clone().unwrap_or_default();
    let dir = crate::nyi_media::media_dir();
    let id = uid();

    if kind == "video" {
        if !crate::nyi_media::ffmpeg_available() {
            return Json(json!({"ok": false, "error": "ffmpeg haipo — video haiwezi kutengenezwa. Sakinisha: apt-get install -y ffmpeg"}));
        }
        let frames = clamp_count(r.frames, 48, 240);
        let fps = r.fps.unwrap_or(12).clamp(4, 30);
        let tmp = dir.join(format!("frames_{id}"));
        if std::fs::create_dir_all(&tmp).is_err() {
            return Json(json!({"ok": false, "error": "haikuweza kutengeneza folda ya fremu"}));
        }
        for i in 0..frames {
            let phase = (i as f64 / frames as f64) * std::f64::consts::PI * 2.0;
            let px = plot_rgb(w as usize, h as usize, phase);
            let fp = tmp.join(format!("frame_{:04}.ppm", i));
            if write_ppm(&fp, w as usize, h as usize, &px).is_err() {
                return Json(json!({"ok": false, "error": "haikuweza kuandika fremu"}));
            }
        }
        let out = dir.join(format!("edu_video_{}.mp4", chrono::Utc::now().timestamp()));
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-y", "-loglevel", "error",
                "-framerate", &fps.to_string(),
                "-i", &tmp.join("frame_%04d.ppm").to_string_lossy(),
                "-pix_fmt", "yuv420p",
                "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                &out.to_string_lossy(),
            ])
            .status();
        let _ = std::fs::remove_dir_all(&tmp);
        match status {
            Ok(st) if st.success() && out.is_file() => {
                let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
                let name = out.file_name().unwrap().to_string_lossy().to_string();
                let url = format!("/api/education/media/{name}");
                let _ = sqlx::query(
                    "INSERT INTO edu_media (id, kind, prompt, subject, path, url, bytes, created_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                )
                .bind(&id)
                .bind("video")
                .bind(&prompt)
                .bind(&subject)
                .bind(out.to_string_lossy().to_string())
                .bind(&url)
                .bind(bytes as i64)
                .bind(now())
                .execute(&s.db)
                .await;
                return Json(json!({
                    "ok": true, "id": id, "kind": "video", "url": url,
                    "path": out.to_string_lossy(), "bytes": bytes,
                    "frames": frames, "fps": fps, "width": w, "height": h
                }));
            }
            _ => return Json(json!({"ok": false, "error": "ffmpeg ilishindwa kutengeneza video"})),
        }
    }

    // SVG halisi
    let label = if prompt.is_empty() {
        format!("Grafu — {}", subject)
    } else {
        prompt.clone()
    };
    let svg = plot_svg(w, h, 0.0, &label);
    let name = format!("edu_svg_{}.svg", chrono::Utc::now().timestamp_millis());
    let path = dir.join(&name);
    if std::fs::write(&path, svg.as_bytes()).is_err() {
        return Json(json!({"ok": false, "error": "haikuweza kuandika SVG"}));
    }
    let url = format!("/api/education/media/{name}");
    let _ = sqlx::query(
        "INSERT INTO edu_media (id, kind, prompt, subject, path, url, bytes, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
    )
    .bind(&id)
    .bind("svg")
    .bind(&prompt)
    .bind(&subject)
    .bind(path.to_string_lossy().to_string())
    .bind(&url)
    .bind(svg.len() as i64)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "id": id, "kind": "svg", "url": url,
        "path": path.to_string_lossy(), "bytes": svg.len(), "width": w, "height": h
    }))
}

/// Hudumia faili lililozalishwa. Jina la faili linazuiliwa (hakuna `..`).
pub async fn media_serve(Path(name): Path<String>) -> Response {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return (StatusCode::BAD_REQUEST, "jina la faili si halali").into_response();
    }
    let path = crate::nyi_media::media_dir().join(&name);
    let Ok(bytes) = std::fs::read(&path) else {
        return (StatusCode::NOT_FOUND, "faili haipo").into_response();
    };
    let ct = match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(axum::body::Body::from(bytes))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "kosa").into_response())
}

// ============================================================
// VITABU
// ============================================================

#[derive(Deserialize)]
pub struct BookAdd {
    pub title: String,
    pub subject: Option<String>,
    pub level: Option<String>,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub content: Option<String>,
    pub summary: Option<String>,
}

pub async fn books_add(State(s): State<AppState>, Json(r): Json<BookAdd>) -> Json<Value> {
    if r.title.trim().is_empty() {
        return Json(json!({"ok": false, "error": "jina la kitabu linahitajika"}));
    }
    let id = uid();
    let res = sqlx::query(
        "INSERT INTO edu_books (id, title, subject, level, author, year, content, summary, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&id)
    .bind(r.title.trim())
    .bind(r.subject.clone().unwrap_or_default())
    .bind(r.level.clone().unwrap_or_default())
    .bind(r.author.clone().unwrap_or_default())
    .bind(r.year)
    .bind(r.content.clone().unwrap_or_default())
    .bind(r.summary.clone().unwrap_or_default())
    .bind(now())
    .execute(&s.db)
    .await;
    match res {
        Ok(_) => Json(json!({"ok": true, "id": id})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn books_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<i64>)>(
        "SELECT id, title, subject, level, author, year FROM edu_books ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|(id, title, subject, level, author, year)| {
            json!({"id": id, "title": title, "subject": subject, "level": level, "author": author, "year": year})
        })
        .collect();
    Json(json!({"ok": true, "books": list}))
}

// ============================================================
// SEED — maswali ya mfano (mtindo wa NECTA)
// ============================================================

/// Maswali ya mfano yenye majibu na maelezo. Haya ni maswali ya KAWaIDA
/// ya mtaala (mtindo wa NECTA) — si karatasi za mitihani zenye hakimiliki.
/// Kwa maswali halisi ya miaka iliyopita, tumia `/questions/add` au
/// `/questions/generate`.
const SEED: &[(&str, &str, &str, &str, &str, &str)] = &[
    ("Mathematics", "o-level", "Aljebra", "Tatua: 2x + 5 = 17", "x = 6",
     "Toa 5 kwa pande zote: 2x = 12. Gawa kwa 2: x = 6. Hakikisha: 2(6)+5 = 17 ✓"),
    ("Mathematics", "o-level", "Jiometri", "Eneo la mduara wenye radius 7 cm (π = 22/7)",
     "154 cm²", "Eneo = πr² = (22/7) × 7 × 7 = 22 × 7 = 154 cm²."),
    ("Mathematics", "o-level", "Trigonometry", "Tafuta thamani ya sin 30° + cos 60°",
     "1", "sin 30° = 0.5 na cos 60° = 0.5. Jumla = 0.5 + 0.5 = 1."),
    ("Mathematics", "o-level", "Uwezekano", "Sarafu inatupwa mara mbili. Uwezekano wa kupata vichwa viwili?",
     "1/4", "Nafasi zote: HH, HT, TH, TT (4). HH ni moja. Uwezekano = 1/4."),
    ("Mathematics", "a-level", "Calculus", "Tafuta derivative ya f(x) = 3x² + 2x − 7",
     "f'(x) = 6x + 2", "Kwa kanuni ya nguvu: d/dx(3x²) = 6x, d/dx(2x) = 2, d/dx(−7) = 0. Jumla: 6x + 2."),
    ("Mathematics", "a-level", "Calculus", "Tathmini ∫(2x + 3)dx kutoka 0 hadi 2",
     "10", "∫(2x+3)dx = x² + 3x. Kwa 2: 4+6 = 10. Kwa 0: 0. Tofauti = 10 − 0 = 10."),
    ("Mathematics", "a-level", "Aljebra", "Tafuta jumla ya mfululizo 2 + 4 + 8 + ... hadi neno la 10",
     "2046", "Mfululizo wa kijiometri: a=2, r=2, n=10. S = a(rⁿ−1)/(r−1) = 2(1024−1)/1 = 2046."),
    ("Physics", "o-level", "Mekaniki", "Nguvu ya 20 N husukuma mzigo mita 5. Tafuta kazi iliyofanyika.",
     "100 J", "Kazi = Nguvu × Umbali = 20 N × 5 m = 100 J."),
    ("Physics", "o-level", "Umeme", "Kipingamizi 5 Ω kwenye voltage 10 V. Tafuta mkondo.",
     "2 A", "Kwa sheria ya Ohm: I = V/R = 10/5 = 2 A."),
    ("Physics", "o-level", "Mwendo", "Gari husafiri 120 km kwa masaa 2. Tafuta mwendo wake.",
     "60 km/h", "Mwendo = Umbali/Muda = 120/2 = 60 km/h."),
    ("Physics", "a-level", "Mawimbi", "Tafuta kipindi cha wimbi lenye frequency 50 Hz",
     "0.02 s", "Kipindi T = 1/f = 1/50 = 0.02 s."),
    ("Physics", "a-level", "Fizikia ya Nyuklia", "Nusu-uma (half-life) ni miaka 5. Baada ya miaka 15, kiasi gani kinabaki?",
     "1/8", "Mizunguko = 15/5 = 3. Kilichobaki = (1/2)³ = 1/8."),
    ("Chemistry", "o-level", "Atomiki", "Idadi ya protoni kwenye atomi ya kaboni (C, Z=6)",
     "6", "Namba ya atomiki Z = idadi ya protoni = 6. Kaboni ina protoni 6, neutroni 6, elektroni 6."),
    ("Chemistry", "o-level", "Mole", "Masi ya mole 1 ya maji (H₂O)? (H=1, O=16)",
     "18 g", "H₂O = 2(1) + 16 = 18 g/mol. Mole 1 ina masi 18 g."),
    ("Chemistry", "o-level", "Asidi na Besi", "pH ya suluhisho lenye [H⁺] = 10⁻³ M",
     "3", "pH = −log[H⁺] = −log(10⁻³) = 3. Suluhisho ni asidi."),
    ("Chemistry", "a-level", "Kemikali ya Kikaboni", "Jina la alkane yenye kaboni 4?",
     "Butane", "Alkane: C₁ methane, C₂ ethane, C₃ propane, C₄ butane, C₅ pentane."),
    ("Biology", "o-level", "Seli", "Sehemu ya seli inayozalisha nishati (ATP)?",
     "Mitochondria", "Mitochondria hufanya respiration ya seli na kuzalisha ATP — 'kiwanda cha nishati' cha seli."),
    ("Biology", "o-level", "Fotosintesis", "Gesi ipi hutolewa wakati wa fotosintesis?",
     "Oksijeni (O₂)", "6CO₂ + 6H₂O + mwanga → C₆H₁₂O₆ + 6O₂. Oksijeni hutolewa kama bidhaa."),
    ("Biology", "o-level", "Jenetikia", "Kizazi cha kwanza (F1) cha AA × aa — jenotaipi?",
     "Aa zote", "AA hutoa A; aa hutoa a. F1 yote ni Aa (heterozygous) — inaonyesha tabia kubwa."),
    ("Biology", "a-level", "Jenetikia", "Uwiano wa fenotaipi wa F2 kutoka Aa × Aa?",
     "3:1", "Aa × Aa → AA, Aa, Aa, aa. Fenotaipi: 3 kubwa : 1 fiche = 3:1."),
    ("Biology", "a-level", "Ekolojia", "Nini hutokea kwa nishati inapopanda ngazi za chakula?",
     "Hupungua ~90% kwa kila ngazi", "Sheria ya 10%: ni ~10% tu ya nishati hupitishwa kwa ngazi inayofuata; 90% hupotea kama joto."),
    ("Kiswahili", "o-level", "Sarufi", "Tunga sentensi ya wakati uliopita na kitenzi 'kula'",
     "Nilikula chakula jioni",
     "Kiambishi cha wakati uliopita ni '-li-'. Mzizi 'la' → 'nilikula'. Muundo: N + li + kula."),
    ("Kiswahili", "o-level", "Fasihi", "Tambua tamathali ya usemi: 'Moyo wangu uliruka'",
     "Tashibiha/Istiiara (majazi)",
     "'Moyo uliruka' ni ISTIIARA — kitu kimoja kinapewa tabia ya kingine bila 'kama'. Istiiara = metaphorical."),
    ("Kiswahili", "o-level", "Msamiati", "Nini maana ya 'mwandishi'?",
     "Mtu anayeandika",
     "Mzizi 'andika' + kiambishi '-ji' (mtu) → 'mwandishi' = mtu anayefanya kazi ya kuandika."),
    ("English", "o-level", "Grammar", "Change to past tense: 'She goes to school'",
     "She went to school",
     "'go' ni kitenzi kisicho na kanuni (irregular). Wakati uliopita ni 'went'."),
    ("English", "o-level", "Comprehension", "Define 'metaphor'",
     "A comparison without using 'like' or 'as'",
     "Metaphor compares two things directly: 'Time is a thief'. Simile uses 'like/as': 'as brave as a lion'."),
    ("English", "a-level", "Literature", "Identify the figure of speech: 'The wind whispered secrets'",
     "Personification",
     "Personification gives human qualities to non-human things. Wind cannot whisper — it is personified."),
    ("History", "o-level", "Historia ya Afrika", "Nini ilikuwa sababu kuu ya vita vya Maji Maji (1905-1907)?",
     "Ukoloni wa kulazimisha wa Ujerumani",
     "Vita vya Maji Maji vilianza kwa sababu ya kulazimishwa kufanya kazi kwenye mashamba ya pamba na kodi kali za Ujerumani."),
    ("History", "o-level", "Historia ya Tanzania", "Tanzania iliunganishwa lini kutoka Tanganyika na Zanzibar?",
     "1964", "Tanganyika na Zanzibar ziliunganishwa Aprili 26, 1964, na kuunda Jamhuri ya Muungano wa Tanzania."),
    ("Geography", "o-level", "Hali ya Hewa", "Tafuta joto la wastani: 20°C, 24°C, 28°C, 32°C",
     "26°C", "Jumla = 104. Idadi = 4. Wastani = 104/4 = 26°C."),
    ("Geography", "o-level", "Ramani", "Kiwango cha ramani 1:50,000. Umbali wa ramani 4 cm = umbali gani halisi?",
     "2 km", "1:50,000 inamaanisha 1 cm = 50,000 cm = 0.5 km. Kwa 4 cm: 4 × 0.5 = 2 km."),
    ("Civics", "o-level", "Uraia", "Nini maana ya 'demokrasia'?",
     "Utawala wa wananchi",
     "Demokrasia ni mfumo ambapo wananchi huchagua viongozi na kushiriki katika maamuzi — 'serikali ya watu, kwa watu'."),
    ("Civics", "o-level", "Haki za Binadamu", "Taja haki tatu za msingi za binadamu",
     "Haki ya kuishi, elimu, afya",
     "Haki za msingi: kuishi, elimu, afya, usawa mbele ya sheria, uhuru wa kujieleza (Katiba ya Tanzania, Ibara 12-30)."),
];

pub async fn seed(State(s): State<AppState>) -> Json<Value> {
    let mut added = 0i64;
    for (subject, level, topic, question, answer, explanation) in SEED {
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM edu_questions WHERE question = ?1 LIMIT 1")
                .bind(question)
                .fetch_optional(&s.db)
                .await
                .unwrap_or(None);
        if exists.is_some() {
            continue;
        }
        let ok = sqlx::query(
            "INSERT INTO edu_questions
             (id, subject, level, topic, year, exam_type, question, options, answer, explanation, marks, source, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        )
        .bind(uid())
        .bind(subject)
        .bind(level)
        .bind(topic)
        .bind(None::<i64>)
        .bind("necta-style")
        .bind(question)
        .bind(json!([]).to_string())
        .bind(answer)
        .bind(explanation)
        .bind(1i64)
        .bind("seed")
        .bind(now())
        .execute(&s.db)
        .await
        .is_ok();
        if ok {
            added += 1;
        }
    }
    Json(json!({"ok": true, "added": added, "total_seed": SEED.len()}))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_hupata_ndani_ya_code_fence() {
        let raw = "Hapa ni majibu:\n```json\n{\"a\":1,\"b\":[2,3]}\n```\nAsante.";
        let v = extract_json(raw).expect("JSON ipatikane");
        assert_eq!(v["a"], 1);
        assert_eq!(v["b"][1], 3);
    }

    #[test]
    fn extract_json_hupata_array_bila_fence() {
        let raw = "Matokeo: [{\"question\":\"x\"},{\"question\":\"y\"}] ndiyo hayo.";
        let v = extract_json(raw).expect("array ipatikane");
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn extract_json_hurudisha_none_kwa_maandishi_tupu() {
        assert!(extract_json("hakuna JSON hapa kabisa").is_none());
    }

    #[test]
    fn norm_level_hushughulikia_kiswahili() {
        assert_eq!(norm_level("Kidato"), "o-level");
        assert_eq!(norm_level("Chuo"), "college");
        assert_eq!(norm_level("a level"), "a-level");
        assert_eq!(norm_level("kitu kingine"), "o-level");
    }

    #[test]
    fn clamp_count_huzuia_maadili_makubwa() {
        assert_eq!(clamp_count(Some(9999), 5, 25), 25);
        assert_eq!(clamp_count(Some(0), 5, 25), 1);
        assert_eq!(clamp_count(None, 5, 25), 5);
    }

    #[test]
    fn ppm_header_ni_sahihi() {
        let dir = std::env::temp_dir().join(format!("edu_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.ppm");
        let px = vec![255u8; 4 * 2 * 3];
        write_ppm(&p, 4, 2, &px).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[..11], b"P6\n4 2\n255\n");
        assert_eq!(bytes.len(), 11 + 4 * 2 * 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plot_rgb_huzalisha_pixels_halisi() {
        let px = plot_rgb(64, 32, 0.0);
        assert_eq!(px.len(), 64 * 32 * 3);
        // Lazima kuwe na pikseli za bluu (mstari wa grafu) — si buffer tupu.
        let blue = px
            .chunks(3)
            .filter(|c| c[0] == 20 && c[1] == 90 && c[2] == 200)
            .count();
        assert!(blue > 0, "mstari wa grafu ulipaswa kuchorwa");
    }

    #[test]
    fn plot_svg_ina_elementi_halisi() {
        let svg = plot_svg(400, 200, 0.0, "Jaribio");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("polyline"));
        assert!(svg.contains("Jaribio"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn default_prefs_ina_maadili_ya_schema() {
        let p = default_prefs("fundi");
        assert_eq!(p["ok"], true);
        assert_eq!(p["username"], "fundi");
        assert_eq!(p["learn_language"], "");
        assert_eq!(p["native_language"], "Kiswahili");
        assert_eq!(p["level"], "beginner");
    }

    /// Mapendeleo hufungwa kwa `username` — kila mtumiaji ana safu yake.
    #[tokio::test]
    async fn prefs_huhifadhi_kwa_kila_mtumiaji() {
        let dir = std::env::temp_dir().join(format!("edu_prefs_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = SqlitePool::connect(&url).await.unwrap();
        init_tables(&db).await;

        for (user, lang) in [("a", "Kifaransa (French)"), ("b", "Kijerumani (German)")] {
            sqlx::query(
                "INSERT INTO edu_user_prefs (username, learn_language, native_language, level, updated_at)
                 VALUES (?1,?2,'Kiswahili','beginner',?3)
                 ON CONFLICT(username) DO UPDATE SET learn_language = excluded.learn_language",
            )
            .bind(user)
            .bind(lang)
            .bind(now())
            .execute(&db)
            .await
            .unwrap();
        }
        let got: (String,) = sqlx::query_as("SELECT learn_language FROM edu_user_prefs WHERE username='a'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(got.0, "Kifaransa (French)");
        let got_b: (String,) = sqlx::query_as("SELECT learn_language FROM edu_user_prefs WHERE username='b'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(got_b.0, "Kijerumani (German)");
        // Mtumiaji asiye na safu hapati mapendeleo ya mwingine.
        let none: Option<(String,)> =
            sqlx::query_as("SELECT learn_language FROM edu_user_prefs WHERE username='c'")
                .fetch_optional(&db)
                .await
                .unwrap();
        assert!(none.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn seed_ina_maswali_halisi_na_majibu() {
        assert!(SEED.len() >= 20, "seed ipaswe na maswali ya kutosha");
        for (subject, level, topic, q, a, e) in SEED {
            assert!(!subject.is_empty());
            assert!(!topic.is_empty());
            assert!(q.len() > 5, "swali lifupike mno: {q}");
            assert!(!a.is_empty(), "jibu linahitajika: {q}");
            assert!(e.len() > 10, "maelezo yanahitajika: {q}");
            assert!(
                matches!(*level, "o-level" | "a-level" | "college"),
                "ngazi isiyojulikana: {level}"
            );
        }
    }

    /// MiniMax M3.1-flash ni model ya KUFIKIRI: kwa prompt kubwa `content` hubaki
    /// tupu na jibu hukaa kwenye `reasoning` (finish_reason: "length").
    /// Bila hii, kizazi cha mtihani kilirudisha "LLM haikujibu" kila mara.
    #[test]
    fn pick_content_hutumia_reasoning_kama_content_ni_tupu() {
        let v = json!({
            "choices": [{
                "finish_reason": "length",
                "message": { "role": "assistant", "content": "", "reasoning": "{\"title\":\"Mtihani\"}" }
            }]
        });
        assert_eq!(pick_content(&v).unwrap(), "{\"title\":\"Mtihani\"}");
    }

    #[test]
    fn pick_content_hupendelea_content_kamili() {
        let v = json!({
            "choices": [{ "message": { "content": "jibu kamili", "reasoning": "kufikiri" } }]
        });
        assert_eq!(pick_content(&v).unwrap(), "jibu kamili");
    }

    #[test]
    fn pick_content_hushughulikia_ollama_generate() {
        let v = json!({ "response": "jibu la ollama" });
        assert_eq!(pick_content(&v).unwrap(), "jibu la ollama");
    }

    #[test]
    fn pick_content_hurudisha_none_kwa_jibu_tupu() {
        let v = json!({ "choices": [{ "message": { "content": "  ", "reasoning": "" } }] });
        assert!(pick_content(&v).is_none());
    }

    #[test]
    fn seed_inashughulikia_masomo_mengi() {
        let mut subjects: Vec<&str> = SEED.iter().map(|s| s.0).collect();
        subjects.sort_unstable();
        subjects.dedup();
        assert!(
            subjects.len() >= 8,
            "masomo yanapaswa kuwa ≥8, yalikuwa {}",
            subjects.len()
        );
    }
}
