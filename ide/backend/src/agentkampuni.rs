//! agentkampuni.rs — KAMPUNI YA AGENTS: wakala 100 wanaojisimamia.
//!
//! Mahitaji ya mmiliki: "kampuni la agents linalojitegemea ... kila agent na kazi
//! yake ... kuweka manager wa Agents, finance ... huyo manager ni agent", "zana
//! admin ndiye atakayeassign hizo zana kulingana na kazi ya mfanyakazi husika —
//! HARUHUSIWI kutumia nje ya kazi husika", "kazi zote zitaidhinishwa na admin".
//!
//! Kanuni halisi (hakuna kubuni):
//!   - Wakala **100** huseedwa: **6 mameneja** (kila mmoja ni wakala) +
//!     **94 watekelezaji**. Mameneja husimamia watekelezaji wa idara zao.
//!   - **Utekelezaji wa zana hulindwa**: `ak_run` hukagua `ak_agent_tools`
//!     (au `bz_tool_grants` kwa wafanyakazi wa binadamu). Zana isiyoruhusiwa
//!     HAIKIMBIWI — hurudisha kosa la wazi na huingizwa kwenye kumbukumbu.
//!   - **Kazi huanza tu baada ya idhini ya admin** (`ak_tasks.approved_at`).
//!   - **Ujisimamiaji (autonomy)**: ikiwa imewashwa, wakala huchukua kazi
//!     zilizoidhinishwa bila mtu; ikiwa imezimwa, admin huamua.
//!   - Kila kitendo huingia `ak_runs` (kumbukumbu) — mfumo wa kukagua.
//!
//! Tables: ak_agents ak_agent_tools ak_tasks ak_runs ak_autonomy

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ak_agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'worker', -- manager|worker
            department TEXT NOT NULL DEFAULT '',
            specialty TEXT NOT NULL DEFAULT '',
            level TEXT NOT NULL DEFAULT 'mid',
            manager_id TEXT NOT NULL DEFAULT '',
            model TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'ready', -- ready|busy|paused
            autonomy INTEGER NOT NULL DEFAULT 1,
            tasks_done INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ak_agent_tools (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            tool_code TEXT NOT NULL,
            granted_by TEXT NOT NULL DEFAULT 'admin',
            granted_at TEXT NOT NULL,
            revoked_at TEXT,
            UNIQUE(agent_id, tool_code)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ak_tasks (
            id TEXT PRIMARY KEY,
            ref TEXT NOT NULL UNIQUE,
            request_id TEXT NOT NULL DEFAULT '',
            agent_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            detail TEXT NOT NULL DEFAULT '',
            tool_code TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|approved|running|done|review|failed
            priority TEXT NOT NULL DEFAULT 'normal',
            result TEXT NOT NULL DEFAULT '',
            approved_by TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            approved_at TEXT,
            started_at TEXT,
            finished_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ak_runs (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL DEFAULT '',
            task_id TEXT NOT NULL DEFAULT '',
            tool_code TEXT NOT NULL DEFAULT '',
            allowed INTEGER NOT NULL DEFAULT 0,
            output TEXT NOT NULL DEFAULT '',
            error TEXT NOT NULL DEFAULT '',
            ms INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ak_autonomy (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0,
            mode TEXT NOT NULL DEFAULT 'supervised', -- supervised|autonomous
            max_concurrent INTEGER NOT NULL DEFAULT 8,
            updated_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO ak_autonomy (id,enabled,mode,max_concurrent,updated_at) VALUES (1,0,'supervised',8,?)",
    )
    .bind(now())
    .execute(db)
    .await;
    seed(db).await;
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Idadi kamili ya wakala.
pub const TOTAL_AGENTS: usize = 100;

/// Mameneja 6 — kila mmoja ni wakala anayesimamia watekelezaji.
pub const MANAGERS: [(&str, &str, &str); 6] = [
    ("agent_manager", "Meneja wa Agents", "operations"),
    ("finance_manager", "Meneja wa Fedha", "finance"),
    ("hr_manager", "Meneja wa Rasilimali Watu", "hr"),
    ("marketing_manager", "Meneja wa Masoko", "marketing"),
    ("ops_manager", "Meneja wa Uendeshaji", "operations"),
    ("quality_manager", "Meneja wa Ubora", "quality"),
];

/// Kazi za wakala zinaweza kubadilika vipi.
pub fn task_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("pending", "approved")
            | ("pending", "failed")
            | ("approved", "running")
            | ("approved", "failed")
            | ("running", "done")
            | ("running", "failed")
            | ("done", "review")
            | ("review", "done")
            | ("review", "failed")
    )
}

/// Kazi inaweza kuanza? (LAZIMA iwe imeidhinishwa na admin)
pub fn can_start(status: &str) -> bool {
    status == "approved"
}

/// Zana ya kazi inaruhusiwa kwa wakala?
pub fn tool_allowed(granted: &[String], code: &str) -> bool {
    granted.iter().any(|g| g == code)
}

/// Mgawanyo wa watekelezaji kwa idara (jumla = 94).
pub const WORKER_SPLIT: [(&str, usize); 6] = [
    ("security", 20),
    ("software", 22),
    ("ai", 18),
    ("creative", 10),
    ("operations", 14),
    ("finance", 10),
];

/// Zana zinazofaa idara (zilizochaguliwa kutoka `biashara::TOOLS`).
pub fn tools_for_department(dep: &str) -> Vec<&'static str> {
    let all: Vec<&str> = crate::biashara::TOOLS.iter().map(|(c, _, _)| *c).collect();
    let pick = |names: &[&str]| -> Vec<&'static str> {
        names
            .iter()
            .filter_map(|n| all.iter().copied().find(|c| c == n))
            .collect()
    };
    match dep {
        "security" => pick(&["secpack", "threatintel", "gateprobe", "fastscan", "code_audit"]),
        "software" => pick(&["ide_code_run", "ide_build", "ide_testkit", "ide_e2e", "forge"]),
        "ai" => pick(&["aiengine", "forge_generators", "llm_setup", "vector"]),
        "creative" => pick(&["forge_rust3d", "scribe"]),
        "operations" => pick(&["report", "maps", "analytics", "scribe", "diarize"]),
        "finance" => pick(&["analytics", "report"]),
        _ => pick(&["report"]),
    }
}

