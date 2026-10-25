//! m517_payment_callback_verify.rs — MODULE 517: PAYMENT CALLBACK VERIFICATION
//!
//! Spec (ulinzi): hati "SECURITY CODE AGENT J 2" inaeleza mashambulizi halisi
//! dhidi ya fedha za simu — kughushi callback ya M-Pesa, kutumia tena
//! CheckoutRequestID, kukwepa IP whitelist, na kubadilisha customer_id
//! (IDOR/Broken Access Control). Module hii NI ULINZI: hutathmini callback ya
//! malipo inayoingia na kuamua ikiwa inaweza kuaminiwa, BILA kughushi lolote.
//!
//! Kanuni: callback inaaminiwa TU ikiwa
//!   - sahihi (HMAC) inathibitika,
//!   - IP ipo kwenye whitelist ya telco,
//!   - CheckoutRequestID haijatumika tena (replay/duplicate),
//!   - kiasi + akaunti vinalingana na ombi halisi,
//!   - timestamp iko ndani ya dirisha la muda (freshness),
//!   - callback haijafika KABLA ya ombi halisi (sequence).
//!
//! Hii huzuia "mark account paid without money", callback replay, IDOR, na
//! callback za DEBUG/stack trace zinazoiba CONSUMER_KEY/SECRET.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Sahihi ya HMAC-SHA256 (hex) kwa mwili wa callback + siri ya merchant.
pub fn hmac_sha256_hex(secret: &str, body: &str) -> String {
    // HMAC-SHA256 halisi (RFC 2104) — bila kutegemea crate ya nje.
    let block = 64usize;
    let mut key = secret.as_bytes().to_vec();
    if key.len() > block {
        key = Sha256::digest(&key).to_vec();
    }
    key.resize(block, 0);
    let mut ipad = vec![0x36u8; block];
    let mut opad = vec![0x5cu8; block];
    for i in 0..block {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = ipad;
    inner.extend_from_slice(body.as_bytes());
    let inner_hash = Sha256::digest(&inner);
    let mut outer = opad;
    outer.extend_from_slice(&inner_hash);
    let out = Sha256::digest(&outer);
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// Matokeo ya uthibitisho wa callback.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub trusted: bool,
    pub score: u8,
    pub reasons: Vec<String>,
}

/// Hali ya ombi halisi (kutoka leja, sio kutoka callback yenyewe).
#[derive(Debug, Clone, Default)]
pub struct PaymentRequest {
    pub checkout_request_id: String,
    pub amount: f64,
    pub account: String,
    pub created_at: i64,
    pub already_settled: bool,
}

/// Callback inayoingia (inaweza kuwa ghushi).
#[derive(Debug, Clone)]
pub struct Callback {
    pub checkout_request_id: String,
    pub amount: f64,
    pub account: String,
    pub signature_hex: String,
    pub source_ip: String,
    pub ts: i64,
    pub settled: bool,
}

const TELCO_IP_WHITELIST: &[&str] = &["196.201.214.200", "196.201.214.206", "196.201.213.114"];

/// Kagua callback dhidi ya ombi halisi. Hii ni DETECTION — haifanyi malipo.
pub fn verify(req: &PaymentRequest, cb: &Callback, merchant_secret: &str, now: i64, replay_seen: bool) -> Verdict {
    let mut score = 0u8;
    let mut reasons = Vec::new();

    // 1) Sahihi halisi (HMAC). Mwili unaothibitishwa = id|account|amount|ts.
    let body = format!("{}|{}|{:.2}|{}", cb.checkout_request_id, cb.account, cb.amount, cb.ts);
    let expected = hmac_sha256_hex(merchant_secret, &body);
    if cb.signature_hex.eq_ignore_ascii_case(&expected) {
        reasons.push("Sahihi HMAC ni sahihi".to_string());
    } else {
        score = score.max(100);
        reasons.push("Sahihi ya callback HAIthibitiki — uwezekano wa kughushi".to_string());
    }

    // 2) IP whitelist ya telco.
    if TELCO_IP_WHITELIST.contains(&cb.source_ip.as_str()) {
        reasons.push(format!("IP {} iko kwenye whitelist", cb.source_ip));
    } else {
        score = score.max(85);
        reasons.push(format!("IP {} HAIPO kwenye whitelist ya telco", cb.source_ip));
    }

    // 3) Replay / duplicate CheckoutRequestID.
    if replay_seen {
        score = score.max(90);
        reasons.push("CheckoutRequestID imetumika tena (replay/duplicate)".to_string());
    }

    // 4) Kiasi na akaunti vinalingana na ombi halisi.
    if (req.amount - cb.amount).abs() > 0.001 {
        score = score.max(90);
        reasons.push(format!("Kiasi hakilingani: ombi {:.2}, callback {:.2}", req.amount, cb.amount));
    }
    if req.account != cb.account {
        score = score.max(95);
        reasons.push("Akaunti HAILINGANI na ombi (IDOR / kubadilisha customer_id)".to_string());
    }

    // 5) Freshness ya timestamp (±10 dakika).
    let drift = (now - cb.ts).abs();
    if drift > 600 {
        score = score.max(70);
        reasons.push(format!("Timestamp imepitwa na wakati (drift {drift}s)"));
    }

    // 6) Sequence: callback KABLA ya ombi halisi kuundwa = ghushi.
    if cb.ts < req.created_at {
        score = score.max(95);
        reasons.push("Callback imefika KABLA ya ombi halisi (callback forgery)".to_string());
    }

    // 7) Ombi limeshalipwa lakini callback inadai malipo — reconciliation.
    if req.already_settled && cb.settled {
        score = score.max(60);
        reasons.push("Ombi limeshalipwa; callback ya pili inadai malipo tena".to_string());
    }

    let trusted = score == 0;
    Verdict { trusted, score, reasons }
}

pub struct M517PaymentCallbackVerify;

#[async_trait::async_trait]
impl TsModule for M517PaymentCallbackVerify {
    fn id(&self) -> u16 {
        517
    }
    fn name(&self) -> &'static str {
        "Payment Callback Verification"
    }
    fn group(&self) -> &'static str {
        "payments"
    }
    fn about(&self) -> &'static str {
        "Huthibitisha callback za malipo (M-Pesa/HaloPesa): HMAC, IP whitelist, \
         replay/duplicate, kiasi, IDOR, freshness na sequence. Huzuia 'malipo \
         yamekamilika' ya uongo na callback ghushi. ULINZI, sio kughushi."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(517, self.name());

        let secret = p.get("merchant_secret").and_then(|v| v.as_str()).unwrap_or("");
        if secret.is_empty() {
            r.ok = false;
            r.summary = "merchant_secret inahitajika kuthibitisha sahihi.".to_string();
            r.finding("config", "merchant_secret haijatolewa", 95);
            return Ok(r);
        }

        let id = p.get("checkout_request_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let amount = p.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let account = p.get("account").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let sig = p.get("signature").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let ip = p.get("source_ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let now = p.get("now").and_then(|v| v.as_i64()).unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
        let req = PaymentRequest {
            checkout_request_id: id.clone(),
            amount: p.get("req_amount").and_then(|v| v.as_f64()).unwrap_or(amount),
            account: p.get("req_account").and_then(|v| v.as_str()).unwrap_or(&account).to_string(),
            created_at: p.get("req_created_at").and_then(|v| v.as_i64()).unwrap_or(now),
            already_settled: p.get("already_settled").and_then(|v| v.as_bool()).unwrap_or(false),
        };
        let cb = Callback {
            checkout_request_id: id,
            amount,
            account,
            signature_hex: sig,
            source_ip: ip,
            ts: p.get("ts").and_then(|v| v.as_i64()).unwrap_or(now),
            settled: p.get("settled").and_then(|v| v.as_bool()).unwrap_or(true),
        };
        let replay = p.get("replay_seen").and_then(|v| v.as_bool()).unwrap_or(false);

        // "DEBUG=True inavujisha funguo" — kataa ikiwa callback ina mabaki ya stack trace.
        if p.get("leaks_credentials").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.finding("debug_leak", "Callback ina CONSUMER_KEY/SECRET (DEBUG mode leak)", 95);
        }

        let v = verify(&req, &cb, secret, now, replay);
        for why in &v.reasons {
            r.finding("callback_check", why.clone(), if v.trusted { 1 } else { v.score });
        }
        r.findings.push(json!({
            "kind": "decision",
            "severity": if v.trusted { 1 } else { v.score },
            "detail": {
                "trusted": v.trusted,
                "score": v.score,
                "action": if v.trusted { "kubali malipo" } else { "kataa callback + weka kwenye foleni ya ukaguzi" },
            }
        }));
        r.summary = format!(
            "Payment Callback Verification: {} (score {}).",
            if v.trusted { "IMETHIBITIKA" } else { "SIKUTUMIKA" },
            v.score
        );
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cb(id: &str, amount: f64, account: &str, ip: &str, ts: i64, secret: &str) -> Callback {
        let body = format!("{id}|{account}|{amount:.2}|{ts}");
        Callback {
            checkout_request_id: id.to_string(),
            amount,
            account: account.to_string(),
            signature_hex: hmac_sha256_hex(secret, &body),
            source_ip: ip.to_string(),
            ts,
            settled: true,
        }
    }

    #[test]
    fn callback_halali_inaaminiwa() {
        let secret = "s3cr3t";
        let req = PaymentRequest {
            checkout_request_id: "ws_CO_1".into(),
            amount: 5000.0,
            account: "255712345678".into(),
            created_at: 1000,
            already_settled: false,
        };
        let c = cb("ws_CO_1", 5000.0, "255712345678", "196.201.214.200", 1005, secret);
        let v = verify(&req, &c, secret, 1006, false);
        assert!(v.trusted, "{:?}", v.reasons);
        assert_eq!(v.score, 0);
    }

    #[test]
    fn callback_ghushi_ya_sahihi_inakataliwa() {
        let secret = "s3cr3t";
        let req = PaymentRequest {
            checkout_request_id: "ws_CO_2".into(),
            amount: 5000.0,
            account: "255712345678".into(),
            created_at: 1000,
            already_settled: false,
        };
        let mut c = cb("ws_CO_2", 5000.0, "255712345678", "196.201.214.200", 1005, secret);
        c.signature_hex = "deadbeef".into();
        let v = verify(&req, &c, secret, 1006, false);
        assert!(!v.trusted);
        assert!(v.score >= 100);
    }

    #[test]
    fn callback_kabla_ya_ombi_ni_ghushi() {
        let secret = "s3cr3t";
        let req = PaymentRequest {
            checkout_request_id: "ws_CO_3".into(),
            amount: 5000.0,
            account: "255712345678".into(),
            created_at: 2000,
            already_settled: false,
        };
        // Callback ts=1500 KABLA ya ombi (created 2000) lakini sahihi njoo kwa hiyo ts.
        let c = cb("ws_CO_3", 5000.0, "255712345678", "196.201.214.200", 1500, secret);
        let v = verify(&req, &c, secret, 2050, false);
        assert!(!v.trusted);
        assert!(v.reasons.iter().any(|r| r.contains("KABLA")));
    }

    #[test]
    fn idor_akaunti_isiyolingana_inakataliwa() {
        let secret = "s3cr3t";
        let req = PaymentRequest {
            checkout_request_id: "ws_CO_4".into(),
            amount: 5000.0,
            account: "255700000000".into(),
            created_at: 1000,
            already_settled: false,
        };
        let c = cb("ws_CO_4", 5000.0, "255799999999", "196.201.214.200", 1005, secret);
        let v = verify(&req, &c, secret, 1006, false);
        assert!(!v.trusted);
        assert!(v.reasons.iter().any(|r| r.contains("IDOR")));
    }

    #[test]
    fn replay_inakataliwa() {
        let secret = "s3cr3t";
        let req = PaymentRequest {
            checkout_request_id: "ws_CO_5".into(),
            amount: 5000.0,
            account: "255712345678".into(),
            created_at: 1000,
            already_settled: false,
        };
        let c = cb("ws_CO_5", 5000.0, "255712345678", "196.201.214.200", 1005, secret);
        let v = verify(&req, &c, secret, 1006, true);
        assert!(!v.trusted);
        assert!(v.reasons.iter().any(|r| r.contains("replay")));
    }

    #[test]
    fn hmac_inalingana_na_rfc_2104() {
        // Kwa "key" + "The quick brown fox jumps over the lazy dog"
        let got = hmac_sha256_hex("key", "The quick brown fox jumps over the lazy dog");
        assert_eq!(
            got,
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }
}
