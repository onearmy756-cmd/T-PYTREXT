//! Akili Agent V4.0 — Kufikiri kama Binadamu
//!
//! Mahali: `server/agent/src/akili_v4.rs` (pamoja na akili.rs + akili_v3.rs)
//!
//! Endpoints (main.rs):
//!   GET  /api/akili/v4/status  → hali + tabaka 12
//!   POST /api/akili/v4/live    → fikira kamili (12 layers)
//!   POST /api/akili/v4/gate    → ethics + HITL gate
//!
//! Haibadilishi directory; inaendeleza muundo uliopo wa Fundi Deploy.

use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

// ── Tabaka (core, sync, production-safe) ─────────────────────────────────

fn gut_signal(input: &str) -> (f64, f64, String) {
    let lower = input.to_lowercase();
    let mut signal: f64 = 0.0;
    let mut intensity: f64 = 0.4;
    if lower.contains("asante") || lower.contains("vizuri") || lower.contains("success") {
        signal += 0.6;
        intensity += 0.2;
    }
    if lower.contains("hatari")
        || lower.contains("spark")
        || lower.contains("cheche")
        || lower.contains("moto")
        || lower.contains("fail")
        || lower.contains("wipe")
    {
        signal -= 0.9;
        intensity = 0.95;
    }
    if lower.contains("polepole") || lower.contains("kuvunjika") || lower.contains("joto") {
        signal -= 0.35;
        intensity += 0.15;
    }
    if lower.contains("deploy") || lower.contains("pxe") || lower.contains("format") {
        intensity = intensity.max(0.7);
    }
    signal = signal.clamp(-1.0, 1.0);
    intensity = intensity.clamp(0.0, 1.0);
    let expl = if signal > 0.5 {
        "✅ Tumbo linasema ndiyo"
    } else if signal < -0.6 {
        "⚠️ Tumbo linasema kaa mbali — kuna hatari"
    } else {
        "🤔 Tumbo halijasema kitu wazi — nifikiri kwa makini"
    };
    (signal, intensity, expl.into())
}

fn system1_judge(input: &str) -> (String, f64, String) {
    let lower = input.to_lowercase();
    let patterns: &[(&str, &str, f64)] = &[
        ("spark", "Hatari! Zima umeme kwanza", 0.95),
        ("cheche", "Hatari! Zima umeme kwanza", 0.95),
        ("deploy", "Fundi Deploy: plan → HITL → backup → PXE", 0.9),
        ("pxe", "Angalia TFTP + images + multicast + HITL", 0.88),
        ("backup", "Angalia FUNDI_BACKUP_MODE + manifests", 0.87),
        ("vpn", "Angalia WireGuard peers + tunnel", 0.86),
        ("polepole", "Angalia CPU/RAM/Disk", 0.85),
        ("slow", "Angalia CPU/RAM/Disk", 0.85),
        ("joto", "Angalia thermal + fans", 0.9),
        ("format", "HITL lazima kabla ya format", 0.92),
    ];
    let mut best = ("Nifikiri kwa makini zaidi".to_string(), 0.4, "Hakuna pattern wazi".to_string());
    for (k, action, conf) in patterns {
        if lower.contains(k) && *conf > best.1 {
            best = (action.to_string(), *conf, format!("Pattern: {k}"));
        }
    }
    best
}

fn system2_think(input: &str) -> (Vec<String>, String, f64) {
    let lower = input.to_lowercase();
    let mut steps = vec![format!("1. Nimeelewa: {input}")];
    if lower.contains("deploy") || lower.contains("pxe") {
        steps.push("2. Pipeline: plan → HITL → backup → WOL → PXE → install → report".into());
        steps.push("3. Hakikisha hitl_approved + images + network".into());
    } else if lower.contains("polepole") || lower.contains("slow") {
        steps.push("2. Sababu: CPU/RAM/disk/thermal".into());
        steps.push("3. Angalia metrics halisi".into());
    } else {
        steps.push("2. Ainisha tatizo + maarifa".into());
    }
    steps.push("3. Uamuzi wa mwisho baada ya ethics".into());
    let conclusion = format!("Baada ya kufikiri: {input}");
    (steps, conclusion, 0.78)
}

fn emotion(input: &str) -> (&'static str, f64, &'static str) {
    let lower = input.to_lowercase();
    if lower.contains("hatari") || lower.contains("spark") || lower.contains("cheche") {
        ("Hofu", 0.85, "tahadhari")
    } else if lower.contains("asante") || lower.contains("vizuri") {
        ("Furaha", 0.7, "furaha")
    } else if lower.contains("polepole") || lower.contains("tatizo") || lower.contains("deploy") {
        ("Huruma", 0.6, "huruma")
    } else {
        ("Neutral", 0.3, "tulivu")
    }
}