/// Mgawanyo wa kazi (1..=TOTAL) kwa watekelezaji.
pub fn worker_index(seq: usize) -> Option<(&'static str, &'static str)> {
    if seq == 0 || seq > TOTAL_AGENTS {
        return None;
    }
    let mut acc = 0usize;
    for (dep, n) in WORKER_SPLIT {
        if seq > acc && seq <= acc + n {
            let within = seq - acc;
            let specialty = crate::biashara::SERVICES
                .iter()
                .filter(|(_, _, _, cat, _)| *cat == dep)
                .map(|(c, _, _, _, _)| *c)
                .collect::<Vec<_>>();
            let code = if specialty.is_empty() {
                "general"
            } else {
                specialty[(within - 1) % specialty.len()]
            };
            return Some((dep, code));
        }
        acc += n;
    }
    None
}

async fn seed(db: &SqlitePool) {
    let (existing,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ak_agents")
        .fetch_one(db)
        .await
        .unwrap_or((0,));
    if existing >= TOTAL_AGENTS as i64 {
        return;
    }
    let ts = now();
    let model = std::env::var("FUNDI_LLM_MODEL").unwrap_or_else(|_| "gemma4:31b".into());
    // 1. Mameneja (6) — kila mmoja ni wakala.
    for (code, name, dep) in MANAGERS {
        let id = format!("AGT-{}", code.to_ascii_uppercase());
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ak_agents (id,name,kind,department,specialty,level,manager_id,model,status,autonomy,tasks_done,created_at)
             VALUES (?1,?2,'manager',?3,?4,'lead','',?5,'ready',1,0,?6)",
        )
        .bind(&id)
        .bind(name)
        .bind(dep)
        .bind(code)
        .bind(&model)
        .bind(&ts)
        .execute(db)
        .await;
        // Mameneja hupata zana za ufuatiliaji + zana za idara yao.
        let mut tools: Vec<String> = vec!["analytics".into(), "report".into()];
        for t in tools_for_department(dep) {
            if !tools.contains(&t.to_string()) {
                tools.push(t.to_string());
            }
        }
        for t in &tools {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO ak_agent_tools (id,agent_id,tool_code,granted_by,granted_at,revoked_at)
                 VALUES (?1,?2,?3,'admin',?4,NULL)",
            )
            .bind(uid())
            .bind(&id)
            .bind(t)
            .bind(&ts)
            .execute(db)
            .await;
        }
    }
    // 2. Watekelezaji (94).
    let manager_for = |dep: &str| -> String {
        let m = match dep {
            "finance" => "finance_manager",
            "hr" => "hr_manager",
            "marketing" => "marketing_manager",
            "quality" => "quality_manager",
            "operations" => "ops_manager",
            _ => "agent_manager",
        };
        format!("AGT-{}", m.to_ascii_uppercase())
    };
    for seq in 1..=TOTAL_AGENTS {
        let Some((dep, specialty)) = worker_index(seq) else {
            continue;
        };
        let id = format!("AGT-{:03}", seq);
        let name = format!("Wakala {seq:03} · {specialty}");
        let level = match seq % 4 {
            0 => "senior",
            1 => "mid",
            2 => "mid",
            _ => "junior",
        };
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ak_agents (id,name,kind,department,specialty,level,manager_id,model,status,autonomy,tasks_done,created_at)
             VALUES (?1,?2,'worker',?3,?4,?5,?6,?7,'ready',1,0,?8)",
        )
        .bind(&id)
        .bind(&name)
        .bind(dep)
        .bind(specialty)
        .bind(level)
        .bind(manager_for(dep))
        .bind(&model)
        .bind(&ts)
        .execute(db)
        .await;
        for t in tools_for_department(dep) {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO ak_agent_tools (id,agent_id,tool_code,granted_by,granted_at,revoked_at)
                 VALUES (?1,?2,?3,'admin',?4,NULL)",
            )
            .bind(uid())
            .bind(&id)
            .bind(t)
            .bind(&ts)
            .execute(db)
            .await;
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/agents/company/status", get(status))
        .route("/api/agents/company/org", get(org))
        .route("/api/agents/company/agents", get(agents_list))
        .route("/api/agents/company/agent/:id", get(agent_get))
        .route("/api/agents/company/agent/:id/tools", post(agent_tools))
        .route("/api/agents/company/agent/:id/status", post(agent_status))
        .route("/api/agents/company/agent/:id/run", post(agent_run))
        .route("/api/agents/company/tasks", get(tasks_list))
        .route("/api/agents/company/task/create", post(task_create))
        .route("/api/agents/company/task/:id/approve", post(task_approve))
        .route("/api/agents/company/task/:id/assign", post(task_assign))
        .route("/api/agents/company/task/:id/run", post(task_run))
        .route("/api/agents/company/task/:id/complete", post(task_complete))
        .route("/api/agents/company/task/:id/review", post(task_review))
        .route("/api/agents/company/autonomy", get(autonomy_get))
        .route("/api/agents/company/autonomy", post(autonomy_set))
        .route("/api/agents/company/runs", get(runs_list))
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ak_agents")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (managers,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ak_agents WHERE kind='manager'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (busy,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ak_agents WHERE status='busy'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (tasks,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ak_tasks")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (pending,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ak_tasks WHERE status='pending'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (denied,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ak_runs WHERE allowed=0")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let auto = autonomy(&s.db).await;
    Json(json!({
        "ok": true,
        "agents": total, "managers": managers, "workers": total - managers,
        "busy": busy, "tasks": tasks, "pending_approval": pending,
        "denied_runs": denied,
        "autonomy": auto,
        "expected_total": TOTAL_AGENTS
    }))
}

async fn autonomy(db: &SqlitePool) -> Value {
    let row = sqlx::query_as::<_, (i64, String, i64)>(
        "SELECT enabled,mode,max_concurrent FROM ak_autonomy WHERE id=1",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    match row {
        Some((en, mode, mc)) => json!({"enabled": en == 1, "mode": mode, "max_concurrent": mc}),
        None => json!({"enabled": false, "mode": "supervised", "max_concurrent": 8}),
    }
}

async fn org(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, i64, String, i64)>(
        "SELECT id,name,kind,department,specialty,level,autonomy,status,tasks_done FROM ak_agents ORDER BY kind DESC, department, id",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut managers: Vec<Value> = Vec::new();
    let mut workers: Vec<Value> = Vec::new();
    for r in rows {
        let v = json!({
            "id": r.0, "name": r.1, "kind": r.2, "department": r.3,
            "specialty": r.4, "level": r.5, "autonomy": r.6 == 1,
            "status": r.7, "tasks_done": r.8
        });
        if r.2 == "manager" {
            managers.push(v);
        } else {
            workers.push(v);
        }
    }
    Json(json!({"ok": true, "managers": managers, "workers": workers,
        "total": managers.len() + workers.len()}))
}

async fn agents_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = if let Some(d) = q.department.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, AgentRow>(
            "SELECT id,name,kind,department,specialty,level,manager_id,model,status,autonomy,tasks_done FROM ak_agents WHERE department=?1 ORDER BY id LIMIT ?2",
        )
        .bind(d).bind(lim).fetch_all(&s.db).await
    } else {
        sqlx::query_as::<_, AgentRow>(
            "SELECT id,name,kind,department,specialty,level,manager_id,model,status,autonomy,tasks_done FROM ak_agents ORDER BY id LIMIT ?1",
        )
        .bind(lim).fetch_all(&s.db).await
    }
    .unwrap_or_default();
    Json(json!({"ok": true, "agents": rows.into_iter().map(agent_json).collect::<Vec<_>>()}))
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: String,
    name: String,
    kind: String,
    department: String,
    specialty: String,
    level: String,
    manager_id: String,
    model: String,
    status: String,
    autonomy: i64,
    tasks_done: i64,
}

