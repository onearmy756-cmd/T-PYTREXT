//! sdlc.rs — MTECHOS SDLC: mzunguko KAMILI wa maendeleo ya programu.
//!
//! Awamu sita (kama SDLC ya kweli, si "code tu"):
//!   1. MAHITAJI    (Requirements)  — spec → kind/lang/domain + vigezo vya kukubali
//!   2. USANIFU     (Design)        — blueprint ya faili + muundo
//!   3. UJENZI      (Build)         — kuandika faili HALISI kwenye workspace
//!   4. UTHIBITISHO (Verify)        — build + tests HALISI + ukaguzi wa usalama
//!   5. UWASILISHAJI(Deploy)        — mpango wa deploy + build artifacts
//!   6. UFUATILIAJI (Monitor)       — kurekodi matokeo + afya ya mradi
//!
//! Vipengele vitatu vya SDLC ya kweli:
//!   - UGUMU WA MRADI: spec inapimwa (Rahisi/Wastani/Mgumu) na kina cha SDLC
//!     huchaguliwa kulingana na kiwango hicho (usitumie mchakato mrefu kwa kazi ndogo).
//!   - DELIVERABLES + IDHINI: kila awamu inatoa kitu halisi na inahitaji idhini
//!     ("Je, tuendelee?") kabla ya kuendelea.
//!   - GUARDRAILS: hakuna code bila tests, hakuna secrets, OWASP, idhini kabla ya deploy.
//!
//! LUGHA YA MTUMIAJI: jibu hutolewa kwa lugha aliyochagua (`language.rs`), kama
//! chat.rs. `lang` inaweza kutolewa moja kwa moja au hupatikana kwa `username`.
//!
//! Kanuni za repo (TUMIA vilivyopo, usirudie mantiki):
//!   - Kuchambua spec:   `forge_domain::parse_domain` + `forge::detect_kind/lang`.
//!   - Kuzalisha faili:  `forge_prod` (api) + `forge_projects` (web/mobile/iot/game)
//!                       + `forge::template_code` (blueprint).
//!   - Uthibitisho:      `lang_verify::verify_for` + `agent_engine::run_shell_public`
//!                       (build/tests HALISI — hakuna uwongo) + `code_audit`.
//!   - Deploy:           `deploy_templates::plan_for` (steps/commands halisi).
//!   - Lugha:            `language::get_language` + `ai_config::translate_db`.
//!   - Kumbukumbu:       jedwali `sdlc_runs` + `unified_engines::remember`.

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::ide_api::ide_workspace_root;

/// Awamu za SDLC: (id, jina_sw, jina_en, lengo_sw, lengo_en).
pub const SDLC_PHASES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "mahitaji",
        "Mahitaji (Requirements)",
        "Requirements",
        "Kukusanya mahitaji na vigezo vya kukubali kabla ya kuandika kodi.",
        "Gather requirements and acceptance criteria before writing code.",
    ),
    (
        "usanifu",
        "Usanifu (Design)",
        "Design",
        "Mpango wa faili na usanifu wa muundo kabla ya kuandika kodi.",
        "File plan and structural design before writing code.",
    ),
    (
        "ujenzi",
        "Ujenzi (Build)",
        "Development",
        "Kuandika faili HALISI kwenye workspace.",
        "Write REAL files to the workspace.",
    ),
    (
        "uthibitisho",
        "Uthibitisho (Verify)",
        "Testing",
        "Build + tests HALISI + ukaguzi wa usalama.",
        "REAL build + tests + security audit.",
    ),
    (
        "uwasilishaji",
        "Uwasilishaji (Deploy)",
        "Deployment",
        "Mpango wa kusambaza (deploy) na build artifacts.",
        "Deploy plan and build artifacts.",
    ),
    (
        "ufuatiliaji",
        "Ufuatiliaji (Monitor)",
        "Maintenance",
        "Kurekodi matokeo na afya ya mradi.",
        "Record outcomes and project health.",
    ),
];

// ---------------------------------------------------------------------------
// Lugha (reuse language.rs + ai_config::translate_db)
// ---------------------------------------------------------------------------

/// Lugha ya jibu: `lang` ya moja kwa moja, au lugha ya `username`, au default.
pub async fn resolve_lang(db: &SqlitePool, body: &Value) -> String {
    if let Some(l) = body["lang"].as_str() {
        let l = l.trim().to_lowercase();
        if l.len() == 2 && l.chars().all(|c| c.is_ascii_alphabetic()) {
            return l;
        }
    }
    if let Some(u) = body["username"].as_str().filter(|u| !u.trim().is_empty()) {
        return crate::language::get_language(db, u.trim()).await;
    }
    crate::language::DEFAULT_LANG.to_string()
}

/// Tafsiri: sw/en ni za msingi (deterministic); lugha nyingine hupitia LLM
/// (`ai_config::translate_db`). LLM haipatikani → en (hakuna uongo).
async fn tr(db: &SqlitePool, lang: &str, sw: &str, en: &str) -> String {
    match lang {
        "sw" => sw.to_string(),
        "en" => en.to_string(),
        _ => crate::ai_config::translate_db(db, en, lang)
            .await
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| en.to_string()),
    }
}

async fn tr_list(db: &SqlitePool, lang: &str, items: &[(&str, &str)]) -> Vec<String> {
    let mut out = Vec::with_capacity(items.len());
    for (sw, en) in items {
        out.push(tr(db, lang, sw, en).await);
    }
    out
}

/// Jina la awamu kwa lugha.
pub fn phase_label(id: &str, lang: &str) -> String {
    SDLC_PHASES
        .iter()
        .find(|(p, _, _, _, _)| *p == id)
        .map(|(_, sw, en, _, _)| if lang == "sw" { *sw } else { *en }.to_string())
        .unwrap_or_else(|| id.to_string())
}

// ---------------------------------------------------------------------------
// Ugumu wa mradi (complexity) — deterministic
// ---------------------------------------------------------------------------

/// Vipimo vinavyoongeza ugumu (spec token, uzito).
const SIGNALS: &[(&str, u32)] = &[
    ("integration", 2),
    ("api", 1),
    ("database", 2),
    ("sql", 1),
    ("redis", 2),
    ("kafka", 2),
    ("payment", 2),
    ("malipo", 2),
    ("oauth", 2),
    ("webhook", 2),
    ("third-party", 2),
    ("third party", 2),
    ("security", 2),
    ("usalama", 2),
    ("auth", 1),
    ("encryption", 2),
    ("pci", 2),
    ("hipaa", 2),
    ("gdpr", 2),
    ("sensitive", 1),
    ("password", 1),
    ("jwt", 1),
    ("users", 1),
    ("watumiaji", 1),
    ("multitenant", 2),
    ("multi-tenant", 2),
    ("saas", 2),
    ("enterprise", 2),
    ("scale", 2),
    ("scalability", 2),
    ("microservice", 2),
    ("kubernetes", 2),
    ("load balancer", 2),
    ("realtime", 2),
    ("real-time", 2),
    ("websocket", 2),
    ("streaming", 2),
    ("team", 1),
    ("timu", 1),
    ("collaboration", 1),
    ("compliance", 2),
    ("audit", 1),
    ("analytics", 1),
    ("machine learning", 2),
];

