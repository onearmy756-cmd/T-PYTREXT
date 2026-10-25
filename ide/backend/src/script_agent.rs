//! script_agent.rs — AGENT MOJA: daraja moja linalounganisha AKILI (ubongo),
//! zana zote za mfumo (`agent_tools` zenye majina + `tool_calls` za IDE), na
//! familia zote za agents (`agents_unified`).
//!
//! KANUNI: hakuna zana inayovumbuliwa. Kila wito hupitia orodha iliyosajiliwa
//! (`agent_tools::TOOLS`) au engine ya IDE (`tool_calls::execute`), kisha
//! hufikia API HALISI ya MTECH OS kwa token ya Admin Mkuu (RBAC).
//!
//! Mtiririko (safu moja):
//!   1. AKILI V6 hupima maadili/HITL kwa tendo lolote hatari.
//!   2. `list` huzalisha katalogi MOJA: zana za Akili, zana za mfumo, zana za
//!      IDE, na familia zote za agents.
//!   3. `run` huchagua zana moja kwa jina na kuitisha kwa data HALISI.
//!   4. `agents` huendesha familia ya agents kwa parallel.

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::AppState;

/// Katalogi MOJA ya kila kitu kilichounganishwa (zana + agents + Akili).
pub async fn catalog(s: &AppState) -> Value {
    // Zana zenye majina (agent_tools) — orodha pekee inayoruhusiwa.
    let named: Vec<Value> = crate::agent_tools::TOOLS
        .iter()
        .map(|t| {
            json!({
                "name": t.name, "desc": t.desc_sw,
                "method": t.method, "path": t.path,
                "params": t.params.iter().map(|p| json!({
                    "name": p.name, "type": p.ty, "required": p.required, "desc": p.desc_sw,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    // Zana za IDE (tool_calls / zed_tool_calls.json).
    let ide = crate::tool_calls::groups_summary();

    // Familia zote za agents.
    let agents = crate::agents_unified::catalog(s).await;

    let akili = json!({
        "engine": "Akili V6 (tabaka 200) + V3 (tabaka 7) + V4",
        "endpoints": [
            "/api/akili/status", "/api/akili/remember", "/api/akili/recall",
            "/api/akili/knowledge", "/api/akili/verify", "/api/akili/plan",
            "/api/akili/tot", "/api/akili/run", "/api/akili/reflect",
            "/api/akili/patterns/apply", "/api/akili/v6/live", "/api/akili/v6/gate",
            "/api/akili/v3/solve",
        ],
        "role_sw": "Ubongo wa mfumo: kumbukumbu, maarifa, mipango, maadili/HITL, uthibitisho halisi.",
    });

    json!({
        "ok": true,
        "agent": "MTECHOS-ONE",
        "note_sw": "Agent MOJA: Akili (ubongo) + zana zote za mfumo + zana za IDE + familia zote za agents.",
        "akili": akili,
        "named_tools": { "count": named.len(), "tools": named },
        "ide_tools": ide,
        "agents": {
            "total": agents["total_agents"],
            "families": agents["families"],
        },
    })
}

/// Endesha zana MOJA kwa jina (zana yenye jina au zana ya IDE) — data HALISI.
pub async fn run_tool(s: &AppState, name: &str, params: &Value) -> Value {
    // (a) Zana zenye majina (agent_tools) → API halisi ya mfumo.
    if crate::agent_tools::find(name).is_some() {
        let call = match crate::agent_tools::resolve(name, params) {
            Ok(c) => c,
            Err(e) => return json!({ "ok": false, "source": "named", "error": e }),
        };
        // Ulinzi wa ziada: sehemu nyeti zimefungwa.
        const BLOCKED: &[&str] = &[
            "/api/ai/key", "/api/portal/", "/api/admin/", "/api/clickpesa/", "/api/setup/",
        ];
        if BLOCKED.iter().any(|b| call.path.contains(b)) {
            return json!({ "ok": false, "error": "zana imefungwa kwa usalama (idhini ya admin inahitajika)" });
        }
        let client = reqwest::Client::new();
        let url = format!("{}{}", crate::chat::self_base_pub(), call.path);
        let m: reqwest::Method = call.method.parse().unwrap_or(reqwest::Method::GET);
        let mut rb = client.request(m, url.as_str()).timeout(std::time::Duration::from_secs(30));
        if let Some(t) = crate::chat::agent_token() {
            rb = rb.header("Authorization", format!("Bearer {t}"));
        }
        if call.method == "POST" {
            if let Some(b) = &call.body {
                rb = rb.json(b);
            }
        }
        return match rb.send().await {
            Ok(r) => {
                let st = r.status().as_u16();
                let txt = r.text().await.unwrap_or_default();
                let body = serde_json::from_str::<Value>(&txt).unwrap_or(json!(txt));
                json!({ "ok": st < 400, "source": "named", "tool": name, "status": st, "result": body })
            }
            Err(e) => json!({ "ok": false, "source": "named", "tool": name, "error": e.to_string() }),
        };
    }

    // (b) Zana za IDE (tool_calls) → engine halisi ya workspace.
    if crate::tool_calls::schema_for(name)["ok"] == true {
        let r = crate::tool_calls::execute(s, name, params.clone()).await;
        return json!({ "ok": r["ok"], "source": "ide", "tool": name, "result": r });
    }

    json!({
        "ok": false,
        "error": format!("zana '{name}' haipo kwenye safu moja"),
        "named_tools": crate::agent_tools::TOOLS.iter().map(|t| t.name).collect::<Vec<_>>(),
    })
}

/// Endesha familia ya agents (swarm/ts/social/calls/…/blockchain) kwa parallel.
pub async fn run_agents(s: &AppState, body: &Value) -> Value {
    let mut v = crate::agents_unified::run_many(s, body).await;
    v["bridge"] = json!("agents_unified");
    v
}

// ------------------------------- HTTP -------------------------------

pub async fn catalog_h(State(s): State<AppState>) -> Json<Value> {
    Json(catalog(&s).await)
}

pub async fn run_h(State(s): State<AppState>, Json(b): Json<Value>) -> Json<Value> {
    let name = b["tool"].as_str().unwrap_or("").to_string();
    if name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "hoja 'tool' ni lazima" }));
    }
    Json(run_tool(&s, &name, &b["params"]).await)
}

pub async fn agents_h(State(s): State<AppState>, Json(b): Json<Value>) -> Json<Value> {
    Json(run_agents(&s, &b).await)
}

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/agent/catalog", get(catalog_h))
        .route("/api/agent/tool", post(run_h))
        .route("/api/agent/agents", post(agents_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zana_za_akili_zimeunganishwa() {
        // Zana zote za Akili ziwe kwenye registry moja.
        for n in [
            "akili_remember", "akili_recall", "akili_verify", "akili_plan",
            "akili_tot", "akili_pipeline", "akili_reflect", "akili_pattern",
            "akili_v6_think", "akili_v6_gate", "akili_v3_solve",
        ] {
            assert!(crate::agent_tools::find(n).is_some(), "zana '{n}' haipo");
        }
        // Agents unified ziwe kwenye registry.
        assert!(crate::agent_tools::find("agents_unified_run").is_some());
        assert!(crate::agent_tools::find("agents_unified_catalog").is_some());
    }

    #[test]
    fn registry_moja_haina_majina_yanayorudia() {
        let mut seen = std::collections::BTreeSet::new();
        for t in crate::agent_tools::TOOLS {
            assert!(seen.insert(t.name), "zana '{}' imerudiwa", t.name);
        }
        // Zana za Akili + agents + zile za awali zote zimo.
        assert!(crate::agent_tools::TOOLS.len() >= 68);
    }
}
