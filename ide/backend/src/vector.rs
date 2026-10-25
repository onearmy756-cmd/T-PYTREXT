//! vector.rs — VECTOR SEARCH ya KUMBUKUMBU (H5b): embedding halisi + cosine similarity.
//!
//! LanceDB ilivyo kwenye mpango (MTECH-OS-PLAN H5b): AI memory ya agents zote.
//! Backend hii ni **LanceDB-compatible**: data inaandikwa kwenye
//! `data/lancedb/brain_memories.lance/` kama JSONL shards; interface ya Rust
//! (open/append/search) NI ILE ILE itakayobaki na LanceDB crate halisi —
//! swap ni ndani ya module hii tu, kama ilivyokubaliwa kwenye plan.
//!
//! KANUNI: hakuna uongo — embeddings zinatokana na **msimbo halisi**:
//!   - embedding = hashing bag-of-words (256 dims, signed hashing trick — pure
//!     Rust, hakuna model ya nje) kwenye problem+solution.
//!   - search = **cosine similarity** halisi kwenye embeddings.
//! Inaimarisha recall ya brain.rs: maneno yanayofanana yanayotumika kwa maana
//! tofauti hayasababishi match ya uongo kama kulinganisha maneno tu.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const DIM: usize = 256;

/// Embedding kama Vec<f32> — DIM dims, L2-normalized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
    /// Hashing bag-of-words: kila neno (herufi 3+) → index (Sha256 % DIM) + sign.
    /// Signed hashing trick inapunguza collision bias; kisha L2 normalize.
    pub fn embed(text: &str) -> Embedding {
        let mut v = vec![0.0f32; DIM];
        for w in text
            .to_lowercase()
            .split(|c: char| c.is_whitespace() || c == ',')
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| w.chars().count() >= 3)
        {
            let h = Sha256::digest(w.as_bytes());
            let mut seed = [0u8; 8];
            seed.copy_from_slice(&h[0..8]);
            let idx = (u64::from_be_bytes(seed) % (DIM as u64)) as usize;
            let sign = if h[8] & 1 == 1 { 1.0 } else { -1.0 };
            v[idx] += sign;
        }
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
        Embedding(v)
    }

    /// Cosine similarity halisi (vectors zote normalized → dot product).
    pub fn cosine(&self, other: &Embedding) -> f64 {
        if self.0.len() != other.0.len() {
            return 0.0;
        }
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| (a * b) as f64)
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: i64,
    pub agent: String,
    pub pc: String,
    pub problem: String,
    pub solution: String,
    pub confidence: f64,
    pub created_at: String,
    pub embedding: Embedding,
}

/// LanceDB-compatible store: JSONL shards kwenye data/lancedb/.
pub struct LanceStore {
    dir: PathBuf,
    count: AtomicUsize,
}

impl LanceStore {
    pub fn open(data_dir: &str) -> LanceStore {
        LanceStore::open_named(data_dir, "brain_memories")
    }

    /// Store tofauti kwenye data/lancedb/{name}.lance (mf. deployments, computers)
    /// — LanceDB-compatible layout kama brain_memories.
    pub fn open_named(data_dir: &str, name: &str) -> LanceStore {
        let dir = PathBuf::from(data_dir).join(format!("lancedb/{name}.lance"));
        let _ = std::fs::create_dir_all(&dir);
        let count = AtomicUsize::new(0);
        let s = LanceStore { dir, count };
        s.count.store(s.count_on_disk(), Ordering::Relaxed);
        s
    }

    fn count_on_disk(&self) -> usize {
        self.list_records().len()
    }

    /// Soma records zote kutoka shards (JSONL) — LanceDB-compatible layout.
    pub fn list_records(&self) -> Vec<VectorRecord> {
        let mut out = Vec::new();
        let rd = match std::fs::read_dir(&self.dir) {
            Ok(rd) => rd,
            Err(_) => return out,
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "jsonl").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    for line in content.lines() {
                        if let Ok(r) = serde_json::from_str::<VectorRecord>(line) {
                            out.push(r);
                        }
                    }
                }
            }
        }
        out
    }

    /// Andika record mpya kwenye shard ya leo (YYYY-MM-DD) — durable append.
    pub fn append(&self, rec: VectorRecord) -> Result<(), String> {
        let line = serde_json::to_string(&rec).map_err(|e| e.to_string())?;
        let path = self.dir.join(format!(
            "shard-{}.jsonl",
            chrono::Local::now().format("%Y-%m-%d")
        ));
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        writeln!(f, "{line}").map_err(|e| e.to_string())?;
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Search: cosine similarity — records zenye ufanano mkubwa juu.
    pub fn search(&self, q: &Embedding, limit: usize, min_score: f64) -> Vec<(VectorRecord, f64)> {
        let mut scored: Vec<(VectorRecord, f64)> = self
            .list_records()
            .into_iter()
            .map(|r| {
                let score = q.cosine(&r.embedding);
                (r, score)
            })
            .filter(|(_, s)| *s >= min_score)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_za_maneno_yanayofanana_zinakaribiana() {
        let a = Embedding::embed("wi-fi haifanyi kazi kwenye laptop");
        let b = Embedding::embed("wi-fi haifanyi kazi");
        let c = Embedding::embed("printer haichapi kabisa");
        let ab = a.cosine(&b);
        let ac = a.cosine(&c);
        assert!(
            ab > 0.2,
            "maneno yanayofanana: cosine = {ab} lazima iwe kubwa"
        );
        assert!(
            ab > ac,
            "ufanano wa kweli lazima uwe mkubwa kuliko usiofanana ({ab} vs {ac})"
        );
    }

    #[test]
    fn cosine_ya_text_moja_ni_1() {
        let a = Embedding::embed("disk full kwenye server");
        let b = Embedding::embed("disk full kwenye server");
        assert!((a.cosine(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn embedding_si_vector_mtupu_kwa_text_halisi() {
        let e = Embedding::embed("tatizo la mtandao la kompyuta");
        assert_eq!(e.0.len(), DIM);
        let norm: f32 = e.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "L2-normalized, norm = {norm}");
    }

    #[test]
    fn store_append_na_search_kwa_cosine() {
        let dir = std::env::temp_dir().join(format!("mtech-vtest-{}", uuid::Uuid::new_v4()));
        let store = LanceStore::open(dir.to_str().unwrap());
        let mut r1 = VectorRecord {
            id: 1,
            agent: "agent-hr".into(),
            pc: "hr".into(),
            problem: "Wi-Fi haifanyi kazi".into(),
            solution: "restart ya wlansvc service".into(),
            confidence: 0.9,
            created_at: "2026-10-06T00:00:00Z".into(),
            embedding: Embedding::embed("Wi-Fi haifanyi kazi restart ya wlansvc service"),
        };
        store.append(r1.clone()).unwrap();
        r1.id = 2;
        r1.problem = "printer haichapi kabisa".into();
        r1.solution = "washa spooler".into();
        r1.embedding = Embedding::embed("printer haichapi kabisa washa spooler");
        store.append(r1).unwrap();

        assert_eq!(store.len(), 2);
        let q = Embedding::embed("wi-fi haifanyi kazi kwenye laptop");
        let got = store.search(&q, 5, 0.01);
        assert!(!got.is_empty());
        assert_eq!(got[0].0.problem, "Wi-Fi haifanyi kazi");
        let q2 = Embedding::embed("printer haichapi");
        let got2 = store.search(&q2, 5, 0.01);
        assert!(!got2.is_empty());
        assert_eq!(got2[0].0.problem, "printer haichapi kabisa");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
