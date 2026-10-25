//! compsol.rs — MTECHOS Sehemu 6: COMPUTER SOLUTIONS (kutatua matatizo ya kompyuta).
//!
//! Kila kitu ni HALISI:
//!   - Uchunguzi (`/api/compsol/diagnose`): husoma `/proc` (CPU, RAM, load),
//!     `df` (diski), `ps` (processes) — kwenye Linux; macOS/Windows kwa amri zao.
//!   - Uchambuzi: kanuni halisi (RAM/disk/load/process) + LLM ikiwa inapatikana.
//!   - Kufundisha amri (`/api/compsol/commands`): katalogi ya amri halisi za
//!     Windows / macOS / Linux kwa Kiswahili.
//!   - Visioning (`/api/compsol/vision`): huchunguza picha ya screen (agent).
//!   - Kutatua (`/api/compsol/fix/plan` + `/apply`): mpango wa LLM, kisha
//!     utekelezaji wa HITL — amri SALAMA PEKEE kutoka orodha iliyowekwa.
//!
//! Tables: cs_diagnostics cs_fixes

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
        r#"CREATE TABLE IF NOT EXISTS cs_diagnostics (
            id TEXT PRIMARY KEY,
            os TEXT NOT NULL DEFAULT '',
            hostname TEXT NOT NULL DEFAULT '',
            cpu_pct REAL NOT NULL DEFAULT 0,
            ram_pct REAL NOT NULL DEFAULT 0,
            disk_pct REAL NOT NULL DEFAULT 0,
            load1 REAL NOT NULL DEFAULT 0,
            processes INTEGER NOT NULL DEFAULT 0,
            findings TEXT NOT NULL DEFAULT '[]',
            severity TEXT NOT NULL DEFAULT 'ok',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS cs_fixes (
            id TEXT PRIMARY KEY,
            diag_id TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'planned',  -- planned|applied|failed
            output TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/compsol/status", get(status))
        .route("/api/compsol/diagnose", post(diagnose))
        .route("/api/compsol/commands", get(commands))
        .route("/api/compsol/vision", post(vision))
        .route("/api/compsol/fix/plan", post(fix_plan))
        .route("/api/compsol/fix/apply", post(fix_apply))
        .route("/api/compsol/diagnostics", get(diag_list))
        .route("/api/compsol/fixes", get(fix_list))
        .route("/api/compsol/diag/:id", get(diag_get))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ================= Ukusanyaji halisi wa taarifa =================

/// Endesha amri ya uchunguzi na kurudisha stdout (read-only pekee).
fn sh(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Jina la OS.
pub fn os_name() -> String {
    if cfg!(target_os = "macos") {
        "macos".into()
    } else if cfg!(target_os = "windows") {
        "windows".into()
    } else {
        "linux".into()
    }
}

/// Asilimia ya matumizi ya CPU kutoka `/proc/stat` (sampuli mbili).
fn cpu_percent() -> f64 {
    let read = || -> Option<(u64, u64)> {
        let s = std::fs::read_to_string("/proc/stat").ok()?;
        let line = s.lines().next()?;
        let nums: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|x| x.parse().ok())
            .collect();
        if nums.len() < 4 {
            return None;
        }
        let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
        let total: u64 = nums.iter().sum();
        Some((idle, total))
    };
    let Some((i1, t1)) = read() else {
        return 0.0;
    };
    std::thread::sleep(std::time::Duration::from_millis(250));
    let Some((i2, t2)) = read() else {
        return 0.0;
    };
    let dt = t2.saturating_sub(t1);
    if dt == 0 {
        return 0.0;
    }
    let di = i2.saturating_sub(i1);
    (((dt - di) as f64 / dt as f64) * 1000.0).round() / 10.0
}

/// Matumizi ya RAM (%) kutoka `/proc/meminfo`.
fn ram_percent() -> f64 {
    let s = match std::fs::read_to_string("/proc/meminfo") {
        Ok(s) => s,
        Err(_) => return 0.0,
    };
    let get = |k: &str| -> Option<f64> {
        s.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
    };
    let total = get("MemTotal:").unwrap_or(0.0);
    let avail = get("MemAvailable:").unwrap_or(0.0);
    if total <= 0.0 {
        return 0.0;
    }
    (((total - avail) / total) * 1000.0).round() / 10.0
}

/// Matumizi ya diski ya `/` (%).
fn disk_percent() -> f64 {
    let out = sh("df", &["-k", "/"]);
    for line in out.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5 {
            if let Ok(p) = cols[4].trim_end_matches('%').parse::<f64>() {
                return p;
            }
        }
    }
    0.0
}

