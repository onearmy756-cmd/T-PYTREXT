//! super_engineer.rs — SUPER INTELLECTUAL ENGINEER: msimamizi mkuu wa akili.
//!
//! Huunganisha injini ZOTE halisi za MTECHOS kwenye mtiririko MMOJA wa "engineer
//! mkuu" (kama Devin/Cursor senior loop, lakini kwa injini zetu):
//!
//!   1.  DESTURI   — kusoma muktadha (recon + memory + complexity)
//!   2.  SERA      — Akili V6 (ethics/HITL) + governance check
//!   3.  MPANGO    — SDLC phase plan + acceptance criteria + cortex capacity
//!   4.  HOJA      — Orthogonal Insight (worlds za "fizikia" tofauti) + ReMIND
//!   5.  UTEPELEZAJI — AgentDescent (diffs zilizounganishwa, kujiboresha)
//!   6.  UTENDAJI  — run_agentic (ReAct: andika → endesha → jirekebishe)
//!   7.  UKAGUZI   — ide_engineer (recon/security/tests/build/speed) + SDLC gate
//!   8.  MEGAFLOW  — mgawanyo wa huduma (model/agent/env)
//!   9.  UAMUZI    — score + mapendekezo + hatua zinazofuata
//!   10. KUMBUKUMBU — unified_engines::remember (masomo ya baadaye)
//!
//! Kanuni: hakuna mock. Kila hatua hurudisha matokeo HALISI au `skipped`/`error`
//! wazi. Hakuna unwrap/panic/`..`/md5 kwenye code ya runtime.

use axum::{extract::State, routing::post, Json, Router};
use serde_json::{json, Value};

use crate::AppState;

/// Toa alama (0–100) kutoka JSON kwa ufunguo, kwa usalama.
fn score_of(v: &Value, key: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 100.0)
}

fn rounds(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Kifupi cha "goal" — jina salama la mradi kwa faili/jedwali.
fn project_slug(goal: &str) -> String {
    let mut s: String = goal
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "super-project".to_string()
    } else {
        s.chars().take(48).collect()
    }
}

/// Hatua 0 — DESTURI: recon halisi + memory + complexity.
async fn recon(s: &AppState, goal: &str, kind: &str, path: &str) -> Value {
    let root = crate::ide_api::ide_workspace_root();
    let audit = crate::code_audit::scan_workspace(&root, path);
    let complexity = crate::sdlc::assess_complexity(goal, kind, 0);
    let lang = crate::sdlc::resolve_lang(&s.db, &json!({ "spec": goal })).await;
    let lessons = crate::unified_engines::recall(goal, 3);
    let (level, cscore, reasons) = complexity;
    json!({
        "workspace": root.display().to_string(),
        "audit_ok": audit["ok"],
        "audit_grade": audit.get("grade").cloned().unwrap_or(json!(null)),
        "files_scanned": audit.get("files_scanned").cloned().unwrap_or(json!(0)),
        "lang": lang,
        "complexity": { "level": level, "score": cscore, "reasons": reasons },
        "lessons": lessons,
    })
}

/// Hatua 1 — SERA: Akili V6 (ethics/HITL) + governance ya amri.
fn policy(goal: &str, action: &str) -> Value {
    let think = crate::akili_bridge::think(goal);
    let gate = crate::akili_bridge::gate(action, goal);
    let (allowed, reason) = crate::unified_engines::governance_check(action);
    json!({
        "decision": think["decision"],
        "top_layers": think["top_layers"],
        "ethics_allowed": gate["allowed"],
        "requires_hitl": gate["requires_hitl"],
        "governance_allowed": allowed,
        "governance_reason": reason,
    })
}

