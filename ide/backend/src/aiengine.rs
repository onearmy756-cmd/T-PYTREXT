//! aiengine.rs — ELIMUPRO Awamu 17: injini ya AI (moduli 17).
//!
//! AI ni msaidizi, si mbadala wa mwalimu. Moduli hii inaunganisha vipande vinne
//! vinavyoshiriki mstari mmoja wa ukweli:
//!   - AI Scribe: maandishi → muhtasari, dokezo, maswali.
//!   - AI Lesson Gen: notes → somo kamili (maelezo, mifano, maswali).
//!   - AI Anti-Cheat: hukagua kazi/mtihani kwa dalili za AI — HUASHIRIA (flag)
//!     kwa hoja, HAIHUKUMU. Mwalimu ndiye anayeamua.
//!   - AI Tutor: mwanafunzi huuliza swali kuhusu notes → jibu.
//!
//! Kanuni halisi:
//!   - LLM ikikosekana, hurudisha `mode="offline"` — SI kubuni jibu la uongo.
//!   - Anti-cheat hupima kwa vipimo vinavyoeleweka (urefu wa sentensi, alama za
//!     AI, maneno ya kimfumo) na kurudisha `score` + `reasons`.
//!   - Kila ombi huhifadhiwa kwa ukaguzi (`ep_ai_jobs`).
//!
//! Tables: ep_ai_jobs ep_ai_flags

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_ai_jobs (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL DEFAULT 'tutor', -- scribe|lesson|anticheat|tutor
            ref_id TEXT NOT NULL DEFAULT '',
            input TEXT NOT NULL DEFAULT '',
            output TEXT NOT NULL DEFAULT '',
            mode TEXT NOT NULL DEFAULT 'offline', -- llm|offline
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_ai_flags (
            id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'assignment', -- assignment|exam
            ref_id TEXT NOT NULL DEFAULT '',
            score INTEGER NOT NULL DEFAULT 0,
            reasons TEXT NOT NULL DEFAULT '[]',
            reviewed INTEGER NOT NULL DEFAULT 0,
            reviewer TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ai/status", get(status))
        .route("/api/ai/scribe", post(scribe))
        .route("/api/ai/lesson", post(lesson))
        .route("/api/ai/anticheat", post(anticheat))
        .route("/api/ai/tutor", post(tutor))
        .route("/api/ai/flags", get(flags_list))
        .route("/api/ai/flag/:id/review", post(flag_review))
        .route("/api/ai/jobs", get(jobs_list))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

/// Aina za kazi zinazokubalika.
pub fn norm_job(k: &str) -> &'static str {
    match k.trim().to_lowercase().as_str() {
        "scribe" => "scribe",
        "lesson" | "lesson_gen" => "lesson",
        "anticheat" | "anti-cheat" | "cheat" => "anticheat",
        _ => "tutor",
    }
}

/// Alama za kawaida za maandishi yaliyozalishwa na AI.
pub const AI_MARKERS: [&str; 12] = [
    "as an ai",
    "kama ai",
    "in conclusion",
    "kwa hitimisho",
    "furthermore",
    "moreover",
    "it is important to note",
    "delve into",
    "tapestry",
    "let's explore",
    "in summary",
    "hapa kuna",
];

/// Uchambuzi wa anti-cheat unaoeleweka.
/// Hurudisha `(score 0..100, reasons)`.
pub fn ai_suspect_score(text: &str) -> (i64, Vec<String>) {
    let t = text.trim();
    if t.is_empty() {
        return (0, vec![]);
    }
    let lower = t.to_lowercase();
    let mut score: i64 = 0;
    let mut reasons: Vec<String> = Vec::new();

    // 1) Alama za AI.
    let hits: Vec<&str> = AI_MARKERS
        .iter()
        .copied()
        .filter(|m| lower.contains(m))
        .collect();
    if !hits.is_empty() {
        score += (hits.len() as i64 * 8).min(32);
        reasons.push(format!("alama za AI: {}", hits.join(", ")));
    }

    // 2) Sentensi ndefu kupita kiasi (mitindo ya AI).
    let sentences: Vec<&str> = t
        .split(|c| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if !sentences.is_empty() {
        let long = sentences.iter().filter(|s| s.split_whitespace().count() > 35).count();
        let ratio = long as f64 / sentences.len() as f64;
        if ratio > 0.5 {
            score += 20;
            reasons.push(format!("{long}/{} sentensi ndefu kupita kiasi", sentences.len()));
        }
        // 3) Sentensi zote zina urefu unaofanana sana (variance ndogo).
        let lens: Vec<f64> = sentences
            .iter()
            .map(|s| s.split_whitespace().count() as f64)
            .collect();
        if lens.len() >= 5 {
            let mean = lens.iter().sum::<f64>() / lens.len() as f64;
            if mean > 0.0 {
                let var = lens.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lens.len() as f64;
                let cv = var.sqrt() / mean;
                if cv < 0.18 {
                    score += 15;
                    reasons.push("urefu wa sentensi unafanana sana".into());
                }
            }
        }
    }

    // 4) Hakuna alama za kibinafsi / mifano ya kawaida ya mwanadamu.
    if t.len() > 200 && !lower.contains("mfano") && !lower.contains("kwa mfano") {
        score += 8;
        reasons.push("hakuna mifano ya kibinafsi".into());
    }

    (score.min(100), reasons)
}

/// Kiwango cha hatari kutokana na alama.
pub fn risk_level(score: i64) -> &'static str {
    match score {
        s if s >= 60 => "high",
        s if s >= 30 => "medium",
        s if s > 0 => "low",
        _ => "none",
    }
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (llm,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_ai_jobs WHERE mode='llm'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (off,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_ai_jobs WHERE mode='offline'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (unrev,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_ai_flags WHERE reviewed=0")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "jobs": table_count(&s.db, "ep_ai_jobs").await,
        "jobs_llm": llm,
        "jobs_offline": off,
        "flags": table_count(&s.db, "ep_ai_flags").await,
        "flags_unreviewed": unrev,
        "capabilities": {
            "scribe": true, "lesson_gen": true, "anticheat": true, "tutor": true
        }
    }))
}

async fn save_job(db: &SqlitePool, kind: &str, ref_id: &str, input: &str, output: &str, mode: &str) {
    let _ = sqlx::query(
        "INSERT INTO ep_ai_jobs (id,kind,ref_id,input,output,mode,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(uid())
    .bind(kind)
    .bind(ref_id)
    .bind(input)
    .bind(output)
    .bind(mode)
    .bind(now())
    .execute(db)
    .await;
}

#[derive(Deserialize)]
pub struct ScribeIn {
    pub transcript: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub subject: String,
}

pub async fn scribe(State(s): State<AppState>, Json(b): Json<ScribeIn>) -> Json<Value> {
    let text = b.transcript.trim();
    if text.is_empty() {
        return Json(json!({ "ok": false, "error": "transcript inahitajika" }));
    }
    let lang = if b.language.trim().is_empty() { "Kiswahili" } else { b.language.trim() };
    let prompt = format!(
        "Wewe ni AI Scribe wa chuo. Lugha: {lang}. Somo: {}.\n\
         MAANDISHI:\n{text}\n\n\
         Rudisha JSON PEKEE: {{\"muhtasari\":\"...\",\"dokezo\":[\"...\"],\"maswali\":[\"...\"]}}",
        b.subject
    );
    let out = crate::education::llm_long(&s.db, &prompt, 90).await;
    let (mode, payload) = match &out {
        Some(t) => ("llm", t.clone()),
        None => (
            "offline",
            json!({
                "muhtasari": "LLM haipatikani — maandishi yamehifadhiwa kwa ukaguzi wa mwalimu.",
                "dokezo": [],
                "maswali": []
            })
            .to_string(),
        ),
    };
    save_job(&s.db, "scribe", "", text, &payload, mode).await;
    Json(json!({ "ok": true, "mode": mode, "result": payload }))
}

#[derive(Deserialize)]
pub struct LessonIn {
    pub notes: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub level: String,
}

pub async fn lesson(State(s): State<AppState>, Json(b): Json<LessonIn>) -> Json<Value> {
    let notes = b.notes.trim();
    if notes.is_empty() {
        return Json(json!({ "ok": false, "error": "notes zinahitajika" }));
    }
    let prompt = format!(
        "Wewe ni mwalimu msaidizi. Somo: {}. Kiwango: {}.\n\
         NOTES:\n{notes}\n\n\
         Tengeneza somo kamili. Rudisha JSON PEKEE: \
         {{\"kichwa\":\"...\",\"malengo\":[\"...\"],\"maelezo\":[\"...\"],\"mifano\":[\"...\"],\"maswali\":[\"...\"]}}",
        b.subject, b.level
    );
    let out = crate::education::llm_long(&s.db, &prompt, 120).await;
    let (mode, payload) = match &out {
        Some(t) => ("llm", t.clone()),
        None => (
            "offline",
            json!({
                "kichwa": "Somo (LLM haipatikani)",
                "malengo": [], "maelezo": [], "mifano": [], "maswali": []
            })
            .to_string(),
        ),
    };
    save_job(&s.db, "lesson", "", notes, &payload, mode).await;
    Json(json!({ "ok": true, "mode": mode, "result": payload }))
}

#[derive(Deserialize)]
pub struct CheatIn {
    pub text: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub ref_id: String,
    #[serde(default)]
    pub subject_id: String,
}

pub async fn anticheat(State(s): State<AppState>, Json(b): Json<CheatIn>) -> Json<Value> {
    let text = b.text.trim();
    if text.is_empty() {
        return Json(json!({ "ok": false, "error": "maandishi yanahitajika" }));
    }
    let (score, reasons) = ai_suspect_score(text);
    let kind = if b.kind.trim().eq_ignore_ascii_case("exam") { "exam" } else { "assignment" };
    let level = risk_level(score);
    let reasons_json = serde_json::to_string(&reasons).unwrap_or_else(|_| "[]".into());
    save_job(&s.db, "anticheat", &b.ref_id, text, &reasons_json, "local").await;
    // Huashiria tu kama kuna dalili.
    let id = if score > 0 {
        let id = uid();
        let _ = sqlx::query(
            "INSERT INTO ep_ai_flags (id,subject_id,kind,ref_id,score,reasons,reviewed,reviewer,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,0,'',?7)",
        )
        .bind(&id)
        .bind(&b.subject_id)
        .bind(kind)
        .bind(&b.ref_id)
        .bind(score)
        .bind(&reasons_json)
        .bind(now())
        .execute(&s.db)
        .await;
        id
    } else {
        String::new()
    };
    Json(json!({
        "ok": true,
        "score": score,
        "risk": level,
        "reasons": reasons,
        "flagged": score > 0,
        "flag_id": id,
        "note": "Hii ni hoja ya uchunguzi — mwalimu ndiye anayeamua."
    }))
}

#[derive(Deserialize)]
pub struct TutorIn {
    pub question: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub subject: String,
}

pub async fn tutor(State(s): State<AppState>, Json(b): Json<TutorIn>) -> Json<Value> {
    let q = b.question.trim();
    if q.is_empty() {
        return Json(json!({ "ok": false, "error": "swali linahitajika" }));
    }
    let ctx = if b.notes.trim().is_empty() {
        String::new()
    } else {
        format!("\nNOTES:\n{}\n", b.notes.trim())
    };
    let prompt = format!(
        "Wewe ni AI Tutor wa chuo. Somo: {}. Jibu kwa Kiswahili, kifupi na kwa mifano.{ctx}\n\
         SWALI: {q}\n\nRudisha JSON PEKEE: {{\"jibu\":\"...\",\"mifano\":[\"...\"],\"marejeo\":[\"...\"]}}",
        b.subject
    );
    let out = crate::education::llm_long(&s.db, &prompt, 60).await;
    let (mode, payload) = match &out {
        Some(t) => ("llm", t.clone()),
        None => (
            "offline",
            json!({
                "jibu": "LLM haipatikani kwa sasa — swali lako limehifadhiwa; mwalimu atajibu.",
                "mifano": [], "marejeo": []
            })
            .to_string(),
        ),
    };
    save_job(&s.db, "tutor", "", q, &payload, mode).await;
    Json(json!({ "ok": true, "mode": mode, "result": payload }))
}

#[derive(Deserialize)]
pub struct FlagQuery {
    #[serde(default)]
    pub reviewed: String,
    #[serde(default)]
    pub kind: String,
}

pub async fn flags_list(State(s): State<AppState>, Query(q): Query<FlagQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, i64, String, String)>(
        "SELECT id,subject_id,kind,ref_id,score,reasons,reviewed,reviewer,created_at FROM ep_ai_flags
         WHERE (?1='' OR reviewed=?1) AND (?2='' OR kind=?2)
         ORDER BY created_at DESC LIMIT 300",
    )
    .bind(&q.reviewed)
    .bind(&q.kind)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "flags": rows.into_iter().map(|r| {
            let reasons: Value = serde_json::from_str(&r.5).unwrap_or(json!([]));
            json!({
                "id": r.0, "subject_id": r.1, "kind": r.2, "ref_id": r.3, "score": r.4,
                "reasons": reasons, "risk": risk_level(r.4), "reviewed": r.6 == 1,
                "reviewer": r.7, "created_at": r.8
            })
        }).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct ReviewIn {
    #[serde(default)]
    pub reviewer: String,
}

pub async fn flag_review(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<ReviewIn>) -> Json<Value> {
    let r = sqlx::query("UPDATE ep_ai_flags SET reviewed=1,reviewer=?1 WHERE id=?2")
        .bind(&b.reviewer)
        .bind(&id)
        .execute(&s.db)
        .await;
    match r {
        Ok(x) if x.rows_affected() > 0 => Json(json!({ "ok": true, "reviewed": true })),
        _ => Json(json!({ "ok": false, "error": "flag haipo" })),
    }
}

#[derive(Deserialize)]
pub struct JobQuery {
    #[serde(default)]
    pub kind: String,
}

pub async fn jobs_list(State(s): State<AppState>, Query(q): Query<JobQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id,kind,ref_id,output,mode,created_at FROM ep_ai_jobs
         WHERE (?1='' OR kind=?1) ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.kind)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "jobs": rows.into_iter().map(|r| json!({
            "id": r.0, "kind": r.1, "ref_id": r.2, "output": r.3, "mode": r.4, "created_at": r.5
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maandishi_ya_kawaida_hayashiriiwi() {
        let (score, reasons) = ai_suspect_score("Nimefanya kazi hii mwenyewe. Nilipata shida kidogo.");
        assert_eq!(score, 0, "maandishi ya kawaida yalishiriwa: {reasons:?}");
        assert!(reasons.is_empty());
    }

    #[test]
    fn maandishi_ya_ai_hushiriwa() {
        let t = "As an AI language model, I can explain. In conclusion, furthermore, moreover, \
                 it is important to note that we must delve into this tapestry of ideas. \
                 Let's explore the nuances of the subject matter in great detail and with much care. \
                 In summary, this is a comprehensive analysis of the topic at hand.";
        let (score, reasons) = ai_suspect_score(t);
        assert!(score >= 30, "score ilikuwa {score}, reasons {reasons:?}");
        assert!(!reasons.is_empty());
    }

    #[test]
    fn maandishi_tupu_hurudisha_sifuri() {
        let (score, reasons) = ai_suspect_score("   ");
        assert_eq!(score, 0);
        assert!(reasons.is_empty());
    }

    #[test]
    fn kiwango_cha_hatari_hupangwa() {
        assert_eq!(risk_level(0), "none");
        assert_eq!(risk_level(1), "low");
        assert_eq!(risk_level(29), "low");
        assert_eq!(risk_level(30), "medium");
        assert_eq!(risk_level(59), "medium");
        assert_eq!(risk_level(60), "high");
        assert_eq!(risk_level(100), "high");
    }

    #[test]
    fn aina_za_kazi_hupangwa() {
        assert_eq!(norm_job("scribe"), "scribe");
        assert_eq!(norm_job("lesson_gen"), "lesson");
        assert_eq!(norm_job("anti-cheat"), "anticheat");
        assert_eq!(norm_job("cheat"), "anticheat");
        assert_eq!(norm_job("tutor"), "tutor");
        assert_eq!(norm_job(""), "tutor");
    }

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn flag_huhifadhiwa_na_kuhakikiwa() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_ai_flags (id,subject_id,kind,ref_id,score,reasons,reviewed,reviewer,created_at) VALUES ('f1','sub1','assignment','a1',45,'[\"alama za AI\"]',0,'',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let (rev,): (i64,) = sqlx::query_as("SELECT reviewed FROM ep_ai_flags WHERE id='f1'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(rev, 0);
        sqlx::query("UPDATE ep_ai_flags SET reviewed=1,reviewer='mwalimu' WHERE id='f1'")
            .execute(&p).await.unwrap();
        let (rev2, who): (i64, String) =
            sqlx::query_as("SELECT reviewed,reviewer FROM ep_ai_flags WHERE id='f1'")
                .fetch_one(&p).await.unwrap();
        assert_eq!(rev2, 1);
        assert_eq!(who, "mwalimu");
    }

    #[tokio::test]
    async fn kazi_huhifadhiwa_kwa_mode_sahihi() {
        let p = db().await;
        save_job(&p, "tutor", "", "swali", "jibu", "offline").await;
        save_job(&p, "scribe", "", "maandishi", "muhtasari", "llm").await;
        let (off,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_ai_jobs WHERE mode='offline'")
            .fetch_one(&p).await.unwrap();
        let (llm,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_ai_jobs WHERE mode='llm'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(off, 1);
        assert_eq!(llm, 1);
    }
}
