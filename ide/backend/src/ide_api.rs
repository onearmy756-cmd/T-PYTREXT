// ide_api.rs — MBILINYITECH IDE: Backend halisi ya IDE (Zed-style Web IDE).
//
// APIs zote za IDE zinatumia filesystem HALISI (workspace = repo root /data/projects)
// na terminal halisi kupitia agent ya fundi-server (tool "terminal" — backend halisi).
//
//   GET  /api/ide/tree            → file tree ya workspace
//   GET  /api/ide/file?path=...   → fungua file (maudhui + metadata)
//   POST /api/ide/file            → hifadhi file (content mpya)
//   POST /api/ide/search          → grep ya workspace (regex halisi)
//   POST /api/ide/terminal        → amri halisi ya shell kwenye workspace
//   POST /api/ide/r               → R script HALISI (Rscript) + matokeo ya stats
//
// SALAMA: kila path inathibitishiwa kuwa ndani ya workspace (path traversal
// badilika: "..", /etc, /root — zinakataliwa). Shell runner inatumia agent layer
// sawa na chat (kuna security policy ya ApprovalDecision kwenye TerminalHandler).
use axum::Json;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Workspace ya IDE: folder ya PROJECT ZA MTUMIAJI (siyo system folders za
/// MTECHOS!). Kwenye container: `/data/ide` — `/data` ni Fly volume ya kudumu
/// (inabaki hata deploy mpya). Kwa dev local: `~/mbilinyi-ide` au `.env` ya
/// `MBILINYI_IDE_ROOT`. IDE inafungua TU folda hii — user anaamua project lake.
pub fn ide_workspace_root() -> PathBuf {
    if let Ok(d) = std::env::var("MBILINYI_IDE_ROOT") {
        if Path::new(&d).exists() {
            return PathBuf::from(d);
        }
    }
    // Container: /data/ide (Fly volume, inadumu). Kama haipo (dev local),
    // HOME/mbilinyi-ide au cwd.
    let volume = PathBuf::from("/data/ide");
    if volume.parent().map(|p| p.exists()).unwrap_or(false) {
        // angalau volume yenyewe: create_dir_all inashindikana kama /data exists
        let _ = std::fs::create_dir_all(&volume);
        return volume;
    }
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home).join("mbilinyi-ide");
        let _ = std::fs::create_dir_all(&h);
        return h;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// SAFEGUARD: path lazima iwe ndani ya workspace. Inarudisha jina linalojulikana
/// kwenda kwenye filesystem au None kama nje.
pub(crate) fn safe_path(rel: &str) -> Option<PathBuf> {
    safe_path_in_root(&ide_workspace_root(), rel)
}

/// SAFEGUARD halisi dhidi ya root MAALUM (hotpatch/timetravel) — logic ile ile
/// (chandulizo kimoja: kila caller inaita hii, hakuna duplicate).
///
/// Hukubali AINA ZOTE MBILI za path: relatif (`src/app.js`) na KAMILI ndani ya
/// root (`/data/ide/src/app.js`). Awali `trim_start_matches('/')` ilikata slash
/// kwa UPOFU, hivyo path kamili ilijoin tena: `/data/ide` → `/data/ide/data/ide`.
/// Modeli hupeana path kamili kiasili (ndiyo inayoonekana kwenye prompt), hivyo
/// hilo lilivunja `run_tests`/`build_run`/`code_audit` kila mara.
pub(crate) fn safe_path_in_root(root: &Path, rel: &str) -> Option<PathBuf> {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return Some(root.to_path_buf());
    }
    if trimmed.split(['/', '\\']).any(|c| c == "..") {
        return None;
    }
    // Path KAMILI iliyo ndani ya root: itumie kama ilivyo (baada ya kuthibitisha).
    let candidate = Path::new(trimmed);
    if candidate.is_absolute() {
        if candidate.starts_with(root) {
            return Some(candidate.to_path_buf());
        }
        return None; // kamili lakini nje ya workspace
    }
    let full = root.join(trimmed.trim_start_matches(['/', '\\']));
    if !full.starts_with(root) {
        return None;
    }
    Some(full)
}

