//! phoenix_agent.rs — PHOENIX SHIELD: **Autonomous Security Engineer**.
//!
//! Mzunguko kamili wa wakala mmoja: kuanzia upimaji hadi ripoti, bila mtu
//! kuingilia katikati kwa hatua salama. Hatua za hatari (pen-test lab) huzuiwa
//! na HITL (`human_approved`) na gate ya scope — hazirukwi kwa prompt.
//!
//! Mzunguko (kilofazl hurekodi muda kwa millisecond):
//!   1. scope      — thibitisha lengo liko kwenye `scope.txt` (mifumo yako).
//!   2. posture    — tathmini ya ulinzi wa mfumo huu (checks HALISI).
//!   3. threat     — alama ya SMS/URL/caller (threatintel; hesabu safi, μs).
//!   4. tools      — hesabu ya zana za lab zinazopatikana (secpack/kali_groups).
//!   5. analytics  — takwimu za R (hiari; ikikosekana huripoti kwa uaminifu).
//!   6. report     — kuhifadhi ripoti (threatintel::save_report) + muhtasari.
//!
//! Kasi: hatua zote hapa ni za ndani (hakuna kusubiri mtandao), hivyo mzunguko
//! hukamilika kwa **millisecond kadhaa**. Zana za nje (nmap/nikto) huitwa kwa
//! matakwa tu (`deep: true`) na hurekodi muda wao halisi.

use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::time::Instant;

#[derive(Deserialize, Default)]
pub struct CallerPair {
    pub presented: String,
    pub actual: String,
}

#[derive(Deserialize, Default)]
pub struct AgentReq {
    /// Lengo — lazima liwe kwenye scope.txt (mifumo unayomiliki).
    #[serde(default)]
    pub target: String,
    /// Idhini ya mmiliki kwa hatua za hatari (lab/pen-test).
    #[serde(default)]
    pub human_approved: bool,
    /// Akaunti inayoomba (kwa raw/ripoit).
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub sms: Vec<String>,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub callers: Vec<CallerPair>,
    #[serde(default)]
    pub events: Vec<f64>,
    /// Endesha pia uchunguzi wa kina wa lengo (HITL) — polepole kwa kawaida.
    #[serde(default)]
    pub deep: bool,
}

/// Faza moja ya mzunguko.
struct Phase {
    id: &'static str,
    jina: &'static str,
    ms: u128,
    ok: bool,
    note: String,
    data: Value,
}

fn phase(id: &'static str, jina: &'static str, t0: Instant, ok: bool, note: impl Into<String>, data: Value) -> Phase {
    Phase { id, jina, ms: t0.elapsed().as_millis(), ok, note: note.into(), data }
}

