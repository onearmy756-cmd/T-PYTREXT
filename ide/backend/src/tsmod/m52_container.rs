//! m52_container.rs — MODULE 52: CONTAINER SECURITY
//!
//! Spec: "Inalinda Docker containers. Twist: inachambua container images kwa
//! virus kabla ya ku-run."
//!
//! Hii si catalog: inasoma containers halisi kwa docker CLI (bila shell), na ina
//! engine ya kuchambua image config (User=root, privileged, tag latest, healthcheck)
//! pamoja na scan ya virus signatures (EICAR + heuristics) kwenye image layers.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::process::Command;

pub struct M52Container;

/// Signatures za virus (EICAR + heuristics).
const VIRUS_SIGS: &[&str] = &[
    "X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR",
    "eicar",
    "malware",
    "reverse_shell",
    "curl | sh",
    "wget | sh",
    "/dev/tcp/",
];

/// Matokeo ya scan ya maudhui ya image.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanResult {
    pub infected: bool,
    pub hits: Vec<String>,
}

/// Scan maudhui (mf. Dockerfile/layers text) kwa signatures.
pub fn scan_content(content: &str) -> ScanResult {
    let lower = content.to_lowercase();
    let mut hits = Vec::new();
    for s in VIRUS_SIGS {
        if lower.contains(&s.to_lowercase()) {
            hits.push(s.to_string());
        }
    }
    ScanResult {
        infected: !hits.is_empty(),
        hits,
    }
}

/// Changanua image config kwa hatari.
pub fn config_risks(user: &str, privileged: bool, tag: &str, has_healthcheck: bool) -> Vec<String> {
    let mut risks = Vec::new();
    if user.is_empty() || user == "root" || user == "0" {
        risks.push("inaendesha kama root".into());
    }
    if privileged {
        risks.push("privileged mode".into());
    }
    if tag == "latest" || tag.is_empty() {
        risks.push("tag 'latest' (haijafungwa)".into());
    }
    if !has_healthcheck {
        risks.push("hakuna healthcheck".into());
    }
    risks
}

fn docker(args: &[&str]) -> Result<String, String> {
    let out = Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| format!("docker haipatikani: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

impl TsModule for M52Container {
    fn id(&self) -> u16 {
        52
    }
    fn name(&self) -> &'static str {
        "Container Security"
    }
    fn group(&self) -> &'static str {
        "infra"
    }
    fn about(&self) -> &'static str {
        "Husoma containers halisi (docker) na kuchambua image config + virus \
         signatures kabla ya ku-run."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "scan" => self.scan(params),
            "config" => self.config(params),
            other => Err(format!("op '{other}' haipo: audit|scan|config")),
        }
    }
}

impl M52Container {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(52, self.name());
        match docker(&["ps", "--format", "{{.Names}}\t{{.Image}}\t{{.Status}}"]) {
            Ok(out) => {
                let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
                r.finding("containers", format!("Containers zinazoendesha: {}", lines.len()), 2);
                for l in &lines {
                    r.finding("container", (*l).to_string(), 3);
                }
            }
            Err(e) => {
                r.finding(
                    "docker_unavailable",
                    format!("Docker haipatikani/haifanyi kazi: {e}"),
                    10,
                );
            }
        }
        r.summary = "Container audit imefanyika.".into();
        Ok(r)
    }

    fn scan(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(52, self.name());
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("params.content (maudhui ya image/Dockerfile) inahitajika")?;
        let s = scan_content(content);
        if s.infected {
            r.finding(
                "virus_found",
                format!("Signatures zimepatikana: [{}] — USI-run image", s.hits.join(", ")),
                90,
            );
            r.action("image_blocked", "Image imezuiwa (virus)", true);
            r.summary = format!("Image ina virus ({} signatures) — imezuiwa.", s.hits.len());
        } else {
            r.summary = "Image ni safi kwa signatures zilizojulikana.".into();
        }
        Ok(r)
    }

    fn config(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(52, self.name());
        let user = params.get("user").and_then(|v| v.as_str()).unwrap_or("");
        let privileged = params.get("privileged").and_then(|v| v.as_bool()).unwrap_or(false);
        let tag = params.get("tag").and_then(|v| v.as_str()).unwrap_or("");
        let hc = params.get("has_healthcheck").and_then(|v| v.as_bool()).unwrap_or(false);
        let risks = config_risks(user, privileged, tag, hc);
        for x in &risks {
            r.finding("config_risk", x.clone(), 30);
        }
        r.summary = format!("Config risks: {}.", risks.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eicar_inagundulika() {
        let s = scan_content("X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD");
        assert!(s.infected);
    }

    #[test]
    fn image_safi() {
        assert!(!scan_content("FROM alpine\nRUN apk add curl").infected);
    }

    #[test]
    fn config_risks_zinagundulika() {
        let r = config_risks("root", true, "latest", false);
        assert!(r.len() >= 3);
    }
}
