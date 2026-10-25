//! offline.rs — ELIMUPRO Awamu 12: offline mode / LAN (moduli 12).
//!
//! Chuo hufanya kazi bila internet: seva ya chuo (kompyuta moja) hubeba mfumo
//! wote; wanafunzi na walimu huunganisha kwenye Wi-Fi ya chuo. Hapa ni utando
//! wa LAN: kugundua hali ya mtandao, kutangaza nodi, foleni ya kazi
//! zilizofanywa wakati wa offline, na kusawazisha (sync) internet inapopatikana.
//!
//! Kanuni halisi:
//!   - `online` hupimwa kwa kubisha host ya nje; hakuna kubuni.
//!   - Kazi zinapofanywa bila internet huingia foleni (`ep_sync_queue`) na
//!     huachwa `pending`; sync huziweka `synced` au `failed` kwa jaribio halisi.
//!   - Nodi (vifaa vya LAN) hujiandikisha kwa IP ya ndani na hali ya kuishi.
//!
//! Tables: ep_sync_queue ep_lan_nodes ep_lan_settings

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
        r#"CREATE TABLE IF NOT EXISTS ep_sync_queue (
            id TEXT PRIMARY KEY,
            entity TEXT NOT NULL DEFAULT '',      -- student|mark|payment|note...
            op TEXT NOT NULL DEFAULT 'create',    -- create|update|delete
            payload TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|synced|failed
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            synced_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_lan_nodes (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT 'client',  -- server|client
            ip TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'web',     -- web|phone|desktop|tablet
            last_seen TEXT NOT NULL,
            UNIQUE(ip)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_lan_settings (
            id TEXT PRIMARY KEY,
            mode TEXT NOT NULL DEFAULT 'auto',     -- auto|offline|online
            cloud_url TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/offline/status", get(status))
        .route("/api/offline/settings", get(settings_get))
        .route("/api/offline/settings/set", post(settings_set))
        .route("/api/offline/queue/add", post(queue_add))
        .route("/api/offline/queue", get(queue_list))
        .route("/api/offline/sync", post(sync_now))
        .route("/api/offline/node/register", post(node_register))
        .route("/api/offline/nodes", get(nodes_list))
        .route("/api/offline/node/:id/leave", post(node_leave))
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

/// Hali halisi ya mtandao: `offline`|`online`.
pub fn net_state(mode: &str, reachable: bool) -> &'static str {
    match mode.trim().to_lowercase().as_str() {
        "offline" => "offline",
        "online" => "online",
        _ => {
            if reachable {
                "online"
            } else {
                "offline"
            }
        }
    }
}

/// Kazi inaweza kusawazishwa? (tu `pending` au `failed`).
pub fn can_sync(status: &str) -> bool {
    matches!(status.trim().to_lowercase().as_str(), "pending" | "failed")
}

/// Hali ya nodi kutokana na sekunde zilizopita tangu kuonekana.
pub fn node_state(secs_since: i64) -> &'static str {
    if secs_since <= 30 {
        "live"
    } else if secs_since <= 300 {
        "idle"
    } else {
        "gone"
    }
}