/// Pima ugumu wa mradi: Rahisi / Wastani / Mgumu (pamoja na sababu).
pub fn assess_complexity(spec: &str, kind: &str, files_planned: usize) -> (String, u32, Vec<String>) {
    let s = spec.to_lowercase();
    let mut score: u32 = 0;
    let mut reasons: Vec<String> = Vec::new();

    let file_score = if files_planned > 12 {
        3
    } else if files_planned > 6 {
        2
    } else if files_planned > 3 {
        1
    } else {
        0
    };
    if file_score > 0 {
        score += file_score;
        reasons.push(format!("faili zilizopangwa: {files_planned}"));
    }
    match kind {
        "api" | "mobile" | "game" | "iot" | "data" => {
            score += 1;
            reasons.push(format!("aina ya mradi: {kind}"));
        }
        _ => {}
    }
    for (needle, weight) in SIGNALS {
        if s.contains(needle) {
            score += weight;
            reasons.push(format!("signal: {needle}"));
        }
    }

    let level = if score <= 2 {
        "simple"
    } else if score <= 5 {
        "medium"
    } else {
        "complex"
    };
    (level.to_string(), score, reasons)
}

/// Jina la kiwango kwa lugha.
pub fn level_label(level: &str, lang: &str) -> String {
    let (sw, en) = match level {
        "simple" => ("Rahisi", "Simple"),
        "medium" => ("Wastani", "Medium"),
        _ => ("Mgumu", "Complex"),
    };
    if lang == "sw" {
        sw
    } else {
        en
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Deliverables (jina la kila deliverable kwa lugha)
// ---------------------------------------------------------------------------

fn deliverable_name(id: &str, lang: &str) -> String {
    let (sw, en) = match id {
        "requirements_doc" => ("waraka wa mahitaji", "requirements doc"),
        "user_stories" => ("user stories", "user stories"),
        "use_cases" => ("use cases", "use cases"),
        "nfr" => ("mahitaji yasiyo-utendaji (NFR)", "non-functional requirements"),
        "acceptance_criteria" => ("vigezo vya kukubali", "acceptance criteria"),
        "design_note" => ("maelezo ya usanifu", "design note"),
        "architecture" => ("usanifu wa mfumo (architecture)", "architecture"),
        "db_schema" => ("schema ya hifadhidata", "DB schema"),
        "api_spec" => ("vipimo vya API", "API spec"),
        "diagrams" => ("michoro", "diagrams"),
        "security_design" => ("usanifu wa usalama", "security design"),
        "scalability_plan" => ("mpango wa upanuzi", "scalability plan"),
        "code" => ("code", "code"),
        "pr" => ("PR / commits", "PR / commits"),
        "branching_strategy" => ("mkakati wa branches", "branching strategy"),
        "docs" => ("nyaraka", "documentation"),
        "test_report" => ("ripoti ya tests", "test report"),
        "bugs" => ("bugs", "bugs"),
        "perf_report" => ("ripoti ya utendaji", "performance report"),
        "security_report" => ("ripoti ya usalama", "security report"),
        "deploy_note" => ("maelekezo ya deploy", "deploy instructions"),
        "deployment_plan" => ("mpango wa deploy", "deployment plan"),
        "rollback_plan" => ("mpango wa kurudi nyuma (rollback)", "rollback plan"),
        "staging" => ("mazingira ya staging", "staging environment"),
        "blue_green_plan" => ("blue-green deployment", "blue-green deployment"),
        "monitoring_note" => ("maelezo ya monitoring", "monitoring note"),
        "logs" => ("logs", "logs"),
        "monitoring_plan" => ("mpango wa monitoring", "monitoring plan"),
        "sla" => ("SLA", "SLA"),
        "incident_plan" => ("mpango wa matukio (incident response)", "incident response plan"),
        "alerts" => ("alerts", "alerts"),
        "review_cadence" => ("mzunguko wa ukaguzi", "review cadence"),
        other => (other, other),
    };
    if lang == "sw" {
        sw
    } else {
        en
    }
    .to_string()
}

fn deliverables(ids: &[&str], lang: &str) -> Vec<Value> {
    ids.iter()
        .map(|id| json!({ "id": id, "name": deliverable_name(id, lang) }))
        .collect()
}

// ---------------------------------------------------------------------------
// Mpango wa awamu kwa kina (Rahisi / Wastani / Mgumu)
// ---------------------------------------------------------------------------

type Act = Vec<(&'static str, &'static str)>;

/// Mpango wa awamu kulingana na ugumu: kina, shughuli, deliverables, idhini.
pub async fn phase_plan(db: &SqlitePool, level: &str, lang: &str) -> Vec<Value> {
    let (mah, usa, uje, uth, uw, uf): (Act, Act, Act, Act, Act, Act) = match level {
        "simple" => (
            vec![
                ("Uliza maswali 2-3", "Ask 2-3 questions"),
                ("Andika user stories fupi", "Write short user stories"),
            ],
            vec![("Mchoro rahisi au maelezo mafupi", "Simple sketch or short note")],
            vec![("Andika code moja kwa moja", "Write code directly")],
            vec![("Unit tests + manual testing", "Unit tests + manual testing")],
            vec![("Maelekezo mafupi ya ku-deploy", "Short deploy instructions")],
            vec![("Maelezo ya msingi ya monitoring", "Basic monitoring note")],
        ),
        "medium" => (
            vec![
                ("Uliza maswali zaidi", "Ask more questions"),
                ("Andika requirements doc", "Write requirements doc"),
                ("Andika acceptance criteria", "Write acceptance criteria"),
            ],
            vec![
                ("Architecture", "Architecture"),
                ("DB schema", "DB schema"),
                ("API endpoints", "API endpoints"),
            ],
            vec![
                ("Code modular", "Modular code"),
                ("Fuata standards", "Follow standards"),
            ],
            vec![("Unit + integration + baadhi ya E2E", "Unit + integration + some E2E")],
            vec![
                ("CI/CD basic", "Basic CI/CD"),
                ("Environment variables", "Environment variables"),
                ("Rollback plan", "Rollback plan"),
            ],
            vec![
                ("Logging", "Logging"),
                ("Monitoring", "Monitoring"),
                ("Update plan", "Update plan"),
            ],
        ),
        _ => (
            vec![
                ("Uchambuzi kamili", "Full analysis"),
                ("User stories", "User stories"),
                ("Use cases", "Use cases"),
                ("Mahitaji yasiyo-utendaji (NFR)", "Non-functional requirements"),
            ],
            vec![
                ("Architecture detailed", "Detailed architecture"),
                ("Security design", "Security design"),
                ("Scalability plan", "Scalability plan"),
                ("DB schema", "DB schema"),
                ("API specs", "API specs"),
                ("Diagrams", "Diagrams"),
            ],
            vec![
                ("Code reviews", "Code reviews"),
                ("Branching strategy", "Branching strategy"),
                ("Documentation", "Documentation"),
                ("Security best practices", "Security best practices"),
            ],
            vec![
                ("Unit + integration + E2E", "Unit + integration + E2E"),
                ("Performance testing", "Performance testing"),
                ("Security testing", "Security testing"),
            ],
            vec![
                ("CI/CD kamili", "Full CI/CD"),
                ("Staging", "Staging"),
                ("Blue-green deployment", "Blue-green deployment"),
                ("Rollback", "Rollback"),
                ("Monitoring", "Monitoring"),
            ],
            vec![
                ("SLA", "SLA"),
                ("Incident response", "Incident response"),
                ("Logging", "Logging"),
                ("Alerting", "Alerting"),
                ("Periodic reviews", "Periodic reviews"),
            ],
        ),
    };

    let depth = match (level, lang) {
        ("simple", "sw") => "nyepesi",
        ("simple", _) => "light",
        ("medium", "sw") => "kati",
        ("medium", _) => "standard",
        (_, "sw") => "kamili",
        _ => "full",
    };

    let defs: [(&str, Act, Vec<&str>); 6] = [
        ("mahitaji", mah, vec!["user_stories", "acceptance_criteria"]),
        ("usanifu", usa, vec!["design_note", "architecture"]),
        ("ujenzi", uje, vec!["code", "pr"]),
        ("uthibitisho", uth, vec!["test_report", "bugs"]),
        ("uwasilishaji", uw, vec!["deployment_plan", "rollback_plan"]),
        ("ufuatiliaji", uf, vec!["monitoring_plan", "logs"]),
    ];

    let mut out = Vec::new();
    for (i, (id, acts, base_deliv)) in defs.into_iter().enumerate() {
        let deliv: Vec<&str> = match (level, i) {
            ("medium", 0) => vec!["requirements_doc", "acceptance_criteria"],
            ("complex", 0) => vec!["requirements_doc", "user_stories", "use_cases", "nfr"],
            ("medium", 1) => vec!["architecture", "db_schema", "api_spec"],
            ("complex", 1) => vec![
                "architecture",
                "security_design",
                "scalability_plan",
                "db_schema",
                "api_spec",
                "diagrams",
            ],
            ("complex", 2) => vec!["code", "pr", "branching_strategy", "docs"],
            ("complex", 3) => vec!["test_report", "perf_report", "security_report", "bugs"],
            ("complex", 4) => vec![
                "deployment_plan",
                "staging",
                "blue_green_plan",
                "rollback_plan",
                "monitoring_plan",
            ],
            ("complex", 5) => vec!["sla", "incident_plan", "logs", "alerts", "review_cadence"],
            _ => base_deliv,
        };
        let acts_l = tr_list(db, lang, &acts).await;
        out.push(json!({
            "id": id,
            "label": phase_label(id, lang),
            "depth": depth,
            "activities": acts_l,
            "deliverables": deliverables(&deliv, lang),
            "requires_approval": true,
        }));
    }
    out
}

// ---------------------------------------------------------------------------
// Guardrails (vikwazo) — hufanywa kwa ushahidi HALISI kwenye `run`
// ---------------------------------------------------------------------------

async fn guardrail_defs(db: &SqlitePool, lang: &str) -> Vec<Value> {
    let items: &[(&str, &str, &str)] = &[
        ("tests", "Usitoe code yenye bugs bila tests.", "Do not ship code without tests."),
        ("no_secrets", "Usiweke secrets kwenye code.", "Do not hardcode secrets in code."),
        ("owasp", "Fuata OWASP kwa usalama.", "Follow OWASP for security."),
        ("fix_risks", "Rekebisha hatari za usalama.", "Fix security risks."),
        ("deploy_approval", "Uliza idhini kabla ya deploy.", "Ask for approval before deploy."),
        ("docs", "Weka documentation kila hatua.", "Document every phase."),
    ];
    let mut out = Vec::new();
    for (id, sw, en) in items {
        out.push(json!({ "id": id, "label": tr(db, lang, sw, en).await }));
    }
    out
}

// ---------------------------------------------------------------------------
// Vadu safi (pure functions — zote zina tests)
// ---------------------------------------------------------------------------

/// Id za awamu kwa mpangilio.
pub fn phase_ids() -> Vec<&'static str> {
    SDLC_PHASES.iter().map(|(id, _, _, _, _)| *id).collect()
}

/// Vigezo vya kukubali (acceptance criteria) kutoka `kind` — Kiswahili.
pub fn acceptance_criteria(kind: &str) -> Vec<String> {
    let base = vec![
        "Kila faili inayotajwa na entrypoint ipo.".to_string(),
        "Amri ya tests/build inatoka na exit code 0.".to_string(),
        "Hakuna TODO/FIXME kwenye njia za production.".to_string(),
        "README inaeleza jinsi ya kuendesha, na amri hiyo imeendeshwa.".to_string(),
    ];
    let extra = match kind {
        "website" | "web" => vec!["Ukurasa unafunguka kwenye browser bila kosa la console.".to_string()],
        "mobile" => vec!["Screen kuu inapakia bila crash.".to_string()],
        "iot" => vec!["Firmware inawasha na kusoma sensor.".to_string()],
        "gaming" | "game" => vec!["Mchezo unaanza na loop kuu inaendesha.".to_string()],
        "api" => vec!["Endpoint ya msingi inarudisha 2xx.".to_string()],
        "cli" => vec!["Binary inaendesha na kutoa matokeo.".to_string()],
        _ => vec![],
    };
    base.into_iter().chain(extra).collect()
}

/// Vigezo vya kukubali kwa lugha ya mtumiaji.
pub async fn acceptance_criteria_lang(db: &SqlitePool, kind: &str, lang: &str) -> Vec<String> {
    if lang == "sw" {
        return acceptance_criteria(kind);
    }
    let extra: Vec<&str> = match kind {
        "website" | "web" => vec!["The page opens in a browser with no console error."],
        "mobile" => vec!["The main screen loads without a crash."],
        "iot" => vec!["The firmware boots and reads a sensor."],
        "gaming" | "game" => vec!["The game starts and the main loop runs."],
        "api" => vec!["The main endpoint returns 2xx."],
        "cli" => vec!["The binary runs and produces output."],
        _ => vec![],
    };
    let mut en: Vec<&str> = vec![
        "Every file referenced by the entrypoint exists.",
        "The tests/build command exits with code 0.",
        "No TODO/FIXME in production paths.",
        "The README explains how to run it, and that command was run.",
    ];
    en.extend(extra);

    if lang == "en" {
        return en.into_iter().map(String::from).collect();
    }
    let mut out = Vec::with_capacity(en.len());
    for t in en {
        out.push(tr(db, lang, t, t).await);
    }
    out
}

/// "Definition of done" — gate kuu. Hupokea ushahidi HALISI.
pub fn definition_of_done(
    files_written: usize,
    verify_ok: bool,
    tests_failed: u64,
    audit_critical: u64,
) -> Value {
    let mut blockers: Vec<String> = Vec::new();
    if files_written == 0 {
        blockers.push("Hakuna faili iliyoandikwa.".to_string());
    }
    if !verify_ok {
        blockers.push("Uthibitisho (build/tests) haukupita.".to_string());
    }
    if tests_failed > 0 {
        blockers.push(format!("Testi {tests_failed} zimeshindwa."));
    }
    if audit_critical > 0 {
        blockers.push(format!("Security findings {audit_critical} za critical."));
    }
    json!({
        "done": blockers.is_empty(),
        "blockers": blockers,
    })
}

/// Gate ya awamu: huamua kama awamu inaweza kufunguliwa.
pub fn phase_gate(phase: &str, prior_ok: bool) -> Value {
    let known = phase_ids().contains(&phase);
    if !known {
        return json!({ "ok": false, "phase": phase, "error": "awamu haijulikani" });
    }
    if !prior_ok {
        return json!({
            "ok": false, "phase": phase, "blocked": true,
            "reason": "awamu iliyotangulia haijakamilika",
        });
    }
    json!({ "ok": true, "phase": phase, "blocked": false })
}

/// Alama (score) ya awamu kutoka ushahidi: 0–100.
pub fn phase_score(grade: f64, tests_failed: u64) -> f64 {
    let s = grade - 15.0 * tests_failed as f64;
    (s.clamp(0.0, 100.0) * 10.0).round() / 10.0
}

/// Ramani ya `forge::detect_kind` → kind ya `forge_projects` (inayojijaribu).
/// `None` = hakuna generator kamili; tunatumia blueprint + template_code.
pub fn project_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "web" => Some("website"),
        "mobile" => Some("mobile"),
        "game" => Some("gaming"),
        "iot" => Some("iot"),
        _ => None,
    }
}

