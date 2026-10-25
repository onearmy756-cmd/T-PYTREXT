//! store.rs — HIFADHIDATA YA TOTAL SECURITY (dynamic, inayojifunza)
//!
//! Hii ni msingi wa "dynamic na halisi na data fuull": badala ya orodha za
//! maneno zilizoandikwa ndani ya code (ambazo zinahitaji kurecompile), sheria
//! (rules), viashiria (IOC) na maamuzi huishi kwenye SQLite. Kwa hivyo:
//!
//!  - Admin anaweza kuongeza/kubadilisha/kuzima sheria bila recompile.
//!  - Mfumo unajifunza kutoka feedback (true/false positive) na kupunguza au
//!    kuongeza uzito wa sheria automatically.
//!  - Kila uamuzi (event) unahifadhiwa kwa audit + ripoti.
//!  - IOC (domain, IP, namba ya simu, hash) zinashirikiwa kwenye modules zote.
//!
//! Sheria za msingi (seed) hutolewa kutoka spec mara ya kwanza tu; baada ya
//! hapo DB ndiyo ukweli.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

/// Sheria moja ya uchanganuzi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: i64,
    pub platform: String,
    pub category: String,
    pub kind: String,
    pub pattern: String,
    pub severity: u8,
    pub source: String,
    pub enabled: bool,
    pub hits: i64,
    pub tp: i64,
    pub fp: i64,
    pub created_at: String,
}

impl Rule {
    /// Uzito unaobadilika kutoka feedback: kila false-positive hupunguza, kila
    /// true-positive huongeza (max 100, min 5).
    pub fn effective_severity(&self) -> u8 {
        let delta = self.tp - self.fp;
        let base = self.severity as i64 + delta.clamp(-40, 40);
        base.clamp(5, 100) as u8
    }
}