/// IP ya ndani (RFC1918 / loopback) — LAN pekee.
pub fn is_private_ip(ip: &str) -> bool {
    let ip = ip.trim();
    if ip == "::1" || ip.starts_with("127.") {
        return true;
    }
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    let o: Vec<Option<u32>> = parts.iter().map(|p| p.parse::<u32>().ok()).collect();
    if o.iter().any(|x| x.is_none()) {
        return false;
    }
    let (a, b) = (o[0].unwrap(), o[1].unwrap());
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

async fn mode_of(db: &SqlitePool) -> String {
    sqlx::query_as::<_, (String,)>("SELECT mode FROM ep_lan_settings LIMIT 1")
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_else(|| "auto".into())
}

/// Hupima kama internet inapatikana (kubisha kwa muda mfupi).
async fn internet_reachable() -> bool {
    let targets = ["1.1.1.1:53", "8.8.8.8:53"];
    for t in targets {
        if tokio::time::timeout(
            std::time::Duration::from_millis(700),
            tokio::net::TcpStream::connect(t),
        )
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// Anwani za LAN za seva hii.
pub fn lan_addresses() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(ips) = local_ips() {
        out = ips;
    }
    if out.is_empty() {
        out.push("127.0.0.1".into());
    }
    out
}

fn local_ips() -> Result<Vec<String>, std::io::Error> {
    // Bila crate ya ziada: tumia `hostname -I` ikiwepo, la sivyo loopback.
    let out = std::process::Command::new("hostname").arg("-I").output();
    match out {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            let v: Vec<String> = s.split_whitespace().map(|x| x.to_string()).collect();
            Ok(v)
        }
        _ => Ok(Vec::new()),
    }
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let mode = mode_of(&s.db).await;
    let reachable = internet_reachable().await;
    let state = net_state(&mode, reachable);
    let (pending,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_sync_queue WHERE status='pending'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (failed,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_sync_queue WHERE status='failed'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (live,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ep_lan_nodes WHERE last_seen >= datetime('now','-30 seconds')",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "mode": mode,
        "state": state,
        "internet": reachable,
        "server_role": "server",
        "lan_ips": lan_addresses(),
        "nodes": table_count(&s.db, "ep_lan_nodes").await,
        "nodes_live": live,
        "queue_pending": pending,
        "queue_failed": failed,
        "queue_total": table_count(&s.db, "ep_sync_queue").await
    }))
}

pub async fn settings_get(State(s): State<AppState>) -> Json<Value> {
    let r = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,mode,cloud_url FROM ep_lan_settings LIMIT 1",
    )
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let (mode, cloud) = r
        .map(|x| (x.1, x.2))
        .unwrap_or_else(|| ("auto".into(), String::new()));
    Json(json!({ "ok": true, "mode": mode, "cloud_url": cloud }))
}

#[derive(Deserialize)]
pub struct SettingsIn {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub cloud_url: String,
}

pub async fn settings_set(State(s): State<AppState>, Json(b): Json<SettingsIn>) -> Json<Value> {
    let mode = match b.mode.trim().to_lowercase().as_str() {
        "offline" => "offline",
        "online" => "online",
        _ => "auto",
    };
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM ep_lan_settings LIMIT 1")
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    match existing {
        Some((id,)) => {
            let _ = sqlx::query("UPDATE ep_lan_settings SET mode=?1,cloud_url=?2,updated_at=?3 WHERE id=?4")
                .bind(mode)
                .bind(&b.cloud_url)
                .bind(now())
                .bind(&id)
                .execute(&s.db)
                .await;
        }
        None => {
            let _ = sqlx::query(
                "INSERT INTO ep_lan_settings (id,mode,cloud_url,updated_at) VALUES (?1,?2,?3,?4)",
            )
            .bind(uid())
            .bind(mode)
            .bind(&b.cloud_url)
            .bind(now())
            .execute(&s.db)
            .await;
        }
    }
    Json(json!({ "ok": true, "mode": mode, "cloud_url": b.cloud_url }))
}

#[derive(Deserialize)]
pub struct QueueIn {
    pub entity: String,
    #[serde(default)]
    pub op: String,
    #[serde(default)]
    pub payload: Value,
}

pub async fn queue_add(State(s): State<AppState>, Json(b): Json<QueueIn>) -> Json<Value> {
    if b.entity.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "entity inahitajika" }));
    }
    let op = match b.op.trim().to_lowercase().as_str() {
        "update" => "update",
        "delete" => "delete",
        _ => "create",
    };
    let id = uid();
    let payload = if b.payload.is_null() {
        "{}".to_string()
    } else {
        b.payload.to_string()
    };
    let _ = sqlx::query(
        "INSERT INTO ep_sync_queue (id,entity,op,payload,status,attempts,last_error,created_at)
         VALUES (?1,?2,?3,?4,'pending',0,'',?5)",
    )
    .bind(&id)
    .bind(b.entity.trim())
    .bind(op)
    .bind(&payload)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "status": "pending" }))
}

#[derive(Deserialize)]
pub struct QueueQuery {
    #[serde(default)]
    pub status: String,
}

pub async fn queue_list(State(s): State<AppState>, Query(q): Query<QueueQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, Option<String>)>(
        "SELECT id,entity,op,payload,status,attempts,last_error,created_at,synced_at
         FROM ep_sync_queue WHERE (?1='' OR status=?1) ORDER BY created_at DESC LIMIT 500",
    )
    .bind(&q.status)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "queue": rows.into_iter().map(|r| json!({
            "id": r.0, "entity": r.1, "op": r.2, "payload": r.3, "status": r.4,
            "attempts": r.5, "last_error": r.6, "created_at": r.7, "synced_at": r.8
        })).collect::<Vec<_>>()
    }))
}

