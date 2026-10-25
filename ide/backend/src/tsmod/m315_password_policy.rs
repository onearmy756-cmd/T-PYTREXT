//! m315_password_policy.rs — MODULE 315: PASSWORD POLICY
//!
//! Spec: "Inahakikisha mteja anatumia nenosiri gumu. Twist: Inahitaji nenosiri la herufi 8+."
//!
//! Injini halisi: hukagua nguvu ya nenosiri (urefu 8+, herufi kubwa/ndogo,
//! tarakimu, alama) na kukadiria entropy; hukataa maneno ya kawaida/yanayojulikana.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Maneno ya kawaida (weak).
const COMMON: &[&str] = &["password", "123456", "qwerty", "admin", "letmein", "welcome", "iloveyou", "111111", "abc123", "password1", "000000", "tanzania", "mteja", "simu"];

/// Kadiria entropy (bits) — kikadirio cha charset × urefu.
pub fn entropy_bits(pw: &str) -> f64 {
    let mut pool = 0u32;
    if pw.chars().any(|c| c.is_ascii_lowercase()) { pool += 26; }
    if pw.chars().any(|c| c.is_ascii_uppercase()) { pool += 26; }
    if pw.chars().any(|c| c.is_ascii_digit()) { pool += 10; }
    if pw.chars().any(|c| !c.is_alphanumeric()) { pool += 33; }
    if pool == 0 { return 0.0; }
    (pw.chars().count() as f64) * (pool as f64).log2()
}

/// Matokeo ya sera.
#[derive(Debug, Clone)]
pub struct PolicyResult {
    pub score: u8,
    pub passes: bool,
    pub issues: Vec<String>,
    pub entropy_bits: f64,
}

/// Kagua nenosiri.
pub fn check(pw: &str, min_len: usize) -> PolicyResult {
    let mut issues = Vec::new();
    let mut score = 0u8;
    let n = pw.chars().count();
    if n >= min_len { score += 30; } else { issues.push(format!("Urefu {n} < {min_len}")); }
    if pw.chars().any(|c| c.is_ascii_uppercase()) { score += 15; } else { issues.push("Hakuna herufi kubwa".into()); }
    if pw.chars().any(|c| c.is_ascii_lowercase()) { score += 15; } else { issues.push("Hakuna herufi ndogo".into()); }
    if pw.chars().any(|c| c.is_ascii_digit()) { score += 15; } else { issues.push("Hakuna tarakimu".into()); }
    if pw.chars().any(|c| !c.is_alphanumeric()) { score += 15; } else { issues.push("Hakuna alama maalum".into()); }
    let low = pw.to_lowercase();
    if COMMON.iter().any(|c| low.contains(c)) {
        score = score.saturating_sub(40);
        issues.push("Nenosiri la kawaida linaonekana".into());
    }
    let ent = entropy_bits(pw);
    if ent >= 60.0 { score = score.min(100).saturating_add(5).min(100); }
    let passes = n >= min_len && score >= 70 && !COMMON.iter().any(|c| low.contains(c));
    PolicyResult { score: score.min(100), passes, issues, entropy_bits: ent }
}

/// Hash ya nenosiri (SHA-256 + salt) — hazihifadhi nenosiri.
pub fn hash_password(pw: &str, salt: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(salt);
    h.update(pw.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub struct M315PasswordPolicy;

impl TsModule for M315PasswordPolicy {
    fn id(&self) -> u16 {
        315
    }
    fn name(&self) -> &'static str {
        "Password Policy"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hukagua nguvu ya nenosiri (urefu 8+, herufi/tarakimu/alama, entropy) na \
         hukataa maneno ya kawaida."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(315, self.name());
        let pw = p.get("password").and_then(|v| v.as_str()).unwrap_or("Nguvu@2026!");
        let min_len = p.get("min_len").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
        let res = check(pw, min_len);
        for i in &res.issues {
            r.finding("policy_issue", i.clone(), 40);
        }
        r.findings.push(json!({ "kind": "policy", "severity": if res.passes { 1 } else { 70 },
            "detail": { "passes": res.passes, "score": res.score, "entropy_bits": format!("{:.1}", res.entropy_bits), "min_len": min_len } }));
        r.summary = format!("Password Policy: {} (score {}, entropy {:.0} bits).", if res.passes { "imekubaliwa" } else { "imekataliwa" }, res.score, res.entropy_bits);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nenosiri_dhaifu_hakubaliwi() {
        assert!(!check("password", 8).passes);
        assert!(!check("123456", 8).passes);
    }

    #[test]
    fn nenosiri_gumu_hukubaliwa() {
        let res = check("Nguvu@2026!x", 8);
        assert!(res.passes, "score {}", res.score);
        assert!(res.entropy_bits > 60.0);
    }
}
