//! agent_native — PROTOCOL YA AGENT-SERVER KWA RUST (badala ya Python).
//!
//! Hii inachukua nafasi ya `agent_server_proxy.rs` (daraja la Python :18000).
//! Inahudumia `/api/*` + `/sockets/*` kwa Rust PEKEE — hakuna Python.
//!
//! Njia zote za protocol (174 operations kutoka OpenAPI ya agent-server)
//! zimegawanywa:
//!   * `state`         — hifadhi (settings/profiles/secrets/conversations/events)
//!   * `events`        — waundaji wa events + frames za WS
//!   * `settings`      — settings, profiles, agent-profiles, meta-profiles,
//!                       secrets, llm (models/providers/provider-connections)
//!   * `conversations` — conversations, events, sockets, bash, file, git,
//!                       workspaces, skills, plugins, tools, canvas, misc

pub mod conversations;
pub mod events;
pub mod settings;
pub mod state;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};
use serde_json::json;

use crate::AppState;

/// Toleo la protocol ya native (huonekana `/server_info`).
pub const NATIVE_PROTOCOL_VERSION: &str = "1.0.0";

/// Key ya session inayotarajiwa (`X-Session-API-Key`).
///
/// Hupata token IMARA ile ile ya `fundi-deploy` (`agent_session_stable`) —
/// SPA huiweka ndani ya `/ide` HTML. Hii inahakikisha protocol ya Rust
/// inakubali key ILE ILE ambayo Canvas inaituma.
fn expected_key() -> Option<String> {
    crate::canvas_ext::agent_key().map(|s| s.to_string())
}

/// Ulinzi: kila ombi la protocol LAZIMA liwe na key sahihi.
///
/// Hufuata kanuni ya agent-server (`OH_SESSION_API_KEYS_0`): bila key
/// sahihi → 401. Bila key iliyowekwa (haipatikani) → tunaruhusu (fail-safe
/// ya dev) ili UI isikwame kwenye mazingira ya ndani.
///
/// Token inayokubalika ni mojawapo ya haya mawili:
///   * key ya `__agent__` (Canvas/`/ide` huiweka kwenye HTML wakati
///     `AGENT_CANVAS_ALLOW_LAN_SESSION_KEY=true`); AU
///   * token HALISI ya login (`/api/auth/login`) iliyopo kwenye table
///     `sessions`. Hii huruhusu UI kufanya kazi bila key yoyote kuvuja
///     kwenye HTML — mtumiaji huingia kwa username/password.
pub async fn auth_mw(
    State(s): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    if is_public_path(&path) {
        return next.run(req).await;
    }
    let got = header_or_cookie_key(&req);
    // Bila key yoyote iliyowekwa kwenye server, ni dev-mode: ruhusu.
    let Some(expected) = expected_key() else {
        return next.run(req).await;
    };
    if expected.trim().is_empty() {
        return next.run(req).await;
    }
    match got {
        Some(k) if k == expected => next.run(req).await,
        // Token ya login halisi (MBILINYI n.k.) — thibitisha kwenye DB.
        Some(k) if crate::auth::verify_token(&s.db, &k).await.is_some() => {
            next.run(req).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "detail": "Session API key si sahihi au haipo" })),
        )
            .into_response(),
    }
}

/// Huchota key kutoka `X-Session-API-Key`, `Authorization: Bearer` au cookie
/// ya workspace.
fn header_or_cookie_key(req: &Request) -> Option<String> {
    req.headers()
        .get("x-session-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            req.headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| cookie_key(req))
}

/// Huchota key kutoka cookie ya `oh_workspace_session_key` (iliyowekwa na
/// `POST /api/auth/workspace-session`). Browser huipakia kiotomatiki kwenye
/// `<iframe src>` / `<img src>` za workspace — njia ambazo haziwezi kubeba
/// header ya `X-Session-API-Key`.
fn cookie_key(req: &Request) -> Option<String> {
    let raw = req.headers().get(axum::http::header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == "oh_workspace_session_key" && !value.is_empty()).then(|| value.to_string())
    })
}

