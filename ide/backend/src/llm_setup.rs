//! llm_setup.rs — UTUNZAJI WA LOCAL MODELS + LLAMA.CPP KWA AUTOMATIKI (bila training)
//!
//! Mtumiaji hupakua tu API key / kubonyeza kitufe — mfumo hujiseti yenyewe:
//!   - Ollama Cloud: API key → URL + model zote (free) + majaribio (ai_config)
//!   - Local: pakua GGUF (HuggingFace) + llama-server (llama.cpp GitHub) → washa → auto-configure
//!   - Chat: "pakua qwen vl 3b" / "pakua bundles" → download inaanza yenyewe (intent deterministic)
//!
//! Endpoint: /api/llm/{status,models,download,download/status,llamacpp/setup,start,stop,use}

use serde::Serialize;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

// ---------- KATALOGI YA MODEL ZA LOCAL (GGUF — mtandaoni, malipo hakuna) ----------

pub struct ModelDef {
    pub id: &'static str,
    pub name_sw: &'static str,
    pub repo: &'static str,
    pub prefer: &'static str, // sehemu ya jina la faili (mf. "q4_k_m")
    pub approx_mb: u64,
    pub desc: &'static str,
}

pub const MODELS: [ModelDef; 8] = [
    // ── FRONTIER CODER MODELS — miradi migumu (context kubwa, coder focused) ──
    // context 128K: inasoma kurasa mamia za kodi kwa wakati mmoja bila kusahau
    // sheria za syntax (inashirikiana na linter ya code_rules.rs self-check).
    ModelDef {
        id: "qwen25coder7b",
        name_sw: "Qwen2.5-Coder 7B (Frontier)",
        repo: "Qwen/Qwen2.5-Coder-7B-Instruct-GGUF",
        prefer: "q4_k_m",
        approx_mb: 4700,
        desc: "CODER — miradi migumu: context 128K, inasoma kurasa nyingi za kodi",
    },
    ModelDef {
        id: "deepseekcoder6b",
        name_sw: "DeepSeek-Coder 6.7B (Frontier)",
        repo: "TheBloke/deepseek-coder-6.7B-instruct-GGUF",
        prefer: "q4_k_m",
        approx_mb: 4100,
        desc: "CODER — reasoning ya code yenye nguvu, nzuri kwenye refactor kubwa",
    },
    // ── MODELS NDUGO ZA KAWAIDA (vision/general) ──
    ModelDef {
        id: "qwen25vl3b",
        name_sw: "Qwen2.5 VL 3B",
        repo: "Qwen/Qwen2.5-VL-3B-Instruct-GGUF",
        prefer: "q4_k_m",
        approx_mb: 2048,
        desc: "Maono + maandishi (vision) — model lengwa la MTECH OS",
    },
    ModelDef {
        id: "qwen34b",
        name_sw: "Qwen3 4B",
        repo: "Qwen/Qwen3-4B-GGUF",
        prefer: "q4",
        approx_mb: 2400,
        desc: "Maandishi — akili nzuri, inasoma Kiswahili vizuri",
    },
    ModelDef {
        id: "llama323b",
        name_sw: "Llama 3.2 3B",
        repo: "bartowski/Llama-3.2-3B-Instruct-GGUF",
        prefer: "q4_k_m",
        approx_mb: 2000,
        desc: "Maandishi — haraka sana hata kwenye PC dhaifu",
    },
    ModelDef {
        id: "phi35",
        name_sw: "Phi-3.5 Mini",
        repo: "bartowski/Phi-3.5-mini-instruct-GGUF",
        prefer: "q4_k_m",
        approx_mb: 2300,
        desc: "Maandishi — nzuri kwa code na akili yenye mantiki",
    },
    ModelDef {
        id: "gemma22b",
        name_sw: "Gemma 2 2B",
        repo: "bartowski/gemma-2-2b-it-GGUF",
        prefer: "q4_k_m",
        approx_mb: 1700,
        desc: "Maandishi — wastani, bidhaa ya Google (DeepMind)",
    },
    ModelDef {
        id: "r1qwen15b",
        name_sw: "DeepSeek R1 1.5B",
        repo: "bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF",
        prefer: "q4_k_m",
        approx_mb: 1100,
        desc: "Maandishi — reasoning ndogo, inafanya kazi kwenye PC yoyote",
    },
];

