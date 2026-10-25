//! canvas_ext.rs — daraja la MBILINYI Agent Canvas (OpenHands SPA) kwa backend
//! ya MTECHOS (fundi-deploy).
//!
//! Canvas (React SPA kwenye `/ide`) ni UI PEKEE ya MTECHOS. Protocol ya
//! agent-server (`/api/settings`, `/api/profiles`, `/api/agent-profiles`,
//! `/api/meta-profiles`, `/api/llm/*`, `/server_info`, `/alive`, `/ready`)
//! hupelekwa kwa **agent-server HALISI** kupitia daraja la Rust
//! (`agent_server_proxy.rs`).
//!
//! Module hii HAIRUDII njia hizo (hakuna "shadow" handlers) — hivyo Settings na
//! Canvas husoma data HALISI kutoka agent-server (settings zilizohifadhiwa,
//! profiles, schemas, providers), si data ya kubuni. Kama tungeziweka hapa,
//! Rust ingezishika KABLA ya daraja (route mahususi hushinda catch-all) na
//! kuzirudisha kwa thamani za kubuni.
//!
//! Hutoa PEKEE kile agent-server HAItoi: extensions za ndani za MTECHOS
//! (Forge/Super/SDLC/Ollama + Monaco IDE) kwenye `/api/canvas-extensions/*`,
//! pamoja na kuingiza session key ya Canvas kwenye HTML (`inject_session_key`).