/// Hujenga PATH kamili ya zana za mtumiaji (`cargo`, `rustup`, `node`, n.k.).
///
/// Huduma hii huendeshwa kama root kupitia `sg docker`/systemd, ambapo `HOME`
/// na PATH ya mtumiaji hazipo. Bila hii `rustup`/`cargo` hazipatikani na
/// `run_tests`/`build_run` hushindwa kwa miradi ya Rust.
pub(crate) fn tool_path_prefix() -> String {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    let mut homes: Vec<std::path::PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        homes.push(std::path::PathBuf::from(home));
    }
    // `sg docker -c`/systemd hufuta HOME; tafuta maeneo yanayojulikana.
    for h in ["/home/openhands", "/root", "/home/user"] {
        homes.push(std::path::PathBuf::from(h));
    }
    let mut rustup_home: Option<String> = None;
    for h in &homes {
        let bin = h.join(".cargo").join("bin");
        if bin.is_dir() {
            dirs.push(bin);
        }
        // rustup huhitaji RUSTUP_HOME: bila yake hushindwa "no default is configured".
        if rustup_home.is_none() && h.join(".rustup").join("toolchains").is_dir() {
            rustup_home = Some(h.join(".rustup").display().to_string());
        }
    }
    if dirs.is_empty() {
        return String::new();
    }
    let mut prefix = String::new();
    if let Some(rh) = rustup_home {
        prefix.push_str(&format!("export RUSTUP_HOME=\"{rh}\"; "));
    }
    let existing: Vec<String> = dirs.into_iter().map(|d| d.display().to_string()).collect();
    prefix.push_str(&format!("export PATH=\"{}:$PATH\"; ", existing.join(":")));
    prefix
}

// ---------- TREE ----------
/// File tree (kina 3 levels default, kikomo 500 entries). Inasoma filesystem HALISI.
pub async fn ide_tree() -> Json<Value> {
    let root = ide_workspace_root();
    let mut entries = Vec::new();
    walk_dir(&root, &root, &mut entries, 0, 3, &mut 0);
    entries.sort_by(|a, b| {
        let a_is_dir = a["type"].as_str() == Some("dir");
        let b_is_dir = b["type"].as_str() == Some("dir");
        b_is_dir.cmp(&a_is_dir) // dirs kwanza
    });
    Json(json!({
        "ok": true,
        "root": root.display().to_string(),
        "entries": entries,
    }))
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    out: &mut Vec<Value>,
    depth: usize,
    max_depth: usize,
    count: &mut usize,
) {
    if depth > max_depth || *count > 400 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        *count += 1;
        if *count > 500 { return; }
        let path = e.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        // tunepuza: .git, target, node_modules, .venv (makubwa, siyo source)
        if matches!(name.as_str(), ".git" | "target" | "node_modules" | ".venv" | "__pycache__") {
            continue;
        }
        let is_dir = path.is_dir();
        let rel = path.strip_prefix(root).map(|p| p.to_string_lossy().to_string()).unwrap_or(name.clone());
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        out.push(json!({
            "name": name,
            "path": rel,
            "type": if is_dir { "dir" } else { "file" },
            "size": size,
        }));
        if is_dir && depth < max_depth && *count <= 400 {
            walk_dir(root, &path, out, depth + 1, max_depth, count);
        }
    }
}

