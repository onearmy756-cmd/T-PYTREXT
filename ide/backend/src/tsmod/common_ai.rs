//! common_ai.rs — visaidizi vya AI/ML vinavyoshirikiwa (SEHEMU 10-11, modules 21-34).
//!
//! Algorithms HALISI (bila crates za nje): digital fingerprint distance, Holt
//! double-exponential forecasting, Q-learning update, streaming stats (Welford),
//! z-score anomaly, cosine similarity, na shannon entropy.

use serde_json::{json, Value};

/// Umbali wa fingerprints (0.0 = sawa kabisa, 1.0 = tofauti kabisa).
/// Vipimo 7 vya spec: typing, voice, gait, device, location, time, social.
pub fn fingerprint_distance(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 1.0;
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        let d = (a[i] - b[i]).abs();
        num += d;
        den += a[i].abs().max(b[i].abs()).max(1.0);
    }
    (num / den).clamp(0.0, 1.0)
}

/// Uamuzi wa kuingia kulingana na umbali wa fingerprint.
/// <0.30 allow, 0.30-0.50 step_up, >0.50 deny (kama spec ya Module 21).
pub fn fingerprint_decision(distance: f64) -> &'static str {
    if distance < 0.30 {
        "allow"
    } else if distance <= 0.50 {
        "step_up"
    } else {
        "deny"
    }
}

/// Holt double-exponential smoothing (forecast ya mfululizo wa muda).
/// Hurudisha (forecast, level, trend). `alpha`, `beta` ni vigezo vya kujifunza.
pub fn holt_forecast(series: &[f64], alpha: f64, beta: f64) -> (f64, f64, f64) {
    if series.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut level = series[0];
    let mut trend = if series.len() > 1 { series[1] - series[0] } else { 0.0 };
    for &y in &series[1..] {
        let prev_level = level;
        level = alpha * y + (1.0 - alpha) * (level + trend);
        trend = beta * (level - prev_level) + (1.0 - beta) * trend;
    }
    (level + trend, level, trend)
}

/// Q-learning update: Q(s,a) <- Q + alpha*(r + gamma*maxQ(s') - Q).
pub fn q_update(q: f64, reward: f64, max_next_q: f64, alpha: f64, gamma: f64) -> f64 {
    q + alpha * (reward + gamma * max_next_q - q)
}

/// Welford streaming mean/variance (single pass).
#[derive(Debug, Clone, Default)]
pub struct Welford {
    pub n: u64,
    pub mean: f64,
    pub m2: f64,
}

impl Welford {
    pub fn push(&mut self, x: f64) {
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / self.n as f64;
        self.m2 += delta * (x - self.mean);
    }
    pub fn variance(&self) -> f64 {
        if self.n < 2 {
            0.0
        } else {
            self.m2 / (self.n - 1) as f64
        }
    }
    pub fn std(&self) -> f64 {
        self.variance().sqrt()
    }
    /// z-score ya thamani mpya.
    pub fn zscore(&self, x: f64) -> f64 {
        let s = self.std();
        if s == 0.0 {
            0.0
        } else {
            (x - self.mean) / s
        }
    }
}

/// Cosine similarity (0..1).
pub fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..n {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot / (na.sqrt() * nb.sqrt())).clamp(0.0, 1.0)
    }
}

/// Shannon entropy ya histogram (bits).
pub fn entropy_bits(counts: &[u64]) -> f64 {
    let total: u64 = counts.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let mut h = 0.0;
    for &c in counts {
        if c > 0 {
            let p = c as f64 / total as f64;
            h -= p * p.log2();
        }
    }
    h
}

/// MinHash-like Jaccard kati ya seti mbili (kwa mifumo/IOC).
pub fn jaccard<T: Eq + std::hash::Hash + Clone>(a: &[T], b: &[T]) -> f64 {
    use std::collections::HashSet;
    let sa: HashSet<&T> = a.iter().collect();
    let sb: HashSet<&T> = b.iter().collect();
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// Uainishaji wa hisia rahisi (Kiswahili) — kwa Module 28.
pub fn sentiment_sw(text: &str) -> (f64, &'static str) {
    const NEG: &[&str] = &["hasira", "hofu", "huzuni", "woga", "kasirika", "kufa", "hatari", "tishio", "kosa", "lalamika"];
    const POS: &[&str] = &["asante", "furaha", "vizuri", "hongera", "salama", "nzuri", "pongezi"];
    let low = text.to_lowercase();
    let neg = NEG.iter().filter(|w| low.contains(*w)).count() as f64;
    let pos = POS.iter().filter(|w| low.contains(*w)).count() as f64;
    let total = neg + pos;
    if total == 0.0 {
        return (0.0, "neutral");
    }
    let score = (pos - neg) / total;
    let label = if score < -0.2 { "negative" } else if score > 0.2 { "positive" } else { "neutral" };
    (score, label)
}

pub fn finding_json(kind: &str, detail: impl Into<String>, severity: u8) -> Value {
    json!({ "kind": kind, "detail": detail.into(), "severity": severity })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_umbali_na_uamuzi() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let b = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        assert!(fingerprint_distance(&a, &b) < 0.05);
        assert_eq!(fingerprint_decision(fingerprint_distance(&a, &b)), "allow");
        let c = [9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0];
        assert_eq!(fingerprint_decision(fingerprint_distance(&a, &c)), "deny");
    }

    #[test]
    fn holt_inatabiri_mwelekeo() {
        let series: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let (f, _l, t) = holt_forecast(&series, 0.5, 0.5);
        assert!(t > 0.0, "trend ilikuwa {t}");
        assert!(f > 20.0);
    }

    #[test]
    fn welford_zscore() {
        let mut w = Welford::default();
        for x in [10.0, 12.0, 11.0, 13.0, 12.0] { w.push(x); }
        assert!((w.mean - 11.6).abs() < 0.01);
        assert!(w.zscore(100.0) > 5.0);
    }

    #[test]
    fn q_update_inajifunza() {
        let q = q_update(0.0, 1.0, 0.0, 0.1, 0.9);
        assert!(q > 0.0);
    }

    #[test]
    fn sentiment_kiswahili() {
        let (s, l) = sentiment_sw("Nina hasira na hofu kubwa");
        assert!(s < 0.0 && l == "negative");
    }
}