use axum::{
    extract::Path as AxPath,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::AppState;

/// Roots zinazoshikilia extensions halisi (kila moja ina canvas-extension.json).
/// MTECHOS inasafirisha extensions tatu:
///   - `engines/mbilinyi-extension`  — MTECHOS IDE (Forge/Super/SDLC/Ollama)
///   - `platforms/mbilinyitech-ide`   — MBILINYITECH IDE (Monaco + Autopilot)
///   - `mts-native/ui/mts-canvas`     — Mbilinyi Tech Solutions (tovuti kamili
///     inayohudumiwa na Rust core: Rust + Go + Elixir + R)
/// Hutafuta kutoka CWD na pia kutoka eneo la binary (repo root), kwa sababu
/// server huenda ikaendeshwa kutoka `server/agent` (CWD tofauti).
fn extension_roots() -> Vec<PathBuf> {
    let rels: [PathBuf; 3] = [
        std::path::Path::new("engines").join("mbilinyi-extension"),
        std::path::Path::new("platforms").join("mbilinyitech-ide"),
        std::path::Path::new("mts-native").join("ui").join("mts-canvas"),
    ];
    let mut cands: Vec<PathBuf> = rels.to_vec();
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            for rel in &rels {
                cands.push(d.join(rel));
            }
            dir = d.parent();
        }
    }
    // CWD na eneo la binary hupata folda ILE ILE kwa njia tofauti; pia nakala
    // za repo zinaweza kuwepo. Canvas hutambua extension kwa `name`, hivyo
    // dedupe kwa jina — kwa njia hiyo extension hujitokeza mara moja.
    let mut seen: Vec<String> = Vec::new();
    cands
        .into_iter()
        .filter(|c| c.join("canvas-extension.json").exists())
        .filter(|c| {
            let name = std::fs::read_to_string(c.join("canvas-extension.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
                .unwrap_or_else(|| c.to_string_lossy().to_string());
            if seen.contains(&name) {
                false
            } else {
                seen.push(name);
                true
            }
        })
        .collect()
}

/// Directory ya extension fulani kwa jina (kutoka roots zote).
fn extension_dir(name: &str) -> Option<PathBuf> {
    extension_roots().into_iter().find(|dir| {
        std::fs::read_to_string(dir.join("canvas-extension.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .as_deref()
            == Some(name)
    })
}

/// Roots ZOTE za extensions (halisi) — hutumika na orodha ya plugins.
pub fn plugin_roots() -> Vec<PathBuf> {
    extension_roots()
}

/// Session key ya agent (`__agent__`, Admin Mkuu) iliyoundwa kwenye startup.
/// Canvas huipata kupitia `window.__AGENT_CANVAS_SESSION_API_KEY__` ili
/// ijiunganishe kiotomatiki na backend ya MTECHOS (hakuna onboarding modal).
static AGENT_KEY: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Weka session key ya agent (huitwa mara moja kwenye startup).
pub fn set_agent_key(token: String) {
    let _ = AGENT_KEY.set(token);
}

/// Rudisha session key ya agent, ikiwa imewekwa.
pub fn agent_key() -> Option<&'static str> {
    AGENT_KEY.get().map(|s| s.as_str())
}

/// Chagua `apiBase` inayoingizwa kwenye HTML. Bila override, ni kiambishi cha
/// proxy cha same-origin (`/mts`) — SI loopback, ambayo ingevunja kwa mgeni.
fn resolve_api_base(override_base: Option<String>) -> String {
    override_base
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| crate::mts_proxy::PROXY_PREFIX.to_string())
}

/// Ingiza `window.__AGENT_CANVAS_SESSION_API_KEY__` kwenye HTML ya Canvas
/// (kabla ya `</head>` au mwanzo wa `<body>`). Canvas hutumia hii kupata
/// default local backend bila kuomba mtumiaji key.
///
/// **USALAMA**: key hii ni token ya Admin Mkuu (`__agent__`). Kuiweka kwenye
/// HTML ya umma humaanisha mgeni yeyote anaweza kuiiba kwa `curl /ide` na
/// kupata madaraka kamili. Kwa hivyo uingizaji ni **opt-in**:
/// `AGENT_CANVAS_ALLOW_LAN_SESSION_KEY=true` (tumia PEKEE ukiwa umefichua
/// port kwa loopback). Bila hivyo, HTML hupata alama
/// `__AGENT_CANVAS_AUTH_REQUIRED__ = true` na Canvas huomba login halisi
/// (username/password → token kwenye `sessions`).
///
/// Pia huingiza `window.__MTS__.apiBase` — anwani ya Rust core ya MTECHOS
/// (mts-native). Extension ya Mbilinyi Tech Solutions (Canvas App) huzungumza
/// NA core kupitia hii.
///
/// LAZIMA iwe **same-origin** (`/mts`, si `http://127.0.0.1:8788`): kwa mgeni
/// loopback ni mashine YAKE, na cookie ya core (`SameSite=Lax`) haitumwi kwenye
/// ombi la cross-site. Hivyo browser hutumia origin inayohudumia ukurasa, na
/// `mts_proxy` hupeleka `/mts/*` kwa core. `MTS_CORE_URL` inabaki ya server-side
/// (proxy na core huipata), na `MTS_PUBLIC_API_BASE` huruhusu override kamili
/// kwa hali ambapo core imefichuliwa moja kwa moja.
pub fn inject_session_key(html: &str) -> String {
    let allow_lan = std::env::var("AGENT_CANVAS_ALLOW_LAN_SESSION_KEY")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut snippet = session_snippet(agent_key(), allow_lan);
    let api_base = resolve_api_base(std::env::var("MTS_PUBLIC_API_BASE").ok());
    snippet.push_str(&format!(
        "<script>window.__MTS__={{apiBase:{}}};</script>",
        serde_json::to_string(&api_base).unwrap_or_else(|_| "\"\"".to_string())
    ));
    if snippet.is_empty() {
        return html.to_string();
    }
    if let Some(pos) = html.find("</head>") {
        let mut out = String::with_capacity(html.len() + snippet.len());
        out.push_str(&html[..pos]);
        out.push_str(&snippet);
        out.push_str(&html[pos..]);
        out
    } else {
        format!("{snippet}{html}")
    }
}

/// Snippet ya usalama PEKEE (bila HTML) — rahisi kupima.
///
/// `allow_lan=false` (default) → HAIINGIZI key; huweka alama ya login badala
/// yake. `allow_lan=true` → huingiza key (hali ya loopback pekee).
fn session_snippet(key: Option<&str>, allow_lan: bool) -> String {
    match key {
        Some(k) if allow_lan => format!(
            "<script>window.__AGENT_CANVAS_SESSION_API_KEY__={};</script>",
            serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string())
        ),
        // Key ipo lakini HAIINGIZWI: Canvas LAZIMA iombe login halisi.
        // `__MTS_ADMIN_LOGIN__` hufanya root ionyeshe skrini ya login MOJA KWA
        // MOJA (bila onboarding ya "Add a backend" ambayo ni mwisho kwa UI hii).
        Some(_) => concat!(
            "<script>window.__AGENT_CANVAS_AUTH_REQUIRED__=true;",
            "window.__MTS_ADMIN_LOGIN__=true;</script>"
        )
        .to_string(),
        None => String::new(),
    }
}

