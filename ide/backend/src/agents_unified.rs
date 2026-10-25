//! agents_unified.rs — SAFU MOJA ya agents wote wa MTECHOS (mradi mmoja).
//!
//! Huunganisha familia zote za agents zinazofanya kazi HALISI:
//!   - swarm   : agents 50 (ide_swarm)  — gen/audit/review/test/docs/analytics
//!   - ts      : ThreatSwarm agents — kill-chain scope-gated (Kali tools + social)
//!   - fleet   : agents wa vifaa (fleet) — rejista/heartbeat/poll (DB)
//!   - orch    : orchestrator (pool 100) — ratiba ya kazi
//!
//! API moja: `/api/agents/unified/{catalog,run,swarm}`.
//! `run` hukimbia agents kwa PARALLEL (JoinSet) kwa kasi; kila moja hurudisha
//! ripoti HALISI. ThreatSwarm hulindwa na scope-gate + HITL.
//!
//! Kanuni: hakuna mock. Hii ni safu ya kuunganisha tu — kazi zote hufanywa na
//! moduli za msingi (ide_swarm, threatswarm, fleet).

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use std::time::Instant;

use crate::AppState;

/// Katalogi ya pamoja: kila agent na familia, uwezo, na kazi zake.
pub async fn catalog(s: &AppState) -> Value {
    // --- SWARM (50) ---
    let swarm: Vec<Value> = crate::ide_swarm::AGENTS
        .iter()
        .map(|a| {
            json!({
                "id": a.id, "name": a.name, "family": "swarm", "domain": a.domain,
                "capability": format!("{:?}", a.cap), "focus": a.focus,
                "scope_gated": false,
            })
        })
        .collect();
    // --- THREATSWARM ---
    let ts: Vec<Value> = crate::threatswarm::agent_entries()
        .iter()
        .map(|(name, tasks)| {
            json!({
                "id": name, "name": name, "family": "ts", "domain": "security",
                "capability": "kill-chain", "tasks": tasks, "task_count": tasks.len(),
                "scope_gated": true,
            })
        })
        .collect();
    // --- FLEET (vifaa) ---
    let fleet_list = crate::fleet::list(&s.db).await;
    let fleet: Vec<Value> = fleet_list
        .get("agents")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|a| {
            json!({
                "id": a.get("id").cloned().unwrap_or(Value::Null),
                "name": a.get("device").cloned().unwrap_or(Value::Null),
                "family": "fleet", "domain": "device", "capability": "device-agent",
                "scope_gated": false, "detail": a,
            })
        })
        .collect();
    // --- SOCIAL MEDIA (modules 58-272, kwa jukwaa) ---
    let social: Vec<Value> = crate::tsmod::social::PLATFORMS
        .iter()
        .map(|p| {
            json!({
                "id": p.key, "name": p.name, "family": "social", "domain": "social",
                "capability": "content-analysis", "scope_gated": false,
                "module_first": p.first, "module_last": p.last,
                "module_count": (p.last - p.first + 1) as usize,
                "engine": "DB (inayojifunza) + LLM (semantic)",
            })
        })
        .collect();
    // --- CALLS (modules 273-287, sauti/video) ---
    let calls: Vec<Value> = crate::secpack::CALLS_MODULE_IDS
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("call-{id}"), "name": m.name(), "family": "calls",
                "domain": "calls", "capability": "call-protection", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    // --- MESSAGING (modules 288-305, SMS & email) ---
    let messaging: Vec<Value> = crate::secpack::MESSAGING_MODULE_IDS
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("msg-{id}"), "name": m.name(), "family": "messaging",
                "domain": "messaging", "capability": "message-protection", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    // --- DEVICE (modules 306-322, SIM & kifaa) ---
    let device: Vec<Value> = crate::secpack::DEVICE_MODULE_IDS
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("dev-{id}"), "name": m.name(), "family": "device",
                "domain": "device", "capability": "device-protection", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    // --- HUMANITARIAN (modules 323-328, kimataifa + future-proof) ---
    let humanitarian: Vec<Value> = crate::secpack::HUMAN_MODULE_IDS
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("hum-{id}"), "name": m.name(), "family": "humanitarian",
                "domain": "humanitarian", "capability": "humanitarian-protection", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    // --- BLOCKCHAIN (modules 468-500, usalama wa blockchain) ---
    let blockchain: Vec<Value> = crate::secpack::BLOCKCHAIN_MODULE_IDS
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("bc-{id}"), "name": m.name(), "family": "blockchain",
                "domain": "blockchain", "capability": "blockchain-protection", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    // --- MODULES (SEHEMU 8-11 + 20: modules 1-34, 501-516) ---
    let module_ids: Vec<u16> = crate::secpack::CALLS1_MODULE_IDS
        .iter()
        .chain(crate::secpack::SMS_MODULE_IDS.iter())
        .chain(crate::secpack::AI_MODULE_IDS.iter())
        .chain(crate::secpack::EXT_MODULE_IDS.iter())
        .copied()
        .collect();
    let modules: Vec<Value> = module_ids
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("mod-{id}"), "name": m.name(), "family": "modules",
                "domain": m.group(), "capability": "module-engine", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    // --- TWIST (twists 31-300, id 1031-1300) ---
    let twists: Vec<Value> = crate::secpack::TWIST_MODULE_IDS
        .iter()
        .filter_map(|&id| crate::tsmod::find(id).map(|m| (id, m)))
        .map(|(id, m)| {
            json!({
                "id": format!("tw-{id}"), "name": m.name(), "family": "twist",
                "domain": "twist", "capability": "twist-engine", "scope_gated": false,
                "module": id, "group": m.group(),
            })
        })
        .collect();

    let total = swarm.len() + ts.len() + fleet.len() + social.len() + calls.len() + messaging.len() + device.len() + humanitarian.len() + blockchain.len() + modules.len() + twists.len();
    let mut all: Vec<Value> = swarm;
    all.extend(ts);
    all.extend(fleet);
    all.extend(social);
    all.extend(calls);
    all.extend(messaging);
    all.extend(device);
    all.extend(humanitarian);
    all.extend(blockchain);
    all.extend(modules);
    all.extend(twists);

    json!({
        "ok": true,
        "families": {
            "swarm": all.iter().filter(|a| a["family"] == "swarm").count(),
            "ts": all.iter().filter(|a| a["family"] == "ts").count(),
            "fleet": all.iter().filter(|a| a["family"] == "fleet").count(),
            "social": all.iter().filter(|a| a["family"] == "social").count(),
            "calls": all.iter().filter(|a| a["family"] == "calls").count(),
            "messaging": all.iter().filter(|a| a["family"] == "messaging").count(),
            "device": all.iter().filter(|a| a["family"] == "device").count(),
            "humanitarian": all.iter().filter(|a| a["family"] == "humanitarian").count(),
            "blockchain": all.iter().filter(|a| a["family"] == "blockchain").count(),
            "modules": all.iter().filter(|a| a["family"] == "modules").count(),
            "twist": all.iter().filter(|a| a["family"] == "twist").count(),
                    },
        "total_agents": total,
        "agents": all,
        "note_sw": "SAFU MOJA: swarm(50) + ThreatSwarm + fleet(vifaa) + social(12 majukwaa, modules 58-272) + calls(15, modules 273-287) + messaging(18, modules 288-305) + device(17, modules 306-322) + humanitarian(6, modules 323-328) + blockchain(33, modules 468-500) + modules(24, modules 1-34) + extended(16, modules 501-516) + twist(270, twists 31-300). \
                    Kazi zote ni HALISI; ThreatSwarm hulindwa na scope-gate + HITL.",
    })
}

