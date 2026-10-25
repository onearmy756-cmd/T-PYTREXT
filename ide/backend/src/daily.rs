//! daily.rs — KAZI ZA KILA SIKU (SEHEMU 3 Hatua 6): scan ya matatizo kiotomatiki
//! → ripoti kwa admin → SUBIRI RUHUSA → solve kwa wakati mmoja.
//!
//! Mahitaji ya mmiliki:
//!   "Kila siku mfumo ukague matatizo yote offline. Kutoa ripoti kwenye server kuu.
//!    Kumpa admin ripoti — jina la kompyuta, tatizo. Mfumo usubiri ruhusa ya mtu.
//!    Akiruhusu, unaanza kutatua matatizo yote kwa wakati mmoja... ukimaliza kila
//!    kompyuta, unaandika ripoti na kumuonyesha admin."
//!
//! KANUNI: scan ni low-risk (TCP/probe halisi) → inaendelea automatic. Solve
//! kwa mabadiliko = High-risk → HITL daima (bounded autonomy).

use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize)]
pub struct DailyFinding {
    pub pc: String, // display_name (hr, hr 1…)
    pub problem: String,
    pub severity: String, // info | warning | critical
    pub ts: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyReport {
    pub date: String,
    pub findings: Vec<DailyFinding>,
    pub status: String, // awaiting_permission | solving | done
    pub summary_sw: String,
}

// ---------- RIPOTI (SQLite — server kuu) ----------

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS daily_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            pc TEXT NOT NULL,
            problem TEXT NOT NULL,
            severity TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

/// KUJIFUNZA KILA SIKU: kila finding mpya inaandikwa kwenye Neuralis Brain
/// (kumbukumbu ya pamoja) — kesho agents wanajua tatizo hili na suluhisho lake.
async fn learn_from_finding(brain: &crate::brain::Brain, pc: &str, problem: &str, severity: &str) {
    let solution = match severity {
        "critical" => "Tatizo la kasi: endesha uchunguzi wa afya (HUDUMA: Uchunguzi wa Afya), safisha disk na anzisha upya huduma husika kwa ruhusa ya admin (HITL).",
        "warning" => "Kifaa hakijibu: hakikisha nguvu/mtandao, kisha endesha HUDUMA ya Uchunguzi wa Afya; kama kiko hai, firewall inaweza kuziba huduma za kawaida.",
        _ => "Fuata hali ya kifaa kwenye REAL REMOTING.",
    };
    let _ = crate::brain::remember(
        brain,
        "daily-scanner",
        pc,
        problem,
        solution,
        match severity {
            "critical" => 0.8,
            "warning" => 0.7,
            _ => 0.5,
        },
    )
    .await;
}

/// Andika findings za scan ya leo (jina la PC + tatizo) — ripoti kwa admin.
pub async fn record_findings(
    db: &SqlitePool,
    brain: &crate::brain::Brain,
    findings: &[DailyFinding],
) -> usize {
    let mut n = 0;
    for f in findings {
        // KUJIFUNZA KILA SIKU — finding yote inaandikwa kwenye kumbukumbu ya pamoja
        learn_from_finding(brain, &f.pc, &f.problem, &f.severity).await;
        let _ = sqlx::query(
            "INSERT INTO daily_reports (date, pc, problem, severity, status, created_at) VALUES (?,?,?,?,?,?)",
        )
        .bind(&f.ts[..10.min(f.ts.len())])
        .bind(&f.pc)
        .bind(&f.problem)
        .bind(&f.severity)
        .bind("awaiting_permission")
        .bind(chrono::Local::now().to_rfc3339())
        .execute(db)
        .await;
        n += 1;
    }
    n
}

/// Ripoti za siku (admin anaona: jina la PC + tatizo).
pub async fn today_report(db: &SqlitePool) -> Vec<DailyFinding> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT pc, problem, severity, created_at FROM daily_reports WHERE date = ? ORDER BY created_at DESC",
    )
    .bind(&today)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    rows.into_iter()
        .map(|(pc, problem, severity, ts)| DailyFinding {
            pc,
            problem,
            severity,
            ts,
        })
        .collect()
}

/// HITL: admin anaruhusu → status ya findings za leo inabadilika → solving inaanza.
pub async fn grant_permission(db: &SqlitePool) -> u64 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    sqlx::query(
        "UPDATE daily_reports SET status='solving' WHERE date=? AND status='awaiting_permission'",
    )
    .bind(&today)
    .execute(db)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0)
}

/// Baada ya solve: andika ripoti ya mwisho (admin anaona kila PC).
pub async fn mark_solved(db: &SqlitePool) -> u64 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    sqlx::query("UPDATE daily_reports SET status='solved' WHERE date=? AND status='solving'")
        .bind(&today)
        .execute(db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0)
}

// ---------- SCAN HALISI (low-risk) ----------

/// Scan ya PC moja (TCP probe halisi — ports za huduma + liveness).
/// Inarudisha findings za PC husika (hakuna uongo — "haijibu" ni finding halisi).
pub async fn scan_pc(name: &str, ip: std::net::Ipv4Addr, timeout_ms: u64) -> Vec<DailyFinding> {
    let ts = chrono::Local::now().to_rfc3339();
    let mut findings = Vec::new();
    let mut any_open = false;
    for (port, svc) in [(22u16, "SSH"), (445, "SMB"), (135, "RPC"), (3389, "RDP")] {
        let fut = tokio::net::TcpStream::connect((std::net::IpAddr::V4(ip), port));
        let open = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), fut)
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false);
        if open {
            any_open = true;
        }
    }
    if !any_open {
        findings.push(DailyFinding {
            pc: name.into(),
            problem: "PC haijibu kwenye mtandao (hakuna service wazi) — inaweza kuwa imewaka au firewall inaziba".into(),
            severity: "warning".into(),
            ts,
        });
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(pc: &str, problem: &str, severity: &str) -> DailyFinding {
        DailyFinding {
            pc: pc.into(),
            problem: problem.into(),
            severity: severity.into(),
            ts: chrono::Local::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn ripoti_kamilika_ya_siku() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // Scan inagundua matatizo → ripoti kwa admin
        let dir = std::env::temp_dir().join(format!("mtech-dtest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db.clone(), dir.to_str().unwrap());
        record_findings(
            &db,
            &brain,
            &[
                f("hr", "Disk 96% imejaa", "critical"),
                f("hr 1", "Wi-Fi haifanyi kazi", "warning"),
            ],
        )
        .await;
        let rep = today_report(&db).await;
        assert_eq!(rep.len(), 2);
        // DESC — ripoti mbili zote zipo (mpangilio: ya mwisho juu)
        assert!(rep.iter().any(|f| f.pc == "hr") && rep.iter().any(|f| f.pc == "hr 1"));
        assert!(rep.iter().any(|f| f.problem.contains("96%")));
        // Bila ruhusa: status awaiting_permission
        // Admin anaruhusu → solving → solved (ripoti ya mwisho)
        assert_eq!(grant_permission(&db).await, 2);
        assert_eq!(mark_solved(&db).await, 2);
    }

    #[tokio::test]
    async fn scan_pc_loopback_hakuna_panic() {
        let f = scan_pc("test-pc", std::net::Ipv4Addr::LOCALHOST, 20).await;
        let _ = f; // lengo: hakuna panic kwenye probe halisi
    }
}
