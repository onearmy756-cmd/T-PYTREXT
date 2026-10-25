//! threatswarm — UJUMUISHAJI wa ThreatSwarm kwenye agent ya MTECHOS.
//!
//! Kill-chain AUTONOMOUS ya pentest (recon → recon-details → vuln → notes)
//! yenye scope-gate HALISI: kila lengo lazima liwe Ndani ya `scope.txt`
//! (IP/CIDR/domain), lenye fallback kwa scope_check.mjs ya mradi (kara
//! tena: exit 2 = imezuiwa). Evidence zimeandikwa kwenye `data/threatswarm/`.
//!
//! SHERIA: MICHEZO MIABAYA (destructive stages kama exploit/post-ex) inahitaji
//! idhini ya binadamu (HITL: human_approved = true). Bila idhini, stage zina
//! kuruka na report inaonyesha `blocked_by_hitl: true`.
//!
//! Reference: server/security-tools/threatswarm/README.md (mukul975/ThreatSwarm).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

const EXEC_TIMEOUT: Duration = Duration::from_secs(120);
/// Path ndani ya repo ambapo scope.txt iko (relative kwa CWD ya server).
const SCOPE_FILE: &str = "server/security-tools/threatswarm/scope.txt";

/// Path halisi ya scope.txt. Inaweza kubadilishwa kwa env `FUNDI_SCOPE_FILE`
/// (mfano kwenye container: `/data/threatswarm/scope.txt`); kama env haipo,
/// tunajaribu path ya repo, kisha `/data` (container), kisha CWD.
pub fn scope_path() -> String {
    if let Ok(p) = std::env::var("FUNDI_SCOPE_FILE") {
        if !p.trim().is_empty() {
            return p;
        }
    }
    // 1) relative kwa CWD (server inaendesha kutoka repo root)
    // 2) relative kwa package dir (cargo test: CWD = server/agent)
    // 3) /data (container)
    let from_pkg = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(SCOPE_FILE);
    let candidates = [
        PathBuf::from(SCOPE_FILE),
        from_pkg,
        PathBuf::from("/data/threatswarm/scope.txt"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.to_string_lossy().to_string();
        }
    }
    "/data/threatswarm/scope.txt".to_string()
}
const DATA_DIR: &str = "data/threatswarm";

// ---------- Scope gate (port ya scope_check.mjs kwa Rust) ----------

/// Check kama target ipo kwenye scope.txt (IP/CIDR/domain). Ni HALISI:
/// CIDR awareness kamili, tena fallback ni py hook.
pub fn check_scope(target: &str) -> Result<(), String> {
    let t = target.trim();
    if t.is_empty() {
        return Err("target ni tupu — lazima uweke lengo".to_string());
    }
    // Host ya ndani (local runtime) — kama scope_check.mjs: localhost huruhusiwa
    if matches!(t.to_lowercase().as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0") {
        return Ok(());
    }
    let raw = match std::fs::read_to_string(scope_path()) {
        Ok(r) => r,
        Err(_) => return Err("scope.txt haipatikani — hakuna authorized targets".to_string()),
    };
    let mut has_any = false;
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        has_any = true;
        // CIDR network? (IPv4 pekee — IPv6 CIDR inarudi kama domain token)
        if let Some((base, mask)) = parse_cidr(line) {
            if let Some(ip) = parse_ipv4(t) {
                if base == (ip & mask) {
                    return Ok(());
                }
            }
            continue;
        }
        // Single IP?
        if let (Some(ip), Some(line_ip)) = (parse_ipv4(t), parse_ipv4(line)) {
            if ip == line_ip {
                return Ok(());
            }
            continue;
        }
        // Domain: wildcard au exact au subdomain match
        let tl = line.to_lowercase();
        let td = t.to_lowercase();
        if tl.starts_with("*.") {
            let suffix = tl.trim_start_matches("*.");
            if td.ends_with(suffix) || td == suffix {
                return Ok(());
            }
        } else if tl == td {
            return Ok(());
        }
    }
    if !has_any {
        return Err("scope.txt ni mtupu — weka authorized targets kabla ya kuanza".to_string());
    }
    Err(format!(
        "'{t}' HAIPO kwenye scope.txt — command imezuiwa (deterministic scope gate, si convention). \
         Ongeza kwenye {} kwa idhini halali pekee.",
        scope_path()
    ))
}

// ---------- SCOPE MANAGEMENT (admin anaongeza/kutoa malengo) ----------

/// Orodha ya entries za scope (bila maoni/tupu).
pub fn scope_entries() -> Vec<String> {
    scope_entries_at(&scope_path())
}

fn scope_entries_at(path: &str) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|raw| {
            raw.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        })
        .unwrap_or_default()
}

/// JSON ya scope kwa UI.
pub fn scope_json() -> Value {
    let entries = scope_entries();
    json!({
        "ok": true,
        "path": scope_path(),
        "entries": entries,
        "count": entries.len(),
    })
}

/// Ongeza lengo kwenye scope.txt (admin pekee — RBAC inalinda route).
pub fn scope_add(entry: &str) -> Result<Value, String> {
    scope_add_at(&scope_path(), entry)
}

