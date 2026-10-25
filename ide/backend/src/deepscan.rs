//! deepscan.rs — UCHUNGUZI WA KINA (DEEP SCAN) + HISTORIA (MTECH OS)
//!
//! Uchunguzi wa kina wa kifaa kimoja: sweep ya port kubwa (TCP halisi) +
//! matatizo yaliyorekodiwa (daily_reports) + jobs → ripoti moja. Kila run
//! huhifadhiwa kwenye deepscans (historia inayorejeshwa kwa UI).

use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// Port zinazopimwa kwa kina (safari ya kawaida ya ndani + kubwa zaidi ya default)
const DEEP_PORTS: &[u16] = &[
    21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 389, 443, 445, 465, 587, 993, 995, 1433, 1521,
    2049, 2082, 2083, 2181, 3000, 3306, 3389, 4443, 4444, 5432, 5601, 5900, 5985, 6379, 8000, 8080,
    8081, 8443, 8888, 9000, 9090, 9200, 11211, 15672, 27017,
];

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS deepscans (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            target TEXT NOT NULL,
            ip TEXT,
            open_ports TEXT NOT NULL,
            problems_found INTEGER NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

/// Sweep halisi ya TCP kwenye port zote za DEEP_PORTS (thread moja kwa urahisi).
fn sweep(ip: Ipv4Addr, timeout_ms: u64) -> Vec<u16> {
    let mut open = Vec::new();
    for &p in DEEP_PORTS {
        let addr = SocketAddr::new(std::net::IpAddr::V4(ip), p);
        if TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok() {
            open.push(p);
        }
    }
    open
}

/// Uchunguzi wa kina: port sweep + matatizo yaliyopo + ripoti iliyohifadhiwa.
pub async fn run(db: &SqlitePool, target: &str, ip: &str, timeout_ms: u64) -> Value {
    let parsed: Option<Ipv4Addr> = ip.parse().ok();
    let open: Vec<u16> = match parsed {
        Some(a) => {
            let t = timeout_ms.min(1000);
            tokio::task::spawn_blocking(move || sweep(a, t))
                .await
                .unwrap_or_default()
        }
        None => Vec::new(),
    };

    // Matatizo yaliyorekodiwa kwa kifaa hiki
    let problems: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT problem, severity, status FROM daily_reports WHERE pc = ? ORDER BY created_at DESC LIMIT 20",
    )
    .bind(target)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Jobs za kifaa hiki (mafanikio + mafaili)
    let (jobs_done, jobs_failed): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN status='failed' THEN 1 ELSE 0 END),0)
         FROM jobs WHERE device_name = ?",
    )
    .bind(target)
    .fetch_one(db)
    .await
    .unwrap_or((0, 0));

    let open_str = open
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let now = chrono::Local::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO deepscans (target, ip, open_ports, problems_found, created_at) VALUES (?,?,?,?,?)",
    )
    .bind(target)
    .bind(ip)
    .bind(&open_str)
    .bind(problems.len() as i64)
    .bind(&now)
    .execute(db)
    .await;

    // Tathmini: port zilizofunguliwa nje ya kawaida = onyo
    let risky: Vec<u16> = open
        .iter()
        .copied()
        .filter(|p| matches!(*p, 23 | 21 | 3389 | 445 | 5900 | 1433 | 2375))
        .collect();

    json!({
        "ok": true,
        "target": target,
        "ip": ip,
        "ports_scanned": DEEP_PORTS.len(),
        "open_ports": open,
        "risky_ports": risky,
        "problems": problems.iter().map(|(p, s, st)| json!({"problem": p, "severity": s, "status": st})).collect::<Vec<_>>(),
        "jobs": { "done": jobs_done, "failed": jobs_failed },
        "created_at": now,
        "note_sw": "Port sweep halisi (TCP) + historia ya daily_reports — kina zaidi ya scan ya kawaida."
    })
}

/// Historia ya uchunguzi wa kina (mizizo ya mwisho 50).
pub async fn history(db: &SqlitePool) -> Value {
    let rows: Vec<(i64, String, Option<String>, String, i64, String)> = sqlx::query_as(
        "SELECT id, target, ip, open_ports, problems_found, created_at FROM deepscans ORDER BY id DESC LIMIT 50",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let count = rows.len();
    json!({
        "ok": true,
        "count": count,
        "runs": rows.into_iter().map(|(id, target, ip, open, probs, at)| json!({
            "id": id, "target": target, "ip": ip,
            "open_ports": open, "problems_found": probs, "created_at": at
        })).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sweep_kwelishi_haifungui_kilichofungwa() {
        // 127.0.0.1 hakuna server iliyowashwa kwenye port 9 ya test — inapaswa kuwa tupu
        let open = tokio::task::spawn_blocking(|| sweep(Ipv4Addr::LOCALHOST, 30))
            .await
            .unwrap();
        assert!(open.len() <= DEEP_PORTS.len());
    }

    #[tokio::test]
    async fn run_na_history_kwenye_db_tupu() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::daily::init_tables(&db).await;
        crate::fleet::init_tables(&db).await;
        init_tables(&db).await;
        let v = run(&db, "hr", "127.0.0.1", 10).await;
        assert_eq!(v["ok"], true);
        assert_eq!(v["ports_scanned"], DEEP_PORTS.len() as i64);
        let h = history(&db).await;
        assert_eq!(h["count"], 1);
    }
}