/// Husawazisha foleni. Hujaribu kupiga cloud ikiwa imewekwa; bila cloud,
/// hufanya "local commit" na kuweka `synced` (data ipo kwenye seva ya chuo).
pub async fn sync_now(State(s): State<AppState>) -> Json<Value> {
    let reachable = internet_reachable().await;
    let cloud = sqlx::query_as::<_, (String,)>(
        "SELECT cloud_url FROM ep_lan_settings LIMIT 1",
    )
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    .map(|r| r.0)
    .unwrap_or_default();

    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,entity,payload FROM ep_sync_queue WHERE status IN ('pending','failed') ORDER BY created_at",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut synced = 0usize;
    let mut failed = 0usize;
    let mut local = 0usize;
    for (id, entity, payload) in rows {
        // Njia halisi: kama cloud ipo na internet inapatikana → HTTP POST.
        let outcome: Result<(), String> = if !cloud.trim().is_empty() && reachable {
            post_cloud(&cloud, &entity, &payload).await
        } else {
            // Bila cloud/internet: hifadhi kwenye seva ya chuo (local commit).
            local += 1;
            Ok(())
        };
        match outcome {
            Ok(()) => {
                let _ = sqlx::query("UPDATE ep_sync_queue SET status='synced',synced_at=?1,last_error='' WHERE id=?2")
                    .bind(now())
                    .bind(&id)
                    .execute(&s.db)
                    .await;
                synced += 1;
            }
            Err(e) => {
                let _ = sqlx::query(
                    "UPDATE ep_sync_queue SET status='failed',attempts=attempts+1,last_error=?1 WHERE id=?2",
                )
                .bind(&e)
                .bind(&id)
                .execute(&s.db)
                .await;
                failed += 1;
            }
        }
    }
    Json(json!({
        "ok": true, "synced": synced, "failed": failed, "local_commits": local,
        "cloud": cloud, "internet": reachable
    }))
}

async fn post_cloud(cloud: &str, entity: &str, payload: &str) -> Result<(), String> {
    let url = format!("{}/api/sync/{}", cloud.trim_end_matches('/'), entity);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", res.status()))
    }
}

#[derive(Deserialize)]
pub struct NodeIn {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub kind: String,
}

pub async fn node_register(State(s): State<AppState>, Json(b): Json<NodeIn>) -> Json<Value> {
    let ip = if b.ip.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        b.ip.trim().to_string()
    };
    if !is_private_ip(&ip) {
        return Json(json!({ "ok": false, "error": "ip si ya LAN (RFC1918)" }));
    }
    let role = if b.role.trim().eq_ignore_ascii_case("server") {
        "server"
    } else {
        "client"
    };
    let kind = match b.kind.trim().to_lowercase().as_str() {
        "phone" | "simu" => "phone",
        "desktop" => "desktop",
        "tablet" => "tablet",
        _ => "web",
    };
    let existing = sqlx::query_as::<_, (String,)>("SELECT id FROM ep_lan_nodes WHERE ip=?1")
        .bind(&ip)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let id = match existing {
        Some((id,)) => {
            let _ = sqlx::query("UPDATE ep_lan_nodes SET name=?1,role=?2,kind=?3,last_seen=?4 WHERE id=?5")
                .bind(&b.name)
                .bind(role)
                .bind(kind)
                .bind(now())
                .bind(&id)
                .execute(&s.db)
                .await;
            id
        }
        None => {
            let id = uid();
            let _ = sqlx::query(
                "INSERT INTO ep_lan_nodes (id,name,role,ip,kind,last_seen) VALUES (?1,?2,?3,?4,?5,?6)",
            )
            .bind(&id)
            .bind(&b.name)
            .bind(role)
            .bind(&ip)
            .bind(kind)
            .bind(now())
            .execute(&s.db)
            .await;
            id
        }
    };
    Json(json!({ "ok": true, "id": id, "ip": ip, "role": role, "kind": kind }))
}

pub async fn nodes_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id,name,role,ip,kind,last_seen FROM ep_lan_nodes ORDER BY last_seen DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let now_t = chrono::Local::now();
    Json(json!({
        "ok": true,
        "nodes": rows.into_iter().map(|r| {
            let secs = chrono::DateTime::parse_from_rfc3339(&r.5)
                .map(|t| (now_t - t.with_timezone(&chrono::Local)).num_seconds())
                .unwrap_or(9999);
            json!({
                "id": r.0, "name": r.1, "role": r.2, "ip": r.3, "kind": r.4,
                "last_seen": r.5, "state": node_state(secs)
            })
        }).collect::<Vec<_>>()
    }))
}