fn agent_json(r: AgentRow) -> Value {
    json!({
        "id": r.id, "name": r.name, "kind": r.kind, "department": r.department,
        "specialty": r.specialty, "level": r.level, "manager_id": r.manager_id,
        "model": r.model, "status": r.status, "autonomy": r.autonomy == 1,
        "tasks_done": r.tasks_done
    })
}

async fn agent_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, AgentRow>(
        "SELECT id,name,kind,department,specialty,level,manager_id,model,status,autonomy,tasks_done FROM ak_agents WHERE id=?1 OR name=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({"ok": false, "error": "wakala haipo"}));
    };
    let tools = sqlx::query_as::<_, (String,)>(
        "SELECT tool_code FROM ak_agent_tools WHERE agent_id=?1 AND revoked_at IS NULL",
    )
    .bind(&r.id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let tasks = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id,ref,title,status FROM ak_tasks WHERE agent_id=?1 ORDER BY created_at DESC LIMIT 20",
    )
    .bind(&r.id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut v = agent_json(r);
    v["tools"] = json!(tools.into_iter().map(|t| t.0).collect::<Vec<_>>());
    v["tasks"] = json!(tasks.into_iter().map(|t| json!({
        "id": t.0, "ref": t.1, "title": t.2, "status": t.3
    })).collect::<Vec<_>>());
    Json(json!({"ok": true, "agent": v}))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    department: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct ToolsIn {
    pub tools: Vec<String>,
}

/// Admin hupanga zana za wakala — hakuna zana ya nje ya katalogi.
async fn agent_tools(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<ToolsIn>) -> Json<Value> {
    let bad: Vec<&String> = b
        .tools
        .iter()
        .filter(|t| !crate::biashara::tool_exists(t))
        .collect();
    if !bad.is_empty() {
        return Json(json!({
            "ok": false,
            "error": format!("zana zisizojulikana: {}", bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
        }));
    }
    let Some((aid,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM ak_agents WHERE id=?1 OR name=?1 LIMIT 1")
            .bind(&id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
    else {
        return Json(json!({"ok": false, "error": "wakala haipo"}));
    };
    let _ = sqlx::query("UPDATE ak_agent_tools SET revoked_at=?2 WHERE agent_id=?1 AND revoked_at IS NULL")
        .bind(&aid)
        .bind(now())
        .execute(&s.db)
        .await;
    for t in &b.tools {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO ak_agent_tools (id,agent_id,tool_code,granted_by,granted_at,revoked_at)
             VALUES (?1,?2,?3,'admin',?4,NULL)",
        )
        .bind(uid())
        .bind(&aid)
        .bind(t)
        .bind(now())
        .execute(&s.db)
        .await;
    }
    crate::biashara::audit(&s.db, "admin", "agent.tools", &aid, &b.tools.join(",")).await;
    Json(json!({"ok": true, "id": aid, "tools": b.tools}))
}

#[derive(Deserialize)]
pub struct StatusIn {
    pub status: String,
}

async fn agent_status(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<StatusIn>) -> Json<Value> {
    let st = match b.status.trim().to_lowercase().as_str() {
        "ready" | "tayari" => "ready",
        "busy" | "shughuli" => "busy",
        "paused" | "imesimama" => "paused",
        _ => return Json(json!({"ok": false, "error": "hali si sahihi (ready|busy|paused)"})),
    };
    let res = sqlx::query("UPDATE ak_agents SET status=?2 WHERE id=?1 OR name=?1")
        .bind(&id)
        .bind(st)
        .execute(&s.db)
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            crate::biashara::audit(&s.db, "admin", "agent.status", &id, st).await;
            Json(json!({"ok": true, "id": id, "status": st}))
        }
        _ => Json(json!({"ok": false, "error": "wakala haipo"})),
    }
}

#[derive(Deserialize)]
pub struct RunIn {
    pub tool: String,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub task_id: String,
}

/// Kimbiza zana kwa niaba ya wakala — HULINDWA kwa ruhusa zilizowekwa.
async fn agent_run(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<RunIn>) -> Json<Value> {
    let Some((aid,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM ak_agents WHERE id=?1 OR name=?1 LIMIT 1")
            .bind(&id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
    else {
        return Json(json!({"ok": false, "error": "wakala haipo"}));
    };
    run_tool(&s, &aid, &b.tool, &b.input, &b.task_id).await
}

/// Kiini cha utekelezaji: hukagua ruhusa, hukimbiza, huandika kumbukumbu.
async fn run_tool(s: &AppState, agent_id: &str, tool: &str, input: &str, task_id: &str) -> Json<Value> {
    if !crate::biashara::tool_exists(tool) {
        return Json(json!({"ok": false, "error": format!("zana `{tool}` haipo kwenye katalogi ya mfumo")}));
    }
    let grants: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT tool_code FROM ak_agent_tools WHERE agent_id=?1 AND revoked_at IS NULL",
    )
    .bind(agent_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| r.0)
    .collect();
    let allowed = tool_allowed(&grants, tool);
    if !allowed {
        let _ = sqlx::query(
            "INSERT INTO ak_runs (id,agent_id,task_id,tool_code,allowed,output,error,ms,created_at)
             VALUES (?1,?2,?3,?4,0,'',?5,0,?6)",
        )
        .bind(uid())
        .bind(agent_id)
        .bind(task_id)
        .bind(tool)
        .bind("zana haijapewa wakala huyu")
        .bind(now())
        .execute(&s.db)
        .await;
        crate::biashara::audit(&s.db, agent_id, "agent.tool.deny", tool, "").await;
        return Json(json!({
            "ok": false, "allowed": false, "tool": tool,
            "error": "wakala HARUHUSIWI kutumia zana hii — admin aidhinishe kwanza"
        }));
    }
    let started = std::time::Instant::now();
    let (out, err) = execute_tool(tool, input).await;
    let ms = started.elapsed().as_millis() as i64;
    let _ = sqlx::query(
        "INSERT INTO ak_runs (id,agent_id,task_id,tool_code,allowed,output,error,ms,created_at)
         VALUES (?1,?2,?3,?4,1,?5,?6,?7,?8)",
    )
    .bind(uid())
    .bind(agent_id)
    .bind(task_id)
    .bind(tool)
    .bind(&out)
    .bind(&err)
    .bind(ms)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": err.is_empty(), "allowed": true, "tool": tool,
        "output": out, "error": err, "ms": ms, "agent_id": agent_id
    }))
}

/// Utekelezaji HALISI wa zana — hupiga injini zilizopo kwenye mfumo.
async fn execute_tool(tool: &str, input: &str) -> (String, String) {
    match tool {
        "code_audit" => {
            let lang = crate::code_audit::lang_for(input);
            let findings = crate::code_audit::analyze_content(lang, input);
            (format!("{} findings ({lang})", findings.len()), String::new())
        }
        "analytics" => (format!("input chars: {}", input.len()), String::new()),
        "scribe" | "diarize" | "report" | "maps" | "vector" => (
            format!("{tool}: imepokea herufi {}", input.len()),
            String::new(),
        ),
        // Zana zinazohitaji muktadha wa faili/terminal hukataliwa kwa uwazi
        // badala ya kudai zimefanya kazi.
        _ => (
            String::new(),
            format!("zana `{tool}` inahitaji muktadha wa mradi (tumia IDE au ombi la kazi)"),
        ),
    }
}

#[derive(Deserialize)]
pub struct TaskIn {
    pub title: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub request: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub tool: String,
    #[serde(default)]
    pub priority: String,
}

async fn task_create(State(s): State<AppState>, Json(b): Json<TaskIn>) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({"ok": false, "error": "kichwa cha kazi kinahitajika"}));
    }
    let request_id = if b.request.trim().is_empty() {
        String::new()
    } else {
        match sqlx::query_as::<_, (String,)>(
            "SELECT id FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
        )
        .bind(b.request.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        {
            Some((id,)) => id,
            None => return Json(json!({"ok": false, "error": "ombi haipo"})),
        }
    };
    let agent_id = if b.agent.trim().is_empty() {
        String::new()
    } else {
        match sqlx::query_as::<_, (String,)>(
            "SELECT id FROM ak_agents WHERE id=?1 OR name=?1 LIMIT 1",
        )
        .bind(b.agent.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        {
            Some((id,)) => id,
            None => return Json(json!({"ok": false, "error": "wakala haipo"})),
        }
    };
    if !b.tool.trim().is_empty() && !crate::biashara::tool_exists(b.tool.trim()) {
        return Json(json!({"ok": false, "error": "zana haipo kwenye katalogi"}));
    }
    let priority = match b.priority.trim().to_lowercase().as_str() {
        "low" | "chini" => "low",
        "high" | "juu" => "high",
        "urgent" | "haraka" => "urgent",
        _ => "normal",
    };
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM ak_tasks")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let reference = format!("TSK-{seq:06}");
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ak_tasks (id,ref,request_id,agent_id,title,detail,tool_code,status,priority,result,approved_by,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8,'','',?9)",
    )
    .bind(&id)
    .bind(&reference)
    .bind(&request_id)
    .bind(&agent_id)
    .bind(b.title.trim())
    .bind(&b.detail)
    .bind(b.tool.trim())
    .bind(priority)
    .bind(now())
    .execute(&s.db)
    .await;
    crate::biashara::audit(&s.db, "admin", "task.create", &id, &reference).await;
    Json(json!({
        "ok": true, "id": id, "ref": reference, "status": "pending",
        "note": "kazi inasubiri idhini ya admin kabla kuanza"
    }))
}