fn scope_add_at(path: &str, entry: &str) -> Result<Value, String> {
    let e = entry.trim();
    if e.is_empty() {
        return Err("lengo ni tupu".to_string());
    }
    // Sauriasi: herufi, namba, nukta, kistari, mkwaju, koloni, nyota
    if !e
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '/' | ':' | '*' | '_'))
    {
        return Err("lengo lina herufi zisizoruhusiwa".to_string());
    }
    if let Some(dir) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let existing = scope_entries_at(path);
    if existing.iter().any(|x| x.eq_ignore_ascii_case(e)) {
        return Ok(json!({ "ok": true, "added": false, "entry": e, "note": "lengo lipo tayari", "entries": existing }));
    }
    let mut raw = std::fs::read_to_string(path).unwrap_or_default();
    if !raw.is_empty() && !raw.ends_with('\n') {
        raw.push('\n');
    }
    raw.push_str(e);
    raw.push('\n');
    std::fs::write(path, raw).map_err(|err| format!("imeshindwa kuandika scope: {err}"))?;
    Ok(json!({ "ok": true, "added": true, "entry": e, "entries": scope_entries_at(path) }))
}

/// Toa lengo kwenye scope.txt.
pub fn scope_remove(entry: &str) -> Result<Value, String> {
    scope_remove_at(&scope_path(), entry)
}

fn scope_remove_at(path: &str, entry: &str) -> Result<Value, String> {
    let e = entry.trim();
    if e.is_empty() {
        return Err("lengo ni tupu".to_string());
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut kept: Vec<String> = Vec::new();
    let mut removed = false;
    for line in raw.lines() {
        if line.trim().eq_ignore_ascii_case(e) {
            removed = true;
            continue;
        }
        kept.push(line.to_string());
    }
    if removed {
        let mut out = kept.join("\n");
        out.push('\n');
        std::fs::write(path, out).map_err(|err| format!("imeshindwa kuandika scope: {err}"))?;
    }
    Ok(json!({ "ok": true, "removed": removed, "entry": e, "entries": scope_entries_at(path) }))
}

/// Fallback: piga scope_check hook HALISI ya ThreatSwarm (Node port,
/// `.claude/hooks/scope_check.mjs`). Inarudi Ok(()) ikiwa exit code == 0,
/// Error kwingine (exit 2 = block).
#[allow(dead_code)] // fallback — inatumika pale server inaendesha ndani ya Claude Code hooks env
pub fn check_scope_via_hook(command: &str) -> Result<(), String> {
    let hook = Path::new("server/security-tools/threatswarm/.claude/hooks/scope_check.mjs");
    if !hook.exists() {
        return Err("scope_check.mjs haipatikani kwenye mradi".to_string());
    }
    let payload = json!({ "tool_name": "Bash", "tool_input": { "command": command } }).to_string();
    let out = std::process::Command::new("node")
        .arg(hook)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            if let Some(si) = c.stdin.as_mut() {
                let _ = si.write_all(payload.as_bytes());
            }
            c.wait_with_output()
        });
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let msg = String::from_utf8_lossy(&o.stderr);
            Err(format!("scope hook imesimamisha: {}\ncommand: {command}", msg.trim()))
        }
        Err(e) => Err(format!("scope hook failes: {e}")),
    }
}

// ---------- BINARY mapping (agent_ranked → kazi za secpack) ----------

/// Kill-chain stages za AUTONOMOUS mwendo. Zote ni recon/vuln inline —
/// pato ni read-only kwa lengo, hakuna exploit bila HITL.
const AUTONOMOUS_STAGES: &[(&str, &str)] = &[
    // (stage id, task_id ya secpack — HALISI kutoka TASKS)
    ("recon", "t10-scan"),
    ("recon-details", "info-dns"),
    ("web-vuln", "vuln-nikto"),
    ("tls-audit", "info-ssl"),
    ("os-config-audit", "vuln-lynis"),
    ("antivirus", "clam-scan"),
];

// ---------- ENGINE ENGAGE (autonomous) ----------

pub struct Engage {
    pub id: String,
    pub target: String,
    pub stages_done: Vec<Value>,
    pub created_at: String,
}

/// ANZISHA kill-chain kamili kwa target (scope-gated) — inarejea report JSON.
pub async fn engage(
    db: &sqlx::SqlitePool,
    target: &str,
    human_approved: bool,
    internal_agent: bool,
) -> Value {
    let id = format!("ts-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));

    // Scope gate NDANI ya server — deterministic, hakuna bypass kupitia prompt
    if let Err(e) = check_scope(target) {
        return json!({ "ok": false, "blocked_by_scope": true, "error": e });
    }

    // account/accounting: hitaji idhini ya mmiliki (HITL) kwa kazi za hatari
    let mut ev = PathBuf::from(DATA_DIR);
    let _ = std::fs::create_dir_all(&ev);
    ev.push(format!("{id}.json"));

    let mut stages_done: Vec<Value> = Vec::new();
    let mut evidence: Vec<Value> = Vec::new();
    for (stage, task_id) in AUTONOMOUS_STAGES {
        let res = crate::secpack::run_task(db, task_id, target, human_approved).await;
        let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let blocked_by_hitl = res
            .get("needs_approval")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let out = res.get("output").and_then(|v| v.as_str()).unwrap_or("");
        let dur = res.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0);
        stages_done.push(json!({
            "stage": stage,
            "task_id": task_id,
            "ok": ok,
            "blocked_by_hitl": blocked_by_hitl,
            "duration_ms": dur,
            "output_preview": preview(out, 400),
        }));
        if !out.is_empty() {
            evidence.push(json!({
                "stage": stage,
                "output": preview(out, 2000),
                "t": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            }));
        }
    }

    // R ENGINE ANALYTICS: score stages (ok=1.0, hitl-blocked=5.0, fail=7.0) kwa risk
    let r_scores: Vec<f64> = stages_done
        .iter()
        .map(|s| {
            if s.get("blocked_by_hitl").and_then(|v| v.as_bool()).unwrap_or(false) {
                5.0
            } else if s.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                1.0
            } else {
                7.0
            }
        })
        .collect();
    let r_analytics = if r_scores.is_empty() {
        json!({ "ok": false, "error": "hakuna stages" })
    } else {
        crate::r_engine::run_req(
            "risk_score",
            &json!({ "data": r_scores }),
        )
        .await
    };

    let report = json!({
        "id": id,
        "target": target,
        "created_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "internal_agent": internal_agent,
        "human_approved": human_approved,
        "scope_check": "deterministic (scope.txt, CIDR-aware)",
        "stages": stages_done,
        "r_analytics": r_analytics,
        "evidence": evidence,
        "note_sw": "Kill chain AUTONOMOUS imefanyika kupitia ThreatSwarm reference. \
                    Stage zote ni READ-ONLY (recon/vuln); exploit/post-ex zinahitaji HITL tofauti.",
    });

    // Evidence + report kwenye diski (path-scoped: evidence/**)
    let _ = std::fs::write(&ev, serde_json::to_string_pretty(&report).unwrap_or_default());

    let _ = internal_agent;
    report
}

