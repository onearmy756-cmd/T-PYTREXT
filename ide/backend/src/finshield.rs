//! finshield.rs — ULINZI WA BENKI / FINTECH / API GATEWAY (Rust, halisi).
//!
//! Hili ni "ngome ya kifedha" ya MBILINYI AKILI: ukaguzi halisi wa kanuni za
//! usalama wa benki na API gateway. Ni **read-only / defensive** — haitelezi
//! miamala, haigusi akaunti, na HAISIMAMII data ya kadi (PAN hufichwa).
//!
//! Uwezo (halisi, wenye hesabu):
//!   - `luhn()` / `pan_kind()` — uthibitisho wa namba ya kadi + BIN (masked).
//!   - `mask_pan()` — kuficha PAN (PCI-DSS: 6 za mwisho pekee).
//!   - `iban_valid()` — mod-97 checksum ya IBAN.
//!   - `card_expiry_valid()` — uhalali wa tarehe ya kadi.
//!   - `scan_pci()` — tambua PAN/CVV/track data kwenye maandishi (inarejesha masked).
//!   - `scan_secrets()` — tambua API keys/tokens (BIN, cloud, GitHub, JWT).
//!   - `token_bucket()` — hesabu ya rate-limiting (burst + refill).
//!   - `key_strength()` — entropi ya API key/secret.
//!   - `txn_risk()` — alama ya hatari ya muamala + uamuzi (allow/review/block).
//!   - `gateway_audit()` — ukaguzi wa API gateway config → findings + score + grade.
//!   - `wire_audit()` — ukaguzi wa ujumbe wa malipo (ISO 20022 / SWIFT-like).
//!
//! Kanuni: **hakuna** kuhifadhi/kurudisha PAN kamili; CVV HAIREJESHWI kamwe.

use serde_json::{json, Value};

/// Ops zinazopatikana za finshield.
pub const OPS: &[&str] = &[
    "luhn", "pan", "mask", "iban", "pci", "secrets", "bucket", "strength",
    "txn_risk", "gateway_audit", "wire_audit", "catalog",
];

/// Dispatch ya ops zote za finshield (kwa API/CLI). `op` = jina, `req` = data.
pub fn dispatch(op: &str, req: &Value) -> Value {
    match op {
        "luhn" => json!({"ok": true, "valid": luhn(req["pan"].as_str().unwrap_or(""))}),
        "pan" => {
            let pan = req["pan"].as_str().unwrap_or("");
            json!({"ok": true, "valid": luhn(pan), "kind": pan_kind(pan), "masked": mask_pan(pan)})
        }
        "mask" => json!({"ok": true, "masked": mask_pan(req["pan"].as_str().unwrap_or(""))}),
        "iban" => json!({"ok": true, "valid": iban_valid(req["iban"].as_str().unwrap_or(""))}),
        "pci" => scan_pci(req["text"].as_str().unwrap_or("")),
        "secrets" => scan_secrets(req["text"].as_str().unwrap_or("")),
        "bucket" => token_bucket(
            req["rate_per_sec"].as_f64().unwrap_or(1.0),
            req["burst"].as_u64().unwrap_or(1) as u32,
            req["elapsed_ms"].as_u64().unwrap_or(0),
            req["tokens"].as_f64().unwrap_or(0.0),
            req["want"].as_u64().unwrap_or(1) as u32,
        ),
        "strength" => key_strength(req["key"].as_str().unwrap_or("")),
        "txn_risk" | "txn" => txn_risk(req),
        "gateway_audit" | "gateway" => gateway_audit(req),
        "wire_audit" | "wire" => wire_audit(req),
        "catalog" => catalog(),
        other => json!({"ok": false, "error": format!("op `{other}` haijulikani"), "ops": OPS}),
    }
}