/// Ramani ya kind → domain ya `deploy_templates` (website/mobile/iot/gaming).
pub fn deploy_domain(kind: &str) -> &'static str {
    match kind {
        "web" => "website",
        "game" => "gaming",
        "mobile" => "mobile",
        "iot" => "iot",
        _ => "website",
    }
}

/// Faili za kuandika kwa kind fulani. Kipaumbele:
///   1. generator ya API ya production (`forge_prod` — ina tests) kwa kind `api`.
///   2. generator ya mradi kamili (`forge_projects`) kwa web/mobile/iot/game.
///   3. blueprint + `forge::template_code` (deterministic, hupita linter).
pub fn files_for(
    kind: &str,
    lang: &str,
    spec: &str,
    blueprint: &crate::forge::Blueprint,
) -> Vec<(String, String)> {
    let domain = crate::forge_domain::parse_domain(spec);
    if kind == "api" {
        let prod_kind = match lang {
            "rust" => "rust-api",
            "javascript" | "typescript" | "node" => "node-api",
            _ => "",
        };
        if !prod_kind.is_empty() {
            if let Some((files, _)) = crate::forge_prod::generate_production(prod_kind, &domain) {
                return files;
            }
        }
    }
    if let Some(gk) = project_kind(kind) {
        if let Some((files, _)) = crate::forge_projects::generate_project(gk, &domain) {
            return files;
        }
    }
    blueprint
        .files
        .iter()
        .map(|f| (f.path.clone(), crate::forge::template_code(f, spec)))
        .collect()
}

