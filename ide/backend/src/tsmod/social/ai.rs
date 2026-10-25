//! ai.rs — LLM HALISI + AGENT ya kujitegemea kwa usalama wa kijamii.
//!
//! Data pekee ya keyword (rules) haitoshelezi: majambazi hubadilisha maneno,
//! lugha, na mbinu. Moduli hii inaongeza tabaka la AKILI:
//!
//!  1. `analyze` — LLM hupima maana (semantic) ya maandishi/URL, si maneno
//!     pekee, na hurudisha uamuzi + sababu + mbinu mpya zilizogundulika.
//!  2. `propose_rules` — LLM hutoa SHERIA MPYA (patterns + kind) kutoka
//!     mifano halisi; mtu huzikagua kisha kuziweka (`store::add_rule`).
//!  3. `synthesize` — LLM huunganisha ripoti ya binadamu inayosomeka.
//!
//! Kila kitu ni FAIL-SAFE: LLM ikikosekana, injini ya DB (engine.rs) inaendelea.

use crate::ai_config;
use crate::tsmod::store;
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Changanua kwa LLM. Hurudisha None kama LLM haipatikani.
pub async fn analyze(
    db: &SqlitePool,
    platform: &str,
    category: &str,
    content: &str,
) -> Option<Value> {
    let cfg = ai_config::load_with_db(db).await;
    let prompt = format!(
        "Wewe ni mtaalamu wa usalama wa mitandao ya kijamii (Total Security, Mbilinyi Tech).\n\
         Jukwaa: {platform}. Aina ya hatari: {category}.\n\
         Changanua maudhui yafuatayo kwa KINA (maana, sio maneno pekee). Jibu kwa JSON TUPU pekee:\n\
         {{\"score\": <0-100>, \"decision\": \"allow|warn|block\", \"category\": \"<scam|phishing|threat|abuse|fraud|spam|safe>\", \
         \"reasons\": [\"...\"], \"new_patterns\": [\"<maneno/mbinu mpya za kuzuia>\"], \"language\": \"<sw|en|...>\"}}\n\n\
         Maudhui:\n{content}"
    );
    let raw = ai_config::call_llm(&cfg, &prompt).await?;
    parse_ai_json(&raw)
}

/// Toa sheria mpya kutoka mifano (LLM). Hurudisha list ya (kind, pattern, severity).
pub async fn propose_rules(
    db: &SqlitePool,
    platform: &str,
    samples: &[String],
) -> Option<Vec<(String, String, u8)>> {
    let cfg = ai_config::load_with_db(db).await;
    let joined = samples.join("\n---\n");
    let prompt = format!(
        "Wewe ni mtaalamu wa usalama. Kutoka mifano ya maudhui ya kijamii hapa chini \
         (platform {platform}), toa SHERIA MPYA za kugundua hatari (scam/phishing/threat/fraud/spam).\n\
         Sheria ziwe FUPI, za jumla (zisinase mfano mmoja tu), na ziwe za Kiswahili au Kiingereza.\n\
         Jibu kwa JSON TUPU: {{\"rules\": [{{\"kind\": \"scam\", \"pattern\": \"tuma pesa\", \"severity\": 80}}]}}\n\n\
         Mifano:\n{joined}"
    );
    let raw = ai_config::call_llm(&cfg, &prompt).await?;
    let v = parse_ai_json(&raw)?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("rules").and_then(|x| x.as_array()) {
        for r in arr {
            let kind = r.get("kind").and_then(|x| x.as_str()).unwrap_or("scam").to_string();
            let pattern = r.get("pattern").and_then(|x| x.as_str()).unwrap_or("").trim().to_lowercase();
            let severity = r.get("severity").and_then(|x| x.as_u64()).unwrap_or(70).min(100) as u8;
            if !pattern.is_empty() {
                out.push((kind, pattern, severity));
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Andika ripoti ya binadamu (LLM) kutoka event + signals.
pub async fn synthesize(db: &SqlitePool, platform: &str, event: &Value) -> Option<String> {
    let cfg = ai_config::load_with_db(db).await;
    let prompt = format!(
        "Wewe ni mchanganuzi wa usalama. Andika muhtasari MWAFUPI (Kiswahili, sentensi 2-3) wa tukio hili \
         la kijamii kwa {platform}, ukieleza hatari na hatua inayopendekezwa:\n\n{event}"
    );
    ai_config::call_llm(&cfg, &prompt).await
}

/// Changanua JSON kutoka jibu la LLM (huvumilia ```json ... ``` na kelele).
pub fn parse_ai_json(raw: &str) -> Option<Value> {
    let t = raw.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return Some(v);
    }
    // Vua code fence.
    let stripped = t
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(v) = serde_json::from_str::<Value>(stripped) {
        return Some(v);
    }
    // Tafuta { ... } ya kwanza.
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    if end > start {
        if let Ok(v) = serde_json::from_str::<Value>(&t[start..=end]) {
            return Some(v);
        }
    }
    None
}

/// AGENT ya kujitegemea: changanua maudhui, kisha JIFUNZE (ongeza sheria mpya
/// kwenye DB) kwa hiari ya `learn`. Hurudisha ripoti kamili.
pub async fn autonomous_scan(
    db: &SqlitePool,
    platform: &str,
    category: &str,
    content: &str,
    learn: bool,
) -> Value {
    let mut out = json!({ "ok": true, "platform": platform, "engine": "llm+db" });
    match analyze(db, platform, category, content).await {
        Some(v) => {
            out["ai"] = v.clone();
            if learn {
                if let Some(pats) = v.get("new_patterns").and_then(|x| x.as_array()) {
                    let mut added = Vec::new();
                    for p in pats {
                        if let Some(s) = p.as_str() {
                            let pat = s.trim().to_lowercase();
                            if !pat.is_empty()
                                && store::add_rule(db, platform, "content", "ai_learned", &pat, 70, "llm")
                                    .await
                                    .is_ok()
                            {
                                added.push(pat);
                            }
                        }
                    }
                    out["learned_rules"] = json!(added);
                }
            }
        }
        None => {
            out["ai"] = json!(null);
            out["note"] = json!("LLM (semantic) haipatikani — caller hutumia injini ya DB (sheria + jukwaa) kwa uamuzi.");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_plain_na_fence() {
        let a = parse_ai_json(r#"{"score":90,"decision":"block"}"#).unwrap();
        assert_eq!(a["score"], 90);
        let b = parse_ai_json("```json\n{\"score\":10}\n```").unwrap();
        assert_eq!(b["score"], 10);
        let c = parse_ai_json("Hapa ni jibu: {\"score\":50} ndiyo").unwrap();
        assert_eq!(c["score"], 50);
        assert!(parse_ai_json("hakuna json").is_none());
    }
}