pub fn find_model(id: &str) -> Option<&'static ModelDef> {
    MODELS.iter().find(|m| m.id == id)
}

// ---------- JOB (download inayoendelea — UI inafuatilia kwa /api/llm/download/status) ----------

#[derive(Clone, Debug, Serialize)]
pub struct Job {
    pub kind: String, // "model" | "llamacpp"
    pub id: String,
    pub name: String,
    pub state: String, // starting | running | done | error
    pub total: u64,
    pub done: u64,
    pub error: String,
    pub saved_to: String,
}

fn job_slot() -> &'static Mutex<Option<Job>> {
    static S: OnceLock<Mutex<Option<Job>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

fn pid_slot() -> &'static Mutex<Option<u32>> {
    static P: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
    P.get_or_init(|| Mutex::new(None))
}

fn job_lock() -> std::sync::MutexGuard<'static, Option<Job>> {
    job_slot().lock().unwrap_or_else(|e| e.into_inner())
}

fn job_update(f: impl FnOnce(&mut Job)) {
    if let Some(j) = job_lock().as_mut() {
        f(j);
    }
}

pub fn job_json() -> serde_json::Value {
    match job_lock().as_ref() {
        Some(j) => serde_json::to_value(j).unwrap_or(serde_json::Value::Null),
        None => serde_json::json!({ "state": "idle" }),
    }
}

// ---------- NJIA (dir) ----------

fn base() -> PathBuf {
    let dir = std::env::var("FUNDI_DATA").unwrap_or_else(|_| "data".into());
    PathBuf::from(dir).join("llm")
}

fn models_dir() -> PathBuf {
    base().join("models")
}

fn bin_dir() -> PathBuf {
    base().join("bin")
}

fn llama_exe() -> PathBuf {
    bin_dir().join("llama-server.exe")
}

// ---------- ORODHA + HALI ----------

pub fn models_list() -> Vec<serde_json::Value> {
    MODELS
        .iter()
        .map(|m| {
            let path = models_dir().join(format!("{}.gguf", m.id));
            let present = path.exists();
            let size_mb = if present {
                std::fs::metadata(&path)
                    .map(|md| md.len() / 1_048_576)
                    .unwrap_or(0)
            } else {
                0
            };
            serde_json::json!({
                "id": m.id, "name_sw": m.name_sw, "approx_mb": m.approx_mb,
                "desc": m.desc, "repo": m.repo, "present": present, "size_mb": size_mb
            })
        })
        .collect()
}

pub fn models_json() -> serde_json::Value {
    serde_json::json!({ "ok": true, "models": models_list(), "job": job_json() })
}