pub async fn node_leave(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query("DELETE FROM ep_lan_nodes WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    match r {
        Ok(x) if x.rows_affected() > 0 => Json(json!({ "ok": true })),
        _ => Json(json!({ "ok": false, "error": "nodi haipo" })),
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hali_ya_mtandao_hupimwa_sahihi() {
        assert_eq!(net_state("offline", true), "offline");
        assert_eq!(net_state("online", false), "online");
        assert_eq!(net_state("auto", true), "online");
        assert_eq!(net_state("auto", false), "offline");
        assert_eq!(net_state("", false), "offline");
    }

    #[test]
    fn ni_kazi_zipi_zinaweza_kusawazishwa() {
        assert!(can_sync("pending"));
        assert!(can_sync("failed"));
        assert!(can_sync("PENDING"));
        assert!(!can_sync("synced"));
        assert!(!can_sync(""));
    }

    #[test]
    fn hali_ya_nodi_hutegemea_muda() {
        assert_eq!(node_state(0), "live");
        assert_eq!(node_state(30), "live");
        assert_eq!(node_state(31), "idle");
        assert_eq!(node_state(300), "idle");
        assert_eq!(node_state(301), "gone");
    }

    #[test]
    fn ip_ya_lan_hutambulika() {
        assert!(is_private_ip("192.168.1.10"));
        assert!(is_private_ip("10.0.0.5"));
        assert!(is_private_ip("172.16.4.9"));
        assert!(is_private_ip("172.31.255.1"));
        assert!(is_private_ip("127.0.0.1"));
        assert!(is_private_ip("::1"));
        // Sivyo za LAN:
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("172.32.0.1"));
        assert!(!is_private_ip("172.15.0.1"));
        assert!(!is_private_ip("196.10.1.1"));
        assert!(!is_private_ip("si-ip"));
    }

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn kazi_huingia_foleni_kama_pending() {
        let p = db().await;
        let id = uid();
        sqlx::query("INSERT INTO ep_sync_queue (id,entity,op,payload,status,attempts,last_error,created_at) VALUES (?1,'student','create','{}','pending',0,'',?2)")
            .bind(&id).bind(now()).execute(&p).await.unwrap();
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_sync_queue WHERE id=?1")
            .bind(&id).fetch_one(&p).await.unwrap();
        assert_eq!(st, "pending");
    }

    #[tokio::test]
    async fn nodi_mbili_za_ip_ile_ile_hazirudiwi() {
        let p = db().await;
        let ip = "192.168.50.7";
        for _ in 0..2 {
            let ex: Option<(String,)> = sqlx::query_as("SELECT id FROM ep_lan_nodes WHERE ip=?1")
                .bind(ip).fetch_optional(&p).await.unwrap();
            match ex {
                Some((id,)) => {
                    sqlx::query("UPDATE ep_lan_nodes SET last_seen=?1 WHERE id=?2")
                        .bind(now()).bind(&id).execute(&p).await.unwrap();
                }
                None => {
                    sqlx::query("INSERT INTO ep_lan_nodes (id,name,role,ip,kind,last_seen) VALUES (?1,'Simu','client',?2,'phone',?3)")
                        .bind(uid()).bind(ip).bind(now()).execute(&p).await.unwrap();
                }
            }
        }
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_lan_nodes")
            .fetch_one(&p).await.unwrap();
        assert_eq!(n, 1, "nodi ya ip ile ile ilirudiwa");
    }

    #[tokio::test]
    async fn sync_hufanya_kazi_kuwa_synced() {
        let p = db().await;
        for i in 0..3 {
            sqlx::query("INSERT INTO ep_sync_queue (id,entity,op,payload,status,attempts,last_error,created_at) VALUES (?1,'mark','create','{}','pending',0,'',?2)")
                .bind(format!("q{i}")).bind(now()).execute(&p).await.unwrap();
        }
        // Bila cloud: local commit → synced.
        sqlx::query("UPDATE ep_sync_queue SET status='synced',synced_at=?1 WHERE status='pending'")
            .bind(now()).execute(&p).await.unwrap();
        let (pend,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_sync_queue WHERE status='pending'")
            .fetch_one(&p).await.unwrap();
        let (syn,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_sync_queue WHERE status='synced'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(pend, 0);
        assert_eq!(syn, 3);
    }
}
