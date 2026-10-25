// deploy_templates.rs — MBILINYITECH IDE: PLAN TEMPLATES za deployment.
//
// Data: data/knowledge/deploy_templates.json (4 domains: website/mobile/iot/gaming).
// Agent + UI wanaita: chagua domain → plan kamili yenye steps, commands, gharama.
// (Kama mwongozo wa ZED AI: PLAN → CODE → TEST → BUILD → DEPLOY → MONITOR)
use serde_json::{json, Value};
use std::path::Path;

/// Load templates JSON (data/knowledge/deploy_templates.json).
pub fn load() -> Value {
    let p = crate::data_io::data_dir()
        .join("knowledge")
        .join("deploy_templates.json");
    if !Path::new(&p).exists() {
        return json!({ "ok": false, "error": "deploy_templates.json haipatikani" });
    }
    let Ok(t) = std::fs::read_to_string(&p) else {
        return json!({ "ok": false, "error": "read fail" });
    };
    serde_json::from_str(&t).unwrap_or(json!({ "ok": false, "error": "json parse fail" }))
}

/// LIST domains zote (kwa UI dropdown /agent).
pub fn list_domains() -> Value {
    let data = load();
    let domains = &data["domains"];
    let mut out: Vec<Value> = Vec::new();
    if let Some(map) = domains.as_object() {
        for (k, v) in map {
            out.push(json!({
                "id": k,
                "label": v["label_sw"].as_str().unwrap_or(k),
                "ugumu": v["ugumu"],
                "gharama": v["gharama"],
            }));
        }
    }
    json!({ "ok": true, "domains": out })
}

/// PLAN kamili kwa domain moja (steps + commands + framework options).
pub fn plan_for(domain: &str) -> Value {
    let data = load();
    let d = &data["domains"][domain];
    if d.is_null() {
        return json!({
            "ok": false,
            "error": format!("domain '{}' haipatikani (website/mobile/iot/gaming)", domain),
        });
    }
    // STACK OPTIONS: frameworks ama stacks ama hardware (kulingana na domain)
    let stack_key = ["website", "mobile", "iot", "gaming"]
        .iter()
        .find_map(|k| {
            let keys = match *k {
                "website" => "stacks",
                "mobile" => "frameworks",
                "iot" => "hardware",
                "gaming" => "engines",
                _ => "",
            };
            if *k == domain { Some(keys) } else { None }
        })
        .unwrap_or("stacks");
    let stack_options = &d[stack_key];
    json!({
        "ok": true,
        "domain": domain,
        "label": d["label_sw"],
        "ugumu": d["ugumu"],
        "gharama": d["gharama"],
        "options_key": stack_key,
        "options": stack_options,
        "steps": d["steps"],
    })
}

/// PLAN KAMILI (text form) kwa agent — inarudisha maandishi mtupu
/// (kk wakati mwingine agent inafikia tool: "deploy" → plan)
pub fn plan_text(domain: &str) -> String {
    let p = plan_for(domain);
    if p["ok"].as_bool() != Some(true) {
        return format!("⚠️ {}", p["error"].as_str().unwrap_or("template haipatikani"));
    }
    let mut out = String::new();
    out.push_str(&format!(
        "📋 PLAN — {} ({} · gharama {})\n",
        p["label"].as_str().unwrap_or(domain),
        p["ugumu"].as_str().unwrap_or(""),
        p["gharama"].as_str().unwrap_or("")
    ));
    if let Some(arr) = p["steps"].as_array() {
        for (i, s) in arr.iter().enumerate() {
            let cmd = s["amri"].as_str().unwrap_or("");
            out.push_str(&format!(
                "{}. {}: {}\n",
                i + 1,
                s["hatua"].as_str().unwrap_or("?"),
                s["kazi_sw"].as_str().unwrap_or("?")
            ));
            if !cmd.is_empty() {
                out.push_str(&format!("     → `{}`\n", cmd));
            }
        }
    }
    if let Some(arr) = p["options"].as_array() {
        out.push_str("\nStack/framework chaguo:\n");
        for o in arr.iter() {
            let name = o["jina"]
                .as_str()
                .or_else(|| o["board"].as_str())
                .unwrap_or("?");
            let tech = o["tech"]
                .as_str()
                .or_else(|| o["lugha"].as_str())
                .unwrap_or("");
            let best = o["bora_kwa"].as_str().unwrap_or("");
            out.push_str(&format!("  - {name} ({tech}) — bora: {best}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sandbox ya test haina /app/data — tunaonyesha workspace ya tests kwa data/
    /// halisi (repo root) kwa kutumia env var kwenye kila test.
    fn with_repo_data_dir() {
        // data/ relative kwa working dir ya cargo test (server/agent) au repo root.
        // FUNDI_DATA ya env siyesha valikot; kama haipo, baada ya cwd/data kama fallback.
        let cand = [
            std::path::PathBuf::from("data/knowledge/deploy_templates.json"),
            std::path::PathBuf::from("../../data/knowledge/deploy_templates.json"),
            std::path::PathBuf::from("/home/daytona/codebase/data/knowledge/deploy_templates.json"),
        ];
        for c in &cand {
            if c.exists() {
                std::env::set_var("FUNDI_DATA", c.parent().unwrap().parent().unwrap());
                return;
            }
        }
    }

    #[test]
    fn test_load_letakata_websites_mwalimu() {
        with_repo_data_dir();
        let v = load();
        let has_domains = v["domains"]["website"].is_object() || v["domains"]["mobile"].is_object();
        assert!(has_domains || v["ok"] == json!(false));
    }

    #[test]
    fn test_plan_for_website_ina_steps() {
        with_repo_data_dir();
        let v = plan_for("website");
        assert_eq!(v["ok"], json!(true), "{}", v);
        assert!(v["steps"].as_array().unwrap().len() >= 4);
    }

    #[test]
    fn test_plan_for_mobile_ina_frameworks() {
        with_repo_data_dir();
        let v = plan_for("mobile");
        assert_eq!(v["ok"], json!(true), "{}", v);
        assert_eq!(v["options_key"], json!("frameworks"));
    }

    #[test]
    fn test_plan_for_unknown_domain() {
        with_repo_data_dir();
        let v = plan_for("xyz");
        assert_eq!(v["ok"], json!(false));
    }

    #[test]
    fn test_plan_text_stderr() {
        with_repo_data_dir();
        let text = plan_text("website");
        assert!(text.contains("PLAN — "), "{}", text);
    }
}
