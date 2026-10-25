//! analytics.rs — ENGINE YA TAKWIMU KWA RUST (mbadala wa "Engine ya R" ya MTECH OS)
//!
//! Doc ya MTECH OS ilieleza engine ya R kwa uchanganuzi; hapa tunatumia Rust
//! moja kwa moja (Rust-only requirement): hesabu HALISI kutoka jobs, daily_reports
//! na fleet_agents — mean/rate/trend bila lugha nyingine au service ya nje.

use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Muhtasari kamili wa takwimu: jobs, matatizo, trend ya siku 7, hali ya agents.
pub async fn summary(db: &SqlitePool) -> Value {
    // Jobs: jumla, zilizokamilika, zilizoshindwa, wastani wa maendeleo
    let (total, done, failed, avg_progress): (i64, i64, i64, f64) = sqlx::query_as(
        r#"SELECT COUNT(*),
                  COALESCE(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END),0),
                  COALESCE(SUM(CASE WHEN status='failed' THEN 1 ELSE 0 END),0),
                  COALESCE(AVG(progress),0.0)
           FROM jobs"#,
    )
    .fetch_one(db)
    .await
    .unwrap_or((0, 0, 0, 0.0));
    let success_rate = if total > 0 {
        (done as f64) * 100.0 / (total as f64)
    } else {
        0.0
    };

    // Matatizo: kwa severity + matatizo 5 bora
    let sev_rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT severity, COUNT(*) FROM daily_reports GROUP BY severity")
            .fetch_all(db)
            .await
            .unwrap_or_default();
    let top_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT problem, COUNT(*) FROM daily_reports GROUP BY problem ORDER BY COUNT(*) DESC LIMIT 5",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Trend ya siku 7 (kwa tarikhito — siku za mwisho 7)
    let trend: Vec<(String, i64)> = sqlx::query_as(
        "SELECT date, COUNT(*) FROM daily_reports GROUP BY date ORDER BY date DESC LIMIT 7",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut trend: Vec<Value> = trend
        .into_iter()
        .map(|(date, count)| json!({ "date": date, "count": count }))
        .collect();
    trend.reverse(); // mpangilio wa kawaida (siku za zamani → mpya)

    // Agents: jumla + hai (last_seen ndani ya dakika 3 kama fleet::OFFLINE_AFTER_MS)
    let now_ms = chrono::Utc::now().timestamp_millis();
    let (agents, online): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN last_seen_ms > ? THEN 1 ELSE 0 END),0) FROM fleet_agents",
    )
    .bind(now_ms - 3 * 60 * 1000)
    .fetch_one(db)
    .await
    .unwrap_or((0, 0));

    json!({
        "ok": true,
        "engine": "rust",
        "jobs": {
            "total": total,
            "done": done,
            "failed": failed,
            "success_rate": (success_rate * 10.0).round() / 10.0,
            "avg_progress": (avg_progress * 10.0).round() / 10.0
        },
        "problems_by_severity": sev_rows.iter().map(|(s, c)| json!({"severity": s, "count": c})).collect::<Vec<_>>(),
        "top_problems": top_rows.iter().map(|(p, c)| json!({"problem": p, "count": c})).collect::<Vec<_>>(),
        "trend_7d": trend,
        "fleet": { "agents": agents, "online": online, "offline": agents - online },
        "note_sw": "Takwimu zinahesabiwa moja kwa moja kwenye Rust (engine ya ndani) — hakuna R wala service ya nje."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn summary_hesabu_kutoka_kwenye_db_tupu() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::daily::init_tables(&db).await;
        crate::fleet::init_tables(&db).await;
        let v = summary(&db).await;
        assert_eq!(v["ok"], true);
        assert_eq!(v["engine"], "rust");
        assert_eq!(v["jobs"]["total"], 0);
        assert_eq!(v["jobs"]["success_rate"], 0.0);
        assert_eq!(v["fleet"]["agents"], 0);
    }

    #[tokio::test]
    async fn summary_hesabu_jobs_halisi() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::daily::init_tables(&db).await;
        crate::fleet::init_tables(&db).await;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY, device_mac TEXT, device_name TEXT, os_type TEXT,
                os_reason TEXT, status TEXT, stage TEXT, progress INTEGER,
                message TEXT, backup_info TEXT)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO jobs (id, device_mac, device_name, os_type, os_reason, status, stage, progress, message, backup_info)
             VALUES ('j1', 'aa:bb', 'hr', 'ubuntu', 'test', 'done', 'done', 100, '', ''),
                    ('j2', 'cc:dd', 'acct', 'win11', 'test', 'failed', 'install', 40, '', '')",
        )
        .execute(&db)
        .await
        .unwrap();
        let v = summary(&db).await;
        assert_eq!(v["jobs"]["total"], 2);
        assert_eq!(v["jobs"]["done"], 1);
        assert_eq!(v["jobs"]["failed"], 1);
        assert_eq!(v["jobs"]["success_rate"], 50.0);
    }
}