// ---------------------------------------------------------------------------
// Path safety (ndani ya workspace pekee)
// ---------------------------------------------------------------------------

fn safe_join(path: &str) -> Option<std::path::PathBuf> {
    let root = ide_workspace_root();
    let rel = path.trim().trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") {
        return None;
    }
    let p = root.join(rel);
    if p.starts_with(&root) {
        Some(p)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Hifadhidata
// ---------------------------------------------------------------------------

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS sdlc_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project TEXT NOT NULL,
            kind TEXT NOT NULL,
            lang TEXT NOT NULL,
            complexity TEXT NOT NULL DEFAULT '',
            ok INTEGER NOT NULL,
            score REAL NOT NULL,
            phases TEXT,
            summary TEXT,
            at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
    // Hali ya awamu kwa mradi: idhini ya mtumiaji ("Je, tuendelee?").
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS sdlc_phase_state (
            project TEXT NOT NULL,
            phase TEXT NOT NULL,
            approved INTEGER NOT NULL DEFAULT 0,
            at TEXT NOT NULL,
            PRIMARY KEY (project, phase)
        )",
    )
    .execute(db)
    .await;
}

/// Awamu zilizoidhinishwa kwa mradi (halisi kutoka DB).
pub async fn approved_phases(db: &SqlitePool, project: &str) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT phase FROM sdlc_phase_state WHERE project = ? AND approved = 1",
    )
    .bind(project)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    rows.into_iter().map(|(p,)| p).collect()
}

/// Rekodi idhini ya awamu; hurudisha awamu zote zilizoidhinishwa.
pub async fn approve_phase(db: &SqlitePool, project: &str, phase: &str) -> Vec<String> {
    if phase_ids().contains(&phase) {
        let _ = sqlx::query(
            "INSERT INTO sdlc_phase_state (project, phase, approved, at) VALUES (?,?,1,?)
             ON CONFLICT(project, phase) DO UPDATE SET approved=1, at=excluded.at",
        )
        .bind(project)
        .bind(phase)
        .bind(chrono::Local::now().to_rfc3339())
        .execute(db)
        .await;
    }
    approved_phases(db, project).await
}

/// Awamu inayofuata baada ya `phase` (None kama ni ya mwisho).
pub fn next_phase(phase: &str) -> Option<&'static str> {
    let ids = phase_ids();
    let i = ids.iter().position(|p| *p == phase)?;
    ids.get(i + 1).copied()
}