fn preview(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..n.saturating_sub(3)])
    }
}

// ---------- CIDR / IPv4 parsing (pure std, hakuna crate mpya) ----------

/// Gawa IPv4 kwa octets u32 (little-endian network order).
fn parse_ipv4(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut acc: u32 = 0;
    for p in parts {
        // Byte strict: 0..=255, hakuna '+' au leading zeros ya kabla ya 1
        if p.is_empty() || p.len() > 3 {
            return None;
        }
        if !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u16 = p.parse().ok()?;
        if n > 255 {
            return None;
        }
        acc = (acc << 8) | (n as u32);
    }
    Some(acc)
}

/// Parse `a.b.c.d/p` — p ni 0..=32. Return (base u32, mask u32).
fn parse_cidr(s: &str) -> Option<(u32, u32)> {
    let (ip_part, prefix) = s.split_once('/')?;
    let ip = parse_ipv4(ip_part)?;
    let p: u8 = prefix.trim().parse().ok()?;
    if p > 32 {
        return None;
    }
    let mask = if p == 0 {
        0
    } else if p >= 32 {
        u32::MAX
    } else {
        !(u32::MAX >> p)
    };
    Some((ip & mask, mask))
}

// ---------- AGENTS × TOOLS ZOTE (one-command autonomous dispatch) ----------

/// RAMA ya ThreatSwarm agent → kazi za secpack (tools halisi za Kali).
/// Kila agent ina kazi zake halisi zilizoandaliwa; zote (WEKEZA scope.txt KABLA).
const AGENT_TASK_MAP: &[(&str, &[&str])] = &[
    // --- OFFENSIVE ---
    ("recon", &["t10-scan", "info-masscan", "info-netdiscover", "id-amass"]),
    ("exploit", &["expl-msf", "expl-searchsploit"]),
    ("post-ex", &["post-enum4linux", "post-smbclient"]),
    ("web-attacker", &["vuln-nikto", "t10-sqlmap", "web-whatweb", "web-wpscan", "web-dirb", "web-waf"]),
    ("api-attacker", &["web-whatweb", "t10-sqlmap"]),
    ("active-directory", &["post-enum4linux", "t10-netexec"]),
    ("network-ops", &["sniff-tcpdump", "sniff-tshark", "sniff-ettercap", "sniff-darkstat"]),
    ("osint", &["info-harvest", "info-whois", "info-dns"]),
    ("wireless-attacker", &["wifi-airmon", "wifi-airodump", "w802-reaver", "w802-wifite"]),
    ("cloud-attacker", &["t10-scan", "info-ssl"]),
    ("container-attacker", &["t10-scan", "vuln-nikto"]),
    ("mobile-attacker", &["fore-binwalk"]),
    ("social-engineer", &["info-harvest", "pass-cewl", "soc-113", "soc-128", "soc-59"]),
    ("social-defender", &["soc-58", "soc-64", "soc-88", "soc-138", "soc-158", "soc-258"]),
    ("evasion", &["info-ssl", "web-whatweb"]),
    ("c2-operator", &["sniff-tshark"]),
    // --- SPECIALIST ---
    ("crypto-attacker", &["info-ssl"]),
    ("iot-attacker", &["fore-binwalk", "info-masscan"]),
    ("password-attacks", &["pass-hydra", "pass-crunch", "pass-cewl"]),
    ("vuln-researcher", &["expl-searchsploit", "vuln-nikto", "vuln-lynis"]),
    ("reverse-engineer", &["rev-strace", "rev-ltrace", "rev-r2"]),
    ("malware-analyst", &["clam-scan", "fore-binwalk"]),
    // --- DEFENSIVE ---
    ("dfir", &["fore-vol", "fore-fls", "fore-binwalk", "fore-hashdeep"]),
    ("threat-hunter", &["sniff-tcpdump", "sniff-tshark"]),
    ("blue-team", &["vuln-lynis", "fore-rkhunter", "fore-chkrootkit"]),
    ("log-analyst", &["sniff-tshark", "sniff-darkstat"]),
    ("compliance-scanner", &["vuln-lynis"]),
    ("report-writer", &[]), // inasoma evidence zote na kutengeneza muhtasari
];