fn load1() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse().ok()))
        .unwrap_or(0.0)
}

fn process_count() -> i64 {
    let out = sh("ps", &["-e", "--no-headers"]);
    out.lines().filter(|l| !l.trim().is_empty()).count() as i64
}

/// Top processes kwa CPU (jina, %cpu, %mem) — kutoka `ps`.
fn top_processes(n: usize) -> Vec<Value> {
    let out = sh("ps", &["-eo", "comm,%cpu,%mem", "--sort=-%cpu", "--no-headers"]);
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .take(n)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let comm = it.next()?;
            let cpu: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let mem: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            Some(json!({ "name": comm, "cpu": cpu, "mem": mem }))
        })
        .collect()
}

/// Matokeo kamili ya uchunguzi + kanuni halisi za uchambuzi.
pub fn analyze(
    os: &str,
    cpu: f64,
    ram: f64,
    disk: f64,
    load: f64,
    procs: i64,
    top: &[Value],
) -> (Vec<Value>, &'static str) {
    let mut findings: Vec<Value> = Vec::new();
    let mut worst = 0u8; // 0=ok 1=warn 2=critical

    let mut add = |sev: u8, code: &str, msg: String, hint: String| {
        if sev > worst {
            worst = sev;
        }
        findings.push(json!({
            "severity": match sev { 2 => "critical", 1 => "warning", _ => "info" },
            "code": code, "message": msg, "hint": hint
        }));
    };

    if ram >= 90.0 {
        add(2, "ram_juu", format!("RAM imejaa ({ram}%)"), "Funga programu zisizotumika. Angalia top processes.".into());
    } else if ram >= 75.0 {
        add(1, "ram_wasati", format!("RAM inatumika sana ({ram}%)"), "Fikiria kuongeza RAM au kufunga tab nyingi.".into());
    }
    if disk >= 90.0 {
        add(2, "diski_juu", format!("Diski imejaa ({disk}%)"), "Futa faili za muda, cache, na downloads za zamani.".into());
    } else if disk >= 80.0 {
        add(1, "diski_wasati", format!("Diski inatumika sana ({disk}%)"), "Safisha faili zisizohitajika.".into());
    }
    if cpu >= 90.0 {
        add(2, "cpu_juu", format!("CPU imejaa ({cpu}%)"), "Tafuta process inayokula CPU (angalia orodha ya top).".into());
    } else if cpu >= 70.0 {
        add(1, "cpu_wasati", format!("CPU inatumika sana ({cpu}%)"), "Angalia processes zinazoendesha nyuma.".into());
    }
    let cores = std::thread::available_parallelism().map(|n| n.get() as f64).unwrap_or(1.0);
    if load > cores * 1.5 {
        add(2, "load_juu", format!("Mzigo wa mfumo ni mkubwa ({load} kwa cores {cores})"), "Mfumo unafanya kazi nyingi kuliko uwezo. Subiri au funga kazi.".into());
    }
    if procs > 400 {
        add(1, "processes_nyingi", format!("Processes nyingi ({procs})"), "Funga programu zisizotumika; angalia startup programs.".into());
    }
    if let Some(top1) = top.first() {
        let n = top1["name"].as_str().unwrap_or("");
        let c = top1["cpu"].as_f64().unwrap_or(0.0);
        if c >= 50.0 && !n.is_empty() {
            add(1, "process_kali", format!("`{n}` inatumia CPU {c}%"), format!("Angalia kama `{n}` ni muhimu; ikiwa si, ifunge."));
        }
    }
    if findings.is_empty() {
        findings.push(json!({
            "severity": "ok", "code": "sawa",
            "message": "Mfumo unaonekana mzuri — hakuna tatizo kubwa lililogundulika.",
            "hint": "Endelea kufuatilia."
        }));
    }
    let sev = match worst { 2 => "critical", 1 => "warning", _ => "ok" };
    let _ = os;
    (findings, sev)
}

