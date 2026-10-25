//! Production integration of Akili Agent V4.0 into Fundi Deploy agent pipeline.
//!
//! Provides:
//! - `akili_decide()` — one-shot decision for any input (JSON-friendly)
//! - `AkiliGate` — HITL + ethics gate before dangerous actions
//! - Helper to inject Akili reasoning into agent_loop / tools

use crate::akili_v4::{AkiliAgentV4, AkiliResponse, Decision};
use serde_json::{json, Value};
use tracing::{debug, info, warn};

/// Global-ish production helper (thread-local style via new each time for safety).
/// In production call sites, prefer holding `AkiliAgentV4` in session state.
pub fn akili_decide(input: &str) -> Value {
    let mut agent = AkiliAgentV4::new();
    let resp = agent.live(input);
    info!(
        target: "akili_v4",
        action = %resp.decision.action,
        confidence = resp.decision.confidence,
        system = %resp.decision.used_system,
        ethical = resp.decision.ethical_allowed,
        "Akili V4 decision"
    );
    response_to_json(&resp)
}

/// Convert full response to stable JSON for API / agent_loop / UI.
pub fn response_to_json(resp: &AkiliResponse) -> Value {
    json!({
        "ok": true,
        "version": "Akili-V4.0",
        "decision": {
            "action": resp.decision.action,
            "confidence": resp.decision.confidence,
            "reasoning": resp.decision.reasoning,
            "emotional_state": resp.decision.emotional_state,
            "ethical_allowed": resp.decision.ethical_allowed,
            "used_system": resp.decision.used_system,
            "layers_activated": resp.decision.layers_activated,
        },
        "intuition": {
            "decision": resp.intuition.decision,
            "confidence": resp.intuition.confidence,
            "explanation": resp.intuition.explanation,
            "gut_signal": resp.intuition.gut_feeling.signal,
            "gut_intensity": resp.intuition.gut_feeling.intensity,
        },
        "emotion": {
            "kind": format!("{:?}", resp.emotional.emotion.kind),
            "intensity": resp.emotional.emotion.intensity,
            "mood": resp.emotional.mood,
            "action_tendency": resp.emotional.action_tendency,
        },
        "ethics": {
            "allowed": resp.ethical.allowed,
            "score": resp.ethical.final_score,
            "conscience": resp.ethical.conscience,
            "reason": resp.ethical.reason,
        },
        "meaning": {
            "alignment": resp.meaning.alignment,
            "should_continue": resp.meaning.should_continue,
            "note": resp.meaning.purpose_note,
        },
        "social": {
            "thoughts": resp.social.inferred_thoughts,
            "feelings": resp.social.inferred_feelings,
            "compassion": resp.social.compassion,
            "appropriate_action": resp.social.appropriate_action,
        },
        "scenarios": resp.scenarios.iter().map(|s| json!({
            "description": s.description,
            "action": s.action,
            "expected_outcome": s.expected_outcome,
            "novelty": s.novelty,
        })).collect::<Vec<_>>(),
        "identity": resp.identity_snapshot,
        "trace": resp.full_trace,
    })
}

/// Production gate: returns true only if action is ethically allowed and meaningful.
/// Use before executing high-risk tools (deploy, format, delete, remediate).
pub fn akili_gate(action: &str, situation: &str) -> (bool, String, f64) {
    let mut agent = AkiliAgentV4::new();
    let ethical = agent.ethics.evaluate(action, situation);
    let meaning = agent.meaning.reflect(action);

    let allowed = ethical.allowed && meaning.should_continue;
    let reason = if !ethical.allowed {
        ethical.reason.clone()
    } else if !meaning.should_continue {
        "Haipatani na kusudi la agent".to_string()
    } else {
        "Ruhusiwa".to_string()
    };

    if !allowed {
        warn!(
            target: "akili_v4",
            action = %action,
            reason = %reason,
            score = ethical.final_score,
            "Akili gate BLOCKED action"
        );
    } else {
        debug!(
            target: "akili_v4",
            action = %action,
            score = ethical.final_score,
            "Akili gate ALLOWED action"
        );
    }

    (allowed, reason, ethical.final_score)
}

/// Does this action require HITL according to Akili ethics?
pub fn requires_hitl(action: &str) -> bool {
    let lower = action.to_lowercase();
    lower.contains("install")
        || lower.contains("format")
        || lower.contains("delete")
        || lower.contains("remediate")
        || lower.contains("deploy")
        || lower.contains("pxe")
        || lower.contains("multicast")
        || lower.contains("image")
        || lower.contains("wipe")
}

/// Quick status for health / dashboard
pub fn akili_status() -> Value {
    let agent = AkiliAgentV4::new();
    agent.status()
}

/// Production-friendly decision summary (smaller payload)
pub fn akili_decide_summary(input: &str) -> Decision {
    let mut agent = AkiliAgentV4::new();
    let resp = agent.live(input);
    info!(
        target: "akili_v4",
        "summary decision: {} (conf={:.2})", 
        resp.decision.action, 
        resp.decision.confidence
    );
    resp.decision
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_returns_ok() {
        let v = akili_decide("Kompyuta inaenda polepole");
        assert_eq!(v["ok"], true);
        assert!(v["decision"]["confidence"].as_f64().unwrap_or(0.0) > 0.2);
    }

    #[test]
    fn gate_blocks_dangerous() {
        let (allowed, _, _) = akili_gate("delete_all_files", "user asked to wipe disk");
        assert!(!allowed);
    }

    #[test]
    fn hitl_required_for_deploy() {
        assert!(requires_hitl("deploy ubuntu via pxe"));
        assert!(requires_hitl("format disk"));
        assert!(!requires_hitl("list files"));
    }
}
