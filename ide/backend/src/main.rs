//! Fundi Deploy Agent v3 — P2 kamili + P4 WIREGUARD VPN + MAJINA KIOTOMATIKI
//!
//! Pipeline: plan (AI+rules) → HITL → backup (halisi) → WOL → PXE → [multicast] → install → report
//! Msimamizi: approve/cancel + UI ya canvas /ide
//! VPN (P4): kazi ZOTE za mbali zinaenda kupitia WireGuard (wg0) — /api/vpn/*
//! Majina: duplicates (hr, hr 1, hr 2…) kulingana na mpangilio wa IP — /computers

mod ai;
mod ai_config;
mod agent_tools;
mod analytics;
mod auth;
use rand::RngCore;
use sqlx::Row;
use sha2::{Digest, Sha256};


mod backup;
mod brain;
mod build_pipeline;
mod bundles;
mod hotpatch;
mod shadow;
mod timetravel;
mod chat;
mod daily;
mod data_io;
mod education;
mod elimupro;
mod exam_lan;
mod teaching;
mod live;
mod proctor;
mod marks;
mod attendance;
mod fees;
mod library;
mod hostel;
mod transcript;
mod offline;
mod voip;
mod clearance;
mod maps;
mod scribe;
mod compsol;
mod akademia;
mod ittools;
mod aiengine;
mod malipo;
mod lesson;
mod tetea;
mod engine_api;
mod deploy_templates;
mod tools_catalog;
mod tool_calls;
mod akili;
mod akili_v3;
mod akili_v4;
mod akili_human;
mod akili_v6;
mod akili_bridge;
mod code_rules;
mod code_skills;
mod nyi;
mod radi;
mod sec_gate;
mod nyi_viz;
mod nyi_media;
mod nyi_ft;
mod deepscan;
mod engines_canvas;
mod unified_engines;
mod lang_verify;
mod canvas_ext;
mod studio;
mod mts_proxy;
mod native_protocol;
mod agent_catalogs;
mod agent_native;
mod agentic_bridge;
mod forge;
mod forge_domain;
mod forge_generators;
mod forge_mobile;
mod forge_prod;
mod forge_projects;
mod forge_rust3d;
mod forge_tools;
mod r_engine;
mod fleet;
mod fundi_agent_client;
mod images;
mod kali_groups;
mod language;
mod llm_setup;
mod memindex;
mod multicast;
mod agent_engine;
mod ide_api;
mod ide_integrations;
mod ide_emulators;
mod ide_lab;
mod ide_swarm;
mod agents_unified;
mod script_agent;
mod feedback;
mod swarm_v2;
mod swarm_v3;
mod code_audit;
mod ide_devices;
mod ide_testkit;
mod ide_build;
mod ide_code_run;
mod ide_engineer;
mod ide_lsp;
mod ide_complete;
mod ide_composer;
mod ide_dap;
mod ide_e2e;
mod ide_pipeline;
mod sdlc;
mod super_engineer;
mod osselect;
mod pfsense;
mod pipeline;
mod quantum;
mod reacon;
mod report;
mod admin_console;
mod secops;
mod secpack;
mod threatswarm;
mod threatintel;
// AGENT-ENGINE akili-security: cyber + digital forensics (agents 27 huongoza zana).
mod platforms;
mod agentbind;
mod finshield;
mod apiauth;
mod gateprobe;
mod fastscan;
mod phoenix_shield;
mod phoenix_agent;
mod tsmod;
mod shield;
mod toolkit;
mod tools;
mod tools_internal;
mod pkicrypto;
// MBILINYI PLATFORM: video calls / live video / voice / SMS (WebRTC + push).
mod vector;
mod vpn;
mod wol;
mod localai;
mod weather;
mod search;
mod e2ee;
mod diarize;
// MTECHOS GROUP: jukwaa la biashara (huduma, wafanyakazi, maombi ya wateja,
// mikataba ya ECDSA), malipo ya mtandaoni (Stripe + escrow), ajira, SMS, na
// kampuni ya wakala 100 wanaojisimamia.
mod biashara;
mod escrow;
mod ajira;
mod sms;
mod agentkampuni;