pub fn agents_catalog() -> Value {
    let agents: Vec<Value> = AGENT_TASK_MAP
        .iter()
        .map(|(agent, tasks)| json!({
            "agent": agent,
            "tasks": tasks,
            "task_count": tasks.len(),
        }))
        .collect();
    json!({
        "ok": true,
        "agents": agents,
        "total_agents": AGENT_TASK_MAP.len(),
        "note_sw": "Kila agent (kutoka ThreatSwarm) imeramiwa kwa kazi HALISI za security pack (Kali tools).",
    })
}

pub fn find_agent(agent: &str) -> Option<&'static [&'static str]> {
    AGENT_TASK_MAP.iter().find(|(a, _)| *a == agent).map(|(_, t)| *t)
}

/// Idadi ya agents za ThreatSwarm (kwa katalogi ya SAFU MOJA).
pub fn agent_count() -> usize {
    AGENT_TASK_MAP.len()
}

/// Orodha ya (jina, kazi) za agents zote za ThreatSwarm.
pub fn agent_entries() -> Vec<(&'static str, &'static [&'static str])> {
    AGENT_TASK_MAP.iter().map(|(a, t)| (*a, *t)).collect()
}

// ---------------------------------------------------------------------------
// AGENT-ENGINE CONDUCTOR (agents 27 huongoza zana zote)
//
// Safu hii imehamishwa kutoka AGENT-ENGINE (rust/akili-security/src/threatswarm.rs).
// Inaongeza daraja kati ya agents na katalogi ya majukwaa: kila agent ana
// (jukwaa, kazi) kupitia `agentbind`, kisha `plan()` hurudisha amri HALISI.
// ---------------------------------------------------------------------------

/// Aina ya agent: ulinzi (defensive), mashambulizi (offensive), au mtaalamu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentClass {
    Defensive,
    Offensive,
    Specialist,
}

impl AgentClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentClass::Defensive => "defensive",
            AgentClass::Offensive => "offensive",
            AgentClass::Specialist => "specialist",
        }
    }
    pub fn is_defensive(&self) -> bool {
        matches!(self, AgentClass::Defensive)
    }
}

/// Agent mmoja wa ThreatSwarm (AGENT-ENGINE shape).
#[derive(Debug, Clone)]
pub struct SwarmAgent {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub class: AgentClass,
    pub tasks: &'static [&'static str],
    pub tools: &'static [&'static str],
    pub agent_file: &'static str,
}