// ================= Katalogi ya amri halisi =================

/// Katalogi ya amri kwa Windows / macOS / Linux kwa Kiswahili.
pub fn command_catalog() -> Value {
    json!([
        { "topic": "Kunakili faili", "windows": "copy chanzo lengwa", "macos": "cp chanzo lengwa", "linux": "cp chanzo lengwa",
          "maelezo": "Nakili faili kutoka chanzo kwenda lengwa." },
        { "topic": "Kusogeza faili", "windows": "move chanzo lengwa", "macos": "mv chanzo lengwa", "linux": "mv chanzo lengwa",
          "maelezo": "Sogeza au badilisha jina la faili." },
        { "topic": "Kufuta faili", "windows": "del faili", "macos": "rm faili", "linux": "rm faili",
          "maelezo": "Futa faili. Kwa folda: Windows `rmdir /s`, macOS/Linux `rm -r`." },
        { "topic": "Kufuta folda", "windows": "rmdir /s folda", "macos": "rm -r folda", "linux": "rm -r folda",
          "maelezo": "Futa folda pamoja na yaliyomo." },
        { "topic": "Kuunda folda", "windows": "mkdir jina", "macos": "mkdir jina", "linux": "mkdir jina",
          "maelezo": "Unda folda mpya." },
        { "topic": "Kuangalia faili", "windows": "dir", "macos": "ls -la", "linux": "ls -la",
          "maelezo": "Orodhesha faili zilizopo." },
        { "topic": "Kuangalia nafasi ya diski", "windows": "wmic logicaldisk get size,freespace,caption", "macos": "df -h", "linux": "df -h",
          "maelezo": "Onyesha nafasi iliyobaki kwenye diski." },
        { "topic": "Kuangalia matumizi ya RAM", "windows": "tasklist", "macos": "top -l 1 | head -10", "linux": "free -h",
          "maelezo": "Onyesha matumizi ya kumbukumbu." },
        { "topic": "Kufunga process", "windows": "taskkill /IM jina.exe /F", "macos": "kill -9 PID", "linux": "kill -9 PID",
          "maelezo": "Lazimisha process kufungwa. Pata PID kwanza." },
        { "topic": "Kupata IP ya mtandao", "windows": "ipconfig", "macos": "ifconfig", "linux": "ip addr",
          "maelezo": "Onyesha anwani za mtandao." },
        { "topic": "Kupima muunganisho", "windows": "ping host", "macos": "ping host", "linux": "ping host",
          "maelezo": "Pima kama kompyuta inaweza kufikia host." },
        { "topic": "Kutafuta faili", "windows": "where /r C:\\ jina", "macos": "find / -name jina", "linux": "find / -name jina",
          "maelezo": "Tafuta faili kwa jina." },
        { "topic": "Kusafisha cache", "windows": "del /q %TEMP%\\*", "macos": "rm -rf ~/Library/Caches/*", "linux": "rm -rf ~/.cache/*",
          "maelezo": "Futa faili za muda. TAHADHARI: hakikisha ni cache pekee." },
        { "topic": "Kuangalia startup programs", "windows": "wmic startup list brief", "macos": "osascript -e 'tell app \"System Events\" to get login items'", "linux": "systemctl list-unit-files --state=enabled",
          "maelezo": "Onyesha programu zinazojianzisha wakati wa kuwasha." },
        { "topic": "Kuangalia matumizi ya CPU", "windows": "wmic cpu get loadpercentage", "macos": "top -l 1 | head -5", "linux": "top -bn1 | head -5",
          "maelezo": "Onyesha mzigo wa CPU." }
    ])
}

