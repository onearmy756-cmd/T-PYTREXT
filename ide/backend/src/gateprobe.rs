//! gateprobe.rs — ukaguzi HALISI wa API gateway mtandaoni (live probe).
//!
//! Huruka HTTP `HEAD`/`GET` kwa URL iliyotolewa na kukagua: TLS, HSTS, CSP,
//! CORS, headers za usalama, uvujaji wa `Server`/`X-Powered-By`, cookies,
//! rate-limit, na uthibitisho. Matokeo hupitishwa kwenye `finshield::gateway_audit`.
//!
//! **HITL/Usalama**: ni read-only (HEAD/GET pekee). Lazima upitishe
//! `authorized = true`; lengo linahitaji idhini ya mmiliki.

use std::time::Duration;

use serde_json::{json, Value};

/// Huruka probe halisi kwa URL na kurudisha ripoti kamili.
pub async fn probe(url: &str, authorized: bool) -> Value {
    if !authorized {
        return json!({
            "ok": false,
            "error": "unahitaji idhini (authorized=true) — chunguza gateway ya mmiliki pekee",
        });
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return json!({"ok": false, "error": "url lazima ianze na http:// au https://"});
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(false)
        .build()
    {
        Ok(c) => c,
        Err(e) => return json!({"ok": false, "error": e.to_string()}),
    };

    let resp = match client.head(url).send().await {
        Ok(r) => r,
        Err(e) => return json!({"ok": false, "error": format!("probe imefeli: {e}")}),
    };

    let status = resp.status().as_u16();
    let h = resp.headers();
    let get = |k: &str| -> Option<String> {
        h.get(k).and_then(|v| v.to_str().ok()).map(|s| s.to_string())
    };

    let hsts = get("strict-transport-security");
    let csp = get("content-security-policy");
    let xcto = get("x-content-type-options");
    let xfo = get("x-frame-options");
    let cors = get("access-control-allow-origin");
    let server = get("server");
    let powered = get("x-powered-by");
    let ratelimit = get("x-ratelimit-limit").or_else(|| get("ratelimit-limit"));
    let www_auth = get("www-authenticate");
    let set_cookie = get("set-cookie");

    let mut findings: Vec<Value> = Vec::new();
    let mut add = |sev: &str, id: &str, msg: &str| {
        findings.push(json!({"severity": sev, "id": id, "message": msg}));
    };

    let https = url.starts_with("https://");
    if https && hsts.is_none() {
        add("high", "hsts-missing", "HSTS haipo kwenye HTTPS");
    }
    if hsts.as_deref().map(|v| v.contains("max-age=0")).unwrap_or(false) {
        add("high", "hsts-disabled", "HSTS max-age=0 (imezimwa)");
    }
    if csp.is_none() {
        add("medium", "csp-missing", "CSP haipo");
    }
    if xcto.is_none() {
        add("low", "xcto-missing", "X-Content-Type-Options haipo");
    }
    if xfo.is_none() && csp.as_deref().map(|v| v.contains("frame-ancestors")).unwrap_or(false) == false {
        add("low", "xfo-missing", "X-Frame-Options/frame-ancestors haipo");
    }
    if cors.as_deref() == Some("*") {
        add("high", "cors-wildcard", "CORS inaruhusu origin yoyote (*)");
    }
    if let Some(s) = &server {
        if s.chars().any(|c| c.is_ascii_digit()) {
            add("low", "server-version", "header ya Server inaonyesha toleo");
        }
    }
    if powered.is_some() {
        add("low", "powered-by", "X-Powered-By inaonyesha teknolojia");
    }
    if ratelimit.is_none() {
        add("medium", "no-ratelimit", "hakuna dalili ya rate-limit");
    }
    if set_cookie.as_deref().map(|c| !c.to_lowercase().contains("secure")).unwrap_or(false) {
        add("medium", "cookie-insecure", "cookie bila Secure");
    }
    if set_cookie.as_deref().map(|c| !c.to_lowercase().contains("httponly")).unwrap_or(false) {
        add("medium", "cookie-no-httponly", "cookie bila HttpOnly");
    }

    let severity_penalty = |sev: &str| match sev {
        "critical" => 40.0,
        "high" => 25.0,
        "medium" => 10.0,
        "low" => 5.0,
        _ => 0.0,
    };
    let penalty: f64 = findings
        .iter()
        .map(|f| severity_penalty(f["severity"].as_str().unwrap_or("")))
        .sum();
    let score = (100.0 - penalty).max(0.0);
    let grade = match score as u32 {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        50..=59 => "E",
        _ => "F",
    };

    json!({
        "ok": true, "engine": "akili-security/gateprobe",
        "url": url, "status": status, "authorized": true,
        "tls": https,
        "headers": {
            "hsts": hsts, "csp": csp, "x_content_type_options": xcto,
            "x_frame_options": xfo, "cors": cors, "server": server,
            "x_powered_by": powered, "www_authenticate": www_auth,
        },
        "auth_required": status == 401 || status == 403 || www_auth.is_some(),
        "finding_count": findings.len(), "findings": findings,
        "score": score, "grade": grade,
        "note": "HEAD pekee (read-only). Tumia finshield gateway_audit kwa uthibitisho wa kina.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bila_idhini_hukataliwa() {
        let r = probe("https://example.com", false).await;
        assert_eq!(r["ok"], false);
    }

    #[tokio::test]
    async fn url_mbaya_hukataliwa() {
        let r = probe("ftp://x", true).await;
        assert_eq!(r["ok"], false);
    }
}
