//! fleet.rs — AGENT LAYER (H14): agents 100+ kwenye kompyuta zote.
//!
//! Mtiririko (kama architecture ya mmiliki — AGENT LAYER):
//!   1. REGISTER: agent ya kompyuta inajiandikisha (jina la kifaa) → inapata
//!      TOKEN ya siri (inatambulisha kila mawasiliano).
//!   2. HEARTBEAT: kila sekunde chache agent anatuma hali (afya, OS) —
//!      server inarekodi last_seen; kama amekimbia (zaidi ya dakika 3) anaonekana
//!      "OFFLINE" kwenye dashboard.
//!   3. POLL: agent anauliza kazi zake ZILIZOIDHINISHWA (HITL) — anapata kazi
//!      moja kwa wakati (oldest first), anaanza kuitekeleza kwenye kifaa chake.
//!   4. REPORT: agent anaripoti maendeleo (progress %, ujumbe) — dashboard
//!      inaona kila kitu kwa wakati halisi.
//!
//! KANUNI YA SIRI: API inarudisha majina salama tu; token zinafichwa kwenye DB
//! (hash SHA-256); agent haoni kamwe usanifu wa ndani wa server.

use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Agent anachukuliwa OFFLINE baada ya muda huu (ms) — dakika 3
pub const OFFLINE_AFTER_MS: i64 = 3 * 60 * 1000;
/// Kikomo cha agents (100+ kama mpango — tunaruhusu hadi 500)
pub const MAX_AGENTS: usize = 500;

pub fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ---------- DB ----------
pub async fn init_tables(db: &SqlitePool) {
    for ddl in [r#"CREATE TABLE IF NOT EXISTS fleet_agents (
            id TEXT PRIMARY KEY,
            token_hash TEXT NOT NULL,
            device TEXT NOT NULL,
            os_type TEXT,
            status TEXT NOT NULL DEFAULT 'idle',
            current_job TEXT,
            progress INTEGER NOT NULL DEFAULT 0,
            health TEXT,
            registered_at TEXT NOT NULL,
            last_seen_ms INTEGER NOT NULL
        )"#]
    {
        let _ = sqlx::query(ddl).execute(db).await;
    }
}

/// REGISTER: agent mpya au kurudi — inapata token (mara moja tu)
pub async fn register(
    db: &SqlitePool,
    device: &str,
    os_type: &str,
    health: &str,
) -> Result<(String, String, i64), String> {
    let device = device.trim();
    if device.is_empty() || device.len() > 64 || !crate::tools_internal::valid_target(device) {
        return Err("jina la kifaa si salama".into());
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let now = chrono::Local::now().to_rfc3339();
    // agent aliyekwisha? (device = PRIMARY key ya kimantiki kwa kifaa)
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM fleet_agents WHERE device=?1")
        .bind(device)
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?;
    let token = format!("mt_{}", Uuid::new_v4().simple());
    let th = hash_token(&token);
    let id = match existing {
        Some((id,)) => {
            // kurudi kwa agent — token mpya (ya kale inafutwa)
            sqlx::query("UPDATE fleet_agents SET token_hash=?2, os_type=?3, health=?4, status='idle', last_seen_ms=?5 WHERE id=?1")
                .bind(&id).bind(&th).bind(os_type).bind(health).bind(now_ms)
                .execute(db).await.map_err(|e| e.to_string())?;
            id
        }
        None => {
            let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM fleet_agents")
                .fetch_one(db)
                .await
                .map_err(|e| e.to_string())?;
            if n.0 >= MAX_AGENTS as i64 {
                return Err(format!("kikomo cha agents kimefika ({MAX_AGENTS})"));
            }
            let id = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO fleet_agents (id, token_hash, device, os_type, status, current_job, progress, health, registered_at, last_seen_ms) VALUES (?1,?2,?3,?4,'idle','',0,?5,?6,?7)")
                .bind(&id).bind(&th).bind(device).bind(os_type).bind(health).bind(&now).bind(now_ms)
                .execute(db).await.map_err(|e| e.to_string())?;
            id
        }
    };
    Ok((id, token, now_ms))
}

/// HEARTBEAT: agent anathibitisha yuupo — inarudisha (id, device, status, current_job)
pub async fn heartbeat(
    db: &SqlitePool,
    agent_id: &str,
    token: &str,
    health: &str,
) -> Result<(String, String, String, Option<String>), String> {
    let th = hash_token(token);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, device, status, current_job FROM fleet_agents WHERE id=?1 AND token_hash=?2",
    )
    .bind(agent_id)
    .bind(&th)
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;
    let row = row.ok_or_else(|| "token si sahihi — jisajili upya".to_string())?;
    sqlx::query("UPDATE fleet_agents SET last_seen_ms=?2, health=?3 WHERE id=?1")
        .bind(agent_id)
        .bind(now_ms)
        .bind(health)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row)
}

