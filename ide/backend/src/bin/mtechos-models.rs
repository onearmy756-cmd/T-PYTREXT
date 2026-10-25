//! mtechos-models — sync ya katalogi ya Ollama Cloud
//! (Rust — hakuna Python).
//!
//! Huchukua modeli zote za Ollama Cloud kwa key iliyowekwa na kuunganisha
//! entries kwenye `data/ai/models.json`. Idempotent: re-run husasisha metadata
//! ya cloud PEKEE, haiondoi local/HuggingFace entries.
//!
//! Ufumbuzi wa key (siri, HAIWEKI kwenye repo):
//!   1. `$OLLAMA_API_KEY` / `$FUNDI_AI_KEY`
//!   2. `<data>/ai_key` (faili ile ile agent inayotumia)
//!
//! Matumizi:
//!   mtechos-models [--data <dir>] [--catalog <models.json>]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use serde_json::{json, Value};

const TAGS_URL: &str = "https://ollama.com/api/tags";

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("FUNDI_DATA").unwrap_or_else(|_| "../data".to_string()))
}

async fn fetch_models(key: &str) -> Result<Vec<Value>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(TAGS_URL)
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body["models"].as_array().cloned().unwrap_or_default())
}

fn resolve_key(data: &PathBuf) -> Result<String, String> {
    for env in ["OLLAMA_API_KEY", "FUNDI_AI_KEY"] {
        if let Ok(v) = std::env::var(env) {
            if !v.trim().is_empty() {
                return Ok(v.trim().to_string());
            }
        }
    }
    for cand in [data.join("ai_key"), data.join("data").join("ai_key")] {
        if let Ok(t) = std::fs::read_to_string(&cand) {
            let k = t.trim();
            if !k.is_empty() {
                return Ok(k.to_string());
            }
        }
    }
    Err("Hakuna key: weka OLLAMA_API_KEY au <data>/ai_key".to_string())
}

fn classify(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.contains("code") {
        return "code";
    }
    if ["vision", "vl", "llava"].iter().any(|t| n.contains(t)) {
        return "vision";
    }
    if n.contains("embed") {
        return "embed";
    }
    "chat|reason|summarize|translate"
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut data = data_dir();
    let mut catalog_arg: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data" => {
                if let Some(v) = args.get(i + 1) {
                    data = PathBuf::from(v);
                }
                i += 2;
            }
            "--catalog" => {
                catalog_arg = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            _ => i += 1,
        }
    }

    let catalog_path = catalog_arg.unwrap_or_else(|| data.join("ai").join("models.json"));
    if !catalog_path.is_file() {
        eprintln!("catalog haipo: {}", catalog_path.display());
        return ExitCode::FAILURE;
    }

    let key = match resolve_key(&data) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let models = match fetch_models(&key).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("fetch imeshindwa: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("Ollama Cloud: modeli {} zimepatikana kwa key hii", models.len());

    let raw = match std::fs::read_to_string(&catalog_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("catalog haisomeki: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut catalog: Value = match serde_json::from_str(&raw) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("catalog sio JSON sahihi: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut existing: HashMap<String, Value> = HashMap::new();
    let mut base_priority: i64 = 2;
    if let Some(list) = catalog["models"].as_array() {
        for m in list {
            if let Some(id) = m["id"].as_str() {
                if !id.contains('/') {
                    base_priority = base_priority.max(m["priority"].as_i64().unwrap_or(999));
                }
                existing.insert(id.to_string(), m.clone());
            }
        }
    }

    let mut names: Vec<String> = models
        .iter()
        .filter_map(|m| m["name"].as_str().map(str::to_string))
        .collect();
    names.sort();

    let mut added = 0usize;
    let mut updated = 0usize;
    for (i, name) in names.iter().enumerate() {
        let size = models
            .iter()
            .find(|m| m["name"].as_str() == Some(name.as_str()))
            .and_then(|m| m["size"].as_u64())
            .unwrap_or(0);
        let params = if size > 0 {
            format!("{:.1}GB", size as f64 / 1e9)
        } else {
            "cloud".to_string()
        };
        let entry = json!({
            "id": name,
            "provider": "ollama_cloud",
            "params": params,
            "task": classify(name),
            "sw": "nzuri",
            "free": true,
            "priority": base_priority + i as i64 + 1,
            "note": "Ollama Cloud — bure kwa key (imeorodheshwa kutoka api/tags)",
        });
        if existing.contains_key(name) {
            // Sasisha params/task pekee — priority ya mkono haibadiliki.
            if let Some(list) = catalog["models"].as_array_mut() {
                for m in list.iter_mut() {
                    if m["id"].as_str() == Some(name.as_str()) {
                        m["params"] = entry["params"].clone();
                        m["task"] = entry["task"].clone();
                        break;
                    }
                }
            }
            updated += 1;
        } else {
            if let Some(list) = catalog["models"].as_array_mut() {
                list.push(entry.clone());
            }
            existing.insert(name.clone(), entry);
            added += 1;
        }
    }

    if catalog["routing"].is_null() {
        catalog["routing"] = json!({});
    }
    catalog["routing"]["cloud_models_synced"] = json!(models.len());

    let pretty = serde_json::to_string_pretty(&catalog).unwrap_or_default();
    if let Err(e) = std::fs::write(&catalog_path, format!("{pretty}\n")) {
        eprintln!("write imeshindwa: {e}");
        return ExitCode::FAILURE;
    }

    let total = catalog["models"].as_array().map(|a| a.len()).unwrap_or(0);
    println!("catalog: {}", catalog_path.display());
    println!("  +{added} mpya, ~{updated} imesasishwa, jumla ya models = {total}");
    println!("  id za cloud: {}", names.join(", "));
    ExitCode::SUCCESS
}