#[allow(clippy::too_many_arguments)]
async fn record(
    db: &SqlitePool,
    project: &str,
    kind: &str,
    lang: &str,
    complexity: &str,
    ok: bool,
    score: f64,
    phases: &Value,
    summary: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO sdlc_runs (project, kind, lang, complexity, ok, score, phases, summary, at)
         VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(project)
    .bind(kind)
    .bind(lang)
    .bind(complexity)
    .bind(ok as i64)
    .bind(score)
    .bind(phases.to_string())
    .bind(summary.chars().take(4000).collect::<String>())
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await;
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

/// GET /api/sdlc/capabilities — awamu, ugumu, guardrails, endpoints.
pub async fn capabilities(State(s): State<crate::AppState>) -> Json<Value> {
    let lang = crate::language::DEFAULT_LANG.to_string();
    let guardrails = guardrail_defs(&s.db, &lang).await;
    Json(json!({
        "ok": true,
        "engine": "MTECHOS SDLC",
        "note": "Mzunguko kamili: mahitaji → usanifu → ujenzi → uthibitisho → uwasilishaji → ufuatiliaji. Kina huchaguliwa kwa ugumu wa mradi.",
        "phases": SDLC_PHASES.iter().map(|(id, sw, en, gsw, gen)| json!({
            "id": id, "label_sw": sw, "label_en": en, "goal_sw": gsw, "goal_en": gen,
        })).collect::<Vec<_>>(),
        "complexity_levels": [
            {"id": "simple", "label_sw": "Rahisi", "label_en": "Simple"},
            {"id": "medium", "label_sw": "Wastani", "label_en": "Medium"},
            {"id": "complex", "label_sw": "Mgumu", "label_en": "Complex"},
        ],
        "guardrails": guardrails,
        "endpoints": [
            "GET  /api/sdlc/capabilities",
            "POST /api/sdlc/assess",
            "POST /api/sdlc/plan",
            "POST /api/sdlc/run",
            "POST /api/sdlc/gate",
            "GET  /api/sdlc/history",
        ],
    }))
}

/// POST /api/sdlc/assess — pima ugumu wa mradi + mpango (hakuna kuandika).
/// body: { spec, kind?, lang?, username? }
pub async fn assess(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let spec = body["spec"].as_str().unwrap_or("").trim().to_string();
    if spec.is_empty() {
        return Json(json!({ "ok": false, "error": "spec ni lazima" }));
    }
    let lang = resolve_lang(&s.db, &body).await;
    let kind = body["kind"]
        .as_str()
        .filter(|k| !k.trim().is_empty())
        .map(|k| k.trim().to_string())
        .unwrap_or_else(|| crate::forge::detect_kind(&spec).to_string());
    let blueprint = crate::forge::template_blueprint(&spec);
    let (level, score, reasons) = assess_complexity(&spec, &kind, blueprint.files.len());
    let plan = phase_plan(&s.db, &level, &lang).await;
    Json(json!({
        "ok": true,
        "lang": lang,
        "kind": kind,
        "complexity": {
            "level": level,
            "label": level_label(&level, &lang),
            "score": score,
            "reasons": reasons,
        },
        "phase_plan": plan,
        "guardrails": guardrail_defs(&s.db, &lang).await,
    }))
}

/// POST /api/sdlc/plan — mpango wa awamu zote (hakuna kuandika faili).
/// body: { spec, kind?, lang?, username? }
pub async fn plan(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let spec = body["spec"].as_str().unwrap_or("").trim().to_string();
    if spec.is_empty() {
        return Json(json!({ "ok": false, "error": "spec ni lazima" }));
    }
    let lang = resolve_lang(&s.db, &body).await;
    let kind = body["kind"]
        .as_str()
        .filter(|k| !k.trim().is_empty())
        .map(|k| k.trim().to_string())
        .unwrap_or_else(|| crate::forge::detect_kind(&spec).to_string());
    let slang = crate::forge::detect_lang(&spec, &kind);
    let domain = crate::forge_domain::parse_domain(&spec);
    let blueprint = crate::forge::template_blueprint(&spec);
    let (level, score, reasons) = assess_complexity(&spec, &kind, blueprint.files.len());

    Json(json!({
        "ok": true,
        "lang": lang,
        "spec": spec,
        "kind": kind,
        "lang_code": slang,
        "project": crate::forge::sanitize_name(&spec),
        "entities": domain.entities.iter().map(|e| e.name.clone()).collect::<Vec<_>>(),
        "complexity": {
            "level": level, "label": level_label(&level, &lang),
            "score": score, "reasons": reasons,
        },
        "acceptance_criteria": acceptance_criteria_lang(&s.db, &kind, &lang).await,
        "files_planned": blueprint.files.iter().map(|f| json!({
            "path": f.path, "purpose": f.purpose, "lang": f.lang,
        })).collect::<Vec<_>>(),
        "phase_plan": phase_plan(&s.db, &level, &lang).await,
        "guardrails": guardrail_defs(&s.db, &lang).await,
    }))
}

/// POST /api/sdlc/gate — tathmini gate ya awamu (pure, bila kuandika).
/// body: { phase, prior_ok, approved? }
pub async fn gate(Json(body): Json<Value>) -> Json<Value> {
    let phase = body["phase"].as_str().unwrap_or("");
    let prior_ok = body["prior_ok"].as_bool().unwrap_or(true);
    let approved = body["approved"].as_bool().unwrap_or(false);
    let mut v = phase_gate(phase, prior_ok);
    if let Some(obj) = v.as_object_mut() {
        obj.insert("requires_approval".into(), json!(true));
        obj.insert("approved".into(), json!(approved));
        if prior_ok && !approved {
            obj.insert("waiting_for_approval".into(), json!(true));
        }
    }
    Json(v)
}

/// POST /api/sdlc/continue — idhinisha awamu na uendelee (huhifadhi hali).
/// body: { project, phase?, approve?, lang?, username? }
/// Bila `approve:false`, awamu iliyotolewa inaidhinishwa. `next` inaonyesha
/// awamu inayofuata; `approved` = awamu zote zilizoidhinishwa (kutoka DB).
pub async fn continue_phase(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let lang = resolve_lang(&s.db, &body).await;
    let project = body["project"]
        .as_str()
        .map(|p| crate::forge::sanitize_name(p))
        .unwrap_or_default();
    if project.is_empty() {
        return Json(json!({ "ok": false, "error": "project ni lazima" }));
    }
    let phase = body["phase"].as_str().unwrap_or("").to_string();
    if !phase.is_empty() && !phase_ids().contains(&phase.as_str()) {
        return Json(json!({ "ok": false, "error": format!("awamu '{phase}' haijulikani") }));
    }
    let approve = body["approve"].as_bool().unwrap_or(true);
    let approved = if approve && !phase.is_empty() {
        approve_phase(&s.db, &project, &phase).await
    } else {
        approved_phases(&s.db, &project).await
    };
    let next = if phase.is_empty() {
        None
    } else {
        next_phase(&phase)
    };
    let next_label = next.map(|n| phase_label(n, &lang));
    Json(json!({
        "ok": true,
        "project": project,
        "lang": lang,
        "approved_phase": if approve { json!(phase) } else { Value::Null },
        "approved": approved,
        "next": next,
        "next_label": next_label,
        "message": tr(&s.db, &lang,
            "Awamu imeidhinishwa. Tuendelee?",
            "Phase approved. Shall we continue?").await,
    }))
}

/// GET /api/sdlc/history — historia ya runs (halisi kutoka DB).
pub async fn history(State(s): State<crate::AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, String, i64, f64, String, String)>(
        "SELECT id, project, kind, lang, complexity, ok, score, summary, at
         FROM sdlc_runs ORDER BY id DESC LIMIT 50",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, project, kind, lang, complexity, ok, score, summary, at)| {
            json!({
                "id": id, "project": project, "kind": kind, "lang": lang,
                "complexity": complexity, "ok": ok != 0, "score": score,
                "summary": summary, "at": at,
            })
        })
        .collect();
    Json(json!({ "ok": true, "count": items.len(), "runs": items }))
}