/// AUTH kwa poll/report — inarudisha (id, device) kama token ni sahihi
pub async fn auth(db: &SqlitePool, agent_id: &str, token: &str) -> Option<(String, String)> {
    let th = hash_token(token);
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, device FROM fleet_agents WHERE id=?1 AND token_hash=?2")
            .bind(agent_id)
            .bind(&th)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
    row
}

// ---------- POLL: claim kazi ILIYOIDHINISHWA (HITL) ----------

/// Pure fn: chagua kazi ya kwanza (oldest) iliyoidhinishwa kwa kifaa hiki.
/// Kazi ya HITL (stage `secops:*` au `bundle:*` au deploy) inakubalika kwa agent
/// tu BAADA ya approve (status = "approved").
pub fn pick_approved_job<'a>(
    jobs: &'a [crate::pipeline::Job],
    device: &str,
    busy_job: Option<&str>,
) -> Option<&'a crate::pipeline::Job> {
    if busy_job.is_some() {
        return None; // agent ana kazi tayari — anaanza mpya baada ya kumaliza
    }
    jobs.iter()
        .filter(|j| j.status == "approved" && j.device_name == device && !j.needs_approval)
        .min_by(|a, b| a.id.cmp(&b.id))
}

/// POLL: agent anauliza kazi — kama ipo, kazi inabadilika kuwa "running"
pub async fn poll(
    db: &SqlitePool,
    jobs: &tokio::sync::RwLock<Vec<crate::pipeline::Job>>,
    device: &str,
) -> serde_json::Value {
    // agent huyu ana kazi inayoendelea?
    let busy: Option<(Option<String>,)> =
        sqlx::query_as("SELECT current_job FROM fleet_agents WHERE device=?1")
            .bind(device)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
    let busy_job = busy.and_then(|b| b.0).filter(|s| !s.is_empty());

    let mut w = jobs.write().await;
    match pick_approved_job(&w, device, busy_job.as_deref()) {
        Some(j) => {
            let id = j.id.clone();
            let stage = j.stage.clone();
            let os = j.os_type.clone();
            // APPS HALISI: kama kazi ni `bundle:<id>`, tuma amri za install (winget/apt)
            // kwa OS ya kifaa — client agent inazitekeleza, si placeholder.
            let apps: Vec<String> = stage
                .strip_prefix("bundle:")
                .map(|bid| crate::bundles::install_commands_for_bundle(bid, &os))
                .unwrap_or_default();
            if let Some(job) = w.iter_mut().find(|x| x.id == id) {
                job.status = "running".into();
                job.progress = 5;
                job.message = "Agent ameanza kazi kwenye kifaa".into();
            }
            drop(w);
            let _ = sqlx::query("UPDATE jobs SET status='running', progress=5, message='Agent ameanza kazi kwenye kifaa' WHERE id=?1")
                .bind(&id)
                .execute(db)
                .await;
            let _ = sqlx::query("UPDATE fleet_agents SET status='working', current_job=?2, progress=5 WHERE device=?1")
                .bind(device).bind(&id)
                .execute(db)
                .await;
            json!({ "ok": true, "job": { "id": id, "stage": stage, "os_type": os, "apps": apps }, "note_sw": "Anza kazi — ripoti maendeleo kwa /api/fleet/report." })
        }
        None => {
            drop(w);
            let _ = sqlx::query("UPDATE fleet_agents SET status='idle', current_job='', progress=0 WHERE device=?1 AND current_job=''")
                .bind(device)
                .execute(db)
                .await;
            json!({ "ok": true, "job": serde_json::Value::Null, "note_sw": "Hakuna kazi mpya — subiri heartbeat." })
        }
    }
}