// ---------- READ ----------
pub async fn ide_file_read(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let Some(path_str) = q.get("path") else {
        return Json(json!({ "ok": false, "error": "path ni lazima (?path=...)" }));
    };
    let Some(full) = safe_path(path_str) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace — imekataliwa" }));
    };
    if !full.is_file() {
        return Json(json!({ "ok": false, "error": "sio file" }));
    }
    let Ok(content) = std::fs::read_to_string(&full) else {
        return Json(json!({ "ok": false, "error": "haikuweza kusomwa (binary ama encoding)" }));
    };
    // kikomo 512KB kwenye editor
    let truncated = content.len() > 512 * 1024;
    let content = if truncated { content.chars().take(400_000).collect::<String>() } else { content };
    Json(json!({
        "ok": true,
        "path": path_str,
        "content": content,
        "truncated": truncated,
        "syntax": syntax_for(&full),
        "size": full.metadata().map(|m| m.len()).unwrap_or(0),
    }))
}

fn syntax_for(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "rs" => "rust",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "ex" | "exs" => "elixir",
        "go" => "go",
        "r" | "R" => "r",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "md" => "markdown",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "sh" => "shell",
        _ => "plaintext",
    }
}

// ---------- WRITE ----------
pub async fn ide_file_write(Json(r): Json<Value>) -> Json<Value> {
    let Some(path_str) = r["path"].as_str() else {
        return Json(json!({ "ok": false, "error": "path ni lazima" }));
    };
    let Some(content) = r["content"].as_str() else {
        return Json(json!({ "ok": false, "error": "content ni lazima" }));
    };
    let Some(full) = safe_path(path_str) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace — imekataliwa" }));
    };
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bak: Option<String> = std::fs::read_to_string(&full).ok();
    let Ok(_) = std::fs::write(&full, content) else {
        // Container: /app inaweza kuwa read-only kwa runtime user — huhifadhi kwenye
        // /tmp/mbilinyi_ide workspace ya muda (HALISI — path halisi inarudishwa).
        let tmp_dir = std::env::temp_dir().join("mbilinyi_ide");
        let rel = full
            .strip_prefix(ide_workspace_root())
            .unwrap_or(&full)
            .to_string_lossy()
            .to_string();
        let tmp_path = tmp_dir.join(&rel);
        if let Some(p) = tmp_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        match std::fs::write(&tmp_path, content) {
            Ok(_) => {
                // LAZIMA: isiripoti `ok:true` — faili haiko kwenye workspace ya
                // mtumiaji (Monaco/file tree haitaiona). Kuripoti mafanikio hapa
                // huwafanya agent na mtumiaji kudhani kazi imefanyika.
                return Json(json!({
                    "ok": false,
                    "error": format!(
                        "workspace ya IDE ({}) haiandikiki — faili imehifadhiwa /tmp pekee, \
                         haitaonekana kwenye file tree. Rekebisha ruhusa za workspace.",
                        ide_workspace_root().display()
                    ),
                    "path": format!("/tmp/mbilinyi_ide/{}", rel),
                    "size": content.len(),
                    "backup_existed": bak.is_some(),
                }));
            }
            Err(e2) => {
                return Json(json!({
                    "ok": false,
                    "error": format!("haikuweza kuhifadhiwa (workspace + /tmp zote zimeshindikana: {})", e2),
                }));
            }
        }
    };
    // TIME-TRAVEL kila mwandiko (video ya kimantiki): best-effort.
    {
        let ok = crate::code_rules::lint(content, path_str)["ok"].as_bool().unwrap_or(false);
        let _ = crate::timetravel::record(&ide_workspace_root(), path_str, content, ok);
    }
    Json(json!({
        "ok": true,
        "path": path_str,
        "size": content.len(),
        "backup_existed": bak.is_some(),
    }))
}

// ---------- SEARCH ----------
pub async fn ide_search(Json(r): Json<Value>) -> Json<Value> {
    let Some(query) = r["query"].as_str() else {
        return Json(json!({ "ok": false, "error": "query ni lazima" }));
    };
    if query.is_empty() {
        return Json(json!({ "ok": false, "error": "query tupu" }));
    }
    let root = ide_workspace_root();
    let mut hits: Vec<Value> = Vec::new();
    let mut files_visited = 0usize;
    let mut hit_count = 0usize;
    search_dir(&root, &root, query.to_lowercase(), &mut hits, &mut files_visited, &mut hit_count);
    Json(json!({
        "ok": true,
        "query": query,
        "files_searched": files_visited,
        "total": hit_count,
        "results": hits.into_iter().take(100).collect::<Vec<_>>(),
    }))
}

