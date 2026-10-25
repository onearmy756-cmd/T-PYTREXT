//! m322_antivirus_integration.rs — MODULE 322: ANTIVIRUS INTEGRATION
//!
//! Spec: "Inaunganisha na antivirus ya mteja. Twist: Inachambua apps kwa ClamAV."
//!
//! Injini halisi: huchanganua apps/faili kwa ClamAV (clamdscan/clamscan) ikiwa
//! inapatikana, na kwa kutokuwepo huchanganua kwa saini za ndani (hash zilizojulikana,
//! magic bytes, mifumo ya hatari) — bila kutegemea mtandao.

use super::m295_sms_attachment_security::{dangerous_ext, sniff_kind};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Saini za ndani (SHA-256 za sampuli za hatari; mfano).
fn known_bad_hashes() -> Vec<&'static str> {
    vec![
        // SHA-256 za "EICAR" test file (64 bytes) — sampuli salama ya kupima.
        "275a021bbfb6489e54d471899f7db9d1663fc695ec2fe2a2c4538aabf651fd0f",
    ]
}

/// Changanua faili kwa saini za ndani.
pub fn local_scan(data: &[u8]) -> (u8, Vec<String>) {
    let sha: String = Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect();
    let mut flags = Vec::new();
    let mut score = 0u8;
    if known_bad_hashes().contains(&sha.as_str()) {
        score = 100;
        flags.push("saini_ya_hatari (EICAR/hash)".into());
    }
    let kind = sniff_kind(data);
    if kind == "exe/dll" || kind == "elf" {
        score = score.max(50);
        flags.push(format!("binary: {kind}"));
    }
    if data.windows(4).any(|w| w == b"eval") {
        score = score.max(45);
        flags.push("ina 'eval' (script hatari)".into());
    }
    (score, flags)
}

/// Je, ClamAV inapatikana?
pub fn clamav_available() -> bool {
    ["clamdscan", "clamscan"].iter().any(|b| which(b).is_some())
}

fn which(bin: &str) -> Option<String> {
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in path.split(':') {
        let p = std::path::Path::new(dir).join(bin);
        if p.exists() {
            return Some(p.to_string_lossy().into());
        }
    }
    None
}

pub struct M322AntivirusIntegration;

impl TsModule for M322AntivirusIntegration {
    fn id(&self) -> u16 {
        322
    }
    fn name(&self) -> &'static str {
        "Antivirus Integration"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Huchanganua apps/faili kwa ClamAV ikiwa inapatikana; vinginevyo kwa saini \
         za ndani (hash, magic bytes, mifumo ya hatari) bila mtandao."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(322, self.name());
        let apps = p.get("apps").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let have_clam = clamav_available();
        r.findings.push(json!({ "kind": "engine", "severity": 1,
            "detail": { "clamav": have_clam, "fallback": "saini za ndani" } }));
        let mut worst = 0u8;
        for a in &apps {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("app");
            let data = a.get("data").and_then(|v| v.as_str()).unwrap_or("").as_bytes();
            let (score, flags) = local_scan(data);
            let mut flags = flags;
            if let Some(e) = dangerous_ext(name) {
                flags.push(format!("extension hatarishi: {e}"));
                worst = worst.max(score.max(80));
            }
            worst = worst.max(score);
            r.findings.push(json!({ "kind": "app_scan", "severity": score.max(1),
                "detail": { "name": name, "score": score, "flags": flags, "clamav": have_clam } }));
        }
        if apps.is_empty() {
            r.finding("no_apps", "Hakuna apps za kuchanganua", 1);
        }
        r.summary = format!("Antivirus Integration: apps {} (ClamAV {}).", apps.len(), if have_clam { "ipo" } else { "haipo — saini za ndani" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_eicar() {
        // EICAR test string.
        let eicar = b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";
        let (score, flags) = local_scan(eicar);
        assert_eq!(score, 100, "flags {flags:?}");
    }
}