/// REPORT: maendeleo ya kazi (progress 0–100, ujumbe) — kazi 100 = imekamilika
pub async fn report(
    db: &SqlitePool,
    jobs: &tokio::sync::RwLock<Vec<crate::pipeline::Job>>,
    device: &str,
    progress: u32,
    message: &str,
) -> serde_json::Value {
    let progress = progress.clamp(0, 100);
    let (id, done): (Option<String>, bool) = {
        let busy: Option<(Option<String>,)> =
            sqlx::query_as("SELECT current_job FROM fleet_agents WHERE device=?1")
                .bind(device)
                .fetch_optional(db)
                .await
                .ok()
                .flatten();
        match busy.and_then(|b| b.0).filter(|s| !s.is_empty()) {
            Some(id) => {
                let mut w = jobs.write().await;
                if let Some(job) = w.iter_mut().find(|x| x.id == id) {
                    job.progress = progress;
                    job.message = message.to_string();
                    if progress >= 100 {
                        job.status = "done".into();
                    }
                }
                drop(w);
                let status = if progress >= 100 { "done" } else { "running" };
                let _ =
                    sqlx::query("UPDATE jobs SET status=?2, progress=?3, message=?4 WHERE id=?1")
                        .bind(&id)
                        .bind(status)
                        .bind(progress)
                        .bind(message)
                        .execute(db)
                        .await;
                (Some(id), progress >= 100)
            }
            None => (None, false),
        }
    };
    // kazi ikikamilika → agent arudi idle
    if done {
        let _ = sqlx::query(
            "UPDATE fleet_agents SET status='idle', current_job='', progress=100 WHERE device=?1",
        )
        .bind(device)
        .execute(db)
        .await;
    } else {
        let _ = sqlx::query("UPDATE fleet_agents SET progress=?2 WHERE device=?1")
            .bind(device)
            .bind(progress)
            .execute(db)
            .await;
    }
    match id {
        Some(id) => json!({ "ok": true, "job": id, "progress": progress, "done": done }),
        None => json!({ "ok": false, "error": "huna kazi inayoendelea" }),
    }
}

