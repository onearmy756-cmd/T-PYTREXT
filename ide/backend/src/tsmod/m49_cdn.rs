//! m49_cdn.rs — MODULE 49: CDN SECURITY
//!
//! Spec: "Inahakikisha CDN inatumika kwa usalama. Twist: inagundua CDN hijacking
//! (content inabadilishwa)."
//!
//! Hii si catalog: inakokotoa hash halisi (SHA-256) ya maudhui na kulinganisha
//! na hash inayotarajiwa → kugundua content tampering/CDN hijacking. Pia husoma
//! headers za CDN.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub struct M49Cdn;

/// SHA-256 hex ya maudhui.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// Changanua header za CDN; rudisha jina la CDN kama linajulikana.
pub fn detect_cdn(headers: &Value) -> Option<String> {
    let obj = headers.as_object()?;
    let get = |k: &str| -> String {
        obj.iter()
            .find(|(kk, _)| kk.eq_ignore_ascii_case(k))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    if !get("cf-ray").is_empty() || get("server").to_lowercase().contains("cloudflare") {
        return Some("cloudflare".into());
    }
    if !get("x-amz-cf-id").is_empty() {
        return Some("cloudfront".into());
    }
    if !get("x-akamai-transformed").is_empty() || !get("x-akamai-request-id").is_empty() {
        return Some("akamai".into());
    }
    if !get("x-served-by").is_empty() && get("x-served-by").to_lowercase().contains("cache") {
        return Some("fastly".into());
    }
    None
}

impl TsModule for M49Cdn {
    fn id(&self) -> u16 {
        49
    }
    fn name(&self) -> &'static str {
        "CDN Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Hukokotoa hash halisi (SHA-256) ya maudhui na kugundua CDN hijacking \
         (content inabadilishwa); hutambua CDN kutoka headers."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "verify" => self.verify(params),
            other => Err(format!("op '{other}' haipo: audit|verify")),
        }
    }
}

impl M49Cdn {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(49, self.name());
        r.finding(
            "cdn_audit",
            "Hakikisha maudhui yanayotolewa na CDN yana hash inayolingana na origin",
            2,
        );
        r.summary = "CDN audit imefanyika.".into();
        Ok(r)
    }

    fn verify(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(49, self.name());
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("params.content inahitajika")?;
        let expected = params
            .get("expected_sha256")
            .and_then(|v| v.as_str())
            .ok_or("params.expected_sha256 inahitajika")?;
        let actual = sha256_hex(content.as_bytes());
        let cdn = params
            .get("headers")
            .and_then(detect_cdn)
            .unwrap_or_else(|| "unknown".into());
        let ok = actual.eq_ignore_ascii_case(expected);
        r.findings.push(json!({
            "kind": "content_integrity", "severity": if ok {2} else {85},
            "detail": { "expected": expected, "actual": actual, "cdn": cdn, "match": ok }
        }));
        if !ok {
            r.finding(
                "cdn_hijack",
                format!("Hash hailingani — content imebadilishwa (CDN {cdn} hijacking?)"),
                85,
            );
        }
        r.summary = format!(
            "CDN {cdn}: integrity {}.",
            if ok { "OK" } else { "IMEVUNJIKA" }
        );
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_ni_sahihi() {
        // echo -n "abc" | sha256sum
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn cdn_inatambulika() {
        let h = json!({"CF-Ray":"abc123","Server":"cloudflare"});
        assert_eq!(detect_cdn(&h).as_deref(), Some("cloudflare"));
    }

    #[test]
    fn hijack_inagundulika() {
        let m = M49Cdn;
        let r = m
            .run("", &json!({ "op": "verify", "content": "hacked", "expected_sha256": "00" }))
            .unwrap();
        assert!(r.findings.iter().any(|f| f["kind"] == "cdn_hijack"));
    }
}