async fn probe(url: &str) -> bool {
    reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn status_json(db: &SqlitePool) -> serde_json::Value {
    let cfg = crate::ai_config::load_with_db(db).await;
    let (h8081, h11434) = tokio::join!(
        probe("http://127.0.0.1:8081/health"),
        probe("http://127.0.0.1:11434/api/version")
    );
    let pid = pid_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or(0);
    let engine = crate::engine_api::global();
    serde_json::json!({
        "ok": true,
        "llamacpp": { "exe": llama_exe().exists(), "server_running": h8081, "pid": pid },
        "ollama_local": h11434,
        "engine": {
            "name": "fundi-engine",
            "loaded": engine.is_loaded(),
            "note_sw": "Injini YETU — GGUF moja kwa moja, pure Rust (bila vLLM/Python).",
        },
        "active": { "url": cfg.url, "model": cfg.model },
        "api_key_set": crate::ai_config::ai_key().is_some(),
        "catalog": crate::data_io::recommended(),
        "models": models_list(),
        "job": job_json(),
        "data_dir": base().display().to_string(),
    })
}

// ---------- KUPAKUA (HuggingFace + GitHub) ----------

pub fn start_download_kind(kind: &str, id: &str) -> Result<serde_json::Value, String> {
    let name = match kind {
        "model" => {
            let d = find_model(id).ok_or("Model haipo katika orodha")?;
            d.name_sw.to_string()
        }
        "llamacpp" => "llama.cpp (llama-server — Windows CPU)".to_string(),
        _ => return Err("Aina isiyojulikana".into()),
    };
    {
        let mut g = job_lock();
        if let Some(j) = g.as_ref() {
            if j.state == "starting" || j.state == "running" {
                return Err(format!(
                    "Kuna download inayoendelea: {} ({})",
                    j.name, j.state
                ));
            }
        }
        *g = Some(Job {
            kind: kind.to_string(),
            id: id.to_string(),
            name,
            state: "starting".into(),
            total: 0,
            done: 0,
            error: String::new(),
            saved_to: String::new(),
        });
    }
    let kind2 = kind.to_string();
    let id2 = id.to_string();
    tokio::spawn(async move {
        let res = match kind2.as_str() {
            "model" => run_model_download(&id2).await,
            _ => run_llamacpp_setup().await,
        };
        if let Err(e) = res {
            job_update(|j| {
                j.state = "error".into();
                j.error = e;
            });
        }
    });
    Ok(job_json())
}

/// Pata URL halisi ya faili la GGUF kutoka HuggingFace (HF API — jina halihitaji kujulikana).
async fn hf_resolve(repo: &str, prefer: &str) -> Result<String, String> {
    let url = format!("https://huggingface.co/api/models/{repo}?blobs=true");
    let v = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::USER_AGENT, "MTECH-OS/1.1")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("HuggingFace: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Hafungui repo ({e}) — labda inahitaji ruhusa"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("JSON: {e}"))?;
    let sibs = v
        .get("siblings")
        .and_then(|a| a.as_array())
        .ok_or("Hakuna faili kwenye repo hii")?;
    let mut first_gguf = String::new();
    let mut preferred = String::new();
    for s in sibs {
        let n = s.get("rfilename").and_then(|x| x.as_str()).unwrap_or("");
        if !n.ends_with(".gguf") {
            continue;
        }
        if first_gguf.is_empty() {
            first_gguf = n.to_string();
        }
        if preferred.is_empty() && n.to_lowercase().contains(&prefer.to_lowercase()) {
            preferred = n.to_string();
        }
    }
    let chosen = if !preferred.is_empty() {
        preferred
    } else if !first_gguf.is_empty() {
        first_gguf
    } else {
        return Err("Hakuna .gguf kwenye repo hii".into());
    };
    Ok(format!(
        "https://huggingface.co/{repo}/resolve/main/{chosen}"
    ))
}

/// Pakua faili moja kwenye diski (streaming — inasasisha job kila chunk).
async fn download_to(url: &str, dest: &Path) -> Result<(), String> {
    let mut resp = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "MTECH-OS/1.1")
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Hakuna muunganiko: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Server ilitoa {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    job_update(|j| {
        j.total = total;
        j.state = "running".into();
    });
    if let Some(p) = dest.parent() {
        tokio::fs::create_dir_all(p)
            .await
            .map_err(|e| format!("Kutengeneza dir: {e}"))?;
    }
    let tmp = dest.with_extension("part");
    let mut f = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("Kufungua faili: {e}"))?;
    use tokio::io::AsyncWriteExt;
    let mut written = 0u64;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                f.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Kuandika diski: {e}"))?;
                written += chunk.len() as u64;
                let w = written;
                job_update(|j| j.done = w);
            }
            Ok(None) => break,
            Err(e) => {
                let _ = f.flush().await;
                return Err(format!("Muunganiko ukatika: {e}"));
            }
        }
    }
    f.flush().await.map_err(|e| e.to_string())?;
    drop(f);
    tokio::fs::rename(&tmp, dest)
        .await
        .map_err(|e| format!("Kuhifadhi: {e}"))?;
    Ok(())
}

