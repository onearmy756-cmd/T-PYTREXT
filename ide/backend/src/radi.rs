//! radi.rs — RADI: edition ya kasi ya umeme (lightning-fast engineer).
//!
//! RADI ni **tabia** inayoingizwa kwenye system prompt ya kila uso wa coding
//! pamoja na 2NYI. Chanzo kimoja cha ukweli ni markdown data-driven chini ya
//! `data/skills/radi/` (`index.json` hueleza skill ipi ni ya kila wakati).
//!
//! MUHIMU: RADI HAIPUNGUZI ukweli wala usalama. Huondoa hatua zisizo na thamani
//! (kusoma kupita kiasi, kurudia, kueleza kila hatua) na hupima mara MOJA kwa
//! command kamili. Kubadilisha RADI HAKUHITAJI kufundisha modeli upya.

use serde_json::Value;
use std::path::PathBuf;

/// Jina la persona — linatumika kwenye prompts na UI.
pub const PERSONA: &str = "RADI";

fn skills_dir() -> PathBuf {
    let candidates = [
        crate::data_io::data_dir().join("skills").join("radi"),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/skills/radi")
            .to_path_buf(),
        PathBuf::from("data/skills/radi"),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| crate::data_io::data_dir().join("skills").join("radi"))
}

fn read_index(dir: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["skills"].as_array().cloned())
        .unwrap_or_default()
}

/// Block KAMILI ya RADI (skills zote za `always`) — inaingizwa kwenye system prompt.
pub fn block() -> String {
    let dir = skills_dir();
    let mut out = String::new();
    for entry in read_index(&dir) {
        if entry["always"].as_bool().unwrap_or(false) {
            if let Some(file) = entry["file"].as_str() {
                if let Ok(body) = std::fs::read_to_string(dir.join(file)) {
                    out.push_str(&body);
                    out.push_str("\n\n");
                }
            }
        }
    }
    out
}

/// Maelekezo MAFUPI ya RADI — kwa prompts ambazo hazihitaji block kamili.
pub fn directive() -> &'static str {
    "Wewe ni RADI — mhandisi wa kasi ya umeme (lightning-fast). KANUNI: (1) soma kidogo \
     (grep/glob, vipande), andika kidogo (faili moja/diff ndogo); (2) fanya kazi nyingi \
     kwa hatua MOJA (zana huru pamoja); (3) pima mara MOJA kwa command KAMILI ya uthibitisho \
     (test/build/lint) na unukuu matokeo HALISI; (4) usirudie command/faili — kurudia bila \
     maendeleo ni kosa; (5) maliza kwa artifact inayoendesha, sio mpango. UKWELI na USALAMA \
     HAVIPUNGUI: usibuni API/namba/tokeo, na HITL/scope-gate zinabaki."
}

/// Uwezo wa RADI (kwa UI/uchunguzi) — husoma `capabilities` kwenye index.json.
pub fn capabilities() -> Value {
    let dir = skills_dir();
    std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| v["capabilities"].clone())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// Orodha ya skills za RADI (kwa UI/uchunguzi).
pub fn list() -> Value {
    let dir = skills_dir();
    let mut items = Vec::new();
    for entry in read_index(&dir) {
        if let Some(file) = entry["file"].as_str() {
            items.push(serde_json::json!({
                "file": file,
                "always": entry["always"].as_bool().unwrap_or(false),
                "role": entry["role"].clone(),
                "exists": dir.join(file).is_file(),
            }));
        }
    }
    serde_json::json!({
        "ok": true,
        "persona": PERSONA,
        "dir": dir.display().to_string(),
        "count": items.len(),
        "skills": items,
        "capabilities": capabilities(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_ina_identity_na_fast_loop() {
        let b = block();
        assert!(b.contains("RADI"), "identity ya RADI lazima iwepo");
        assert!(b.contains("Mkataba wa kasi"), "mkataba wa kasi lazima uwepo");
        assert!(b.contains("anti-loop") || b.contains("Kuzuia kurudia"),
                "kanuni ya kuzuia kurudia lazima iwepo");
    }

    #[test]
    fn directive_inataja_uthibitisho_na_usalama() {
        let d = directive();
        assert!(d.contains("uthibitisho"), "directive lazima itaje uthibitisho");
        assert!(d.contains("USALAMA") || d.contains("HITL"), "usalama lazima utajwe");
    }

    #[test]
    fn list_inarudisha_skills_zote_zilizopo() {
        let v = list();
        let arr = v["skills"].as_array().cloned().unwrap_or_default();
        assert!(!arr.is_empty(), "skills za RADI lazima zionekane");
        assert!(arr.iter().all(|s| s["exists"].as_bool().unwrap_or(false)),
                "kila skill ya RADI lazima faili lake liwepo");
    }
}
