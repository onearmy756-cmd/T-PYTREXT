//! admin_console.rs — KITUO CHA ADMIN: CHAT YA CYBER/FORENSICS + RIPOTI YENYE MUHURI.
//!
//! Mahitaji ya mmiliki:
//!   - "sehemu ya chat ambapo admin ataandika prompt; mfumo utafanya kazi
//!     automatically na kukagua au kufanya kazi uliyoambiwa, na kuleta report
//!     safi ya muhuri na nembo ya kampuni na ya kuvutia"
//!   - "weka sehemu ambapo admin anaona request zote" na "mifumo mizima"
//!
//! MTIRIRIKO:
//!   1. Admin anaandika prompt (Kiswahili/Kiingereza) — mf. "chunguza domain
//!      example.co.tz kwa hatari".
//!   2. `classify()` huamua aina ya kazi (domain/url/sms/phone/host/ip/prompt).
//!   3. `execute()` huendesha uchanganuzi HALISI (threatintel) na kukusanya
//!      matokeo ya kweli — si maoni.
//!   4. `report()` hutengeneza RIPOTI ya HTML yenye nembo + MUHURI wa ECDSA +
//!      SHA-256 (pkicrypto::seal_document). PDF inapatikana kwa `report_pdf`.
//!
//! Hii ndiyo "mfumo unafanya kazi automatically na kukagua" — AI haihitajiki
//! kwa uchanganuzi wa msingi (deterministic engines); LLM (chat.rs) inaweza
//! kuongeza uchambuzi, lakini ripoti huthibitishwa kwa data halisi.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::pkicrypto;
use crate::threatintel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub kind: String,
    pub target: String,
    pub verdict: String,
    pub score: i64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub job: String,
    pub target: String,
    pub findings: Vec<Finding>,
    pub severity: String,
    pub note: String,
}

/// Amua aina ya kazi kutoka kwa prompt huru ya admin.
pub fn classify(prompt: &str) -> (&'static str, String) {
    let p = prompt.to_lowercase();
    // Tafuta token ya kwanza inayofanana na domain/URL.
    for tok in prompt.split_whitespace() {
        let t = tok.trim_matches(|c: char| c == ',' || c == '.' || c == ':' || c == '"' || c == '\'');
        if t.contains("http://") || t.contains("https://") {
            return ("url", t.to_string());
        }
        if t.contains('.') && t.len() > 3 && !t.ends_with('.') {
            let host = t.split('/').next().unwrap_or(t);
            let looks_like_domain = host
                .split('.')
                .last()
                .map(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
                .unwrap_or(false);
            if looks_like_domain {
                return ("domain", host.to_string());
            }
        }
    }
    // Namba ya simu?
    for tok in prompt.split_whitespace() {
        let digits: String = tok.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 9 {
            return ("phone", tok.to_string());
        }
    }
    // IP?
    for tok in prompt.split_whitespace() {
        let t = tok.trim_matches(|c: char| c == ',' || c == ':' || c == '"' || c == '\'');
        let parts: Vec<&str> = t.split('.').collect();
        if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
            return ("ip", t.to_string());
        }
    }
    if p.contains("sms") || p.contains("ujumbe") {
        return ("sms", prompt.to_string());
    }
    if p.contains("host") || p.contains("server") || p.contains("kifaa") {
        return ("host", prompt.to_string());
    }
    ("prompt", prompt.to_string())
}

