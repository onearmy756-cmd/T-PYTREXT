//! unified_engines.rs — MTECHOS UNIFIED ENGINE (nguvu moja).
//!
//! Huunganisha uwezo wa engines zilizobaki ndani ya backend mmoja:
//!
//!   * **OpenHands Agent Canvas** — UI/IDE (`engines/mbilinyi`, `/ide`).
//!   * **Skills** — engineering + zilizoingizwa (`data/skills/engineering`,
//!     `data/skills/imported`: OpenHands, Claude Code, Trae) + RADI.
//!   * **Fundi Server** — agenti halisi wa tools 70 (`fundi_agent_client`).
//!   * **Memory/governance** — masomo ya awali + zuio la amri hatari.
//!
//! Engines za biashara/vision (Magma, VisualClaw) zimeondolewa pamoja na
//! vendored folders zake; `see_image`/probes zake zinarudi `available:false`
//! wazi kama hazipo.
//!
//! Kanuni kuu: **hakuna kubuni**. Engine haipatikani → hurudisha `available:false`
//! wazi; haivunji loop na haidai kitu ambacho hakipo.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Uchunguzi wa paths
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CWD ni `server/agent`; repo root ni juu mara mbili.
    let from_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    if let Some(p) = from_manifest {
        if p.join("engines").is_dir() {
            return p;
        }
    }
    PathBuf::from("../..")
}

fn engines_dir() -> PathBuf {
    repo_root().join("engines")
}

/// `engines/visualclaw` (au `visualclaw`) — huheshimu env kwa majaribio.
fn visualclaw_root() -> PathBuf {
    std::env::var("VISUALCLAW_ROOT")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| engines_dir().join("visualclaw"))
}

fn magma_base() -> String {
    std::env::var("MAGMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8001".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn visualclaw_base() -> String {
    std::env::var("VISUALCLAW_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8800".to_string())
        .trim_end_matches('/')
        .to_string()
}

// ---------------------------------------------------------------------------
// Probing (haraka, TCP — hakuna adhabu kama engine haipo)
// ---------------------------------------------------------------------------

/// Kipimo cha TCP cha haraka. `timeout_ms` ndogo (default 150ms).
pub fn tcp_reachable(host_port: &str, timeout_ms: u64) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    let Ok(mut addrs) = host_port.to_socket_addrs() else {
        return false;
    };
    match addrs.next() {
        Some(addr) => TcpStream::connect_timeout(
            &addr,
            std::time::Duration::from_millis(timeout_ms),
        )
        .is_ok(),
        None => false,
    }
}

fn host_port(base: &str) -> String {
    base.trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string()
}

fn probe_base(base: &str) -> bool {
    tcp_reachable(&host_port(base), 250)
}

// ---------------------------------------------------------------------------
// Hali ya engines (status)
// ---------------------------------------------------------------------------

/// Hali ya kila engine: upatikaji halisi (probe) + ushahidi wa path/port.
pub fn status() -> Value {
    let oh = engines_dir().join("mbilinyi");
    let oh_build = oh.join("build").join("index.html");
    let vc = visualclaw_root();
    let magma = engines_dir().join("magma");
    // Magma/VisualClaw hazipo tena kwenye repo (IDE + cyber pekee). Hali
    // inarudi `available:false` wazi — hakuna kubuni; skills za SKILL.md
    // zinaendelea kuchukuliwa kutoka diski kama mtumiaji amezipa.
    let magma_ref = magma.join("README.md");
    let vc_skills = vc.join("visualclaw").join("skills_seed");

    let magma_up = probe_base(&magma_base());
    let vc_up = probe_base(&visualclaw_base());
    let fundi_up = tcp_reachable("127.0.0.1:8787", 250);

    json!({
        "ok": true,
        "engine": "MTECHOS Unified Engine",
        "note": "Nguvu moja — uwezo wa engines zote zilizovendored, bila kubuni.",
        "engines": {
            "mbilinyi": {
                "role": "UI / Agent Canvas IDE",
                "available": oh_build.is_file(),
                "serving": "/ide",
                "build": oh_build.is_file(),
                "root": oh.display().to_string(),
                "note": if oh_build.is_file() {
                    "Canvas imejengwa — /ide hutumikia SPA halisi."
                } else {
                    "Canvas haijajengwa — endesha scripts/ide-start.sh (huijenga)."
                },
            },
            "visualclaw": {
                "role": "skill injection + memory + governance",
                "available": vc_skills.is_dir(),
                "gateway_running": vc_up,
                "base_url": visualclaw_base(),
                "root": vc.display().to_string(),
                "note": "SKILL.md zake huingizwa kwenye prompt (marejeo ya skills; hakuna gateway ya Python).",
            },
            "magma": {
                "role": "vision / multimodal",
                "available": magma_ref.is_file(),
                "server_running": magma_up,
                "base_url": magma_base(),
                "root": magma.display().to_string(),
                "note": "Marejeo ya docs/assets; hakuna server ya Python. Tool `see_image` hutumia endpoint ya nje kama ipo.",
            },
            "fundi_server": {
                "role": "agenti wa tools (70)",
                "available": fundi_up,
                "base_url": "http://127.0.0.1:8787",
                "note": "Backend agent halisi (terminal/execute_code/filesystem).",
            },
        },
        "skills_sources": skills_sources_summary(),
    })
}

/// Lugha zinazotambulika (kutoka rejista ya `lang_verify`).
fn langs_supported() -> Vec<&'static str> {
    use crate::lang_verify::Lang::*;
    [Rust, Node, Deno, Bun, Dart, Flutter, Go, JavaMaven, JavaGradle,
     Dotnet, Ruby, Php, Swift, Kotlin, CMake, CMakeCpp, Make, Elixir, Haskell,
     Zig, Scala]
        .iter()
        .map(|l| l.name())
        .collect()
}

