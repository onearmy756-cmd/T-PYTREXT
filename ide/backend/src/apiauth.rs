//! apiauth.rs — ukaguzi wa uthibitisho wa API (JWT, OAuth2/OIDC) na
//! huduma za usajili (Nacos/Consul). Hii ni **uchambuzi tuli** (read-only):
//! haina kuthibitisha saini na haitumi token yoyote.
//!
//! - `jwt_inspect(token)`: hutenganisha JWT na kukagua `alg=none`, HS256 kwa
//!   mteja wa umma, `exp`/`iat`/`nbf`, `iss`/`aud`, `kid`, scopes.
//! - `oauth_audit(cfg)`: hukagua usanidi wa OAuth2/OIDC (PKCE, implicit,
//!   redirect_uri, muda wa token, rotation ya refresh).
//! - `nacos_audit(cfg)`: hukagua usajili wa huduma (Nacos/Consul) — auth,
//!   token ya default, TLS, console iliyo wazi, namespace isolation.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};

/// Dispatch ya ops za uthibitisho wa API (jwt | oauth | nacos | catalog).
pub fn dispatch(op: &str, req: &Value) -> Value {
    match op {
        "jwt" | "jwt_inspect" => jwt_inspect(req["token"].as_str().unwrap_or("")),
        "oauth" | "oauth_audit" => oauth_audit(req),
        "nacos" | "nacos_audit" | "consul" => nacos_audit(req),
        "catalog" => json!({
            "ok": true, "engine": "akili-security/apiauth",
            "ops": ["jwt", "oauth", "nacos"],
            "desc": {
                "jwt": "tenganisha JWT + kagua alg=none/HS*/exp/iss/aud/scopes",
                "oauth": "kagua OAuth2/OIDC (PKCE/implicit/redirect/ttl/rotation)",
                "nacos": "kagua Nacos/Consul (auth/TLS/token ya default/console)"
            }
        }),
        other => json!({"ok": false, "error": format!("op `{other}` haijulikani (jwt|oauth|nacos|catalog)")}),
    }
}

fn findings_to_score(findings: &[Value], max: f64) -> (f64, String) {
    let penalty: f64 = findings
        .iter()
        .map(|f| f["penalty"].as_f64().unwrap_or(0.0))
        .sum();
    let score = (max - penalty).max(0.0);
    let grade = match score as u32 {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        50..=59 => "E",
        _ => "F",
    };
    (score, grade.to_string())
}

fn f(sev: &str, id: &str, msg: &str, penalty: f64) -> Value {
    json!({"severity": sev, "id": id, "message": msg, "penalty": penalty})
}

