// tools_catalog.rs — MBILINYITECH IDE: katalogi ya ZED-level backend tools
// (20 makundi, 100+ tools: agent frameworks, web, LLM, db, storage, protocols,
// sandbox, queues, auth, gateway, websocket, observability, caching, tasks,
// notifications, payments, search, devops, iac, cli agents, codegen models).
// Data: data/knowledge/zed_tools_catalog.json
use serde_json::{json, Value};
use std::path::Path;

/// Load katalogi JSON.
pub fn load() -> Value {
    let p = crate::data_io::data_dir()
        .join("knowledge")
        .join("zed_tools_catalog.json");
    if !Path::new(&p).exists() {
        return json!({ "ok": false, "error": "zed_tools_catalog.json haipatikani" });
    }
    let Ok(t) = std::fs::read_to_string(&p) else {
        return json!({ "ok": false, "error": "read fail" });
    };
    serde_json::from_str(&t).unwrap_or(json!({ "ok": false, "error": "json parse fail" }))
}

/// Muhtasari: makundi yote + idadi ya tools (kwa UI).
pub fn summary() -> Value {
    let data = load();
    let groups = &data["groups"];
    let mut out: Vec<Value> = Vec::new();
    let mut total = 0usize;
    if let Some(m) = groups.as_object() {
        for (k, v) in m {
            let count = v["tools"].as_array().map(|a| a.len()).unwrap_or(0);
            total += count;
            out.push(json!({
                "id": k,
                "label": v["label_sw"],
                "icon": v["icon"],
                "count": count,
            }));
        }
    }
    json!({ "ok": true, "total_tools": total, "total_groups": out.len(), "groups": out })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_repo_data_dir() {
        let cand = [
            std::path::PathBuf::from("data/knowledge/zed_tools_catalog.json"),
            std::path::PathBuf::from("../../data/knowledge/zed_tools_catalog.json"),
        ];
        for c in &cand {
            if c.exists() {
                std::env::set_var("FUNDI_DATA", c.parent().unwrap().parent().unwrap());
                return;
            }
        }
    }

    #[test]
    fn test_summary_ina_makundi_yote() {
        with_repo_data_dir();
        let s = summary();
        assert_eq!(s["ok"], json!(true));
        assert!(s["total_groups"].as_u64().unwrap() >= 15);
        assert!(s["total_tools"].as_u64().unwrap() >= 80);
    }

    #[test]
    fn test_load_website_framework_ina_axum() {
        with_repo_data_dir();
        let d = load();
        let wf = &d["groups"]["web_framework"]["tools"];
        assert!(wf.as_array().map(|a| a.iter().any(|t| t["jina"] == "Axum")).unwrap_or(false));
    }
}
