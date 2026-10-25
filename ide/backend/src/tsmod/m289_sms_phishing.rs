//! m289_sms_phishing.rs — MODULE 289: SMS PHISHING
//!
//! Spec: "Inagundua na kuzuia SMS za ulaghai. Twist: Inabonyeza link kwenye sandbox."
//!
//! Injini halisi: huchanganua SMS kwa alama za smishing (OTP, zawadi, uharaka,
//! viungo hatarishi) na "hubonyeza" kiungo kwenye sandbox ya uchanganuzi
//! (haitekelezi ombi la mtandao — inachanganua muundo/hatari pekee, salama).

use super::common_smsemail::{extract_links, link_score, smishing_score};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Uchanganuzi wa "sandbox": hatari ya kiungo bila kutoka mtandao.
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub url: String,
    pub domain: String,
    pub score: u8,
    pub reasons: Vec<String>,
    pub redirect_chain_suspected: bool,
    pub asks_credentials: bool,
    pub downloads_file: bool,
}

pub fn sandbox_scan(url: &str) -> SandboxResult {
    let (score, why) = link_score(url);
    let d = super::common_smsemail::domain_of(url);
    let low = url.to_lowercase();
    let asks = low.contains("login") || low.contains("verify") || low.contains("account") || low.contains("thibitisha") || low.contains("weka") || low.contains("password");
    let dl = low.ends_with(".apk") || low.ends_with(".exe") || low.ends_with(".zip") || low.contains("download") || low.contains(".apk?");
    SandboxResult {
        url: url.to_string(),
        domain: d,
        score,
        reasons: why.into_iter().map(|(k, v)| format!("{k}: {v}")).collect(),
        redirect_chain_suspected: super::common_smsemail::is_shortener(url),
        asks_credentials: asks,
        downloads_file: dl,
    }
}

pub struct M289SmsPhishing;

#[async_trait::async_trait]
impl TsModule for M289SmsPhishing {
    fn id(&self) -> u16 {
        289
    }
    fn name(&self) -> &'static str {
        "SMS Phishing"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Hugundua SMS za ulaghai (smishing) na huchanganua viungo kwenye sandbox \
         (muundo/hatari) bila kutekeleza ombi la mtandao."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(289, self.name());
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or(target);
        let sender = params.get("sender").and_then(|v| v.as_str()).unwrap_or("");
        let (score, why) = smishing_score(sender, text);
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        for l in extract_links(text) {
            let sb = sandbox_scan(&l);
            r.findings.push(json!({
                "kind": "sandbox", "severity": sb.score.max(1),
                "detail": { "url": sb.url, "domain": sb.domain, "score": sb.score,
                            "asks_credentials": sb.asks_credentials, "downloads_file": sb.downloads_file,
                            "redirect": sb.redirect_chain_suspected, "reasons": sb.reasons }
            }));
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("SMS Phishing: uamuzi {decision} (score {score}).");
        Ok(r)
    }

    async fn run_db(&self, _db: &SqlitePool, target: &str, params: &Value) -> Result<ModReport, String> {
        self.run(target, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_smishing_ya_otp() {
        let (s, why) = smishing_score("+23480", "Thibitisha akaunti yako haraka! Tuma OTP yako kwa https://bit.ly/x");
        assert!(s >= 70);
        assert!(why.iter().any(|(k, _)| *k == "otp_request"));
    }

    #[test]
    fn sandbox_inaona_apk() {
        let sb = sandbox_scan("http://192.168.0.9/app.apk");
        assert!(sb.downloads_file);
        assert!(sb.score >= 75);
    }
}