/// LIST: hali ya agents wote — online/offline kwa last_seen (kwa dashboard)
pub async fn list(db: &SqlitePool) -> serde_json::Value {
    let rows: Vec<(String, String, String, String, Option<String>, i64, Option<String>, i64)> = sqlx::query_as(
        "SELECT id, device, os_type, status, current_job, progress, health, last_seen_ms FROM fleet_agents ORDER BY device",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut online = 0i64;
    let mut working = 0i64;
    let mut agents = Vec::new();
    for (id, device, os, status, job, progress, health, last) in rows {
        let is_online = now_ms - last <= OFFLINE_AFTER_MS;
        if is_online {
            online += 1;
            if status == "working" {
                working += 1;
            }
        }
        agents.push(json!({
            "id": id, "device": device, "os": os,
            "status": if is_online { status } else { "offline".to_string() },
            "job": job.filter(|s| !s.is_empty()),
            "progress": progress, "health": health, "online": is_online,
        }));
    }
    json!({ "ok": true, "agents": agents, "total": agents.len(), "online": online, "working": working })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem() -> SqlitePool {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        db
    }

    #[test]
    fn pick_approved_job_inachagua_oldest_ya_kifaa_hiki_tu() {
        use crate::pipeline::Job;
        let mk = |id: &str, device: &str, status: &str, appr: bool| Job {
            id: id.into(),
            device_mac: String::new(),
            device_name: device.into(),
            os_type: String::new(),
            status: status.into(),
            stage: "secops:security".into(),
            progress: 0,
            message: String::new(),
            needs_approval: appr,
            image: None,
            multicast: false,
        };
        let jobs = vec![
            mk("b-2", "pc-01", "approved", false),
            mk("a-1", "pc-01", "approved", false),
            mk("c-3", "pc-02", "approved", false),
            mk("d-4", "pc-01", "queued", true),
            mk("e-5", "pc-01", "approved", true),
        ];
        // oldest (a-1) ya pc-01, isipokuwa za HITL
        let p = pick_approved_job(&jobs, "pc-01", None).unwrap();
        assert_eq!(p.id, "a-1");
        // busy → hakuna kazi mpya
        assert!(pick_approved_job(&jobs, "pc-01", Some("a-1")).is_none());
        // kifaa kingine inapata yake
        assert_eq!(pick_approved_job(&jobs, "pc-02", None).unwrap().id, "c-3");
        // kifaa kisicho na kazi → None
        assert!(pick_approved_job(&jobs, "pc-99", None).is_none());
    }

    #[test]
    fn token_inahifadhiwa_kama_hash_tu() {
        let h1 = hash_token("mt_secret123");
        let h2 = hash_token("mt_secret123");
        let h3 = hash_token("mt_other");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 64);
        assert!(!h1.contains("secret"), "hash haina token halisi");
    }

    #[tokio::test]
    async fn agents_120_zote_zinasajiliwa_na_kila_mmoja_token_yake() {
        let db = mem().await;
        let mut tokens = Vec::new();
        for i in 1..=120 {
            let device = format!("lab_user_{i:03}");
            let (id, token, _) = register(&db, &device, "win11", "nzuri").await.unwrap();
            assert!(!id.is_empty() && token.starts_with("mt_"));
            tokens.push((device, token));
        }
        let l = list(&db).await;
        assert_eq!(l["total"], serde_json::json!(120));
        assert_eq!(l["online"], serde_json::json!(120));
        // kikomo cha MAX_AGENTS (500) — agents wapya wanakataliwa wakati umefika
        assert!(
            register(&db, "x", "win11", "").await.is_ok(),
            "chini ya kikomo: inaruhusu"
        );
    }

    #[tokio::test]
    async fn jina_la_kifaa_mibaya_linakataliwa() {
        let db = mem().await;
        for bad in ["", "bad; rm -rf /", "$(whoami)", &"a".repeat(100)] {
            assert!(register(&db, bad, "win11", "").await.is_err(), "bad: {bad}");
        }
    }

    #[tokio::test]
    async fn token_si_sahihi_inakataa_heartbeat_na_report() {
        let db = mem().await;
        let (id, token, _) = register(&db, "pc-01", "win11", "nzuri").await.unwrap();
        // heartbeat sahihi
        let hb = heartbeat(&db, &id, &token, "nzuri").await.unwrap();
        assert_eq!(hb.1, "pc-01");
        // heartbeat token mbaya
        assert!(heartbeat(&db, &id, "mt_hela", "nzuri").await.is_err());
        assert!(auth(&db, &id, "mt_mbaya").await.is_none());
    }

    #[tokio::test]
    async fn mtiririko_kamili_hitl_kadi_ya_kazi_na_report() {
        let db = mem().await;
        let jobs = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new()));
        let (id, token, _) = register(&db, "pc-01", "win11", "nzuri").await.unwrap();
        // 1. poll bila kazi → job: null
        let p0 = poll(&db, &jobs, "pc-01").await;
        assert!(p0["job"].is_null());
        // 2. kazi ya HITL (needs_approval) — agent HAIITOI
        jobs.write().await.push(crate::pipeline::Job {
            id: "j-1".into(),
            device_mac: String::new(),
            device_name: "pc-01".into(),
            os_type: String::new(),
            status: "queued".into(),
            stage: "secops:security".into(),
            progress: 0,
            message: String::new(),
            needs_approval: true,
            image: None,
            multicast: false,
        });
        let p1 = poll(&db, &jobs, "pc-01").await;
        assert!(p1["job"].is_null(), "HITL: kazi kabla ya idhini haitolewi");
        // 3. idhini (kama approve_job) → agent anapata
        {
            let mut w = jobs.write().await;
            let j = w.iter_mut().find(|x| x.id == "j-1").unwrap();
            j.status = "approved".into();
            j.needs_approval = false;
        }
        let p2 = poll(&db, &jobs, "pc-01").await;
        assert_eq!(p2["job"]["id"], serde_json::json!("j-1"));
        // agent sasa working
        let l = list(&db).await;
        assert_eq!(l["working"], serde_json::json!(1));
        // 4. poll tena wakati busy → hakuna kazi mpya
        let p3 = poll(&db, &jobs, "pc-01").await;
        assert!(p3["job"].is_null());
        // 5. report maendeleo → 50%
        let r1 = report(&db, &jobs, "pc-01", 50, "Nusu yaja").await;
        assert_eq!(r1["progress"], serde_json::json!(50));
        // 6. report 100 → kazi done, agent idle
        let r2 = report(&db, &jobs, "pc-01", 100, "Imekamilika").await;
        assert_eq!(r2["done"], serde_json::Value::Bool(true));
        let w = jobs.read().await;
        assert_eq!(w[0].status, "done");
        drop(w);
        let l2 = list(&db).await;
        assert_eq!(l2["working"], serde_json::json!(0));
        // report bila kazi → kosa laini
        let r3 = report(&db, &jobs, "pc-01", 10, "hakuna").await;
        assert_eq!(r3["ok"], serde_json::Value::Bool(false));
        // token haikutumika kwenye poll/report handlers (auth inaitwa hapo juu) — thibitisha
        assert!(auth(&db, &id, &token).await.is_some());
    }

    #[tokio::test]
    async fn offline_baada_ya_dakika_3() {
        let db = mem().await;
        let (id, _token, _) = register(&db, "pc-01", "win11", "nzuri").await.unwrap();
        // weka last_seen ya zamani (dakika 10 zilizopita)
        let old = chrono::Utc::now().timestamp_millis() - (10 * 60 * 1000);
        let _ = sqlx::query("UPDATE fleet_agents SET last_seen_ms=?2 WHERE id=?1")
            .bind(&id)
            .bind(old)
            .execute(&db)
            .await;
        let l = list(&db).await;
        assert_eq!(l["online"], serde_json::json!(0));
        assert_eq!(l["agents"][0]["status"], serde_json::json!("offline"));
    }
}