/// Endesha agent MOJA ya kijamii: changanua maudhui/URL kwa injini ya jukwaa.
/// Hii ni njia HALISI (DB + LLM), sio catalog.
pub async fn run_social(db: &sqlx::SqlitePool, key: &str, target: &str) -> Value {
    use crate::tsmod::TsModule;
    let Some(p) = crate::tsmod::social::PLATFORMS.iter().find(|p| p.key == key) else {
        return json!({ "ok": false, "error": format!("jukwaa '{key}' haipo") });
    };
    // Chagua module inayofaa zaidi kwa uchanganuzi wa maudhui: phishing/scam
    // kwanza, kisha content/fraud, kisha module yoyote ya jukwaa.
    let cands: Vec<_> = crate::tsmod::social::all()
        .iter()
        .filter(|m| m.platform.key == p.key)
        .collect();
    let pick = |needle: &str| cands.iter().find(|m| m.name.to_lowercase().contains(needle)).copied();
    let m = pick("phishing")
        .or_else(|| pick("scam"))
        .or_else(|| pick("spam"))
        .or_else(|| cands.iter().copied().find(|m| matches!(m.cat(), crate::tsmod::social::Cat::Content | crate::tsmod::social::Cat::Fraud)))
        .or_else(|| cands.first().copied());
    let Some(m) = m else {
        return json!({ "ok": false, "error": "hakuna module ya jukwaa hili" });
    };
    let params = json!({ "op": "analyze", "text": target, "use_ai": true });
    match m.run_db(db, target, &params).await {
        Ok(rep) => json!({
            "ok": true, "platform": p.name, "module": m.id,
            "summary": rep.summary, "report": rep.to_json(),
        }),
        Err(e) => json!({ "ok": false, "platform": p.name, "error": e }),
    }
}

