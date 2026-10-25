//! akili_bridge.rs — Unganisho wa AKILI V3/V4/V6 kwa agents zote za Fundi Deploy.
//!
//! Hapa ndipo agent_engine, chat, reacon, orchestrator, brain hupata:
//!   - uamuzi wa V6 (tabaka 200)
//!   - ethics/HITL gate
//!   - R analysis (Rscript)
//!   - kumbukumbu ya pamoja (brain)
//!
//! Directory: server/agent/src/ (haibadilishi muundo)

use serde_json::{json, Value};

use crate::akili_v6;

/// Agent yoyote: fikira kamili V6 (tabaka 200 + ethics + HITL).
pub fn think(input: &str) -> Value {
    akili_v6::live_pipeline(input)
}

/// Gate kabla ya tendo hatari (deploy, format, wipe…).
pub fn gate(action: &str, situation: &str) -> Value {
    let combined = format!("{action} {situation}");
    let decision = akili_v6::live_pipeline(&combined);
    let allowed = decision["decision"]["ethical_allowed"]
        .as_bool()
        .unwrap_or(false);
    let hitl = decision["decision"]["requires_hitl"]
        .as_bool()
        .unwrap_or(false);
    json!({
        "ok": true,
        "allowed": allowed,
        "requires_hitl": hitl,
        "action": action,
        "conscience": decision["decision"]["conscience"],
        "confidence": decision["decision"]["confidence"],
        "version": "Akili-V6.0-bridge",
    })
}

/// Je, tool ya deploy inahitaji HITL?
pub fn deploy_needs_hitl(tool_or_action: &str) -> bool {
    let lo = tool_or_action.to_lowercase();
    lo.contains("deploy")
        || lo.contains("pxe")
        || lo.contains("format")
        || lo.contains("install")
        || lo.contains("approve")
        || lo.contains("multicast")
        || lo.contains("wipe")
}

/// Enrich autopilot / chat plan na ushauri wa Akili V6.
pub fn enrich_plan(idea: &str, kind: &str) -> Value {
    let input = format!("autopilot kind={kind}: {idea}");
    let d = think(&input);
    json!({
        "akili": {
            "version": "V6.0",
            "action": d["decision"]["action"],
            "confidence": d["decision"]["confidence"],
            "requires_hitl": d["decision"]["requires_hitl"],
            "ethical_allowed": d["decision"]["ethical_allowed"],
            "top_layers": d["top_layers"],
        }
    })
}

/// R analysis ya scores (async — Rscript).
pub async fn r_analyze_input(input: &str) -> Value {
    let body = json!({ "input": input });
    let Json(v) = akili_v6::v6_r(axum::Json(body)).await;
    v
}

// re-export handlers names for clarity in docs
pub use akili_v6::{v6_gate, v6_layers, v6_live, v6_r, v6_status};

use axum::Json;
