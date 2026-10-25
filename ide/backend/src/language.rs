//! language.rs — LUGHA (maelezo ya mmiliki): mfumo unatumia language husika
//! ambayo mtu AMECHAGUA wakati wa kujisajili; anaweza kubadilisha baadaye;
//! AI ndiyo inatafsiri automatically (LLM ya server kuu ya LAN — inafanya
//! kazi offline pia).
//!
//! Interfaces ya Rust + SQLite (per-user), translation kupitia ai_config::translate.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLanguage {
    pub username: String,
    pub language: String, // sw | en | fr | ar | … (code)
    pub updated_at: String,
}

pub const DEFAULT_LANG: &str = "sw";

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_language (
            username TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

/// Wakati wa kusajili: mtu anachagua lugha yake.
pub async fn set_language(db: &SqlitePool, username: &str, language: &str) -> Result<(), String> {
    let lang = language.trim().to_lowercase();
    if lang.len() != 2 || !lang.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!(
            "language code '{language}' si sahihi (mf: sw, en, fr)"
        ));
    }
    sqlx::query(
        "INSERT INTO user_language (username, language, updated_at) VALUES (?,?,?)
         ON CONFLICT(username) DO UPDATE SET language=excluded.language, updated_at=excluded.updated_at",
    )
    .bind(username.trim())
    .bind(&lang)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Lugha ya mtumiaji (default: sw — Kiswahili kwanza).
pub async fn get_language(db: &SqlitePool, username: &str) -> String {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT language FROM user_language WHERE username = ?")
            .bind(username)
            .fetch_optional(db)
            .await
            .unwrap_or(None);
    row.map(|(l,)| l).unwrap_or_else(|| DEFAULT_LANG.into())
}

/// AI auto-translate: text → lugha ya mtumiaji (LLM ya server kuu ya LAN —
/// inafanya kazi offline pia). Inarudisha (translated, lang).
pub async fn auto_translate(db: &SqlitePool, username: &str, text: &str) -> (String, String) {
    let lang = get_language(db, username).await;
    match crate::ai_config::translate_db(db, text, &lang).await {
        Some(t) if !t.is_empty() => (t, lang),
        _ => (text.to_string(), lang), // LLM haipatikani → text ya asili (hakuna uongo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn chagua_wakati_wa_kusajili_na_kubadilisha_baadaye() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // Kusajili: anachagua en
        set_language(&db, "admin", "en").await.unwrap();
        assert_eq!(get_language(&db, "admin").await, "en");
        // Baadaye anabadilisha kuwa sw
        set_language(&db, "admin", "sw").await.unwrap();
        assert_eq!(get_language(&db, "admin").await, "sw");
        // Mtu mwingine bila choice → default sw
        assert_eq!(get_language(&db, "fundi").await, "sw");
    }

    #[tokio::test]
    async fn code_mbaya_inakataliwa() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        assert!(set_language(&db, "u", "kiswahili").await.is_err()); // si code ya 2
        assert!(set_language(&db, "u", "1").await.is_err());
    }

    #[tokio::test]
    async fn auto_translate_hakuna_panic_bila_llm() {
        // Sandbox haina LLM → inarudisha text ya asili (hakuna uongo)
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        set_language(&db, "admin", "sw").await.unwrap();
        let (text, lang) = auto_translate(&db, "admin", "Hello, how are you?").await;
        assert_eq!(lang, "sw");
        assert!(!text.is_empty());
    }
}