async fn run_model_download(id: &str) -> Result<(), String> {
    let def = find_model(id).ok_or("Model haipo")?;
    job_update(|j| {
        j.state = "running".into();
        j.name = def.name_sw.into();
    });
    let url = hf_resolve(def.repo, def.prefer).await?;
    job_update(|j| {
        j.name = format!("{} — {}", def.name_sw, def.repo);
    });
    let dest = models_dir().join(format!("{id}.gguf"));
    download_to(&url, &dest).await?;
    job_update(|j| {
        j.state = "done".into();
        j.saved_to = dest.display().to_string();
    });
    Ok(())
}

async fn run_llamacpp_setup() -> Result<(), String> {
    job_update(|j| {
        j.state = "running".into();
        j.name = "llama.cpp — kutafuta toleo la sasa...".into();
    });
    let rel = reqwest::Client::new()
        .get("https://api.github.com/repos/ggml-org/llama.cpp/releases/latest")
        .header(reqwest::header::USER_AGENT, "MTECH-OS/1.1")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("GitHub: {e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub: {e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("JSON: {e}"))?;
    let assets = rel
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or("Hakuna assets kwenye release")?;
    let mut zip_url = String::new();
    let mut zip_name = String::new();
    for a in assets {
        let n = a.get("name").and_then(|x| x.as_str()).unwrap_or("");
        if n.ends_with(".zip") && n.contains("win-cpu-x64") {
            zip_url = a
                .get("browser_download_url")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            zip_name = n.to_string();
            break;
        }
    }
    if zip_url.is_empty() {
        return Err(
            "Hakuna build ya Windows CPU kwenye release ya sasa (jaribu kuipakua mwenyewe)".into(),
        );
    }
    job_update(|j| j.name = format!("llama.cpp {zip_name}"));
    let zip_path = base().join("llama-win.zip");
    download_to(&zip_url, &zip_path).await?;
    job_update(|j| {
        j.state = "running".into();
        j.name = "Inafungua llama.cpp...".into();
    });
    // Funkua kwa PowerShell Expand-Archive (hakuna crate ya zip inayohitajika)
    let ps = if Path::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe").exists()
    {
        "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
    } else {
        "powershell"
    };
    let cmd = format!(
        "Expand-Archive -LiteralPath '{z}' -DestinationPath '{d}' -Force",
        z = zip_path.display(),
        d = base().display()
    );
    let out = tokio::process::Command::new(ps)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &cmd,
        ])
        .output()
        .await
        .map_err(|e| format!("PowerShell: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Imeshindwa kufungua ZIP: {}",
            String::from_utf8_lossy(&out.stderr)
                .chars()
                .take(200)
                .collect::<String>()
        ));
    }
    let _ = std::fs::remove_file(&zip_path); // okoa nafasi ya diski
    let exe_src = find_file(&base(), "llama-server.exe")
        .ok_or("llama-server.exe haipatikani baada ya kufungua")?;
    let bin = bin_dir();
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let exe_dst = bin.join("llama-server.exe");
    std::fs::copy(&exe_src, &exe_dst).map_err(|e| format!("Kunakili exe: {e}"))?;
    // DLL zilizo karibu na exe
    if let Some(parent) = exe_src.parent() {
        if let Ok(rd) = std::fs::read_dir(parent) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("dll") {
                    let _ = std::fs::copy(&p, bin.join(p.file_name().unwrap_or_default()));
                }
            }
        }
    }
    job_update(|j| {
        j.state = "done".into();
        j.saved_to = exe_dst.display().to_string();
    });
    Ok(())
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_file() && p.file_name().and_then(|f| f.to_str()) == Some(name) {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(f) = find_file(&p, name) {
                return Some(f);
            }
        }
    }
    None
}

// ---------- KUENDESHA / KUZIMA LLAMA-SERVER + AUTO-CONFIGURE ----------