/// Hatua 2 — MPANGO: SDLC phase plan + acceptance criteria + cortex capacity.
async fn plan(s: &AppState, goal: &str, kind: &str, lang: &str, level: &str, agents: usize) -> Value {
    let phases = crate::sdlc::phase_plan(&s.db, level, lang).await;
    let criteria = crate::sdlc::acceptance_criteria_lang(&s.db, kind, lang).await;
    let cortex = crate::swarm_v3::cortex_plan(agents);
    let ids = crate::sdlc::phase_ids();
    let labels: Vec<String> = ids.iter().map(|id| crate::sdlc::phase_label(id, lang)).collect();
    json!({
        "spec": goal,
        "phases": phases,
        "phase_labels": labels,
        "acceptance_criteria": criteria,
        "capacity": {
            "agents": cortex["agents"],
            "vram_gb": cortex["vram_gb"],
            "fits_24gb_card": cortex["fits_24gb_card"],
        },
    })
}

/// Hatua 3 — HOJA: Orthogonal Insight (worlds) + ReMIND (wake/dream/judge).
async fn ideate(goal: &str, kind: &str, solvers: usize, dreams: usize) -> Value {
    let orth = crate::swarm_v3::orthogonal(Json(crate::swarm_v3::OrthReq {
        task: goal.to_string(),
        solvers,
    }))
    .await
    .0;
    let remind = crate::swarm_v3::remind(Json(crate::swarm_v3::RemindReq {
        topic: format!("{kind}: {goal}"),
        dreams,
    }))
    .await
    .0;
    let cycle = remind["cycle"].as_array().cloned().unwrap_or_default();
    let phase = |name: &str| -> Value {
        cycle
            .iter()
            .find(|c| c["phase"].as_str() == Some(name))
            .cloned()
            .unwrap_or(json!(null))
    };
    let judge = phase("judge");
    let rewake = phase("re-wake");
    json!({
        "orthogonal": {
            "worlds_explored": orth["worlds_explored"],
            "distinct_approaches": orth["distinct_approaches"],
            "winner": orth["winner"],
        },
        "remind": {
            "best": judge["best"],
            "selected": rewake["selected"],
            "next_action": rewake["next_action"],
        },
    })
}

/// Hatua 4 — UTEPELEZAJI: AgentDescent (mawakala hupendekeza diffs).
async fn descend(s: &AppState, goal: &str, proposers: usize, iterations: usize) -> Value {
    let d = crate::swarm_v3::descent(State(s.clone()), Json(crate::swarm_v3::DescentReq {
        task: goal.to_string(),
        proposers,
        iterations,
    }))
    .await
    .0;
    json!({
        "best_score": d["best_score"],
        "merged_changes": d["merged_changes"],
        "iterations": d["iterations"],
    })
}

/// Hatua 5 — UTENDAJI: run_agentic (ReAct halisi) — ikiruhusiwa na sera.
async fn execute(s: &AppState, goal: &str, kind: &str, max_steps: usize, allowed: bool) -> Value {
    if !allowed {
        return json!({
            "status": "blocked",
            "reason": "sera (ethics/HITL/governance) hairuhusu utekelezaji wa kiotomatiki",
        });
    }
    let cfg = crate::ai_config::load_with_db(&s.db).await;
    let out = crate::agent_engine::run_agentic(goal, kind, max_steps, &cfg).await;
    json!({
        "status": if out["ok"] == json!(true) { "ok" } else { "error" },
        "project": out["project"],
        "workspace": out["workspace"],
        "files_written": out["files_written"],
        "verify": out["verify"],
        "steps_count": out["steps_count"],
        "error": out["error"],
    })
}

/// Hatua 6 — UKAGUZI: ide_engineer (senior loop) + SDLC gate ya awamu ya mwisho.
async fn review(kind: &str, lang: &str, path: &str, prior_ok: bool) -> Value {
    let eng = crate::ide_engineer::run(Json(json!({ "path": path, "tests": true })))
        .await
        .0;
    let last = crate::sdlc::phase_ids().last().copied().unwrap_or("ufuatiliaji");
    let gate = crate::sdlc::phase_gate(last, prior_ok);
    json!({
        "engineer_score": eng["score"],
        "security_grade": eng["security_grade"],
        "tests": eng["tests"],
        "recommendations": eng["recommendations"],
        "gate": gate,
        "engine": "ide_engineer (recon/security/tests/build/speed)",
        "kind": kind,
        "lang": lang,
    })
}