use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::{get, post},
    Json, Router,
};
use multicast::SharedAcks;
use pipeline::{DeployRequest, Job};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use axum::response::IntoResponse;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    jobs: Arc<RwLock<Vec<Job>>>,
    acks: SharedAcks,
    brain: Arc<brain::Brain>,
    pfsense: Arc<pfsense::PfClient>,
    agent: Option<Arc<fundi_agent_client::FundiAgentClient>>,
    engine: engine_api::EngineState,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    std::fs::create_dir_all("/data").ok();
    std::fs::create_dir_all("/var/lib/tftpboot/pxelinux.cfg").ok();
    std::fs::create_dir_all(vpn::dir()).ok();
    // API key ya OLLAMA CLOUD (faili, SIO DB) — pakia kabla ya server kuanza
    ai_config::load_key_file();

    // DB: FUNDI_DB (URL kamili) au /data/fundi.db (production default).
    // FUNDI_DB inaruhusu kuendesha bila root kwenye dev/CI.
    let db_url = match std::env::var("FUNDI_DB") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => "sqlite:///data/fundi.db?mode=rwc".to_string(),
    };
    // WAL + busy_timeout: uingizaji wa TETEA huendesha sehemu sambamba, hivyo
    // waandishi wengi hukutana. Bila busy_timeout, SQLite hurudisha "database is
    // locked" mara moja.
    let db_opts = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)?
        .busy_timeout(std::time::Duration::from_secs(30));
    let db = SqlitePool::connect_with(db_opts).await?;
    let _ = sqlx::query("PRAGMA journal_mode=WAL").execute(&db).await;
    let _ = sqlx::query("PRAGMA synchronous=NORMAL").execute(&db).await;
    for ddl in [
        r#"CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            device_mac TEXT,
            device_name TEXT,
            os_type TEXT,
            os_reason TEXT,
            status TEXT,
            stage TEXT,
            progress INTEGER,
            message TEXT,
            backup_info TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT,
            role TEXT,
            status TEXT,
            last_seen TEXT
        )"#,
    ] {
        sqlx::query(ddl).execute(&db).await?;
    }
    // Columns mpya kwenye DB ya zamani (migration ndogo)
    for col in ["os_reason TEXT", "backup_info TEXT"] {
        let _ = sqlx::query(&format!("ALTER TABLE jobs ADD COLUMN {col}"))
            .execute(&db)
            .await;
    }
    vpn::init_tables(&db).await;
    bundles::init_tables(&db).await;
    brain::init_tables(&db).await;
    auth::init_tables(&db).await;
    // RBAC KWA AGENT: session ya Admin Mkuu kwa fundi_api (chat inaita API zenye ulinzi)
    // Token IMARA (agent_token) — Canvas (/ide) huipata kupitia
    // window.__AGENT_CANVAS_SESSION_API_KEY__, hivyo haibadiliki kwenye restart.
    let data_dir = std::env::var("FUNDI_DATA").unwrap_or_else(|_| "/data".to_string());
    let agent_token = auth::agent_session_stable(&db, &data_dir).await;
    chat::set_agent_token(agent_token.clone());
    canvas_ext::set_agent_key(agent_token);
    ide_integrations::init_tables(&db).await;
    daily::init_tables(&db).await;
    language::init_tables(&db).await;
    // MIGRATION: legacy admin (admin/mtech2026) haipo tena — MBILINYI ndiye Admin Mkuu
    let _ = sqlx::query("DELETE FROM sessions WHERE username = 'admin'").execute(&db).await;
    let _ = sqlx::query("DELETE FROM users WHERE username = 'admin'").execute(&db).await;
    secops::init_tables(&db).await;
    threatintel::init_tables(&db).await;
    toolkit::init_tables(&db).await;
    fleet::init_tables(&db).await;
    deepscan::init_tables(&db).await;
    shield::init_tables(&db).await;
    // (mikataba na ripoti hutia muhuri kwa funguo ya kampuni).
    pkicrypto::init_tables(&db).await;
    if let Err(e) = pkicrypto::ensure_company_key(&db).await {
        tracing::warn!("pkicrypto key: {e}");
    }
    feedback::init_tables(&db).await;
    admin_console::init_tables(&db).await;
    swarm_v2::init_tables(&db).await;
    swarm_v3::init_tables(&db).await;
    sdlc::init_tables(&db).await;
    // TOTAL SECURITY (tsmod): hifadhidata ya sheria/IOC/events (dynamic) + seed
    // ya sheria za msingi kutoka spec mara ya kwanza tu.
    tsmod::store::init_tables(&db).await;
    if let Err(e) = tsmod::store::seed_defaults(&db).await {
        tracing::warn!("tsmod seed: {e}");
    }

    // NEURALIS BRAIN (H5b): SQLite + LanceDB-compatible vector store (cosine semantic search)
    let brain = Arc::new(brain::Brain::new(db.clone(), "/data"));
    ai_config::init_tables(&db).await;
    education::init_tables(&db).await;
    lesson::init_tables(&db).await;
    elimupro::init_tables(&db).await;
    teaching::init_tables(&db).await;
    live::init_tables(&db).await;
    proctor::init_tables(&db).await;
    marks::init_tables(&db).await;
    attendance::init_tables(&db).await;
    fees::init_tables(&db).await;
    library::init_tables(&db).await;
    hostel::init_tables(&db).await;
    transcript::init_tables(&db).await;
    offline::init_tables(&db).await;
    voip::init_tables(&db).await;
    clearance::init_tables(&db).await;
    maps::init_tables(&db).await;
    scribe::init_tables(&db).await;
    compsol::init_tables(&db).await;
    akademia::init_tables(&db).await;
    search::init_tables(&db).await;
    e2ee::init_tables(&db).await;
    ittools::init_tables(&db).await;
    aiengine::init_tables(&db).await;
    malipo::init_tables(&db).await;
    exam_lan::init_tables(&db).await;
    tetea::init_tables(&db).await;
    // MTECHOS GROUP: biashara kwanza (inaseed katalogi/idara/maeneo ambayo
    // ajira na kampuni ya wakala hutegemea), kisha malipo, ajira, SMS, wakala.
    biashara::init_tables(&db).await;
    escrow::init_tables(&db).await;
    ajira::init_tables(&db).await;
    sms::init_tables(&db).await;
    agentkampuni::init_tables(&db).await;
    ai_config::sync_env_from_db(&db).await;
    // PFSENSE API (H5b): client halisi — bila env, kazi za firewall zinarudisha error ya configuration
    let pfsense = Arc::new(pfsense::PfClient::from_env());

    // FUNDI AGENT BRIDGE: REST client wa Fundi Server (crates/fundi-server, port 8787)
    let agent_cfg = fundi_agent_client::FundiAgentConfig::default();
    let agent: Option<Arc<fundi_agent_client::FundiAgentClient>> = if agent_cfg.enabled {
        tracing::info!(
            "Fundi Agent bridge: enabled addr={} timeout={}s",
            agent_cfg.base_url,
            agent_cfg.timeout_secs
        );
        Some(Arc::new(fundi_agent_client::FundiAgentClient::new(
            agent_cfg,
        )))
    } else {
        tracing::info!("Fundi Agent bridge: disabled (FUNDI_AGENT_ENABLED=0)");
        None
    };

    // Seed agents 10 (agentic vision)
    let agents: [(&str, &str); 10] = [
        ("receptionist", "Mpokeaji"),
        ("vision", "Muono"),
        ("diagnoser", "Mgunduzi"),
        ("planner", "Mpangaji"),
        ("solver", "Mtatuzi"),
        ("tester", "Mjaribu"),
        ("verifier", "Mthibitishaji"),
        ("scribe", "Mwandishi"),
        ("reporter", "Mripoti"),
        ("learner", "Mwanafunzi"),
    ];
    for (id, name) in agents {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO agents (id, name, role, status, last_seen) VALUES (?,?,?,?,?)",
        )
        .bind(id)
        .bind(name)
        .bind("pipeline")
        .bind("ready")
        .bind(chrono::Local::now().to_rfc3339())
        .execute(&db)
        .await;
    }

    let acks: SharedAcks = Arc::new(multicast::AckBoard::default());
    let state = AppState {
        db: db.clone(),
        jobs: Arc::new(RwLock::new(Vec::new())),
        acks: acks.clone(),
        brain,
        pfsense,
        agent,
        engine: engine_api::global().clone(),
    };

    // KUJIFUNZA KILA SIKU (self-learning): kila saa 24, agent atafuta 3 topics
    // mtandaoni na kuhifadhi kwenye Brain (kila entry ina "on_tz:" timestamp).
    // Hii inafanya MTECH OS ajifunze kutoka web — bila training, bila uongo.
    {
        let learn_db = db.clone();
        let brain_ref = state.brain.clone();
        tokio::spawn(async move {
            loop {
                // Saa 24 kati ya kila scan — hakuna kuwalazimisha, ni polepole tu.
                tokio::time::sleep(std::time::Duration::from_secs(24 * 3600)).await;
                let topics = ["vya leo za cybersecurity CVE", "best practices Docker production", "malengo ya Ubuntu LTS latest", "vitalu vya AI agents latest"];
                let mut learned = 0;
                for t in &topics {
                    if learned >= 3 { break; }  // kikomo: 3 kila siku
                    let url = format!("https://html.duckduckgo.com/html/?q={}", urlenc(t));
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(15))
                        .build();
                    let Ok(client) = client else { continue };
                    let Ok(c) = client.get(&url).send().await else { continue };
                    let Ok(text) = c.text().await else { continue };
                    let first_three: Vec<String> = text
                        .lines()
                        .filter(|l| l.contains("result__snippet") || l.contains("result__a"))
                        .map(|s| s.trim().to_string())
                        .take(3)
                        .collect();
                    let summary = first_three.join("\n");
                    if summary.is_empty() { continue; }
                    let problem = format!("kujifunza-{}: {}", chrono::Local::now().format("%y%m%d"), t);
                    let _ = crate::brain::remember(
                        &brain_ref,
                        "self_learning",
                        "web_learning",
                        &problem,
                        &summary,
                        0.8,
                    ).await;
                    learned += 1;
                }
                if learned > 0 {
                    tracing::info!("Self-learning: {} topics zimefanyiwa kichwa kutoka web", learned);
                }
            }
        });
    }

    // Load jobs za zamani kutoka DB
    let old: Vec<(String, String, String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, device_mac, device_name, os_type, status, stage, progress, message FROM jobs",
    )
    .fetch_all(&db)
    .await
    .unwrap_or_default();
    for (id, mac, name, os, status, stage, prog, msg) in old {
        state.jobs.write().await.push(Job {
            id,
            device_mac: mac,
            device_name: name,
            os_type: os,
            status,
            stage,
            progress: prog as u32,
            message: msg,
            needs_approval: false,
            image: None,
            multicast: false,
        });
    }


    // WG0.CONF: regen kila boot (peers DB → conf halisi); tunnel haianzi yenyewe (HITL: /api/vpn/up)
    match vpn::write_server_conf(&db).await {
        Ok(p) => tracing::info!("vpn: wg conf tayari: {} (up: /api/vpn/up)", p.display()),
        Err(e) => tracing::warn!("vpn: server conf imeshindikana: {e}"),
    }

    let ui = ServeDir::new("static").append_index_html_on_directories(true);

    // MBILINYI Agent Canvas (React) — snapshot halisi `engines/mbilinyi/build/`.
    // Kama `build/` haipo (snapshot source-only), /ide inarudisha JSON inayoelekeza
    // ku-build kwanza — hakuna stub/mock HTML. Build: cd engines/mbilinyi && npm ci && npm run build
    let oh_ready = canvas_build_dir().is_some();
    if oh_ready {
        tracing::info!("canvas: React UI build/ ipo — /ide inahudumia Agent Canvas (SPA)");
    } else {
        tracing::info!("canvas: build/ haipo (source snapshot) — /ide inarudisha build hint");
    }

    let app = Router::new()
        // `/` inapeleka kwenye canvas (kitovu kimoja cha MTECHOS).
        .route("/", get(canvas_redirect))
        // `/ide` + `/ide.html` = OpenHands Agent Canvas (React) MOJA KWA MOJA.
        // Canvas imejengwa kwa base `/ide` (VITE_BASE_PATH=/ide) — assets zake
        // zote ni `/ide/assets/*`, hivyo wildcard inahudumia kutoka build/.
        .route("/ide", get(canvas_main))
        .route("/ide/", get(canvas_main))
        .route("/ide.html", get(canvas_main))
        .route("/ide/monaco/*path", get(monaco_asset))
        .route("/ide/*path", get(canvas_asset))
        // `/studio` = skrini MOJA yenye UI zote mbili (MTECHOS IDE + OpenHands).
        .merge(crate::studio::router())
        .fallback_service(ui.clone())
        .route("/manifest", get(root))
        .route("/health", get(health))
        // Deploy
        .route("/jobs", get(list_jobs))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/approve", post(approve_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/jobs/:id/install-report", post(install_report))
        .route("/jobs/:id/install-status", get(install_status))
        // Discovery + images
        .route("/images", get(list_images))
        .route("/images/:name", get(get_image))
        // OS selection (AI + rules)
        .route("/os/select", post(os_select))
        .route("/os/profiles", get(os_profiles))
        // Backup
        .route("/backups", get(list_backups))
        // Ripoti ya PDF (logo ya FUNDI)
        // FUNDI MAP (hardware scan halisi)
        // MULTI-AGENT orchestrator
        // LAN REMOTE
        // REMOTE OS INSTALL (app moja ya kwake: mteja ↔ agent)
        // Multicast ACK board
        .route("/api/mc/ack", post(mc_ack))
        .route("/api/mc/status", get(mc_status))
        // BUNDLES + PRICING + REACT (SEHEMU 2/3/13 — code kwa Rust)
        .route("/api/bundles", get(bundles_list))
        .route("/api/bundles", post(bundles_add))
        .route("/api/bundles/:id", axum::routing::delete(bundles_delete))
        .route("/api/bundles/apps", get(bundles_apps))
        .route("/api/bundles/distribute", post(bundles_distribute))
        .route("/api/secops/start", post(secops_start))
        .route("/api/secops/result", post(secops_result))
        .route("/api/secops/summary/:account", get(secops_summary))
        .route("/api/toolkit/catalog", get(toolkit_catalog))
        .route("/api/toolkit/run", post(toolkit_run))
        .route("/api/toolkit/batch", post(toolkit_batch))
        .route("/api/toolkit/runs", get(toolkit_runs))
        // SECURITY PACK (kutoka kali-meta debian/control) + SHIELD + POOL 100
        .route("/api/security/groups", get(security_groups))
        .route("/api/security/availability", get(security_availability))
        .route("/api/security/task", post(security_task))
        .route("/api/security/runs", get(security_runs))
        .route("/api/security/shield", get(security_shield))
        .route("/api/security/mass", post(security_mass))
        .route("/api/security/report", get(security_report))
        // ADMIN CONSOLE: chat ya cyber/forensics + ripoti yenye muhuri (ECDSA/SHA-256)
        .route("/api/admin-console/run", post(admin_console_run))
        .route("/api/admin-console/jobs", get(admin_console_jobs))
        .route("/api/admin-console/overview", get(admin_console_overview))
        // THREATSWARM (autonomous kill-chain, scope-gated, HITL-respecting)
        .route("/api/ts/engage", post(threatswarm_engage))
        .route("/api/ts/scope", post(threatswarm_scope_check))
        .route("/api/ts/reports", get(threatswarm_reports))
        .route("/api/ts/agents", get(threatswarm_agents))
        .route("/api/ts/agent", post(threatswarm_agent_run))
        // SCOPE MANAGEMENT (admin: RBAC inalinda /api/ts/scope/*)
        .route("/api/ts/scope/list", get(threatswarm_scope_list))
        .route("/api/ts/scope/add", post(threatswarm_scope_add))
        .route("/api/ts/scope/remove", post(threatswarm_scope_remove))
        // ---------- THREAT INTELLIGENCE (defensive, consent-based) ----------
        .route("/api/ti/analyze/sms", post(ti_analyze_sms))
        .route("/api/ti/analyze/url", post(ti_analyze_url))
        .route("/api/ti/analyze/caller", post(ti_analyze_caller))
        .route("/api/ti/blacklist", get(ti_blacklist))
        .route("/api/ti/blacklist/add", post(ti_blacklist_add))
        .route("/api/ti/blacklist/remove", post(ti_blacklist_remove))
        .route("/api/ti/patterns", get(ti_patterns))
        .route("/api/ti/patterns/add", post(ti_patterns_add))
        .route("/api/ti/summary/:account", get(ti_summary))
        .route("/api/ti/report", post(ti_report))
        .route("/api/ti/agents", get(ti_agents))
        .route("/api/ti/verify/callback", post(ti_verify_callback))
        // ---------- PHOENIX SHIELD (ULINZI/defensive — mtu binafsi au kampuni) ----------
        .route("/api/shield/posture", get(shield_posture))
        .route("/api/shield/lab/plan", post(shield_lab_plan))
        .route("/api/shield/harden/plan", post(shield_harden_plan))
        .route("/api/shield/analytics", post(shield_analytics))
        .route("/api/shield/coverage", get(shield_coverage))
        .route("/api/shield/agent", post(shield_agent))
        // TOTAL SECURITY MODULES — injini HALISI (nftables, sockets, syscalls)
        .route("/api/ts/catalog", get(ts_modules_catalog))
        .route("/api/ts/module/:id", post(ts_module_run))
        .route("/api/ts/module/:id/status", get(ts_module_status))
        // TOTAL SECURITY: usimamizi wa sheria/IOC/events (dynamic, Admin Mkuu)
        .route("/api/ts/rules", get(ts_rules_list))
        .route("/api/ts/rules/add", post(ts_rules_add))
        .route("/api/ts/rules/toggle", post(ts_rules_toggle))
        .route("/api/ts/rules/remove", post(ts_rules_remove))
        .route("/api/ts/ioc", get(ts_ioc_list))
        .route("/api/ts/ioc/add", post(ts_ioc_add))
        .route("/api/ts/events", get(ts_events_list))
        .route("/api/ts/feedback", post(ts_feedback_add))
        .route("/api/ts/stats", get(ts_store_stats))
        // TOTAL SECURITY: AKILI (LLM) — semantic analysis + rule proposal
        .route("/api/ts/ai/analyze", post(ts_ai_analyze))
        .route("/api/ts/ai/propose", post(ts_ai_propose))
        // ENGINE YA HESABU: matrix + calculus + quantum (Rust mtupu)
        .route("/api/quantum/run", post(quantum_run))
        .route("/api/quantum/random", post(quantum_random))
        // MIKATABA NA RIPOTI (HTML + muhuri wa SHA-256)
        // ===== PKI (ECDSA P-256 + SHA-256) — muhuri wa kampuni =====
        .route("/api/crypto/suite", get(crypto_suite))
        .route("/api/crypto/keys", get(crypto_keys))
        .route("/api/crypto/keys/rotate", post(crypto_rotate))
        .route("/api/crypto/sign", post(crypto_sign))
        .route("/api/crypto/verify", post(crypto_verify))
        .route("/api/crypto/seal", post(crypto_seal))
        .route("/api/crypto/verify-seal", post(crypto_verify_seal))
        .route("/api/crypto/ledger", get(crypto_ledger))
        .route("/api/crypto/ledger/verify", get(crypto_ledger_verify))
        // ===== WAFANYAKAZI (HR): kuajiri, mikataba, ruhusa, payroll =====
        // ===== ADMIN CONSOLE (cyber/forensics chat + ripoti yenye muhuri) =====
        // ===== AGENT-ENGINE: CYBER SECURITY + DIGITAL FORENSICS =====
        // akili-security (AGENT-ENGINE) — agents 27 huongoza zana zote.
        .route("/api/security/engine/status", get(sec_engine_status))
        .route("/api/security/platforms", get(sec_platforms))
        .route("/api/security/platforms/full", get(sec_platforms_full))
        .route("/api/security/platforms/lookup", get(sec_platforms_lookup))
        .route("/api/security/platforms/plan", post(sec_platforms_plan))
        .route("/api/security/finshield", post(sec_finshield))
        .route("/api/security/apiauth", post(sec_apiauth))
        .route("/api/security/gateprobe", post(sec_gateprobe))
        .route("/api/security/swarm/catalog", get(sec_swarm_catalog))
        .route("/api/security/swarm/plan", post(sec_swarm_plan))
        .route("/api/security/swarm/conduct", post(sec_swarm_conduct))
        .route("/api/security/agentbind", get(sec_agentbind))
        .route("/api/security/agentbind/finops", get(sec_agentbind_finops))
        .route("/api/security/agentbind/authops", get(sec_agentbind_authops))
        .route("/api/security/agentbind/plan", post(sec_agentbind_plan))
        .route("/api/security/agentbind/conduct", get(sec_agentbind_conduct))
        .route("/api/security/forensics", get(sec_forensics))
        // ===== PUBLIC (bure): fleet + react + brain =====
        .route("/api/fleet/register", post(fleet_register))
        .route("/api/fleet/heartbeat", post(fleet_heartbeat))
        .route("/api/fleet/poll", post(fleet_poll))
        .route("/api/fleet/report", post(fleet_report))
        .route("/api/fleet/agents", get(fleet_agents))
        .route("/api/react/start", post(react_start))
        .route("/api/react/sweep", post(react_sweep))
        .route("/api/brain/remember", post(brain_remember))
        // PFSENSE (H5b): firewall kwenye mtandao mmoja — rules/aliases/services/status kupitia API halisi
        .route("/api/pfsense/rules", get(pfsense_rules))
        .route("/api/pfsense/rules", post(pfsense_rule_add))
        .route("/api/pfsense/aliases", get(pfsense_aliases))
        .route("/api/pfsense/aliases", post(pfsense_alias_add))
        .route("/api/pfsense/services", post(pfsense_service_restart))
        .route("/api/pfsense/status", get(pfsense_status))
        .route("/api/system/disk", get(system_disk))
        // RBAC + AGENTIC MPYA: WOL, simu USB, mtandaoni, wamiliki wa PC
        .route("/api/mobile/usb", get(mobile_usb))
        .route("/api/web/search", get(web_search))
        .route("/api/web/searxng", get(web_searxng))
        .route("/api/web/fetch", get(web_fetch))
        // ONBOARDING (server/computer/raspberry) — config automatic
        // CREDITS (billed per huduma + subscription)
        // NETWORK + SECURITY MANAGEMENT (vifaa vya mtandao + usalama wa kampuni)
        .route("/api/brain/recall", get(brain_recall))
        .route("/api/brain/stats", get(brain_stats))
        // TAKWIMU (engine ya Rust — mbadala wa R) + MATENDO YA KABLA + DEEP SCAN + IPAM
        .route("/api/analytics/summary", get(analytics_summary))
        .route("/api/deepscan", post(deepscan_run))
        .route("/api/deepscan/runs", get(deepscan_runs))
        // R ENGINE (uchambuzi + visualization halisi kwa R)
        .route("/api/r/engine", post(r_engine_run))
        // WASILIANO WA MSAAADA (Mbilinyi Tech)
        // MAONI YA AI (thumbs up/down) — huhifadhiwa DB
        .route("/api/feedback", post(feedback::submit))
        .route("/api/feedback/summary", get(feedback::summary))
        // INDEX YA VECTOR: DEPLOYMENTS + COMPUTERS → LanceDB
        .route("/api/memindex/rebuild", post(memindex_rebuild))
        .route("/api/memindex/stats", get(memindex_stats))
        .route("/api/fleet/exec", post(fleet_exec))
        // AUTH + ADMIN ROLES + LICENSE + MODE (H3)
        .route("/api/auth/login", post(auth_login))
        .route("/api/auth/verify", post(auth_verify))
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/auth/password", post(auth_password))
        // CUSTOM MODEL / API (mteja anaweka model yake au API yake — inatumika mara moja)
        .route("/api/ai/custom", get(ai_custom_get))
        .route("/api/ai/custom", post(ai_custom_set))
        .route("/api/ai/custom", axum::routing::delete(ai_custom_clear))
        // OLLAMA CLOUD: API key (faili) + model zote za free + majaribio halisi
        .route("/api/ai/key", post(ai_key_set))
        .route("/api/ai/cloud-models", get(ai_cloud_models))
        .route("/api/ai/test", post(ai_test))
        // LLAMA.CPP + LOCAL MODELS (automatiki: pakua -> washa -> configure; hakuna training)
        .route("/api/llm/status", get(llm_status))
        .route("/api/llm/models", get(llm_models))
        .route("/api/llm/download", post(llm_download))
        .route("/api/llm/download/status", get(llm_download_status))
        .route("/api/llm/llamacpp/setup", post(llm_llamacpp_setup))
        .route("/api/llm/start", post(llm_start))
        .route("/api/llm/stop", post(llm_stop))
        .route("/api/llm/use", post(llm_use))
        .route("/api/bundles/fetch", post(bundles_fetch))
        // DATA/ + AI CATALOG (AI inapata folder lote la data + models.json)
        .route("/api/ai/catalog", get(ai_catalog))
        .route("/api/data/tree", get(data_tree))
        .route("/api/data/file", get(data_file))
        // CHAT + AUTO-DAILY + UPDATES (H3)
        // MTIRIRIKO: agent inaonyesha hatua zake (thinking + actions) moja kwa moja
        // FUNDI AGENT BRIDGE (REST + status): API za Fundi Agent yenye tool registry
        .route("/api/agent/health", get(agent_health))
        .route("/api/agent/config", get(agent_config))
        .route(
            "/api/agent/sessions",
            get(agent_list_sessions).post(agent_create_session),
        )
        .route(
            "/api/agent/sessions/:id/messages",
            get(agent_list_messages).post(agent_send_message),
        )
        .route("/api/agent/chat", post(agent_chat))
        .route("/api/agent/image", post(agent_image_gen))
        .route("/api/agent/vision", post(agent_vision))
        .route("/api/code/run", post(agent_code_run))
        .route("/api/ide/tree", get(crate::ide_api::ide_tree))
        .route("/api/ide/file", get(crate::ide_api::ide_file_read).post(crate::ide_api::ide_file_write))
        .route("/api/ide/search", post(crate::ide_api::ide_search))
        .route("/api/ide/delete", post(crate::ide_api::ide_delete))
        .route("/api/ide/mkdir", post(crate::ide_api::ide_mkdir))
        .route("/api/ide/rename", post(crate::ide_api::ide_rename))
        .route("/api/ide/languages", get(ide_languages))
        .route("/api/ide/tools-groups", get(tools_groups))
        .route("/api/ide/tools/:group", get(tools_group_detail))
        .route("/api/ide/tool-calls", get(tool_calls_summary))
        .route("/api/ide/tool/:group", get(tool_calls_group))
        .route("/api/ide/tool-execute", post(tool_execute))
        .route("/api/ide/terminal", post(crate::ide_api::ide_terminal))
        // AKILI ENGINE (ubongo halisi kwenye server: memory SQLite, knowledge,
        // patterns, code index, ToT, constitution, verifier HALISI — compilers)
        .route("/api/akili/status", get(crate::akili::akili_status))
        .route("/api/akili/remember", post(crate::akili::akili_remember))
        .route("/api/akili/recall", get(crate::akili::akili_recall))
        .route(
            "/api/akili/knowledge",
            get(crate::akili::akili_knowledge_query).post(crate::akili::akili_knowledge_add),
        )
        .route("/api/akili/patterns", get(crate::akili::akili_patterns_list))
        .route("/api/akili/patterns/apply", post(crate::akili::akili_pattern_apply))
        .route("/api/akili/verify", post(crate::akili::akili_verify))
        .route("/api/akili/graph", get(crate::akili::akili_graph))
        .route("/api/akili/constitution", get(crate::akili::akili_constitution))
        .route("/api/akili/reflect", post(crate::akili::akili_reflect))
        .route("/api/akili/plan", post(crate::akili::akili_plan))
        .route("/api/akili/plans", get(crate::akili::akili_plans))
        .route("/api/akili/tot", post(crate::akili::akili_tot_solve))
        .route("/api/akili/run", post(crate::akili::akili_pipeline))
        // AKILI V3.0 — TABAKA 7 (memories 3, MCTS/Beam/A*, causal/world/
        // counterfactual, analogy/blending, team/negotiation/ToM, meta/
        // curriculum/emergence/formal + R layer HALISI)
        .route("/api/akili/v3/status", get(crate::akili_v3::v3_status))
        .route("/api/akili/v3/solve", post(crate::akili_v3::v3_solve))
        .route("/api/akili/v3/episode", post(crate::akili_v3::v3_episode_add))
        .route("/api/akili/v3/recall", get(crate::akili_v3::v3_recall))
        .route("/api/akili/v3/concept", post(crate::akili_v3::v3_concept_add))
        .route("/api/akili/v3/concepts", get(crate::akili_v3::v3_concepts_query))
        .route("/api/akili/v3/skill", post(crate::akili_v3::v3_skill_add))
        .route("/api/akili/v3/skills", get(crate::akili_v3::v3_skills_list))
        .route("/api/akili/v3/mcts", post(crate::akili_v3::v3_mcts))
        .route("/api/akili/v3/beam", post(crate::akili_v3::v3_beam))
        .route("/api/akili/v3/astar", post(crate::akili_v3::v3_astar))
        .route("/api/akili/v3/why", post(crate::akili_v3::v3_causal_why))
        .route("/api/akili/v3/whatif", post(crate::akili_v3::v3_causal_whatif))
        .route("/api/akili/v3/world", post(crate::akili_v3::v3_world_simulate))
        .route("/api/akili/v3/counterfactual", post(crate::akili_v3::v3_counterfactual))
        .route("/api/akili/v3/analogy", post(crate::akili_v3::v3_analogy))
        .route("/api/akili/v3/blend", post(crate::akili_v3::v3_blend))
        .route("/api/akili/v3/team", post(crate::akili_v3::v3_team))
        .route("/api/akili/v3/negotiate", post(crate::akili_v3::v3_negotiate))
        .route("/api/akili/v3/tom", post(crate::akili_v3::v3_tom))
        .route("/api/akili/v3/meta", post(crate::akili_v3::v3_meta))
        .route("/api/akili/v3/curriculum", post(crate::akili_v3::v3_curriculum))
        .route("/api/akili/v3/curriculum/list", get(crate::akili_v3::v3_curriculum_list))
        .route("/api/akili/v3/emergence", get(crate::akili_v3::v3_emergence_get))
        .route("/api/akili/v3/formal", post(crate::akili_v3::v3_formal))
        .route("/api/akili/v3/r", post(crate::akili_v3::v3_r_analyze))
        .route("/api/akili/v4/status", get(crate::akili_v4::v4_status))
        .route("/api/akili/v4/live", post(crate::akili_v4::v4_live))
        .route("/api/akili/v4/gate", post(crate::akili_v4::v4_gate))
        .route("/api/akili/human/status", get(crate::akili_human::human_status))
        .route("/api/akili/human/think", post(crate::akili_human::human_live))
        .route("/api/akili/v6/status", get(crate::akili_v6::v6_status))
        .route("/api/akili/v6/layers", get(crate::akili_v6::v6_layers))
        .route("/api/akili/v6/live", post(crate::akili_v6::v6_live))
        .route("/api/akili/v6/gate", post(crate::akili_v6::v6_gate))
        .route("/api/akili/v6/r", post(crate::akili_v6::v6_r))
        .route("/api/ide/r", post(crate::ide_api::ide_r_analyze))
        .route("/api/ide/agent", post(ide_agent_chat))
        .route("/api/ide/agent/stream", post(ide_agent_chat_stream))
        .route("/api/ide/agent/run", post(ide_agent_run))
        .route("/api/ide/agent/agentic", post(ide_agent_agentic))
        .route("/api/ide/agent/agentic/stream", post(ide_agent_agentic_stream))
        .route("/api/ide/agent/agentic/result", get(ide_agent_agentic_result))
        .route("/api/ide/agent/build/run", post(ide_agent_build_run))
        .route("/api/ide/agent/status", get(ide_agent_status))
        .route("/api/ide/skills", get(ide_skills_list))
        .route("/api/nyi", get(nyi_info))
        .route("/api/radi", get(radi_info))
        .route("/api/nyi/media/status", get(nyi_media_status))
        .route("/api/nyi/chart", post(nyi_chart))
        .route("/api/nyi/video/probe", post(nyi_video_probe))
        .route("/api/nyi/video/make", post(nyi_video_make))
        .route("/api/nyi/finetune", get(nyi_ft_info))
        .route("/api/nyi/finetune/build", post(nyi_ft_build))
        .route("/api/nyi/finetune/script", post(nyi_ft_script))
        .merge(crate::ide_emulators::router())
        .merge(crate::ide_lab::router())
        .merge(crate::education::router())
        .merge(crate::lesson::router())
        .merge(crate::elimupro::router())
        .merge(crate::teaching::router())
        .merge(crate::live::router())
        .merge(crate::proctor::router())
        .merge(crate::marks::router())
        .merge(crate::attendance::router())
        .merge(crate::fees::router())
        .merge(crate::library::router())
        .merge(crate::hostel::router())
        .merge(crate::transcript::router())
        .merge(crate::offline::router())
        .merge(crate::voip::router())
        .merge(crate::clearance::router())
        .merge(crate::maps::router())
        .merge(crate::scribe::router())
        .merge(crate::localai::router())
        .merge(crate::weather::router())
        .merge(crate::search::router())
        .merge(crate::e2ee::router())
        .merge(crate::compsol::router())
        .merge(crate::akademia::router())
        .merge(crate::ittools::router())
        .merge(crate::aiengine::router())
        .merge(crate::malipo::router())
        .merge(crate::exam_lan::router())
        .merge(crate::tetea::router())
        .merge(crate::code_audit::router())
        .merge(crate::ide_swarm::router())
        .merge(crate::agents_unified::router())
        .merge(crate::script_agent::router())
        .merge(crate::swarm_v2::router())
        .merge(crate::swarm_v3::router())
        .merge(crate::ide_devices::router())
        .merge(crate::ide_testkit::router())
        .merge(crate::ide_build::router())
        .merge(crate::ide_engineer::router())
        .merge(crate::ide_lsp::router())
        .merge(crate::ide_complete::router())
        .merge(crate::ide_composer::router())
        .merge(crate::ide_dap::router())
        .merge(crate::ide_e2e::router())
        .merge(crate::ide_pipeline::router())
        .merge(crate::sec_gate::router())
        .merge(crate::sdlc::router())
        .merge(crate::super_engineer::router())
        .merge(crate::engines_canvas::router())
        .route("/api/engines/unified/status", get(crate::unified_engines::unified_status))
        .route("/api/engines/unified/capabilities", get(crate::unified_engines::unified_capabilities))
        .route("/api/engines/unified/skills", get(crate::unified_engines::unified_skills))
        .merge(crate::canvas_ext::router())
        // MTECHOS GROUP: biashara, malipo/escrow, ajira, SMS, kampuni ya wakala.
        .merge(crate::biashara::router())
        .merge(crate::escrow::router())
        .merge(crate::ajira::router())
        .merge(crate::sms::router())
        .merge(crate::agentkampuni::router())
        // MTS-CORE PROXY (same-origin): Canvas App huzungumza na `/mts/*` kwenye
        // origin hii, na hapa tunapeleka kwa mts-core (`MTS_CORE_URL`). Hii
        // inafanya cookies za core (Path=/) na `SameSite=Lax` kufanya kazi kwa
        // mgeni — loopback `127.0.0.1` kwenye HTML ingevunja kila wito.
        .route("/mts/*path", axum::routing::any(crate::mts_proxy::proxy))
        // PROTOCOL YA AGENT-SERVER (RUST-NATIVE): /api/* + /sockets/* kwa Rust
        // PEKEE — badala ya daraja la Python (:18000). Hii hutoa conversation
        // protocol (conversations, settings/secrets, profiles, llm, file, git,
        // workspaces, sockets) ambayo Canvas (/ide) inahitaji.
        .merge(crate::agent_native::router(state.clone()))
        // PROTOCOL YA RUST-NATIVE (/api/native/*) — uso wa ziada wa Rust
        // (hauna Python; hutoa exec/think kwa Go/Elixir sidecars).
        .merge(crate::native_protocol::router())
        .merge(crate::agentic_bridge::router())
        .merge(crate::ide_integrations::router())
        .merge(crate::forge::router())
        .route("/api/ide/selfcheck", post(crate::code_rules::ide_selfcheck))
        .route("/api/ide/build-pipeline", post(crate::build_pipeline::run))
        .route("/api/ide/shadow", post(crate::shadow::api))
        .route("/api/ide/hotpatch", post(crate::hotpatch::api))
        .route("/api/ide/timetravel/list", get(crate::timetravel::api_list))
        .route("/api/ide/timetravel/revert", post(crate::timetravel::api_revert))
        .route("/api/ide/timetravel/bisect", post(crate::timetravel::api_bisect))
        .route("/api/ide/selfcheck/rules", get(crate::code_rules::rules_view))
        .route("/api/engine/status", get(crate::engine_api::status))
        .route("/api/engine/models", get(crate::engine_api::models))
        .route("/api/engine/load", post(crate::engine_api::load))
        .route("/api/engine/stream", post(crate::engine_api::stream))
        .route("/api/engine/snn", get(crate::engine_api::snn_info))
        .route("/api/engine/snn/prefill", post(crate::engine_api::snn_prefill))
        .route("/api/engine/snn/chat", post(crate::engine_api::snn_chat))
        .route("/api/engine/cortex/info", get(crate::engine_api::cortex_info))
        .route("/api/engine/cortex/prefill", post(crate::engine_api::cortex_prefill))
        .route("/api/engine/precision/roundtrip", post(crate::engine_api::precision_roundtrip))
        .route("/api/ide/arch", get(crate::code_rules::ide_arch))
        .route("/api/daily/report", get(daily_report))
        .route("/api/daily/permission", post(daily_permission))
        // LANGUAGE + REAL REMOTING (maelezo ya mmiliki)
        .route("/api/language", post(lang_set))
        .route("/api/language/:username", get(lang_get))
        .route("/api/translate", post(translate_post))
        // COMPANY (kampuni/matawi kupitia wg0)
        // WIREGUARD VPN (P4) — kazi zote za mbali kupitia wg0
        .route("/api/vpn/init", post(vpn_init))
        .route("/api/vpn/status", get(vpn_status))
        .route("/api/vpn/peers", get(vpn_peers))
        .route("/api/vpn/peers", post(vpn_peer_add))
        .route("/api/vpn/peers/:name/conf", get(vpn_peer_conf))
        .route(
            "/api/vpn/peers/:name",
            axum::routing::delete(vpn_peer_delete),
        )
        .route("/api/vpn/up", post(vpn_up))
        .route("/api/vpn/down", post(vpn_down))
        .route("/api/vpn/scan", get(vpn_scan))
        .route("/api/vpn/server-conf", get(vpn_server_conf))
        // Agents
        .route("/agents", get(list_agents))
        // Supervisor
        .route("/supervisor/summary", get(supervisor_summary))
        // FUNDI MOBILE (Daktari wa Simu): frontend yake inaita /api/* halisi
        .route("/api/jobs", get(list_jobs))
        // BSOD FIXER — uchunguzi wa BugCheck halisi + mpango HITL
        // RBAC MWISHO (middleware hushika njia ZOTE zilizotajwa hapo juu):
        // route nyeti zinahitaji session; baadhi (approve/license/fleet) — Admin Mkuu.
        .layer(axum::middleware::from_fn(static_headers_mw))
        .layer(axum::middleware::from_fn_with_state(state.clone(), rbac_mw))
        .layer(CorsLayer::permissive())
        // Gzip: UI ni SPA yenye assets ~3MB; bila hii kila load hupakua raw.
        .layer(CompressionLayer::new())
        .with_state(state);

    tracing::info!(
        "MTECHOS Agent v3 :8080 — UI /ide"
    );
    // Bind: FUNDI_BIND (host:port) > PORT (port pekee) > 0.0.0.0:8080.
    // Hosted/cloud runtimes hufichua port moja pekee; bila hii UI hutoa
    // "Server haipatikani" kwa sababu server haisikilizi port inayofichuliwa.
    let bind_addr = match std::env::var("FUNDI_BIND") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => match std::env::var("PORT") {
            Ok(p) if !p.trim().is_empty() => format!("0.0.0.0:{}", p.trim()),
            _ => "0.0.0.0:8080".to_string(),
        },
    };
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("listening on {bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}


/// DEPLOY TEMPLATES: domains zote (website/mobile/iot/gaming) — UI dropdown.
async fn deploy_domains() -> Json<serde_json::Value> {
    Json(crate::deploy_templates::list_domains())
}

/// DEPLOY PLAN ya domain moja: steps + commands + options (kwa agent + UI).
async fn deploy_plan(
    axum::extract::Path(domain): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    Json(crate::deploy_templates::plan_for(&domain))
}

/// ZED TOOLS CATALOG: makundi yote ya backend tools (20, kwa UI dropdown).
async fn tools_groups() -> Json<serde_json::Value> {
    Json(crate::tools_catalog::summary())
}

/// ZED TOOLS: tools zote za kundi moja (agent/web/llm/...).
async fn tools_group_detail(
    axum::extract::Path(group): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let data = crate::tools_catalog::load();
    let g = &data["groups"][&group];
    if g.is_null() {
        return Json(json!({ "ok": false, "error": "kundi haipatikani" }));
    }
    Json(json!({ "ok": true, "group": group, "label": g["label_sw"], "icon": g["icon"], "tools": g["tools"] }))
}

/// TOOL CALLS: 20 makundi, 235 tools (katalogi).
async fn tool_calls_summary() -> Json<serde_json::Value> {
    Json(crate::tool_calls::groups_summary())
}

/// TOOL CALLS: tools za kundi moja + schemas.
async fn tool_calls_group(
    axum::extract::Path(group): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    Json(crate::tool_calls::group_tools(&group))
}

/// TOOL EXECUTE: endesha tool HALISI (agent layer ama native).
async fn tool_execute(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = r["tool"].as_str().unwrap_or("").to_string();
    if name.is_empty() {
        return Json(json!({ "ok": false, "error": "tool ni lazima" }));
    }
    Json(crate::tool_calls::execute(&s, &name, r["params"].clone()).await)
}


async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "name": "MTECHOS",
        "product": "MTECHOS",
        "main_modules": ["CODING IDE", "CYBERSECURITY & DIGITAL FORENSICS"],
        "licensed_by": "Mbilinyi Tech (mbilinyitech.co.tz)",
        "version": "3.0.0",
        "role": "Agent inafanya kazi; msimamizi anasimamia (approve/cancel)",
        "features": ["coding-ide", "monaco-editor", "autopilot", "agent-swarm", "cybersecurity", "digital-forensics", "threatswarm", "scope-gate", "real-remoting", "agentic-ai-react", "neuralis-brain-vector", "language-ai-translate", "network-device-mgmt"],
        "ui": "/ide"
    }))
}