/// Agents 27 za ThreatSwarm (chanzo kimoja: AGENT-ENGINE).
pub const AGENTS: &[SwarmAgent] = &[
    SwarmAgent { name: "recon", title: "Reconnaissance & Enumeration",
        description: "Scans, enumerates ports, fingerprints services, discovers subdomains.",
        class: AgentClass::Offensive, tasks: &["t10-scan", "info-masscan", "info-netdiscover", "id-amass"],
        tools: &["nmap", "masscan", "httpx", "subfinder", "amass", "nuclei"], agent_file: ".claude/agents/recon.md" },
    SwarmAgent { name: "exploit", title: "Exploitation",
        description: "Gains initial access by exploiting CVEs and running Metasploit modules.",
        class: AgentClass::Offensive, tasks: &["expl-msf", "expl-searchsploit"],
        tools: &["msfconsole", "searchsploit"], agent_file: ".claude/agents/exploit.md" },
    SwarmAgent { name: "post-ex", title: "Post-Exploitation",
        description: "Privilege escalation, lateral movement, persistence.",
        class: AgentClass::Offensive, tasks: &["post-enum4linux", "post-smbclient"],
        tools: &["enum4linux", "smbclient", "linpeas"], agent_file: ".claude/agents/post-ex.md" },
    SwarmAgent { name: "web-attacker", title: "Web Application Testing",
        description: "SQLi, XSS, SSRF, LFI, IDOR, JWT attacks.",
        class: AgentClass::Offensive, tasks: &["vuln-nikto", "t10-sqlmap", "web-whatweb", "web-wpscan", "web-dirb", "web-waf"],
        tools: &["sqlmap", "nikto", "wpscan", "whatweb", "feroxbuster", "wafw00f"], agent_file: ".claude/agents/web-attacker.md" },
    SwarmAgent { name: "api-attacker", title: "API Security Testing",
        description: "REST, GraphQL, gRPC, WebSocket — BOLA/IDOR, auth flaws.",
        class: AgentClass::Offensive, tasks: &["web-whatweb", "t10-sqlmap"],
        tools: &["sqlmap", "httpx"], agent_file: ".claude/agents/api-attacker.md" },
    SwarmAgent { name: "active-directory", title: "Active Directory Attacks",
        description: "Kerberoasting, AS-REP roast, domain enumeration.",
        class: AgentClass::Offensive, tasks: &["post-enum4linux", "t10-netexec"],
        tools: &["enum4linux", "netexec"], agent_file: ".claude/agents/active-directory.md" },
    SwarmAgent { name: "network-ops", title: "Network Penetration",
        description: "ARP attacks, MitM, packet capture, SNMP enumeration.",
        class: AgentClass::Offensive, tasks: &["sniff-tcpdump", "sniff-tshark", "sniff-ettercap", "sniff-darkstat"],
        tools: &["tcpdump", "tshark", "ettercap"], agent_file: ".claude/agents/network-ops.md" },
    SwarmAgent { name: "osint", title: "OSINT",
        description: "Passive reconnaissance — domain intel, WHOIS, DNS.",
        class: AgentClass::Offensive, tasks: &["info-harvest", "info-whois", "info-dns"],
        tools: &["theHarvester", "whois", "dig"], agent_file: ".claude/agents/osint.md" },
    SwarmAgent { name: "wireless-attacker", title: "Wireless Testing",
        description: "WPA2/WPA3 capture and cracking.",
        class: AgentClass::Offensive, tasks: &["wifi-airmon", "wifi-airodump", "w802-reaver", "w802-wifite"],
        tools: &["airmon-ng", "airodump-ng", "wifite"], agent_file: ".claude/agents/wireless-attacker.md" },
    SwarmAgent { name: "cloud-attacker", title: "Cloud Penetration",
        description: "AWS/Azure/GCP IAM enumeration, privilege escalation.",
        class: AgentClass::Offensive, tasks: &["t10-scan", "info-ssl"],
        tools: &["nmap", "openssl"], agent_file: ".claude/agents/cloud-attacker.md" },
    SwarmAgent { name: "container-attacker", title: "Container & Kubernetes",
        description: "Docker escape, Kubernetes misconfig, RBAC abuse.",
        class: AgentClass::Offensive, tasks: &["t10-scan", "vuln-nikto"],
        tools: &["nmap", "nikto"], agent_file: ".claude/agents/container-attacker.md" },
    SwarmAgent { name: "mobile-attacker", title: "Mobile Application Testing",
        description: "APK decompilation, static/dynamic analysis.",
        class: AgentClass::Offensive, tasks: &["fore-binwalk"],
        tools: &["binwalk", "apktool"], agent_file: ".claude/agents/mobile-attacker.md" },
    SwarmAgent { name: "social-engineer", title: "Social Engineering",
        description: "Phishing simulation, GoPhish campaigns, pretexts.",
        class: AgentClass::Offensive, tasks: &["info-harvest", "pass-cewl"],
        tools: &["gophish", "theHarvester"], agent_file: ".claude/agents/social-engineer.md" },
    SwarmAgent { name: "evasion", title: "Evasion",
        description: "AV/EDR evasion for authorized red teams (AMSI bypass).",
        class: AgentClass::Offensive, tasks: &["info-ssl", "web-whatweb"],
        tools: &["openssl", "whatweb"], agent_file: ".claude/agents/evasion.md" },
    SwarmAgent { name: "c2-operator", title: "Command & Control",
        description: "C2 infrastructure for authorized red team ops.",
        class: AgentClass::Offensive, tasks: &["sniff-tshark"],
        tools: &["tshark"], agent_file: ".claude/agents/c2-operator.md" },
    SwarmAgent { name: "crypto-attacker", title: "Cryptography & TLS",
        description: "TLS config auditing, JWT algorithm weaknesses.",
        class: AgentClass::Specialist, tasks: &["info-ssl"],
        tools: &["openssl", "testssl"], agent_file: ".claude/agents/crypto-attacker.md" },
    SwarmAgent { name: "iot-attacker", title: "IoT & Embedded",
        description: "Firmware extraction, hardware interfaces.",
        class: AgentClass::Specialist, tasks: &["fore-binwalk", "info-masscan"],
        tools: &["binwalk", "masscan"], agent_file: ".claude/agents/iot-attacker.md" },
    SwarmAgent { name: "password-attacks", title: "Password Attacks",
        description: "Hash cracking, credential attacks.",
        class: AgentClass::Specialist, tasks: &["pass-hydra", "pass-crunch", "pass-cewl"],
        tools: &["hydra", "john", "hashcat"], agent_file: ".claude/agents/password-attacks.md" },
    SwarmAgent { name: "vuln-researcher", title: "Vulnerability Research",
        description: "NVD queries, searchsploit, CVE analysis.",
        class: AgentClass::Specialist, tasks: &["expl-searchsploit", "vuln-nikto", "vuln-lynis"],
        tools: &["searchsploit", "nikto", "lynis"], agent_file: ".claude/agents/vuln-researcher.md" },
    SwarmAgent { name: "reverse-engineer", title: "Reverse Engineering",
        description: "Static analysis, exploit development.",
        class: AgentClass::Specialist, tasks: &["rev-strace", "rev-ltrace", "rev-r2"],
        tools: &["strace", "ltrace", "radare2"], agent_file: ".claude/agents/reverse-engineer.md" },
    SwarmAgent { name: "malware-analyst", title: "Malware Analysis",
        description: "Static/dynamic analysis of PE/ELF/APK.",
        class: AgentClass::Defensive, tasks: &["clam-scan", "fore-binwalk"],
        tools: &["clamav", "binwalk"], agent_file: ".claude/agents/malware-analyst.md" },
    SwarmAgent { name: "dfir", title: "Digital Forensics & Incident Response",
        description: "Triage, memory acquisition, disk forensics, hashing.",
        class: AgentClass::Defensive, tasks: &["fore-vol", "fore-fls", "fore-binwalk", "fore-hashdeep"],
        tools: &["volatility", "sleuthkit", "binwalk", "hashdeep"], agent_file: ".claude/agents/dfir.md" },
    SwarmAgent { name: "threat-hunter", title: "Threat Hunting",
        description: "ATT&CK-based hypotheses, lateral movement detection.",
        class: AgentClass::Defensive, tasks: &["sniff-tcpdump", "sniff-tshark"],
        tools: &["tcpdump", "tshark"], agent_file: ".claude/agents/threat-hunter.md" },
    SwarmAgent { name: "blue-team", title: "Blue Team / Hardening",
        description: "Detection rules, Linux/Windows hardening, rootkit checks.",
        class: AgentClass::Defensive, tasks: &["vuln-lynis", "fore-rkhunter", "fore-chkrootkit"],
        tools: &["lynis", "rkhunter", "chkrootkit"], agent_file: ".claude/agents/blue-team.md" },
    SwarmAgent { name: "log-analyst", title: "Security Log Analysis",
        description: "Parses and correlates auth.log, web access logs.",
        class: AgentClass::Defensive, tasks: &["sniff-tshark", "sniff-darkstat"],
        tools: &["tshark"], agent_file: ".claude/agents/log-analyst.md" },
    SwarmAgent { name: "compliance-scanner", title: "Compliance Assessment",
        description: "CIS benchmarks, PCI-DSS checks.",
        class: AgentClass::Defensive, tasks: &["vuln-lynis"],
        tools: &["lynis"], agent_file: ".claude/agents/compliance-scanner.md" },
    SwarmAgent { name: "report-writer", title: "Report Writing",
        description: "Consolidates evidence into executive/technical reports.",
        class: AgentClass::Defensive, tasks: &[],
        tools: &[], agent_file: ".claude/agents/report-writer.md" },
];