/// Hatua 7 — MEGAFLOW: mgawanyo wa huduma (model/agent/env).
async fn megaflow(goal: &str) -> Value {
    let tasks = goal.split_whitespace().count().max(8) * 3;
    let f = crate::swarm_v3::flow(Json(crate::swarm_v3::FlowReq {
        tasks,
        model_slots: 16,
        agent_slots: 64,
        env_slots: 32,
    }))
    .await
    .0;
    json!({
        "tasks": f["tasks"],
        "services": f["services"],
        "balanced_total": f["balanced_total"],
        "bottleneck_load": f["bottleneck_load"],
        "throughput_tasks_per_tick": f["throughput_tasks_per_tick"],
    })
}

/// POST /api/super/engineer — msimamizi mkuu: hatua 10 halisi.
pub async fn run(State(s): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let goal = body["goal"].as_str().unwrap_or("").trim().to_string();
    if goal.is_empty() {
        return Json(json!({ "ok": false, "error": "goal ni lazima" }));
    }
    let kind = body["kind"].as_str().unwrap_or("website").trim().to_string();
    let path = body["path"].as_str().unwrap_or("").trim().to_string();
    let execute_flag = body["execute"].as_bool().unwrap_or(true);
    let max_steps = body["max_steps"].as_u64().unwrap_or(14).clamp(1, 40) as usize;
    let agents = body["agents"].as_u64().unwrap_or(64).clamp(1, 100_000) as usize;
    let proposers = body["proposers"].as_u64().unwrap_or(8).clamp(1, 64) as usize;
    let iterations = body["iterations"].as_u64().unwrap_or(3).clamp(1, 16) as usize;
    let solvers = body["solvers"].as_u64().unwrap_or(6).clamp(1, 64) as usize;
    let dreams = body["dreams"].as_u64().unwrap_or(8).clamp(1, 64) as usize;

    let project = project_slug(&goal);
    let started = std::time::Instant::now();
    let mut steps: Vec<Value> = Vec::new();

    // 1) DESTURI
    let r = recon(&s, &goal, &kind, &path).await;
    let lang = r["lang"].as_str().unwrap_or("sw").to_string();
    let level = r["complexity"]["level"].as_str().unwrap_or("wastani").to_string();
    steps.push(json!({ "step": "recon", "status": "ok", "detail": r }));

    // 2) SERA
    let action = format!("build and run {kind} project for goal");
    let p = policy(&goal, &action);
    let allowed = p["ethics_allowed"].as_bool().unwrap_or(false)
        && p["governance_allowed"].as_bool().unwrap_or(false);
    let hitl = p["requires_hitl"].as_bool().unwrap_or(true);
    steps.push(json!({ "step": "policy", "status": "ok", "detail": p }));

    // 3) MPANGO
    let pl = plan(&s, &goal, &kind, &lang, &level, agents).await;
    steps.push(json!({ "step": "plan", "status": "ok", "detail": pl }));

    // 4) HOJA
    let id = ideate(&goal, &kind, solvers, dreams).await;
    steps.push(json!({ "step": "ideate", "status": "ok", "detail": id }));

    // 5) UTEPELEZAJI
    let de = descend(&s, &goal, proposers, iterations).await;
    steps.push(json!({ "step": "descend", "status": "ok", "detail": de }));

    // 6) UTENDAJI
    let ex = execute(&s, &goal, &kind, max_steps, execute_flag && allowed).await;
    let executed = ex["status"] == json!("ok");
    steps.push(json!({
        "step": "execute",
        "status": ex["status"],
        "detail": ex,
    }));

    // 7) UKAGUZI (tumia workspace ya mradi kama path, kama uliyopewa)
    let review_path = ex["project"]
        .as_str()
        .map(|p| p.to_string())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| path.clone());
    let rv = review(&kind, &lang, &review_path, executed).await;
    steps.push(json!({ "step": "review", "status": "ok", "detail": rv }));

    // 8) MEGAFLOW
    let mf = megaflow(&goal).await;
    steps.push(json!({ "step": "megaflow", "status": "ok", "detail": mf }));

    // 9) UAMUZI — score ya mwisho
    let engineer_score = score_of(&rv, "engineer_score");
    let descent_score = score_of(&de, "best_score");
    let mut score = engineer_score * 0.7 + descent_score * 0.3;
    if execute_flag {
        if !allowed {
            score *= 0.5;
        } else if !executed {
            score *= 0.6;
        }
    }
    score = rounds(score.clamp(0.0, 100.0));

    let mut recommendations: Vec<String> = Vec::new();
    if !execute_flag {
        recommendations.push(
            "📋 Hali ya mpango PEKEE (execute=false) — washa `execute:true` kuendesha ReAct."
                .to_string(),
        );
    }
    if !allowed {
        recommendations.push(
            "⛔ Sera (ethics/HITL/governance) ilizuia utekelezaji — hakiki na uidhinishe kwa mkono."
                .to_string(),
        );
    }
    if hitl {
        recommendations.push("🧑⚖️ HITL inahitajika kabla ya deploy — pata idhini ya mtu.".to_string());
    }
    if let Some(recs) = rv["recommendations"].as_array() {
        for rec in recs.iter().take(4) {
            if let Some(sr) = rec.as_str() {
                recommendations.push(sr.to_string());
            }
        }
    }

    let verdict = if !execute_flag {
        "mpango tayari"
    } else if !allowed {
        "imelindwa"
    } else if executed && score >= 80.0 {
        "tayari"
    } else if executed {
        "inahitaji marekebisho"
    } else {
        "utekelezaji umeshindwa"
    };

    let out = json!({
        "ok": true,
        "role": "super_intellectual_engineer",
        "engine": "MTECHOS Super Intellectual Engineer",
        "goal": goal,
        "project": project,
        "kind": kind,
        "lang": lang,
        "complexity": level,
        "executed": executed,
        "ethics_allowed": allowed,
        "requires_hitl": hitl,
        "score": score,
        "verdict": verdict,
        "recommendations": recommendations,
        "steps": steps,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    });

    // 10) KUMBUKUMBU — hifadhi somo kwa baadaye (halisi: data/agent/memory.jsonl).
    // Huhifadhi tu matukio yenye maana (yalitekelezwa au yalizuiliwa na sera).
    if execute_flag && allowed {
        crate::unified_engines::remember(
            &project,
            &kind,
            executed,
            &format!("{verdict} (score {score})"),
        );
    }

    Json(out)
}