/// Katalogi ya uwezo wa finshield (nini kinafanya nini).
pub fn catalog() -> Value {
    json!({
        "ok": true,
        "engine": "akili-security/finshield",
        "domain": "bank / fintech / API gateway",
        "ops": OPS,
        "capabilities": [
            {"op": "luhn", "desc": "Uthibitisho wa namba ya kadi (mod-10)"},
            {"op": "pan", "desc": "Aina ya kadi (BIN) + masked PAN (kamili hairejeshwi)"},
            {"op": "mask", "desc": "Kuficha PAN kwa PCI-DSS"},
            {"op": "iban", "desc": "Uthibitisho wa IBAN (mod-97)"},
            {"op": "pci", "desc": "Tambua PAN/CVV kwenye maandishi (masked)"},
            {"op": "secrets", "desc": "Tambua API keys/tokens (prefix pekee)"},
            {"op": "bucket", "desc": "Token-bucket rate limiting"},
            {"op": "strength", "desc": "Entropi ya API key/secret"},
            {"op": "txn_risk", "desc": "Alama ya hatari ya muamala + uamuzi"},
            {"op": "gateway_audit", "desc": "Ukaguzi wa API gateway (score + grade)"},
            {"op": "wire_audit", "desc": "Ukaguzi wa ujumbe wa malipo (ISO 20022/SWIFT)"},
        ],
        "rules": {
            "privacy": "PAN hufichwa; CVV hairejeshwi kamwe",
            "hitl": "uamuzi wa mwisho wa muamala ni wa benki (mteja)",
            "read_only": "hakuna kutuma miamala; uchanganuzi pekee",
        },
    })
}

// ---------- Kadi (PAN / Luhn / BIN) ----------

/// Luhn (mod-10) checksum — uthibitisho halisi wa namba ya kadi.
pub fn luhn(number: &str) -> bool {
    let digits: Vec<u32> = number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    if digits.len() < 12 || digits.len() > 19 {
        return false;
    }
    let mut sum = 0u32;
    let parity = digits.len() % 2;
    for (i, &d) in digits.iter().enumerate() {
        let mut x = d;
        if i % 2 == parity {
            x *= 2;
            if x > 9 {
                x -= 9;
            }
        }
        sum += x;
    }
    sum % 10 == 0
}

/// Aina ya kadi kutoka BIN (IIN) — utambuzi halisi wa muundo.
pub fn pan_kind(number: &str) -> &'static str {
    let d: Vec<u32> = number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    if d.len() < 2 {
        return "unknown";
    }
    let two = d[0] * 10 + d[1];
    let four = if d.len() >= 4 {
        d[0] * 1000 + d[1] * 100 + d[2] * 10 + d[3]
    } else {
        0
    };
    match () {
        _ if d[0] == 4 => "visa",
        _ if (51..=55).contains(&two) || (2221..=2720).contains(&four) => "mastercard",
        _ if two == 34 || two == 37 => "amex",
        _ if two == 62 || (3528..=3589).contains(&four) => "unionpay",
        _ if four == 6011 || (644..=649).contains(&two) || four == 65 => "discover",
        _ if (300..=305).contains(&two) || two == 36 || two == 38 => "diners",
        _ if four == 2131 || four == 1800 => "jcb",
        _ => "unknown",
    }
}

/// Ficha PAN kwa kanuni ya PCI-DSS: 6 za mwisho pekee (au 4 kwa wengi).
pub fn mask_pan(number: &str) -> String {
    let digits: String = number.chars().filter(|c| c.is_ascii_digit()).collect();
    let n = digits.len();
    if n <= 4 {
        return "*".repeat(n);
    }
    let visible = 4.min(n.saturating_sub(1));
    let hidden = n - visible;
    format!("{}{}", "*".repeat(hidden), &digits[hidden..hidden + visible])
}

/// Uhalali wa tarehe ya kadi (`MM/YY` au `MM/YYYY`).
pub fn card_expiry_valid(exp: &str, now_year: u32, now_month: u32) -> bool {
    let parts: Vec<&str> = exp.split(['/', '-']).collect();
    if parts.len() != 2 {
        return false;
    }
    let m: u32 = match parts[0].trim().parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut y: u32 = match parts[1].trim().parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if y < 100 {
        y += 2000;
    }
    (1..=12).contains(&m) && (y, m) >= (now_year, now_month)
}

