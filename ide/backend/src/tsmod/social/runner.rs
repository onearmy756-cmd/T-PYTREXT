//! runner.rs — mantiki halisi ya uchanganuzi kwa module ya jukwaa.
//!
//! Hutumia sheria za HIFADHIDATA (`store`) + injini (`engine`), hurekodi kila
//! uamuzi kwenye `ts_events` (audit) na kurudisha `ModReport`. Uzito wa sheria
//! hubadilika kutoka feedback (learning loop) — hakuna word-list iliyogandishwa
//! ndani ya code.

use super::engine;
use super::platform::PlatformEngine;
use super::{Cat, SocialMod};
use crate::tsmod::store;
use crate::tsmod::ModReport;
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Changanua maudhui kwa injini ya jukwaa (code halisi ya jukwaa) na uunganishe
/// matokeo kwenye ripoti. Hurudisha alama ya juu iliyopatikana.
fn apply_platform_engine(m: &SocialMod, target: &str, params: &Value, r: &mut ModReport) -> u32 {
    let Some(eng) = super::engine_for(m.platform.key) else {
        return 0;
    };
    let mut pr = super::platform::PReport::default();
    if let Some(text) = params.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            pr.findings.extend(eng.text_checks(text).findings);
        }
    }
    if let Some(url) = params.get("url").and_then(|v| v.as_str()) {
        if !url.is_empty() {
            pr.findings.extend(eng.url_checks(url).findings);
        }
    }
    if target.starts_with("http://") || target.starts_with("https://") {
        pr.findings.extend(eng.url_checks(target).findings);
    } else if !target.is_empty() && params.get("text").is_none() {
        pr.findings.extend(eng.text_checks(target).findings);
    }
    pr.findings.extend(eng.account_checks(params).findings);
    let mut score = 0u32;
    for f in &pr.findings {
        score = score.max(f.severity as u32);
        r.finding(f.kind, f.detail.clone(), f.severity);
    }
    for n in &pr.notes {
        r.findings.push(json!({ "kind": "platform_note", "severity": 2, "detail": n }));
    }
    score
}