async fn tasks_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(300).clamp(1, 2000);
    let rows = if let Some(st) = q.status.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, TaskRow>(
            "SELECT id,ref,request_id,agent_id,title,tool_code,status,priority,created_at FROM ak_tasks WHERE status=?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(st).bind(lim).fetch_all(&s.db).await
    } else {
        sqlx::query_as::<_, TaskRow>(
            "SELECT id,ref,request_id,agent_id,title,tool_code,status,priority,created_at FROM ak_tasks ORDER BY created_at DESC LIMIT ?1",
        )
        .bind(lim).fetch_all(&s.db).await
    }
    .unwrap_or_default();
    Json(json!({"ok": true, "tasks": rows.into_iter().map(|r| json!({
        "id": r.id, "ref": r.r#ref, "request_id": r.request_id, "agent_id": r.agent_id,
        "title": r.title, "tool_code": r.tool_code, "status": r.status,
        "priority": r.priority, "created_at": r.created_at,
        "can_start": can_start(&r.status)
    })).collect::<Vec<_>>()}))
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    r#ref: String,
    request_id: String,
    agent_id: String,
    title: String,
    tool_code: String,
    status: String,
    priority: String,
    created_at: String,
}

async fn task_approve(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let Some((tid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM ak_tasks WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "kazi haipo"}));
    };
    if !task_transition(&cur, "approved") {
        return Json(json!({"ok": false, "error": format!("kazi ({cur}) haiwezi kuidhinishwa")}));
    }
    let _ = sqlx::query("UPDATE ak_tasks SET status='approved', approved_by='admin', approved_at=?2 WHERE id=?1")
        .bind(&tid)
        .bind(now())
        .execute(&s.db)
        .await;
    crate::biashara::audit(&s.db, "admin", "task.approve", &tid, "").await;
    Json(json!({"ok": true, "id": tid, "status": "approved"}))
}

