//! platforms.rs — Katalogi ya amri za majukwaa (Kali, Windows, macOS, Android, UOS).
//!
//! ThreatSwarm (agents 27) hutumia katalogi hii kuchagua **amri sahihi** kwa kila
//! jukwaa. Data ipo kwenye `data/platforms/*.json` (hazijengwi kwenye code) ili
//! kuongeza/kuupdate amri bila kubadilisha Rust.
//!
//! Kanuni:
//!   - **Read-only**: hii ni katalogi + upangaji; HAITELESHI amri.
//!   - **HITL**: amri za `risk: high` zinaalamishwa `hitl: true` (zinahitaji idhini).
//!   - **`$T` placeholder**: hubadilishwa na lengo (target) wakati wa plan.

use serde_json::{json, Value};

/// (jina la jukwaa, JSON yake) — ime-embed kwenye binary.
pub const PLATFORMS: &[(&str, &str)] = &[
    ("kali", include_str!("../data/platforms/kali.json")),
    ("windows", include_str!("../data/platforms/windows.json")),
    ("macos", include_str!("../data/platforms/macos.json")),
    ("android", include_str!("../data/platforms/android.json")),
    ("uos", include_str!("../data/platforms/uos.json")),
];

/// Orodha ya majina ya majukwaa yanayopatikana.
pub fn names() -> Vec<&'static str> {
    PLATFORMS.iter().map(|(n, _)| *n).collect()
}

/// Rekebisha tahajia/alias za majukwaa (mf. "nacos"/"mac"/"osx" → "macos").
fn canonical(platform: &str) -> String {
    let p = platform.to_lowercase();
    match p.as_str() {
        "nacos" | "mac" | "osx" | "mac-os" | "mac_os" | "darwin" => "macos".into(),
        "win" | "windows10" | "windows11" | "win10" | "win11" => "windows".into(),
        "linux" | "kali-linux" | "kali_linux" => "kali".into(),
        "android-adb" | "adb" => "android".into(),
        "uos" | "deepin" | "uniontech" | "linux-deepin" => "uos".into(),
        _ => p,
    }
}

fn raw(platform: &str) -> Option<Value> {
    let p = canonical(platform);
    PLATFORMS
        .iter()
        .find(|(n, _)| *n == p)
        .and_then(|(_, s)| serde_json::from_str(s).ok())
}

/// Katalogi ya muhtasari: jukwaa, shell, idadi ya kategoria na amri.
pub fn catalog() -> Value {
    let platforms: Vec<Value> = PLATFORMS
        .iter()
        .filter_map(|(name, s)| {
            let v: Value = serde_json::from_str(s).ok()?;
            let cats = v["categories"].as_array()?;
            let cmds: usize = cats
                .iter()
                .map(|c| c["commands"].as_array().map(|a| a.len()).unwrap_or(0))
                .sum();
            Some(json!({
                "platform": name,
                "title": v["title"],
                "shell": v["shell"],
                "categories": cats.len(),
                "commands": cmds,
            }))
        })
        .collect();
    let total: u64 = platforms.iter().filter_map(|p| p["commands"].as_u64()).sum();
    json!({
        "ok": true,
        "engine": "akili-security/platforms",
        "platforms": platforms,
        "total_commands": total,
        "note": "Katalogi pekee — haitelezi amri. HITL kwa risk=high.",
    })
}

/// Katalogi kamili ya jukwaa moja (au zote kama `platform` haipo).
pub fn catalog_full(platform: Option<&str>) -> Value {
    match platform {
        Some(p) => match raw(p) {
            Some(v) => json!({ "ok": true, "platform": v }),
            None => json!({
                "ok": false,
                "error": format!("jukwaa `{p}` haijulikani"),
                "platforms": names(),
            }),
        },
        None => {
            let all: Vec<Value> = PLATFORMS
                .iter()
                .filter_map(|(_, s)| serde_json::from_str(s).ok())
                .collect();
            json!({ "ok": true, "platforms": all })
        }
    }
}