/// Endesha module ya jukwaa kwa DB.
pub async fn run(
    m: &SocialMod,
    db: &SqlitePool,
    target: &str,
    params: &Value,
) -> Result<ModReport, String> {
    let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
    let mut r = ModReport::new(m.id, m.name);
    r.findings.push(json!({
        "kind": "social_meta", "severity": 2,
        "detail": {
            "platform": m.platform.name, "category": m.cat().key(),
            "kazi": m.kazi, "twist": m.twist
        }
    }));
    if op == "audit" || op == "status" {
        r.summary = format!("{} — {}: injini ya DB (sheria za kujifunza) ipo tayari; tuma `text`/`url` na op=analyze/ai kuchanganua.", m.platform.name, m.name);
        return Ok(r);
    }
    // OPS ZA AI/AGENT: injini ya DB (rules + platform) HALISI, kisha LLM ikiwa ipo.
    if op == "ai" || op == "autonomous" {
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let url = params.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let learn = op == "autonomous" || params.get("learn").and_then(|v| v.as_bool()).unwrap_or(false);

        // Tabaka 1 (lazima): sheria za DB + injini ya jukwaa.
        let mut db_score = 0u32;
        if !text.is_empty() {
            for h in engine::match_text(db, m.platform.name, text).await {
                db_score = db_score.max(h.severity as u32);
                r.finding(&h.kind, format!("Sheria '{}' imegusa — chujio limewashwa", h.pattern), h.severity);
            }
        }
        if !url.is_empty() {
            let v = engine::match_url(db, m.platform.name, url).await;
            db_score = db_score.max(v.score);
            for h in &v.hits {
                r.finding(&h.kind, format!("{url}: sheria '{}'", h.pattern), h.severity);
            }
        }
        db_score = db_score.max(apply_platform_engine(m, target, params, &mut r));

        // Tabaka 2 (hiari): LLM hupima maana ikiwa imeunganishwa.
        let ai = super::ai::autonomous_scan(db, m.platform.name, m.cat().key(), if !text.is_empty() { text } else { url }, learn).await;
        let ai_present = ai.get("ai").map(|v| !v.is_null()).unwrap_or(false);
        let ai_score = ai.get("ai").and_then(|v| v.get("score")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let mut score = db_score.max(if ai_present { ai_score } else { 0 });
        if let Some(learned) = ai.get("learned_rules").and_then(|v| v.as_array()) {
            if !learned.is_empty() {
                r.action("learned_rules", format!("Sheria {} zimejifunza kutoka LLM", learned.len()), true);
            }
        }
        if ai_present {
            r.findings.push(json!({ "kind": "ai", "severity": ai_score.min(100) as u8, "detail": ai["ai"].clone() }));
        } else {
            r.findings.push(json!({ "kind": "engine", "severity": 2,
                "detail": "LLM haipatikani — uamuzi umetolewa na injini ya DB (sheria + jukwaa) PEKEE." }));
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.summary = format!("{} — {}: uamuzi {decision} (score {score}; DB {db_score}{}).",
            m.platform.name, m.name, if ai_present { format!(" + AI {ai_score}") } else { ", LLM haipatikani".into() });
        r.findings.push(json!({ "kind": "decision", "severity": score.min(100) as u8,
            "detail": { "decision": decision, "score": score, "db_score": db_score, "ai": ai_present } }));
        let signals = json!([]);
        let _ = store::record_event(db, params.get("account").and_then(|v| v.as_str()).unwrap_or(""),
            m.id, m.platform.name, target, decision, score, &signals, if ai_present { "ai" } else { "db" }).await;
        return Ok(r);
    }
    if op == "synthesize" {
        let ev = params.get("event").cloned().unwrap_or(json!({}));
        match super::ai::synthesize(db, m.platform.name, &ev).await {
            Some(text) => {
                r.summary = text.clone();
                r.findings.push(json!({ "kind": "ai_report", "severity": 2, "detail": text }));
            }
            None => r.summary = "LLM haipatikani — ripoti haiwezi kuandikwa kwa AI.".into(),
        }
        return Ok(r);
    }
    if op != "analyze" {
        return Err(format!("op '{op}' haipo: audit|analyze|ai|autonomous|synthesize"));
    }

    let mut score = 0u32;
    let mut rule_ids: Vec<i64> = Vec::new();
    let subject = target.to_string();

    match m.cat() {
        Cat::Content => {
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let hits = engine::match_text(db, m.platform.name, text).await;
            for h in &hits {
                rule_ids.push(h.rule_id);
                score = score.max(h.severity as u32);
                r.finding(&h.kind, format!("Sheria '{}' imegusa — chujio limewashwa", h.pattern), h.severity);
            }
            if !hits.is_empty() {
                r.action("content_filtered", format!("Vitu {} vimechujwa/imefichwa", hits.len()), true);
            }
        }
        Cat::Link => {
            if let Some(url) = params.get("url").and_then(|v| v.as_str()) {
                let v = engine::match_url(db, m.platform.name, url).await;
                score = score.max(v.score);
                for h in &v.hits {
                    rule_ids.push(h.rule_id);
                    r.finding(&h.kind, format!("{url}: sheria '{}'", h.pattern), h.severity);
                }
                for n in &v.notes {
                    r.finding("url_structure", format!("{url}: {n}"), v.score.min(100) as u8);
                }
                if v.score >= 40 {
                    r.action("sandbox_open", format!("{url} imefunguliwa kwenye sandbox"), true);
                }
            }
            if let Some(qr) = params.get("qr").and_then(|v| v.as_str()) {
                let v = engine::match_qr(db, m.platform.name, qr).await;
                score = score.max(v.score);
                for h in &v.hits {
                    rule_ids.push(h.rule_id);
                    r.finding(&h.kind, format!("QR: sheria '{}'", h.pattern), h.severity);
                }
                for n in &v.notes {
                    r.finding("qr_risk", n.clone(), v.score.min(100) as u8);
                }
            }
        }
        Cat::Account => {
            if let Some(a) = params.get("logins").and_then(|v| v.as_array()) {
                let logins: Vec<engine::Login> = a
                    .iter()
                    .map(|l| engine::Login {
                        lat: l.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        lon: l.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        ts: l.get("ts").and_then(|v| v.as_i64()).unwrap_or(0),
                        device: l.get("device").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect();
                for t in engine::impossible_travel(&logins) {
                    score = score.max(85);
                    r.finding("impossible_travel", t, 85);
                }
            }
            if params.get("new_device").and_then(|v| v.as_bool()).unwrap_or(false) {
                score = score.max(50);
                r.finding("new_device", "Kuingia kutoka kifaa kipya — uthibitisho unahitajika", 50);
            }
        }
        Cat::Privacy => {
            if params.get("viewer_blacklisted").and_then(|v| v.as_bool()).unwrap_or(false) {
                score = score.max(40);
                r.finding("blacklist_block", "Mtazamaji yuko kwenye blacklist — amezuiwa kuona", 40);
            }
            let audience = params.get("audience").and_then(|v| v.as_str()).unwrap_or("default");
            r.findings.push(json!({ "kind": "audience_policy", "severity": 2,
                "detail": format!("Audience: {audience}") }));
        }
        Cat::Fraud => {
            if params.get("reported").and_then(|v| v.as_bool()).unwrap_or(false) {
                score = score.max(80);
                r.finding("reported_seller", "Muuzaji/mtumaji ameripotiwa — onyo limeonyeshwa", 80);
            }
            if params.get("asks_money").and_then(|v| v.as_bool()).unwrap_or(false) {
                score = score.max(80);
                r.finding("asks_money", "Anaomba pesa/ada — ulaghai unaowezekana", 80);
            }
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let hits = engine::match_text(db, m.platform.name, text).await;
            for h in &hits {
                if h.kind.contains("scam") {
                    rule_ids.push(h.rule_id);
                    score = score.max(h.severity as u32);
                    r.finding(&h.kind, format!("Sheria '{}' — ulaghai", h.pattern), h.severity);
                }
            }
        }
        Cat::Encryption => {
            let double = params.get("double").and_then(|v| v.as_bool()).unwrap_or(true);
            let has_key = params.get("has_key").and_then(|v| v.as_bool()).unwrap_or(false);
            if double && !has_key {
                score = score.max(60);
                r.finding("no_key", "Usimbaji wa pili unahitaji ufunguo wa SIM", 60);
            } else {
                r.finding("encryption_ok", "Usimbaji fiche wa tabaka mbili umewekwa", 2);
            }
        }
        Cat::Bot => {
            let followers = params.get("new_followers").and_then(|v| v.as_u64()).unwrap_or(0);
            let age = params.get("account_age_days").and_then(|v| v.as_i64()).unwrap_or(999);
            if followers > 500 && age < 7 {
                score = score.max(75);
                r.finding("fake_followers", format!("Wafuasi {followers} kwa siku {age} — bandia"), 75);
            }
            let bot_score = params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if bot_score > 0.7 {
                score = score.max(80);
                r.finding("bot", format!("Alama ya bot {bot_score} — inazuiwa"), 80);
            }
        }
        Cat::Moderation => {
            r.finding("moderation", "Hakuna data ya uchanganuzi iliyotolewa", 2);
        }
    }

    // INJINI YA JUKWAA (code halisi ya jukwaa): kimuundo, kinachohusika na jukwaa
    // hili pekee (lookalike domains, mbinu za jukwaa, tabia ya akaunti).
    score = score.max(apply_platform_engine(m, target, params, &mut r));

    let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
    r.summary = format!("{} — {}: uamuzi {decision} (score {score}).", m.platform.name, m.name);
    r.findings.push(json!({ "kind": "decision", "severity": score.min(100) as u8,
        "detail": { "decision": decision, "score": score } }));
    if !rule_ids.is_empty() {
        store::bump_hits(db, &rule_ids).await;
    }
    // HYBRID: ikiwa `use_ai` = true, ongeza uchanganuzi wa maana (LLM) juu ya
    // sheria za DB — hushika mbinu mpya ambazo keyword hazijui.
    if params.get("use_ai").and_then(|v| v.as_bool()).unwrap_or(false) {
        let text = params.get("text")
            .or_else(|| params.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or(target);
        let learn = params.get("learn").and_then(|v| v.as_bool()).unwrap_or(false);
        let ai = super::ai::autonomous_scan(db, m.platform.name, m.cat().key(), text, learn).await;
        if let Some(a) = ai.get("ai") {
            let ai_score = a.get("score").and_then(|v| v.as_u64()).unwrap_or(0);
            score = score.max(ai_score as u32);
            r.findings.push(json!({ "kind": "ai_hybrid", "severity": ai_score.min(100) as u8, "detail": a.clone() }));
            if let Some(l) = ai.get("learned_rules").and_then(|v| v.as_array()) {
                if !l.is_empty() {
                    r.action("learned_rules", format!("Sheria {} zimejifunza (LLM)", l.len()), true);
                }
            }
            r.summary = format!("{} — {}: uamuzi {} (DB+AI score {score}).",
                m.platform.name, m.name,
                if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" });
        }
    }
    // Audit: rekodi event (hurekebisha tp/fp baadaye kwa feedback).
    let signals = Value::Array(rule_ids.iter().map(|id| json!({ "rule_id": id })).collect());
    let _ = store::record_event(
        db,
        params.get("account").and_then(|v| v.as_str()).unwrap_or(""),
        m.id,
        m.platform.name,
        &subject,
        if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" },
        score,
        &signals,
        "",
    )
    .await;
    Ok(r)
}