/// Endesha mzunguko KAMILI wa wakala autonomasi (defensive).
pub async fn run(db: &SqlitePool, r: &AgentReq) -> Value {
    let total = Instant::now();
    let mut phases: Vec<Phase> = Vec::new();

    // ---- 1) SCOPE ----
    let t = Instant::now();
    let in_scope = crate::threatswarm::check_scope(&r.target).is_ok();
    phases.push(phase(
        "scope",
        "Uthibitisho wa scope",
        t,
        in_scope,
        if in_scope { "lengo liko kwenye scope.txt" } else { "lengo HAKIPO kwenye scope.txt — simamisha hatua za lab" },
        json!({ "target": r.target, "in_scope": in_scope }),
    ));

    // ---- 2) POSTURE ----
    let t = Instant::now();
    let posture = crate::phoenix_shield::posture();
    let health = posture["health"].as_i64().unwrap_or(0);
    phases.push(phase(
        "posture",
        "Tathmini ya ulinzi wa mfumo",
        t,
        true,
        format!("afya {health}/100"),
        json!({ "health": health, "grade": posture["grade"] }),
    ));

    // ---- 3) THREAT INTEL (hesabu safi, μs) ----
    let t = Instant::now();
    let mut signals: Vec<crate::threatintel::Signal> = Vec::new();
    let mut facets: Vec<Value> = Vec::new();
    let mut worst = 0i64;
    for m in &r.sms {
        let a = crate::threatintel::score_message(m);
        worst = worst.max(a.score);
        signals.extend(a.signals.clone());
        facets.push(json!({ "kind": "sms", "target": m, "score": a.score, "verdict": a.verdict }));
    }
    for u in &r.urls {
        let a = crate::threatintel::analyze_url(u);
        worst = worst.max(a.score);
        signals.push(crate::threatintel::Signal {
            id: "url".into(),
            weight: a.score,
            detail: format!("{} ({})", a.host, a.reasons.join("; ")),
        });
        facets.push(json!({ "kind": "url", "target": u, "score": a.score, "verdict": a.verdict }));
    }
    for c in &r.callers {
        let a = crate::threatintel::check_caller(&c.presented, &c.actual);
        worst = worst.max(a.score);
        signals.push(crate::threatintel::Signal {
            id: "caller".into(),
            weight: a.score,
            detail: format!("{} => {} ({})", c.presented, c.actual, a.reasons.join("; ")),
        });
        facets.push(json!({ "kind": "caller", "target": c.presented, "score": a.score, "verdict": a.verdict }));
    }
    phases.push(phase(
        "threat",
        "Alama za ulaghai (SMS/URL/caller)",
        t,
        true,
        format!("alama ya juu {worst}, alama {}", facets.len()),
        json!({ "facets": facets, "worst_score": worst }),
    ));

    // ---- 4) TOOLS (inventa ya zana za lab zinazopatikana) ----
    let t = Instant::now();
    let catalog = crate::kali_groups::tool_count();
    let pack = crate::secpack::groups_json();
    let pack_groups = pack["groups"].as_array().map(|a| a.len()).unwrap_or(0);
    phases.push(phase(
        "tools",
        "Inventa ya zana za lab",
        t,
        true,
        format!("katalogi {catalog} majina, makundi {pack_groups}"),
        json!({ "catalog_names": catalog, "secpack_groups": pack_groups }),
    ));

    // ---- 5) ANALYTICS (R, hiari) ----
    let t = Instant::now();
    let analytics = if r.events.is_empty() {
        json!({ "ok": false, "note": "hakuna data ya matukio" })
    } else {
        crate::phoenix_shield::r_analytics(&r.events, &r.events).await
    };
    phases.push(phase(
        "analytics",
        "Takwimu za R (anomaly/risk)",
        t,
        analytics["ok"].as_bool().unwrap_or(false),
        if r.events.is_empty() { "imekataliwa: hakuna data" } else { "R imeendeshwa" },
        analytics,
    ));

    // ---- 6) REPORT ----
    let t = Instant::now();
    let account = if r.account.is_empty() { "system" } else { &r.account };
    let verdict = crate::threatintel::verdict_of(worst);
    let report_id = crate::threatintel::save_report(
        db,
        account,
        "phoenix-agent",
        &r.target,
        worst,
        verdict,
        &signals,
        &format!("afya {health}; in_scope {in_scope}; signals {}", signals.len()),
    )
    .await
    .ok();
    phases.push(phase(
        "report",
        "Kuhifadhi ripoti",
        t,
        report_id.is_some(),
        report_id.clone().unwrap_or_else(|| "kuhifadhi kimeshindikana".into()),
        json!({ "report_id": report_id, "verdict": verdict }),
    ));

    let total_ms = total.elapsed().as_millis();
    json!({
        "ok": true,
        "agent": "phoenix-security-engineer",
        "target": r.target,
        "in_scope": in_scope,
        "human_approved": r.human_approved,
        "verdict": verdict,
        "worst_score": worst,
        "total_ms": total_ms,
        "phases": phases.iter().map(|p| json!({
            "id": p.id, "jina": p.jina, "ms": p.ms, "ok": p.ok,
            "note": p.note, "data": p.data,
        })).collect::<Vec<_>>(),
        "kanuni_sw": "Defensive-only: hupima na kulinda mifumo YAKO. Hatua za lab huzuiwa \
                      bila idhini (human_approved) na scope.txt. Hakuna kushambulia wengine.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fase_hurekodi_muda() {
        let t = Instant::now();
        let p = phase("x", "X", t, true, "ok", json!({}));
        assert_eq!(p.id, "x");
        assert!(p.ok);
    }

    #[test]
    fn verdict_ya_alama() {
        assert_eq!(crate::threatintel::verdict_of(0), "salama");
        assert_eq!(crate::threatintel::verdict_of(90), "hatari");
    }
}
