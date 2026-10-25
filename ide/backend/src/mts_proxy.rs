//! mts_proxy.rs — same-origin daraja kwa Rust core ya MTECHOS (mts-core).
//!
//! Kwa nini proxy: Canvas App hupakiwa kutoka origin ya fundi-deploy, lakini
//! mts-core hulistening kwenye loopback (`127.0.0.1:8788`). Kuingiza
//! `apiBase = http://127.0.0.1:8788` kwenye HTML kunavunja kwa mgeni — browser
//! yake hufikiri `127.0.0.1` ni mashine YAKE, na `SameSite=Lax` cookies
//! hazitumwa kwenye ombi la cross-site. Hivyo browser huzungumza na origin
//! ILE ILE inayohudumia ukurasa, na hapa tunapeleka `/mts/*` kwa core.
//!
//! Njia huhifadhiwa kama zilivyo (`/mts/api/auth` → `<core>/api/auth`), hivyo
//! cookies za core (`Path=/`) hutumika kwenye origin ya umma na hakuna CORS.

use axum::{
    body::Bytes,
    extract::Path,
    http::{header, HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Kiambishi cha njia kinachopelekwa kwa core. Canvas hupata hii kama
/// `window.__MTS__.apiBase`, hivyo ni chanzo KIMOJA na `canvas_ext.rs`.
pub const PROXY_PREFIX: &str = "/mts";

/// Upstream base ya core (server-side pekee): `MTS_CORE_URL`.
pub fn core_url() -> String {
    std::env::var("MTS_CORE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8788".into())
        .trim()
        .trim_end_matches('/')
        .to_string()
}

/// Headers zinazopelekwa kwa core. `host`/`content-length` huachwa — zina
/// maana kwa upstream mpya pekee; `connection` ni hop-by-hop.
fn forwardable(name: &header::HeaderName) -> bool {
    !matches!(
        name.as_str(),
        "host" | "content-length" | "connection" | "transfer-encoding" | "upgrade"
    )
}

/// `GET|POST /mts/*path` — hupeleka ombi kwa core na kurudisha jibu lake
/// (status, headers ikiwemo `set-cookie`, na body) bila kubadilisha.
pub async fn proxy(
    method: Method,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if path.contains("..") {
        return (StatusCode::BAD_REQUEST, "path si sahihi").into_response();
    }
    let url = format!("{}/{}", core_url(), path.trim_start_matches('/'));

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => return upstream_error(&format!("client: {e}")),
    };

    let mut req = client.request(method, &url).body(body.to_vec());
    for (name, value) in headers.iter() {
        if forwardable(name) {
            req = req.header(name, value);
        }
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => return upstream_error(&format!("core haifikiki ({url}): {e}")),
    };

    let status = resp.status();
    let upstream_headers = resp.headers().clone();
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return upstream_error(&format!("body: {e}")),
    };

    let mut out = Response::new(axum::body::Body::from(bytes));
    *out.status_mut() = status;
    for (name, value) in upstream_headers.iter() {
        // `set-cookie` LAZIMA ipite (session ya core inahifadhiwa kwenye origin
        // ya umma); hop-by-hop hazina maana upande huu.
        if name.as_str() != "transfer-encoding" && name.as_str() != "connection" {
            out.headers_mut().append(name.clone(), value.clone());
        }
    }
    out
}

fn upstream_error(msg: &str) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        axum::Json(json!({ "ok": false, "error": format!("mts-core: {msg}") })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kiambishi_kinalingana_na_injection() {
        // canvas_ext.rs huingiza PROXY_PREFIX kama apiBase; ikiwa ni njia
        // inayoanza na `/`, browser huitumia kwa origin ILE ILE.
        assert!(PROXY_PREFIX.starts_with('/'));
        assert_eq!(PROXY_PREFIX, "/mts");
    }

    #[test]
    fn headers_za_hop_by_hop_hazupelekwi() {
        assert!(!forwardable(&header::HOST));
        assert!(!forwardable(&header::CONTENT_LENGTH));
        assert!(!forwardable(&header::CONNECTION));
        assert!(forwardable(&header::COOKIE));
        assert!(forwardable(&header::CONTENT_TYPE));
    }
}