/// Endesha uchanganuzi HALISI kulingana na aina ya kazi.
pub fn execute(job: &str, target: &str) -> AnalysisResult {
    let mut findings = Vec::new();
    let (verdict, score, detail) = match job {
        "url" | "domain" => {
            let a = threatintel::analyze_url(target);
            let d = format!(
                "host={} brand={} sababu={}",
                a.host,
                a.brand.clone().unwrap_or_else(|| "-".into()),
                if a.reasons.is_empty() { "-".to_string() } else { a.reasons.join("; ") }
            );
            (a.verdict, a.score, d)
        }
        "sms" => {
            let a = threatintel::score_message(target);
            let d = a
                .signals
                .iter()
                .map(|s| format!("{}({})", s.id, s.weight))
                .collect::<Vec<_>>()
                .join("; ");
            (a.verdict, a.score, if d.is_empty() { "hakuna ishara".into() } else { d })
        }
        "phone" => {
            let a = threatintel::check_caller(target, target);
            (a.verdict, a.score, a.reasons.join("; "))
        }
        "ip" | "host" => (
            "info".to_string(),
            10,
            format!("Lengo: {target} — kwa uchanganuzi wa kina tumia zana za secops/deepscan (admin)."),
        ),
        _ => (
            "prompt".to_string(),
            0,
            "Hakuna lengo la kiufundi lililogundulika — tumia chat ya agenti kwa kazi ya jumla.".into(),
        ),
    };
    findings.push(Finding {
        kind: job.to_string(),
        target: target.to_string(),
        verdict: verdict.clone(),
        score,
        detail,
    });
    let severity = severity_of(score, &verdict);
    AnalysisResult {
        job: job.to_string(),
        target: target.to_string(),
        findings,
        severity,
        note: "Uchanganuzi huu ni wa kweli (deterministic engines) — hakuna makisio.".into(),
    }
}

fn severity_of(score: i64, verdict: &str) -> String {
    let v = verdict.to_lowercase();
    if v.contains("malicious") || v.contains("hatari") || score >= 70 {
        "critical".into()
    } else if score >= 45 {
        "high".into()
    } else if score >= 20 {
        "medium".into()
    } else {
        "low".into()
    }
}

// =====================================================================
// RIPOTI YA ADMIN — HTML yenye muhuri (na PDF)
// =====================================================================

/// HTML ya ripoti (bila muhuri) — inaingizwa kwenye muhuri.
pub fn report_body_html(res: &AnalysisResult) -> String {
    let rows = res
        .findings
        .iter()
        .map(|f| {
            format!(
                "<tr><td style='padding:6px 10px;border-bottom:1px solid #e2e8f0'><b>{}</b></td>\
<td style='padding:6px 10px;border-bottom:1px solid #e2e8f0'>{}</td>\
<td style='padding:6px 10px;border-bottom:1px solid #e2e8f0'>{}</td>\
<td style='padding:6px 10px;border-bottom:1px solid #e2e8f0'>{}</td>\
<td style='padding:6px 10px;border-bottom:1px solid #e2e8f0;font-size:12px;color:#475569'>{}</td></tr>",
                esc(&f.kind), esc(&f.target), esc(&f.verdict), f.score, esc(&f.detail)
            )
        })
        .collect::<String>();
    format!(
        r#"<div style="font-family:system-ui,Segoe UI,sans-serif">
<p style="color:#475569;font-size:13px">Aina ya kazi: <b>{job}</b> · Lengo: <b>{target}</b> · Ukali: <b style="color:{sev_color}">{sev}</b></p>
<table style="border-collapse:collapse;width:100%;font-size:13px">
<tr style="background:#f1f5f9;text-align:left"><th style="padding:8px 10px">Aina</th><th style="padding:8px 10px">Lengo</th><th style="padding:8px 10px">Hukumu</th><th style="padding:8px 10px">Alama</th><th style="padding:8px 10px">Maelezo</th></tr>
{rows}
</table>
<p style="margin-top:14px;color:#64748b;font-size:12px">{note}</p>
</div>"#,
        job = esc(&res.job),
        target = esc(&res.target),
        sev = esc(&res.severity),
        sev_color = sev_color(&res.severity),
        rows = rows,
        note = esc(&res.note),
    )
}