/// Hutenganisha JWT (bila kuthibitisha saini) na kukagua hatari.
pub fn jwt_inspect(token: &str) -> Value {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return json!({"ok": false, "error": "JWT si sahihi (sehemu 3 zinahitajika)"});
    }
    let dec = |s: &str| -> Option<Value> {
        URL_SAFE_NO_PAD
            .decode(s.as_bytes())
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
    };
    let header = match dec(parts[0]) {
        Some(h) => h,
        None => return json!({"ok": false, "error": "header si JSON/base64url sahihi"}),
    };
    let payload = dec(parts[1]).unwrap_or(json!({}));
    let alg = header["alg"].as_str().unwrap_or("haijulikani").to_string();

    let mut findings = Vec::new();
    if alg.eq_ignore_ascii_case("none") {
        findings.push(f("critical", "alg-none", "alg=none — token halijasainiwa", 60.0));
    }
    if alg.starts_with("HS") {
        findings.push(f(
            "high",
            "alg-hs",
            "alg ya ulinganifu (HS*) — siri iliyoshirikiwa, hatari kwa mteja wa umma",
            25.0,
        ));
    }
    if alg.is_empty() {
        findings.push(f("high", "alg-missing", "alg haipo kwenye header", 20.0));
    }
    if header["kid"].as_str().is_none() {
        findings.push(f("low", "kid-missing", "kid haipo — ugumu wa kuzungusha funguo", 5.0));
    }
    let now = chrono::Utc::now().timestamp();
    match payload["exp"].as_i64() {
        None => findings.push(f("high", "exp-missing", "exp haipo — token haiishi", 25.0)),
        Some(exp) if exp < now => {
            findings.push(f("medium", "exp-expired", "token limeisha muda", 10.0))
        }
        Some(exp) if exp - now > 24 * 3600 => findings.push(f(
            "medium",
            "exp-long",
            "muda wa token ni mrefu sana (> saa 24)",
            10.0,
        )),
        _ => {}
    }
    if payload["iss"].as_str().is_none() {
        findings.push(f("low", "iss-missing", "iss (mtangazaji) haipo", 5.0));
    }
    if payload["aud"].as_str().is_none() && payload["aud"].as_array().is_none() {
        findings.push(f("low", "aud-missing", "aud (hadhira) haipo", 5.0));
    }
    if let Some(scope) = payload["scope"].as_str() {
        let dangerous = ["admin", "write", "*", "root", "superuser"];
        if dangerous.iter().any(|d| scope.to_lowercase().contains(d)) {
            findings.push(f("medium", "scope-broad", "scopes pana (admin/write/*)", 10.0));
        }
    }

    let (score, grade) = findings_to_score(&findings, 100.0);
    json!({
        "ok": true, "engine": "akili-security/apiauth",
        "alg": alg, "has_exp": payload["exp"].is_number(),
        "has_iss": payload["iss"].is_string(), "has_aud": !payload["aud"].is_null(),
        "claims": payload,
        "finding_count": findings.len(), "findings": findings,
        "score": score, "grade": grade,
        "note": "uchambuzi tuli — saini HAJATHIBITISHWA; usitumie token halisi.",
    })
}

/// Ukaguzi wa usanidi wa OAuth2/OIDC.
pub fn oauth_audit(cfg: &Value) -> Value {
    let flow = cfg["flow"].as_str().unwrap_or("authorization_code");
    let mut findings = Vec::new();
    if flow.eq_ignore_ascii_case("implicit") {
        findings.push(f("high", "implicit-flow", "implicit flow imekwisha (deprecated)", 25.0));
    }
    if flow.eq_ignore_ascii_case("authorization_code") && cfg["pkce"].as_bool() != Some(true) {
        findings.push(f("high", "pkce-missing", "PKCE haitumiki kwenye authorization_code", 25.0));
    }
    match cfg["redirect_uri"].as_str().unwrap_or("") {
        r if r.contains('*') => {
            findings.push(f("critical", "redirect-wildcard", "redirect_uri ina wildcard", 40.0))
        }
        "" => findings.push(f("high", "redirect-missing", "redirect_uri haipo", 20.0)),
        _ => {}
    }
    if cfg["token_in_query"].as_bool() == Some(true) {
        findings.push(f("high", "token-in-query", "token inatumwa kwenye query", 20.0));
    }
    if cfg["refresh_rotation"].as_bool() != Some(true) {
        findings.push(f("medium", "refresh-no-rotation", "refresh token hairuzushwi", 10.0));
    }
    if let Some(t) = cfg["access_token_ttl_s"].as_i64() {
        if t > 3600 {
            findings.push(f("medium", "ttl-long", "muda wa access token ni mrefu (> saa 1)", 10.0));
        }
    }
    if cfg["public_client"].as_bool() == Some(true) && cfg["client_secret"].as_str().is_some() {
        findings.push(f("high", "secret-in-public", "client_secret kwenye mteja wa umma", 25.0));
    }
    if cfg["consent"].as_bool() == Some(false) {
        findings.push(f("medium", "no-consent", "ridhaa (consent) haitumiki", 10.0));
    }

    let (score, grade) = findings_to_score(&findings, 100.0);
    json!({
        "ok": true, "engine": "akili-security/apiauth", "flow": flow,
        "finding_count": findings.len(), "findings": findings,
        "score": score, "grade": grade,
    })
}

