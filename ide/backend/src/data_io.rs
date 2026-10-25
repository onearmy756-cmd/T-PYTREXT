//! data_io.rs — UFIKIAJI WA data/ KWA AI: orodha ya mtindo, usomaji salama, katalogi ya model.
//!
//! Kanuni: AI (chat) inapata folder LOTE la data/ kwa kuandika "soma data/<path>";
//! data/ai/models.json ndio chanzo cha model (hakuna hard-code); siri/DB/binaries zimezuiwa.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Data dir iliyosetwa na server startup (FUNDI_DATA).
pub fn data_dir() -> PathBuf {
    let d = std::env::var("FUNDI_DATA").unwrap_or_else(|_| "data".into());
    PathBuf::from(d)
}

/// data/ai/models.json — katalogi rasmi ya model za bure (MTECHOS).
pub fn load_models_json() -> Value {
    let p = data_dir().join("ai").join("models.json");
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(
            || json!({ "error": "data/ai/models.json haipo", "path": p.display().to_string() }),
        )
}

/// Map ya mapendekezo kutoka models.json: default_chat, cloud URL, fallback chain.
pub fn recommended() -> Value {
    let c = load_models_json();
    let def = c["routing"]["default_chat"].as_str().unwrap_or("");
    let base = c["providers"]["ollama_cloud"]["base_url"]
        .as_str()
        .unwrap_or("https://ollama.com");
    let total = c["models"].as_array().map(|a| a.len()).unwrap_or(0);
    json!({
        "source": "data/ai/models.json",
        "default_chat": def,
        "ollama_cloud_url": base,
        "fallback": c["routing"]["fallback_chain"].clone(),
        "kiswahili": c["routing"]["kiswahili"].clone(),
        "models_total": total,
    })
}

/// Orodha ya mtindo wa data/ (diri + faili kwa ukubwa) — AI/UI inaona kila kitu kilichopo.
pub fn tree_json() -> Value {
    let root = data_dir();
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<Value> = Vec::new();
    let mut total = 0u64;
    walk(&root, &root, 0, 4, &mut dirs, &mut files, &mut total);
    json!({
        "ok": true,
        "root": root.display().to_string(),
        "dirs": dirs,
        "files": files,
        "total_files": total,
        "shown": files.len(),
    })
}

fn walk(
    root: &Path,
    dir: &Path,
    depth: u8,
    max: u8,
    dirs: &mut Vec<String>,
    files: &mut Vec<Value>,
    total: &mut u64,
) {
    if depth > max {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let rel = p
            .strip_prefix(root)
            .unwrap_or(&p)
            .to_string_lossy()
            .replace('\\', "/");
        if p.is_dir() {
            dirs.push(rel);
            walk(root, &p, depth + 1, max, dirs, files, total);
        } else {
            let sz = e.metadata().map(|m| m.len()).unwrap_or(0);
            *total += 1;
            if files.len() < 500 {
                files.push(json!({ "path": rel, "kb": (sz + 1023) / 1024 }));
            }
        }
    }
}

/// Usomaji salama wa faili lolote la data/: relative path tu, ZUIA siri/DB/binaries,
/// lazima ibaki ndani ya data/ (canonicalize), ukubwa ≤ 500KB.
pub fn read_file(rel: &str) -> Result<(String, String), String> {
    let rel = rel.trim().trim_start_matches('/').replace('\\', "/");
    //ondoa kiambato "data/" kama kimetolewa (root tayari ni data dir)
    let rel = rel.strip_prefix("data/").unwrap_or(&rel).to_string();
    if rel.is_empty() {
        return Err("Hakuna path iliyotolewa".into());
    }
    if rel.contains("..") {
        return Err("Si ruhusa: '..' haijakubalika".into());
    }
    let low = rel.to_lowercase();
    let blocked = low.ends_with("ai_key")
        || low.ends_with(".db")
        || low.ends_with(".sqlite")
        || low.ends_with(".gguf")
        || low.ends_with(".exe")
        || low.contains("/llm/")
        || low.ends_with(".env");
    if blocked {
        return Err("Faili hii ni siri/ya binary — haijasomwi".into());
    }
    let root = data_dir();
    let full = root.join(&rel);
    let can_root = std::fs::canonicalize(&root).map_err(|e| format!("data dir: {e}"))?;
    let can_file = std::fs::canonicalize(&full).map_err(|_| format!("Haipo: data/{rel}"))?;
    if !can_file.starts_with(&can_root) {
        return Err("Nje ya data/ — si ruhusa".into());
    }
    let sz = std::fs::metadata(&can_file).map(|m| m.len()).unwrap_or(0);
    if sz > 500_000 {
        return Err(format!(
            "Faili kubwa mno ({} KB) — chagua faili ndogo",
            sz / 1024
        ));
    }
    let txt = std::fs::read_to_string(&can_file)
        .map_err(|_| "Si faili ya maandishi (binary?)".to_string())?;
    Ok((txt, rel))
}