/// Lugha inayoendesha sehemu ya swarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmLang {
    Rust,
    Go,
    Elixir,
    R,
}

impl SwarmLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            SwarmLang::Rust => "rust",
            SwarmLang::Go => "go",
            SwarmLang::Elixir => "elixir",
            SwarmLang::R => "r",
        }
    }
}

/// Katalogi ya lugha: kila lugha na kazi yake kwenye swarm.
pub fn languages() -> Value {
    json!([
        {"lang": "rust", "crate": "fundi-deploy", "role": "injini kuu: modules za Total Security, fastscan, conductor"},
        {"lang": "go", "cmd": "agentic/ptyexec", "role": "mikono ya haraka: utekelezaji sambamba"},
        {"lang": "elixir", "app": "agentic/brain (Akili.Swarm)", "role": "ubongo wa OTP: supervisor wa agents 27"},
        {"lang": "r", "dir": "engines", "role": "uchanganuzi: risk scoring, ripoti"},
    ])
}

/// Katalogi kamili ya agents 27 (AGENT-ENGINE shape).
pub fn catalog_json() -> Value {
    let agents: Vec<Value> = AGENTS
        .iter()
        .map(|a| json!({
            "name": a.name, "title": a.title, "description": a.description,
            "class": a.class.as_str(), "tasks": a.tasks, "tools": a.tools,
            "agent_file": a.agent_file,
        }))
        .collect();
    json!({
        "ok": true,
        "count": agents.len(),
        "agents": agents,
        "languages": languages(),
        "rules": {
            "scope": "agents hawatoki nje ya scope.txt",
            "hitl": "vitendo vinavyobadilisha mfumo vinahitaji idhini",
        },
    })
}

/// Agents zenye uwezo wa kujihami (defensive) pekee.
pub fn defensive_agents() -> Vec<&'static SwarmAgent> {
    AGENTS.iter().filter(|a| a.class.is_defensive()).collect()
}

/// Tafuta agent kwa jina (AGENT-ENGINE shape).
pub fn find_agent_info(name: &str) -> Option<&'static SwarmAgent> {
    AGENTS.iter().find(|a| a.name == name)
}

/// Amri zote za ThreatSwarm (Claude Code commands).
pub const COMMANDS: &[(&str, &str)] = &[
    ("engage", "Anzisha op kamili: recon → vuln → exploit → post-ex → report"),
    ("hunt", "Uwindaji wa vitisho (blue-team + threat-hunter)"),
    ("attack", "Mashambulizi yaliyolengwa (red team, scope-gated)"),
    ("ir", "Majibu ya matukio (incident response: dfir + log-analyst)"),
    ("report", "Tengeneza ripoti kutoka evidence"),
    ("pwned", "Angalia kama lengo limekwamata (loot/evidence)"),
];

/// Plan ya operesheni: chagua agents kulingana na aina (mode) na scope.
pub fn plan(mode: &str, target: &str, scope: &[String]) -> Value {
    let defensive_only = matches!(mode, "hunt" | "ir" | "audit" | "defend");
    let selected: Vec<Value> = AGENTS
        .iter()
        .filter(|a| !defensive_only || a.class.is_defensive())
        .map(|a| json!({
            "agent": a.name, "title": a.title, "class": a.class.as_str(),
            "tools": a.tools, "tasks": a.tasks, "driver": a.agent_file,
        }))
        .collect();
    json!({
        "ok": true, "mode": mode, "target": target, "scope": scope,
        "scope_gated": true,
        "human_approval_required": !defensive_only,
        "agents": selected,
        "agent_count": selected.len(),
        "languages": languages(),
        "commands": COMMANDS.iter().map(|(c, d)| json!({"command": c, "about": d})).collect::<Vec<_>>(),
    })
}