pub fn capabilities() -> Value {
    let magma_up = probe_base(&magma_base());
    let vc_up = probe_base(&visualclaw_base());
    // LLM ya kuona (LAN/cloud) hufanya kazi kama akiba ya Magma.
    let cfg = crate::ai_config::load();
    let vision_llm_up = probe_base(&cfg.url);
    let oh = engines_dir()
        .join("mbilinyi")
        .join("build")
        .join("index.html")
        .is_file();
    let skills = crate::code_skills::list();
    json!({
        "ok": true,
        "capabilities": [
            {"id":"agentic_loop","name":"Agentic ReAct (andika→endesha→jirekebishe)","available":true},
            {"id":"code_generation","name":"Uzalishaji wa kodi (lugha ZOTE)","available":true,
             "languages": langs_supported()},
            {"id":"multilang_verify","name":"Uthibitisho KALI kwa kila lugha (tests halisi)",
             "available":true, "languages": langs_supported()},
            {"id":"ide_canvas","name":"Agent Canvas IDE","available":oh},
            {"id":"skill_injection","name":"Skill injection (VisualClaw-style)","available":true,
             "skills": skills["count"].clone()},
            {"id":"memory","name":"Memory (masomo ya awali)","available":true},
            {"id":"governance","name":"Governance (constitution + cost)","available":true},
            {"id":"vision","name":"Vision (Magma / LLM ya kuona)","available":magma_up || vision_llm_up},
            {"id":"visualclaw_gateway","name":"VisualClaw gateway","available":vc_up},
            {"id":"fundi_agent","name":"Fundi agent (tools 70)","available":tcp_reachable("127.0.0.1:8787",250)},
        ],
    })
}

// ---------------------------------------------------------------------------
// Skills: vyanzo VYOTE vilivyopo (ndio malalamiko ya mtumiaji)
// ---------------------------------------------------------------------------