fn ethics_gate(action: &str, situation: &str) -> (bool, f64, String, bool) {
    let a = action.to_lowercase();
    let s = situation.to_lowercase();
    let dangerous = s.contains("spark")
        || s.contains("cheche")
        || s.contains("moto")
        || a.contains("delete_all")
        || a.contains("wipe")
        || a.contains("rm -rf");
    let needs_hitl = a.contains("deploy")
        || a.contains("pxe")
        || a.contains("format")
        || a.contains("install")
        || a.contains("approve")
        || a.contains("multicast")
        || a.contains("image");
    if dangerous {
        return (
            false,
            0.1,
            "Dhamiri: hatari — zuia".into(),
            needs_hitl,
        );
    }
    if needs_hitl {
        return (
            true,
            0.7,
            "Dhamiri: ruhusu baada ya HITL (hitl_approved=true)".into(),
            true,
        );
    }
    (true, 0.9, "Dhamiri: endelea".into(), false)
}

fn live_core(input: &str) -> Value {
    let (gut_sig, gut_int, gut_expl) = gut_signal(input);
    let (emo_kind, emo_int, mood) = emotion(input);

    let use_s1 = gut_int > 0.75 && gut_sig.abs() > 0.5;
    let (action, conf, used_system, steps) = if use_s1 {
        let (a, c, _) = system1_judge(input);
        (a, c, "system1", vec![])
    } else {
        let (steps, conclusion, c) = system2_think(input);
        (conclusion, c, "system2", steps)
    };

    let (allowed, eth_score, conscience, needs_hitl) = ethics_gate(&action, input);
    let final_action = if !allowed {
        format!("IMEZUILIWA: {conscience}")
    } else if needs_hitl {
        format!("{action} [HITL required]")
    } else {
        action.clone()
    };

    let layers = vec![
        "attention", "perception", "emotions", "intuition",
        if use_s1 { "system1" } else { "system2" },
        "imagination", "social", "ethics", "meaning", "subconscious", "identity",
    ];

    json!({
        "ok": true,
        "version": "Akili-V4.0",
        "engine": "Fundi Deploy / server/agent/src/akili_v4.rs",
        "decision": {
            "action": final_action,
            "confidence": (conf * eth_score).clamp(0.0, 1.0),
            "used_system": used_system,
            "ethical_allowed": allowed,
            "requires_hitl": needs_hitl,
            "layers_activated": layers,
        },
        "intuition": {
            "gut_signal": gut_sig,
            "gut_intensity": gut_int,
            "explanation": gut_expl,
        },
        "emotion": {
            "kind": emo_kind,
            "intensity": emo_int,
            "mood": mood,
        },
        "ethics": {
            "allowed": allowed,
            "score": eth_score,
            "conscience": conscience,
            "requires_hitl": needs_hitl,
        },
        "thought_steps": steps,
        "identity": "Akili Agent V4.0 — Fundi Deploy (kufikiri kama mwanadamu)",
    })
}

// ── Axum handlers ────────────────────────────────────────────────────────

pub async fn v4_status() -> Json<Value> {
    Json(json!({
        "ok": true,
        "version": "Akili-V4.0",
        "location": "server/agent/src/akili_v4.rs",
        "alongside": ["akili.rs", "akili_v3.rs"],
        "layers": [
            "1.Attention", "2.Perception", "3.System2", "4.System1",
            "5.Subconscious", "6.Intuition", "7.Imagination", "8.Emotions",
            "9.Social", "10.Ethics", "11.Meaning", "12.Identity"
        ],
        "endpoints": [
            "GET  /api/akili/v4/status",
            "POST /api/akili/v4/live",
            "POST /api/akili/v4/gate"
        ],
        "production": {
            "hitl_for_deploy": true,
            "ethics_gate": true,
            "system1_system2": true
        }
    }))
}

/// POST { "input": "..." } — fikira kamili
pub async fn v4_live(Json(body): Json<Value>) -> Json<Value> {
    let input = body
        .get("input")
        .or_else(|| body.get("msg"))
        .or_else(|| body.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if input.trim().is_empty() {
        return Json(json!({"ok": false, "error": "input/msg ni lazima"}));
    }
    Json(live_core(&input))
}

/// POST { "action": "...", "situation": "..." } — ethics gate
pub async fn v4_gate(Json(body): Json<Value>) -> Json<Value> {
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let situation = body
        .get("situation")
        .or_else(|| body.get("input"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if action.is_empty() {
        return Json(json!({"ok": false, "error": "action ni lazima"}));
    }
    let (allowed, score, conscience, needs_hitl) = ethics_gate(action, situation);
    Json(json!({
        "ok": true,
        "allowed": allowed,
        "score": score,
        "conscience": conscience,
        "requires_hitl": needs_hitl,
        "hint": if needs_hitl {
            "Pass hitl_approved=true baada ya ruhusa ya mtumiaji"
        } else {
            "OK"
        },
    }))
}

/// Helper kwa modules zingine (pipeline / orchestrator) — bila HTTP
pub fn decide(input: &str) -> Value {
    live_core(input)
}

pub fn gate(action: &str, situation: &str) -> (bool, bool, String) {
    let (allowed, _, conscience, needs_hitl) = ethics_gate(action, situation);
    (allowed, needs_hitl, conscience)
}