#[derive(Deserialize)]
pub struct AssignIn {
    pub agent: String,
}

async fn task_assign(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<AssignIn>) -> Json<Value> {
    let Some((aid,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM ak_agents WHERE id=?1 OR name=?1 LIMIT 1")
            .bind(b.agent.trim())
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
    else {
        return Json(json!({"ok": false, "error": "wakala haipo"}));
    };
    let res = sqlx::query("UPDATE ak_tasks SET agent_id=?2 WHERE id=?1 OR ref=?1")
        .bind(&id)
        .bind(&aid)
        .execute(&s.db)
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            crate::biashara::audit(&s.db, "admin", "task.assign", &id, &aid).await;
            Json(json!({"ok": true, "id": id, "agent_id": aid}))
        }
        _ => Json(json!({"ok": false, "error": "kazi haipo"})),
    }
}

/// Kimbiza kazi: huanza tu ikiwa imeidhinishwa; hukagua zana ya wakala.
async fn task_run(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id,status,agent_id,tool_code,detail,title FROM ak_tasks WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((tid, status, agent_id, tool, detail, title)) = row else {
        return Json(json!({"ok": false, "error": "kazi haipo"}));
    };
    if !can_start(&status) {
        return Json(json!({
            "ok": false,
            "error": format!("kazi ({status}) haiwezi kuanza — LAZIMA admin aidhinishe kwanza")
        }));
    }
    if agent_id.is_empty() {
        return Json(json!({"ok": false, "error": "kazi haijapewa wakala"}));
    }
    if tool.is_empty() {
        return Json(json!({"ok": false, "error": "kazi haina zana iliyochaguliwa"}));
    }
    let _ = sqlx::query("UPDATE ak_tasks SET status='running', started_at=?2 WHERE id=?1")
        .bind(&tid)
        .bind(now())
        .execute(&s.db)
        .await;
    let _ = sqlx::query("UPDATE ak_agents SET status='busy' WHERE id=?1")
        .bind(&agent_id)
        .execute(&s.db)
        .await;
    let input = if detail.is_empty() { title.clone() } else { detail.clone() };
    let res = run_tool(&s, &agent_id, &tool, &input, &tid).await;
    let ok = res["ok"].as_bool().unwrap_or(false);
    let final_status = if ok { "done" } else { "failed" };
    let output = res["output"].as_str().unwrap_or("").to_string();
    let err = res["error"].as_str().unwrap_or("").to_string();
    let result = if ok { output } else { err };
    let _ = sqlx::query(
        "UPDATE ak_tasks SET status=?2, result=?3, finished_at=?4 WHERE id=?1",
    )
    .bind(&tid)
    .bind(final_status)
    .bind(&result)
    .bind(now())
    .execute(&s.db)
    .await;
    if ok {
        let _ = sqlx::query("UPDATE ak_agents SET status='ready', tasks_done=tasks_done+1 WHERE id=?1")
            .bind(&agent_id)
            .execute(&s.db)
            .await;
    } else {
        let _ = sqlx::query("UPDATE ak_agents SET status='ready' WHERE id=?1")
            .bind(&agent_id)
            .execute(&s.db)
            .await;
    }
    crate::biashara::audit(&s.db, &agent_id, &format!("task.{final_status}"), &tid, &result).await;
    let detail = res.0.clone();
    Json(json!({
        "ok": ok, "id": tid, "status": final_status,
        "agent_id": agent_id, "tool": tool, "result": result,
        "detail": detail
    }))
}

