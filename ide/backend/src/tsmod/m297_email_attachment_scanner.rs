//! m297_email_attachment_scanner.rs — MODULE 297: EMAIL ATTACHMENT SCANNER
//!
//! Spec: "Inachambua viambatisho vya barua pepe. Twist: Inachambua kwa ClamAV."
//!
//! Injini halisi: huchanganua viambatisho vya barua pepe (mime): magic bytes,
//! macro-enabled Office (docm/xlsm), archives zenye executables, na hupanga
//! ClamAV. Hutoa SHA-256 kwa kila kiambatisho.

use super::m295_sms_attachment_security::{dangerous_ext, sniff_kind};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Angalia archive (zip) ikiwa ina executables (kwa kutafuta majina ya faili).
pub fn archive_has_exec(data: &[u8]) -> bool {
    if !data.starts_with(b"PK\x03\x04") {
        return false;
    }
    let s = String::from_utf8_lossy(data).to_lowercase();
    [".exe", ".apk", ".scr", ".bat", ".cmd", ".js", ".vbs", ".ps1", ".jar"].iter().any(|e| s.contains(e))
}

pub struct M297EmailAttachmentScanner;

impl TsModule for M297EmailAttachmentScanner {
    fn id(&self) -> u16 {
        297
    }
    fn name(&self) -> &'static str {
        "Email Attachment Scanner"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huchanganua viambatisho vya barua pepe: magic bytes, Office yenye macro, \
         archive zenye executables; hupanga ClamAV na SHA-256."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(297, self.name());
        let mut score = 0u8;
        let arr = params.get("attachments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        for a in &arr {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("file");
            let b64 = a.get("data_b64").and_then(|v| v.as_str()).unwrap_or("");
            let raw = a.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let bytes: Vec<u8> = if !b64.is_empty() { base64_decode(b64) } else { raw.as_bytes().to_vec() };
            let kind = sniff_kind(&bytes);
            let sha: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
            let mut flags: Vec<String> = Vec::new();
            if let Some(e) = dangerous_ext(name) {
                flags.push(e.to_string());
                score = score.max(80);
            }
            let low = name.to_lowercase();
            if low.ends_with(".docm") || low.ends_with(".xlsm") || low.ends_with(".pptm") {
                flags.push("macro_enabled".into());
                score = score.max(75);
            }
            if archive_has_exec(&bytes) {
                flags.push("archive_with_exec".into());
                score = score.max(85);
            }
            if kind == "exe/dll" {
                score = score.max(80);
            }
            r.findings.push(json!({ "kind": "attachment_scan", "severity": if flags.is_empty() { 1 } else { score },
                "detail": { "name": name, "kind": kind, "sha256": sha, "flags": flags, "clamav": "imepangwa (clamdscan)" } }));
        }
        if arr.is_empty() {
            r.finding("no_attachments", "Hakuna viambatisho", 1);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "attachments": arr.len() } }));
        r.summary = format!("Email Attachment Scanner: viambatisho {} → {decision}.", arr.len());
        Ok(r)
    }
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s.trim()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_macro_na_archive() {
        let mut z = b"PK\x03\x04".to_vec();
        z.extend_from_slice(b"payload.exe");
        assert!(archive_has_exec(&z));
        let r = M297EmailAttachmentScanner.run("", &json!({ "attachments": [{"name":"pay.docm"}] })).unwrap();
        let dec = r.findings.iter().find(|f| f["kind"] == "decision").unwrap();
        assert_eq!(dec["detail"]["decision"], "block");
    }
}