/// Skani SKILL.md za VisualClaw (`skills_seed/*/*/SKILL.md`) na urudishe
/// `(name, description, category, content)`.
pub fn visualclaw_skills() -> Vec<Value> {
    let seed = visualclaw_root()
        .join("visualclaw")
        .join("skills_seed");
    let mut out = Vec::new();
    let Ok(roots) = std::fs::read_dir(&seed) else {
        return out;
    };
    for group in roots.flatten() {
        let Ok(subs) = std::fs::read_dir(group.path()) else {
            continue;
        };
        for sub in subs.flatten() {
            let f = sub.path().join("SKILL.md");
            if let Some(skill) = parse_skill_md(&f) {
                out.push(skill);
            }
        }
    }
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    out
}

/// Chambua SKILL.md (frontmatter `---` kama VisualClaw).
pub fn parse_skill_md(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    if !raw.starts_with("---") {
        return None;
    }
    let end = raw[3..].find("\n---").map(|i| i + 3)?;
    let fm = &raw[3..end];
    let body = raw[end + 4..].trim().to_string();
    let mut name = String::new();
    let mut desc = String::new();
    let mut category = "general".to_string();
    for line in fm.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.trim(), v.trim());
            match k {
                "name" => name = v.to_string(),
                "description" => desc = v.to_string(),
                "category" => category = v.to_string(),
                _ => {}
            }
        }
    }
    if name.is_empty() || desc.is_empty() {
        return None;
    }
    Some(json!({
        "name": name, "description": desc, "category": category,
        "content": body, "source": "visualclaw",
    }))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

/// Ramani ya vyanzo vya skills (kwa `/api/engines/unified/status`).
fn skills_sources_summary() -> Value {
    let vc = visualclaw_skills().len();
    let eng = crate::code_skills::list();
    let imported = eng["count"].as_u64().unwrap_or(0);
    let radi = crate::radi::block();
    let prompt_sw = crate::data_io::data_dir()
        .join("code_rules")
        .join("AGENT_PROMPT_SW.md")
        .is_file();
    json!({
        "visualclaw_skill_md": vc,
        "engineering_and_imported_md": imported,
        "radi_edition": !radi.trim().is_empty(),
        "agent_prompt_sw": prompt_sw,
    })
}

// ---------------------------------------------------------------------------
// Block ya skills ZOTE zinazoingizwa kwenye prompt ya agentic
// ---------------------------------------------------------------------------

