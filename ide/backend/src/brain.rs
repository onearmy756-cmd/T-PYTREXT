//! brain.rs — NEURALIS BRAIN: kumbukumbu ya PAMOJA ya agents wote (SEHEMU 13 / A.3).
//!
//! Kama Sentinel Desktop: "kila agent inajifunza kutoka kwa wengine." Agent
//! anayetatua tatizo kwenye PC moja anaandika lesson hapa — agent mwingine
//! anayekutana na tatizo lifanalo anarecall suluhisho mara moja.
//!
//! **H5b KAMILI**: dual-write — SQLite (durability, ripoti) + **vector store
//! (LanceDB-compatible: `data/lancedb/brain_memories.lance/`, JSONL shards)**.
//! Recall sasa ni **semantic search (cosine similarity)** juu ya embeddings —
//! si maneno yanayofanana tu. Interface ya Rust NI ILE ILE; swap ya backend
//! (JSONL → Lance crate) ni ndani ya vector.rs tu.
//!
//! KANUNI: kumbukumbu ZINATOKA na kazi HALISI (finding + solution + confidence
//! halisi ya ReAct session). Hakuna kujificha kwenye JSON config.

use crate::vector::{Embedding, LanceStore};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub agent: String,    // agent aliyeandika (mf. "reacon-PC01", "hr")
    pub pc: String,       // display_name ya PC (hr, hr 1…)
    pub problem: String,  // dalili/tatizo (mf. "Wi-Fi haifanyi kazi")
    pub solution: String, // nini kilifanya kazi
    pub confidence: f64,  // 0.0–1.0 (kutoka confidence_score ya reacon)
    pub created_at: String,
}

/// Dual-write store: SQLite + LanceDB-compatible vector store.
pub struct Brain {
    pub db: SqlitePool,
    pub vectors: Arc<LanceStore>,
}

impl Brain {
    pub fn new(db: SqlitePool, data_dir: &str) -> Brain {
        Brain {
            db,
            vectors: Arc::new(LanceStore::open(data_dir)),
        }
    }
}

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS brain_memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent TEXT NOT NULL,
            pc TEXT NOT NULL,
            problem TEXT NOT NULL,
            solution TEXT NOT NULL,
            confidence REAL NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

/// Remember: dual-write — SQLite row + record yenye embedding kwenye vector store.
pub async fn remember(
    brain: &Brain,
    agent: &str,
    pc: &str,
    problem: &str,
    solution: &str,
    confidence: f64,
) -> Result<i64, String> {
    if problem.trim().is_empty() || solution.trim().is_empty() {
        return Err("problem na solution ni lazima".into());
    }
    let conf = confidence.clamp(0.0, 1.0);
    let created = chrono::Local::now().to_rfc3339();
    let r = sqlx::query(
        "INSERT INTO brain_memories (agent, pc, problem, solution, confidence, created_at) VALUES (?,?,?,?,?,?)",
    )
    .bind(agent)
    .bind(pc)
    .bind(problem.trim())
    .bind(solution.trim())
    .bind(conf)
    .bind(&created)
    .execute(&brain.db)
    .await
    .map_err(|e| e.to_string())?;
    let id = r.last_insert_rowid();

    // Vector write (LanceDB-compatible) — embedding ya problem+solution.
    let rec = crate::vector::VectorRecord {
        id,
        agent: agent.to_string(),
        pc: pc.to_string(),
        problem: problem.trim().to_string(),
        solution: solution.trim().to_string(),
        confidence: conf,
        created_at: created,
        embedding: Embedding::embed(&format!("{problem} {solution}")),
    };
    brain.vectors.append(rec)?;

    Ok(id)
}

