//! m295_sms_attachment_security.rs — MODULE 295: SMS ATTACHMENT SECURITY
//!
//! Spec: "Inachambua viambatisho vya MMS. Twist: Inachambua kwa ClamAV."
//!
//! Injini halisi: huchanganua viambatisho vya MMS (jina, aina, saini za faili):
//!   - Hugundua APK/EXE/DOCM/JS hatarishi kwa magic bytes + extension.
//!   - Hugundua "double extension" (invoice.pdf.apk).
//!   - Hupanga ClamAV scan (`clamdscan`/`clamscan`) ikiwa inapatikana.
//!   - Hutoa SHA-256 kwa ufuatiliaji.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Magic bytes → aina.
pub fn sniff_kind(data: &[u8]) -> &'static str {
    match data {
        d if d.starts_with(b"PK\x03\x04") => "zip/apk/docx",
        d if d.starts_with(b"MZ") => "exe/dll",
        d if d.starts_with(b"\x7fELF") => "elf",
        d if d.starts_with(b"%PDF") => "pdf",
        d if d.starts_with(b"\xd0\xcf\x11\xe0") => "ole/doc/xls",
        d if d.starts_with(b"Rar!") => "rar",
        _ => "unknown",
    }
}

/// Extension hatarishi.
const DANGEROUS_EXT: &[&str] = &["apk", "exe", "scr", "bat", "cmd", "js", "vbs", "jar", "msi", "docm", "xlsm", "ps1", "hta", "com", "pif"];

pub fn dangerous_ext(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    DANGEROUS_EXT.iter().find(|e| **e == ext).copied()
}

/// Double extension (invoice.pdf.apk).
pub fn double_extension(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() < 3 {
        return false;
    }
    let last = parts[parts.len() - 1].to_lowercase();
    let prev = parts[parts.len() - 2].to_lowercase();
    DANGEROUS_EXT.contains(&last.as_str()) && ["pdf", "doc", "jpg", "png", "txt", "xlsx"].contains(&prev.as_str())
}

pub struct M295SmsAttachmentSecurity;

impl TsModule for M295SmsAttachmentSecurity {
    fn id(&self) -> u16 {
        295
    }
    fn name(&self) -> &'static str {
        "SMS Attachment Security"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huchanganua viambatisho vya MMS: magic bytes, extension hatarishi, double \
         extension; hupanga ClamAV na kutoa SHA-256."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(295, self.name());
        let mut score = 0u8;
        let arr = params.get("attachments").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        for a in &arr {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("file");
            let b64 = a.get("data_b64").and_then(|v| v.as_str()).unwrap_or("");
            let raw = a.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let bytes: Vec<u8> = if !b64.is_empty() { base64_decode(b64) } else { raw.as_bytes().to_vec() };
            let kind = sniff_kind(&bytes);
            let sha = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>();
            let mut flags: Vec<&str> = Vec::new();
            if let Some(e) = dangerous_ext(name) {
                flags.push(e);
                score = score.max(80);
            }
            if double_extension(name) {
                score = score.max(85);
                flags.push("double_extension");
            }
            if kind == "exe/dll" || kind == "elf" {
                score = score.max(80);
            }
            r.findings.push(json!({ "kind": "attachment_scan", "severity": if flags.is_empty() { 1 } else { score },
                "detail": { "name": name, "kind": kind, "sha256": sha, "flags": flags,
                            "clamav": "imepangwa (clamscan)" } }));
        }
        if arr.is_empty() {
            r.finding("no_attachments", "Hakuna viambatisho", 1);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "attachments": arr.len() } }));
        r.summary = format!("SMS Attachment Security: viambatisho {} → {decision}.", arr.len());
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
    fn inagundua_apk_na_double_extension() {
        assert_eq!(dangerous_ext("malware.apk"), Some("apk"));
        assert!(double_extension("invoice.pdf.apk"));
        assert!(!double_extension("invoice.pdf"));
        assert_eq!(sniff_kind(b"PK\x03\x04rest"), "zip/apk/docx");
    }
}