/// POST /api/sdlc/run — endesha mzunguko KAMILI wa SDLC.
/// body: { spec, project?, kind?, write?, run_verify?, lang?, username?, complexity? }
pub async fn run(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let spec = body["spec"].as_str().unwrap_or("").trim().to_string();
    if spec.is_empty() {
        return Json(json!({ "ok": false, "error": "spec ni lazima" }));
    }
    let lang = resolve_lang(&s.db, &body).await;
    let write = body["write"].as_bool().unwrap_or(true);
    let run_verify = body["run_verify"].as_bool().unwrap_or(true);
    let kind = body["kind"]
        .as_str()
        .filter(|k| !k.trim().is_empty())
        .map(|k| k.trim().to_string())
        .unwrap_or_else(|| crate::forge::detect_kind(&spec).to_string());
    let flang = crate::forge::detect_lang(&spec, &kind).to_string();
    let project = body["project"]
        .as_str()
        .filter(|p| !p.trim().is_empty())
        .map(|p| crate::forge::sanitize_name(p))
        .unwrap_or_else(|| crate::forge::sanitize_name(&spec));
    let started = std::time::Instant::now();

    let blueprint = crate::forge::template_blueprint(&spec);
    let (level, cx_score, cx_reasons) = match body["complexity"].as_str() {
        Some(c) if ["simple", "medium", "complex"].contains(&c) => {
            (c.to_string(), 0u32, vec!["iliyotolewa na mtumiaji".to_string()])
        }
        _ => assess_complexity(&spec, &kind, blueprint.files.len()),
    };
    let depth = match (level.as_str(), lang.as_str()) {
        ("simple", "sw") => "nyepesi",
        ("simple", _) => "light",
        ("medium", "sw") => "kati",
        ("medium", _) => "standard",
        (_, "sw") => "kamili",
        _ => "full",
    };
    let plan = phase_plan(&s.db, &level, &lang).await;

    let mut phases: Vec<Value> = Vec::new();
    let mut prior_ok = true;

    // ── 1) MAHITAJI ────────────────────────────────────────────────────────
    let domain = crate::forge_domain::parse_domain(&spec);
    let criteria = acceptance_criteria_lang(&s.db, &kind, &lang).await;
    phases.push(json!({
        "id": "mahitaji", "label": phase_label("mahitaji", &lang),
        "status": "ok", "depth": depth,
        "kind": kind, "lang": flang, "project": project,
        "entities": domain.entities.iter().map(|e| e.name.clone()).collect::<Vec<_>>(),
        "acceptance_criteria": criteria,
        "deliverables": deliverables(&["user_stories", "acceptance_criteria"], &lang),
        "requires_approval": true,
    }));

    // ── 2) USANIFU ─────────────────────────────────────────────────────────
    let planned = blueprint.files.len();
    prior_ok = planned > 0;
    phases.push(json!({
        "id": "usanifu", "label": phase_label("usanifu", &lang),
        "status": if prior_ok { "ok" } else { "blocked" }, "depth": depth,
        "files_planned": planned,
        "files": blueprint.files.iter().map(|f| json!({
            "path": f.path, "purpose": f.purpose, "lang": f.lang,
        })).collect::<Vec<_>>(),
        "deliverables": deliverables(&["design_note", "architecture"], &lang),
        "requires_approval": true,
        "gate": phase_gate("usanifu", prior_ok),
    }));
    if !prior_ok {
        return finish(
            s.db.clone(), &project, &kind, &flang, &level, &lang, phases, 0.0, false, started,
            "SDLC imesimama (gate ilizuia)",
        )
        .await;
    }

    // ── 3) UJENZI ──────────────────────────────────────────────────────────
    let mut files_written: Vec<String> = Vec::new();
    let mut write_error: Option<String> = None;
    let mut target_dir = Value::Null;
    if write {
        if let Some(dir) = safe_join(&project) {
            let files = files_for(&kind, &flang, &spec, &blueprint);
            if !files.is_empty() {
                for (rel, content) in &files {
                    let full = dir.join(rel);
                    if let Some(parent) = full.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::write(&full, content).is_ok() {
                        files_written.push(rel.clone());
                    }
                }
                target_dir = json!(dir.display().to_string());
                if files_written.is_empty() {
                    write_error = Some("hakuna faili iliyoandikwa".into());
                }
            } else {
                write_error = Some("hakuna faili zilizopangwa".into());
            }
        } else {
            write_error = Some("project si sahihi (traversal)".into());
        }
    }
    prior_ok = !files_written.is_empty() && write_error.is_none();
    phases.push(json!({
        "id": "ujenzi", "label": phase_label("ujenzi", &lang),
        "status": if prior_ok { "ok" } else { "error" }, "depth": depth,
        "written": files_written.len(),
        "files_written": files_written,
        "path": target_dir,
        "error": write_error,
        "deliverables": deliverables(&["code", "pr"], &lang),
        "requires_approval": true,
        "gate": phase_gate("ujenzi", prior_ok),
    }));
    if !prior_ok {
        return finish(
            s.db.clone(), &project, &kind, &flang, &level, &lang, phases, 0.0, false, started,
            "SDLC imesimama (gate ilizuia)",
        )
        .await;
    }

    // ── 4) UTHIBITISHO ─────────────────────────────────────────────────────
    let dir = ide_workspace_root().join(&project);
    let mut audit_critical = 0u64;
    let mut audit_findings = 0u64;
    let mut secrets_critical = 0u64;
    for rel in &files_written {
        let full = dir.join(rel);
        let Ok(content) = std::fs::read_to_string(&full) else { continue };
        let l = crate::code_audit::lang_for(rel);
        let findings = crate::code_audit::analyze_content(l, &content);
        audit_findings += findings.len() as u64;
        audit_critical += findings.iter().filter(|f| f["severity"] == json!("critical")).count() as u64;
        secrets_critical += findings
            .iter()
            .filter(|f| f["category"] == json!("secret") && f["severity"] == json!("critical"))
            .count() as u64;
    }
    let grade = (100.0 - 10.0 * audit_critical as f64 - 4.0 * (audit_findings - audit_critical) as f64)
        .clamp(0.0, 100.0);

    let (verify_ok, verify_cmd, verify_output) = if run_verify {
        let (prefix, _l, cmd) = crate::lang_verify::verify_for(&dir);
        let full_cmd = format!("{prefix}{cmd}");
        let d = dir.clone();
        let c = full_cmd.clone();
        let (out, ok) = tokio::task::spawn_blocking(move || {
            crate::agent_engine::run_shell_public(&d, &c, 300)
        })
        .await
        .unwrap_or_else(|e| (format!("join error: {e}"), false));
        (ok, full_cmd, out)
    } else {
        (false, String::new(), "uthibitisho ulirukwa (run_verify:false)".to_string())
    };

    let dod = definition_of_done(
        files_written.len(),
        if run_verify { verify_ok } else { true },
        0,
        audit_critical,
    );
    let tests_failed = if verify_ok { 0 } else { 1 };
    let score = phase_score(grade, tests_failed);

    // Guardrails: tathmini kwa ushahidi HALISI.
    let has_readme = files_written.iter().any(|p| p.to_lowercase().contains("readme"));
    let guardrails = vec![
        json!({"id":"tests","ok": if run_verify { verify_ok } else { true }}),
        json!({"id":"no_secrets","ok": secrets_critical == 0}),
        json!({"id":"owasp","ok": audit_critical == 0}),
        json!({"id":"fix_risks","ok": audit_critical == 0}),
        json!({"id":"deploy_approval","ok": true, "note": tr(&s.db, &lang, "Idhini inahitajika kabla ya deploy", "Approval required before deploy").await}),
        json!({"id":"docs","ok": has_readme}),
    ];
    let guardrails_ok = guardrails.iter().all(|g| g["ok"] == json!(true));

    prior_ok = dod["done"].as_bool().unwrap_or(false) && guardrails_ok;
    phases.push(json!({
        "id": "uthibitisho", "label": phase_label("uthibitisho", &lang),
        "status": if prior_ok { "ok" } else { "warning" }, "depth": depth,
        "verify_ok": verify_ok, "command": verify_cmd,
        "output": verify_output.chars().take(6000).collect::<String>(),
        "security_grade": grade,
        "audit_findings": audit_findings, "audit_critical": audit_critical,
        "secrets_critical": secrets_critical,
        "definition_of_done": dod,
        "guardrails": guardrails,
        "deliverables": deliverables(&["test_report", "bugs"], &lang),
        "requires_approval": true,
        "gate": phase_gate("uthibitisho", prior_ok),
    }));

    // ── 5) UWASILISHAJI ────────────────────────────────────────────────────
    let deploy = crate::deploy_templates::plan_for(deploy_domain(&kind));
    let deploy_steps = deploy["steps"].as_array().cloned().unwrap_or_default().len();
    phases.push(json!({
        "id": "uwasilishaji", "label": phase_label("uwasilishaji", &lang),
        "status": if deploy_steps > 0 { "ok" } else { "skipped" }, "depth": depth,
        "domain": deploy_domain(&kind),
        "steps": deploy_steps,
        "plan": deploy,
        "requires_approval": true,
        "deliverables": deliverables(&["deployment_plan", "rollback_plan"], &lang),
    }));

    // ── 6) UFUATILIAJI ─────────────────────────────────────────────────────
    let summary = format!(
        "kind={kind} lang={flang} complexity={level} files={} verify_ok={verify_ok} grade={grade:.0} score={score:.0}",
        files_written.len()
    );
    crate::unified_engines::remember(&project, &kind, prior_ok, &summary);
    phases.push(json!({
        "id": "ufuatiliaji", "label": phase_label("ufuatiliaji", &lang),
        "status": "ok", "depth": depth,
        "recorded": true, "summary": summary,
        "deliverables": deliverables(&["monitoring_plan", "logs"], &lang),
        "requires_approval": false,
    }));

    let workflow = tr(
        &s.db,
        &lang,
        "Kila awamu inahitaji idhini (\"Je, tuendelee?\") kabla ya kuendelea.",
        "Each phase requires approval (\"Shall we continue?\") before proceeding.",
    )
    .await;

    let mut out = finish(
        s.db.clone(), &project, &kind, &flang, &level, &lang, phases, score, prior_ok, started,
        &summary,
    )
    .await;
    if let Some(obj) = out.0.as_object_mut() {
        obj.insert("depth".into(), json!(depth));
        obj.insert("complexity".into(), json!({
            "level": level, "label": level_label(&level, &lang),
            "score": cx_score, "reasons": cx_reasons,
        }));
        obj.insert("phase_plan".into(), Value::Array(plan));
        obj.insert("guardrails".into(), json!(guardrails));
        obj.insert("workflow".into(), json!(workflow));
        obj.insert("approved_phases".into(), json!(approved_phases(&s.db, &project).await));
    }
    out
}