pub async fn init_tables(db: &SqlitePool) {
    for ddl in [
        r#"CREATE TABLE IF NOT EXISTS ts_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            platform TEXT NOT NULL DEFAULT '*',
            category TEXT NOT NULL DEFAULT '*',
            kind TEXT NOT NULL,
            pattern TEXT NOT NULL,
            severity INTEGER NOT NULL DEFAULT 50,
            source TEXT NOT NULL DEFAULT 'seed',
            enabled INTEGER NOT NULL DEFAULT 1,
            hits INTEGER NOT NULL DEFAULT 0,
            tp INTEGER NOT NULL DEFAULT 0,
            fp INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            UNIQUE(platform, category, kind, pattern)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ts_iocs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            value TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'admin',
            severity INTEGER NOT NULL DEFAULT 70,
            created_at TEXT NOT NULL,
            last_seen TEXT,
            UNIQUE(kind, value)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ts_events (
            id TEXT PRIMARY KEY,
            at TEXT NOT NULL,
            account TEXT,
            module INTEGER NOT NULL,
            platform TEXT,
            subject TEXT,
            decision TEXT NOT NULL,
            score INTEGER NOT NULL,
            signals TEXT NOT NULL,
            note TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ts_feedback (
            id TEXT PRIMARY KEY,
            event_id TEXT NOT NULL,
            verdict TEXT NOT NULL,
            by TEXT,
            at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ts_ioc_hits (
            ioc_id INTEGER NOT NULL,
            at TEXT NOT NULL
        )"#,
    ] {
        let _ = sqlx::query(ddl).execute(db).await;
    }
}

/// Sheria za msingi (hutolewa kutoka spec mara ya kwanza tu).
/// (platform, category, kind, pattern, severity)
pub const SEED: &[(&str, &str, &str, &str, u8)] = &[
    ("*", "content", "profanity", "pumbavu", 60),
    ("*", "content", "profanity", "mjinga", 60),
    ("*", "content", "profanity", "shenzi", 60),
    ("*", "content", "profanity", "malaya", 65),
    ("*", "content", "profanity", "fuck", 60),
    ("*", "content", "profanity", "bitch", 60),
    ("*", "content", "profanity", "asshole", 60),
    ("*", "content", "profanity", "bastard", 55),
    ("*", "content", "threat", "nitakuua", 90),
    ("*", "content", "threat", "nitakupiga", 85),
    ("*", "content", "threat", "nitakumaliza", 90),
    ("*", "content", "threat", "i will kill", 90),
    ("*", "content", "threat", "kill you", 85),
    ("*", "content", "scam", "tuma pesa", 80),
    ("*", "content", "scam", "tuma tsh", 80),
    ("*", "content", "scam", "otp", 70),
    ("*", "content", "scam", "umeshinda", 75),
    ("*", "content", "scam", "umebahatika", 75),
    ("*", "content", "scam", "click here", 60),
    ("*", "content", "scam", "verify account", 80),
    ("*", "content", "scam", "account suspended", 80),
    ("*", "content", "scam", "you have won", 75),
    ("*", "content", "scam", "lottery", 70),
    ("*", "content", "crypto_scam", "double your", 85),
    ("*", "content", "crypto_scam", "guaranteed returns", 85),
    ("*", "content", "crypto_scam", "guaranteed profit", 85),
    ("*", "content", "crypto_scam", "ponzi", 90),
    ("*", "content", "crypto_scam", "bitcoin giveaway", 85),
    ("*", "content", "crypto_scam", "faida ya uhakika", 80),
    ("*", "content", "job_scam", "pay for training", 80),
    ("*", "content", "job_scam", "registration fee", 80),
    ("*", "content", "job_scam", "processing fee", 75),
    ("*", "content", "job_scam", "refundable deposit", 75),
    ("*", "link", "bad_tld", ".zip", 40),
    ("*", "link", "bad_tld", ".top", 40),
    ("*", "link", "bad_tld", ".xyz", 40),
    ("*", "link", "bad_tld", ".tk", 45),
    ("*", "link", "bad_tld", ".gq", 45),
    ("*", "link", "bad_tld", ".cf", 45),
    ("*", "link", "bad_tld", ".ml", 45),
    ("*", "link", "shortener", "bit.ly", 30),
    ("*", "link", "shortener", "tinyurl.com", 30),
    ("*", "link", "shortener", "t.co", 25),
    ("*", "link", "shortener", "cutt.ly", 30),
    ("*", "link", "shortener", "rb.gy", 30),
    ("*", "link", "brand_impersonation", "facebook", 35),
    ("*", "link", "brand_impersonation", "whatsapp", 35),
    ("*", "link", "brand_impersonation", "instagram", 35),
    ("*", "link", "brand_impersonation", "telegram", 35),
    ("*", "link", "credential_lure", "login", 20),
    ("*", "link", "credential_lure", "verify", 20),
    ("*", "link", "credential_lure", "wallet", 25),
];

/// Weka sheria za msingi kama jedwali ni tupu.
pub async fn seed_defaults(db: &SqlitePool) -> Result<usize, String> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ts_rules")
        .fetch_one(db)
        .await
        .map_err(|e| e.to_string())?;
    if count.0 > 0 {
        return Ok(0);
    }
    let now = chrono::Local::now().to_rfc3339();
    let mut n = 0;
    for (p, c, k, pat, sev) in SEED {
        let r = sqlx::query(
            "INSERT OR IGNORE INTO ts_rules (platform, category, kind, pattern, severity, source, enabled, created_at)
             VALUES (?,?,?,?,?,'seed',1,?)",
        )
        .bind(p).bind(c).bind(k).bind(pat).bind(*sev as i64).bind(&now)
        .execute(db)
        .await;
        if r.is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

/// Sheria zinazohusika na platform + category (pamoja na '*' wildcard).
pub async fn rules_for(db: &SqlitePool, platform: &str, category: &str) -> Vec<Rule> {
    let rows: Vec<(i64, String, String, String, String, i64, String, i64, i64, i64, i64, String)> =
        sqlx::query_as(
            "SELECT id, platform, category, kind, pattern, severity, source, enabled, hits, tp, fp, created_at
             FROM ts_rules
             WHERE enabled=1 AND (platform=? OR platform='*') AND (category=? OR category='*')
             ORDER BY id",
        )
        .bind(platform)
        .bind(category)
        .fetch_all(db)
        .await
        .unwrap_or_default();
    rows.into_iter()
        .map(|(id, platform, category, kind, pattern, severity, source, enabled, hits, tp, fp, created_at)| Rule {
            id, platform, category, kind, pattern, severity: severity as u8,
            source, enabled: enabled != 0, hits, tp, fp, created_at,
        })
        .collect()
}

/// Ongeza sheria (admin/learning).
pub async fn add_rule(
    db: &SqlitePool,
    platform: &str,
    category: &str,
    kind: &str,
    pattern: &str,
    severity: u8,
    source: &str,
) -> Result<Rule, String> {
    let pat = pattern.trim().to_lowercase();
    if pat.is_empty() {
        return Err("pattern tupu".into());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO ts_rules (platform, category, kind, pattern, severity, source, enabled, created_at)
         VALUES (?,?,?,?,?,?,1,?)",
    )
    .bind(platform).bind(category).bind(kind).bind(&pat).bind(severity as i64).bind(source).bind(&now)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT id, severity, tp, fp FROM ts_rules WHERE platform=? AND category=? AND kind=? AND pattern=?",
    )
    .bind(platform).bind(category).bind(kind).bind(&pat)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(Rule {
        id: row.0, platform: platform.into(), category: category.into(), kind: kind.into(),
        pattern: pat, severity: row.1 as u8, source: source.into(), enabled: true,
        hits: 0, tp: row.2, fp: row.3, created_at: now,
    })
}

pub async fn list_rules(db: &SqlitePool, platform: Option<&str>, limit: i64) -> serde_json::Value {
    let rows: Vec<(i64, String, String, String, String, i64, String, i64, i64, i64)> = if let Some(p) = platform {
        sqlx::query_as(
            "SELECT id, platform, category, kind, pattern, severity, source, enabled, tp, fp
             FROM ts_rules WHERE platform=? OR platform='*' ORDER BY id DESC LIMIT ?2",
        ).bind(p).bind(limit.clamp(1, 5000)).fetch_all(db).await.unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT id, platform, category, kind, pattern, severity, source, enabled, tp, fp
             FROM ts_rules ORDER BY id DESC LIMIT ?1",
        ).bind(limit.clamp(1, 5000)).fetch_all(db).await.unwrap_or_default()
    };
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, p, c, k, pat, sev, src, en, tp, fp)| {
            let eff = (*sev as i64 + (tp - fp).clamp(-40, 40)).clamp(5, 100);
            json!({ "id": id, "platform": p, "category": c, "kind": k, "pattern": pat,
                    "severity": sev, "effective_severity": eff, "source": src,
                    "enabled": *en != 0, "tp": tp, "fp": fp })
        })
        .collect();
    json!({ "ok": true, "count": items.len(), "items": items })
}