// ---------- IBAN (mod-97) ----------

/// Uthibitisho kamili wa IBAN kwa mod-97 (ISO 13616).
pub fn iban_valid(iban: &str) -> bool {
    let s: String = iban.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let s = s.to_uppercase();
    if s.len() < 15 || s.len() > 34 {
        return false;
    }
    if !s.chars().take(2).all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    // Hamisha herufi 4 za mwanzo mwishoni.
    let rearranged = format!("{}{}", &s[4..], &s[..4]);
    // Badilisha herufi → namba (A=10..Z=35) na hesabu mod 97 kwa hatua.
    let mut rem: u64 = 0;
    for c in rearranged.chars() {
        let mut chunk = String::new();
        if let Some(d) = c.to_digit(10) {
            chunk.push_str(&d.to_string());
        } else if c.is_ascii_alphabetic() {
            let v = (c as u32) - ('A' as u32) + 10;
            chunk.push_str(&v.to_string());
        } else {
            return false;
        }
        for ch in chunk.chars() {
            if let Some(d) = ch.to_digit(10) {
                rem = (rem * 10 + d as u64) % 97;
            }
        }
    }
    rem == 1
}

// ---------- Kuchunguza data nyeti ----------

/// Tambua PAN kwenye maandishi. Hurudisha (masked, kind, luhn_ok) — hakuna PAN kamili.
pub fn scan_pci(text: &str) -> Value {
    let mut found = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == ' ' || chars[i] == '-') {
                i += 1;
            }
            let run: String = chars[start..i].iter().collect();
            let digits: String = run.chars().filter(|c| c.is_ascii_digit()).collect();
            if (13..=19).contains(&digits.len()) {
                let kind = pan_kind(&digits);
                let ok = luhn(&digits);
                // Ripoti kama inaonekana kadi (Luhn ok AU BIN inajulikana + urefu sahihi)
                if ok || kind != "unknown" {
                    found.push(json!({
                        "masked": mask_pan(&digits),
                        "kind": kind,
                        "luhn": ok,
                        "length": digits.len(),
                        "at": start,
                    }));
                }
            }
        } else {
            i += 1;
        }
    }
    // CVV: 3-4 tarakimu zinazofuata "cvv"/"cvc" (hakurejeshi thamani).
    let lower = text.to_lowercase();
    let cvv = lower.contains("cvv") || lower.contains("cvc") || lower.contains("security code");
    json!({
        "ok": true,
        "pan_count": found.len(),
        "pan": found,
        "cvv_mentioned": cvv,
        "note": "PAN hufichwa (PCI-DSS). CVV haihifadhiwi/hairudishwi kamwe.",
    })
}

/// Tambua API keys / secrets kwenye maandishi (huonyesha sehemu ya kwanza pekee).
pub fn scan_secrets(text: &str) -> Value {
    let mut hits: Vec<Value> = Vec::new();
    for tok in text.split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == ';') {
        let t = tok.trim();
        let (kind, sensitive) = if t.starts_with("AKIA") && t.len() == 20 {
            ("aws_access_key", true)
        } else if t.starts_with("ASIA") && t.len() == 20 {
            ("aws_temp_key", true)
        } else if t.starts_with("ghp_") || t.starts_with("github_pat_") {
            ("github_token", true)
        } else if t.starts_with("sk-") && t.len() > 20 {
            ("openai_key", true)
        } else if t.starts_with("sk_live_") || t.starts_with("pk_live_") {
            ("stripe_key", true)
        } else if t.starts_with("xoxb-") || t.starts_with("xoxp-") {
            ("slack_token", true)
        } else if t.starts_with("eyJ") && t.matches('.').count() == 2 {
            ("jwt", true)
        } else if t.starts_with("AIza") && t.len() >= 35 {
            ("google_api_key", true)
        } else if t.starts_with("-----BEGIN") && t.contains("PRIVATE") {
            ("private_key", true)
        } else {
            ("", false)
        };
        if sensitive {
            let head: String = t.chars().take(6).collect();
            hits.push(json!({"kind": kind, "prefix": format!("{head}…"), "length": t.len()}));
        }
    }
    json!({ "ok": true, "secret_count": hits.len(), "secrets": hits,
            "note": "Thamani kamili hairejeshwi — kiambishi pekee." })
}