/// Ukaguzi wa usanidi wa huduma ya usajili (Nacos/Consul).
pub fn nacos_audit(cfg: &Value) -> Value {
    let kind = cfg["kind"].as_str().unwrap_or("nacos");
    let mut findings = Vec::new();
    if cfg["auth_enabled"].as_bool() != Some(true) {
        findings.push(f("critical", "auth-off", "uthibitisho haujawashwa", 45.0));
    }
    if cfg["tls"].as_bool() != Some(true) {
        findings.push(f("high", "tls-off", "TLS haijawashwa (traffic ya wazi)", 25.0));
    }
    let token = cfg["default_token"].as_str().unwrap_or("");
    if !token.is_empty() || cfg["uses_default_token"].as_bool() == Some(true) {
        findings.push(f(
            "critical",
            "default-token",
            "token ya default (Nacos: default secret key) bado inatumika",
            45.0,
        ));
    }
    if cfg["console_public"].as_bool() == Some(true) {
        findings.push(f("high", "console-public", "console/UI iko kwenye mtandao wa umma", 20.0));
    }
    if cfg["namespace_isolation"].as_bool() != Some(true) {
        findings.push(f("medium", "ns-no-isolation", "namespace isolation haipo", 10.0));
    }
    if cfg["allow_anonymous"].as_bool() == Some(true) {
        findings.push(f("high", "anon", "usajili wa bila mtu huruhusiwa", 20.0));
    }
    if cfg["persistence"].as_bool() == Some(true) && cfg["backup"].as_bool() != Some(true) {
        findings.push(f("medium", "no-backup", "hakuna backup ya config", 10.0));
    }

    let (score, grade) = findings_to_score(&findings, 100.0);
    json!({
        "ok": true, "engine": "akili-security/apiauth", "kind": kind,
        "finding_count": findings.len(), "findings": findings,
        "score": score, "grade": grade,
        "note": "endpoint za default: Nacos 8848/9848, Consul 8500.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_jwt_inatenganisha_alg() {
        let token = format!(
            "{}.{}.sig",
            b64(&json!({"alg": "none"})),
            b64(&json!({"exp": 1}))
        );
        let r = dispatch("jwt", &json!({"token": token}));
        assert_eq!(r["ok"], true);
        assert_eq!(r["alg"], "none");
        assert!(r["finding_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn dispatch_haijulikani_hurudisha_error() {
        let r = dispatch("hakuna", &json!({}));
        assert_eq!(r["ok"], false);
    }

    fn b64(v: &Value) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(v).unwrap())
    }

    #[test]
    fn jwt_alg_none_ni_hatari() {
        let t = format!("{}.{}.x", b64(&json!({"alg":"none"})), b64(&json!({"sub":"a"})));
        let r = jwt_inspect(&t);
        assert_eq!(r["ok"], true);
        assert!(r["findings"].as_array().unwrap().iter().any(|f| f["id"] == "alg-none"));
        assert_eq!(r["grade"], "F");
    }

    #[test]
    fn jwt_nzuri_hupata_alama_kubwa() {
        let exp = chrono::Utc::now().timestamp() + 600;
        let t = format!(
            "{}.{}.x",
            b64(&json!({"alg":"RS256","kid":"k1"})),
            b64(&json!({"exp":exp,"iss":"https://idp","aud":"api","scope":"read"}))
        );
        let r = jwt_inspect(&t);
        assert!(r["score"].as_f64().unwrap() >= 90.0);
    }

    #[test]
    fn oauth_implicit_na_pkce_hukamatwa() {
        let r = oauth_audit(&json!({"flow":"implicit","redirect_uri":"*"}));
        let ids: Vec<&str> = r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"implicit-flow"));
        assert!(ids.contains(&"redirect-wildcard"));
    }

    #[test]
    fn nacos_default_token_hukamatwa() {
        let r = nacos_audit(&json!({"auth_enabled":false,"uses_default_token":true}));
        assert_eq!(r["grade"], "F");
    }

    #[test]
    fn nacos_nzuri_hupata_a() {
        let r = nacos_audit(&json!({
            "auth_enabled":true,"tls":true,"console_public":false,
            "namespace_isolation":true,"backup":true
        }));
        assert_eq!(r["grade"], "A");
    }
}