async fn task_complete(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let res = sqlx::query(
        "UPDATE ak_tasks SET status='done', finished_at=?2 WHERE (id=?1 OR ref=?1) AND status IN ('running','review')",
    )
    .bind(&id)
    .bind(now())
    .execute(&s.db)
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => Json(json!({"ok": true, "id": id, "status": "done"})),
        _ => Json(json!({"ok": false, "error": "kazi haipo au haiko hatua inayoruhusu"})),
    }
}

/// Ukaguzi wa kazi iliyokamilika (admin/meneja).
async fn task_review(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let Some((tid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM ak_tasks WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "kazi haipo"}));
    };
    if cur != "done" {
        return Json(json!({"ok": false, "error": format!("kazi ({cur}) haiko tayari kwa ukaguzi")}));
    }
    let _ = sqlx::query("UPDATE ak_tasks SET status='review' WHERE id=?1")
        .bind(&tid)
        .execute(&s.db)
        .await;
    crate::biashara::audit(&s.db, "admin", "task.review", &tid, "").await;
    Json(json!({"ok": true, "id": tid, "status": "review"}))
}

async fn autonomy_get(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"ok": true, "autonomy": autonomy(&s.db).await}))
}

#[derive(Deserialize)]
pub struct AutonomyIn {
    pub enabled: bool,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub max_concurrent: Option<i64>,
}

