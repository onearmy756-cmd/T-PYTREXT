//! secops.rs — CYBER SECURITY & DIGITAL FORENSICS CENTER (H11)
//!
//! Uchunguzi wa usalama (afya ya kila kompyuta, mashambulizi yaliyozuiwa,
//! udhaifu, virusi) na uchanganuzi wa kidijitali (ushahidi: diski, kumbukumbu,
//! mtandao, logs — unafungwa na hash) kwa kompyuta NYINGI kwa WAKATI MMOJA.
//! Kazi ni HITL (idhini kwanza).
//!
//! KANUNI YA SIRI: API inarudisha majina SALAMA ya Kiswahili tu — kamwe
//! jina la zana, bandari, au usanifu wa ndani wa mfumo.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

// ---------- HALI YA KAZI ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Security,
    Forensics,
    Both,
}

impl Mode {
    /// "security" | "usalama" | "forensics" | "uchunguzi" | "both" | "zote"
    pub fn parse(s: &str) -> Option<Mode> {
        match s.trim().to_lowercase().as_str() {
            "security" | "usalama" => Some(Mode::Security),
            "forensics" | "forensic" | "uchunguzi" => Some(Mode::Forensics),
            "both" | "zote" => Some(Mode::Both),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Security => "security",
            Mode::Forensics => "forensics",
            Mode::Both => "both",
        }
    }

    /// Op zitaendeshwa kwa kila kifaa (kila op = kazi moja HITL)
    pub fn ops(&self) -> &'static [&'static str] {
        match self {
            Mode::Security => &["security"],
            Mode::Forensics => &["forensics"],
            Mode::Both => &["security", "forensics"],
        }
    }
}

// ---------- FINDINGS + AFYA YA USALAMA ----------
pub const KIND_ATTACK: &str = "attack_blocked";
pub const KIND_VULN: &str = "vulnerability";
pub const KIND_MALWARE: &str = "malware";
pub const KIND_PROBLEM: &str = "problem";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub kind: String,
    pub detail: String,
    pub severity: i64,
}

impl Finding {
    /// Kind iwe ndogo na safi (mf. " Attack_Blocked " -> "attack_blocked")
    pub fn normalized(mut self) -> Self {
        self.kind = self.kind.trim().to_lowercase();
        if self.kind.is_empty() {
            self.kind = KIND_PROBLEM.into();
        }
        self.severity = self.severity.clamp(0, 100);
        self
    }
}

/// Afya ya usalama: 100 hadi 0 (kila finding inapunguza kwa severity yake)
pub fn health_from_findings(findings: &[Finding]) -> i64 {
    let mut h: i64 = 100;
    for f in findings {
        h -= f.severity.clamp(0, 100);
    }
    h.clamp(0, 100)
}

/// Daraja la hatari kutoka kwa afya
pub fn severity_of(health: i64) -> &'static str {
    if health >= 80 {
        "salama"
    } else if health >= 50 {
        "tahadhari"
    } else {
        "hatari"
    }
}