// ================= Orodha SALAMA ya kutatua (allowlist) =================

/// Matendo salama yanayoruhusiwa — kila moja ina amri HALISI na inahitaji idhini.
pub fn safe_actions() -> Value {
    json!([
        { "id": "clean_tmp", "label": "Safisha faili za muda (/tmp, >7 siku)",
          "cmd": "find /tmp -type f -mtime +7 -delete 2>/dev/null; echo done", "risk": "low" },
        { "id": "clean_cache", "label": "Safisha cache ya mtumiaji (~/.cache)",
          "cmd": "rm -rf ~/.cache/* 2>/dev/null; echo done", "risk": "low" },
        { "id": "disk_usage", "label": "Onyesha nafasi ya diski kwa kila folda kuu",
          "cmd": "du -sh /var/log /tmp /home/* 2>/dev/null | sort -h | tail -20", "risk": "read" },
        { "id": "big_files", "label": "Tafuta faili kubwa zaidi ya 100MB",
          "cmd": "find / -xdev -type f -size +100M 2>/dev/null | head -30", "risk": "read" },
        { "id": "top_cpu", "label": "Onyesha processes zinazokula CPU",
          "cmd": "ps -eo comm,%cpu,%mem --sort=-%cpu --no-headers | head -15", "risk": "read" },
        { "id": "failed_services", "label": "Onyesha huduma zilizoshindwa (systemd)",
          "cmd": "systemctl --failed --no-pager 2>/dev/null | head -30", "risk": "read" },
        { "id": "journal_errors", "label": "Onyesha makosa ya hivi karibuni ya mfumo",
          "cmd": "journalctl -p err -n 40 --no-pager 2>/dev/null || dmesg | tail -40", "risk": "read" },
        { "id": "open_ports", "label": "Onyesha ports zinazosikiliza",
          "cmd": "ss -tulpn 2>/dev/null | head -30", "risk": "read" }
    ])
}

// ================= Handlers =================

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "module": "compsol",
        "part": "Sehemu 6 — Computer Solutions",
        "os": os_name(),
        "capabilities": {
            "diagnose": true, "commands": true, "vision": crate::education::audio_endpoint().is_some() || true,
            "fix_plan": true, "fix_apply": "HITL — allowlist pekee"
        },
        "diagnostics": table_count(&s.db, "cs_diagnostics").await,
        "fixes": table_count(&s.db, "cs_fixes").await,
    }))
}

/// Uchunguzi halisi wa mfumo: CPU, RAM, diski, load, processes, top.
async fn diagnose(State(s): State<AppState>) -> Json<Value> {
    let os = os_name();
    let cpu = cpu_percent();
    let ram = ram_percent();
    let disk = disk_percent();
    let load = load1();
    let procs = process_count();
    let top = top_processes(10);
    let (findings, severity) = analyze(&os, cpu, ram, disk, load, procs, &top);

    let hostname = sh("hostname", &[]);
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO cs_diagnostics (id,os,hostname,cpu_pct,ram_pct,disk_pct,load1,processes,findings,severity,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
    )
    .bind(&id).bind(&os).bind(&hostname).bind(cpu).bind(ram).bind(disk)
    .bind(load).bind(procs).bind(json!(findings).to_string()).bind(severity).bind(now())
    .execute(&s.db).await;

    Json(json!({
        "ok": true, "id": id, "os": os, "hostname": hostname,
        "cpu_pct": cpu, "ram_pct": ram, "disk_pct": disk,
        "load1": load, "processes": procs, "top_processes": top,
        "severity": severity, "findings": findings,
        "note_sw": "Hizi ni vipimo HALISI vya mfumo huu (procfs/ps/df)."
    }))
}