pub async fn start_server(id: &str) -> Result<serde_json::Value, String> {
    find_model(id).ok_or("Model haipo")?;
    let model_path = models_dir().join(format!("{id}.gguf"));
    if !model_path.exists() {
        return Err("Model haijapakuliwa bado — bonyeza Pakua kwanza".into());
    }
    let exe = llama_exe();
    if !exe.exists() {
        return Err("llama-server haipo — bonyeza Sakinisha llama.cpp kwanza".into());
    }
    if probe("http://127.0.0.1:8081/health").await {
        return Ok(serde_json::json!({
            "ok": true, "already": true,
            "note": "llama-server tayari inaendeshwa (:8081)"
        }));
    }
    let bin = exe.parent().unwrap().to_path_buf();
    let mut child = tokio::process::Command::new(&exe)
        .args([
            "-m",
            model_path.to_str().unwrap_or(""),
            "--port",
            "8081",
            "--host",
            "127.0.0.1",
            "-ngl",
            "0",
            "-c",
            "4096",
            "--jinja",
        ])
        .current_dir(&bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Imeshindwa kuanzisha llama-server: {e}"))?;
    let pid = child.id().unwrap_or(0);
    *pid_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(pid);
    // Subiri iwe ready (model 3B inapakia diski → RAM — sekunde 10-30)
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(750)).await;
        if probe("http://127.0.0.1:8081/health").await {
            return Ok(serde_json::json!({
                "ok": true, "pid": pid, "health": "ready",
                "note": "llama-server IMEJAZIWA kwenye :8081"
            }));
        }
    }
    Ok(serde_json::json!({
        "ok": true, "pid": pid, "health": "cold",
        "note": "Imeanzishwa — model bado inapakia; jaribu tena baada ya sekunde chache"
    }))
}

pub fn stop_server() -> serde_json::Value {
    let pid = pid_slot().lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .output();
        serde_json::json!({ "ok": true, "killed": pid })
    } else {
        serde_json::json!({ "ok": false, "error": "Hakuna server iliyorekodiwa kutoka MTECH OS" })
    }
}

/// Weka model hii kama AI kuu ya MTECH OS (llama.cpp :8081) — skills ni za mfumo, hakuna training.
pub async fn use_model(db: &SqlitePool, id: &str) -> Result<serde_json::Value, String> {
    find_model(id).ok_or("Model haipo")?;
    crate::ai_config::set_custom(
        db,
        &crate::ai_config::CustomAi {
            url: "http://127.0.0.1:8081".into(),
            model: id.to_string(),
        },
    )
    .await?;
    Ok(serde_json::json!({
        "ok": true,
        "url": "http://127.0.0.1:8081",
        "model": id,
        "note": "MTECH OS sasa inatumia model hii — skills/brain/vipengele vyote ni vya mfumo (hakuna training)"
    }))
}

// ---------- CHAT INTENTS: mtu aandika chat → kitendo kinaanza yenyewe ----------

fn reply_sw(question: &str, answer: String) -> crate::chat::ChatReply {
    crate::chat::ChatReply {
        question: question.to_string(),
        answer_sw: answer,
        answer_translated: String::new(),
        language: "sw".to_string(),
        source: "action".to_string(),
        confidence: 1.0,
        references: vec![],
        agent_session_id: None,
        agent_trace_id: None,
        agent_used: false,
    }
}