fn search_dir(
    root: &Path,
    dir: &Path,
    q: String,
    out: &mut Vec<Value>,
    files_seen: &mut usize,
    hit_count: &mut usize,
) {
    if out.len() > 150 || *hit_count > 300 { return; }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let path = e.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if matches!(name.as_str(), ".git" | "target" | "node_modules" | ".venv" | "__pycache__") {
            continue;
        }
        if path.is_dir() {
            search_dir(root, &path, q.clone(), out, files_seen, hit_count);
            continue;
        }
        *files_seen += 1;
        if path.metadata().map(|m| m.len() > 512 * 1024).unwrap_or(true) { continue; }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let rel = path.strip_prefix(root).map(|p| p.to_string_lossy().to_string()).unwrap_or(name.clone());
        for (lineno, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&q) && !line.trim().is_empty() {
                *hit_count += 1;
                out.push(json!({
                    "path": rel,
                    "line": lineno + 1,
                    "text": line.trim().chars().take(200).collect::<String>(),
                }));
                if out.len() >= 150 || *hit_count > 300 { return; }
            }
        }
    }
}

/// ---------- MKDIR (folder mpya) ----------
/// UI ya Explorer ina button "➕ Folder" — inatengeneza folder halisi ndani ya
/// workspace. `parents:true` hufanya `a/b/c` yote. Kama ipo, hairudii kosa.
pub async fn ide_mkdir(Json(r): Json<Value>) -> Json<Value> {
    let Some(path_str) = r["path"].as_str() else {
        return Json(json!({ "ok": false, "error": "path ni lazima" }));
    };
    let Some(full) = safe_path(path_str) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace — imekataliwa" }));
    };
    if full.exists() {
        return Json(json!({ "ok": false, "error": "ipo tayari" }));
    }
    match std::fs::create_dir_all(&full) {
        Ok(_) => Json(json!({ "ok": true, "path": path_str, "kind": "dir" })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// ---------- RENAME / MOVE ----------
pub async fn ide_rename(Json(r): Json<Value>) -> Json<Value> {
    let Some(from) = r["from"].as_str() else {
        return Json(json!({ "ok": false, "error": "from ni lazima" }));
    };
    let Some(to) = r["to"].as_str() else {
        return Json(json!({ "ok": false, "error": "to ni lazima" }));
    };
    let (Some(src), Some(dst)) = (safe_path(from), safe_path(to)) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace — imekataliwa" }));
    };
    if !src.exists() {
        return Json(json!({ "ok": false, "error": "chanzo hakipo" }));
    }
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::rename(&src, &dst) {
        Ok(_) => Json(json!({ "ok": true, "from": from, "to": to })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// ---------- LANGUAGES (orodha + detect) ----------
/// Orodha ya lugha zinazotumika kwenye Monaco (id za `basic-languages`),
/// pamoja na extension → id ramani kwa "detect automatically".
pub fn languages_catalog() -> Value {
    let pairs: &[(&str, &str)] = &[
        ("rs", "rust"), ("ex", "elixir"), ("exs", "elixir"), ("js", "javascript"), ("mjs", "javascript"),
        ("cjs", "javascript"), ("ts", "typescript"), ("tsx", "typescript"), ("jsx", "javascript"),
        ("go", "go"), ("java", "java"), ("kt", "kotlin"), ("swift", "swift"), ("c", "c"),
        ("h", "c"), ("cpp", "cpp"), ("cc", "cpp"), ("hpp", "cpp"), ("cs", "csharp"),
        ("rb", "ruby"), ("php", "php"), ("ex", "elixir"), ("exs", "elixir"), ("r", "r"),
        ("sql", "sql"), ("sh", "shell"), ("bash", "shell"), ("zsh", "shell"), ("ps1", "powershell"),
        ("html", "html"), ("htm", "html"), ("css", "css"), ("scss", "scss"), ("less", "less"),
        ("json", "json"), ("yaml", "yaml"), ("yml", "yaml"), ("toml", "ini"), ("ini", "ini"),
        ("md", "markdown"), ("mdx", "mdx"), ("xml", "xml"), ("svg", "xml"), ("dockerfile", "dockerfile"),
        ("lua", "lua"), ("pl", "perl"), ("dart", "dart"), ("scala", "scala"), ("hs", "haskell"),
        ("clj", "clojure"), ("graphql", "graphql"), ("proto", "protobuf"), ("tf", "hcl"),
        ("vue", "html"), ("svelte", "html"),
    ];
    let langs: Vec<Value> = [
        "rust","javascript","typescript","go","java","kotlin","swift","c","cpp",
        "csharp","ruby","php","elixir","r","sql","shell","powershell","html","css","scss",
        "less","json","yaml","ini","markdown","mdx","xml","dockerfile","lua","perl","dart",
        "scala","haskell","clojure","graphql","protobuf","hcl","bat","azcli","bicep","pascal",
        "julia","objective-c","fsharp","vb","csp","coffee","cypher","pgsql","redis","tcl",
    ]
    .into_iter()
    .map(|s| json!(s))
    .collect();
    let ext: serde_json::Map<String, Value> = pairs
        .iter()
        .map(|(e, l)| (e.to_string(), json!(l)))
        .collect();
    json!({
        "ok": true,
        "languages": langs,
        "by_extension": ext,
        "filename_map": { "Dockerfile": "dockerfile", "Makefile": "shell", ".gitignore": "shell" },
    })
}

/// Tambua lugha kutoka jina la file (kwa UI "detect automatically").
pub fn detect_language(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name {
        "Dockerfile" => return "dockerfile".into(),
        "Makefile" => return "shell".into(),
        _ => {}
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let map = [
        ("rs", "rust"), ("ex", "elixir"), ("exs", "elixir"), ("js", "javascript"), ("mjs", "javascript"),
        ("cjs", "javascript"), ("ts", "typescript"), ("tsx", "typescript"), ("jsx", "javascript"),
        ("go", "go"), ("java", "java"), ("kt", "kotlin"), ("swift", "swift"), ("c", "c"),
        ("h", "c"), ("cpp", "cpp"), ("cc", "cpp"), ("hpp", "cpp"), ("cs", "csharp"),
        ("rb", "ruby"), ("php", "php"), ("ex", "elixir"), ("exs", "elixir"), ("r", "r"),
        ("sql", "sql"), ("sh", "shell"), ("bash", "shell"), ("html", "html"), ("htm", "html"),
        ("css", "css"), ("scss", "scss"), ("json", "json"), ("yaml", "yaml"), ("yml", "yaml"),
        ("md", "markdown"), ("xml", "xml"), ("toml", "ini"), ("ini", "ini"), ("lua", "lua"),
        ("dart", "dart"), ("scala", "scala"), ("pl", "perl"), ("graphql", "graphql"),
    ];
    for (e, l) in map {
        if ext == e {
            return l.into();
        }
    }
    "plaintext".into()
}

// ---------- DELETE (file au folder recursively) ----------
/// UI ya Pick Project Panel una button "✕" kwa kila entry (kama Zed: Delete
/// Project Entry). Folder inafutwa kwa kutumia `std::fs::remove_dir_all`.
pub async fn ide_delete(Json(r): Json<Value>) -> Json<Value> {
    let Some(path_str) = r["path"].as_str() else {
        return Json(json!({ "ok": false, "error": "path ni lazima" }));
    };
    let Some(full) = safe_path(path_str) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace — imekataliwa" }));
    };
    if !full.exists() {
        return Json(json!({ "ok": false, "error": "haipo" }));
    }
    let res = if full.is_dir() {
        std::fs::remove_dir_all(&full)
    } else {
        std::fs::remove_file(&full)
    };
    match res {
        Ok(_) => Json(json!({ "ok": true, "path": path_str })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}
/// Zana halisi ya shell inatumika: agent layer's terminal tool (inayo security
/// approval policy kwenye TerminalHandler). Kama agent haipatikani, inarudisha
/// ujumbe wa wazi — hakuna fallback ya uwongo.
pub async fn ide_terminal(
    axum::extract::State(s): axum::extract::State<crate::AppState>,
    Json(r): Json<Value>,
) -> Json<Value> {
    let Some(cmd) = r["command"].as_str() else {
        return Json(json!({ "ok": false, "error": "command ni lazima" }));
    };
    let workdir = r["workdir"].as_str().unwrap_or("").to_string();
    let Some(agent) = s.agent.as_ref() else {
        return Json(json!({ "ok": false, "error": "agent layer imezumzishwa (FUNDI_AGENT_ENABLED=0) — terminal haipatikani" }));
    };
    let mut params = json!({ "command": cmd, "timeout": 30000 });
    if !workdir.is_empty() {
        params["workdir"] = json!(workdir);
    }
    let body = json!({ "tool": "terminal", "params": params });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => Json(json!({
            "ok": true,
            "command": cmd,
            "output": v["output"].as_str().unwrap_or("").to_string(),
            "tool_result": v,
        })),
        Err(e) => Json(json!({ "ok": false, "command": cmd, "error": e.to_string() })),
    }
}

// ---------- R ANALYSIS (HALISI — Rscript kwenye workspace) ----------
/// R script ya kuchambua data halisi: inaweka file ya CSV/JSON inapo, inaendesha
/// Rscript kwenye workspace. Matokeo: stats halisi + matokeo ya kipimo.
pub async fn ide_r_analyze(Json(r): Json<Value>) -> Json<Value> {
    let code = r["code"].as_str().unwrap_or("").trim().to_string();
    let data_path = r["data_path"].as_str().unwrap_or("").trim().to_string();
    if code.is_empty() {
        return Json(json!({ "ok": false, "error": "code ya R ni lazima" }));
    }
    // data kama inapasa: kwenye workspace tu
    let data_arg = match safe_path(&data_path) {
        Some(p) if p.is_file() => p.display().to_string(),
        _ => String::new(),
    };
    // script kwenye temp (Rscript inasomea kando)
    let script_path = std::env::temp_dir().join(format!(
        "mbilinyitech_ide_r_{}.R",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    if let Err(e) = std::fs::write(&script_path, &code) {
        return Json(json!({ "ok": false, "error": format!("haikuweza andikwa: {}", e) }));
    }
    let mut cmd = tokio::process::Command::new("Rscript");
    cmd.arg(&script_path);
    if !data_arg.is_empty() {
        cmd.arg(&data_arg);
    }
    cmd.current_dir(ide_workspace_root());
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        cmd.output(),
    ).await;
    let res = match output {
        Err(_) => json!({ "ok": false, "error": "Rscript imesimama (timeout 60s)" }),
        Ok(Err(e)) => json!({ "ok": false, "error": format!("Rscript haipatikani: {} — sakinisha r-base-core", e) }),
        Ok(Ok(out)) => json!({
            "stdout": String::from_utf8_lossy(&out.stdout),
            "stderr": String::from_utf8_lossy(&out.stderr),
            "success": out.status.success(),
        }),
    };
    let _ = std::fs::remove_file(&script_path);
    Json(json!({
        "ok": true,
        "engine": "R (Rscript, base stats)",
        "result": res,
    }))
}