fn risk_rank(r: &str) -> u8 {
    match r {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

/// Tafuta amri kwa jukwaa (query inalinganishwa na jina/kategoria/maelezo).
///
/// `min_risk` huhitaji kama `"low"|"medium"|"high"` ili kuchuja.
pub fn lookup(platform: &str, query: &str, min_risk: Option<&str>) -> Value {
    let v = match raw(platform) {
        Some(v) => v,
        None => {
            return json!({
                "ok": false,
                "error": format!("jukwaa `{platform}` haijulikani"),
                "platforms": names(),
            })
        }
    };
    let q = query.to_lowercase();
    let floor = min_risk.map(risk_rank).unwrap_or(0);
    let mut hits = Vec::new();
    if let Some(cats) = v["categories"].as_array() {
        for c in cats {
            let cid = c["id"].as_str().unwrap_or("");
            let ctitle = c["title"].as_str().unwrap_or("");
            if let Some(cmds) = c["commands"].as_array() {
                for cmd in cmds {
                    let hay = format!(
                        "{} {} {} {}",
                        cmd["name"].as_str().unwrap_or(""),
                        cmd["usage"].as_str().unwrap_or(""),
                        cmd["desc"].as_str().unwrap_or(""),
                        cid
                    )
                    .to_lowercase();
                    let risk = cmd["risk"].as_str().unwrap_or("low");
                    if (q.is_empty() || hay.contains(&q)) && risk_rank(risk) >= floor {
                        hits.push(json!({
                            "category": cid,
                            "category_title": ctitle,
                            "name": cmd["name"],
                            "usage": cmd["usage"],
                            "desc": cmd["desc"],
                            "target": cmd["target"],
                            "risk": risk,
                            "needs_root": cmd["needs_root"],
                            "hitl": risk == "high",
                        }));
                    }
                }
            }
        }
    }
    json!({
        "ok": true,
        "platform": platform,
        "query": query,
        "count": hits.len(),
        "commands": hits,
    })
}

/// PLAN: pendekeza amri za kazi fulani kwa jukwaa, ukitumia ramani ya nia→kategoria.
///
/// Hii ni upangaji (read-only); ThreatSwarm agents huamua utekelezaji.
pub fn plan(platform: &str, task: &str, target: &str) -> Value {
    let v = match raw(platform) {
        Some(v) => v,
        None => {
            return json!({
                "ok": false,
                "error": format!("jukwaa `{platform}` haijulikani"),
                "platforms": names(),
            })
        }
    };
    let t = task.to_lowercase();
    // Ramani ya nia → kategoria zinazofaa.
    let wanted: Vec<&str> = if t.contains("recon") || t.contains("osint") || t.contains("enumer") {
        vec!["recon"]
    } else if t.contains("web") || t.contains("http") {
        vec!["web"]
    } else if t.contains("api") || t.contains("graphql") || t.contains("grpc") {
        vec!["api"]
    } else if t.contains("cloud") || t.contains("container") || t.contains("kube") || t.contains("aws") {
        vec!["cloud"]
    } else if t.contains("wifi") || t.contains("wireless") {
        vec!["wireless"]
    } else if t.contains("network") || t.contains("exploit") || t.contains("brute") {
        vec!["network", "exploit"]
    } else if t.contains("post") || t.contains("privesc") || t.contains("pivot") {
        vec!["postex"]
    } else if t.contains("ad") || t.contains("active-directory") || t.contains("kerberos") {
        vec!["active-directory"]
    } else if t.contains("forensic") || t.contains("dfir") || t.contains("incident") || t.contains("malware") {
        vec!["dfir"]
    } else if t.contains("gui") || t.contains("desktop") {
        vec!["gui"]
    } else if t.contains("audit") || t.contains("compliance") || t.contains("hardening") {
        vec!["audit", "hardening"]
    } else {
        // kazi isiyojulikana → defensive (recon + audit)
        vec!["recon", "audit"]
    };

    let mut picks = Vec::new();
    if let Some(cats) = v["categories"].as_array() {
        for c in cats {
            let cid = c["id"].as_str().unwrap_or("");
            if !wanted.contains(&cid) {
                continue;
            }
            if let Some(cmds) = c["commands"].as_array() {
                for cmd in cmds.iter().take(8) {
                    let usage = cmd["usage"].as_str().unwrap_or("").replace("$T", target);
                    picks.push(json!({
                        "category": cid,
                        "name": cmd["name"],
                        "usage": usage,
                        "desc": cmd["desc"],
                        "risk": cmd["risk"],
                        "needs_root": cmd["needs_root"],
                        "hitl": cmd["risk"].as_str() == Some("high"),
                    }));
                }
            }
        }
    }
    json!({
        "ok": true,
        "platform": platform,
        "task": task,
        "target": target,
        "categories": wanted,
        "count": picks.len(),
        "commands": picks,
        "note": "Upangaji pekee (read-only). Amri za hitl=true zinahitaji idhini.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn majukwaa_yote_yapo() {
        let n = names();
        for p in ["kali", "windows", "macos", "android", "uos"] {
            assert!(n.contains(&p), "jukwaa {p} halipo");
        }
    }

    #[test]
    fn katalogi_ina_amri_nyingi() {
        let c = catalog();
        assert_eq!(c["ok"], true);
        assert!(c["total_commands"].as_u64().unwrap() > 250);
    }

    #[test]
    fn jukwaa_isiyojulikana_hurudisha_kosa() {
        let r = lookup("beos", "nmap", None);
        assert_eq!(r["ok"], false);
    }

    #[test]
    fn alias_za_majukwaa_zinafanya_kazi() {
        // "nacos" (tahajia ya kawaida) → macos; "win" → windows.
        let m = lookup("nacos", "csrutil", None);
        assert_eq!(m["ok"], true);
        assert!(m["count"].as_u64().unwrap() >= 1);
        let w = lookup("win", "systeminfo", None);
        assert_eq!(w["ok"], true);
    }

    #[test]
    fn lookup_inapata_nmap_kwenye_kali() {
        let r = lookup("kali", "nmap", None);
        assert_eq!(r["ok"], true);
        assert!(r["count"].as_u64().unwrap() >= 1);
        let first = &r["commands"][0];
        assert_eq!(first["name"], "nmap");
        assert_eq!(first["hitl"], false);
    }

    #[test]
    fn lookup_min_risk_huchuja() {
        let r = lookup("kali", "nmap", Some("high"));
        // nmap ni "medium" → haipaswi kuonekana kwenye floor=high
        assert_eq!(r["count"], 0);
    }

    #[test]
    fn plan_hubadilisha_target() {
        let r = plan("kali", "recon", "example.tz");
        assert_eq!(r["ok"], true);
        assert!(r["count"].as_u64().unwrap() >= 1);
        let usages: Vec<&str> = r["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["usage"].as_str())
            .collect();
        assert!(usages.iter().any(|u| u.contains("example.tz") || u.contains("$T") == false));
    }

    #[test]
    fn windows_ina_audit_na_dfir() {
        let a = lookup("windows", "Get-WinEvent", None);
        assert!(a["count"].as_u64().unwrap() >= 1);
        let f = plan("windows", "dfir", "C:\\evidence");
        assert!(f["count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn android_ina_dumpsys() {
        let r = lookup("android", "dumpsys", None);
        assert!(r["count"].as_u64().unwrap() >= 3);
    }

    #[test]
    fn uos_ina_ufw_na_hardening() {
        let r = plan("uos", "hardening", "server1");
        assert!(r["count"].as_u64().unwrap() >= 1);
        let u = lookup("uos", "ufw", None);
        assert!(u["count"].as_u64().unwrap() >= 1);
    }
}