/// Folda ya Monaco (offline): `node_modules/monaco-editor/min`. Huhudumiwa
/// chini ya `/ide/monaco/*` (mfano `/ide/monaco/vs/loader.js`) ili editor
/// ifanye kazi bila internet/CDN.
fn monaco_dir() -> Option<std::path::PathBuf> {
    let mut cands: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("engines/mbilinyi/node_modules/monaco-editor/min"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            cands.push(d.join("engines/mbilinyi/node_modules/monaco-editor/min"));
            dir = d.parent();
        }
    }
    cands.into_iter().find(|c| c.join("vs").join("loader.js").exists())
}

/// `/ide/monaco/*` — Monaco (min/vs) kwa offline. Kanuni: hakuna `..`.
async fn monaco_asset(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if path.is_empty() || path.contains("..") {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    let rel = path.trim_start_matches('/');
    let ctype = match rel.rsplit('.').next().unwrap_or("") {
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "svg" => "image/svg+xml",
        "html" => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    };
    match monaco_dir() {
        Some(dir) => match std::fs::read(dir.join(rel)) {
            Ok(bytes) => (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static(ctype))],
                bytes,
            )
                .into_response(),
            Err(_) => axum::http::StatusCode::NOT_FOUND.into_response(),
        },
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}


/// `/` inapeleka kwenye canvas (`/ide`).
async fn canvas_redirect() -> axum::response::Redirect {
    axum::response::Redirect::temporary("/ide")
}


async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": "2.0.0",
        "services": ["pxe", "tftp", "http", "smb", "ai", "wol", "backup", "multicast"],
    }))
}

// ---------- jobs ----------

async fn list_jobs(State(s): State<AppState>) -> Json<Vec<Job>> {
    Json(s.jobs.read().await.clone())
}

async fn get_job(State(s): State<AppState>, Path(id): Path<String>) -> Json<Option<Job>> {
    Json(s.jobs.read().await.iter().find(|j| j.id == id).cloned())
}

async fn approve_job(State(s): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let mut jobs = s.jobs.write().await;
    if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
        if j.status == "awaiting_approval" {
            j.status = "approved".into();
            j.needs_approval = false;
            j.message = "Imeidhinishwa na msimamizi".into();
            let _ = sqlx::query("UPDATE jobs SET status=?, message=? WHERE id=?")
                .bind("approved")
                .bind("Imeidhinishwa na msimamizi")
                .bind(&id)
                .execute(&s.db)
                .await;
            return Json(json!({ "ok": true, "id": id, "status": "approved" }));
        }
        return Json(
            json!({ "ok": false, "error": "Job si awaiting_approval", "status": j.status }),
        );
    }
    Json(json!({ "ok": false, "error": "Job haipo" }))
}

async fn cancel_job(State(s): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    pipeline::set_job(
        &s.db,
        &s.jobs,
        &id,
        "cancelled",
        "cancel",
        0,
        "Imeghairiwa na msimamizi",
    )
    .await;
    Json(json!({ "ok": true, "id": id, "status": "cancelled" }))
}

#[derive(serde::Deserialize)]
struct InstallReportReq {
    mac: String,
    #[serde(default)]
    os: String,
    ok: bool,
    #[serde(default)]
    message: String,
}

/// Client ya PXE (live/initramfs) inaripoti matokeo ya kuandika disk — hii
/// inafanya hatua ya INSTALL kuwa halisi (server husubiri ripoti hii).
async fn install_report(
    Path(id): Path<String>,
    Json(r): Json<InstallReportReq>,
) -> Json<serde_json::Value> {
    pipeline::record_install(pipeline::InstallReport {
        mac: r.mac.clone(),
        os: r.os.clone(),
        ok: r.ok,
        message: r.message.clone(),
        installed_at: chrono::Local::now().to_rfc3339(),
    });
    Json(json!({ "ok": true, "job": id, "mac": r.mac, "recorded": true }))
}

/// Hali ya install kwa MAC (ripoti ya mwisho kutoka client).
async fn install_status(Path(_id): Path<String>, Query(q): Query<std::collections::HashMap<String, String>>) -> Json<serde_json::Value> {
    let mac = q.get("mac").cloned().unwrap_or_default();
    match pipeline::install_status(&mac) {
        Some(rep) => Json(json!({ "ok": true, "report": rep })),
        None => Json(json!({ "ok": false, "report": null, "mac": mac })),
    }
}

// ---------- deploy ----------


// ---------- discovery ----------


// ---------- images ----------

async fn list_images() -> Json<serde_json::Value> {
    let imgs = images::list_images();
    let valid = imgs.iter().filter(|i| i.valid).count();
    Json(
        json!({ "path": images::images_root(), "count": imgs.len(), "valid": valid, "images": imgs }),
    )
}

async fn get_image(
    State(_s): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    match images::find_image(&name) {
        Some(i) => Json(json!({ "image": i })),
        None => Json(json!({ "error": "haipatikani", "name": name })),
    }
}

// ---------- os selection ----------

#[derive(serde::Deserialize)]
struct OsSelectReq {
    specs: String,
    #[serde(default)]
    user_need: Option<String>,
}

async fn os_select(Json(req): Json<OsSelectReq>) -> Json<serde_json::Value> {
    let d = osselect::decide_full(&req.specs, req.user_need.as_deref().unwrap_or("office")).await;
    Json(json!({ "decision": d }))
}

async fn os_profiles() -> Json<serde_json::Value> {
    let cat = osselect::catalog();
    Json(json!({ "profiles": cat.profiles.len(), "skip_rules": cat.skip_rules }))
}

// ---------- backups ----------