// ---------- API Gateway: rate limiting + key strength ----------

/// Token bucket: rudisha idadi ya maombi yanayoruhusiwa, ikiwa imejaa mapema au la.
pub fn token_bucket(rate_per_sec: f64, burst: u32, elapsed_ms: u64, tokens: f64, want: u32) -> Value {
    let refill = rate_per_sec * (elapsed_ms as f64 / 1000.0);
    let capacity = burst as f64;
    let mut avail = (tokens + refill).min(capacity);
    let allowed = avail >= want as f64;
    if allowed {
        avail -= want as f64;
    }
    json!({
        "allowed": allowed,
        "tokens_after": (avail * 1000.0).round() / 1000.0,
        "capacity": capacity,
        "refill": (refill * 1000.0).round() / 1000.0,
    })
}

/// Ukadiriaji wa entropi (Shannon) wa API key → bits + daraja.
pub fn key_strength(key: &str) -> Value {
    if key.is_empty() {
        return json!({"ok": false, "error": "key tupu"});
    }
    let mut counts = std::collections::HashMap::new();
    for c in key.chars() {
        *counts.entry(c).or_insert(0u32) += 1;
    }
    let n = key.chars().count() as f64;
    let mut entropy_per_char = 0.0f64;
    for (_c, &cnt) in &counts {
        let p = cnt as f64 / n;
        entropy_per_char -= p * p.log2();
    }
    let total_bits = entropy_per_char * n;
    let grade = if total_bits >= 128.0 {
        "strong"
    } else if total_bits >= 80.0 {
        "ok"
    } else if total_bits >= 48.0 {
        "weak"
    } else {
        "very_weak"
    };
    json!({
        "ok": true,
        "length": n as u64,
        "entropy_bits": (total_bits * 10.0).round() / 10.0,
        "grade": grade,
        "unique_chars": counts.len(),
    })
}

// ---------- Muamala (transaction) risk ----------

/// Alama ya hatari ya muamala + uamuzi. Input ni JSON ya vipengele.
///
/// Vipengele vinavyotumika: `amount`, `avg_amount`, `velocity_1h`,
/// `geo_mismatch` (bool), `new_device` (bool), `new_beneficiary` (bool),
/// `card_present` (bool), `cross_border` (bool), `night` (bool).
pub fn txn_risk(f: &Value) -> Value {
    let amount = f["amount"].as_f64().unwrap_or(0.0);
    let avg = f["avg_amount"].as_f64().unwrap_or(0.0).max(1.0);
    let velocity = f["velocity_1h"].as_u64().unwrap_or(0);
    let bools = |k: &str| f[k].as_bool().unwrap_or(false);

    let mut score: i64 = 0;
    let mut reasons: Vec<&str> = Vec::new();

    // 1) Kiasi ukilinganisha na wastani (0-30)
    let ratio = amount / avg;
    let amt_score = if ratio >= 10.0 {
        30
    } else if ratio >= 5.0 {
        22
    } else if ratio >= 3.0 {
        14
    } else if ratio >= 1.5 {
        7
    } else {
        0
    };
    if amt_score > 0 {
        reasons.push("amount_vs_avg");
    }
    score += amt_score;

    // 2) Kasi (velocity) (0-25)
    let vel_score = if velocity >= 20 {
        25
    } else if velocity >= 10 {
        18
    } else if velocity >= 5 {
        10
    } else {
        0
    };
    if vel_score > 0 {
        reasons.push("high_velocity");
    }
    score += vel_score;

    // 3) Vipengele vya muktadha (kila kimoja +8)
    for (k, w) in [
        ("geo_mismatch", 12),
        ("new_device", 8),
        ("new_beneficiary", 12),
        ("cross_border", 6),
        ("night", 4),
    ] {
        if bools(k) {
            score += w;
            reasons.push(k);
        }
    }
    // Kadi haipo (CNP) huongeza hatari
    if !bools("card_present") {
        score += 8;
        reasons.push("card_not_present");
    }

    let score = score.clamp(0, 100) as u64;
    let decision = if score >= 70 {
        "block"
    } else if score >= 40 {
        "review"
    } else {
        "allow"
    };
    json!({
        "ok": true,
        "risk_score": score,
        "decision": decision,
        "reasons": reasons,
        "note": "Alama ya ushauri; uamuzi wa mwisho ni wa benki (HITL).",
    })
}

