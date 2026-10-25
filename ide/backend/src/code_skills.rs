//! code_skills.rs — SENIOR ENGINEERING SKILLS LIBRARY kwa agentic loop.
//!
//! Huhifadhi skills kama markdown chini ya `data/skills/engineering/` (chanzo
//! kimoja cha ukweli, data-driven). `index.json` hueleza skill ipi ni ya kila
//! wakati (`always`) na ipi inalingana na `kind`/maneno ya `idea` (`match`).
//!
//! `skills_for(kind, idea)` hurudisha block ya markdown inayoingizwa kwenye
//! system prompt ya agent — ili LLM iandike kama senior engineer wa lugha husika.
//! Kubadilisha skills hakuhitaji kufundisha modeli upya.

use std::path::PathBuf;
use serde_json::Value;

fn skills_dir() -> PathBuf {
    let candidates = [
        crate::data_io::data_dir().join("skills").join("engineering"),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/skills/engineering")
            .to_path_buf(),
        PathBuf::from("data/skills/engineering"),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| crate::data_io::data_dir().join("skills").join("engineering"))
}

/// Skills zilizoingizwa kutoka OpenHands / Claude Code / Trae.
fn imported_dir() -> PathBuf {
    let candidates = [
        crate::data_io::data_dir().join("skills").join("imported"),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/skills/imported")
            .to_path_buf(),
        PathBuf::from("data/skills/imported"),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| crate::data_io::data_dir().join("skills").join("imported"))
}

fn read_index(dir: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["skills"].as_array().cloned())
        .unwrap_or_default()
}

/// Markdown ya skills zinazolingana na `hay` kutoka saraka moja.
fn block_from_dir(dir: &std::path::Path, hay: &str) -> String {
    let mut out = String::new();
    for entry in read_index(dir) {
        let always = entry["always"].as_bool().unwrap_or(false);
        let matches = entry["match"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| m.as_str())
                    .any(|m| hay.contains(&m.to_lowercase()))
            })
            .unwrap_or(false);
        if !always && !matches {
            continue;
        }
        let Some(file) = entry["file"].as_str() else { continue };
        if let Ok(body) = std::fs::read_to_string(dir.join(file)) {
            out.push_str(&body);
            out.push_str("\n\n");
        }
    }
    out
}

/// Block ya skills inayolingana na `kind` + maneno ya `idea` (engine + zilizoingizwa).
pub fn skills_for(kind: &str, idea: &str) -> String {
    let hay = format!("{} {}", kind.to_lowercase(), idea.to_lowercase());
    let mut out = block_from_dir(&skills_dir(), &hay);
    out.push_str(&block_from_dir(&imported_dir(), &hay));
    out
}

/// Orodha ya skills zilizopo (kwa UI/uchunguzi).
pub fn list() -> Value {
    let dir = skills_dir();
    let mut items = Vec::new();
    for entry in read_index(&dir) {
        if let Some(file) = entry["file"].as_str() {
            items.push(serde_json::json!({
                "file": file,
                "source": "engineering",
                "always": entry["always"].as_bool().unwrap_or(false),
                "match": entry["match"].clone(),
                "exists": dir.join(file).is_file(),
            }));
        }
    }
    let idir = imported_dir();
    for entry in read_index(&idir) {
        if let Some(file) = entry["file"].as_str() {
            items.push(serde_json::json!({
                "file": file,
                "source": entry["source"].clone(),
                "name": entry["name"].clone(),
                "always": entry["always"].as_bool().unwrap_or(false),
                "match": entry["match"].clone(),
                "exists": idir.join(file).is_file(),
            }));
        }
    }
    serde_json::json!({ "ok": true, "dir": dir.display().to_string(), "imported_dir": idir.display().to_string(), "count": items.len(), "skills": items })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_skill_always_included() {
        let s = skills_for("unknown-kind", "kitu kidogo");
        assert!(s.contains("Core Agentic Loop"), "core skill lazima iwepo kila wakati");
        assert!(s.contains("Security Review"));
    }

    #[test]
    fn rust_skill_matches_rust_kind() {
        let s = skills_for("rust-cli", "hesabu");
        assert!(s.contains("Rust CLI"), "skill ya Rust lazima ichaguliwe");
    }

    #[test]
    fn node_skill_matches_website() {
        let s = skills_for("website", "duka la nguo");
        assert!(s.contains("Node.js Service") || s.contains("Frontend"));
    }

    #[test]
    fn imported_skills_are_loaded_and_listed() {
        // Skills zilizoingizwa (OpenHands/Claude Code/Trae) zinaruhusiwa kwenye
        // orodha, na index yao ipo kwenye diski.
        let listed = list();
        let skills = listed["skills"].as_array().cloned().unwrap_or_default();
        let imported: Vec<_> = skills
            .iter()
            .filter(|s| s["source"].as_str().map(|x| x != "engineering").unwrap_or(false))
            .collect();
        assert!(!imported.is_empty(), "skills zilizoingizwa lazima zionekane kwenye list()");
        assert!(imported.iter().all(|s| s["exists"].as_bool().unwrap_or(false)),
                "kila skill iliyoingizwa lazima faili lake liwepo");
    }

    #[test]
    fn imported_skill_matches_keywords() {
        // "mcp" inalingana na skill ya mcp-builder ya Claude Code.
        let s = skills_for("rust-cli", "jenga mcp server ya api");
        assert!(s.contains("MCP"), "skill ya MCP kutoka Claude Code lazima ichaguliwe");
    }
}