/// Huunganisha skills ZOTE: engineering (data-driven) + VisualClaw SKILL.md
/// zinazolingana + kanuni za Swahili codegen (`AGENT_PROMPT_SW.md`).
/// Hii ndiyo "nguvu moja" — agent haikosi kitu kilichopo kwenye repo.
pub fn unified_skills_block(kind: &str, idea: &str) -> String {
    let mut out = String::new();

    // 1) Senior engineering skills (data/skills/engineering).
    let eng = crate::code_skills::skills_for(kind, idea);
    if !eng.trim().is_empty() {
        out.push_str(&eng);
        out.push('\n');
    }

    // 2) Kanuni za Swahili codegen (data/code_rules/AGENT_PROMPT_SW.md) — zilikuwa hazitumiki.
    let sw = crate::data_io::data_dir()
        .join("code_rules")
        .join("AGENT_PROMPT_SW.md");
    if let Ok(body) = std::fs::read_to_string(&sw) {
        if !body.trim().is_empty() {
            out.push_str("=== KANUNI ZA KODI (KISWAHILI) ===\n");
            out.push_str(&body);
            out.push_str("\n\n");
        }
    }

    // 3) RADI — edition ya kasi ya umeme (data/skills/radi). Tabia ya coding
    //    inayoondoa hatua zisizo na thamani bila kupunguza ukweli/usalama.
    let radi = crate::radi::block();
    if !radi.trim().is_empty() {
        out.push_str("=== RADI (MWENDO WA KASI — fuata) ===\n");
        out.push_str(&radi);
        out.push('\n');
    }

    // 4) VisualClaw SKILL.md — chagua zile zinazolingana na idea (au za jumla).
    let skills = visualclaw_skills();
    if !skills.is_empty() {
        let hay = idea.to_lowercase();
        let picked: Vec<&Value> = skills
            .iter()
            .filter(|s| {
                let name = s["name"].as_str().unwrap_or("").to_lowercase();
                let desc = s["description"].as_str().unwrap_or("").to_lowercase();
                hay.split_whitespace()
                    .filter(|w| w.len() > 3)
                    .any(|w| name.contains(w) || desc.contains(w))
            })
            .take(4)
            .collect();
        if !picked.is_empty() {
            out.push_str("=== VISUALCLAW SKILLS (zinazolingana) ===\n");
            for s in picked {
                out.push_str(&format!(
                    "### {}\n{}\n\n",
                    s["name"].as_str().unwrap_or(""),
                    s["content"].as_str().unwrap_or("")
                ));
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Magma: vision tool (`see_image`)
// ---------------------------------------------------------------------------

/// Eleza picha kwa Magma server (`POST /predict`). Hurudisha `(text, ok)`.
/// Magma haipo → hujaribu LLM yenye uwezo wa kuona (LAN/cloud) kama akiba.
/// Zote zikikosekana → `ok=false` (hakuna kubuni).
pub async fn see_image(image_base64: &str, prompt: &str) -> (String, bool) {
    let base = magma_base();
    if probe_base(&base) {
        let body = json!({ "image": image_base64, "prompt": prompt });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        match client.post(format!("{base}/predict")).json(&body).send().await {
            Ok(r) if r.status().is_success() => {
                let v: Value = r.json().await.unwrap_or(Value::Null);
                let txt = v["response"]
                    .as_str()
                    .or_else(|| v["text"].as_str())
                    .or_else(|| v["answer"].as_str())
                    .unwrap_or("")
                    .to_string();
                if !txt.trim().is_empty() {
                    return (txt, true);
                }
            }
            Ok(r) => {
                eprintln!("Magma HTTP {} — najaribu LLM ya kuona", r.status());
            }
            Err(e) => {
                eprintln!("Magma error: {e} — najaribu LLM ya kuona");
            }
        }
    }
    // Akiba: LLM ya LAN/cloud yenye uwezo wa kuona (qwen2.5vl na wenzake).
    let cfg = crate::ai_config::load();
    let (out, ok) = crate::ai_config::describe_image(&cfg, image_base64, prompt).await;
    if ok {
        return (out, true);
    }
    (
        format!(
            "Hakuna injini ya kuona: Magma haipatikani kwenye {base}, na LLM ya kuona ({}) imefeli — {out}",
            cfg.model
        ),
        false,
    )
}

// ---------------------------------------------------------------------------
// Memory (VisualClaw-style): masomo ya awali yanayobaki kwenye disk
// ---------------------------------------------------------------------------

fn memory_path() -> PathBuf {
    crate::data_io::data_dir().join("agent").join("memory.jsonl")
}

/// Hifadhi somo la mradi uliokamilika (kwa loop ijayo).
pub fn remember(project: &str, kind: &str, ok: bool, summary: &str) {
    let rec = json!({
        "ts": chrono::Utc::now().timestamp(),
        "project": project, "kind": kind, "ok": ok, "summary": summary,
    });
    let p = memory_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "{rec}");
    }
}

/// Masomo yanayolingana na `idea` (yaliyofanikiwa hivi karibuni).
pub fn recall(idea: &str, limit: usize) -> Vec<Value> {
    let Ok(txt) = std::fs::read_to_string(memory_path()) else {
        return Vec::new();
    };
    let hay = idea.to_lowercase();
    let mut out: Vec<Value> = txt
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| {
            v["ok"].as_bool().unwrap_or(false)
                && hay
                    .split_whitespace()
                    .filter(|w| w.len() > 3)
                    .any(|w| v["project"].to_string().to_lowercase().contains(w))
        })
        .collect();
    out.reverse();
    out.truncate(limit);
    out
}

/// Block ya memory kwa prompt (masomo ya awali).
pub fn memory_block(idea: &str) -> String {
    let lessons = recall(idea, 3);
    if lessons.is_empty() {
        return String::new();
    }
    let mut s = String::from("=== MASOMO YA AWALI (matumizi) ===\n");
    for l in lessons {
        s.push_str(&format!(
            "- {} ({})\n",
            l["summary"].as_str().unwrap_or(""),
            l["project"].as_str().unwrap_or("")
        ));
    }
    s.push('\n');
    s
}

/// Governance: kanuni rahisi (VisualClaw constitution) — hazuii kazi isipokuwa
/// hatari za wazi. Hurudisha `(allowed, reason)`.
pub fn governance_check(command: &str) -> (bool, String) {
    let c = command.to_lowercase();
    let banned = [
        "rm -rf /",
        "rm -rf ~",
        ":(){",
        "mkfs",
        "dd if=/dev/zero",
        "> /dev/sda",
        "chmod -r 777 /",
    ];
    for b in banned {
        if c.contains(b) {
            return (false, format!("GOVERNANCE: amri hatari imezuiwa ({b})"));
        }
    }
    (true, String::new())
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// `GET /api/engines/unified/status`
pub async fn unified_status() -> axum::Json<Value> {
    axum::Json(status())
}

/// `GET /api/engines/unified/capabilities`
pub async fn unified_capabilities() -> axum::Json<Value> {
    axum::Json(capabilities())
}

/// `GET /api/engines/unified/skills`
pub async fn unified_skills() -> axum::Json<Value> {
    axum::Json(json!({
        "ok": true,
        "engineering": crate::code_skills::list(),
        "visualclaw": visualclaw_skills(),
        "radi": crate::radi::block().lines().count(),
        "sources": skills_sources_summary(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_skill_md_frontmatter() {
        let dir = std::env::temp_dir().join(format!("ue_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("SKILL.md");
        std::fs::write(
            &f,
            "---\nname: test-skill\ndescription: does a thing\ncategory: general\n---\n\nBody here.\n",
        )
        .unwrap();
        let s = parse_skill_md(&f).expect("skill lazima iparse");
        assert_eq!(s["name"], "test-skill");
        assert_eq!(s["category"], "general");
        assert!(s["content"].as_str().unwrap().contains("Body here"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn governance_blocks_dangerous() {
        assert!(!governance_check("rm -rf /").0);
        assert!(governance_check("cargo test").0);
    }

    #[test]
    fn status_has_all_engines() {
        let s = status();
        for k in ["mbilinyi", "visualclaw", "magma", "fundi_server"] {
            assert!(s["engines"][k].is_object(), "engine {k} lazima iwepo");
        }
    }

    #[test]
    fn capabilities_includes_core() {
        let c = capabilities();
        let ids: Vec<String> = c["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["id"].as_str().unwrap_or("").to_string())
            .collect();
        assert!(ids.contains(&"agentic_loop".to_string()));
        assert!(ids.contains(&"skill_injection".to_string()));
    }

    #[test]
    fn unified_skills_block_never_panics() {
        let b = unified_skills_block("rust-cli", "hesabu ya maneno");
        // Core skill inapaswa kuwepo kila wakati (kutoka code_skills).
        assert!(b.contains("Core Agentic Loop") || !b.is_empty());
    }

    #[test]
    fn unified_skills_block_ina_radi() {
        // REQ4: RADI (kasi ya umeme) LAZIMA iingizwe kwenye prompt ya coding,
        // sio kutajwa kwa jina pekee — block halisi ya data/skills/radi.
        let b = unified_skills_block("rust-cli", "hesabu");
        assert!(b.contains("RADI"), "RADI lazima iwepo kwenye block");
        assert!(b.contains("Mkataba wa kasi"),
                "block halisi ya RADI lazima iingizwe (si jina pekee)");
    }
}