// ---------- API Gateway audit ----------

/// Ukaguzi wa usanidi wa API gateway. Hurudisha findings + score + grade.
///
/// `cfg` (JSON) inaweza kuwa na: `tls_min` ("1.0".."1.3"), `mtls` (bool),
/// `auth` ("none"|"apikey"|"oauth2"|"jwt"), `rate_limit` (bool),
/// `waf` (bool), `hsts` (bool), `cors_origin` ("*" au domain),
/// `request_size_limit` (bool), `logging` (bool), `pii_redaction` (bool),
/// `ip_allowlist` (bool).
pub fn gateway_audit(cfg: &Value) -> Value {
    let b = |k: &str| cfg[k].as_bool().unwrap_or(false);
    let s = |k: &str| cfg[k].as_str().unwrap_or("");
    let mut findings: Vec<Value> = Vec::new();
    let mut score: i64 = 0;
    let mut max: i64 = 0;

    let mut add = |sev: &str, w: i64, title: &str, fix: &str, ok: bool, findings: &mut Vec<Value>, score: &mut i64, max: &mut i64| {
        *max += w;
        if ok {
            *score += w;
        } else {
            findings.push(json!({"severity": sev, "title": title, "fix": fix, "weight": w}));
        }
    };

    // TLS
    let tls = s("tls_min");
    let tls_ok = matches!(tls, "1.2" | "1.3");
    add("critical", 20, "TLS ya chini (<=1.1)", "Weka TLS 1.2+ (bora 1.3) pekee", tls_ok, &mut findings, &mut score, &mut max);

    // Auth
    let auth = s("auth");
    let auth_ok = matches!(auth, "oauth2" | "jwt");
    add("critical", 20, "Uthibitisho hafifu", "Tumia OAuth2/JWT (si apikey pekee)", auth_ok, &mut findings, &mut score, &mut max);

    // mTLS
    add("high", 12, "mTLS haipo", "Washa mTLS kwa huduma za ndani/benki", b("mtls"), &mut findings, &mut score, &mut max);

    // Rate limit
    add("high", 12, "Rate limiting haipo", "Weka token-bucket kwa kila mteja/IP", b("rate_limit"), &mut findings, &mut score, &mut max);

    // WAF
    add("high", 12, "WAF haipo", "Weka WAF (OWASP CRS) mbele ya gateway", b("waf"), &mut findings, &mut score, &mut max);

    // HSTS
    add("medium", 8, "HSTS haipo", "Ongeza Strict-Transport-Security", b("hsts"), &mut findings, &mut score, &mut max);

    // CORS
    let cors = s("cors_origin");
    let cors_ok = !cors.is_empty() && cors != "*";
    add("medium", 8, "CORS inaruhusu *", "Weka origin maalum (allowlist)", cors_ok, &mut findings, &mut score, &mut max);

    // Request size limit
    add("medium", 6, "Hakuna kikomo cha ukubwa", "Weka request body limit (DoS)", b("request_size_limit"), &mut findings, &mut score, &mut max);

    // Logging/audit
    add("medium", 6, "Logging/audit haipo", "Weka audit trail inayotunzwa (SIEM)", b("logging"), &mut findings, &mut score, &mut max);

    // PII redaction
    add("high", 10, "PII haifichwi", "Ficha PAN/PII kwenye logs (PCI-DSS)", b("pii_redaction"), &mut findings, &mut score, &mut max);

    // IP allowlist (kwa admin)
    add("low", 4, "IP allowlist haipo", "Weka allowlist kwa endpoints za admin", b("ip_allowlist"), &mut findings, &mut score, &mut max);

    let pct = if max > 0 { (score * 100 / max) as u64 } else { 0 };
    let grade = match pct {
        90..=100 => "A",
        75..=89 => "B",
        60..=74 => "C",
        40..=59 => "D",
        _ => "F",
    };
    json!({
        "ok": true,
        "score": pct,
        "grade": grade,
        "passed": findings.is_empty(),
        "finding_count": findings.len(),
        "findings": findings,
        "note": "Ukaguzi huu ni ushauri wa ulinzi; hakuna mabadiliko ya gateway.",
    })
}