// ---------- USHAHIDI (FORENSICS — chain-of-custody) ----------
/// SHA-256 ya account|target|sources|at — ushahidi unafungwa na hash hii
pub fn evidence_hash(account: &str, target: &str, sources: &str, at: &str) -> String {
    let mut h = Sha256::new();
    h.update(account.as_bytes());
    h.update(b"|");
    h.update(target.as_bytes());
    h.update(b"|");
    h.update(sources.as_bytes());
    h.update(b"|");
    h.update(at.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ---------- DB ----------
pub async fn init_tables(db: &SqlitePool) {
    for ddl in [
        r#"CREATE TABLE IF NOT EXISTS secops_reports (
            id TEXT PRIMARY KEY,
            at TEXT,
            account TEXT,
            target TEXT,
            mode TEXT,
            health INTEGER,
            findings TEXT,
            severity TEXT,
            status TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS forensics_cases (
            id TEXT PRIMARY KEY,
            at TEXT,
            account TEXT,
            target TEXT,
            evidence_hash TEXT,
            sources TEXT,
            sealed INTEGER,
            note TEXT
        )"#,
    ] {
        let _ = sqlx::query(ddl).execute(db).await;
    }
}

/// Hifadhi ripoti ya usalama — inarudisha (id, health, severity)
pub async fn save_report(
    db: &SqlitePool,
    account: &str,
    target: &str,
    mode: &str,
    findings: &[Finding],
) -> Result<(String, i64, String), String> {
    let health = health_from_findings(findings);
    let sev = severity_of(health);
    let id = Uuid::new_v4().to_string();
    let at = chrono::Local::now().to_rfc3339();
    let fj = serde_json::to_string(findings).unwrap_or_else(|_| "[]".into());
    sqlx::query(
        "INSERT INTO secops_reports (id, at, account, target, mode, health, findings, severity, status) VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&at)
    .bind(account)
    .bind(target)
    .bind(mode)
    .bind(health)
    .bind(&fj)
    .bind(sev)
    .bind("imekamilika")
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok((id, health, sev.to_string()))
}

/// Hifadhi kesi ya forensics — ushahidi unafungwa (sealed) na hash
pub async fn save_case(
    db: &SqlitePool,
    account: &str,
    target: &str,
    sources: &str,
    note: &str,
) -> Result<(String, String), String> {
    let id = Uuid::new_v4().to_string();
    let at = chrono::Local::now().to_rfc3339();
    let hash = evidence_hash(account, target, sources, &at);
    sqlx::query(
        "INSERT INTO forensics_cases (id, at, account, target, evidence_hash, sources, sealed, note) VALUES (?,?,?,?,?,?,1,?)",
    )
    .bind(&id)
    .bind(&at)
    .bind(account)
    .bind(target)
    .bind(&hash)
    .bind(sources)
    .bind(note)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok((id, hash))
}

/// Muhtasari wa usalama kwa account — data HALISI tu (hakuna demo).
/// Wateja wanaona ripoti ZAO PEKEE (access control — SEHEMU 8.3).
pub async fn summary(db: &SqlitePool, account: &str) -> serde_json::Value {
    let rows: Vec<(String, String, String, i64, String, String, String)> = sqlx::query_as(
        "SELECT at, target, mode, health, findings, severity, status FROM secops_reports WHERE account=?1 ORDER BY at DESC",
    )
    .bind(account)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // hali ya SASA ya kila target = ripoti yake MPYA zaidi (mstari wa kwanza)
    type TargetState = (i64, String, usize, std::collections::HashMap<String, i64>);
    let mut per_target: std::collections::HashMap<String, TargetState> = Default::default();
    let mut kinds: std::collections::HashMap<String, i64> = Default::default();
    let mut reports_out: Vec<serde_json::Value> = Vec::new();

    for (i, (at, target, mode, health, fj, sev, _status)) in rows.iter().enumerate() {
        let findings: Vec<Finding> = serde_json::from_str(fj).unwrap_or_default();
        for f in &findings {
            *kinds.entry(f.kind.clone()).or_insert(0) += 1;
        }
        let ent = per_target
            .entry(target.clone())
            .or_insert_with(|| (0, String::new(), 0, Default::default()));
        if ent.1.is_empty() {
            let mut counts: std::collections::HashMap<String, i64> = Default::default();
            for f in &findings {
                *counts.entry(f.kind.clone()).or_insert(0) += 1;
            }
            *ent = (*health, sev.clone(), findings.len(), counts);
        }
        if i < 20 {
            reports_out.push(json!({
                "at": at, "target": target, "mode": mode,
                "health": health, "severity": sev, "findings_count": findings.len(),
            }));
        }
    }

    let mut salama = 0i64;
    let mut tahadhari = 0i64;
    let mut hatari = 0i64;
    let mut computers: Vec<serde_json::Value> = Vec::new();
    for (target, (health, sev, n, counts)) in &per_target {
        match sev.as_str() {
            "salama" => salama += 1,
            "tahadhari" => tahadhari += 1,
            _ => hatari += 1,
        }
        computers.push(json!({
            "target": target, "health": health, "severity": sev, "findings_count": n,
            "attacks_blocked": counts.get(KIND_ATTACK).copied().unwrap_or(0),
            "vulnerabilities": counts.get(KIND_VULN).copied().unwrap_or(0),
            "malware": counts.get(KIND_MALWARE).copied().unwrap_or(0),
        }));
    }
    computers.sort_by(|a, b| {
        a["target"]
            .as_str()
            .unwrap_or("")
            .cmp(b["target"].as_str().unwrap_or(""))
    });

    let cases: Vec<(String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT at, target, evidence_hash, sources, sealed, note FROM forensics_cases WHERE account=?1 ORDER BY at DESC LIMIT 10",
    )
    .bind(account)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let forensics: Vec<serde_json::Value> = cases
        .iter()
        .map(|(at, target, hash, sources, sealed, note)| {
            json!({
                "at": at, "target": target, "evidence_hash": hash,
                "sources": sources, "sealed": *sealed == 1, "note": note,
            })
        })
        .collect();

    json!({
        "ok": true,
        "account": account,
        "computers": computers,
        "totals": { "salama": salama, "tahadhari": tahadhari, "hatari": hatari },
        "kinds": kinds,
        "reports": reports_out,
        "forensics": forensics,
        "note_sw": "Kila kompyuta anaona hali yake ya usalama; ushahidi wa forensics umefungwa na hash.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_parse_inakubali_sahihi_na_inakataa_mibaya() {
        assert_eq!(Mode::parse("security"), Some(Mode::Security));
        assert_eq!(Mode::parse(" Usalama "), Some(Mode::Security));
        assert_eq!(Mode::parse("forensics"), Some(Mode::Forensics));
        assert_eq!(Mode::parse("uchunguzi"), Some(Mode::Forensics));
        assert_eq!(Mode::parse("zote"), Some(Mode::Both));
        assert_eq!(Mode::parse("both"), Some(Mode::Both));
        assert_eq!(Mode::parse("hehe"), None);
        assert_eq!(Mode::parse(""), None);
    }

    #[test]
    fn ops_zinaendana_na_mode() {
        assert_eq!(Mode::Security.ops(), &["security"]);
        assert_eq!(Mode::Forensics.ops(), &["forensics"]);
        assert_eq!(Mode::Both.ops().len(), 2);
    }

    #[test]
    fn afya_inapungua_kwa_severity_na_inafungwa_0_100() {
        assert_eq!(health_from_findings(&[]), 100);
        let f1 = Finding {
            kind: KIND_VULN.into(),
            detail: "x".into(),
            severity: 30,
        };
        assert_eq!(health_from_findings(&[f1.clone()]), 70);
        let f2 = Finding {
            kind: KIND_ATTACK.into(),
            detail: "y".into(),
            severity: 70,
        };
        assert_eq!(health_from_findings(&[f1, f2]), 0);
        let bad = Finding {
            kind: KIND_PROBLEM.into(),
            detail: "z".into(),
            severity: -5,
        };
        assert_eq!(
            health_from_findings(&[bad]),
            100,
            "severity hasi hupuuzwa (clamp)"
        );
    }

    #[test]
    fn daraja_la_hatari_lina_vizingiti_sahihi() {
        assert_eq!(severity_of(100), "salama");
        assert_eq!(severity_of(80), "salama");
        assert_eq!(severity_of(79), "tahadhari");
        assert_eq!(severity_of(50), "tahadhari");
        assert_eq!(severity_of(49), "hatari");
        assert_eq!(severity_of(0), "hatari");
    }

    #[test]
    fn kind_inanormalishwa() {
        let n = Finding {
            kind: " Attack_Blocked ".into(),
            detail: "d".into(),
            severity: 250,
        }
        .normalized();
        assert_eq!(n.kind, "attack_blocked");
        assert_eq!(n.severity, 100);
        let e = Finding {
            kind: "  ".into(),
            detail: "d".into(),
            severity: 0,
        }
        .normalized();
        assert_eq!(e.kind, KIND_PROBLEM);
    }

    #[test]
    fn hash_ya_ushahidi_ni_thabiti_na_tofauti_kwa_data_tofauti() {
        let a = evidence_hash("mteja1", "pc-01", "diski,logs", "2026-10-06T10:00:00+03:00");
        let b = evidence_hash("mteja1", "pc-01", "diski,logs", "2026-10-06T10:00:00+03:00");
        let c = evidence_hash("mteja2", "pc-01", "diski,logs", "2026-10-06T10:00:00+03:00");
        assert_eq!(a, b, "input sawa = hash sawa");
        assert_ne!(a, c, "account tofauti = hash tofauti");
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn ripoti_na_kesi_zinahifadhiwa_na_kusomeka_kurudi() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        let f = Finding {
            kind: KIND_VULN.into(),
            detail: "patch inahitajika".into(),
            severity: 40,
        };
        let (rid, health, sev) = save_report(&db, "mteja1", "pc-01", "security", &[f])
            .await
            .unwrap();
        assert!(!rid.is_empty());
        assert_eq!(health, 60);
        assert_eq!(sev, "tahadhari");
        let (cid, hash) = save_case(
            &db,
            "mteja1",
            "pc-02",
            "diski,kumbukumbu",
            "kesi ya jaribio",
        )
        .await
        .unwrap();
        assert!(!cid.is_empty() && hash.len() == 64);
        let s = summary(&db, "mteja1").await;
        assert_eq!(s["ok"], serde_json::Value::Bool(true));
        // computers zinatoka kwenye ripoti za usalama pekee (pc-01); pc-02 ni kesi ya forensics
        assert_eq!(s["computers"].as_array().unwrap().len(), 1);
        assert_eq!(s["totals"]["tahadhari"], serde_json::json!(1));
        assert_eq!(s["forensics"].as_array().unwrap().len(), 1);
        assert_eq!(s["forensics"][0]["sealed"], serde_json::Value::Bool(true));
        let s2 = summary(&db, "mwingine").await;
        assert_eq!(
            s2["computers"].as_array().unwrap().len(),
            0,
            "account nyingine haioni ripoti za mwingine"
        );
    }
}