fn sev_color(sev: &str) -> &'static str {
    match sev {
        "critical" => "#dc2626",
        "high" => "#ea580c",
        "medium" => "#ca8a04",
        _ => "#059669",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Tengeneza ripoti ya admin iliyotiwa muhuri (HTML + ECDSA + SHA-256).
pub async fn report(db: &SqlitePool, res: &AnalysisResult) -> Result<pkicrypto::SealedDoc, String> {
    let title = format!("Ripoti ya Usalama — {}", res.target);
    let body = report_body_html(res);
    pkicrypto::seal_document(db, &title, &body).await
}

/// PDF rahisi (single-page) yenye muhuri wa SHA-256 — bila deps.
/// Hutumia PdfBuilder ya report.rs kupitia `crate::report`.
pub fn report_pdf(res: &AnalysisResult, seal: &pkicrypto::Seal) -> Vec<u8> {
    crate::report::security_report_pdf(res, seal)
}

// =====================================================================
// KUMBUKUMBU YA KAZI ZA ADMIN (request log)
// =====================================================================

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS admin_console_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prompt TEXT NOT NULL,
            job TEXT NOT NULL,
            target TEXT NOT NULL,
            severity TEXT NOT NULL,
            report_hash TEXT NOT NULL,
            at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

pub async fn log_job(
    db: &SqlitePool,
    prompt: &str,
    res: &AnalysisResult,
    report_hash: &str,
) -> Result<i64, String> {
    let r = sqlx::query(
        "INSERT INTO admin_console_jobs (prompt, job, target, severity, report_hash, at) \
         VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(prompt)
    .bind(&res.job)
    .bind(&res.target)
    .bind(&res.severity)
    .bind(report_hash)
    .bind(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(r.last_insert_rowid())
}

pub async fn jobs(db: &SqlitePool, limit: i64) -> serde_json::Value {
    let rows: Vec<(i64, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, prompt, job, target, severity, report_hash, at FROM admin_console_jobs \
         ORDER BY id DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    serde_json::json!({
        "ok": true,
        "jobs": rows.into_iter().map(|(id, prompt, job, target, severity, report_hash, at)| serde_json::json!({
            "id": id, "prompt": prompt, "job": job, "target": target,
            "severity": severity, "report_hash": report_hash, "at": at,
        })).collect::<Vec<_>>(),
    })
}

// =====================================================================
// Vipimo
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let p = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_tables(&p).await;
        pkicrypto::init_tables(&p).await;
        pkicrypto::ensure_company_key(&p).await.unwrap();
        p
    }

    #[test]
    fn classify_hutambua_aina_za_kazi() {
        assert_eq!(classify("chunguza https://evil.tk/login kwa hatari").0, "url");
        assert_eq!(classify("scan example.co.tz").0, "domain");
        assert_eq!(classify("namba +255712345678 inatisha").0, "phone");
        assert_eq!(classify("sms: umeshinda zawadi bonyeza hapa").0, "sms");
        assert_eq!(classify("habari za leo").0, "prompt");
    }

    #[test]
    fn execute_hurudisha_matokeo_halisi() {
        let r = execute("url", "http://bit.ly/xyz");
        assert_eq!(r.job, "url");
        assert!(!r.findings.is_empty());
        // Shortener → alama kubwa / ukali wa juu.
        assert!(r.severity == "high" || r.severity == "critical" || r.severity == "medium");
        // SMS ya utapeli.
        let s = execute("sms", "Umeshinda zawadi ya milioni 5, tuma namba yako ya siri");
        assert!(s.findings[0].score > 0);
    }

    #[tokio::test]
    async fn ripoti_ina_muhuri_na_huthibitika() {
        let db = pool().await;
        let res = execute("domain", "example.co.tz");
        let doc = report(&db, &res).await.unwrap();
        assert!(doc.sealed_html.contains("MBILINYI TECH SOLUTIONS"));
        assert!(doc.sealed_html.contains("Muhuri wa kidijitali"));
        assert_eq!(doc.seal.alg, pkicrypto::ALG_ECDSA);
        assert_eq!(doc.seal.content_hash.len(), 64);
        // Uthibitisho wa muhuri.
        let payload = format!("{}\n{}", doc.title, doc.body_html);
        let v = pkicrypto::verify_seal(&db, &payload, &doc.seal).await.unwrap();
        assert!(v.valid);
        // PDF ni halali.
        let pdf = report_pdf(&res, &doc.seal);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    }

    #[tokio::test]
    async fn kumbukumbu_ya_kazi_huhifadhiwa() {
        let db = pool().await;
        let res = execute("url", "http://bit.ly/x");
        let id = log_job(&db, "chunguza link", &res, "hash123").await.unwrap();
        assert!(id > 0);
        let j = jobs(&db, 10).await;
        assert_eq!(j["jobs"].as_array().unwrap().len(), 1);
        assert_eq!(j["jobs"][0]["target"], "http://bit.ly/x");
    }
}