/// Njia ambazo HAZIHITAJI key kwenye header (probes za health — agent-server
/// vivyo hivyo). Soketi za WS pia hazihitaji header: browser haiwezi kuweka
/// `X-Session-API-Key` kwenye handshake, hivyo uthibitisho hufanyika kwa
/// `auth` frame ya kwanza baada ya kuunganisha (`handle_session`).
fn is_public_path(path: &str) -> bool {
    matches!(path, "/alive" | "/ready" | "/health") || path.starts_with("/sockets/")
}

/// Router ya protocol YOTE ya agent-server (Rust-native).
///
/// Kumbuka: njia maalum zilizosajiliwa kwenye `main.rs` (mf. `/api/skills`,
/// `/api/llm/models`) hushinda kwa sababu Axum hupa kipaumbele route mahususi
/// kuliko wildcard. Hii ni sawa — zile zote ni za MTECHOS na zinaumbo sawa.
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        // ── Probes + metadata (bila kiambishi /api) ──────────────────────
        .route("/alive", get(conversations::alive))
        .route("/ready", get(conversations::ready))
        .route("/server_info", get(conversations::server_info))
        .route("/api/init", get(conversations::init_get).post(conversations::init_post))
        // ── Settings ─────────────────────────────────────────────────────
        .route(
            "/api/settings",
            get(settings::get_settings).patch(settings::patch_settings),
        )
        .route("/api/settings/agent-schema", get(settings::agent_schema))
        .route(
            "/api/settings/conversation-schema",
            get(settings::conversation_schema),
        )
        .route(
            "/api/settings/secrets",
            get(settings::list_secrets).put(settings::put_secret),
        )
        .route(
            "/api/settings/secrets/:name",
            get(settings::get_secret).delete(settings::delete_secret),
        )
        .route(
            "/api/settings/mcp/:key",
            post(settings::mcp_set)
                .patch(settings::mcp_set)
                .delete(settings::mcp_delete),
        )
        // ── Profiles ─────────────────────────────────────────────────────
        .route(
            "/api/profiles",
            get(settings::list_profiles).post(settings::upsert_profile),
        )
        .route(
            "/api/profiles/:name",
            get(settings::get_profile)
                .post(settings::upsert_profile_named)
                .delete(settings::delete_profile),
        )
        .route("/api/profiles/:name/activate", post(settings::activate_profile))
        .route("/api/profiles/:name/rename", post(settings::rename_profile))
        .route("/api/profiles/:name/validate", post(settings::validate_profile))
        // ── Agent profiles ───────────────────────────────────────────────
        .route(
            "/api/agent-profiles",
            get(settings::list_agent_profiles).post(settings::upsert_agent_profile),
        )
        .route(
            "/api/agent-profiles/:name",
            get(settings::get_agent_profile)
                .post(settings::upsert_agent_profile_named)
                .delete(settings::delete_agent_profile),
        )
        .route(
            "/api/agent-profiles/:name/activate",
            post(settings::activate_agent_profile),
        )
        .route(
            "/api/agent-profiles/:name/rename",
            post(settings::rename_agent_profile),
        )
        .route(
            "/api/agent-profiles/:name/materialize",
            post(settings::materialize_agent_profile),
        )
        // ── Meta profiles ────────────────────────────────────────────────
        .route(
            "/api/meta-profiles",
            get(settings::list_meta_profiles).post(settings::upsert_meta_profile),
        )
        .route(
            "/api/meta-profiles/:name",
            get(settings::get_meta_profile)
                .post(settings::upsert_meta_profile_named)
                .delete(settings::delete_meta_profile),
        )
        .route(
            "/api/meta-profiles/:name/activate",
            post(settings::activate_meta_profile),
        )
        // ── LLM ──────────────────────────────────────────────────────────
        .route("/api/llm/models/verified", get(settings::llm_models_verified))
        .route("/api/llm/providers", get(settings::llm_providers))
        .route(
            "/api/llm/provider-connections",
            get(settings::list_provider_connections).post(settings::create_provider_connection),
        )
        .route(
            "/api/llm/provider-connections/:id",
            axum::routing::patch(settings::update_provider_connection)
                .delete(settings::delete_provider_connection),
        )
        .route("/api/keys/current", get(settings::keys_current))
        .route("/api/llm/subscription/openai/status", get(settings::sub_status))
        .route("/api/llm/subscription/openai/models", get(settings::sub_models))
        .route(
            "/api/llm/subscription/openai/device/start",
            post(settings::sub_device_start),
        )
        .route(
            "/api/llm/subscription/openai/device/poll",
            post(settings::sub_device_poll),
        )
        .route(
            "/api/llm/subscription/openai/logout",
            post(settings::sub_logout),
        )
        // ── Conversations ────────────────────────────────────────────────
        .route(
            "/api/conversations",
            get(conversations::batch_get_conversations).post(conversations::start_conversation),
        )
        .route("/api/conversations/count", get(conversations::count_conversations))
        .route("/api/conversations/search", get(conversations::search_conversations))
        .route(
            "/api/conversations/:id",
            get(conversations::get_conversation)
                .patch(conversations::update_conversation)
                .delete(conversations::delete_conversation),
        )
        .route("/api/conversations/:id/events", get(conversations::get_events).post(conversations::send_message))
        .route("/api/conversations/:id/events/count", get(conversations::events_count))
        .route("/api/conversations/:id/events/search", get(conversations::search_events))
        .route("/api/conversations/:id/events/:event_id", get(conversations::get_event))
        .route(
            "/api/conversations/:id/events/respond_to_confirmation",
            post(conversations::respond_to_confirmation),
        )
        .route("/api/conversations/:id/run", post(conversations::run_conversation))
        .route("/api/conversations/:id/pause", post(conversations::pause_conversation))
        .route("/api/conversations/:id/interrupt", post(conversations::interrupt_conversation))
        .route("/api/conversations/:id/condense", post(conversations::condense))
        .route("/api/conversations/:id/fork", post(conversations::fork_conversation))
        .route(
            "/api/conversations/:id/confirmation_policy",
            post(conversations::set_confirmation_policy),
        )
        .route(
            "/api/conversations/:id/security_analyzer",
            post(conversations::set_security_analyzer),
        )
        .route("/api/conversations/:id/switch_llm", post(conversations::switch_llm))
        .route(
            "/api/conversations/:id/switch_profile",
            post(conversations::switch_profile),
        )
        .route(
            "/api/conversations/:id/switch_acp_model",
            post(conversations::switch_acp_model),
        )
        .route("/api/conversations/:id/secrets", post(conversations::add_secrets))
        .route("/api/conversations/:id/load_plugin", post(conversations::load_plugin))
        .route("/api/conversations/:id/navigate", post(conversations::navigate))
        .route("/api/conversations/:id/ask_agent", post(conversations::ask_agent))
        .route(
            "/api/conversations/:id/agent_final_response",
            get(conversations::agent_final_response),
        )
        .route("/api/conversations/:id/runtime", get(conversations::get_runtime))
        .route(
            "/api/conversations/:id/runtime/reprovision",
            post(conversations::reprovision_runtime),
        )
        .route(
            "/api/conversations/:id/workspace",
            get(conversations::list_workspace_files),
        )
        .route(
            "/api/conversations/:id/workspace/*path",
            get(conversations::get_workspace_file),
        )
        .route("/api/conversations/:id/goal", post(conversations::set_goal))
        .route("/api/conversations/:id/goal/resume", post(conversations::goal_resume))
        .route("/api/conversations/:id/goal/stop", post(conversations::goal_stop))
        // ── Bash (conversation-scoped + unscoped) ────────────────────────
        .route(
            "/api/bash/execute_bash_command",
            post(conversations::execute_bash_command),
        )
        .route("/api/bash/start_bash_command", post(conversations::start_bash_command))
        .route(
            "/api/bash/bash_commands/:id/stop",
            post(conversations::stop_bash_command),
        )
        .route(
            "/api/bash/bash_events",
            get(conversations::bash_events_search).delete(conversations::clear_bash_events),
        )
        .route("/api/bash/bash_events/", get(conversations::list_bash_events))
        .route("/api/bash/bash_events/search", get(conversations::bash_events_search))
        .route("/api/bash/bash_events/:id", get(conversations::get_bash_event))
        .route(
            "/api/conversations/:id/bash/execute_bash_command",
            post(conversations::execute_bash_command),
        )
        .route(
            "/api/conversations/:id/bash/start_bash_command",
            post(conversations::start_bash_command),
        )
        .route(
            "/api/conversations/:id/bash/bash_commands/:cmd/stop",
            post(conversations::stop_bash_command),
        )
        .route(
            "/api/conversations/:id/bash/bash_events",
            get(conversations::bash_events_search).delete(conversations::clear_bash_events),
        )
        .route(
            "/api/conversations/:id/bash/bash_events/",
            get(conversations::list_bash_events),
        )
        .route(
            "/api/conversations/:id/bash/bash_events/search",
            get(conversations::bash_events_search),
        )
        .route(
            "/api/conversations/:id/bash/bash_events/:event_id",
            get(conversations::get_bash_event),
        )
        // ── File ─────────────────────────────────────────────────────────
        .route("/api/file/home", get(conversations::file_home))
        .route("/api/file/upload", post(conversations::file_upload))
        .route("/api/file/download", get(conversations::file_download))
        .route("/api/file/archive", get(conversations::file_archive))
        .route("/api/file/create_directory", post(conversations::file_create_directory))
        .route("/api/file/search_subdirs", get(conversations::file_search_subdirs))
        .route(
            "/api/file/download-trajectory/:conversation_id",
            get(conversations::download_trajectory),
        )
        .route("/api/conversations/:id/file/upload", post(conversations::file_upload))
        .route("/api/conversations/:id/file/download", get(conversations::file_download))
        .route("/api/conversations/:id/file/archive", get(conversations::file_archive))
        .route(
            "/api/conversations/:id/file/create_directory",
            post(conversations::file_create_directory),
        )
        .route(
            "/api/conversations/:id/file/download-trajectory/:cid",
            get(conversations::download_trajectory),
        )
        // ── Git ──────────────────────────────────────────────────────────
        .route("/api/git/changes", get(conversations::git_changes))
        .route("/api/git/commits", get(conversations::git_commits))
        .route("/api/git/commits/:sha/changes", get(conversations::git_commit_changes))
        .route("/api/git/diff", get(conversations::git_diff))
        .route(
            "/api/git/repositories/search",
            get(conversations::git_repositories_search),
        )
        .route("/api/conversations/:id/git/changes", get(conversations::git_changes))
        .route("/api/conversations/:id/git/commits", get(conversations::git_commits))
        .route(
            "/api/conversations/:id/git/commits/:sha/changes",
            get(conversations::git_commit_changes),
        )
        .route("/api/conversations/:id/git/diff", get(conversations::git_diff))
        // ── VSCode ───────────────────────────────────────────────────────
        .route("/api/vscode/status", get(conversations::vscode_status))
        .route("/api/vscode/url", get(conversations::vscode_url))
        .route("/api/conversations/:id/vscode/status", get(conversations::vscode_status))
        .route("/api/conversations/:id/vscode/url", get(conversations::vscode_url))
        // ── Workspaces ───────────────────────────────────────────────────
        .route(
            "/api/workspaces",
            get(conversations::list_workspaces)
                .post(conversations::add_workspace)
                .delete(conversations::delete_workspace),
        )
        .route(
            "/api/workspaces/parents",
            get(conversations::list_workspace_parents)
                .post(conversations::add_workspace_parent)
                .delete(conversations::delete_workspace_parent),
        )
        // ── Skills ───────────────────────────────────────────────────────
        .route(
            "/api/skills/installed",
            get(conversations::list_skills).post(conversations::install_skill),
        )
        .route("/api/skills/marketplace", get(conversations::skills_marketplace))
        .route("/api/skills/install", post(conversations::install_skill))
        .route("/api/skills/sync", post(conversations::skills_sync))
        .route(
            "/api/skills/installed/:name",
            get(conversations::get_skill)
                .patch(conversations::patch_skill)
                .delete(conversations::delete_skill),
        )
        .route(
            "/api/skills/installed/:name/refresh",
            post(conversations::refresh_skill),
        )
        // ── Plugins ──────────────────────────────────────────────────────
        .route(
            "/api/plugins/installed",
            get(conversations::list_plugins).post(conversations::install_plugin),
        )
        .route("/api/plugins/marketplace", get(conversations::plugins_marketplace))
        .route("/api/plugins/install", post(conversations::install_plugin))
        .route(
            "/api/plugins/installed/:name",
            get(conversations::get_plugin)
                .patch(conversations::patch_plugin)
                .delete(conversations::delete_plugin),
        )
        .route(
            "/api/plugins/installed/:name/refresh",
            post(conversations::refresh_plugin),
        )
        // ── Tools + hooks + sub-agents + acp ─────────────────────────────
        .route("/api/tools/", get(conversations::tools_list))
        .route("/api/tools/catalog", get(conversations::tools_catalog))
        .route("/api/hooks", post(conversations::create_hook))
        .route("/api/sub-agents", post(conversations::create_sub_agent))
        .route("/api/acp/models", post(conversations::acp_models))
        .route("/api/shared-conversations", get(conversations::shared_conversations))
        .route("/api/shared-events/search", get(conversations::shared_events_search))
        .route("/api/auth/workspace-session", post(conversations::workspace_session))
        .route(
            "/api/auth/workspace-session",
            axum::routing::delete(conversations::workspace_session_delete),
        )
        // ── MCP OAuth ────────────────────────────────────────────────────
        .route("/api/mcp/test", post(conversations::mcp_test))
        .route("/api/mcp/oauth/start", post(conversations::mcp_oauth_start))
        .route("/api/mcp/oauth/status/:job_id", get(conversations::mcp_oauth_status))
        .route(
            "/api/mcp/oauth/callback/:job_id",
            post(conversations::mcp_oauth_callback),
        )
        // ── v1 (App conversations — Canvas hutumia) ──────────────────────
        .route("/api/v1/settings", get(settings::get_settings).patch(settings::patch_settings))
        .route("/api/v1/settings/agent-schema", get(settings::agent_schema))
        .route(
            "/api/v1/settings/conversation-schema",
            get(settings::conversation_schema),
        )
        .route("/api/v1/settings/profiles", get(settings::list_profiles))
        .route(
            "/api/v1/secrets",
            get(settings::list_secrets).put(settings::put_secret),
        )
        .route(
            "/api/v1/secrets/search",
            get(settings::list_secrets),
        )
        .route(
            "/api/v1/secrets/:id",
            get(settings::get_secret).delete(settings::delete_secret),
        )
        .route(
            "/api/v1/app-conversations",
            get(conversations::app_list_conversations).post(conversations::app_start_conversation),
        )
        .route(
            "/api/v1/app-conversations/search",
            get(conversations::app_search_conversations),
        )
        .route(
            "/api/v1/app-conversations/start-tasks",
            post(conversations::app_start_tasks),
        )
        .route(
            "/api/v1/app-conversations/:id",
            get(conversations::app_get_conversation),
        )
        .route(
            "/api/v1/app-conversations/:id/file",
            get(conversations::app_get_file),
        )
        .route(
            "/api/v1/app-conversations/:id/files",
            get(conversations::app_list_files),
        )
        .route(
            "/api/v1/app-conversations/:id/download",
            get(conversations::app_download),
        )
        .route(
            "/api/v1/app-conversations/:id/skills",
            get(conversations::app_skills),
        )
        .route(
            "/api/v1/app-conversations/:id/git/changes",
            get(conversations::git_changes),
        )
        .route(
            "/api/v1/app-conversations/:id/git/diff",
            get(conversations::git_diff),
        )
        .route(
            "/api/v1/app-conversations/:id/switch_acp_model",
            post(conversations::switch_acp_model),
        )
        .route(
            "/api/v1/app-conversations/:id/switch_profile",
            post(conversations::switch_profile),
        )
        .route(
            "/api/v1/config/models/search",
            get(settings::v1_models_search),
        )
        .route(
            "/api/v1/config/providers/search",
            get(settings::v1_providers_search),
        )
        .route(
            "/api/v1/conversation/:id/events/search",
            get(conversations::search_events),
        )
        .route(
            "/api/v1/conversations/:id/pending-messages",
            get(conversations::pending_messages),
        )
        .route("/api/v1/sandboxes", get(conversations::list_sandboxes))
        .route("/api/v1/sandboxes/:id/pause", post(conversations::sandbox_pause))
        .route("/api/v1/sandboxes/:id/resume", post(conversations::sandbox_resume))
        .route(
            "/api/v1/git/branches/search",
            get(conversations::git_branches_search),
        )
        .route(
            "/api/v1/git/repositories/search",
            get(conversations::git_repositories_search),
        )
        .route(
            "/api/v1/git/installations/search",
            get(conversations::git_installations_search),
        )
        .route("/api/v1/skills/search", get(conversations::v1_skills_search))
        .route("/api/v1/mcp/test", post(conversations::mcp_test))
        .route(
            "/api/v1/mcp/oauth/status/:job_id",
            get(conversations::mcp_oauth_status),
        )
        .route(
            "/api/v1/mcp/oauth/*path",
            any(conversations::mcp_oauth_any),
        )
        .route("/api/v1/web-client/config", get(conversations::web_client_config))
        // ── Cloud proxy + organizations (haipo kwenye Rust — hujibu tupu) ─
        .route("/api/cloud-proxy", any(conversations::cloud_proxy))
        .route("/api/organizations", get(conversations::list_organizations))
        .route("/api/organizations/:id/me", get(conversations::org_me))
        .route("/api/organizations/:id/profiles", get(settings::list_profiles))
        .route(
            "/api/organizations/:id/provider-connections",
            get(settings::list_provider_connections),
        )
        .route(
            "/api/organizations/:id/meta-profiles",
            get(settings::list_meta_profiles),
        )
        .route("/api/admin/setup-state", get(conversations::setup_state))
        .route("/api/analytics/events", post(conversations::analytics_events))
        // ── Sockets (WS) ─────────────────────────────────────────────────
        .route("/sockets/session/:id", get(conversations::socket_session))
        .route("/sockets/events/:id", get(conversations::socket_events))
        .route("/sockets/bash-events", get(conversations::socket_bash_events))
        // Ulinzi: kila ombi la protocol huthibitishwa kwa `X-Session-API-Key`
        // AU token halisi ya login (`/api/auth/login`) iliyo kwenye `sessions`.
        .layer(axum::middleware::from_fn_with_state(state, auth_mw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;

    fn req_with(header_name: &str, value: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .header(header_name, value)
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn header_or_cookie_key_hupata_x_session_api_key() {
        let r = req_with("x-session-api-key", "abc123");
        assert_eq!(header_or_cookie_key(&r).as_deref(), Some("abc123"));
    }

    #[test]
    fn header_or_cookie_key_hupata_bearer() {
        let r = req_with("authorization", "Bearer tok-xyz");
        assert_eq!(header_or_cookie_key(&r).as_deref(), Some("tok-xyz"));
    }

    #[test]
    fn header_or_cookie_key_hupata_cookie_ya_workspace() {
        let r = req_with("cookie", "a=1; oh_workspace_session_key=ck-9; b=2");
        assert_eq!(header_or_cookie_key(&r).as_deref(), Some("ck-9"));
    }

    #[test]
    fn header_or_cookie_key_haikubali_nafasi_tupu() {
        let r = req_with("x-session-api-key", "   ");
        assert_eq!(header_or_cookie_key(&r), None);
    }

    #[test]
    fn is_public_path_ni_probes_na_sockets_pekee() {
        assert!(is_public_path("/alive"));
        assert!(is_public_path("/ready"));
        assert!(is_public_path("/health"));
        assert!(is_public_path("/sockets/events/1"));
        assert!(!is_public_path("/api/settings"));
        assert!(!is_public_path("/api/conversations"));
    }
}