async fn list_backups() -> Json<serde_json::Value> {
    let root = backup::backup_root();
    let mut out = Vec::new();
    if let Ok(macdirs) = std::fs::read_dir(&root) {
        for md in macdirs.flatten() {
            if let Ok(stamps) = std::fs::read_dir(md.path()) {
                for st in stamps.flatten() {
                    let mf = st.path().join("manifest.json");
                    if let Ok(txt) = std::fs::read_to_string(&mf) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                            out.push(v);
                        }
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| {
        let ka = a["finished"].as_str().unwrap_or("");
        let kb = b["finished"].as_str().unwrap_or("");
        kb.cmp(ka)
    });
    Json(json!({ "root": root, "count": out.len(), "backups": out }))
}

// ---------- ripoti ya PDF (logo ya FUNDI) ----------


// ---------- FUNDI MAP (hardware scan halisi) ----------


// ---------- MULTI-AGENT orchestrator ----------

#[derive(serde::Deserialize)]
struct OrchLaunchReq {
    targets: Vec<String>,
    #[serde(default = "default_concurrency")]
    max_concurrent: usize,
}
fn default_concurrency() -> usize {
    32
}





// ---------- LAN REMOTE ----------


// ---------- REMOTE OS INSTALL (app moja ya kwake) ----------








// ---------- Bundles + Pricing + ReAct (Rust logic) ----------

async fn bundles_list(State(s): State<AppState>) -> Json<serde_json::Value> {
    let cat = bundles::catalog();
    let custom = bundles::list_custom_bundles(&s.db).await;
    Json(json!({
        "categories": bundles::Category::all().iter().map(|c| json!({
            "id": c.id(), "name_sw": c.name_sw(), "icon": c.icon(),
            "bundles": cat.iter().filter(|b| b.category == *c).map(|b| json!({
                "id": b.id, "name_sw": b.name_sw, "description": b.description, "version": bundles::version_of(&b.id),
                "apps": b.apps.iter().map(|ap| json!({ "id": ap.id, "name": ap.name, "critical": ap.critical }))
                    .collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "total_bundles": cat.len(),
        "custom_bundles": custom,
    }))
}

async fn bundles_apps() -> Json<serde_json::Value> {
    Json(
        json!({ "apps": bundles::catalog().into_iter().flat_map(|b| b.apps)
        .map(|ap| json!({ "id": ap.id, "name": ap.name, "critical": ap.critical }))
        .collect::<Vec<_>>() }),
    )
}

#[derive(serde::Deserialize)]
struct BundleAddReq {
    id: String,
    name_sw: String,
    #[serde(default)]
    description: String,
    category: String,
    app_ids: Vec<String>,
    #[serde(default = "default_creator")]
    created_by: String,
}
fn default_creator() -> String {
    "admin (dashboard)".into()
}

async fn bundles_add(
    State(s): State<AppState>,
    Json(r): Json<BundleAddReq>,
) -> Json<serde_json::Value> {
    match bundles::add_custom_bundle(
        &s.db,
        &r.id,
        &r.name_sw,
        &r.description,
        &r.category,
        r.app_ids,
        &r.created_by,
    )
    .await
    {
        Ok(b) => Json(json!({ "ok": true, "bundle": b })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn bundles_delete(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let ok = bundles::remove_custom_bundle(&s.db, &id).await;
    Json(json!({ "ok": ok, "id": id }))
}

#[derive(serde::Deserialize)]
struct ReactStartReq {
    problem: String,
    #[serde(default)]
    targets: Vec<String>,
}

async fn react_start(Json(r): Json<ReactStartReq>) -> Json<serde_json::Value> {
    let mut sess = reacon::ReActSession::new(&r.problem, r.targets);
    sess.think_plan();
    Json(json!({ "ok": true, "session": sess }))
}

#[derive(serde::Deserialize)]
struct ReactSweepReq {
    problem: String,
    targets: Vec<String>,
}

async fn react_sweep(Json(r): Json<ReactSweepReq>) -> Json<serde_json::Value> {
    // THINK → ACT (ping sweep halisi /24, low-risk) → OBSERVE
    let cfg = vpn::Config::load();
    let (a, b, c) = cfg.subnet_base();
    let alive = reacon::ping_sweep((a, b, c), 400).await;
    let mut sess = reacon::ReActSession::new(&r.problem, r.targets);
    sess.think_plan();
    sess.observe_sweep(&alive);
    Json(json!({
        "ok": true,
        "session": sess,
        "alive": alive,
        "note_sw": "ReAct THINK→ACT→OBSERVE: sweep ni low-risk (automatic). Install/reboot zinahitaji RUHUSU (bounded autonomy)."
    }))
}

// ---------- Neuralis Brain (kumbukumbu ya pamoja ya agents) ----------

#[derive(serde::Deserialize)]
struct BrainRememberReq {
    agent: String,
    pc: String,
    problem: String,
    solution: String,
    confidence: f64,
}

async fn brain_remember(
    State(s): State<AppState>,
    Json(r): Json<BrainRememberReq>,
) -> Json<serde_json::Value> {
    match brain::remember(
        &s.brain,
        &r.agent,
        &r.pc,
        &r.problem,
        &r.solution,
        r.confidence,
    )
    .await
    {
        Ok(id) => Json(json!({ "ok": true, "id": id })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn brain_recall(
    State(s): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let Some(query) = q.get("q") else {
        return Json(json!({ "ok": false, "error": "q ni lazima (?q=wi-fi haifanyi kazi)" }));
    };
    let limit: usize = q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(5);
    let got = brain::recall(&s.brain, query, limit).await;
    Json(json!({
        "ok": true,
        "query": query,
        "results": got.iter().map(|(m, score)| json!({
            "agent": m.agent, "pc": m.pc, "problem": m.problem,
            "solution": m.solution, "confidence": m.confidence,
            "match_score": (score * 1000.0).round() / 1000.0,
        })).collect::<Vec<_>>(),
        "note_sw": "Kumbukumbu za agents wenzako — kila agent anajifunza kutoka kwa wengine (Neuralis Brain)."
    }))
}

async fn brain_stats(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(brain::stats(&s.brain).await)
}

// ---------- TAKWIMU (engine ya Rust) + PREDICTIVE + DEEPSCAN + IPAM + MSAAADA ----------

async fn analytics_summary(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(analytics::summary(&s.db).await)
}


#[derive(serde::Deserialize)]
struct DeepscanReq {
    target: String,
    ip: String,
    #[serde(default = "default_ds_timeout")]
    timeout_ms: u64,
}
fn default_ds_timeout() -> u64 {
    300
}

async fn deepscan_run(
    State(s): State<AppState>,
    Json(r): Json<DeepscanReq>,
) -> Json<serde_json::Value> {
    if r.target.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "target (jina la PC) ni lazima" }));
    }
    Json(deepscan::run(&s.db, r.target.trim(), r.ip.trim(), r.timeout_ms).await)
}

/// R ENGINE: uchambuzi wa hali ya juu + visualization (chart PNG, image,
/// audio, forecast) kwa Rscript. Kila model/tool/agent inaotumia `/api/r/engine`.
async fn r_engine_run(
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let task = r["task"].as_str().unwrap_or("").trim().to_string();
    if task.is_empty() {
        return Json(json!({
            "ok": false,
            "error": "task ni lazima (summary | chart | image | audio | forecast | custom)",
        }));
    }
    if task == "custom" {
        // Custom R code ni sandbox-only (IDE emulator / HITL) — si free API.
        return Json(json!({
            "ok": false,
            "error": "task 'custom' inahitaji HITL (sandbox ya IDE). Tumia summary/chart/image/audio/forecast."
        }));
    }
    Json(r_engine::run_req(&task, &r).await)
}

async fn deepscan_runs(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(deepscan::history(&s.db).await)
}


#[derive(serde::Deserialize)]
struct IpamAssignReq {
    pc: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    ip: String,
    #[serde(default = "default_subnet")]
    subnet: String,
}
fn default_subnet() -> String {
    "192.168.1.0/24".into()
}


#[derive(serde::Deserialize)]
struct IpamReleaseReq {
    pc: String,
}


// ---------- INDEX YA VECTOR (LanceDB): deployments + computers ----------
// Data dir ni ile ile ya Brain ("/data") — sawa na lancedb/brain_memories.lance.

async fn memindex_rebuild(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(memindex::rebuild(&s.db, "/data").await)
}

async fn memindex_stats(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(memindex::stats(&s.db, "/data").await)
}

// ---------- Fleet exec (ombi moja → OS nyingi, kama RDM) ----------

#[derive(serde::Deserialize)]
struct FleetExecReq {
    /// mfano: [{"os":"win11","apps":["util-anydesk"]},{"os":"ubuntu","apps":["dev-python"]}]
    os_groups: Vec<OsGroup>,
    #[serde(default)]
    human_approved: bool,
}
#[derive(serde::Deserialize)]
struct OsGroup {
    os: String,
    apps: Vec<String>,
}

async fn fleet_exec(Json(r): Json<FleetExecReq>) -> Json<serde_json::Value> {
    // Bounded autonomy: install = High risk → lazima idhini ya binadamu
    if !reacon::action_allowed(reacon::Action::InstallApp, r.human_approved) {
        return Json(json!({
            "ok": false,
            "error": "InstallApp ni High-risk — lazima idhini ya binadamu (human_approved: true). Hii ni bounded autonomy.",
            "status": "awaiting_approval"
        }));
    }
    let groups: Vec<(String, Vec<String>)> =
        r.os_groups.into_iter().map(|g| (g.os, g.apps)).collect();
    let plans = reacon::multi_os_plan(groups);
    Json(json!({
        "ok": true,
        "plans": plans.iter().map(|p| json!({
            "target": p.display_name, "os": p.os,
            "apps": p.app_ids,
            "install_commands": p.install_commands(),
        })).collect::<Vec<_>>(),
        "note_sw": "Ombi moja → OS nyingi kwa wakati mmoja (kama RDM AI assistant)."
    }))
}

// ---------- Auth (login/roles) ----------

#[derive(serde::Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

// AUTH VERIFY: UI inaita hii kila load kupima session (Bearer token).
// Ndani: token halali → { ok: true, session } (UI inaonyesha dashboard).
async fn auth_verify(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string))
        .unwrap_or_default();
    match auth::verify_token(&s.db, &token).await {
        Some(sess) => Json(json!({ "ok": true, "session": {
            "token": sess.token, "username": sess.username, "role": sess.role,
            "role_name": if sess.role == "admin_mkuu" { "Admin Mkuu" } else { "Msaidizi" },
            "can_assign": sess.role == "admin_mkuu", "can_approve_high_risk": sess.role == "admin_mkuu",
        } })).into_response(),
        None => (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "error": "Session imekadatuliwa au imeisha" })),
        )
            .into_response(),
    }
}

async fn auth_login(State(s): State<AppState>, Json(r): Json<LoginReq>) -> Json<serde_json::Value> {
    match auth::login(&s.db, &r.username, &r.password).await {
        Ok(sess) => Json(json!({ "ok": true, "session": {
            "token": sess.token, "username": sess.username, "role": sess.role,
            "role_name": if sess.role == "admin_mkuu" { "Admin Mkuu" } else { "Msaidizi" },
            "can_assign": sess.role == "admin_mkuu", "can_approve_high_risk": sess.role == "admin_mkuu",
        } })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

#[derive(serde::Deserialize)]
struct TokenReq {
    token: String,
}

async fn auth_logout(
    State(s): State<AppState>,
    Json(r): Json<TokenReq>,
) -> Json<serde_json::Value> {
    Json(json!({ "ok": auth::logout(&s.db, &r.token).await }))
}

#[derive(serde::Deserialize)]
struct PasswordReq {
    token: String,
    new_password: String,
}

async fn auth_password(
    State(s): State<AppState>,
    Json(r): Json<PasswordReq>,
) -> Json<serde_json::Value> {
    match auth::change_password(&s.db, &r.token, &r.new_password).await {
        Ok(()) => Json(json!({ "ok": true, "message": "Password imebadilishwa" })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

// ---------- Admin tasks (admin mkuu → wasaidizi) ----------


// ADMIN: List all users (admin_mkuu only)
async fn admin_users_list(
    State(s): State<AppState>,
    axum::extract::Extension(token): axum::extract::Extension<String>,
) -> Json<serde_json::Value> {
    let sess = match auth::verify_token(&s.db, &token).await {
        Some(s) if s.role == "admin_mkuu" => s,
        _ => return Json(json!({ "ok": false, "error": "Admin Mkuu pekee" })),
    };
    let rows = sqlx::query("SELECT username, role, created_at FROM users ORDER BY created_at DESC")
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    let users: Vec<serde_json::Value> = rows.iter().map(|r| {
        json!({ "username": r.try_get::<String,_>("username").unwrap_or_default(), "role": r.try_get::<String,_>("role").unwrap_or_default(), "created_at": r.try_get::<String,_>("created_at").unwrap_or_default() })
    }).collect();
    Json(json!({ "ok": true, "users": users, "count": users.len() }))
}

// ADMIN: Change any user's password (admin_mkuu only)
async fn admin_user_password(
    State(s): State<AppState>,
    axum::extract::Extension(token): axum::extract::Extension<String>,
    Json(r): Json<AdminPassReq>,
) -> Json<serde_json::Value> {
    let sess = match auth::verify_token(&s.db, &token).await {
        Some(s) if s.role == "admin_mkuu" => s,
        _ => return Json(json!({ "ok": false, "error": "Admin Mkuu pekee" })),
    };
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let salt: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let mut h = Sha256::new(); h.update(salt.as_bytes()); h.update(b":mtech:"); h.update(r.new_password.as_bytes()); let hash = format!("{:x}", h.finalize());
    match sqlx::query("UPDATE users SET password_hash=?, salt=? WHERE username=?")
        .bind(hash).bind(salt).bind(&r.username)
        .execute(&s.db).await {
        Ok(res) if res.rows_affected() > 0 => Json(json!({ "ok": true, "message": "Password imebadilishwa" })),
        Ok(_) => Json(json!({ "ok": false, "error": "User haipo" })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

// ADMIN: Delete user (admin_mkuu only) -- cannot delete self
async fn admin_user_delete(
    State(s): State<AppState>,
    axum::extract::Extension(token): axum::extract::Extension<String>,
    Json(r): Json<AdminDeleteReq>,
) -> Json<serde_json::Value> {
    let sess = match auth::verify_token(&s.db, &token).await {
        Some(s) if s.role == "admin_mkuu" => s,
        _ => return Json(json!({ "ok": false, "error": "Admin Mkuu pekee" })),
    };
    if r.username == sess.username {
        return Json(json!({ "ok": false, "error": "Haiwezi kufuta mwenyewe" }));
    }
    match sqlx::query("DELETE FROM users WHERE username=?").bind(&r.username).execute(&s.db).await {
        Ok(res) if res.rows_affected() > 0 => Json(json!({ "ok": true, "message": "User imefutwa" })),
        Ok(_) => Json(json!({ "ok": false, "error": "User haipo" })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
struct AdminPassReq {
    username: String,
    new_password: String,
}

#[derive(serde::Deserialize)]
struct AdminDeleteReq {
    username: String,
}
// ---------- Admin tasks (admin mkuu → wasaidizi) ----------

// ---------- License (MST- keys) ----------

#[derive(serde::Deserialize)]
struct LicenseIssueReq {
    tier: String,
    owner: String,
}


#[derive(serde::Deserialize)]
struct LicenseValidateReq {
    key: String,
}


// ---------- Mode (offline/online — mtu anaamua) ----------


#[derive(serde::Deserialize)]
struct ModeSetReq {
    mode: String,
}


// ---------- Chat (agent inajibu) ----------

#[derive(serde::Deserialize)]
struct ChatReq {
    question: String,
    /// urefu wa jawabu: "short" | "medium" | "long" — default medium
    #[serde(default)]
    length: String,
    /// mtumiaji (-account) — jawabu linatafsiriwa kwa lugha yake kiatomatiki (H19)
    #[serde(default)]
    username: String,
    /// optional: agent session id (kwa Fundi Server reuse)
    #[serde(default)]
    agent_session_id: Option<String>,
    /// optional: ikiwa true → lazima tumia Fundi Agent; sio fallback.
    #[serde(default)]
    prefer_agent: Option<bool>,
    /// Hali ya mazungumzo: "chat" = majibu safi (bila actions), "agent" = tenda kwenye mfumo.
    #[serde(default)]
    mode: Option<String>,
    /// MBILINYITECH IDE — konteksti ya file inayofunguliwa (path + snippets za search hits).
    /// Zinatumiwa na coding agent kupanga kazi kwenye project halisi.
    #[serde(default)]
    ide_context: Option<serde_json::Value>,
}

/// IDE AGENT ENDPOINT — coding agent halisi ya MBILINYITECH IDE.
/// Inaitwa na panel ya Agent kwenye IDE. Inapeleka ombi kwa chat::ask kwa
/// prefer_agent=true na mode="agent": fundi-server agent inatenda kwenye
/// project (read/write files, run commands, R analysis) kupitia tools zake
/// halisi (60 tools: terminal, execute_code, memory...). Matokeo yanaonekana
/// moja kwa moja kwenye panel ya Agent (muundo wa Claude Code / Cursor side-panel).
async fn ide_agent_chat(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let prompt = r["prompt"].as_str().unwrap_or("").trim().to_string();
    if prompt.is_empty() {
        return Json(json!({ "ok": false, "error": "prompt ni lazima" }));
    }
    // Konteksti ya IDE (file + uchunguzi wa code) inakuwa sehemu ya ombi la agent.
    let ctx = r["ide_context"].as_object();
    // NB: `Map` indexer ya serde_json hupanic kama key haipo — tumia `get`.
    let file = ctx
        .and_then(|c| c.get("file"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let search_hits = ctx
        .and_then(|c| c.get("search_hits"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut full_prompt = prompt;
    if !file.is_empty() {
        full_prompt = format!("{} (file: {})", full_prompt, file);
    }
    if !search_hits.is_empty() {
        full_prompt = format!("{}\nKonteksti: {}", full_prompt, search_hits);
    }
    /* Lugha husika ya mtumiaji (ide_context.language): SW ni default ya IDE;
       EN ni kwa wachaguaji — agent anajibu kwa lugha hiyo (directive tu
       kwenye ombi; system prompt haibadiliki). */
    let lang = ctx
        .and_then(|c| c.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if lang == "en" {
        full_prompt = format!(
            "[Language preference: Reply to the user in ENGLISH.\n] {}",
            full_prompt
        );
    }
    let reply = chat::ask(
        &s.brain,
        &full_prompt,
        true, // online — coding agent inatumia LLM halisi
        chat::Length::Medium,
        s.agent.as_deref(),
        r["session_id"].as_str(),
    )
    .await;
    Json(json!({ "ok": true, "reply": reply }))
}

/// CHAT YA IDE KWA MTIRIRIKO (SSE): ni njia ile ile ya `ide_agent_chat`, lakini
/// jibu linatiririka (delta kwa delta) ili UI ionyeshe maandishi mara moja —
/// hakuna kusubiri jibu zima. Hutumia `chat::ask_stream` (haitegemei tool loop,
/// hivyo ni ya haraka kwa chat safi).
async fn ide_agent_chat_stream(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use axum::response::IntoResponse;
    use tokio_stream::StreamExt;

    let prompt = r["prompt"].as_str().unwrap_or("").trim().to_string();
    let ctx = r["ide_context"].as_object();
    let file = ctx
        .and_then(|c| c.get("file"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let search_hits = ctx
        .and_then(|c| c.get("search_hits"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let lang = ctx
        .and_then(|c| c.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let (tx, rx) = tokio::sync::mpsc::channel::<chat::ChatEvent>(256);
    if prompt.is_empty() {
        let _ = tx
            .send(chat::ChatEvent::done("prompt ni lazima"))
            .await;
    } else {
        let brain = s.brain.clone();
        let agent = s.agent.clone();
        let session_id = r["session_id"].as_str().map(String::from);
        tokio::spawn(async move {
            let mut full_prompt = prompt;
            if !file.is_empty() {
                full_prompt = format!("{} (file: {})", full_prompt, file);
            }
            if !search_hits.is_empty() {
                full_prompt = format!("{}\nKonteksti: {}", full_prompt, search_hits);
            }
            if lang == "en" {
                full_prompt = format!(
                    "[Language preference: Reply to the user in ENGLISH.\n] {}",
                    full_prompt
                );
            }
            let reply = chat::ask_stream(
                brain.as_ref(),
                &full_prompt,
                true,
                chat::Length::Medium,
                agent.as_deref(),
                session_id.as_deref(),
                tx.clone(),
            )
            .await;
            let _ = tx.send(chat::ChatEvent::done(&reply.answer_sw)).await;
        });
    }
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|ev| {
        Ok::<_, std::convert::Infallible>(
            Event::default().data(serde_json::to_string(&ev).unwrap_or_default()),
        )
    });
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// PIGA PICHA HALISI: tumia fundi-server tool "image_gen" (fal.ai / gpt-image).
/// Hii inavuta kwa /v1/tools/invoke kwenye loopback 8787 — bila kufunga kwa
/// RBAC (ni service ya ndani).
async fn agent_image_gen(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let prompt = r["prompt"].as_str().unwrap_or("").trim().to_string();
    if prompt.is_empty() {
        return Json(json!({ "ok": false, "error": "prompt ni lazima" }));
    }
    let Some(agent) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "agent disabled" }));
    };
    let body = serde_json::json!({ "tool": "image_generate", "params": { "prompt": prompt } });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// ENDESHA CODE HALISI kwa ndani (sandbox ya muda, interpreter halisi).
///
/// Awali ilivuta `fundi-server :8787` ambao haukimbii kwenye dev → zana
/// `code_execute` ilishindwa kila mara. Sasa hutekeleza kwa ndani; kama
/// `fundi-server` ipo (uzalishaji), inatumika kwanza kwa uwezo wake kamili.
async fn agent_code_run(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let code = r["code"].as_str().unwrap_or("").trim().to_string();
    let language = r["language"].as_str().unwrap_or("rust").trim().to_string();
    // `cwd`: endesha ndani ya workspace ili faili za mradi zipatikane.
    let cwd = r["cwd"].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if code.is_empty() {
        return Json(json!({ "ok": false, "error": "code ni lazima" }));
    }
    // fundi-server ikiwa hai, itumie kwanza (sandbox kamili ya uzalishaji).
    if let Some(agent) = s.agent.as_ref() {
        if agent.health().await.is_ok() {
            let body = serde_json::json!({
                "tool": "execute_code",
                "params": { "code": code, "language": language },
            });
            if let Ok(v) = agent.invoke_tool_raw(body).await {
                if v["ok"].as_bool().unwrap_or(false) {
                    return Json(v);
                }
            }
        }
    }
    // Fallback: utekelezaji halisi wa ndani.
    Json(crate::ide_code_run::run_code(&code, &language, cwd.as_deref()).await)
}

/// ANAONA PICHA HALISI: fundi-server tool "vision" (BrowserVision / moondream).
async fn agent_vision(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let url = r["image_url"].as_str().unwrap_or("").trim().to_string();
    let question = r["question"].as_str().unwrap_or("Eleza picha hii kwa kifupi.").trim().to_string();
    if url.is_empty() {
        return Json(json!({ "ok": false, "error": "image_url ni lazima" }));
    }
    let Some(agent) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "agent disabled" }));
    };
    let body = serde_json::json!({
        "tool": "vision_analyze",
        "params": { "image_url": url, "question": question },
    });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// AUTOPILOT (Devin/Trae-style): PLAN → SCAFFOLD → TEST → IMAGE → REPORT halisi.
/// UI: ipe "idea" na "kind" (website/mobile/iot/gaming/3d/image…), agent
/// inajenga project kamilifu kwenye workspace ya mtumiaji (/data/ide/<project>).
async fn ide_agent_run(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let idea = r["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = r["kind"].as_str().unwrap_or("website").trim().to_string();
    if idea.is_empty() {
        return Json(json!({ "ok": false, "error": "idea ni lazima" }));
    }
    if crate::agent_engine::is_running() {
        return Json(json!({ "ok": false, "error": "autopilot inaendesha tayari" }));
    }
    // DEFAULT = AGENTIC (ReAct): LLM inaandika faili moja kwa wakati, inaendesha,
    // inajirekebisha — mradi KAMILI. `mode:"classic"` hurudia one-shot ya zamani.
    let mode = r["mode"].as_str().unwrap_or("agentic");
    if mode != "classic" {
        let max_steps = r["maxSteps"].as_u64().unwrap_or(14).clamp(1, 40) as usize;
        let cfg = crate::ai_config::load_with_db(&s.db).await;
        return Json(crate::agent_engine::run_agentic(&idea, &kind, max_steps, &cfg).await);
    }
    let agent = s.agent.as_deref();
    let out = crate::agent_engine::run_autopilot(&idea, &kind, agent).await;
    Json(out)
}

async fn ide_agent_status() -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "running": crate::agent_engine::is_running() }))
}

/// Orodha ya lugha + ramani ya extension → lugha (kwa Monaco "detect automatically").
async fn ide_languages() -> Json<serde_json::Value> {
    Json(crate::ide_api::languages_catalog())
}

/// Orodha ya senior engineering skills zinazopachikwa kwenye agentic prompt.
async fn ide_skills_list() -> Json<serde_json::Value> {
    Json(crate::code_skills::list())
}

/// 2NYI — persona ya uhandisi wa kiwango cha juu (super senior). Hurudisha
/// skills, uwezo (OS/IoT/mobile/desktop/...), na block halisi ya prompt.
async fn nyi_info() -> Json<serde_json::Value> {
    let mut v = crate::nyi::list();
    if let Some(o) = v.as_object_mut() {
        o.insert("block".into(), serde_json::json!(crate::nyi::block()));
    }
    Json(v)
}

/// RADI — edition ya kasi ya umeme (lightning-fast). Hurudisha skills, uwezo,
/// na block halisi ya prompt.
async fn radi_info() -> Json<serde_json::Value> {
    let mut v = crate::radi::list();
    if let Some(o) = v.as_object_mut() {
        o.insert("block".into(), serde_json::json!(crate::radi::block()));
    }
    Json(v)
}

/// 2NYI MEDIA — hali ya zana (chart/picha/video) kwa UI.
async fn nyi_media_status() -> Json<serde_json::Value> {
    Json(crate::nyi_media::status())
}

/// 2NYI CHART — SVG halisi kutoka data. Body: { kind, title, data }.
async fn nyi_chart(Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let kind = r["kind"].as_str().unwrap_or("line");
    let title = r["title"].as_str().unwrap_or("Chati");
    let data = r.get("data").cloned().unwrap_or(serde_json::json!([]));
    Json(crate::nyi_media::chart(kind, title, &data))
}

/// 2NYI VIDEO — metadata ya faili la video (ffprobe). Body: { path }.
async fn nyi_video_probe(Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let path = r["path"].as_str().unwrap_or("");
    Json(crate::nyi_media::probe_video(path))
}

/// 2NYI VIDEO — tengeneza mp4/gif kutoka fremu (ffmpeg). Body: { frames:[...], fps?, gif? }.
async fn nyi_video_make(Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let frames: Vec<String> = r["frames"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let fps = r["fps"].as_u64().unwrap_or(8).clamp(1, 60) as u32;
    let gif = r["gif"].as_bool().unwrap_or(false);
    let res = if gif {
        crate::nyi_media::gif_from_frames(&frames, fps)
    } else {
        crate::nyi_media::video_from_frames(&frames, fps)
    };
    match res {
        Ok(p) => Json(serde_json::json!({
            "ok": true, "path": p.display().to_string(), "frames": frames.len(), "fps": fps,
        })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// 2NYI FINE-TUNE — maelekezo + hali (GPU inahitajika; sandbox haina).
async fn nyi_ft_info() -> Json<serde_json::Value> {
    Json(crate::nyi_ft::instructions())
}

/// 2NYI FINE-TUNE — jenga dataset ya ChatML kutoka uhalisia.
async fn nyi_ft_build() -> Json<serde_json::Value> {
    match crate::nyi_ft::build_dataset() {
        Ok(v) => Json(v),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// 2NYI FINE-TUNE — andika script ya LoRA (Unsloth).
async fn nyi_ft_script() -> Json<serde_json::Value> {
    match crate::nyi_ft::write_lora_script() {
        Ok(p) => Json(serde_json::json!({
            "ok": true, "path": p.display().to_string(),
            "note": "Script tayari. Endesha kwenye GPU: bash <path>"
        })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// AGENTIC (ReAct) — LLM inaandika faili moja kwa wakati, inaendesha, inajirekebisha.
/// Body: { idea, kind, maxSteps? }.
async fn ide_agent_agentic(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let idea = r["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = r["kind"].as_str().unwrap_or("website").trim().to_string();
    if idea.is_empty() {
        return Json(json!({ "ok": false, "error": "idea ni lazima" }));
    }
    if crate::agent_engine::is_running() {
        return Json(json!({ "ok": false, "error": "agent inaendesha tayari" }));
    }
    let max_steps = r["maxSteps"].as_u64().unwrap_or(12).clamp(1, 40) as usize;
    let cfg = crate::ai_config::load_with_db(&s.db).await;
    let out = crate::agent_engine::run_agentic(&idea, &kind, max_steps, &cfg).await;
    Json(out)
}

/// AGENTIC STREAM (SSE) — kila hatua (write/run/verify/fix/done) hutumwa live
/// kwa UI, kisha matokeo ya mwisho (`type:"result"`). Hii ni njia ya "kuona
/// akili ikifanya kazi" — sio kusubiri mradi umalizike.
async fn ide_agent_agentic_stream(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use axum::response::IntoResponse;
    use tokio_stream::StreamExt;

    let idea = r["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = r["kind"].as_str().unwrap_or("auto").trim().to_string();
    let max_steps = r["maxSteps"].as_u64().unwrap_or(16).clamp(1, 40) as usize;

    let (tx, rx) = tokio::sync::mpsc::channel::<Value>(256);
    if idea.is_empty() {
        let _ = tx.send(json!({"type":"error","error":"idea ni lazima"})).await;
    } else {
        let db = s.db.clone();
        tokio::spawn(async move {
            let cfg = crate::ai_config::load_with_db(&db).await;
            let sink: crate::agent_engine::EventSink = Some(std::sync::Arc::new(move |ev: Value| {
                // try_send: kama mteja ameondoka, usizuie loop.
                let _ = tx.try_send(ev);
            }));
            let _ = crate::agent_engine::run_agentic_stream(&idea, &kind, max_steps, &cfg, sink).await;
        });
    }

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|ev| {
        Ok::<_, std::convert::Infallible>(
            Event::default().data(serde_json::to_string(&ev).unwrap_or_default()),
        )
    });
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// Matokeo ya mwisho ya agentic run (kwa UI kupoll; proxy inaweza kuvunja SSE).
async fn ide_agent_agentic_result() -> Json<serde_json::Value> {
    match crate::agent_engine::last_result() {
        Some(v) => Json(v),
        None => Json(json!({"ok": false, "pending": true, "running": crate::agent_engine::is_running()})),
    }
}

/// RUN — endesha mradi uliojengwa (build/run/test) na kurudisha matokeo halisi.
/// Body: { project?, kind?, command? }. Bila `command`, huchagua amri salama ya
/// kukimbia kutoka lugha iliyotambuliwa (`lang_verify::run_for`).
async fn ide_agent_build_run(
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let workspace = crate::ide_api::ide_workspace_root();
    let project = r["project"].as_str().unwrap_or("").trim().to_string();
    let dir = if project.is_empty() {
        workspace.clone()
    } else {
        // Zuia traversal.
        if project.contains("..") || project.starts_with('/') {
            return Json(json!({"ok": false, "error": "project si sahihi"}));
        }
        workspace.join(&project)
    };
    if !dir.exists() {
        return Json(json!({"ok": false, "error": format!("mradi haupo: {}", dir.display())}));
    }
    let command = match r["command"].as_str() {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => crate::lang_verify::run_for(&dir),
    };
    let (allowed, reason) = crate::unified_engines::governance_check(&command);
    if !allowed {
        return Json(json!({"ok": false, "error": reason, "command": command}));
    }
    let dir2 = dir.clone();
    let cmd2 = command.clone();
    let started = std::time::Instant::now();
    let (out, ok) = tokio::task::spawn_blocking(move || {
        crate::agent_engine::run_shell_public(&dir2, &cmd2, 300)
    })
    .await
    .unwrap_or_else(|e| (format!("join error: {e}"), false));
    Json(json!({
        "ok": ok,
        "project": project,
        "workspace": dir.display().to_string(),
        "command": command,
        "elapsed_ms": started.elapsed().as_millis() as u64,
        "output": out
    }))
}

// ---------- Chat STREAM (agent inaonyesha hatua zake moja kwa moja) ----------


// ---------- Fundi Agent Bridge (kupitia server/agent/crates/fundi-server :8787) ----------

#[derive(serde::Deserialize, Default)]
struct AgentCreateSessionReq {
    #[serde(default)]
    title: String,
}

#[derive(serde::Deserialize, Default)]
struct AgentSendMessageReq {
    text: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    personality: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct AgentChatReq {
    text: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    personality: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
}

async fn agent_health(State(s): State<AppState>) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(
            json!({ "ok": false, "enabled": false, "error": "Fundi Agent bridge disabled" }),
        );
    };
    match a.health().await {
        Ok(v) => Json(json!({ "ok": true, "enabled": true, "server": v })),
        Err(e) => Json(json!({ "ok": false, "enabled": true, "error": e.to_string() })),
    }
}

async fn agent_config(State(s): State<AppState>) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "enabled": false }));
    };
    let c = a.config();
    Json(json!({
        "ok": true,
        "enabled": c.enabled,
        "base_url": c.base_url,
        "timeout_secs": c.timeout_secs,
        "has_token": c.token.is_some(),
    }))
}

async fn agent_list_sessions(State(s): State<AppState>) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "sessions": [], "total": 0 }));
    };
    match a.list_sessions().await {
        Ok(v) => Json(json!({ "ok": true, "total": v.total, "sessions": v.sessions })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn agent_create_session(
    State(s): State<AppState>,
    Json(r): Json<AgentCreateSessionReq>,
) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "Fundi Agent bridge disabled" }));
    };
    match a.create_session(r.title.as_str()).await {
        Ok(v) => Json(json!({ "ok": true, "session": v })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn agent_list_messages(
    State(s): State<AppState>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "messages": [] }));
    };
    match a.list_messages(&session_id).await {
        Ok(v) => Json(json!({ "ok": true, "session_id": v.session_id, "messages": v.messages })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn agent_send_message(
    State(s): State<AppState>,
    Path(session_id): Path<String>,
    Json(r): Json<AgentSendMessageReq>,
) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "Fundi Agent bridge disabled" }));
    };
    let opts = fundi_agent_client::SendMessageOptions {
        model: r.model.as_deref(),
        provider: r.provider.as_deref(),
        personality: r.personality.as_deref(),
        user_id: r.user_id.as_deref(),
        request_id: r.request_id.as_deref(),
    };
    match a.send_message(&session_id, &r.text, opts).await {
        Ok(v) => Json(json!({
            "ok": true,
            "session_id": v.session_id,
            "reply": v.reply,
            "message_count": v.message_count,
            "trace_id": v.trace_id,
        })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn agent_chat(
    State(s): State<AppState>,
    Json(r): Json<AgentChatReq>,
) -> Json<serde_json::Value> {
    let Some(a) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "Fundi Agent bridge disabled" }));
    };
    let opts = fundi_agent_client::SendMessageOptions {
        model: r.model.as_deref(),
        provider: r.provider.as_deref(),
        personality: r.personality.as_deref(),
        user_id: r.user_id.as_deref(),
        request_id: r.request_id.as_deref(),
    };
    match a
        .chat(r.session_id.as_deref(), &r.title, &r.text, opts)
        .await
    {
        Ok(v) => Json(json!({
            "ok": true,
            "session_id": v.session_id,
            "reply": v.reply,
            "message_count": v.message_count,
            "trace_id": v.trace_id,
        })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

// ---------- Auto-daily (scan → ripoti → ruhusa → solve) ----------

async fn daily_report(State(s): State<AppState>) -> Json<serde_json::Value> {
    let findings = daily::today_report(&s.db).await;
    let awaiting = findings.iter().filter(|f| f.severity != "info").count();
    Json(json!({
        "ok": true,
        "date": chrono::Local::now().format("%Y-%m-%d").to_string(),
        "count": findings.len(),
        "findings": findings,
        "summary_sw": format!("Scan ya leo: matatizo {awaiting} — admin anaruhusu, agents zinaanza kutatua kwa wakati mmoja."),
    }))
}

async fn daily_permission(State(s): State<AppState>) -> Json<serde_json::Value> {
    let n = daily::grant_permission(&s.db).await;
    Json(json!({
        "ok": true,
        "granted": n,
        "message_sw": if n > 0 { format!("Ruhusa imepewa — matatizo {n} yameanza kutatuliwa kwa wakati mmoja.") } else { "Hakuna mapya yanayosubiri ruhusa.".into() },
    }))
}

// ---------- Updates (notification ya version mpya) ----------


// ---------- Language (chagua wakati wa kusajili + AI translate) ----------

#[derive(serde::Deserialize)]
struct LangSetReq {
    username: String,
    language: String,
}

async fn lang_set(State(s): State<AppState>, Json(r): Json<LangSetReq>) -> Json<serde_json::Value> {
    match language::set_language(&s.db, &r.username, &r.language).await {
        Ok(()) => Json(json!({ "ok": true, "username": r.username, "language": r.language })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn lang_get(
    State(s): State<AppState>,
    Path(username): Path<String>,
) -> Json<serde_json::Value> {
    Json(
        json!({ "ok": true, "username": username, "language": language::get_language(&s.db, &username).await }),
    )
}

#[derive(serde::Deserialize)]
struct TranslateReq {
    username: String,
    text: String,
}

async fn translate_post(
    State(s): State<AppState>,
    Json(r): Json<TranslateReq>,
) -> Json<serde_json::Value> {
    let (translated, lang) = language::auto_translate(&s.db, &r.username, &r.text).await;
    Json(json!({ "ok": true, "language": lang, "translated": translated }))
}

// ---------- WireGuard VPN (P4) ----------

async fn vpn_init(State(s): State<AppState>) -> Json<serde_json::Value> {
    match vpn::write_server_conf(&s.db).await {
        Ok(path) => Json(json!({
            "ok": true,
            "conf": path.to_string_lossy(),
            "endpoint_hint": "Weka FUNDI_WG_ENDPOINT (public IP au domain ya server) kwenye docker-compose ili client confs ziwe na Endpoint halisi.",
            "up": "POST /api/vpn/up (inahitaji NET_ADMIN + /dev/net/tun + wireguard-tools kwenye container)"
        })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn vpn_status(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(vpn::status(&s.db).await)
}

async fn vpn_peers(State(s): State<AppState>) -> Json<serde_json::Value> {
    let peers = vpn::list_peers(&s.db).await;
    Json(
        json!({ "count": peers.len(), "peers": peers.iter().map(|p| json!({
        "name": p.name, "ip": p.ip, "created_at": p.created_at, "enabled": p.enabled,
    })).collect::<Vec<_>>() }),
    )
}

#[derive(serde::Deserialize)]
struct VpnPeerAdd {
    name: String,
    #[serde(default)]
    note: Option<String>,
}

async fn vpn_peer_add(
    State(s): State<AppState>,
    Json(r): Json<VpnPeerAdd>,
) -> Json<serde_json::Value> {
    match vpn::add_peer(&s.db, &r.name, r.note.as_deref().unwrap_or("")).await {
        Ok(p) => Json(json!({
            "ok": true,
            "peer": { "name": p.name, "ip": p.ip },
            "conf_url": format!("/api/vpn/peers/{}/conf", p.name),
            "next": "Pakua conf hii kwenye computer ya mbali → wg-quick up <faili> — kisha kazi zote za mbali zinaenda kupitia VPN"
        })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn vpn_peer_conf(
    State(s): State<AppState>,
    Path(name): Path<String>,
) -> impl axum::response::IntoResponse {
    let peers = vpn::list_peers(&s.db).await;
    let Some(peer) = peers.iter().find(|p| p.name == name) else {
        return (
            [
                (axum::http::header::CONTENT_TYPE, "text/plain".to_string()),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    "inline".to_string(),
                ),
            ],
            format!("Peer '{name}' haipo"),
        );
    };
    let cfg = vpn::Config::load();
    let (_, server_pub) = match vpn::server_keys() {
        Ok(k) => k,
        Err(e) => {
            return (
                [
                    (axum::http::header::CONTENT_TYPE, "text/plain".to_string()),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "inline".to_string(),
                    ),
                ],
                format!("Server keys: {e}"),
            )
        }
    };
    let conf = vpn::client_conf(&cfg, peer, &server_pub);
    (
        [
            (axum::http::header::CONTENT_TYPE, "text/plain".to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"wg-{name}.conf\""),
            ),
        ],
        conf,
    )
}

async fn vpn_peer_delete(
    State(s): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let ok = vpn::remove_peer(&s.db, &name).await;
    if ok {
        let _ = vpn::write_server_conf(&s.db).await; // regen conf bila peer
    }
    Json(json!({ "ok": ok, "name": name }))
}

async fn vpn_up(State(s): State<AppState>) -> Json<serde_json::Value> {
    let _ = vpn::write_server_conf(&s.db).await; // conf ya sasa (peers zote) kabla ya up
    match vpn::up(None).await {
        Ok(msg) => Json(json!({ "ok": true, "message": msg })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn vpn_down(State(_s): State<AppState>) -> Json<serde_json::Value> {
    match vpn::down().await {
        Ok(msg) => Json(json!({ "ok": true, "message": msg })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn vpn_scan(State(s): State<AppState>) -> Json<serde_json::Value> {
    let cfg = vpn::Config::load();
    let active = tokio::process::Command::new("wg")
        .args(["show", &cfg.interface])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    let hosts = vpn::scan_vpn_subnet(&cfg, 400).await;
    let rows: Vec<serde_json::Value> = hosts
        .iter()
        .map(|(ip, ports)| {
            let names: Vec<String> = ports
                .iter()
                .map(|p| match p {
                    22 => "SSH".to_string(),
                    445 => "SMB".to_string(),
                    135 => "RPC".to_string(),
                    3389 => "RDP".to_string(),
                    other => format!("port-{other}"),
                })
                .collect();
            json!({ "ip": ip, "services": names })
        })
        .collect();
    Json(json!({
        "subnet": cfg.subnet,
        "vpn_active": active,
        "hosts_alive": rows.len(),
        "hosts": rows,
        "note_sw": if active { "Scan hii ilifanyika JUU YA TUNNEL ya WireGuard (wg0) — hosts ni za site ya mbali." } else { "Tunnel haipo — hosts hizi ni za LAN/ndani tu. Washa VPN: POST /api/vpn/up" }
    }))
}

async fn vpn_server_conf(State(s): State<AppState>) -> impl axum::response::IntoResponse {
    let peers = vpn::list_peers(&s.db).await;
    let cfg = vpn::Config::load();
    match vpn::server_keys() {
        Ok((priv_key, _)) => {
            let conf = vpn::server_conf(&cfg, &priv_key, &peers);
            (
                [
                    (axum::http::header::CONTENT_TYPE, "text/plain"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"wg0.conf\"",
                    ),
                ],
                conf,
            )
        }
        Err(e) => (
            [
                (axum::http::header::CONTENT_TYPE, "text/plain"),
                (axum::http::header::CONTENT_DISPOSITION, "inline"),
            ],
            format!("Server keys: {e}"),
        ),
    }
}

// ---------- multicast ----------

#[derive(serde::Deserialize)]
struct McAckReq {
    mac: String,
    sha256: String,
    #[serde(default)]
    received_bytes: u64,
    #[serde(default = "default_true")]
    ok: bool,
}

fn default_true() -> bool {
    true
}

async fn mc_ack(State(s): State<AppState>, Json(a): Json<McAckReq>) -> Json<serde_json::Value> {
    s.acks.record(multicast::McAck {
        mac: a.mac,
        sha256: a.sha256,
        received_bytes: a.received_bytes,
        ok: a.ok,
    });
    Json(json!({ "ok": true, "total_acks": s.acks.count() }))
}

async fn mc_status(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "group": multicast::MC_GROUP,
        "port": multicast::MC_PORT,
        "acks": s.acks.count(),
        "verified_ok": s.acks.ok_count()
    }))
}

// ---------- agents ----------

async fn list_agents(State(s): State<AppState>) -> Json<serde_json::Value> {
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT id, name, role, status FROM agents ORDER BY id")
            .fetch_all(&s.db)
            .await
            .unwrap_or_default();
    let agents: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, name, role, status)| json!({ "id": id, "name": name, "role": role, "status": status }))
        .collect();
    Json(json!({ "agents": agents, "count": agents.len() }))
}

// ---------- supervisor ----------

async fn supervisor_summary(State(s): State<AppState>) -> Json<serde_json::Value> {
    let jobs = s.jobs.read().await;
    Json(json!({
        "role": "msimamizi",
        "awaiting_your_approval": jobs.iter().filter(|j| j.status == "awaiting_approval").count(),
        "running": jobs.iter().filter(|j| j.status == "running").count(),
        "done": jobs.iter().filter(|j| j.status == "done").count(),
        "failed": jobs.iter().filter(|j| j.status == "failed").count(),
        "total": jobs.len(),
    }))
}

// ---------- PFSENSE (H5b): firewall ya mtandao mmoja — API halisi ----------

#[derive(serde::Deserialize)]
struct PfRuleReq {
    /// mf. {"action":"pass","interface":"lan","protocol":"tcp","source":"lan","destination":"any","destination_port":"80"}
    #[serde(flatten)]
    rule: serde_json::Value,
}

async fn pfsense_rules(State(s): State<AppState>) -> Json<serde_json::Value> {
    if !s.pfsense.is_configured() {
        return Json(
            json!({ "ok": false, "error": "pfSense API haijawekwa: PFSENSE_API_URL + PFSENSE_API_KEY (env)", "configured": false }),
        );
    }
    match s.pfsense.list_rules().await {
        Ok(v) => Json(json!({ "ok": true, "configured": true, "data": v })),
        Err(e) => Json(json!({ "ok": false, "configured": true, "error": e })),
    }
}

async fn pfsense_rule_add(
    State(s): State<AppState>,
    Json(r): Json<PfRuleReq>,
) -> Json<serde_json::Value> {
    if !s.pfsense.is_configured() {
        return Json(
            json!({ "ok": false, "error": "pfSense API haijawekwa: PFSENSE_API_URL + PFSENSE_API_KEY (env)", "configured": false }),
        );
    }
    match s.pfsense.add_rule(r.rule).await {
        Ok(v) => Json(json!({ "ok": true, "configured": true, "data": v })),
        Err(e) => Json(json!({ "ok": false, "configured": true, "error": e })),
    }
}

async fn pfsense_aliases(State(s): State<AppState>) -> Json<serde_json::Value> {
    if !s.pfsense.is_configured() {
        return Json(
            json!({ "ok": false, "error": "pfSense API haijawekwa: PFSENSE_API_URL + PFSENSE_API_KEY (env)", "configured": false }),
        );
    }
    match s.pfsense.list_aliases().await {
        Ok(v) => Json(json!({ "ok": true, "configured": true, "data": v })),
        Err(e) => Json(json!({ "ok": false, "configured": true, "error": e })),
    }
}

async fn pfsense_alias_add(
    State(s): State<AppState>,
    Json(r): Json<PfRuleReq>,
) -> Json<serde_json::Value> {
    if !s.pfsense.is_configured() {
        return Json(
            json!({ "ok": false, "error": "pfSense API haijawekwa: PFSENSE_API_URL + PFSENSE_API_KEY (env)", "configured": false }),
        );
    }
    match s.pfsense.add_alias(r.rule).await {
        Ok(v) => Json(json!({ "ok": true, "configured": true, "data": v })),
        Err(e) => Json(json!({ "ok": false, "configured": true, "error": e })),
    }
}

#[derive(serde::Deserialize)]
struct PfServiceReq {
    name: String,
}

async fn pfsense_service_restart(
    State(s): State<AppState>,
    Json(r): Json<PfServiceReq>,
) -> Json<serde_json::Value> {
    if !s.pfsense.is_configured() {
        return Json(
            json!({ "ok": false, "error": "pfSense API haijawekwa: PFSENSE_API_URL + PFSENSE_API_KEY (env)", "configured": false }),
        );
    }
    match s.pfsense.restart_service(&r.name).await {
        Ok(v) => Json(json!({ "ok": true, "configured": true, "service": r.name, "data": v })),
        Err(e) => Json(json!({ "ok": false, "configured": true, "service": r.name, "error": e })),
    }
}

async fn pfsense_status(State(s): State<AppState>) -> Json<serde_json::Value> {
    if !s.pfsense.is_configured() {
        return Json(
            json!({ "ok": false, "configured": false, "error": "pfSense API haijawekwa: PFSENSE_API_URL + PFSENSE_API_KEY (env)" }),
        );
    }
    match s.pfsense.system_status().await {
        Ok(v) => Json(json!({ "ok": true, "configured": true, "data": v })),
        Err(e) => Json(json!({ "ok": false, "configured": true, "error": e })),
    }
}
// ---------- NET MGMT ----------

// ---------- CUSTOM MODEL / API (AI ya mteja yenyewe) ----------

async fn ai_custom_get(State(s): State<AppState>) -> Json<serde_json::Value> {
    let cfg = ai_config::load_with_db(&s.db).await;
    Json(json!({
        "ok": true,
        "url": cfg.url,
        "model": cfg.model,
        "offline_capable": cfg.offline_capable,
        "note_sw": "Weka endpoint ya model yako (Ollama / llama.cpp / vLLM / OpenAI-compatible) — inatumika mara moja."
    }))
}

async fn ai_custom_set(
    State(s): State<AppState>,
    Json(r): Json<ai_config::CustomAi>,
) -> Json<serde_json::Value> {
    match ai_config::set_custom(&s.db, &r).await {
        Ok(()) => Json(
            json!({ "ok": true, "url": r.url.trim().trim_end_matches('/'), "model": r.model.trim() }),
        ),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ai_custom_clear(State(s): State<AppState>) -> Json<serde_json::Value> {
    match ai_config::clear_custom(&s.db).await {
        Ok(()) => Json(json!({ "ok": true, "note_sw": "Rejea AI ya ndani ya mfumo." })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// OLLAMA CLOUD: hifadhi API key (faili + env, SIO DB).
async fn ai_key_set(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let res = ai_config::save_key(r["key"].as_str().unwrap_or(""));
    if res["ok"] != true {
        return Json(res);
    }
    // AUTOMATIKI: weka tu key - mfumo hujiseti kutoka data/ai/models.json (default_chat + URL)
    let cat = data_io::recommended();
    let base = cat["ollama_cloud_url"]
        .as_str()
        .unwrap_or("https://ollama.com")
        .to_string();
    let prefer = cat["default_chat"].as_str().unwrap_or("").to_string();
    let models = ai_config::cloud_models().await;
    if models["ok"] == true {
        let list: Vec<String> = models["models"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        // chagua default_chat ya models.json kama ipo, vinginevyo modeli ya kwanza
        let chosen = list
            .iter()
            .find(|m| m.as_str() == prefer)
            .cloned()
            .or_else(|| list.first().cloned());
        if let Some(m) = chosen {
            match ai_config::set_custom(
                &s.db,
                &ai_config::CustomAi {
                    url: base.clone(),
                    model: m.clone(),
                },
            )
            .await
            {
                Ok(()) => {
                    let test = ai_config::test_current(&s.db).await;
                    return Json(
                        json!({ "ok": true, "auto": { "configured": true, "url": base, "model": m, "default_from": cat["source"], "models_count": models["count"], "test": test } }),
                    );
                }
                Err(e) => {
                    return Json(json!({ "ok": true, "auto": { "configured": false, "error": e } }))
                }
            }
        }
    }
    Json(json!({ "ok": true, "auto": { "configured": false, "error": models["error"] } }))
}

/// OLLAMA CLOUD: model ZOTE zinazopatikana kwa key hii (free tier ikiwa ni pamoja).
async fn ai_cloud_models() -> Json<serde_json::Value> {
    Json(ai_config::cloud_models().await)
}

/// MAJARIBIO HALISI: swali dogo kwenye LLM iliyosetwa → reply + muda (ms).
async fn ai_test(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(ai_config::test_current(&s.db).await)
}
// ---------- LLAMA.CPP + LOCAL MODELS (automatiki: pakua -> washa -> configure) ----------

async fn llm_status(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(llm_setup::status_json(&s.db).await)
}

async fn llm_models() -> Json<serde_json::Value> {
    // UI ya ndani ya MTECHOS inasoma `models` (array ya ModelDef). Canvas
    // (agent-server protocol) inasoma `models` (array ya strings za `ollama/<id>`),
    // hivyo tunaunganisha zote mbili kwenye jibu moja.
    let mut v = llm_setup::models_json();
    let ids: Vec<serde_json::Value> = llm_setup::models_list()
        .iter()
        .filter_map(|m| m.get("id").and_then(|x| x.as_str()).map(|id| format!("ollama/{id}")))
        .map(serde_json::Value::String)
        .collect();
    if let Some(obj) = v.as_object_mut() {
        obj.insert("models".to_string(), serde_json::Value::Array(ids));
    }
    Json(v)
}

async fn llm_download(Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    match llm_setup::start_download_kind("model", r["id"].as_str().unwrap_or("")) {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn llm_download_status() -> Json<serde_json::Value> {
    Json(llm_setup::job_json())
}

async fn llm_llamacpp_setup() -> Json<serde_json::Value> {
    match llm_setup::start_download_kind("llamacpp", "llamacpp") {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn llm_start(Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    match llm_setup::start_server(r["id"].as_str().unwrap_or("")).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn llm_stop() -> Json<serde_json::Value> {
    Json(llm_setup::stop_server())
}

async fn llm_use(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    match llm_setup::use_model(&s.db, r["id"].as_str().unwrap_or("")).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn bundles_fetch(State(s): State<AppState>) -> Json<serde_json::Value> {
    match bundles::fetch_and_store(&s.db).await {
        Ok((n, src)) => Json(json!({ "ok": true, "count": n, "source": src })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}
// ---------- DATA/ + AI CATALOG: AI inapata folder lote la data + models.json ----------

async fn ai_catalog() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "catalog": data_io::load_models_json(),
        "recommended": data_io::recommended()
    }))
}

async fn data_tree() -> Json<serde_json::Value> {
    Json(data_io::tree_json())
}

async fn data_file(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let p = q.get("path").cloned().unwrap_or_default();
    match data_io::read_file(&p) {
        Ok((txt, rel)) => {
            Json(json!({ "ok": true, "path": rel, "bytes": txt.len(), "content": txt }))
        }
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

// ---------- BUNDLE REPOSITORY: DISTRIBUTE (H10) ----------

#[derive(serde::Deserialize)]
struct BundleDistributeReq {
    bundle: String,
    targets: Vec<String>,
    account: String,
    /// os ya kifaa (win11/ubuntu/kali…) — inatumika kwenye install commands
    #[serde(default = "default_os")]
    os: String,
}

fn default_os() -> String {
    "win11".into()
}

async fn bundles_distribute(
    State(s): State<AppState>,
    Json(r): Json<BundleDistributeReq>,
) -> Json<serde_json::Value> {
    if r.targets.is_empty() {
        return Json(json!({ "ok": false, "error": "chagua kompyuta angalau moja" }));
    }
    // Thibitisha bundle ipo (katalogi halisi ya Rust)
    let bundle = match bundles::find_bundle(&r.bundle) {
        Some(b) => b,
        None => {
            return Json(
                json!({ "ok": false, "error": format!("bundle '{}' haipo kwenye repository", r.bundle) }),
            )
        }
    };
    // Unda kazi za distribusheni (HITL — needs_approval) kwa kila target
    let mut jobs = s.jobs.write().await;
    let mut ids = Vec::new();
    for name in &r.targets {
        let id = Uuid::new_v4().to_string();
        jobs.push(Job {
            id: id.clone(),
            device_mac: String::new(),
            device_name: name.clone(),
            os_type: r.os.clone(),
            status: "queued".into(),
            stage: format!("bundle:{}", bundle.id),
            progress: 0,
            message: format!(
                "Distribution ya '{}' inasubiri idhini (HITL)",
                bundle.name_sw
            ),
            needs_approval: true,
            image: None,
            multicast: false,
        });
        ids.push(id);
    }
    drop(jobs);
    Json(json!({
        "ok": true,
        "jobs": ids,
        "count": ids.len(),
        "bundle": bundle.name_sw,
        "note_sw": "Kazi zimesubiri RUHUSU (HITL) kwenye tab Jobs — agent inasakinisha apps kwa wakati mmoja baada ya idhini."
    }))
}

// ---------- CYBER SECURITY & FORENSICS CENTER (H11) ----------

#[derive(serde::Deserialize)]
struct SecOpsStartReq {
    account: String,
    targets: Vec<String>,
    mode: String,
}

async fn secops_start(
    State(s): State<AppState>,
    Json(r): Json<SecOpsStartReq>,
) -> Json<serde_json::Value> {
    let targets: Vec<String> = r
        .targets
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    if targets.is_empty() {
        return Json(json!({ "ok": false, "error": "chagua kompyuta angalau moja" }));
    }
    let mode = match secops::Mode::parse(&r.mode) {
        Some(m) => m,
        None => {
            return Json(
                json!({ "ok": false, "error": "chagua hali: security | forensics | both" }),
            )
        }
    };
    // Kazi za HITL — kila target × op, zinaendeshwa kwa WAKATI MMOJA baada ya idhini
    let mut jobs = s.jobs.write().await;
    let mut ids = Vec::new();
    for name in &targets {
        for op in mode.ops() {
            let id = Uuid::new_v4().to_string();
            let label = if *op == "forensics" {
                "Uchunguzi wa kidijitali (ushahidi)"
            } else {
                "Uchunguzi wa usalama"
            };
            jobs.push(Job {
                id: id.clone(),
                device_mac: String::new(),
                device_name: name.clone(),
                os_type: String::new(),
                status: "queued".into(),
                stage: format!("secops:{}", op),
                progress: 0,
                message: format!("{} inasubiri idhini (HITL)", label),
                needs_approval: true,
                image: None,
                multicast: false,
            });
            ids.push(id);
        }
    }
    drop(jobs);
    Json(json!({
        "ok": true,
        "jobs": ids,
        "count": ids.len(),
        "mode": mode.as_str(),
        "note_sw": "Kazi zinaendeshwa kwa WAKATI MMOJA kwenye kompyuta zote baada ya RUHUSU (HITL, tab Jobs)."
    }))
}

#[derive(serde::Deserialize)]
struct SecOpsResultReq {
    account: String,
    target: String,
    mode: String,
    #[serde(default)]
    findings: Vec<secops::Finding>,
    #[serde(default)]
    sources: String,
    #[serde(default)]
    note: String,
}

async fn secops_result(
    State(s): State<AppState>,
    Json(mut r): Json<SecOpsResultReq>,
) -> Json<serde_json::Value> {
    if !tools_internal::valid_target(&r.target) {
        return Json(json!({ "ok": false, "error": "jina la kompyuta si salama" }));
    }
    r.findings = r.findings.drain(..).map(|f| f.normalized()).collect();
    let mode = match secops::Mode::parse(&r.mode) {
        Some(m) => m,
        None => {
            return Json(json!({ "ok": false, "error": "hali si sahihi: security | forensics" }))
        }
    };
    match mode {
        secops::Mode::Security => {
            match secops::save_report(&s.db, &r.account, &r.target, "security", &r.findings).await {
                Ok((id, health, sev)) => {
                    Json(json!({ "ok": true, "report_id": id, "health": health, "severity": sev }))
                }
                Err(e) => Json(json!({ "ok": false, "error": e })),
            }
        }
        secops::Mode::Forensics => {
            match secops::save_case(&s.db, &r.account, &r.target, &r.sources, &r.note).await {
                Ok((id, hash)) => Json(
                    json!({ "ok": true, "case_id": id, "evidence_hash": hash, "sealed": true }),
                ),
                Err(e) => Json(json!({ "ok": false, "error": e })),
            }
        }
        secops::Mode::Both => Json(
            json!({ "ok": false, "error": "tuma matokeo ya kila hali peke yake (security au forensics)" }),
        ),
    }
}

async fn secops_summary(
    State(s): State<AppState>,
    Path(account): Path<String>,
) -> Json<serde_json::Value> {
    Json(secops::summary(&s.db, &account).await)
}

// ---------- TOOLKIT LAYER (H12) — registry + executor + batch wakati mmoja ----------

#[derive(serde::Deserialize)]
struct ToolkitRunReq {
    account: Option<String>,
    tool: String,
    target: String,
    #[serde(default)]
    internal_agent: bool,
}

async fn toolkit_run(
    State(s): State<AppState>,
    Json(r): Json<ToolkitRunReq>,
) -> Json<serde_json::Value> {
    // AGENT INTERNAL: wito kutoka Fundi Agent (agent_tools) — linabillishwa kwenye
    // "agent_internal" (sezamali hakuna kuvunja bili za wateja halisi).
    let account = match r.account.as_deref() {
        Some(a) if !a.trim().is_empty() => a.to_string(),
        _ => "agent_internal".to_string(),
    };
    let out = toolkit::run_single(
        &s.db,
        toolkit::BatchJob {
            tool: r.tool,
            targets: vec![r.target],
            account: account.clone(),
        },
    )
    .await;
    Json(out)
}

#[derive(serde::Deserialize)]
struct ToolkitBatchReq {
    account: String,
    tool: String,
    targets: Vec<String>,
}

async fn toolkit_batch(
    State(s): State<AppState>,
    Json(r): Json<ToolkitBatchReq>,
) -> Json<serde_json::Value> {
    let targets: Vec<String> = r
        .targets
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    if targets.is_empty() {
        return Json(json!({ "ok": false, "error": "chagua kompyuta angalau moja" }));
    }
    let tool_id = r.tool.clone();
    let out = toolkit::run_batch(
        &s.db,
        toolkit::BatchJob {
            tool: r.tool,
            targets,
            account: r.account.clone(),
        },
    )
    .await;
    let okc = out
        .iter()
        .filter(|o| o["ok"] == serde_json::Value::Bool(true))
        .count();
    Json(json!({
        "ok": true, "tool": tool_id, "results": out, "count": out.len(), "success": okc,
        "note_sw": "Kazi zinaendeshwa kwa WAKATI MMOJA kwenye kompyuta zote.",
    }))
}

async fn toolkit_catalog(State(s): State<AppState>) -> Json<serde_json::Value> {
    let _ = &s.db;
    Json(toolkit::catalog_json(None))
}

async fn toolkit_runs(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(toolkit::runs_json(&s.db).await)
}

// ---------- SECURITY PACK: katalogi + kazi halisi + shield + pool ya agents ----------
// (vikundi 30 kwa majina ya debian/control, zana 564+, commands halisi ndani ya Rust)

async fn security_groups(State(s): State<AppState>) -> Json<serde_json::Value> {
    let _ = &s.db;
    Json(secpack::groups_json())
}

/// Hali HALISI ya upatikanaji wa zana (which kwenye PATH) — bila kuongea uongo.
async fn security_availability(State(_s): State<AppState>) -> Json<serde_json::Value> {
    Json(secpack::availability_json())
}

#[derive(serde::Deserialize)]
pub struct SecTaskReq {
    pub task: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub human_approved: bool,
    #[serde(default)]
    pub internal_agent: bool,
}

/// NAFASI HALISI YA DISKI (Windows CIM) — data ya kweli kwa ajili ya agent/chat.
async fn system_disk() -> Json<serde_json::Value> {
    // LINUX / container (Fly, Docker, server kuu ya LAN): `df -P -B1` inatoa
    // nafasi HALISI ya kila filesystem. Bila tawi hili, agent alikuwa hapati
    // data yoyote ya diski kwenye server ya Linux ("disks": []) — akakata tamaa.
    let df = tokio::task::spawn_blocking(|| {
        std::process::Command::new("df")
            .args(["-P", "-B1"])
            .output()
            .ok()
            .filter(|r| r.status.success())
            .map(|r| String::from_utf8_lossy(&r.stdout).to_string())
    })
    .await
    .ok()
    .flatten();
    if let Some(raw) = df {
        let mut disks = Vec::new();
        for line in raw.lines().skip(1) {
            // Muundo: /dev/sda1 10485760 5242880 5242880 50% /
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() < 6 {
                continue;
            }
            let size: f64 = p[1].parse().unwrap_or(0.0) / 1_073_741_824.0;
            let free: f64 = p[3].parse().unwrap_or(0.0) / 1_073_741_824.0;
            if size <= 0.0 {
                continue;
            }
            disks.push(json!({
                "drive": p[5],
                "device": p[0],
                "size_gb": (size * 10.0).round() / 10.0,
                "free_gb": (free * 10.0).round() / 10.0,
                "used_gb": ((size - free) * 10.0).round() / 10.0,
                "used_pct": (((size - free) / size) * 1000.0).round() / 10.0,
            }));
        }
        return Json(json!({
            "ok": !disks.is_empty(),
            "count": disks.len(),
            "disks": disks,
            "source": "halisi (Linux df)",
            "note_sw": "Nafasi halisi ya diski kutoka kifaa — si makadirio."
        }));
    }

    // WINDOWS: njia ya awali (CIM) inabaki kama nyongeza.
    let out = tokio::task::spawn_blocking(|| {
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | ForEach-Object { $_.DeviceID + '|' + [math]::Round($_.Size/1GB,1) + '|' + [math]::Round($_.FreeSpace/1GB,1) }",
            ])
            .output()
    })
    .await
    .ok()
    .and_then(|r| r.ok());
    let mut disks = Vec::new();
    if let Some(o) = out {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let p: Vec<&str> = line.split('|').collect();
            if p.len() == 3 {
                let size: f64 = p[1].trim().parse().unwrap_or(0.0);
                let free: f64 = p[2].trim().parse().unwrap_or(0.0);
                disks.push(json!({
                    "drive": p[0].trim(),
                    "size_gb": size,
                    "free_gb": free,
                    "used_gb": ((size - free) * 10.0).round() / 10.0,
                    "used_pct": if size > 0.0 { (((size - free) / size) * 1000.0).round() / 10.0 } else { 0.0 },
                }));
            }
        }
    }
    Json(json!({
        "ok": !disks.is_empty(),
        "disks": disks,
        "source": "halisi (Windows CIM)",
        "note_sw": "Nafasi halisi ya diski kutoka kifaa — si makadirio."
    }))
}

// ========== RBAC (Role-Based Access Control) ==========

/// Static assets: assets zenye hash hupata cache ya kudumu (`immutable`),
/// HTML/SPA routes hupata `no-cache`. Bila hii browser hudhani bundle ya canvas
/// (`/ide/assets/*`) haibadiliki na hubakiza toleo la zamani, au hupakua upya
/// kila load (bundle ni ~3MB).
async fn static_headers_mw(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let is_static = {
        let p = path.as_str();
        let known = p.ends_with(".js")
            || p.ends_with(".css")
            || p.ends_with(".txt")
            || p.ends_with(".html")
            || p.starts_with("/ide/")
            || p == "/"
            || p == "/ide";
        known && !p.starts_with("/api/")
    };
    let mut res = next.run(req).await;
    if is_static {
        // Assets za build zina hash kwenye jina (mfano `vendor-styling-IswJGxFM.js`)
        // hivyo zinabadilika tu jina linapobadilika — cache ya kudumu ni salama.
        // HTML/SPA routes hazina hash, hivyo `no-cache` (validator kila load).
        let file = path.rsplit('/').next().unwrap_or("");
        let hashed = path.starts_with("/ide/assets/")
            || (file.matches('-').count() >= 1
                && matches!(file.rsplit('.').next(), Some("js" | "css" | "woff" | "woff2")));
        let value = if hashed {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        res.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static(value),
        );
    }
    res
}


// ========== WOL: kuamsha PC iliyozima ==========

#[derive(serde::Deserialize)]
struct WolReq {
    mac: String,
    #[serde(default)]
    ip: Option<String>,
}


// ========== SIMU ZILIZOUNGANISHWA USB (halisi — Windows PnP) ==========

async fn mobile_usb() -> Json<serde_json::Value> {
    let out = tokio::task::spawn_blocking(|| {
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "$vids='18D1|04E8|2A70|22D9|2717|12D1|05C6|2AE0|2A45|0BB4|1004|1D6D|22B8|0FCE|19D2|1EBF|2340|1050'; Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | Where-Object { ($_.Class -eq 'WPD') -or (($_.InstanceId -like 'USB\\VID_*') -and ($_.InstanceId -match ('VID_(' + $vids + ')'))) } | ForEach-Object { $_.FriendlyName + '|' + $_.InstanceId + '|' + $_.Class + '|' + $_.Status }",
            ])
            .output()
    })
    .await
    .ok()
    .and_then(|r| r.ok());
    let mut phones = Vec::new();
    if let Some(o) = out {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let p: Vec<&str> = line.splitn(4, '|').collect();
            if p.len() != 4 {
                continue;
            }
            let name = p[0].trim().to_string();
            let iid = p[1].trim().to_string();
            let class = p[2].trim().to_string();
            let status = p[3].trim().to_string();
            if name.is_empty() || iid.is_empty() {
                continue;
            }
            let up = iid.to_uppercase();
            let adb = up.contains("PID_4EE7")
                || up.contains("PID_D00D")
                || name.to_uppercase().contains("ADB");
            let mode = if adb {
                "ADB (USB debugging — uchunguzi wa kina)"
            } else if class == "WPD" {
                "MTP (picha/muziki/faili)"
            } else {
                "USB (modem au diagnostic)"
            };
            let vid = up
                .split("VID_")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .map(|s| s.to_string())
                .unwrap_or_default();
            phones.push(json!({
                "name": name, "mode": mode, "status": status,
                "class": class, "vendor_id": format!("VID_{vid}"), "instance": iid
            }));
        }
    }
    let n = phones.len();
    Json(json!({
        "ok": true,
        "count": n,
        "phones": phones,
        "source": "halisi (Windows PnP)",
        "note_sw": if n == 0 {
            "Hakuna simu iliyounganishwa USB kwa server hii sasa. Unganisha simu kwa USB — chagua mode MTP (picha/faili) au weka USB debugging kisha jaribu tena."
        } else {
            "Simu zilizounganishwa USB — data halisi kutoka kifaa. AI inaweza kutumia modi hii (MTP/ADB) kuchunguza na kutatua matatizo ya simu."
        }
    }))
}

// ========== KUTAFUTA MTANDAONI (DuckDuckGo — hakuna API key) ==========

fn urlenc(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/' => out.push(c),
            ' ' => out.push('+'),
            _ => {
                let mut b = [0u8; 4];
                let enc = c.encode_utf8(&mut b);
                for &byte in enc.as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if hex.is_ascii() {
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn strip_html(s: &str) -> String {
    let re1 = regex::Regex::new(r"(?is)<(script|style)[^>]*>.*?</\1>").ok();
    let mut t = s.to_string();
    if let Some(re) = re1 {
        t = re.replace_all(&t, " ").to_string();
    }
    if let Ok(re2) = regex::Regex::new(r"(?s)<[^>]+>") {
        t = re2.replace_all(&t, " ").to_string();
    }
    if let Ok(re3) = regex::Regex::new(r"\s+") {
        t = re3.replace_all(&t, " ").to_string();
    }
    t.trim().to_string()
}

pub(crate) fn http_client(secs: u64) -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(secs))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) MTECHOS/1.0")
        .build()
        .ok()
}

/// Utafutaji wa mtandaoni (DuckDuckGo HTML) — hutumika ndani ya moduli nyingine
/// (akademia, threatintel) pamoja na handler ya `/api/web/search`.
pub(crate) async fn web_search_internal(query: &str) -> Result<serde_json::Value, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("hakuna swali".into());
    }
    let client = http_client(12).ok_or("imeshindwa kujenga client")?;
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlenc(&query));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Mtandaoni haupatikani sasa: {e}"))?;
    let html = resp.text().await.unwrap_or_default();
    let Ok(anchor_re) = regex::Regex::new(r#"(?s)<a([^>]*)>(.*?)</a>"#) else {
        return Err("regex".into());
    };
    let Ok(href_re) = regex::Regex::new(r#"href="([^"]+)""#) else {
        return Err("regex".into());
    };
    let mut snips: Vec<String> = Vec::new();
    if let Ok(sr) = regex::Regex::new(r#"(?s)<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#) {
        for c in sr.captures_iter(&html) {
            if snips.len() >= 8 {
                break;
            }
            snips.push(strip_html(&c[1]));
        }
    }
    let mut results = Vec::new();
    for cap in anchor_re.captures_iter(&html) {
        let attrs = &cap[1];
        if !attrs.contains("result__a") {
            continue;
        }
        let Some(h) = href_re.captures(attrs) else {
            continue;
        };
        let href = h[1].to_string();
        let title = strip_html(&cap[2]);
        if title.is_empty() {
            continue;
        }
        let real = if let Some(pos) = href.find("uddg=") {
            pct_decode(&href[pos + 5..].split('&').next().unwrap_or(""))
        } else if href.starts_with("//") {
            format!("https:{href}")
        } else {
            href
        };
        let mut item = json!({ "title": title, "url": real });
        if let Some(s) = snips.get(results.len()) {
            item["snippet"] = json!(s);
        }
        results.push(item);
        if results.len() >= 8 {
            break;
        }
    }
    let n = results.len();
    Ok(json!({
        "ok": n > 0,
        "query": query,
        "count": n,
        "results": results,
        "source": "DuckDuckGo (mtandaoni halisi)",
        "note_sw": if n == 0 { "Hakuna matokeo (mtandaoni umekwama au hakuna kwa lile jina)." } else { "Matokeo ya utafutaji halisi kutoka mtandaoni." }
    }))
}

async fn web_search(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = q.get("q").cloned().unwrap_or_default().trim().to_string();
    match web_search_internal(&query).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

// ========== KUTAFUTA KWA SEARXNG (meta-search — self-hosted au public) ==========
// Njia halisi ya meta-search: SearXNG hukusanya matokeo kutoka engines nyingi
// (Google, Bing, DDG…) na kurudisha JSON. Self-hosted: server/searxng (port 8888,
// format ya json imewashwa). Weka `SEARXNG_URL`; la sivyo tunatumia instances za
// umma (zinaweza ku-rate-limit). Nyongeza — DDG (web_search) haibadiliki.

fn searxng_base() -> String {
    std::env::var("SEARXNG_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8888".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// SearXNG huchukua `time_range` (day|week|month|year) na `language` (sw, en…).
fn searxng_time_range(v: &str) -> Option<&'static str> {
    match v.to_lowercase().as_str() {
        "day" | "siku" => Some("day"),
        "week" | "wiki" => Some("week"),
        "month" | "mwezi" => Some("month"),
        "year" | "mwaka" => Some("year"),
        _ => None,
    }
}

/// Toa matokeo kutoka JSON ya SearXNG (`{"results":[{"title","url","content",…}]}`).
fn searxng_parse(v: &serde_json::Value, limit: usize) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    if let Some(arr) = v["results"].as_array() {
        for r in arr {
            if out.len() >= limit {
                break;
            }
            let title = r["title"].as_str().unwrap_or("").trim().to_string();
            let url = r["url"].as_str().unwrap_or("").trim().to_string();
            if title.is_empty() || url.is_empty() {
                continue;
            }
            let mut item = json!({ "title": title, "url": url });
            if let Some(c) = r["content"].as_str() {
                if !c.trim().is_empty() {
                    item["snippet"] = json!(c.trim());
                }
            }
            if let Some(e) = r["engine"].as_str() {
                item["engine"] = json!(e);
            }
            out.push(item);
        }
    }
    out
}

async fn web_searxng(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = q.get("q").cloned().unwrap_or_default().trim().to_string();
    if query.is_empty() {
        return Json(json!({ "ok": false, "error": "hakuna swali — tumia ?q=..." }));
    }
    let limit: usize = q
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
        .clamp(1, 30);
    let lang = q.get("lang").cloned().unwrap_or_else(|| "sw".to_string());
    let base = searxng_base();
    let mut url = format!(
        "{base}/search?q={}&format=json&safesearch=1&language={}",
        urlenc(&query),
        urlenc(&lang)
    );
    if let Some(tr) = q.get("time_range").map(|s| s.as_str()).and_then(searxng_time_range) {
        url.push_str(&format!("&time_range={tr}"));
    }
    let Some(client) = http_client(15) else {
        return Json(json!({ "ok": false, "error": "imeshindwa kujenga client" }));
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Json(json!({
                "ok": false,
                "error": format!("SearXNG haipatikani ({base}): {e}"),
                "note_sw": "Washa SearXNG: docker compose -f server/searxng/docker-compose.yml up -d, kisha weka SEARXNG_URL=http://127.0.0.1:8888",
                "fallback": "/api/web/search (DuckDuckGo — bila key)"
            }))
        }
    };
    if !resp.status().is_success() {
        return Json(json!({
            "ok": false,
            "error": format!("SearXNG status {}", resp.status()),
            "note_sw": "Hakikisha `formats` ina `json` kwenye settings.yml (tayari ipo kwenye server/searxng)."
        }));
    }
    let body = resp.text().await.unwrap_or_default();
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({
                "ok": false,
                "error": format!("SearXNG haikurudisha JSON: {e}"),
                "note_sw": "Washa format ya json kwenye settings.yml ya SearXNG."
            }))
        }
    };
    let results = searxng_parse(&parsed, limit);
    let n = results.len();
    Json(json!({
        "ok": n > 0,
        "query": query,
        "count": n,
        "results": results,
        "source": "SearXNG (meta-search)",
        "base": base,
        "engines_used": parsed["results"].as_array().map(|a| {
            let mut e: Vec<String> = a.iter()
                .filter_map(|r| r["engine"].as_str().map(|s| s.to_string()))
                .collect();
            e.sort();
            e.dedup();
            e
        }).unwrap_or_default(),
        "note_sw": if n == 0 { "Hakuna matokeo kutoka SearXNG (jaribu tena au tumia /api/web/search)." } else { "Matokeo halisi kutoka SearXNG meta-search." }
    }))
}

async fn web_fetch(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let url = q.get("url").cloned().unwrap_or_default().trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Json(json!({ "ok": false, "error": "url lazima ianze na http:// au https://" }));
    }
    let Some(client) = http_client(15) else {
        return Json(json!({ "ok": false, "error": "imeshindwa kujenga client" }));
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => return Json(json!({ "ok": false, "error": format!("Haijapatikana: {e}") })),
    };
    let html = resp.text().await.unwrap_or_default();
    let title = regex::Regex::new(r"(?is)<title[^>]*>(.*?)</title>")
        .ok()
        .and_then(|re| re.captures(&html))
        .map(|c| strip_html(&c[1]))
        .unwrap_or_default();
    let text = strip_html(&html);
    let clipped: String = text.chars().take(6000).collect();
    Json(json!({
        "ok": true,
        "url": url,
        "title": title,
        "text": clipped,
        "chars_total": text.chars().count(),
        "bytes": html.len(),
        "note": "maudhui ya ukurasa yametolewa na kusafishwa (HTML imeondolewa)"
    }))
}

// ========== MMILIKI WA PC (majina halisi ya wenye computer) ==========

fn owners_path() -> std::path::PathBuf {
    data_io::data_dir().join("computers_owners.json")
}

fn load_owners() -> serde_json::Value {
    std::fs::read_to_string(owners_path())
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

async fn computers_owners() -> Json<serde_json::Value> {
    let owners = load_owners();
    let n = owners.as_object().map(|m| m.len()).unwrap_or(0);
    Json(json!({ "ok": true, "count": n, "owners": owners }))
}

#[derive(serde::Deserialize)]
struct OwnerReq {
    mac: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    note: String,
}

async fn computers_owner_set(Json(r): Json<OwnerReq>) -> Json<serde_json::Value> {
    let mac = r.mac.trim().to_lowercase();
    if mac.split(|c| c == ':' || c == '-').count() != 6 {
        return Json(json!({ "ok": false, "error": format!("MAC batili: {}", r.mac) }));
    }
    let mut owners = load_owners();
    let count;
    {
        let m = owners.as_object_mut().expect("owners ni object");
        if r.owner.trim().is_empty() {
            m.remove(&mac);
        } else {
            m.insert(
                mac.clone(),
                json!({
                    "owner": r.owner.trim(),
                    "note": r.note.trim(),
                    "updated_at": chrono::Local::now().to_rfc3339()
                }),
            );
        }
        count = m.len();
    }
    let p = owners_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::write(
        &p,
        serde_json::to_string_pretty(&owners).unwrap_or_else(|_| "{}".into()),
    ) {
        Ok(_) => Json(json!({ "ok": true, "mac": mac, "owners": count })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn security_task(
    State(s): State<AppState>,
    Json(r): Json<SecTaskReq>,
) -> Json<serde_json::Value> {
    Json(secpack::run_task(&s.db, &r.task, &r.target, r.human_approved).await)
}

async fn security_runs(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(secpack::runs_json(&s.db).await)
}

// ---------- THREATSWARM (autonomous kill-chain) ----------

#[derive(serde::Deserialize)]
pub struct SecTsAgentReq {
    pub agent: String,
    pub target: String,
    #[serde(default)]
    pub human_approved: bool,
}

#[derive(serde::Deserialize)]
pub struct TsEngageReq {
    pub target: String,
    #[serde(default)]
    pub human_approved: bool,
    #[serde(default)]
    pub internal_agent: bool,
}

async fn threatswarm_engage(
    State(s): State<AppState>,
    Json(r): Json<TsEngageReq>,
) -> Json<serde_json::Value> {
    Json(threatswarm::engage(&s.db, &r.target, r.human_approved, r.internal_agent).await)
}

async fn threatswarm_scope_check(
    Json(r): Json<TsEngageReq>,
) -> Json<serde_json::Value> {
    match threatswarm::check_scope(&r.target) {
        Ok(()) => Json(json!({ "ok": true, "target": r.target, "in_scope": true })),
        Err(e) => Json(json!({ "ok": false, "target": r.target, "in_scope": false, "error": e })),
    }
}

async fn threatswarm_reports() -> Json<serde_json::Value> {
    Json(threatswarm::reports_json().await)
}

async fn threatswarm_agents() -> Json<serde_json::Value> {
    Json(threatswarm::agents_catalog())
}

async fn threatswarm_agent_run(
    State(s): State<AppState>,
    Json(r): Json<SecTsAgentReq>,
) -> Json<serde_json::Value> {
    Json(threatswarm::run_agent(&s.db, &r.agent, &r.target, r.human_approved).await)
}

// ---------- SCOPE MANAGEMENT (admin anaongeza/kutoa malengo) ----------

async fn threatswarm_scope_list() -> Json<serde_json::Value> {
    Json(threatswarm::scope_json())
}

#[derive(serde::Deserialize)]
pub struct ScopeEntryReq {
    pub entry: String,
}

async fn threatswarm_scope_add(Json(r): Json<ScopeEntryReq>) -> Json<serde_json::Value> {
    match threatswarm::scope_add(&r.entry) {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn threatswarm_scope_remove(Json(r): Json<ScopeEntryReq>) -> Json<serde_json::Value> {
    match threatswarm::scope_remove(&r.entry) {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

// ========== THREAT INTELLIGENCE (defensive, consent-based) ==========
// Kila uchanganuzi unachanganua data ALIYOTUMWA na akaunti (consent).
// Huhifadhi ripoti pale `save` = true.

#[derive(serde::Deserialize)]
pub struct TiSmsReq {
    pub text: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub save: bool,
    #[serde(default)]
    pub note: String,
}

async fn ti_analyze_sms(
    State(s): State<AppState>,
    Json(r): Json<TiSmsReq>,
) -> Json<serde_json::Value> {
    if r.text.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "ujumbe ni tupu" }));
    }
    let a = threatintel::score_message(&r.text);
    let mut out = json!({
        "ok": true,
        "kind": "sms",
        "score": a.score,
        "verdict": a.verdict,
        "signals": a.signals,
    });
    if r.save && !r.account.trim().is_empty() {
        let id = threatintel::save_report(
            &s.db,
            &r.account,
            "sms",
            "(ujumbe)",
            a.score,
            &a.verdict,
            &a.signals,
            &r.note,
        )
        .await
        .unwrap_or_default();
        out["report_id"] = json!(id);
    }
    Json(out)
}

#[derive(serde::Deserialize)]
pub struct TiUrlReq {
    pub url: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub save: bool,
}

async fn ti_analyze_url(
    State(s): State<AppState>,
    Json(r): Json<TiUrlReq>,
) -> Json<serde_json::Value> {
    if r.url.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "url ni tupu" }));
    }
    let a = threatintel::analyze_url(&r.url);
    let signals: Vec<threatintel::Signal> = a
        .reasons
        .iter()
        .map(|d| threatintel::Signal {
            id: "url_reason".into(),
            weight: 0,
            detail: d.clone(),
        })
        .collect();
    let mut out = json!({
        "ok": true,
        "kind": "url",
        "host": a.host,
        "score": a.score,
        "verdict": a.verdict,
        "brand": a.brand,
        "reasons": a.reasons,
    });
    if r.save && !r.account.trim().is_empty() {
        let id = threatintel::save_report(
            &s.db,
            &r.account,
            "url",
            &a.host,
            a.score,
            &a.verdict,
            &signals,
            "",
        )
        .await
        .unwrap_or_default();
        out["report_id"] = json!(id);
    }
    Json(out)
}

#[derive(serde::Deserialize)]
pub struct TiCallerReq {
    pub presented: String,
    pub actual: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub save: bool,
}

async fn ti_analyze_caller(
    State(s): State<AppState>,
    Json(r): Json<TiCallerReq>,
) -> Json<serde_json::Value> {
    let a = threatintel::check_caller(&r.presented, &r.actual);
    let signals: Vec<threatintel::Signal> = a
        .reasons
        .iter()
        .map(|d| threatintel::Signal {
            id: "caller_reason".into(),
            weight: 0,
            detail: d.clone(),
        })
        .collect();
    let mut out = json!({
        "ok": true,
        "kind": "caller",
        "spoofed": a.spoofed,
        "score": a.score,
        "verdict": a.verdict,
        "reasons": a.reasons,
    });
    if r.save && !r.account.trim().is_empty() {
        let id = threatintel::save_report(
            &s.db,
            &r.account,
            "caller",
            &threatintel::normalize_phone(&r.presented),
            a.score,
            &a.verdict,
            &signals,
            "",
        )
        .await
        .unwrap_or_default();
        out["report_id"] = json!(id);
    }
    Json(out)
}

#[derive(serde::Deserialize)]
pub struct TiCallbackReq {
    #[serde(default)]
    pub checkout_request_id: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub expected_amount: f64,
    #[serde(default)]
    pub request_created_at: i64,
    #[serde(default)]
    pub callback_at: i64,
    #[serde(default)]
    pub signature_ok: bool,
    #[serde(default)]
    pub ip_whitelisted: bool,
    #[serde(default)]
    pub already_settled: bool,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub save: bool,
}

/// ULINZI: tathmini callback ya malipo inayoingia (M-Pesa/HaloPesa n.k.) kabla
/// ya kuamini malipo. Hutambua ughushi, replay, na kukwepa whitelist.
async fn ti_verify_callback(
    State(s): State<AppState>,
    Json(r): Json<TiCallbackReq>,
) -> Json<serde_json::Value> {
    let already = r.already_settled
        || threatintel::callback_seen(&s.db, &r.checkout_request_id).await;
    let input = threatintel::CallbackInput {
        checkout_request_id: r.checkout_request_id.clone(),
        amount: r.amount,
        expected_amount: r.expected_amount,
        request_created_at: r.request_created_at,
        callback_at: r.callback_at,
        signature_ok: r.signature_ok,
        ip_whitelisted: r.ip_whitelisted,
        already_settled: already,
    };
    let a = threatintel::verify_payment_callback(&input);
    // Rekodi callback kwenye leja ili replay ijayo igundulike.
    let _ = threatintel::callback_record(
        &s.db,
        &r.checkout_request_id,
        a.trusted,
        a.score,
    )
    .await;
    let signals: Vec<threatintel::Signal> = a
        .reasons
        .iter()
        .map(|d| threatintel::Signal {
            id: "callback_reason".into(),
            weight: 0,
            detail: d.clone(),
        })
        .collect();
    let mut out = json!({
        "ok": true,
        "kind": "payment_callback",
        "trusted": a.trusted,
        "score": a.score,
        "verdict": a.verdict,
        "reasons": a.reasons,
        "action_sw": if a.trusted { "kubali malipo" } else { "USIKUBALI — thibitisha kwa njia nyingine" },
    });
    if r.save && !r.account.trim().is_empty() {
        let id = threatintel::save_report(
            &s.db,
            &r.account,
            "payment_callback",
            &r.checkout_request_id,
            a.score,
            &a.verdict,
            &signals,
            "",
        )
        .await
        .unwrap_or_default();
        out["report_id"] = json!(id);
    }
    Json(out)
}

#[derive(serde::Deserialize)]
pub struct TiLimitReq {
    #[serde(default)]
    pub limit: i64,
}

// ---------- PHOENIX SHIELD (defensive) ----------

/// GET /api/shield/posture — tathmini ya ulinzi wa mfumo huu.
async fn shield_posture() -> Json<serde_json::Value> {
    Json(phoenix_shield::posture())
}

#[derive(serde::Deserialize)]
pub struct ShieldLabReq {
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub authorized: bool,
}

/// POST /api/shield/lab/plan — mpango wa pen-testing wa LAB (HITL, mfumo wako).
async fn shield_lab_plan(Json(r): Json<ShieldLabReq>) -> Json<serde_json::Value> {
    let target = if r.target.trim().is_empty() {
        "(mfumo wako)".to_string()
    } else {
        r.target
    };
    Json(phoenix_shield::lab_plan(&target, r.authorized))
}

#[derive(serde::Deserialize)]
pub struct ShieldHardenReq {
    #[serde(default)]
    pub apply: bool,
}

/// POST /api/shield/harden/plan — mpango wa kuimarisha (dry-run bila apply).
async fn shield_harden_plan(Json(r): Json<ShieldHardenReq>) -> Json<serde_json::Value> {
    Json(phoenix_shield::hardening_plan(r.apply))
}

#[derive(serde::Deserialize)]
pub struct ShieldAnalyticsReq {
    #[serde(default)]
    pub events: Vec<f64>,
    #[serde(default)]
    pub severities: Vec<f64>,
}

/// POST /api/shield/analytics — takwimu za R (anomaly/risk/forecast).
async fn shield_analytics(Json(r): Json<ShieldAnalyticsReq>) -> Json<serde_json::Value> {
    Json(phoenix_shield::r_analytics(&r.events, &r.severities).await)
}

/// GET /api/shield/coverage — ramani ya hati → engine ya ulinzi halisi.
async fn shield_coverage() -> Json<serde_json::Value> {
    Json(phoenix_shield::coverage())
}

/// POST /api/shield/agent — **Autonomous Security Engineer**: mzunguko kamili
/// kuanzia upimaji hadi ripoti kwa millisecond. Hatua za lab huzuiwa bila HITL.
async fn shield_agent(
    State(s): State<AppState>,
    Json(r): Json<phoenix_agent::AgentReq>,
) -> Json<serde_json::Value> {
    Json(phoenix_agent::run(&s.db, &r).await)
}

async fn ti_blacklist(
    State(s): State<AppState>,
    Query(q): Query<TiLimitReq>,
) -> Json<serde_json::Value> {
    let limit = if q.limit <= 0 { 100 } else { q.limit };
    Json(threatintel::blacklist_list(&s.db, limit).await)
}

#[derive(serde::Deserialize)]
pub struct TiBlacklistAddReq {
    pub number: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub added_by: String,
}

async fn ti_blacklist_add(
    State(s): State<AppState>,
    Json(r): Json<TiBlacklistAddReq>,
) -> Json<serde_json::Value> {
    let by = if r.added_by.trim().is_empty() {
        "admin"
    } else {
        r.added_by.trim()
    };
    match threatintel::blacklist_add(&s.db, &r.number, &r.reason, by).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ti_blacklist_remove(
    State(s): State<AppState>,
    Json(r): Json<ScopeEntryReq>,
) -> Json<serde_json::Value> {
    match threatintel::blacklist_remove(&s.db, &r.entry).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ti_patterns(
    State(s): State<AppState>,
    Query(q): Query<TiLimitReq>,
) -> Json<serde_json::Value> {
    let limit = if q.limit <= 0 { 100 } else { q.limit };
    Json(threatintel::pattern_list(&s.db, limit).await)
}

#[derive(serde::Deserialize)]
pub struct TiPatternAddReq {
    pub pattern: String,
    #[serde(default)]
    pub description: String,
}

async fn ti_patterns_add(
    State(s): State<AppState>,
    Json(r): Json<TiPatternAddReq>,
) -> Json<serde_json::Value> {
    match threatintel::pattern_add(&s.db, &r.pattern, &r.description).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ti_summary(
    State(s): State<AppState>,
    axum::extract::Path(account): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    Json(threatintel::summary(&s.db, &account).await)
}

/// Agents za ULINZI kutoka ThreatSwarm (zinaendeshwa kwa scope.txt + HITL).
async fn ti_agents() -> Json<serde_json::Value> {
    Json(threatintel::defensive_agents())
}

// ========== TOTAL SECURITY MODULES (tsmod) — injini HALISI ==========
// Module moja kwa faili moja. Kila run hufanya kazi kwenye mfumo halisi pale
// apply=true (HITL); vinginevyo dry-run. HAKUNA stub/demo.

async fn ts_modules_catalog() -> Json<serde_json::Value> {
    Json(tsmod::catalog_json())
}

#[derive(serde::Deserialize)]
pub struct TsModuleReq {
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

async fn ts_module_run(
    State(s): State<AppState>,
    Path(id): Path<u16>,
    Json(r): Json<TsModuleReq>,
) -> Json<serde_json::Value> {
    let Some(m) = tsmod::find(id) else {
        return Json(json!({ "ok": false, "error": format!("module {id} haipo kwenye rejista") }));
    };
    match m.run_db(&s.db, &r.target, &r.params).await {
        Ok(rep) => Json(rep.to_json()),
        Err(e) => Json(json!({ "ok": false, "module": id, "error": e })),
    }
}

async fn ts_module_status(State(s): State<AppState>, Path(id): Path<u16>) -> Json<serde_json::Value> {
    let Some(m) = tsmod::find(id) else {
        return Json(json!({ "ok": false, "error": format!("module {id} haipo kwenye rejista") }));
    };
    match m
        .run_db(&s.db, "", &serde_json::json!({ "op": "status" }))
        .await
    {
        Ok(rep) => Json(rep.to_json()),
        Err(e) => Json(json!({ "ok": false, "module": id, "error": e })),
    }
}

// ---- TOTAL SECURITY: usimamizi wa sheria/IOC/events (Admin Mkuu pekee) ----

#[derive(serde::Deserialize)]
pub struct TsRuleReq {
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub category: String,
    pub kind: String,
    pub pattern: String,
    #[serde(default = "default_sev")]
    pub severity: u8,
    #[serde(default)]
    pub source: String,
}
fn default_sev() -> u8 {
    60
}

async fn ts_rules_list(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(tsmod::store::list_rules(&s.db, None, 2000).await)
}

async fn ts_rules_add(State(s): State<AppState>, Json(r): Json<TsRuleReq>) -> Json<serde_json::Value> {
    let platform = if r.platform.trim().is_empty() { "*" } else { r.platform.trim() };
    let category = if r.category.trim().is_empty() { "*" } else { r.category.trim() };
    let source = if r.source.trim().is_empty() { "admin" } else { r.source.trim() };
    match tsmod::store::add_rule(&s.db, platform, category, &r.kind, &r.pattern, r.severity, source).await {
        Ok(rule) => Json(json!({ "ok": true, "rule": rule })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

#[derive(serde::Deserialize)]
pub struct TsIdReq {
    pub id: i64,
    #[serde(default)]
    pub enabled: bool,
}

async fn ts_rules_toggle(State(s): State<AppState>, Json(r): Json<TsIdReq>) -> Json<serde_json::Value> {
    match tsmod::store::set_enabled(&s.db, r.id, r.enabled).await {
        Ok(()) => Json(json!({ "ok": true, "id": r.id, "enabled": r.enabled })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ts_rules_remove(State(s): State<AppState>, Json(r): Json<TsIdReq>) -> Json<serde_json::Value> {
    match tsmod::store::remove_rule(&s.db, r.id).await {
        Ok(hit) => Json(json!({ "ok": true, "id": r.id, "removed": hit })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

#[derive(serde::Deserialize)]
pub struct TsIocReq {
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub source: String,
    #[serde(default = "default_sev")]
    pub severity: u8,
}

async fn ts_ioc_add(State(s): State<AppState>, Json(r): Json<TsIocReq>) -> Json<serde_json::Value> {
    let source = if r.source.trim().is_empty() { "admin" } else { r.source.trim() };
    match tsmod::store::add_ioc(&s.db, &r.kind, &r.value, source, r.severity).await {
        Ok(()) => Json(json!({ "ok": true, "kind": r.kind, "value": r.value })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ts_ioc_list(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(tsmod::store::list_iocs(&s.db, 2000).await)
}

async fn ts_events_list(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(tsmod::store::list_events(&s.db, 500).await)
}

#[derive(serde::Deserialize)]
pub struct TsFeedbackReq {
    pub event_id: String,
    pub verdict: String,
    #[serde(default)]
    pub by: String,
}

async fn ts_feedback_add(State(s): State<AppState>, Json(r): Json<TsFeedbackReq>) -> Json<serde_json::Value> {
    match tsmod::store::add_feedback(&s.db, &r.event_id, &r.verdict, &r.by).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

async fn ts_store_stats(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(tsmod::store::stats(&s.db).await)
}

// ---- TOTAL SECURITY: AKILI (LLM) — uchanganuzi wa maana + kujifunza ----

#[derive(serde::Deserialize)]
pub struct TsAiReq {
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub category: String,
    pub content: String,
    #[serde(default)]
    pub learn: bool,
}

/// Uchanganuzi wa AI (semantic) — hurudisha uamuzi + sababu + mbinu mpya.
async fn ts_ai_analyze(State(s): State<AppState>, Json(r): Json<TsAiReq>) -> Json<serde_json::Value> {
    let platform = if r.platform.trim().is_empty() { "Facebook" } else { r.platform.trim() };
    let category = if r.category.trim().is_empty() { "content" } else { r.category.trim() };
    Json(tsmod::social::ai::autonomous_scan(&s.db, platform, category, &r.content, r.learn).await)
}

#[derive(serde::Deserialize)]
pub struct TsAiProposeReq {
    #[serde(default)]
    pub platform: String,
    pub samples: Vec<String>,
    #[serde(default)]
    pub apply: bool,
}

/// LLM hupendekeza SHERIA MPYA kutoka mifano; `apply` huweka moja kwa moja (Admin).
async fn ts_ai_propose(State(s): State<AppState>, Json(r): Json<TsAiProposeReq>) -> Json<serde_json::Value> {
    let platform = if r.platform.trim().is_empty() { "*" } else { r.platform.trim() };
    match tsmod::social::ai::propose_rules(&s.db, platform, &r.samples).await {
        Some(rules) => {
            let mut added = Vec::new();
            if r.apply {
                for (kind, pattern, sev) in &rules {
                    if tsmod::store::add_rule(&s.db, platform, "content", kind, pattern, *sev, "llm")
                        .await
                        .is_ok()
                    {
                        added.push(pattern.clone());
                    }
                }
            }
            Json(json!({
                "ok": true, "platform": platform,
                "proposed": rules.iter().map(|(k,p,s)| json!({"kind":k,"pattern":p,"severity":s})).collect::<Vec<_>>(),
                "applied": added, "applied_count": added.len()
            }))
        }
        None => Json(json!({ "ok": false, "error": "LLM haipatikani au haikutoa sheria" })),
    }
}

#[derive(serde::Deserialize)]
pub struct TiReportReq {
    pub account: String,
    #[serde(default)]
    pub data: Vec<f64>,
    #[serde(default)]
    pub note: String,
}

/// Ripoti ya threat intel: muhtasari + (kama data ipo) trend ya ulaghai kwa R.
async fn ti_report(
    State(s): State<AppState>,
    Json(r): Json<TiReportReq>,
) -> Json<serde_json::Value> {
    if r.account.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "account inahitajika" }));
    }
    let mut out = threatintel::summary(&s.db, &r.account).await;
    // Trend ya ulaghai kwa R (kama data ya matukio/siku imetolewa)
    if r.data.len() >= 4 {
        let trend = r_engine::run_req(
            "fraud_trend",
            &json!({ "data": r.data }),
        )
        .await;
        out["fraud_trend"] = trend;
    }
    if !r.note.trim().is_empty() {
        out["note"] = json!(r.note);
    }
    Json(out)
}


async fn security_shield(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(shield::stats(&s.db).await)
}

// MASS FLEET: masundo ya lengo 700M max, wendeshaji 64 kwa WAKATI MMOJA
#[derive(serde::Deserialize)]
struct SecurityMassReq {
    task: String,
    targets: Vec<String>,
    #[serde(default)]
    limit: usize,
    #[serde(default)]
    human_approved: bool,
    #[serde(default)]
    internal_agent: bool,
}

async fn security_mass(
    State(s): State<AppState>,
    Json(r): Json<SecurityMassReq>,
) -> Json<serde_json::Value> {
    Json(
        secpack::mass_run(
            &s.db,
            &secpack::MassReq {
                task: r.task,
                targets: r.targets,
                limit: r.limit,
                human_approved: r.human_approved,
                internal_agent: r.internal_agent,
            },
        )
        .await,
    )
}

// RIPOTI YA HTML venye rangi: /api/security/report (kwa browser au agent)
async fn security_report(State(s): State<AppState>) -> Html<String> {
    Html(secpack::report_html(&s.db).await)
}

// ---------- ENGINE YA HESABU: MATRIX + CALCULUS + QUANTUM (Rust mtupu) ----------
// Hakuna eval wala shell: parser wa kazi + AST, matriksi ndani ya Rust,
// quantum ni statevector halisi (C64 yetu). Zote zinarudi JSON.


async fn quantum_run(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let _ = &s.db;
    Json(quantum::api_run(&r))
}

async fn quantum_random(
    State(s): State<AppState>,
    Json(r): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let _ = &s.db;
    Json(quantum::api_qrng(&r))
}

// ---------- MIKATABA NA RIPOTI (templates 7 + muhuri wa SHA-256 + usaini) ----------


#[derive(serde::Deserialize)]
pub struct ContractCreateReq {
    pub template: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub fields: serde_json::Value,
}


#[derive(serde::Deserialize)]
pub struct ContractSignReq {
    #[serde(default)]
    pub signature: String,
}



// ---------- RIPOTI ZA WATEJA (H13): PDF + CSV ----------


// ---------- AGENT LAYER (H14): register / heartbeat / poll / report ----------

#[derive(serde::Deserialize)]
struct FleetRegisterReq {
    device: String,
    #[serde(default)]
    os_type: String,
    #[serde(default)]
    health: String,
}

async fn fleet_register(
    State(s): State<AppState>,
    Json(r): Json<FleetRegisterReq>,
) -> Json<serde_json::Value> {
    match fleet::register(&s.db, &r.device, &r.os_type, &r.health).await {
        Ok((id, token, now_ms)) => Json(json!({
            "ok": true, "agent_id": id, "token": token,
            "heartbeat_after_ms": 20_000,
            "note_sw": "Hifadhi token mahali salama kwenye kifaa — ndiyo utambulisho wako wa kazi.",
        })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

#[derive(serde::Deserialize)]
struct FleetPingReq {
    agent_id: String,
    token: String,
    #[serde(default)]
    health: String,
}

async fn fleet_heartbeat(
    State(s): State<AppState>,
    Json(r): Json<FleetPingReq>,
) -> Json<serde_json::Value> {
    match fleet::heartbeat(&s.db, &r.agent_id, &r.token, &r.health).await {
        Ok((id, device, status, job)) => Json(json!({
            "ok": true, "agent_id": id, "device": device, "status": status, "current_job": job,
        })),
        Err(e) => Json(json!({ "ok": false, "error": e, "reauth": true })),
    }
}

#[derive(serde::Deserialize)]
struct FleetPollReq {
    agent_id: String,
    token: String,
}

async fn fleet_poll(
    State(s): State<AppState>,
    Json(r): Json<FleetPollReq>,
) -> Json<serde_json::Value> {
    let Some((_, device)) = fleet::auth(&s.db, &r.agent_id, &r.token).await else {
        return Json(
            json!({ "ok": false, "error": "token si sahihi — jisajili upya", "reauth": true }),
        );
    };
    Json(fleet::poll(&s.db, &s.jobs, &device).await)
}

#[derive(serde::Deserialize)]
struct FleetReportReq {
    agent_id: String,
    token: String,
    progress: u32,
    #[serde(default)]
    message: String,
}

async fn fleet_report(
    State(s): State<AppState>,
    Json(r): Json<FleetReportReq>,
) -> Json<serde_json::Value> {
    let Some((_, device)) = fleet::auth(&s.db, &r.agent_id, &r.token).await else {
        return Json(
            json!({ "ok": false, "error": "token si sahihi — jisajili upya", "reauth": true }),
        );
    };
    Json(fleet::report(&s.db, &s.jobs, &device, r.progress, &r.message).await)
}

async fn fleet_agents(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(fleet::list(&s.db).await)
}

// =====================================================================
// HANDLERS: ADMIN CONSOLE (chat ya cyber/forensics + ripoti yenye muhuri)
// =====================================================================

#[derive(serde::Deserialize)]
pub struct ConsoleReq {
    pub prompt: String,
    #[serde(default)]
    pub format: String,
}

async fn admin_console_run(State(s): State<AppState>, Json(r): Json<ConsoleReq>) -> Json<serde_json::Value> {
    if r.prompt.trim().is_empty() {
        return Json(json!({"ok": false, "error": "prompt ni lazima"}));
    }
    let (job, target) = admin_console::classify(&r.prompt);
    let res = admin_console::execute(job, &target);
    let doc = match admin_console::report(&s.db, &res).await {
        Ok(d) => d,
        Err(e) => return Json(json!({"ok": false, "error": e})),
    };
    let _ = admin_console::log_job(&s.db, &r.prompt, &res, &doc.seal.content_hash).await;
    if r.format == "pdf" {
        // PDF inarudishwa kama base64 ili UI iweze kuipakua (download).
        use base64::Engine as _;
        let pdf = admin_console::report_pdf(&res, &doc.seal);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&pdf);
        return Json(json!({
            "ok": true, "job": job, "target": target, "severity": res.severity,
            "seal": doc.seal, "pdf_b64": b64, "pdf_bytes": pdf.len(),
        }));
    }
    Json(json!({
        "ok": true, "job": job, "target": target,
        "severity": res.severity, "findings": res.findings,
        "seal": doc.seal, "sealed_html": doc.sealed_html,
    }))
}

async fn admin_console_jobs(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(admin_console::jobs(&s.db, 200).await)
}

async fn admin_console_overview(State(s): State<AppState>) -> Json<serde_json::Value> {
    let chain = pkicrypto::verify_chain(&s.db).await.ok();
    let jobs = admin_console::jobs(&s.db, 5).await;
    Json(json!({
        "ok": true,
        "crypto_chain": chain,
        "recent_jobs": jobs["jobs"],
        "note": "Admin anaona muhuri (crypto) na kumbukumbu za kazi za cyber/forensics.",
    }))
}

// =====================================================================
// HANDLERS: PKI (ECDSA + SHA-256)
// =====================================================================

async fn crypto_suite(State(s): State<AppState>) -> Json<serde_json::Value> {
    let _ = &s.db;
    Json(pkicrypto::crypto_suite_json())
}

async fn crypto_keys(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(pkicrypto::list_keys(&s.db).await)
}

async fn crypto_rotate(State(s): State<AppState>) -> Json<serde_json::Value> {
    let _ = sqlx::query("UPDATE crypto_keys SET active=0").execute(&s.db).await;
    match pkicrypto::ensure_company_key(&s.db).await {
        Ok(k) => Json(json!({"ok": true, "rotated": true, "fingerprint": k.fingerprint})),
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}

async fn crypto_sign(State(s): State<AppState>, Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let data = r["data"].as_str().unwrap_or("");
    match pkicrypto::active_private_pem(&s.db).await {
        Ok(pem) => match pkicrypto::sign(&pem, data.as_bytes()) {
            Ok(sig) => Json(json!({"ok": true, "alg": pkicrypto::ALG_ECDSA,
                "signature": sig, "sha256": pkicrypto::sha256_hex(data.as_bytes())})),
            Err(e) => Json(json!({"ok": false, "error": e})),
        },
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}

async fn crypto_verify(State(s): State<AppState>, Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let data = r["data"].as_str().unwrap_or("");
    let sig = r["signature"].as_str().unwrap_or("");
    let key = match pkicrypto::active_key(&s.db).await {
        Ok(k) => k,
        Err(e) => return Json(json!({"ok": false, "error": e})),
    };
    match pkicrypto::verify(&key.public_pem, data.as_bytes(), sig) {
        Ok(v) => Json(json!({"ok": true, "valid": v, "key_fingerprint": key.fingerprint})),
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}

async fn crypto_seal(State(s): State<AppState>, Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let title = r["title"].as_str().unwrap_or("Hati");
    let body = r["body"].as_str().unwrap_or("");
    match pkicrypto::seal_document(&s.db, title, body).await {
        Ok(doc) => Json(json!({"ok": true, "seal": doc.seal, "sealed_html": doc.sealed_html})),
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}

async fn crypto_verify_seal(State(s): State<AppState>, Json(r): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let payload = r["payload"].as_str().unwrap_or("");
    let seal: pkicrypto::Seal = match serde_json::from_value(r["seal"].clone()) {
        Ok(x) => x,
        Err(e) => return Json(json!({"ok": false, "error": format!("seal si sahihi: {e}")})),
    };
    match pkicrypto::verify_seal(&s.db, payload, &seal).await {
        Ok(v) => Json(json!({"ok": true, "result": v})),
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}

async fn crypto_ledger(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(pkicrypto::ledger(&s.db, 200).await)
}

async fn crypto_ledger_verify(State(s): State<AppState>) -> Json<serde_json::Value> {
    match pkicrypto::verify_chain(&s.db).await {
        Ok(st) => Json(json!({"ok": true, "chain": st})),
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}



// =====================================================================
// HANDLERS: AGENT-ENGINE — CYBER SECURITY + DIGITAL FORENSICS
// (akili-security imehamishwa kutoka AGENT-ENGINE; agents 27 huongoza zana)
// =====================================================================

/// Hali ya injini ya usalama: modules, agents 27, majukwaa, finshield, apiauth.
async fn sec_engine_status() -> Json<serde_json::Value> {
    let cat = tsmod::catalog_json();
    let f = finshield::catalog();
    Json(json!({
        "ok": true,
        "engine": "akili-security (AGENT-ENGINE)",
        "modules": cat["count"].clone(),
        "swarm_agents": threatswarm::AGENTS.len(),
        "swarm_languages": ["rust", "go", "elixir", "r"],
        "platforms": platforms::names(),
        "platform_commands": platforms::catalog()["total_commands"].clone(),
        "finshield_ops": finshield::OPS.len(),
        "apiauth_ops": ["jwt", "oauth", "nacos"],
        "gateprobe": "live HEAD probe (HITL)",
        "forensics": "dfir agent + majukwaa (dfir task) + threatintel",
        "hitl": "vitendo vinavyobadilisha mfumo vinahitaji apply=true",
        "shell": "hakuna (arg-array pekee)",
        "surveillance": "haujengwi",
        "scope": "swarm ni scope-gated",
        "finshield_catalog": f,
    }))
}

async fn sec_platforms() -> Json<serde_json::Value> {
    Json(platforms::catalog())
}

/// Katalogi kamili ya jukwaa moja (`?platform=kali`) au zote.
async fn sec_platforms_full(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let p = q.get("platform").map(|s| s.as_str()).filter(|s| !s.is_empty());
    Json(platforms::catalog_full(p))
}

/// Tafuta amri kwenye jukwaa (`?platform=kali&q=nmap&min_risk=high`).
async fn sec_platforms_lookup(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let platform = q.get("platform").cloned().unwrap_or_default();
    let query = q.get("q").cloned().unwrap_or_default();
    let min_risk = q.get("min_risk").map(|s| s.as_str()).filter(|s| !s.is_empty());
    Json(platforms::lookup(&platform, &query, min_risk))
}

#[derive(serde::Deserialize)]
pub struct SecPlatformPlanReq {
    #[serde(default)]
    pub platform: String,
    #[serde(default = "default_recon_task")]
    pub task: String,
    #[serde(default)]
    pub target: String,
}
fn default_recon_task() -> String {
    "recon".to_string()
}

async fn sec_platforms_plan(Json(r): Json<SecPlatformPlanReq>) -> Json<serde_json::Value> {
    Json(platforms::plan(&r.platform, &r.task, &r.target))
}

#[derive(serde::Deserialize)]
pub struct SecOpReq {
    #[serde(default)]
    pub op: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

async fn sec_finshield(Json(r): Json<SecOpReq>) -> Json<serde_json::Value> {
    Json(finshield::dispatch(&r.op, &r.data))
}

async fn sec_apiauth(Json(r): Json<SecOpReq>) -> Json<serde_json::Value> {
    Json(apiauth::dispatch(&r.op, &r.data))
}

#[derive(serde::Deserialize)]
pub struct SecGateprobeReq {
    pub url: String,
    #[serde(default)]
    pub authorized: bool,
}

async fn sec_gateprobe(Json(r): Json<SecGateprobeReq>) -> Json<serde_json::Value> {
    Json(gateprobe::probe(&r.url, r.authorized).await)
}

async fn sec_swarm_catalog() -> Json<serde_json::Value> {
    Json(threatswarm::catalog_json())
}

#[derive(serde::Deserialize)]
pub struct SecSwarmPlanReq {
    #[serde(default = "default_hunt_mode")]
    pub mode: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub scope: Vec<String>,
}
fn default_hunt_mode() -> String {
    "hunt".to_string()
}

async fn sec_swarm_plan(Json(r): Json<SecSwarmPlanReq>) -> Json<serde_json::Value> {
    Json(threatswarm::plan(&r.mode, &r.target, &r.scope))
}

#[derive(serde::Deserialize)]
pub struct SecSwarmConductReq {
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default = "default_hunt_mode")]
    pub mode: String,
    #[serde(default)]
    pub scope: Vec<String>,
}

async fn sec_swarm_conduct(Json(r): Json<SecSwarmConductReq>) -> Json<serde_json::Value> {
    let ports = if r.ports.is_empty() { vec![22, 80, 443, 8080] } else { r.ports };
    Json(threatswarm::conduct(&r.targets, &ports, &r.mode, &r.scope).await)
}

async fn sec_agentbind() -> Json<serde_json::Value> {
    Json(agentbind::catalog())
}

async fn sec_agentbind_finops() -> Json<serde_json::Value> {
    Json(agentbind::finops_catalog())
}

async fn sec_agentbind_authops() -> Json<serde_json::Value> {
    Json(agentbind::authops_catalog())
}

#[derive(serde::Deserialize)]
pub struct SecAgentPlanReq {
    pub agent: String,
    #[serde(default)]
    pub target: String,
}

async fn sec_agentbind_plan(Json(r): Json<SecAgentPlanReq>) -> Json<serde_json::Value> {
    Json(agentbind::agent_plan(&r.agent, &r.target))
}

async fn sec_agentbind_conduct(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let target = q.get("target").cloned().unwrap_or_default();
    Json(agentbind::conduct(&target))
}

/// DIGITAL FORENSICS: muhtasari wa uwezo wa uchunguzi (dfir agents + majukwaa + tools).
async fn sec_forensics() -> Json<serde_json::Value> {
    let dfir_agents: Vec<serde_json::Value> = threatswarm::AGENTS
        .iter()
        .filter(|a| a.class.is_defensive())
        .map(|a| json!({
            "agent": a.name, "title": a.title, "tasks": a.tasks, "tools": a.tools,
        }))
        .collect();
    let platforms_dfir: Vec<serde_json::Value> = platforms::names()
        .iter()
        .map(|p| platforms::plan(p, "dfir", "$EVIDENCE"))
        .collect();
    let ti = threatintel::defensive_agents();
    Json(json!({
        "ok": true,
        "engine": "akili-security/digital-forensics",
        "dfir_agents": dfir_agents,
        "dfir_agent_count": dfir_agents.len(),
        "platform_dfir_plans": platforms_dfir,
        "threatintel_defensive": ti,
        "rules": {
            "read_only": "uchunguzi wa forensics haubadilishi ushahidi",
            "hitl": "acquisition inahitaji idhini (--i-have-authorization)",
            "chain_of_custody": "hashing (SHA-256) kwa kila sampuli",
        },
    }))
}

// =====================================================================
// HANDLERS: PUBLIC (bure)
// =====================================================================


// =====================================================================
// MBILINYI PLATFORM: realtime (WebRTC signaling + push + chat/SMS)
// =====================================================================


#[derive(serde::Deserialize)]
pub struct RoomCreateReq {
    pub kind: String,
    #[serde(default)]
    pub created_by: String,
}


#[derive(serde::Deserialize)]
pub struct CommsMsgReq {
    pub room: String,
    pub sender: String,
    pub body: String,
}


#[cfg(test)]
mod web_tests {
    use super::*;

    /// RBAC huamua njia gani zinahitaji login na zipi ni za Admin Mkuu pekee.
    #[test]
    fn rbac_gate_inaainisha_kazi_za_admin() {
        // Njia za maamuzi ya biashara ni za Admin Mkuu.
        let (needs, admin) = rbac_gate("/api/biashara/request/7/approve");
        assert!(needs && admin);
        let (needs, admin) = rbac_gate("/api/biashara/employee/list");
        assert!(needs && admin);
        // Njia za mteja (kuwasilisha/kufuatilia ombi kwa jumla) zinabaki wazi
        // ili mteja aone maendeleo bila login.
        let (needs, admin) = rbac_gate("/api/biashara/status");
        assert!(!needs && !admin, "status ya biashara ni ya mteja — wazi");
        // Orodha/maelezo ya maombi yana jina+simu za wateja → Admin Mkuu pekee.
        let (needs, admin) = rbac_gate("/api/biashara/requests");
        assert!(needs && admin, "orodha ya maombi ina data ya wateja — Admin pekee");
        let (needs, admin) = rbac_gate("/api/biashara/request/REQ-2026-000001");
        assert!(needs && admin, "maelezo ya ombi yana data ya mteja — Admin pekee");
        // Mteja hana login: kuwasilisha na kufuatilia kwa ref+simu ni wazi.
        let (needs, admin) = rbac_gate("/api/biashara/request/create");
        assert!(!needs && !admin, "kuwasilisha ombi ni kwa mteja — wazi");
        let (needs, admin) = rbac_gate("/api/biashara/client/track");
        assert!(!needs && !admin, "kufuatilia maendeleo ni kwa mteja — wazi");
        // Kupeana kazi kwa mfanyakazi husika ni uamuzi wa Admin.
        let (needs, admin) = rbac_gate("/api/biashara/request/REQ-1/auto-assign");
        assert!(needs && admin);
        let (needs, admin) = rbac_gate("/api/biashara/employee/EMP-1/tasks");
        assert!(needs && admin);
        let (needs, admin) = rbac_gate("/api/ajira/status");
        assert!(!needs && !admin, "status ya ajira ni ya mwombaji — wazi");
        let (needs, admin) = rbac_gate("/api/security/engine/status");
        assert!(needs && !admin, "status ni ya session yoyote, si Admin pekee");
        // Njia isiyolindwa.
        let (needs, admin) = rbac_gate("/api/health");
        assert!(!needs && !admin);
    }

    /// Token hutolewa kutoka `X-Session-API-Key` (Canvas) au `Bearer` (nje).
    /// Kudhoofika huku kuliwahi kufanya UI yote ya Canvas ipate 401.
    #[test]
    fn rbac_token_inakubali_header_zote_mbili() {
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-session-api-key", "abc123".parse().unwrap());
        assert_eq!(rbac_token(&h).as_deref(), Some("abc123"));

        let mut h = axum::http::HeaderMap::new();
        h.insert("authorization", "Bearer xyz789".parse().unwrap());
        assert_eq!(rbac_token(&h).as_deref(), Some("xyz789"));

        // `X-Session-API-Key` ina kipaumbele ikiwa zote zipo.
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-session-api-key", "session".parse().unwrap());
        h.insert("authorization", "Bearer bearer".parse().unwrap());
        assert_eq!(rbac_token(&h).as_deref(), Some("session"));

        // Bila token, au header tupu, hakuna token.
        assert_eq!(rbac_token(&axum::http::HeaderMap::new()), None);
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-session-api-key", "   ".parse().unwrap());
        assert_eq!(rbac_token(&h), None);
        let mut h = axum::http::HeaderMap::new();
        h.insert("authorization", "Basic abc".parse().unwrap());
        assert_eq!(rbac_token(&h), None);
    }

    #[test]
    fn searxng_time_range_inaheshimu_maneno() {
        assert_eq!(searxng_time_range("day"), Some("day"));
        assert_eq!(searxng_time_range("Siku"), Some("day"));
        assert_eq!(searxng_time_range("week"), Some("week"));
        assert_eq!(searxng_time_range("mwezi"), Some("month"));
        assert_eq!(searxng_time_range("mwaka"), Some("year"));
        assert_eq!(searxng_time_range("hakuna"), None);
    }

    #[test]
    fn searxng_parse_inatoa_matokeo_salama() {
        let v = json!({
            "results": [
                {"title": "Umeme", "url": "https://x.tz", "content": "maelezo", "engine": "google"},
                {"title": "", "url": "https://y.tz"},
                {"title": "Solar", "url": "", "content": "z"}
            ]
        });
        let r = searxng_parse(&v, 10);
        assert_eq!(r.len(), 1, "maingizo yenye title/url tupu yanapuuzwa");
        assert_eq!(r[0]["title"], "Umeme");
        assert_eq!(r[0]["engine"], "google");
        assert_eq!(r[0]["snippet"], "maelezo");
    }

    #[test]
    fn searxng_parse_inaheshimu_limit() {
        let v = json!({ "results": [
            {"title": "a", "url": "u1"},
            {"title": "b", "url": "u2"},
            {"title": "c", "url": "u3"}
        ]});
        assert_eq!(searxng_parse(&v, 2).len(), 2);
    }

    #[test]
    fn searxng_base_default_ni_localhost() {
        std::env::remove_var("SEARXNG_URL");
        assert_eq!(searxng_base(), "http://127.0.0.1:8888");
    }
}
/// Root ya build ya canvas (`engines/mbilinyi/build`) — hutafutwa kwa CWD kwanza,
/// kisha karibu na binary. Hurudisha path ya kwanza yenye `index.html`.
fn canvas_build_dir() -> Option<std::path::PathBuf> {
    let mut cands: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from("engines/mbilinyi/build")];
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            cands.push(d.join("engines/mbilinyi/build"));
            dir = d.parent();
        }
    }
    cands.into_iter().find(|c| c.join("index.html").exists())
}

/// (inahitaji_session, ni_admin_pekee) kwa kila path.
fn rbac_gate(path: &str) -> (bool, bool) {
    // MTECHOS GROUP: vitendo vya MAAMUZI ya biashara ni vya Admin Mkuu pekee.
    // Njia za mteja/mwombaji (kuwasilisha ombi, kufuatilia maendeleo, kufanya
    // tathmini) zinabaki wazi kwa makusudi ili mteja aone maendeleo.
    let bz_request_admin = path.starts_with("/api/biashara/request/")
        && ["/approve", "/reject", "/assign", "/progress", "/review", "/deliver", "/close"]
            .iter()
            .any(|a| path.ends_with(a));
    // Orodha na maelezo ya maombi yana jina/simu/barua pepe za wateja — Admin
    // Mkuu pekee. `request/create` NA `client/track` zinabaki wazi: mteja
    // hana login, na hufuatilia kwa `ref` + simu (uthibitisho wa pili).
    let bz_requests_private = path == "/api/biashara/requests"
        || (path.starts_with("/api/biashara/request/") && !path.ends_with("/create"));
    let aj_admin = path.starts_with("/api/ajira/application/")
        && ["/stage", "/hire", "/reject", "/practical/assign", "/practical/grade"]
            .iter()
            .any(|a| path.ends_with(a));
    let admin = path.starts_with("/api/crypto/")
        || path.starts_with("/api/ts/scope/")
        || path.starts_with("/api/ts/rules")
        || path.starts_with("/api/ts/ioc")
        || path.starts_with("/api/ts/feedback")
        || path.starts_with("/api/ts/ai")
        || path.starts_with("/api/ti/blacklist/add")
        || path.starts_with("/api/ti/blacklist/remove")
        || path.starts_with("/api/ti/patterns/add")
        || path.starts_with("/api/security/gateprobe")
        || path.starts_with("/api/security/swarm/conduct")
        || path.starts_with("/api/security/agentbind/conduct")
        || path.starts_with("/deploy/auto")
        || path.contains("/approve")
        || path.contains("/cancel")
        || path.starts_with("/api/fleet/exec")
        // MTECHOS GROUP: usimamizi wa wafanyakazi, mikataba, maamuzi ya ombi,
        // escrow, ajira, wakala na SMS.
        || bz_request_admin
        || bz_requests_private
        || aj_admin
        || path.starts_with("/api/biashara/employee")
        || path.starts_with("/api/biashara/contract/")
        || path.starts_with("/api/biashara/audit")
        || path.starts_with("/api/biashara/branding/set")
        || path.starts_with("/api/biashara/branding/logo")
        || path.starts_with("/api/biashara/branding/reset")
        || path.starts_with("/api/escrow/hold")
        || path.starts_with("/api/escrow/release")
        || path.starts_with("/api/escrow/refund")
        || path.starts_with("/api/escrow/confirm")
        || path.starts_with("/api/ajira/role/")
        || path.starts_with("/api/ajira/question/add")
        || path.starts_with("/api/agents/company/agent/")
        || path.starts_with("/api/agents/company/task/")
        || path.starts_with("/api/agents/company/autonomy")
        || path.starts_with("/api/agents/company/runs")
        || path.starts_with("/api/sms/send")
        || path.starts_with("/api/sms/outbox")
        || path.starts_with("/api/sms/commands")
        || path.starts_with("/api/sms/command/");
    let session = admin
        || path.starts_with("/deploy")
        || path.starts_with("/api/toolkit/run")
        || path.starts_with("/api/security/task")
        || path.starts_with("/api/security/platforms")
        || path.starts_with("/api/security/finshield")
        || path.starts_with("/api/security/apiauth")
        || path.starts_with("/api/security/swarm")
        || path.starts_with("/api/security/agentbind")
        || path.starts_with("/api/security/engine")
        || path.starts_with("/api/security/forensics")
        || path.starts_with("/api/fleet/");
    (session, admin)
}

/// Toa token ya session kutoka headers za ombi: `X-Session-API-Key` (Canvas)
/// au `Authorization: Bearer` (wateja wa nje). `None` ikiwa hakuna.
fn rbac_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-session-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

/// Middleware: 401 bila token, 403 kwa Msaidizi kwenye kazi za Admin Mkuu.
async fn rbac_mw(
    State(s): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let path = req.uri().path().to_string();
    let (needs, admin) = rbac_gate(&path);
    if !needs {
        return next.run(req).await;
    }
    // Canvas (/ide) hutuma `X-Session-API-Key` (session key iliyoingizwa kwenye
    // HTML); wateja wa nje hutuma `Authorization: Bearer`. Tukubali zote mbili,
    // la sivyo UI ya Canvas hupata 401 kwenye njia zote za RBAC.
    let tok = rbac_token(req.headers());
    let sess = match tok {
        Some(t) => auth::verify_token(&s.db, &t).await,
        None => None,
    };
    match sess {
        None => (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({
                "ok": false,
                "error": "RBAC: kazi hii inahitaji login. Weka username na password (MBILINYI = Admin Mkuu)."
            })),
        )
            .into_response(),
        Some(sess) => {
            if admin && sess.role != "admin_mkuu" {
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    Json(json!({
                        "ok": false,
                        "error": format!(
                            "RBAC: kazi hii ni ya Admin Mkuu pekee — rollu yako ni {}.",
                            auth::Role::from_id(&sess.role).name_sw()
                        )
                    })),
                )
                    .into_response();
            }
            next.run(req).await
        }
    }
}

/// MBILINYI Agent Canvas — UI halisi (`engines/mbilinyi/build/index.html`)
/// ikiwa build/ ipo; vinginevyo JSON hint ya ku-build. Canvas hujengwa kwa base
/// `/ide`, hivyo assets zake ni `/ide/assets/*` (zinahudumiwa na `canvas_asset`).
async fn canvas_ui() -> axum::response::Response {
    use axum::response::{Html, IntoResponse};
    if let Some(dir) = canvas_build_dir() {
        return match std::fs::read_to_string(dir.join("index.html")) {
            Ok(html) => Html(crate::canvas_ext::inject_session_key(&html)).into_response(),
            Err(e) => Json(json!({
                "ok": false,
                "error": format!("index.html imeshindwa kusomwa: {e}")
            }))
            .into_response(),
        };
    }
    Json(json!({
        "ok": false,
        "ui": "MBILINYI IDE (React 19 SPA)",
        "error": "build/ haipo — snapshot ni source tu (MBILINYI IDE ni React, si HTML iliyo-build).",
        "fix": "cd engines/mbilinyi && npm ci && npm run build, kisha restart server"
    }))
    .into_response()
}

/// Canvas ya IDE — `/ide` (na `/ide.html`, `/ide/`) inatolea MBILINYI Agent
/// Canvas (React) moja kwa moja. Build haipo (local dev fresh) → hint halisi
/// ya ku-build (hakuna restart ya server inahitajika).
async fn canvas_main() -> axum::response::Response {
    use axum::response::{Html, IntoResponse};
    if canvas_build_dir().is_some() {
        return canvas_ui().await;
    }
    let hint = "<!DOCTYPE html><html lang=\"sw\"><meta charset=\"utf-8\"><body style=\"background:#1a1f2b;color:#dce0e5;font:14px/1.6 system-ui;display:grid;place-items:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-size:18px\">MBILINYI Agent Canvas — build haipo</h1><p style=\"color:#a9afbc\">Endesha: <code>cd engines/mbilinyi &amp;&amp; VITE_BASE_PATH=/ide npm ci &amp;&amp; VITE_BASE_PATH=/ide npm run build</code> — kisha fungua /ide upya (hakuna restart ya server inahitajika).</p></div></body></html>";
    Html(hint).into_response()
}

/// Faili moja moja (assets) za OpenHands build/ — wildcard-salama (.. imepigiwa marufuku).
async fn canvas_asset(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if path.is_empty() || path.contains("..") {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    let rel = path.trim_start_matches('/');
    let ctype = match path.rsplit('.').next().unwrap_or("") {
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "html" => "text/html; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    match canvas_build_dir() {
        Some(dir) => match std::fs::read(dir.join(rel)) {
            Ok(bytes) => (
                axum::http::StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static(ctype),
                )],
                bytes,
            )
                .into_response(),
            // SPA fallback: a client route (no file extension) must resolve to
            // index.html so deep links like /ide/extensions/<name>/<page> work
            // on a hard reload instead of 404ing.
            Err(_) if !rel.contains('.') => canvas_ui().await,
            Err(_) => axum::http::StatusCode::NOT_FOUND.into_response(),
        },
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}