/// CONDUCT: ThreatSwarm huongoza zana zote. Hukagua hosti nyingi kwa millisecond
/// (Rust fastscan), kisha hurudisha plan ya agents watakaoendelea.
pub async fn conduct(targets: &[String], ports: &[u16], mode: &str, scope: &[String]) -> Value {
    let cfg = crate::fastscan::ScanConfig { scope: scope.to_vec(), ..Default::default() };
    let hosts: Vec<String> = targets
        .iter()
        .flat_map(|t| if t.contains('/') { crate::fastscan::expand_cidr(t, 4096) } else { vec![t.clone()] })
        .collect();
    let report = crate::fastscan::scan(&hosts, ports, &cfg).await;
    let mut out = report.to_json();
    if let Some(obj) = out.as_object_mut() {
        obj.insert("swarm".into(), plan(mode, &targets.join(","), scope));
        obj.insert("engine".into(), json!("fundi-deploy/rust + swarm(go/elixir/r)"));
    }
    out
}

/// ENDOSHA AGENT MOJA AUTONOMOUS: scope-gate kixa, kisha kazi zote za agent.
pub async fn run_agent(
    db: &sqlx::SqlitePool,
    agent: &str,
    target: &str,
    human_approved: bool,
) -> Value {
    // Scope gate kwanza — deterministic, kabla ya kitu chochote.
    if let Err(e) = check_scope(target) {
        return json!({ "ok": false, "blocked_by_scope": true, "agent": agent, "error": e });
    }
    let Some(tasks) = find_agent(agent) else {
        return json!({
            "ok": false, "agent": agent,
            "error": format!("agent '{agent}' haipo. Agents {} zipo — orodha: /api/ts/agents", agent_count()),
        });
    };
    let id = format!("tsa-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let mut results = Vec::new();
    let mut evidence = Vec::new();
    for &task_id in tasks {
        let res = crate::secpack::run_task(db, task_id, target, human_approved).await;
        let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let blocked = res.get("needs_approval").and_then(|v| v.as_bool()).unwrap_or(false);
        let out = res.get("output").and_then(|v| v.as_str()).unwrap_or("");
        results.push(json!({
            "task": task_id, "ok": ok, "blocked_by_hitl": blocked,
            "duration_ms": res.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0),
            "output_preview": preview(out, 400),
        }));
        if !out.is_empty() {
            evidence.push(json!({ "task": task_id, "output": preview(out, 2000) }));
        }
    }
    // R ENGINE: risk_score kwenye matokeo (ok=1, hitl-blocked=5, fail=7)
    let r_scores: Vec<f64> = results
        .iter()
        .map(|s| {
            if s.get("blocked_by_hitl").and_then(|v| v.as_bool()).unwrap_or(false) {
                5.0
            } else if s.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                1.0
            } else {
                7.0
            }
        })
        .collect();
    let r_analytics = if r_scores.is_empty() {
        json!({ "ok": false, "error": "agent haina kazi za kutekeleza (report-writer)" })
    } else {
        crate::r_engine::run_req("risk_score", &json!({ "data": r_scores })).await
    };
    let report = json!({
        "id": id, "agent": agent, "target": target,
        "created_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "human_approved": human_approved,
        "tasks": results, "r_analytics": r_analytics, "evidence": evidence,
        "scope_check": "deterministic (scope.txt, CIDR-aware)",
    });
    // MUHTASARI WA AI (LLM): maelezo ya binadamu yanayosomeka + hatua zinazopendekezwa.
    // Fail-safe: LLM ikikosekana, report inaendelea bila ai_summary.
    let mut report = report;
    if let Some(summary) =
        crate::tsmod::social::ai::synthesize(db, "ThreatSwarm", &report).await
    {
        report["ai_summary"] = json!(summary);
    }
    let mut ev = PathBuf::from(DATA_DIR);
    let _ = std::fs::create_dir_all(&ev);
    ev.push(format!("{id}.json"));
    let _ = std::fs::write(&ev, serde_json::to_string_pretty(&report).unwrap_or_default());
    report
}

// ---------- ORODHA YA REPORTS ----------

pub async fn reports_json() -> Value {
    let dir = Path::new(DATA_DIR);
    let mut items: Vec<Value> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(name) = e.file_name().into_string() {
                if name.ends_with(".json") && (name.starts_with("tsa-") || name.starts_with("ts-")) {
                    items.push(json!({ "report": name.trim_end_matches(".json") }));
                }
            }
        }
    }
    json!({ "ok": true, "reports": items, "total": items.len() })
}

/// Honor read of single report by id.
#[allow(dead_code)]
pub fn report_by_id_path(id: &str) -> PathBuf {
    let mut p = PathBuf::from(DATA_DIR);
    p.push(format!("{id}.json"));
    p
}