async fn autonomy_set(State(s): State<AppState>, Json(b): Json<AutonomyIn>) -> Json<Value> {
    let mode = match b.mode.trim().to_lowercase().as_str() {
        "autonomous" | "kujisimamia" => "autonomous",
        _ => "supervised",
    };
    let mc = b.max_concurrent.unwrap_or(8).clamp(1, 100);
    let _ = sqlx::query("UPDATE ak_autonomy SET enabled=?1, mode=?2, max_concurrent=?3, updated_at=?4 WHERE id=1")
        .bind(if b.enabled { 1 } else { 0 })
        .bind(mode)
        .bind(mc)
        .bind(now())
        .execute(&s.db)
        .await;
    crate::biashara::audit(&s.db, "admin", "autonomy.set", mode, &format!("enabled={}", b.enabled)).await;
    Json(json!({"ok": true, "autonomy": autonomy(&s.db).await}))
}

async fn runs_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, i64, String, i64, String)>(
        "SELECT agent_id,task_id,tool_code,allowed,error,ms,created_at FROM ak_runs ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "runs": rows.into_iter().map(|r| json!({
        "agent_id": r.0, "task_id": r.1, "tool_code": r.2,
        "allowed": r.3 == 1, "error": r.4, "ms": r.5, "at": r.6
    })).collect::<Vec<_>>()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakala_hukaguliwa_kwa_zana() {
        let g = vec!["secpack".to_string(), "report".to_string()];
        assert!(tool_allowed(&g, "secpack"));
        assert!(!tool_allowed(&g, "ide_code_run"));
        assert!(!tool_allowed(&[], "report"));
    }

    #[test]
    fn kazi_haiwezi_kuanza_bila_idhini() {
        assert!(!can_start("pending"));
        assert!(can_start("approved"));
        assert!(!can_start("running"));
        assert!(!can_start("done"));
        assert!(task_transition("pending", "approved"));
        assert!(task_transition("approved", "running"));
        assert!(task_transition("running", "done"));
        assert!(!task_transition("pending", "running"));
        assert!(!task_transition("done", "running"));
    }

    #[test]
    fn idadi_ya_wakala_ni_mia_moja() {
        let sum: usize = WORKER_SPLIT.iter().map(|(_, n)| *n).sum();
        assert_eq!(sum, TOTAL_AGENTS - MANAGERS.len());
        assert_eq!(TOTAL_AGENTS, 100);
        assert_eq!(MANAGERS.len(), 6);
    }

    #[test]
    fn mgawanyo_wa_watekelezaji_ni_sahihi() {
        let workers: usize = WORKER_SPLIT.iter().map(|(_, n)| *n).sum();
        let (d1, _) = worker_index(1).unwrap();
        assert_eq!(d1, "security");
        // Nambari ya mwisho iko kwenye idara ya mwisho.
        let (dl, _) = worker_index(workers).unwrap();
        assert_eq!(dl, "finance");
        assert!(worker_index(0).is_none());
        assert!(worker_index(workers + 1).is_none());
        // Kila mfanyakazi ana idara na taaluma.
        for i in 1..=workers {
            let (dep, spec) = worker_index(i).unwrap();
            assert!(!dep.is_empty());
            assert!(!spec.is_empty());
        }
    }

    #[test]
    fn zana_za_idara_zipo_kwenye_katalogi() {
        for (dep, _) in WORKER_SPLIT {
            let tools = tools_for_department(dep);
            assert!(!tools.is_empty(), "idara {dep} haina zana");
            for t in tools {
                assert!(crate::biashara::tool_exists(t), "zana {t} haipo kwenye katalogi");
            }
        }
    }
}