/// Recall: SEMANTIC SEARCH (cosine) — problem ya ulizo inalinganishwa na
/// embeddings za kumbukumbu (problem+solution). Score ya mwisho =
/// cosine × confidence ya kumbukumbu. Iliyoju zaidi juu.
pub async fn recall(brain: &Brain, query: &str, limit: usize) -> Vec<(Memory, f64)> {
    let q = Embedding::embed(query);
    brain
        .vectors
        .search(&q, limit, 0.01)
        .into_iter()
        .map(|(r, cosine)| {
            let m = Memory {
                agent: r.agent,
                pc: r.pc,
                problem: r.problem,
                solution: r.solution,
                confidence: r.confidence,
                created_at: r.created_at,
            };
            let score = cosine * m.confidence;
            (m, score)
        })
        .collect()
}

pub async fn stats(brain: &Brain) -> serde_json::Value {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM brain_memories")
        .fetch_one(&brain.db)
        .await
        .unwrap_or(0);
    let agents: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT agent FROM brain_memories")
        .fetch_all(&brain.db)
        .await
        .unwrap_or_default();
    serde_json::json!({
        "memories": total,
        "vectors": brain.vectors.len(),
        "backend": "lancedb-compatible (cosine semantic search)",
        "agents_sharing": agents.len(),
        "note_sw": "Kumbukumbu ya pamoja — kila agent anajifunza kutoka kwa wengine (Neuralis Brain)."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_brain() -> Brain {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-btest-{}", uuid::Uuid::new_v4()));
        Brain::new(db, dir.to_str().unwrap())
    }

    #[tokio::test]
    async fn remember_na_recall_zinajifunza_kutoka_wengine() {
        let b = test_brain().await;
        // Agent A anatatua Wi-Fi kwenye PC ya kwanza
        remember(
            &b,
            "agent-hr",
            "hr",
            "Wi-Fi haifanyi kazi",
            "restart ya wlansvc service",
            0.9,
        )
        .await
        .unwrap();
        // Agent B (PC nyingine) anakutana na tatizo lifanalo → anarecall (semantic)
        let got = recall(&b, "wi-fi haifanyi kazi kwenye laptop", 3).await;
        assert!(
            !got.is_empty(),
            "brain lazima ikumbuke suluhisho la tatizo linalofanana"
        );
        assert_eq!(got[0].0.solution, "restart ya wlansvc service");
        assert!(got[0].1 > 0.0, "score ya match lazima iwe > 0");
    }

    #[tokio::test]
    async fn recall_haitoi_isiyofanana() {
        let b = test_brain().await;
        remember(&b, "a", "pc1", "disk full", "kufuta temp files", 1.0)
            .await
            .unwrap();
        let got = recall(&b, "printer haichapi", 5).await;
        assert!(
            got.is_empty(),
            "cosine ya mada tofauti inabaki chini ya min_score"
        );
    }

    #[tokio::test]
    async fn remember_inakataa_mtupu() {
        let b = test_brain().await;
        assert!(remember(&b, "a", "pc", "", "sol", 1.0).await.is_err());
        assert!(remember(&b, "a", "pc", "prob", "", 1.0).await.is_err());
    }

    #[tokio::test]
    async fn confidence_inapunguzwa_kati_0_na_1() {
        let b = test_brain().await;
        remember(
            &b,
            "a",
            "p",
            "printer haichapi kabisa",
            "washa spooler",
            5.0,
        )
        .await
        .unwrap(); // 5.0 → clamp 1.0
        let got = recall(&b, "printer haichapi", 1).await;
        assert!(!got.is_empty());
        assert!(got[0].0.confidence <= 1.0);
    }

    #[tokio::test]
    async fn dual_write_vector_store_ina_records() {
        let b = test_brain().await;
        remember(&b, "a1", "pc1", "ram inakwama", "safisha slots", 0.8)
            .await
            .unwrap();
        remember(&b, "a2", "pc2", "ram inakwama tena", "badilisha stick", 0.7)
            .await
            .unwrap();
        assert_eq!(b.vectors.len(), 2, "kila remember inaandika vector pia");
        let got = recall(&b, "ram inakwama kwenye kompyuta", 2).await;
        assert_eq!(got.len(), 2);
    }
}