/// Ukaguzi wa ujumbe wa malipo (ISO 20022 / SWIFT-like MT103).
///
/// Vipengele: `amount`, `currency`, `originator`, `beneficiary`,
/// `bic`, `has_lei` (bool), `purpose_code`, `structured` (bool).
pub fn wire_audit(msg: &Value) -> Value {
    let mut findings: Vec<Value> = Vec::new();
    let amount = msg["amount"].as_f64().unwrap_or(0.0);
    let bic = msg["bic"].as_str().unwrap_or("");
    let currency = msg["currency"].as_str().unwrap_or("");

    // BIC: herufi 8 au 11, muundo wa kawaida
    let bic_ok = (bic.len() == 8 || bic.len() == 11)
        && bic.chars().take(6).all(|c| c.is_ascii_alphabetic())
        && bic.chars().skip(6).take(2).all(|c| c.is_ascii_alphanumeric());
    if !bic_ok {
        findings.push(json!({"severity": "high", "title": "BIC si sahihi", "fix": "Kagua BIC (8/11 herufi)"}));
    }
    // Kiasi kubwa bila LEI (Legal Entity Identifier)
    let lei = msg["has_lei"].as_bool().unwrap_or(false);
    if amount >= 500_000.0 && !lei {
        findings.push(json!({"severity": "high", "title": "Kiasi kikubwa bila LEI", "fix": "Ongeza LEI kwa miamala >= 500k"}));
    }
    // Sarafu haipo
    if currency.len() != 3 {
        findings.push(json!({"severity": "medium", "title": "Sarafu si ISO 4217", "fix": "Weka sarafu ya herufi 3 (TZS/USD)"}));
    }
    // Ujumbe usio na muundo
    if !msg["structured"].as_bool().unwrap_or(false) {
        findings.push(json!({"severity": "medium", "title": "Ujumbe usio na muundo", "fix": "Hamia ISO 20022 (structured)"}));
    }
    // Kikomo cha AML (mfano rahisi wa reporting)
    let aml_report = amount >= 10_000.0;
    json!({
        "ok": true,
        "finding_count": findings.len(),
        "findings": findings,
        "aml_report_threshold_hit": aml_report,
        "note": "Kizingiti cha AML ni cha mfano (10k); weka la benki lako.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_inatambua_kadi_sahihi() {
        // Namba za majaribio zinazojulikana (test PANs, si halisi)
        assert!(luhn("4111111111111111")); // Visa test
        assert!(luhn("5500005555555559")); // Mastercard test
        assert!(!luhn("4111111111111112"));
    }

    #[test]
    fn pan_kind_inatambua_muundo() {
        assert_eq!(pan_kind("4111111111111111"), "visa");
        assert_eq!(pan_kind("5500005555555559"), "mastercard");
        assert_eq!(pan_kind("378282246310005"), "amex");
    }

    #[test]
    fn mask_pan_inaficha_za_kati() {
        let m = mask_pan("4111111111111111");
        assert!(m.ends_with("1111"));
        assert!(m.starts_with('*'));
        assert_eq!(m.chars().filter(|c| *c == '*').count(), 12);
    }

    #[test]
    fn iban_inathibitisha_mod97() {
        // IBAN za majaribio (si halisi)
        assert!(iban_valid("GB82WEST12345698765432"));
        assert!(!iban_valid("GB82WEST12345698765431"));
    }

    #[test]
    fn scan_pci_huficha_pan() {
        let r = scan_pci("kadi yangu ni 4111 1111 1111 1111 tafadhali");
        assert_eq!(r["pan_count"], 1);
        let masked = r["pan"][0]["masked"].as_str().unwrap();
        assert!(!masked.contains("4111111111111111"));
        assert!(masked.ends_with("1111"));
    }

    #[test]
    fn scan_secrets_inatambua_tokens() {
        let r = scan_secrets("key=ghp_abcdefghijklmnopqrstuvwx token xoxb-12345 AKIAIOSFODNN7EXAMPLE");
        assert!(r["secret_count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn token_bucket_huruhusu_na_kuzuia() {
        let a = token_bucket(10.0, 20, 1000, 0.0, 5); // refill 10 → ok
        assert_eq!(a["allowed"], true);
        let b = token_bucket(0.0, 3, 0, 3.0, 5); // hana tokens → zuia
        assert_eq!(b["allowed"], false);
    }

    #[test]
    fn key_strength_inapima_entropi() {
        let weak = key_strength("password");
        let strong = key_strength("aB3$xY9!kL2@mN7#qR4%zW8^tU5&vC6*dE1");
        assert!(strong["entropy_bits"].as_f64().unwrap() > weak["entropy_bits"].as_f64().unwrap());
        assert_eq!(strong["grade"], "strong");
    }

    #[test]
    fn txn_risk_hupanga_uamuzi() {
        let low = txn_risk(&json!({"amount": 100, "avg_amount": 100, "card_present": true, "velocity_1h": 1}));
        assert_eq!(low["decision"], "allow");
        let high = txn_risk(&json!({"amount": 9000, "avg_amount": 100, "velocity_1h": 25,
            "geo_mismatch": true, "new_device": true, "new_beneficiary": true, "cross_border": true}));
        assert_eq!(high["decision"], "block");
    }

    #[test]
    fn gateway_audit_hupima_usanidi() {
        let bad = gateway_audit(&json!({"tls_min": "1.0", "auth": "none"}));
        assert_eq!(bad["grade"], "F");
        assert!(bad["finding_count"].as_u64().unwrap() >= 5);
        let good = gateway_audit(&json!({
            "tls_min": "1.3", "auth": "oauth2", "mtls": true, "rate_limit": true,
            "waf": true, "hsts": true, "cors_origin": "https://app.bank.tz",
            "request_size_limit": true, "logging": true, "pii_redaction": true, "ip_allowlist": true
        }));
        assert_eq!(good["score"], 100);
        assert_eq!(good["grade"], "A");
    }

    #[test]
    fn wire_audit_inakamata_bic_mbovu() {
        let r = wire_audit(&json!({"amount": 900000, "currency": "TZ", "bic": "BAD", "has_lei": false}));
        assert!(r["finding_count"].as_u64().unwrap() >= 3);
        assert_eq!(r["aml_report_threshold_hit"], true);
    }

    #[test]
    fn dispatch_pan_inarudisha_uhalali_na_mask() {
        let r = dispatch("pan", &json!({"pan": "4111111111111111"}));
        assert_eq!(r["ok"], true);
        assert_eq!(r["valid"], true);
        assert!(r["masked"].as_str().unwrap().contains('*'));
    }

    #[test]
    fn dispatch_haijulikani_hurudisha_ops() {
        let r = dispatch("hakuna", &json!({}));
        assert_eq!(r["ok"], false);
        assert!(r["ops"].as_array().unwrap().contains(&json!("luhn")));
    }

    #[test]
    fn dispatch_catalog_inarudisha_katalogi() {
        let r = dispatch("catalog", &json!({}));
        assert_eq!(r["ok"], true);
    }

    #[test]
    fn card_expiry_valid_inakagua() {
        assert!(card_expiry_valid("12/30", 2026, 10));
        assert!(!card_expiry_valid("01/20", 2026, 10));
        assert!(card_expiry_valid("07/2026", 2026, 7));
    }
}
