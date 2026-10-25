//! feedback.rs — maoni (thumbs up/down) ya jibu la AI. Hali halisi: huhifadhiwa
//! kwenye DB na hutoa muhtasari wa kura. Hakuna no-op.

use crate::AppState;
use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn init_tables(db: &sqlx::SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS feedback (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            rating INTEGER NOT NULL,
            question TEXT,
            answer TEXT,
            at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

#[derive(Debug, Deserialize)]
pub struct FeedbackReq {
    pub rating: i64,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub username: Option<String>,
}

fn clamp_rating(r: i64) -> i64 {
    if r > 0 {
        1
    } else if r < 0 {
        -1
    } else {
        0
    }
}

async fn tally(db: &sqlx::SqlitePool) -> (i64, i64) {
    sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(rating = 1), 0), COALESCE(SUM(rating = -1), 0) FROM feedback",
    )
    .fetch_one(db)
    .await
    .unwrap_or((0, 0))
}

/// POST /api/feedback — rekodi kura (-1 au 1) ya jibu la AI.
pub async fn submit(State(s): State<AppState>, Json(r): Json<FeedbackReq>) -> Json<Value> {
    let rating = clamp_rating(r.rating);
    if rating == 0 {
        return Json(json!({ "ok": false, "error": "rating lazima -1 au 1" }));
    }
    let user = r
        .username
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| "MBILINYI".into());
    let _ = sqlx::query(
        "INSERT INTO feedback (username, rating, question, answer, at) VALUES (?,?,?,?,?)",
    )
    .bind(&user)
    .bind(rating)
    .bind(r.question.as_deref().map(|q| q.chars().take(2000).collect::<String>()))
    .bind(r.answer.as_deref().map(|a| a.chars().take(8000).collect::<String>()))
    .bind(chrono::Local::now().to_rfc3339())
    .execute(&s.db)
    .await;
    let (up, down) = tally(&s.db).await;
    Json(json!({ "ok": true, "rating": rating, "up": up, "down": down }))
}

/// GET /api/feedback/summary — jumla ya kura.
pub async fn summary(State(s): State<AppState>) -> Json<Value> {
    let (up, down) = tally(&s.db).await;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedback")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    Json(json!({ "ok": true, "up": up, "down": down, "total": total }))
}