/// Rudisha ChatReply kama swali ni KITENDO (pakua/washa/simamisha) — deterministic,
/// hufanya kazi hata bila LLM (offline). Hakuna → None (aende kwenye LLM kawaida).
pub async fn chat_intent(question: &str, db: &SqlitePool) -> Option<crate::chat::ChatReply> {
    // SOMA faili la data/ — AI inapata folder LOTE la data ("soma data/ai/models.json")
    {
        let q = question.trim();
        let ql = q.to_lowercase();
        for vb in ["soma ", "fungua ", "onyesha "] {
            if let Some(rest) = ql.strip_prefix(vb) {
                let p = (&q[vb.len()..]).trim();
                let pl = p.to_lowercase();
                let looks_path = pl.contains('/')
                    || pl.ends_with(".json")
                    || pl.ends_with(".md")
                    || pl.ends_with(".txt")
                    || pl.starts_with("data");
                let _ = rest;
                if looks_path {
                    return match crate::data_io::read_file(p) {
                        Ok((txt, rel)) => {
                            let kb = (txt.len() + 1023) / 1024;
                            let n_ch = txt.chars().count();
                            let mut body: String = txt.chars().take(1800).collect();
                            if n_ch > 1800 {
                                body.push_str(&format!(
                                    "\n... (imekatwa — faili ni {n_ch} herufi jumla)"
                                ));
                            }
                            Some(reply_sw(
                                question,
                                format!("FAILI: data/{rel} ({kb} KB)\n{body}"),
                            ))
                        }
                        Err(e) => Some(reply_sw(question, format!("{e} (uliomba: {p})"))),
                    };
                }
                break; // "soma hii..." si path — aende LLM kawaida
            }
        }
    }
    let low = question.to_lowercase();
    let hit = |ws: &[&str]| ws.iter().any(|w| low.contains(w));
    let verbs: &[&str] = &["pakua", "download", "tua ", "sakinisha"];

    if hit(verbs) {
        let ans = if hit(&["qwen", "vl", "vision"]) {
            match start_download_kind("model", "qwen25vl3b") {
                Ok(_) => "Nimeanza KUPAKUA Qwen2.5 VL 3B (≈2GB) kutoka HuggingFace kwenye server yako. Fuatilia uendelezi kwenye AI card → sehemu ya LOCAL MODELS; itakapomalizika bonyeza \"Washa + Weka\".".into(),
                Err(e) => format!("Haiwezi kuanza kupakua: {e}"),
            }
        } else if hit(&["llama", "llamacpp", "llama.cpp"]) {
            match start_download_kind("llamacpp", "llamacpp") {
                Ok(_) => "Nimeanza KUPAKUA llama.cpp (llama-server, Windows CPU ≈35MB) na kuisakinisha yenyewe. Fuatilia kwenye AI card.".into(),
                Err(e) => format!("Haiwezi kuanza: {e}"),
            }
        } else if hit(&["bundle"]) {
            match crate::bundles::fetch_and_store(db).await {
                Ok((n, src)) => format!("Bundles {n} ZIMEPAKULIWA na kuhifadhiwa kwenye server (chanzo: {src}). Fungua ukurasa wa Bundles utaona zote."),
                Err(e) => format!("Imeshindwa kupakua bundles: {e}"),
            }
        } else if hit(&["model", "modeli"]) {
            let list = models_list()
                .iter()
                .map(|m| {
                    format!(
                        "- {} (≈{} MB){}",
                        m["name_sw"],
                        m["approx_mb"],
                        if m["present"] == true {
                            " ✓ imepakuliwa"
                        } else {
                            ""
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("Andika jina, mfano: \"pakua qwen vl 3b\"\nOrodha ya model za local:\n{list}")
        } else {
            return None;
        };
        return Some(reply_sw(question, ans));
    }

    if hit(&["washa", "start", "anzisha"])
        && hit(&["llama", "server", "local model", "model ya ndani"])
    {
        let first = models_list().into_iter().find(|m| m["present"] == true)?;
        let id = first["id"].as_str().unwrap_or("").to_string();
        let nm = first["name_sw"].as_str().unwrap_or("").to_string();
        tokio::spawn(async move {
            let _ = start_server(&id).await;
        });
        return Some(reply_sw(
            question,
            format!("Nimeanzisha llama-server na model {nm} — itakuwa tayari baada ya sekunde chache (AI card → LOCAL MODELS)."),
        ));
    }

    if hit(&["simamisha", "simiza", "acha", "stop", "zima"]) && hit(&["llama", "server", "model"]) {
        let r = stop_server();
        return Some(reply_sw(
            question,
            format!(
                "llama-server: {}",
                if r["ok"] == true {
                    "imezimwa".to_string()
                } else {
                    r["error"].to_string()
                }
            ),
        ));
    }

    None
}