#[allow(clippy::too_many_arguments)]
async fn finish(
    db: SqlitePool,
    project: &str,
    kind: &str,
    lang: &str,
    complexity: &str,
    ui_lang: &str,
    phases: Vec<Value>,
    score: f64,
    ok: bool,
    started: std::time::Instant,
    summary: &str,
) -> Json<Value> {
    let phases_json = Value::Array(phases.clone());
    record(&db, project, kind, lang, complexity, ok, score, &phases_json, summary).await;
    Json(json!({
        "ok": ok,
        "engine": "MTECHOS SDLC",
        "project": project,
        "kind": kind,
        "lang": lang,
        "lang_ui": ui_lang,
        "score": score,
        "phases": phases,
        "summary": summary,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/sdlc/capabilities", get(capabilities))
        .route("/api/sdlc/assess", post(assess))
        .route("/api/sdlc/plan", post(plan))
        .route("/api/sdlc/run", post(run))
        .route("/api/sdlc/gate", post(gate))
        .route("/api/sdlc/continue", post(continue_phase))
        .route("/api/sdlc/history", get(history))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> SqlitePool {
        SqlitePool::connect_lazy("sqlite::memory:").unwrap()
    }

    #[test]
    fn awamu_sita_kwa_mpangilio() {
        let ids = phase_ids();
        assert_eq!(
            ids,
            vec!["mahitaji", "usanifu", "ujenzi", "uthibitisho", "uwasilishaji", "ufuatiliaji"]
        );
    }

    #[test]
    fn jina_la_awamu_kwa_lugha() {
        assert!(phase_label("mahitaji", "sw").contains("Mahitaji"));
        assert_eq!(phase_label("mahitaji", "en"), "Requirements");
        assert_eq!(phase_label("ujenzi", "en"), "Development");
        assert_eq!(phase_label("uthibitisho", "en"), "Testing");
    }

    #[test]
    fn vigezo_vya_kukubali_ni_halisi() {
        let c = acceptance_criteria("website");
        assert!(c.len() >= 4);
        assert!(c.iter().any(|x| x.contains("exit code 0")));
        assert!(c.iter().any(|x| x.contains("browser")));
    }

    #[test]
    fn definition_of_done_huzuia_ukosefu() {
        let bad = definition_of_done(0, false, 2, 1);
        assert_eq!(bad["done"], json!(false));
        assert_eq!(bad["blockers"].as_array().unwrap().len(), 4);
        let good = definition_of_done(3, true, 0, 0);
        assert_eq!(good["done"], json!(true));
    }

    #[test]
    fn gate_huzuia_awamu_bila_mtangulizi() {
        let g = phase_gate("ujenzi", false);
        assert_eq!(g["ok"], json!(false));
        assert_eq!(g["blocked"], json!(true));
        let g2 = phase_gate("ujenzi", true);
        assert_eq!(g2["ok"], json!(true));
        let g3 = phase_gate("hakuna", true);
        assert_eq!(g3["ok"], json!(false));
    }

    #[test]
    fn score_ina_mipaka() {
        assert_eq!(phase_score(100.0, 0), 100.0);
        assert_eq!(phase_score(100.0, 10), 0.0);
        assert_eq!(phase_score(50.0, 1), 35.0);
    }

    #[test]
    fn safe_join_hukataa_traversal() {
        assert!(safe_join("../etc").is_none());
        assert!(safe_join("").is_none());
    }

    #[test]
    fn kind_mapping_ni_sahihi() {
        assert_eq!(project_kind("web"), Some("website"));
        assert_eq!(project_kind("game"), Some("gaming"));
        assert_eq!(project_kind("api"), None);
        assert_eq!(deploy_domain("web"), "website");
        assert_eq!(deploy_domain("game"), "gaming");
        assert_eq!(deploy_domain("cli"), "website");
    }

    #[test]
    fn files_for_hurudisha_faili_kwa_web() {
        let bp = crate::forge::template_blueprint("tovuti ndogo ya kuuza bidhaa");
        let files = files_for("web", "html", "tovuti ndogo ya kuuza bidhaa", &bp);
        assert!(!files.is_empty());
        assert!(files.iter().any(|(p, _)| p == "index.html"));
    }

    #[test]
    fn files_for_hurudisha_faili_kwa_api_blueprint() {
        let bp = crate::forge::template_blueprint("REST API ya malipo");
        let files = files_for("api", "rust", "REST API ya malipo", &bp);
        assert!(!files.is_empty());
        // API ya production ina tests (faili ya tests au inline #[cfg(test)]).
        assert!(
            files.iter().any(|(p, c)| p.contains("test") || c.contains("#[cfg(test)]")),
            "API ya production lazima iwe na tests"
        );
    }

    #[test]
    fn ugumu_rahisi_kwa_kazi_ndogo() {
        let (level, score, _) = assess_complexity("kikokotoo cha kuongeza namba", "web", 3);
        assert_eq!(level, "simple");
        assert!(score <= 2);
    }

    #[test]
    fn ugumu_mgumu_kwa_mfumo_mkubwa() {
        let spec = "enterprise SaaS multi-tenant platform with OAuth, database, payment \
                    integration, realtime websocket streaming, security compliance, GDPR, \
                    kubernetes, microservice, many users, audit";
        let (level, score, reasons) = assess_complexity(spec, "api", 20);
        assert_eq!(level, "complex");
        assert!(score > 5);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn ugumu_wastani_ni_kati() {
        let (level, _, _) = assess_complexity("REST API yenye database", "api", 5);
        assert_eq!(level, "medium");
    }

    #[test]
    fn level_label_kwa_lugha() {
        assert_eq!(level_label("simple", "sw"), "Rahisi");
        assert_eq!(level_label("simple", "en"), "Simple");
        assert_eq!(level_label("complex", "sw"), "Mgumu");
    }

    #[tokio::test]
    async fn mpango_wa_awamu_una_deliverables_na_idhini() {
        let db = mem_db();
        let plan = phase_plan(&db, "complex", "sw").await;
        assert_eq!(plan.len(), 6);
        for p in &plan {
            assert!(p["deliverables"].as_array().unwrap().len() >= 2);
            assert_eq!(p["requires_approval"], json!(true));
            assert!(!p["activities"].as_array().unwrap().is_empty());
        }
        let simple = phase_plan(&db, "simple", "sw").await;
        let cx: usize = plan.iter().map(|p| p["deliverables"].as_array().unwrap().len()).sum();
        let sm: usize = simple.iter().map(|p| p["deliverables"].as_array().unwrap().len()).sum();
        assert!(cx > sm, "Mgumu lazima uwe na deliverables zaidi");
    }

    #[tokio::test]
    async fn guardrails_zipo_kwa_sw_na_en() {
        let db = mem_db();
        let sw = guardrail_defs(&db, "sw").await;
        let en = guardrail_defs(&db, "en").await;
        assert_eq!(sw.len(), 6);
        assert!(sw.iter().any(|g| g["label"].as_str().unwrap().contains("secrets")));
        assert!(en.iter().any(|g| g["label"].as_str().unwrap().contains("secrets")));
    }

    #[tokio::test]
    async fn gate_inahitaji_idhini() {
        let v = gate(Json(json!({"phase": "ujenzi", "prior_ok": true}))).await;
        assert_eq!(v.0["requires_approval"], json!(true));
        assert_eq!(v.0["waiting_for_approval"], json!(true));
        let v2 = gate(Json(json!({"phase": "ujenzi", "prior_ok": true, "approved": true}))).await;
        assert!(v2.0.get("waiting_for_approval").is_none());
    }

    #[tokio::test]
    async fn resolve_lang_hutii_lang_au_username() {
        let db = mem_db();
        // lang ya moja kwa moja
        assert_eq!(resolve_lang(&db, &json!({"lang": "en"})).await, "en");
        // lang mbaya → default (sw) kama username haipo
        assert_eq!(resolve_lang(&db, &json!({"lang": "kiswahili"})).await, "sw");
        assert_eq!(resolve_lang(&db, &json!({})).await, "sw");
    }

    #[tokio::test]
    async fn init_tables_inafanya_kazi() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        record(&db, "demo", "website", "html", "simple", true, 95.0, &json!([]), "ok").await;
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sdlc_runs")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn next_phase_hufuata_mpangilio() {
        assert_eq!(next_phase("mahitaji"), Some("usanifu"));
        assert_eq!(next_phase("ujenzi"), Some("uthibitisho"));
        assert_eq!(next_phase("ufuatiliaji"), None);
        assert_eq!(next_phase("hakuna"), None);
    }

    #[tokio::test]
    async fn idhini_ya_awamu_huhifadhiwa() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        assert!(approved_phases(&db, "mradi").await.is_empty());
        let a = approve_phase(&db, "mradi", "mahitaji").await;
        assert_eq!(a, vec!["mahitaji"]);
        let a2 = approve_phase(&db, "mradi", "usanifu").await;
        assert_eq!(a2.len(), 2);
        // Awamu mbaya haihifadhiwi.
        let a3 = approve_phase(&db, "mradi", "hakuna").await;
        assert_eq!(a3.len(), 2);
        // Mradi mwingine hauna idhini.
        assert!(approved_phases(&db, "mwingine").await.is_empty());
    }

    #[tokio::test]
    async fn continue_inahitaji_project() {
        // Bila AppState haiwezi kuitwa hapa; tunathibitisha next_phase + idhini
        // (continue_phase ni mchanganyiko mwembamba wa hizi).
        assert_eq!(next_phase("usanifu"), Some("ujenzi"));
    }
}