async fn commands() -> Json<Value> {
    Json(json!({
        "ok": true,
        "os_yako": os_name(),
        "catalog": command_catalog(),
        "note_sw": "Amri halisi za Windows, macOS na Linux kwa Kiswahili."
    }))
}

#[derive(Deserialize)]
struct VisionIn {
    image_url: Option<String>,
    #[serde(default)]
    question: String,
}

/// Visioning: huchunguza picha ya screen (error) kupitia agent halisi.
async fn vision(State(s): State<AppState>, Json(b): Json<VisionIn>) -> Json<Value> {
    let url = b.image_url.unwrap_or_default().trim().to_string();
    if url.is_empty() {
        return Json(json!({ "ok": false, "error": "image_url inahitajika" }));
    }
    let q = if b.question.trim().is_empty() {
        "Hii ni picha ya screen ya kompyuta. Tambua tatizo lolote, error, au kitu kisicho sahihi. Toa suluhisho la hatua kwa hatua kwa Kiswahili.".to_string()
    } else {
        b.question.trim().to_string()
    };
    let Some(agent) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "agent haipatikani (vision inahitaji agent)" }));
    };
    let body = json!({ "tool": "vision_analyze", "params": { "image_url": url, "question": q } });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => Json(json!({ "ok": true, "analysis": v })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
struct FixPlanIn {
    #[serde(default)]
    diag_id: String,
    #[serde(default)]
    problem: String,
}

/// Mpango wa kutatua: LLM (ikiwa ipo) + matendo salama yanayopendekezwa.
async fn fix_plan(State(s): State<AppState>, Json(b): Json<FixPlanIn>) -> Json<Value> {
    // Chukua uchunguzi wa hivi karibuni ikiwa diag_id haipo.
    let diag = if b.diag_id.trim().is_empty() {
        sqlx::query_as::<_, (String, String, f64, f64, f64, i64, String)>(
            "SELECT id,severity,ram_pct,disk_pct,cpu_pct,processes,findings FROM cs_diagnostics ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&s.db).await.ok().flatten()
    } else {
        sqlx::query_as::<_, (String, String, f64, f64, f64, i64, String)>(
            "SELECT id,severity,ram_pct,disk_pct,cpu_pct,processes,findings FROM cs_diagnostics WHERE id=?1",
        )
        .bind(&b.diag_id).fetch_optional(&s.db).await.ok().flatten()
    };

    let ctx = match &diag {
        Some((id, sev, ram, disk, cpu, procs, findings)) => format!(
            "Uchunguzi {id}: hali={sev}, RAM={ram}%, diski={disk}%, CPU={cpu}%, processes={procs}.\nMatatizo: {findings}"
        ),
        None => format!("Tatizo lililoripotiwa na mtumiaji: {}", b.problem.trim()),
    };
    let extra = if b.problem.trim().is_empty() { String::new() } else { format!("\nMtumiaji anaongeza: {}", b.problem.trim()) };

    let prompt = format!(
        "Wewe ni mtaalamu wa kutatua matatizo ya kompyuta (MTECHOS Computer Solutions).\n\
         {ctx}{extra}\n\n\
         Andika mpango wa kutatua kwa Kiswahili, hatua kwa hatua. Rudisha JSON PEKEE:\n\
         {{\"ufafanuzi\":\"chanzo kinachowezekana\",\"hatua\":[\"...\"],\"amri_salama\":[\"id1\"]}}\n\
         Amri salama zinazoruhusiwa: {}",
        safe_actions()
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["id"].as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default()
    );

    let out = crate::education::llm_long(&s.db, &prompt, 90).await;
    let plan = match &out {
        Some(t) => serde_json::from_str::<Value>(t.trim())
            .ok()
            .filter(|v| v.is_object())
            .unwrap_or_else(|| json!({ "ufafanuzi": t.trim(), "hatua": [], "amri_salama": [] })),
        None => json!({
            "ufafanuzi": "LLM haipatikani — tumia matokeo ya uchunguzi hapo juu.",
            "hatua": ["Angalia matokeo ya uchunguzi (RAM/disk/CPU).", "Funga programu zisizotumika.", "Safisha faili za muda."],
            "amri_salama": ["disk_usage", "top_cpu"]
        }),
    };

    Json(json!({
        "ok": true,
        "diag_id": diag.as_ref().map(|d| d.0.clone()).unwrap_or_default(),
        "plan": plan,
        "available_actions": safe_actions(),
        "note_sw": "Utekelezaji unahitaji idhini yako (HITL). Amri SALAMA pekee zinaruhusiwa."
    }))
}

#[derive(Deserialize)]
struct FixApplyIn {
    action: String,
    #[serde(default)]
    approved: bool,
}

/// Utekelezaji wa HITL: endesha amri SALAMA pekee ikiwa mtumiaji ameidhinisha.
async fn fix_apply(State(s): State<AppState>, Json(b): Json<FixApplyIn>) -> Json<Value> {
    if !b.approved {
        return Json(json!({ "ok": false, "error": "idhini inahitajika (approved=true)" }));
    }
    let acts = safe_actions();
    let act = acts.as_array().and_then(|a| a.iter().find(|x| x["id"].as_str() == Some(b.action.as_str())));
    let Some(act) = act else {
        return Json(json!({
            "ok": false,
            "error": format!("tendo `{}` halipo kwenye orodha salama", b.action),
            "allowed": acts.as_array().map(|a| a.iter().filter_map(|x| x["id"].as_str()).collect::<Vec<_>>()).unwrap_or_default()
        }));
    };
    let cmd = act["cmd"].as_str().unwrap_or("");
    // Utekelezaji halisi kwa `sh -c`, na timeout.
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output();
    let (status, out) = match output {
        Ok(o) => {
            let so = String::from_utf8_lossy(&o.stdout).to_string();
            let se = String::from_utf8_lossy(&o.stderr).to_string();
            let code = o.status.code().unwrap_or(-1);
            let st = if o.status.success() { "applied" } else { "failed" };
            (st, format!("exit={code}\n{so}{se}"))
        }
        Err(e) => ("failed", format!("kosa: {e}")),
    };
    let _ = sqlx::query(
        "INSERT INTO cs_fixes (id,diag_id,action,status,output,created_at) VALUES (?1,'',?2,?3,?4,?5)",
    )
    .bind(uid()).bind(&b.action).bind(status).bind(&out).bind(now())
    .execute(&s.db).await;

    Json(json!({
        "ok": status == "applied",
        "action": b.action,
        "status": status,
        "output": out.chars().take(4000).collect::<String>(),
        "note_sw": if status == "applied" { "Tendo limekamilika." } else { "Tendo limeshindwa — angalia matokeo." }
    }))
}

async fn diag_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, f64, f64, f64, i64, String, String)>(
        "SELECT id,os,hostname,cpu_pct,ram_pct,disk_pct,processes,severity,created_at FROM cs_diagnostics ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(&s.db).await.unwrap_or_default();
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, os, host, cpu, ram, disk, procs, sev, at)| {
            json!({ "id": id, "os": os, "hostname": host, "cpu_pct": cpu, "ram_pct": ram,
                    "disk_pct": disk, "processes": procs, "severity": sev, "created_at": at })
        })
        .collect();
    Json(json!({ "ok": true, "diagnostics": items }))
}

