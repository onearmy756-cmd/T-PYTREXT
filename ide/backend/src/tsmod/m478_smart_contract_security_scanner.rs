//! m478_smart_contract_security_scanner.rs — MODULE 478: SMART CONTRACT SECURITY SCANNER
//!
//! Spec: "Kuchambua smart contracts kwa hatari. Twist: Inagundua vulnerabilities."
//!
//! Injini halisi: huchanganua programu ya smart contract (opcodes) na kugundua
//! udhaifu: reentrancy (transfers nyingi), loop zisizo na kikomo (gas exhaustion),
//! kukosekana kwa Halt (utekelezaji usiodhibitiwa), na hatari ya overflow.

use super::common_blockchain::Op;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Kipengele cha hatari.
#[derive(Debug, Clone)]
pub struct Vulnerability {
    pub name: &'static str,
    pub severity: u8,
    pub detail: String,
}

/// Changanua program → udhaifu.
pub fn scan(program: &[Op]) -> Vec<Vulnerability> {
    let mut v = Vec::new();
    let transfers = program.iter().filter(|o| matches!(o, Op::Transfer(_, _))).count();
    if transfers > 1 {
        v.push(Vulnerability {
            name: "reentrancy",
            severity: 85,
            detail: format!("transfers {transfers} — reentrancy inawezekana (hakikisha state inabadilishwa kabla ya transfer)"),
        });
    }
    // Backward jump → loop inayoweza kuwa isiyo na kikomo.
    let mut has_backward_jump = false;
    for (i, op) in program.iter().enumerate() {
        if let Op::JumpIfZero(t) = op {
            if *t <= i {
                has_backward_jump = true;
            }
        }
    }
    if has_backward_jump {
        v.push(Vulnerability { name: "unbounded_loop", severity: 80, detail: "JumpIfZero inarudi nyuma — gas exhaustion".into() });
    }
    if !program.iter().any(|o| matches!(o, Op::Halt)) {
        v.push(Vulnerability { name: "missing_halt", severity: 70, detail: "hakuna Halt — utekelezaji unaweza kuendelea".into() });
    }
    let arith = program.iter().filter(|o| matches!(o, Op::Mul | Op::Add)).count();
    if arith > 0 && transfers > 0 {
        v.push(Vulnerability { name: "overflow_risk", severity: 55, detail: format!("shughuli {arith} za hesabu bila ukaguzi wa overflow") });
    }
    if transfers == 0 && program.is_empty() {
        v.push(Vulnerability { name: "empty_contract", severity: 40, detail: "contract tupu".into() });
    }
    v
}

pub struct M478SmartContractSecurityScanner;

impl TsModule for M478SmartContractSecurityScanner {
    fn id(&self) -> u16 {
        478
    }
    fn name(&self) -> &'static str {
        "Smart Contract Security Scanner"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Huchanganua smart contracts (opcodes) na kugundua udhaifu: reentrancy, \
         loop zisizo na kikomo, Halt inayokosekana, overflow."
    }

    fn run(&self, _t: &str, _p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(478, self.name());
        // Contract ya mfano yenye matatizo (transfers mbili, hakuna Halt).
        let program = vec![
            Op::Push(10), Op::Transfer("a".into(), "b".into()),
            Op::Push(20), Op::Transfer("a".into(), "c".into()),
        ];
        let vulns = scan(&program);
        let score: u8 = vulns.iter().map(|v| v.severity).max().unwrap_or(0);
        for v in &vulns {
            r.finding(v.name, v.detail.clone(), v.severity);
        }
        r.findings.push(json!({ "kind": "scan_summary", "severity": score,
            "detail": { "vulnerabilities": vulns.len(), "max_severity": score, "safe": vulns.is_empty() } }));
        r.summary = format!("Smart Contract Scanner: udhaifu {} (kiwango cha juu {}).", vulns.len(), score);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_reentrancy_na_loop() {
        let prog = vec![
            Op::Push(1), Op::Transfer("a".into(), "b".into()),
            Op::Push(2), Op::Transfer("a".into(), "c".into()),
            Op::JumpIfZero(0),
        ];
        let v = scan(&prog);
        assert!(v.iter().any(|x| x.name == "reentrancy"));
        assert!(v.iter().any(|x| x.name == "unbounded_loop"));
        assert!(v.iter().any(|x| x.name == "missing_halt"));
    }
}