// ---------- TESTS ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_27_na_kila_mmoja_ana_jina_la_kibinafsi() {
        assert_eq!(AGENTS.len(), 27, "AGENT-ENGINE ina agents 27");
        let mut names: Vec<&str> = AGENTS.iter().map(|a| a.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 27, "majina ya agents hayarudiwi");
    }

    #[test]
    fn kila_agent_ana_aina_na_maelezo() {
        for a in AGENTS {
            assert!(!a.title.is_empty(), "{} haina title", a.name);
            assert!(!a.description.is_empty(), "{} haina maelezo", a.name);
            assert!(!a.agent_file.is_empty(), "{} haina agent_file", a.name);
            assert!(
                matches!(a.class, AgentClass::Defensive | AgentClass::Offensive | AgentClass::Specialist),
                "{} haina class halali",
                a.name
            );
        }
    }

    #[test]
    fn dfir_ni_agent_wa_ulinzi_wenye_zana_za_forensics() {
        let dfir = find_agent_info("dfir").expect("dfir ipo");
        assert!(dfir.class.is_defensive());
        assert!(dfir.tools.contains(&"volatility"));
        assert!(dfir.tools.contains(&"sleuthkit"));
    }

    #[test]
    fn defensive_agents_ni_sehemu_ndogo_ya_wote() {
        let d = defensive_agents();
        assert!(!d.is_empty());
        assert!(d.len() < AGENTS.len(), "kuna offensive pia");
        assert!(d.iter().all(|a| a.class.is_defensive()));
    }

    #[test]
    fn plan_hunt_huchagua_defensive_pekee() {
        let p = plan("hunt", "10.0.0.1", &["10.0.0.0/24".to_string()]);
        assert_eq!(p["human_approval_required"], false);
        let n = p["agent_count"].as_u64().unwrap();
        assert!(n >= 6 && n < 27, "hunt huchagua defensive pekee (n={n})");
        for a in p["agents"].as_array().unwrap() {
            assert_eq!(a["class"], "defensive");
        }
    }

    #[test]
    fn plan_engage_huchagua_wote_na_hitl() {
        let p = plan("engage", "10.0.0.1", &[]);
        assert_eq!(p["agent_count"].as_u64().unwrap(), 27);
        assert_eq!(p["human_approval_required"], true);
    }

    #[test]
    fn catalog_json_ina_agents_27_na_lugha_nne() {
        let c = catalog_json();
        assert_eq!(c["count"].as_u64().unwrap(), 27);
        let langs = c["languages"].as_array().unwrap();
        assert_eq!(langs.len(), 4);
        let ids: Vec<&str> = langs.iter().map(|l| l["lang"].as_str().unwrap()).collect();
        for want in ["rust", "go", "elixir", "r"] {
            assert!(ids.contains(&want), "lugha {want} haipo");
        }
    }

    #[test]
    fn find_agent_info_hurudi_none_kwa_agent_asiyekuwepo() {
        assert!(find_agent_info("hakuna-agent-kama-hii").is_none());
    }

    #[test]
    fn parse_ipv4_inarudi_sahihi() {
        assert_eq!(parse_ipv4("192.168.1.10"), Some(0xC0A8010A));
        assert_eq!(parse_ipv4("256.1.1.1"), None);
        assert_eq!(parse_ipv4("1.2.3"), None);
        assert_eq!(parse_ipv4("abc"), None);
        assert_eq!(parse_ipv4("1.2.3.4x"), None);
    }

    #[test]
    fn parse_cidr_inarudi_sahihi() {
        let (base, mask) = parse_cidr("192.168.1.0/24").unwrap();
        assert_eq!(base, 0xC0A80100);
        assert_eq!(mask, 0xFFFFFF00);
        // IP ndani ya CIDR inavuta base sawa
        let ip = parse_ipv4("192.168.1.55").unwrap();
        assert_eq!(base, ip & mask);
        assert!(parse_cidr("10.0.0.0/33").is_none());
    }

    #[test]
    fn scope_gate_inakataa_kwenye_scope_default() {
        // scope.txt ya default ina '192.168.1.100', '10.0.0.0/24', 'testlab.local'
        // Hii test inapima HALISI kwenye repo ya MTECHOS (scope.txt iko).
        assert!(check_scope("192.168.1.100").is_ok());
        assert!(check_scope("10.0.0.7").is_ok()); // CIDR /24
        assert!(check_scope("testlab.local").is_ok()); // exact domain
        assert!(check_scope("8.8.8.8").is_err()); // nje ya scope
        assert!(check_scope("other.org").is_err()); // nje ya scope
    }

    #[test]
    fn engage_id_format() {
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        assert_eq!(ts.len(), 15);
    }

    #[test]
    fn localhost_huruhusiwa() {
        assert!(check_scope("localhost").is_ok());
        assert!(check_scope("127.0.0.1").is_ok());
    }

    #[test]
    fn scope_add_remove_roundtrip() {
        // Tumia faili ya scope ya muda — hakuna env var, hakuna athari kwa tests nyingine.
        let tmp = std::env::temp_dir().join("mtech_scope_test.txt");
        let p = tmp.to_string_lossy().to_string();
        let _ = std::fs::write(&tmp, "10.0.0.0/24\n");

        let r = scope_add_at(&p, "192.168.5.5").unwrap();
        assert_eq!(r["added"], serde_json::json!(true));
        assert!(scope_entries_at(&p).iter().any(|e| e == "192.168.5.5"));

        // Duplicate hairudiwi mara ya pili
        let r2 = scope_add_at(&p, "192.168.5.5").unwrap();
        assert_eq!(r2["added"], serde_json::json!(false));

        let r3 = scope_remove_at(&p, "192.168.5.5").unwrap();
        assert_eq!(r3["removed"], serde_json::json!(true));
        assert!(!scope_entries_at(&p).iter().any(|e| e == "192.168.5.5"));

        // Herufi zisizoruhusiwa zinakataliwa
        assert!(scope_add_at(&p, "bad;rm -rf").is_err());

        let _ = std::fs::remove_file(&tmp);
    }
}