/// Pakua canvas-extension.json ya extension fulani (manifest).
fn read_manifest(name: &str) -> Option<Value> {
    // Extension inaweza kuwa ya ndani (dir) — tunathibitisha jina linalingana.
    let dir = extension_dir(name)?;
    let text = std::fs::read_to_string(dir.join("canvas-extension.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    if v.get("name").and_then(|n| n.as_str()) == Some(name) {
        Some(v)
    } else {
        None
    }
}

// ═══════════════════ Canvas extensions (MTECHOS-PEKE) ═══════════════════
//
// Agent-server haina extensions za MTECHOS (hurudisha `{"canvas_extensions":[]}`),
// hivyo njia hizi hukaa Rust. Njia ZOTE nyingine za protocol (`/api/settings`,
// `/api/profiles`, `/server_info`, ...) hupelekwa kwa agent-server HALISI.

/// `GET /api/canvas-extensions/installed` — orodha ya extensions zilizowekwa.
/// MTECHOS inasafirisha extensions tatu za ndani: `mtechos-ide` (Forge/Super/
/// SDLC/Ollama), `mbilinyitech-ide` (Monaco editor + Autopilot) na
/// `mbilinyi-tech` (tovuti ya Mbilinyi Tech, Rust + Go + Elixir + R).
/// Orodha hupatikana kwa kutembea roots ZOTE — kuongeza extension hapa
/// hakutaji kubadilisha jina lolote la mkono.
async fn installed() -> Json<Value> {
    let mut list: Vec<Value> = Vec::new();
    for dir in extension_roots() {
        let Ok(text) = std::fs::read_to_string(dir.join("canvas-extension.json")) else {
            continue;
        };
        let Ok(m) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(name) = m.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        list.push(json!({
            "name": name,
            "version": m.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0"),
            "description": m.get("description").cloned().unwrap_or(Value::Null),
            "enabled": true,
            "source": format!("local:{}", dir.to_string_lossy()),
            "requested_ref": Value::Null,
            "resolved_ref": Value::Null,
            "repo_path": Value::Null,
            "installed_at": "1970-01-01T00:00:00Z",
            "install_path": dir.to_string_lossy(),
            "manifest": m,
        }));
    }
    Json(json!({"canvas_extensions": list}))
}

/// `GET /api/canvas-extensions/installed/:name/bundle` — entrypoint ESM.
/// Canvas huipakia kama module halisi (blob import).
async fn bundle(AxPath(name): AxPath<String>) -> Response {
    let Some(dir) = extension_dir(&name) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"detail": format!("extension `{name}` haipatikani")})),
        )
            .into_response();
    };
    let entry = dir.join("extension.js");
    match std::fs::read_to_string(&entry) {
        Ok(src) => (
            [(axum::http::header::CONTENT_TYPE, "text/javascript")],
            src,
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"detail": format!("entrypoint imeshindwa kusomwa: {e}")})),
        )
            .into_response(),
    }
}

/// `GET /api/canvas-extensions/installed/:name/icon` — SVG icon (inline).
async fn icon(AxPath(name): AxPath<String>) -> Response {
    if extension_dir(&name).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    // Rangi ya brand ya MBILINYI (ember) — icons za sidebar zinafuata theme,
    // sio bluu ya ajabu. Tumia currentColor-style fallback ya theme.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="7" fill="#ff6b2c"/><path d="M9 22V10h4l3 6 3-6h4v12h-3v-7l-3 6h-2l-3-6v7z" fill="#1e0f0a"/></svg>"##;
    (
        [(axum::http::header::CONTENT_TYPE, "image/svg+xml")],
        svg,
    )
        .into_response()
}

/// `POST /api/canvas-extensions/install` — kusajili extension ya ndani.
/// Extensions za MTECHOS ni za ndani (zipo kwenye repo); tunarudisha ya kwanza
/// (`mtechos-ide`) ili ombi lisiwe na madhara (idempotent).
async fn install() -> Json<Value> {
    let name = "mtechos-ide";
    let m = read_manifest(name).unwrap_or(Value::Null);
    let dir = extension_dir(name).unwrap_or_default();
    Json(json!({
        "name": name,
        "version": m.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0"),
        "enabled": true,
        "source": format!("local:{}", dir.to_string_lossy()),
        "install_path": dir.to_string_lossy(),
        "installed_at": "1970-01-01T00:00:00Z",
        "manifest": m,
    }))
}