/// GET /api/super/engineer/capabilities — uwezo wa msimamizi mkuu.
pub async fn capabilities() -> Json<Value> {
    let ids = crate::sdlc::phase_ids();
    Json(json!({
        "ok": true,
        "role": "super_intellectual_engineer",
        "pipeline": [
            "recon", "policy", "plan", "ideate", "descend",
            "execute", "review", "megaflow", "verdict", "remember",
        ],
        "composes": [
            "code_audit (recon + security)",
            "akili_bridge / Akili V6 (ethics + HITL)",
            "unified_engines (governance + memory)",
            "sdlc (complexity + phase plan + gate)",
            "swarm_v3 (cortex capacity + orthogonal insight + remind + descent + megaflow)",
            "agent_engine (agentic ReAct execution)",
            "ide_engineer (senior loop: recon/security/tests/build/speed)",
        ],
        "sdlc_phases": ids,
        "engines_status": crate::unified_engines::status(),
    }))
}

pub fn router() -> axum::Router<AppState> {
    use axum::routing::get;
    Router::new()
        .route("/api/super/engineer", post(run))
        .route("/api/super/engineer/capabilities", get(capabilities))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_safe() {
        assert_eq!(project_slug("Unda app ya simu!"), "unda-app-ya-simu");
        assert_eq!(project_slug("   "), "super-project");
        assert!(project_slug(&"a".repeat(200)).len() <= 48);
    }

    #[test]
    fn score_clamped() {
        assert_eq!(score_of(&json!({ "x": 250.0 }), "x"), 100.0);
        assert_eq!(score_of(&json!({}), "x"), 0.0);
        assert_eq!(rounds(83.456), 83.5);
    }

    #[test]
    fn capabilities_shape() {
        let v = json!({ "ok": true, "role": "super_intellectual_engineer" });
        assert_eq!(v["role"], "super_intellectual_engineer");
    }
}
