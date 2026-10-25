//! m287_emergency_call_security.rs — MODULE 287: EMERGENCY CALL SECURITY
//!
//! Spec: "Inapa priority kwa simu za dharura. Twist: Inaweza kukata simu ya kawaida kufungua nafasi."
//!
//! Injini halisi: mpangaji wa kipaumbele wa simu (priority scheduler):
//!   - Hutambua namba za dharura (112, 110, 111, 114, 115 — TZ; 911, 999).
//!   - Hupanga foleni: dharura zina kipaumbele cha juu; simu za kawaida zinaweza
//!     kukatwa (preemption) kufungua nafasi kwenye mstari.
//!   - Huhesabu "congestion" na hutoa maamuzi: admit | preempt | queue | drop.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Namba za dharura za Tanzania + kimataifa.
const EMERGENCY: &[&str] = &["112", "110", "111", "114", "115", "116", "999", "911", "000", "0800"];

pub fn is_emergency(num: &str) -> bool {
    let d: String = num.chars().filter(|c| c.is_ascii_digit()).collect();
    if EMERGENCY.contains(&d.as_str()) || d.starts_with("0800") {
        return true;
    }
    // Ondoa country code ya TZ kama ipo (mfano +255-112 → 112).
    if let Some(rest) = d.strip_prefix("255") {
        if EMERGENCY.contains(&rest) {
            return true;
        }
    }
    false
}

/// Kipaumbele (0 = juu).
pub fn priority(num: &str) -> u8 {
    if is_emergency(num) {
        0
    } else if num.starts_with("+255") || num.len() >= 9 {
        2
    } else {
        1
    }
}

/// Maamuzi ya mpangaji kwa simu inayoingia.
#[derive(Debug, PartialEq)]
pub enum Decision {
    Admit,
    Preempt,
    Queue,
    Drop,
}

pub fn schedule(incoming: &str, active_normal: usize, capacity: usize) -> Decision {
    let p = priority(incoming);
    if p == 0 {
        // Dharura: ingiza; kama imejaa, kata simu ya kawaida.
        if active_normal >= capacity {
            Decision::Preempt
        } else {
            Decision::Admit
        }
    } else if active_normal < capacity {
        Decision::Admit
    } else if active_normal < capacity + 3 {
        Decision::Queue
    } else {
        Decision::Drop
    }
}

pub struct M287EmergencyCallSecurity;

impl TsModule for M287EmergencyCallSecurity {
    fn id(&self) -> u16 {
        287
    }
    fn name(&self) -> &'static str {
        "Emergency Call Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Hupa kipaumbele simu za dharura (112/110/115); hukata simu za kawaida \
         kufungua nafasi mstari unapokuwa umejaa."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(287, self.name());
        let incoming = params.get("number").and_then(|v| v.as_str()).unwrap_or("112");
        let active = params.get("active_normal").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let cap = params.get("capacity").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let d = schedule(incoming, active, cap);
        let (k, sev) = match d {
            Decision::Admit => ("admit", 1),
            Decision::Preempt => ("preempt", 40),
            Decision::Queue => ("queue", 30),
            Decision::Drop => ("drop", 55),
        };
        r.finding(k, format!("Simu {incoming}: {d:?} (active {active}/{cap})"), sev);
        r.findings.push(json!({ "kind": "scheduler", "severity": 1, "detail": {
            "emergency": is_emergency(incoming), "priority": priority(incoming),
            "active_normal": active, "capacity": cap } }));
        r.summary = format!("Emergency Call Security: {incoming} → {d:?}.");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dharura_inapata_kipaumbele() {
        assert!(is_emergency("112"));
        assert!(is_emergency("+255-112"));
        assert!(!is_emergency("0712345678"));
        assert_eq!(schedule("112", 5, 5), Decision::Preempt);
        assert_eq!(schedule("0712345678", 5, 5), Decision::Queue);
    }
}