/// `PATCH /api/canvas-extensions/installed/:name` — wezesha/kuzima.
async fn set_enabled(AxPath(name): AxPath<String>, body: Option<Json<Value>>) -> Json<Value> {
    let enabled = body
        .and_then(|Json(b)| b.get("enabled").and_then(|v| v.as_bool()))
        .unwrap_or(true);
    Json(json!({"name": name, "enabled": enabled}))
}

/// `DELETE /api/canvas-extensions/installed/:name` — ondoa.
async fn uninstall(AxPath(name): AxPath<String>) -> Json<Value> {
    Json(json!({"message": format!("extension `{name}` imeondolewa")}))
}

/// Routes za Canvas extensions (merge kwenye router kuu). Njia za protocol ya
/// agent-server HAZIPO hapa — zinapita kwenye `agent_server_proxy`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/canvas-extensions/installed", get(installed))
        .route("/api/canvas-extensions/install", axum::routing::post(install))
        .route(
            "/api/canvas-extensions/installed/:name",
            axum::routing::patch(set_enabled).delete(uninstall),
        )
        .route("/api/canvas-extensions/installed/:name/bundle", get(bundle))
        .route("/api/canvas-extensions/installed/:name/icon", get(icon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_ya_mtechos_ide_inasomwa() {
        // Kama repo iko, manifest halisi inasomwa (name + pages).
        let m = read_manifest("mtechos-ide");
        if let Some(m) = m {
            assert_eq!(m["name"], "mtechos-ide");
            let pages = &m["contributes"]["pages"];
            assert!(pages.is_array(), "pages lazima iwe array");
        }
    }

    #[test]
    fn manifest_ya_jina_lisilopo_ni_none() {
        assert!(read_manifest("hakuna-extension-hii").is_none());
    }

    #[test]
    fn api_base_bila_override_ni_same_origin_proxy() {
        // Bila override, Canvas LAZIMA itumie `/mts` (same-origin), SI loopback
        // `127.0.0.1` — ambayo kwa mgeni ni mashine YAKE na huvunja kila wito.
        let base = resolve_api_base(None);
        assert_eq!(base, crate::mts_proxy::PROXY_PREFIX);
        assert!(base.starts_with('/'), "apiBase lazima iwe njia ya same-origin");
        assert!(!base.contains("127.0.0.1"));
        assert!(!base.contains("localhost"));
    }

    #[test]
    fn api_base_override_husafishwa_na_kutumika() {
        let base = resolve_api_base(Some("  https://api.mbilinyitech.co.tz/  ".into()));
        assert_eq!(base, "https://api.mbilinyitech.co.tz");
        // Tupu/nafasi pekee → kurudi kwenye proxy ya same-origin.
        assert_eq!(resolve_api_base(Some("   ".into())), crate::mts_proxy::PROXY_PREFIX);
        assert_eq!(resolve_api_base(Some(String::new())), crate::mts_proxy::PROXY_PREFIX);
    }

    #[test]
    fn session_snippet_bila_opt_in_haingizi_key() {
        // USALAMA: bila AGENT_CANVAS_ALLOW_LAN_SESSION_KEY, key HAIPASWI
        // kuonekana kwenye HTML. Badala yake, alama ya login.
        let s = session_snippet(Some("siri-kubwa"), false);
        assert!(!s.contains("siri-kubwa"), "key imevuja kwenye HTML!");
        assert!(s.contains("__AGENT_CANVAS_AUTH_REQUIRED__"));
        // Bila hii, root ingeonyesha onboarding ("Add a backend") — mwisho
        // usio na matokeo kwa UI ya MTECHOS iliyohudumiwa.
        assert!(s.contains("__MTS_ADMIN_LOGIN__"));
    }

    #[test]
    fn session_snippet_na_opt_in_haingizi_alama_ya_login() {
        let s = session_snippet(Some("siri-kubwa"), true);
        assert!(!s.contains("__MTS_ADMIN_LOGIN__"));
        assert!(!s.contains("__AGENT_CANVAS_AUTH_REQUIRED__"));
    }

    #[test]
    fn session_snippet_na_opt_in_huingiza_key() {
        let s = session_snippet(Some("siri-kubwa"), true);
        assert!(s.contains("__AGENT_CANVAS_SESSION_API_KEY__"));
        assert!(s.contains("siri-kubwa"));
        assert!(!s.contains("__AGENT_CANVAS_AUTH_REQUIRED__"));
    }

    #[test]
    fn session_snippet_bila_key_ni_tupu() {
        assert!(session_snippet(None, true).is_empty());
        assert!(session_snippet(None, false).is_empty());
    }

    #[test]
    fn inject_bila_opt_in_haingizi_key_kwenye_html() {
        // Hali ya default (hakuna env opt-in kwenye CI) — key LAZIMA isivuje.
        let html = "<html><head><title>x</title></head><body></body></html>";
        let out = inject_session_key(html);
        // apiBase huingizwa kila mara (same-origin), lakini key haipo.
        assert!(out.contains("window.__MTS__"));
        if let Some(k) = agent_key() {
            assert!(!out.contains(k), "token ya Admin Mkuu imevuja kwenye HTML!");
        }
    }

    #[test]
    fn icon_svg_ni_valid_kwa_jina_halali() {
        // Jina halali (mtechos-ide) hurudisha SVG; lisilopo → 404 (hapana panic).
        let dir = extension_dir("mtechos-ide");
        assert!(dir.map(|d| d.join("canvas-extension.json").exists()).unwrap_or(true));
    }

    #[test]
    fn extensions_zote_tatu_zinasomwa() {
        // Kama repo iko, roots tatu zinapatikana (Forge + Monaco IDE + tovuti).
        for name in ["mtechos-ide", "mbilinyitech-ide", "mbilinyi-tech"] {
            if let Some(m) = read_manifest(name) {
                assert_eq!(m["name"], name);
                assert!(m["contributes"]["pages"].is_array());
            }
        }
    }

    #[test]
    fn installed_inaorodhesha_kila_extension_kwenye_roots() {
        // Orodha LAZIMA itokane na roots halisi, sio majina ya mkono — ndiyo
        // inavyohakikisha extension ya tatu (mbilinyi-tech) haipotei.
        let names: Vec<String> = extension_roots()
            .into_iter()
            .filter_map(|d| {
                let t = std::fs::read_to_string(d.join("canvas-extension.json")).ok()?;
                let v: Value = serde_json::from_str(&t).ok()?;
                v.get("name").and_then(|n| n.as_str()).map(str::to_string)
            })
            .collect();
        // Kila root yenye manifest LAZIMA iwe kwenye orodha (roots zote zinasomwa).
        for dir in extension_roots() {
            let t = std::fs::read_to_string(dir.join("canvas-extension.json")).unwrap();
            let v: Value = serde_json::from_str(&t).unwrap();
            let name = v["name"].as_str().unwrap();
            assert!(names.contains(&name.to_string()));
        }
        // Kama mts-canvas iko kwenye disk, jina lake LAZIMA liwe kwenye orodha.
        let mts = std::path::Path::new("mts-native")
            .join("ui")
            .join("mts-canvas")
            .join("canvas-extension.json");
        if mts.exists() {
            assert!(
                names.iter().any(|n| n == "mbilinyi-tech"),
                "mbilinyi-tech lazima iorodheshwe kutoka mts-native/ui/mts-canvas"
            );
        }
    }

    /// KIKINGA: module hii HAIRUHUSIWI kusajili njia za protocol ya agent-server.
    /// Kila moja ya njia hizi LAZIMA ipite kwenye daraja la agent-server (data
    /// halisi); kuzisajili hapa huzifanya "shadow" na hurudisha thamani za kubuni.
    #[test]
    fn haisajili_shadow_routes_za_agent_server() {
        let src = include_str!("canvas_ext.rs");
        // Sehemu ya msimbo PEKEE (bila maoni/tests) — maoni hurejea njia kwa backticks.
        let code: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let code = code.split("#[cfg(test)]").next().unwrap_or(&code);
        let forbidden = [
            ".route(\"/api/settings\"",
            ".route(\"/api/settings/",
            ".route(\"/api/profiles\"",
            ".route(\"/api/profiles/",
            ".route(\"/api/agent-profiles\"",
            ".route(\"/api/meta-profiles\"",
            ".route(\"/api/llm/providers\"",
            ".route(\"/api/llm/models/verified\"",
            ".route(\"/api/llm/provider-connections\"",
            ".route(\"/api/llm/provider-connections/",
            ".route(\"/api/options/models\"",
            ".route(\"/server_info\"",
            ".route(\"/alive\"",
            ".route(\"/ready\"",
        ];
        for f in forbidden {
            assert!(
                !code.contains(f),
                "shadow route `{f}` hairuhusiwi kwenye canvas_ext (ipite kwa agent-server)"
            );
        }
    }
}