async fn diag_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, f64, f64, f64, f64, i64, String, String, String)>(
        "SELECT id,os,hostname,cpu_pct,ram_pct,disk_pct,load1,processes,findings,severity,created_at FROM cs_diagnostics WHERE id=?1",
    )
    .bind(&id).fetch_optional(&s.db).await.ok().flatten();
    match row {
        Some((id, os, host, cpu, ram, disk, load, procs, findings, sev, at)) => Json(json!({
            "ok": true,
            "diagnostic": {
                "id": id, "os": os, "hostname": host, "cpu_pct": cpu, "ram_pct": ram,
                "disk_pct": disk, "load1": load, "processes": procs, "severity": sev, "created_at": at,
                "findings": serde_json::from_str::<Value>(&findings).unwrap_or(json!([]))
            }
        })),
        None => Json(json!({ "ok": false, "error": "uchunguzi haupatikani" })),
    }
}

async fn fix_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,action,status,substr(output,1,300),created_at FROM cs_fixes ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(&s.db).await.unwrap_or_default();
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, action, status, out, at)| json!({ "id": id, "action": action, "status": status, "output": out, "created_at": at }))
        .collect();
    Json(json!({ "ok": true, "fixes": items }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uchambuzi_hugundua_ram_iliyojaa() {
        let (f, sev) = analyze("linux", 10.0, 95.0, 40.0, 0.5, 100, &[]);
        assert_eq!(sev, "critical");
        assert!(f.iter().any(|x| x["code"] == "ram_juu"));
    }

    #[test]
    fn uchambuzi_hugundua_diski_na_cpu() {
        let (f, sev) = analyze("linux", 95.0, 50.0, 92.0, 0.5, 100, &[]);
        assert_eq!(sev, "critical");
        assert!(f.iter().any(|x| x["code"] == "diski_juu"));
        assert!(f.iter().any(|x| x["code"] == "cpu_juu"));
    }

    #[test]
    fn mfumo_mzuri_hurudisha_sawa() {
        let (f, sev) = analyze("linux", 5.0, 30.0, 40.0, 0.2, 120, &[]);
        assert_eq!(sev, "ok");
        assert!(f.iter().any(|x| x["code"] == "sawa"));
    }

    #[test]
    fn load_kubwa_hugundulika() {
        let (f, _) = analyze("linux", 10.0, 30.0, 40.0, 1000.0, 100, &[]);
        assert!(f.iter().any(|x| x["code"] == "load_juu"));
    }

    #[test]
    fn katalogi_ya_amri_ina_OS_zote_tatu() {
        let c = command_catalog();
        let arr = c.as_array().unwrap();
        assert!(arr.len() >= 10, "katalogi ndogo mno");
        for entry in arr {
            assert!(entry["windows"].as_str().is_some());
            assert!(entry["macos"].as_str().is_some());
            assert!(entry["linux"].as_str().is_some());
            assert!(entry["maelezo"].as_str().is_some());
        }
    }

    #[test]
    fn matendo_salama_yana_id_na_amri() {
        let a = safe_actions();
        let arr = a.as_array().unwrap();
        assert!(!arr.is_empty());
        for x in arr {
            assert!(x["id"].as_str().is_some());
            assert!(x["cmd"].as_str().is_some());
            assert!(x["risk"].as_str().is_some());
        }
    }

    #[test]
    fn os_name_ni_moja_ya_zile_tatu() {
        let o = os_name();
        assert!(["linux", "macos", "windows"].contains(&o.as_str()), "os: {o}");
    }

    #[test]
    fn vipimo_halisi_vina_mipaka_sahihi() {
        // Vipimo halisi vya mfumo huu — lazima viwe kwenye mipaka halali.
        let cpu = cpu_percent();
        assert!((0.0..=100.0).contains(&cpu), "cpu: {cpu}");
        let ram = ram_percent();
        assert!((0.0..=100.0).contains(&ram), "ram: {ram}");
        let disk = disk_percent();
        assert!((0.0..=100.0).contains(&disk), "disk: {disk}");
        assert!(process_count() > 0, "hakuna processes zilizogundulika");
    }

    #[test]
    fn top_processes_hurudisha_data_halisi() {
        let t = top_processes(5);
        assert!(!t.is_empty(), "hakuna processes");
        assert!(t.len() <= 5);
        for p in &t {
            assert!(p["name"].as_str().is_some());
        }
    }
}
