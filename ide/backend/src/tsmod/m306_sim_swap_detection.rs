//! m306_sim_swap_detection.rs — MODULE 306: SIM SWAP DETECTION
//!
//! Spec: "Inafuatilia ikiwa SIM imebadilishwa. Twist: SIM Swap Kill Switch."
//!
//! Injini halisi: hufuatilia mabadiliko ya ICCID/IMSI ya SIM kwa namba; ikiwa SIM
//! imebadilishwa ghafla (haswa karibu na ombi la OTP), inawasha kill switch:
//! inasimamisha miamala na inahitaji uthibitisho wa kibinafsi.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Tukio la mabadiliko ya SIM.
#[derive(Debug, Clone)]
pub struct SimChange {
    pub iccid_old: String,
    pub iccid_new: String,
    pub ts: i64,
    pub otp_recently: bool,
}

/// Uamuzi wa sim-swap.
pub fn evaluate(prev_iccid: Option<&str>, cur_iccid: &str, otp_recently: bool, hours_since_change: f64) -> (u8, bool, Vec<String>) {
    let mut reasons = Vec::new();
    let mut score = 0u8;
    let changed = prev_iccid.map(|p| p != cur_iccid).unwrap_or(false);
    if changed {
        score = 60;
        reasons.push("ICCID imebadilika".to_string());
    }
    if changed && otp_recently {
        score = 95;
        reasons.push("Mabadiliko ya SIM karibu na ombi la OTP — sim-swap!".to_string());
    }
    if changed && hours_since_change < 24.0 {
        score = score.max(80);
        reasons.push(format!("SIM ilibadilishwa saa {hours_since_change:.1} zilizopita"));
    }
    let kill = score >= 80;
    (score, kill, reasons)
}

pub struct M306SimSwapDetection;

#[async_trait::async_trait]
impl TsModule for M306SimSwapDetection {
    fn id(&self) -> u16 {
        306
    }
    fn name(&self) -> &'static str {
        "SIM Swap Detection"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hufuatilia mabadiliko ya ICCID/IMSI; sim-swap karibu na OTP inawasha \
         kill switch (miamala inasimama, uthibitisho wa kibinafsi unahitajika)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(306, self.name());
        let prev = p.get("prev_iccid").and_then(|v| v.as_str());
        let cur = p.get("iccid").and_then(|v| v.as_str()).unwrap_or("8900000000000000000");
        let otp = p.get("otp_recently").and_then(|v| v.as_bool()).unwrap_or(false);
        let hrs = p.get("hours_since_change").and_then(|v| v.as_f64()).unwrap_or(48.0);
        let (score, kill, why) = evaluate(prev, cur, otp, hrs);
        for w in &why {
            r.finding("sim_change", w.clone(), score);
        }
        r.findings.push(json!({ "kind": "kill_switch", "severity": if kill { 95 } else { 1 },
            "detail": { "active": kill, "score": score, "action": if kill { "simamisha miamala + uthibitisho wa kibinafsi" } else { "hakuna" } } }));
        r.summary = format!("SIM Swap Detection: score {score}, kill switch {}.", if kill { "IMEWASHWA" } else { "imezimwa" });
        Ok(r)
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, p: &Value) -> Result<ModReport, String> {
        let mut rep = self.run(target, p)?;
        let prev: Option<(String,)> = sqlx::query_as("SELECT value FROM ts_iocs WHERE kind='sim_iccid' AND value<>? LIMIT 1")
            .bind(p.get("iccid").and_then(|v| v.as_str()).unwrap_or(""))
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
        if let Some((old,)) = prev {
            rep.finding("db_history", format!("ICCID ya nyuma kwenye DB: {old}"), 1);
        }
        Ok(rep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_swap_na_otp_inawasha_kill() {
        let (s, kill, _) = evaluate(Some("890old"), "890new", true, 2.0);
        assert!(s >= 80);
        assert!(kill);
    }

    #[test]
    fn sim_ile_ile_haina_alama() {
        let (s, kill, _) = evaluate(Some("890same"), "890same", true, 2.0);
        assert_eq!(s, 0);
        assert!(!kill);
    }
}