/// Endesha agents wengi kwa PARALLEL (JoinSet) — kasi.
/// Body: `{ "family": "swarm"|"ts"|"auto", "agents": ["id",...], "idea", "kind",
///         "path", "target", "human_approved", "concurrency" }`.
/// Kama `agents` haipo, huchagua: family=auto → swarm wote; family=ts → ts wote.
pub async fn run_many(s: &AppState, body: &Value) -> Value {
    let family = body["family"].as_str().unwrap_or("auto");
    let idea = body["idea"].as_str().unwrap_or("").to_string();
    let kind = body["kind"].as_str().unwrap_or("website").to_string();
    let path = body["path"].as_str().unwrap_or("").to_string();
    let target = body["target"].as_str().unwrap_or("").to_string();
    let approved = body["human_approved"].as_bool().unwrap_or(false);
    let conc = body["concurrency"]
        .as_u64()
        .unwrap_or(8)
        .clamp(1, 64) as usize;

    // Chagua agents.
    let mut ids: Vec<String> = body["agents"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        match family {
            "ts" => {
                ids = crate::threatswarm::agent_entries()
                    .iter()
                    .map(|(n, _)| n.to_string())
                    .collect()
            }
            "swarm" => {
                ids = crate::ide_swarm::AGENTS
                    .iter()
                    .map(|a| a.id.to_string())
                    .collect()
            }
            _ => {
                // auto: swarm wote (halisi, haraka)
                ids = crate::ide_swarm::AGENTS
                    .iter()
                    .map(|a| a.id.to_string())
                    .collect()
            }
        }
    }
    if ids.is_empty() {
        return json!({ "ok": false, "error": "hakuna agents waliochaguliwa" });
    }

    // Tambua familia ya kila id (single source of truth).
    let is_ts = |id: &str| crate::threatswarm::find_agent(id).is_some();
    let is_swarm = |id: &str| crate::ide_swarm::AGENTS.iter().any(|a| a.id == id);
    let is_social = |id: &str| crate::tsmod::social::PLATFORMS.iter().any(|p| p.key == id);
    let is_call = |id: &str| id.strip_prefix("call-").and_then(|s| s.parse::<u16>().ok()).is_some();
    let is_msg = |id: &str| id.strip_prefix("msg-").and_then(|s| s.parse::<u16>().ok()).is_some();
    let is_dev = |id: &str| id.strip_prefix("dev-").and_then(|s| s.parse::<u16>().ok()).is_some();
    let is_hum = |id: &str| id.strip_prefix("hum-").and_then(|s| s.parse::<u16>().ok()).is_some();
    let is_bc = |id: &str| id.strip_prefix("bc-").and_then(|s| s.parse::<u16>().ok()).is_some();

    // ThreatSwarm inahitaji target; kama haipo, irukwe kwa uwazi (halisi).
    let mut skipped: Vec<Value> = Vec::new();
    let mut swarm_jobs: Vec<String> = Vec::new();
    let mut ts_jobs: Vec<String> = Vec::new();
    let mut social_jobs: Vec<String> = Vec::new();
    let mut call_jobs: Vec<String> = Vec::new();
    let mut msg_jobs: Vec<String> = Vec::new();
    let mut dev_jobs: Vec<String> = Vec::new();
    let mut hum_jobs: Vec<String> = Vec::new();
    let mut bc_jobs: Vec<String> = Vec::new();
    for id in ids {
        if is_swarm(&id) {
            swarm_jobs.push(id);
        } else if is_ts(&id) {
            if target.trim().is_empty() {
                skipped.push(json!({ "agent": id, "reason": "ThreatSwarm inahitaji 'target'" }));
            } else {
                ts_jobs.push(id);
            }
        } else if is_social(&id) {
            if target.trim().is_empty() {
                skipped.push(json!({ "agent": id, "reason": "social agent inahitaji 'target' (maandhui/URL)" }));
            } else {
                social_jobs.push(id);
            }
        } else if is_call(&id) {
            call_jobs.push(id);
        } else if is_msg(&id) {
            msg_jobs.push(id);
        } else if is_dev(&id) {
            dev_jobs.push(id);
        } else if is_hum(&id) {
            hum_jobs.push(id);
        } else if is_bc(&id) {
            bc_jobs.push(id);
        } else {
            skipped.push(json!({ "agent": id, "reason": "agent haijulikani" }));
        }
    }

    let started = Instant::now();
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(conc));
    let db = s.db.clone();

    let mut set: tokio::task::JoinSet<Value> = tokio::task::JoinSet::new();

    // --- SWARM jobs (parallel, bound by semaphore) ---
    for id in swarm_jobs {
        let sem = sem.clone();
        let idea = idea.clone();
        let kind = kind.clone();
        let path = path.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let r = crate::ide_swarm::run_one(&id, &idea, &kind, &path).await;
            json!({ "agent": id, "family": "swarm", "result": r })
        });
    }
    // --- THREATSWARM jobs (parallel; kila moja ina scope-gate + HITL) ---
    for id in ts_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let r = crate::threatswarm::run_agent(&db, &id, &target, approved).await;
            json!({ "agent": id, "family": "ts", "result": r })
        });
    }
    // --- SOCIAL jobs (parallel; injini ya DB + LLM kwa maudhui) ---
    for key in social_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let r = crate::agents_unified::run_social(&db, &key, &target).await;
            json!({ "agent": key, "family": "social", "result": r })
        });
    }
    // --- CALLS jobs (parallel; injini za sauti/video za tsmod) ---
    for id in call_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let n: u16 = id.strip_prefix("call-").and_then(|s| s.parse().ok()).unwrap_or(0);
            let r = crate::secpack::ts_module_task(&db, n, &target).await;
            json!({ "agent": id, "family": "calls", "result": r })
        });
    }
    // --- MESSAGING jobs (parallel; injini za SMS/email za tsmod) ---
    for id in msg_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let n: u16 = id.strip_prefix("msg-").and_then(|s| s.parse().ok()).unwrap_or(0);
            let r = crate::secpack::ts_module_task_p(&db, n, &target, "msg").await;
            json!({ "agent": id, "family": "messaging", "result": r })
        });
    }
    // --- DEVICE jobs (parallel; injini za SIM/kifaa za tsmod) ---
    for id in dev_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let n: u16 = id.strip_prefix("dev-").and_then(|s| s.parse().ok()).unwrap_or(0);
            let r = crate::secpack::ts_module_task_p(&db, n, &target, "dev").await;
            json!({ "agent": id, "family": "device", "result": r })
        });
    }
    // --- HUMANITARIAN jobs (parallel; injini za kimataifa/future-proof za tsmod) ---
    for id in hum_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let n: u16 = id.strip_prefix("hum-").and_then(|s| s.parse().ok()).unwrap_or(0);
            let r = crate::secpack::ts_module_task_p(&db, n, &target, "hum").await;
            json!({ "agent": id, "family": "humanitarian", "result": r })
        });
    }
    // --- BLOCKCHAIN jobs (parallel; injini za blockchain za tsmod) ---
    for id in bc_jobs {
        let sem = sem.clone();
        let db = db.clone();
        let target = target.clone();
        set.spawn(async move {
            let _p = sem.acquire().await.ok();
            let n: u16 = id.strip_prefix("bc-").and_then(|s| s.parse().ok()).unwrap_or(0);
            let r = crate::secpack::ts_module_task_p(&db, n, &target, "bc").await;
            json!({ "agent": id, "family": "blockchain", "result": r })
        });
    }

    let mut results: Vec<Value> = Vec::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(v) => results.push(v),
            Err(e) => results.push(json!({ "error": format!("task join error: {e}") })),
        }
    }

    let elapsed_ms = started.elapsed().as_millis() as u64;
    json!({
        "ok": true,
        "agents_run": results.len(),
        "agents_skipped": skipped,
        "concurrency": conc,
        "elapsed_ms": elapsed_ms,
        "results": results,
    })
}

// ---------- HTTP handlers ----------

/// GET /api/agents/unified/catalog
pub async fn catalog_h(State(s): State<AppState>) -> Json<Value> {
    Json(catalog(&s).await)
}

/// POST /api/agents/unified/run
pub async fn run_h(State(s): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    Json(run_many(&s, &body).await)
}

/// POST /api/agents/unified/swarm — swarm yote (50) + ripoti + R analytics.
/// Huhifadhi sarafu ya utendaji: hukimbia kwa parallel ndani ya moduli.
pub async fn swarm_h(State(s): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let idea = body["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = body["kind"].as_str().unwrap_or("website").trim().to_string();
    let path = body["path"].as_str().unwrap_or("").trim().to_string();
    let include_llm = body["use_llm"].as_bool().unwrap_or(false);
    Json(crate::ide_swarm::run_all(&idea, &kind, &path, include_llm, &s.brain).await)
}

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/agents/unified/catalog", get(catalog_h))
        .route("/api/agents/unified/run", post(run_h))
        .route("/api/agents/unified/swarm", post(swarm_h))
}
