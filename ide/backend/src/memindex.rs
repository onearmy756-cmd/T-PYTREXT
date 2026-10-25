//! memindex.rs — INDEX YA VECTOR: DEPLOYMENTS + COMPUTERS → LanceDB (MTECH OS)
//!
//! Doc ya MTECH OS inadai Deployments/Problems/Computers wakiwa kwenye LanceDB.
//! Problems tayari naenda kwenye brain_memories (daily::learn_from_finding);
//! hapa tunajenga index huru ya `deployments.lance` (kila job) na
//! `computers.lance` (kila agent) — JSONL shards LanceDB-compatible, append
//! tu kwa rekodi MPyA (hakuna marudio).

use crate::vector::{Embedding, LanceStore, VectorRecord};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashSet;

fn ids_of(store: &LanceStore) -> HashSet<i64> {
    store.list_records().into_iter().map(|r| r.id).collect()
}

/// Jenga/up-date index: ongeza jobs na agents ambazo hazijasajiliwa bado.
pub async fn rebuild(db: &SqlitePool, data_dir: &str) -> Value {
    let now = chrono::Local::now().to_rfc3339();

    // ---------- DEPLOYMENTS (kila job → deployments.lance) ----------
    let dep_store = LanceStore::open_named(data_dir, "deployments");
    let dep_have = ids_of(&dep_store);
    let jobs: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        r#"SELECT rowid,
                  COALESCE(device_name,''), COALESCE(os_type,''),
                  COALESCE(status,''), COALESCE(message,'')
           FROM jobs"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut dep_added = 0usize;
    for (id, device, os, status, message) in jobs {
        if dep_have.contains(&id) {
            continue;
        }
        let problem = format!("Deployment ya {os} kwa {device}: {status}");
        let solution = if message.is_empty() {
            status.clone()
        } else {
            message
        };
        let rec = VectorRecord {
            id,
            agent: "deployer".into(),
            pc: device,
            problem,
            solution,
            confidence: 1.0,
            created_at: now.clone(),
            embedding: Embedding::embed(&format!("deployment {os} {status}")),
        };
        if dep_store.append(rec).is_ok() {
            dep_added += 1;
        }
    }

    // ---------- COMPUTERS (kila agent → computers.lance) ----------
    let cmp_store = LanceStore::open_named(data_dir, "computers");
    let cmp_have = ids_of(&cmp_store);
    let agents: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        r#"SELECT rowid, device,
                  COALESCE(os_type,''), COALESCE(status,''), COALESCE(health,'')
           FROM fleet_agents"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut cmp_added = 0usize;
    for (id, device, os, status, health) in agents {
        if cmp_have.contains(&id) {
            continue;
        }
        let problem = format!("Kompyuta {device} (OS {os}) — hali {status}");
        let solution = format!("afya: {health}");
        // embedding kwanza — kabla ya kuhamaisha device kwenye `pc:`
        let emb = Embedding::embed(&format!("kompyuta {device} {os} {health}"));
        let rec = VectorRecord {
            id,
            agent: "inventory".into(),
            pc: device,
            problem,
            solution,
            confidence: 1.0,
            created_at: now.clone(),
            embedding: emb,
        };
        if cmp_store.append(rec).is_ok() {
            cmp_added += 1;
        }
    }

    json!({
        "ok": true,
        "deployments_added": dep_added,
        "computers_added": cmp_added,
        "deployments_total": dep_store.len(),
        "computers_total": cmp_store.len(),
        "note_sw": "Index imehifadhiwa LanceDB-compatible (JSONL shards) — hakuna marudio."
    })
}

/// Takwimu za index zote tatu: brain (problems), deployments, computers.
pub async fn stats(db: &SqlitePool, data_dir: &str) -> Value {
    let brain_n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM brain_memories")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let dep = LanceStore::open_named(data_dir, "deployments");
    let cmp = LanceStore::open_named(data_dir, "computers");
    let brain = LanceStore::open(data_dir);
    json!({
        "ok": true,
        "problems": { "sqlite": brain_n, "lance": brain.len() },
        "deployments": dep.len(),
        "computers": cmp.len(),
        "note_sw": "Problems tayari ndani ya brain_memories; deployments/computers ni index huru."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup() -> (SqlitePool, String) {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY, device_mac TEXT, device_name TEXT, os_type TEXT,
                os_reason TEXT, status TEXT, stage TEXT, progress INTEGER,
                message TEXT, backup_info TEXT)",
        )
        .execute(&db)
        .await
        .unwrap();
        crate::fleet::init_tables(&db).await;
        crate::brain::init_tables(&db).await;
        sqlx::query("INSERT INTO brain_memories (agent, pc, problem, solution, confidence, created_at) VALUES ('a','p','x','y',0.9,'now')")
            .execute(&db)
            .await
            .unwrap();
        let dir = std::env::temp_dir().join(format!("mtech-memindex-{}", uuid::Uuid::new_v4()));
        (db, dir.to_str().unwrap().to_string())
    }

    #[tokio::test]
    async fn rebuild_huongeza_bila_marudio() {
        let (db, dir) = setup().await;
        sqlx::query("INSERT INTO jobs (device_mac, device_name, os_type, os_reason, status, stage, progress, message, backup_info) VALUES ('aa','hr','ubuntu','auto','done','done',100,'','')")
            .execute(&db)
            .await
            .unwrap();
        let r1 = rebuild(&db, &dir).await;
        assert_eq!(r1["deployments_added"], 1);
        let r2 = rebuild(&db, &dir).await;
        assert_eq!(
            r2["deployments_added"], 0,
            "rebuild ya pili haziongezi marudio"
        );
        let s = stats(&db, &dir).await;
        assert_eq!(s["deployments"], 1);
        assert_eq!(s["problems"]["sqlite"], 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