pub async fn set_enabled(db: &SqlitePool, id: i64, enabled: bool) -> Result<(), String> {
    sqlx::query("UPDATE ts_rules SET enabled=? WHERE id=?")
        .bind(if enabled { 1 } else { 0 })
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_rule(db: &SqlitePool, id: i64) -> Result<bool, String> {
    let r = sqlx::query("DELETE FROM ts_rules WHERE id=?")
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(r.rows_affected() > 0)
}

// ---------------- IOC ----------------

pub async fn add_ioc(db: &SqlitePool, kind: &str, value: &str, source: &str, severity: u8) -> Result<(), String> {
    let v = value.trim().to_lowercase();
    if v.is_empty() {
        return Err("IOC tupu".into());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query(
        "INSERT OR REPLACE INTO ts_iocs (kind, value, source, severity, created_at, last_seen)
         VALUES (?,?,?,?,?, COALESCE((SELECT last_seen FROM ts_iocs WHERE kind=? AND value=?), ?))",
    )
    .bind(kind).bind(&v).bind(source).bind(severity as i64).bind(&now)
    .bind(kind).bind(&v).bind(&now)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_iocs(db: &SqlitePool, limit: i64) -> serde_json::Value {
    let rows: Vec<(i64, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, kind, value, source, severity, created_at FROM ts_iocs ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(limit.clamp(1, 5000))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, k, v, s, sev, at)| json!({ "id": id, "kind": k, "value": v, "source": s, "severity": sev, "created_at": at }))
        .collect();
    json!({ "ok": true, "count": items.len(), "items": items })
}

/// Angalia kama thamani (domain/ip/phone/hash) ipo kwenye IOC.
pub async fn ioc_lookup(db: &SqlitePool, value: &str) -> Option<(i64, String, u8)> {
    let v = value.trim().to_lowercase();
    if v.is_empty() {
        return None;
    }
    let row: Option<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, kind, severity FROM ts_iocs WHERE value=? LIMIT 1",
    )
    .bind(&v)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    if let Some((id, kind, sev)) = row {
        let now = chrono::Local::now().to_rfc3339();
        let _ = sqlx::query("INSERT INTO ts_ioc_hits (ioc_id, at) VALUES (?,?)")
            .bind(id).bind(&now).execute(db).await;
        let _ = sqlx::query("UPDATE ts_iocs SET last_seen=? WHERE id=?").bind(&now).bind(id).execute(db).await;
        Some((id, kind, sev as u8))
    } else {
        None
    }
}

// ---------------- EVENTS ----------------

pub async fn record_event(
    db: &SqlitePool,
    account: &str,
    module: u16,
    platform: &str,
    subject: &str,
    decision: &str,
    score: u32,
    signals: &serde_json::Value,
    note: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let at = chrono::Local::now().to_rfc3339();
    let sj = serde_json::to_string(signals).unwrap_or_else(|_| "[]".into());
    sqlx::query(
        "INSERT INTO ts_events (id, at, account, module, platform, subject, decision, score, signals, note)
         VALUES (?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id).bind(&at).bind(account).bind(module as i64).bind(platform).bind(subject)
    .bind(decision).bind(score as i64).bind(&sj).bind(note)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn list_events(db: &SqlitePool, limit: i64) -> serde_json::Value {
    let rows: Vec<(String, String, String, i64, String, String, String, i64)> = sqlx::query_as(
        "SELECT id, at, COALESCE(account,''), module, COALESCE(platform,''), COALESCE(subject,''), decision, score
         FROM ts_events ORDER BY at DESC LIMIT ?1",
    )
    .bind(limit.clamp(1, 5000))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, at, acc, m, p, s, d, sc)| json!({
            "id": id, "at": at, "account": acc, "module": m, "platform": p,
            "subject": s, "decision": d, "score": sc
        }))
        .collect();
    json!({ "ok": true, "count": items.len(), "items": items })
}

/// Feedback kwa event -> hurekebisha tp/fp za sheria zilizogusa.
pub async fn add_feedback(
    db: &SqlitePool,
    event_id: &str,
    verdict: &str,
    by: &str,
) -> Result<serde_json::Value, String> {
    if verdict != "true_positive" && verdict != "false_positive" {
        return Err("verdict lazima true_positive|false_positive".into());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query("INSERT INTO ts_feedback (id, event_id, verdict, by, at) VALUES (?,?,?,?,?)")
        .bind(uuid::Uuid::new_v4().to_string()).bind(event_id).bind(verdict).bind(by).bind(&now)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    // Rekebisha sheria zilizogusa event hii.
    let row: Option<(String,)> = sqlx::query_as("SELECT signals FROM ts_events WHERE id=?")
        .bind(event_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    let mut tuned = 0;
    if let Some((sj,)) = row {
        if let Ok(signals) = serde_json::from_str::<serde_json::Value>(&sj) {
            if let Some(arr) = signals.as_array() {
                for s in arr {
                    if let Some(rid) = s.get("rule_id").and_then(|v| v.as_i64()) {
                        let col = if verdict == "true_positive" { "tp" } else { "fp" };
                        let _ = sqlx::query(&format!("UPDATE ts_rules SET {col}={col}+1 WHERE id=?"))
                            .bind(rid)
                            .execute(db)
                            .await;
                        tuned += 1;
                    }
                }
            }
        }
    }
    Ok(json!({ "ok": true, "event_id": event_id, "verdict": verdict, "rules_tuned": tuned }))
}

/// Ongeza hit kwa sheria (baada ya kutumika).
pub async fn bump_hits(db: &SqlitePool, ids: &[i64]) {
    for id in ids {
        let _ = sqlx::query("UPDATE ts_rules SET hits=hits+1 WHERE id=?")
            .bind(id)
            .execute(db)
            .await;
    }
}

/// Takwimu za hifadhidata.
pub async fn stats(db: &SqlitePool) -> serde_json::Value {
    let rules: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ts_rules WHERE enabled=1").fetch_one(db).await.unwrap_or((0,));
    let iocs: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ts_iocs").fetch_one(db).await.unwrap_or((0,));
    let events: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ts_events").fetch_one(db).await.unwrap_or((0,));
    let feedback: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ts_feedback").fetch_one(db).await.unwrap_or((0,));
    let learned: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ts_rules WHERE source NOT IN ('seed')").fetch_one(db).await.unwrap_or((0,));
    json!({ "ok": true, "rules_enabled": rules.0, "iocs": iocs.0, "events": events.0,
            "feedback": feedback.0, "learned_rules": learned.0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> SqlitePool {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        db
    }

    #[tokio::test]
    async fn seed_na_rules_zinapatikana() {
        let db = pool().await;
        let n = seed_defaults(&db).await.unwrap();
        assert!(n > 40, "seed {n}");
        let r = rules_for(&db, "Facebook", "content").await;
        assert!(r.iter().any(|x| x.kind == "threat"));
        // seed haijirudii
        assert_eq!(seed_defaults(&db).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn add_rule_na_wildcard() {
        let db = pool().await;
        seed_defaults(&db).await.unwrap();
        add_rule(&db, "WhatsApp", "content", "scam", "Nipige simu", 70, "admin").await.unwrap();
        let r = rules_for(&db, "WhatsApp", "content").await;
        assert!(r.iter().any(|x| x.pattern == "nipige simu"));
        let f = rules_for(&db, "Facebook", "content").await;
        assert!(!f.iter().any(|x| x.pattern == "nipige simu"));
    }

    #[tokio::test]
    async fn feedback_hurekebisha_uzito() {
        let db = pool().await;
        seed_defaults(&db).await.unwrap();
        let rule = add_rule(&db, "*", "content", "scam", "jaribu hii", 50, "learn").await.unwrap();
        let ev = record_event(&db, "acct", 64, "Facebook", "s", "block", 50,
            &json!([{ "rule_id": rule.id, "kind": "scam" }]), "").await.unwrap();
        add_feedback(&db, &ev, "false_positive", "MBILINYI").await.unwrap();
        add_feedback(&db, &ev, "false_positive", "MBILINYI").await.unwrap();
        let rows = rules_for(&db, "Facebook", "content").await;
        let r = rows.iter().find(|x| x.pattern == "jaribu hii").unwrap();
        assert_eq!(r.fp, 2);
        assert_eq!(r.effective_severity(), 48);
    }

    #[tokio::test]
    async fn ioc_lookup_inafanya_kazi() {
        let db = pool().await;
        add_ioc(&db, "domain", "evil.example", "admin", 90).await.unwrap();
        assert!(ioc_lookup(&db, "EVIL.EXAMPLE").await.is_some());
        assert!(ioc_lookup(&db, "safi.example").await.is_none());
    }
}
