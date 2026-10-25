//! sampler.rs — KUCHAGUA tokeni (temperature, top-k, top-p, repetition penalty).
//!
//! Kila kazi ni PURE (logits → id) — hivyo inajaribiwa bila modeli.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SamplingParams {
    pub temperature: f32,
    pub top_k: usize,
    pub top_p: f32,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    pub seed: u64,
}

impl Default for SamplingParams {
    fn default() -> Self {
        SamplingParams {
            temperature: 0.7,
            top_k: 40,
            top_p: 0.95,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
            seed: 0x9E3779B97F4A7C15,
        }
    }
}

/// RNG ndogo (xorshift64*) — bila crate ya nje.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// Chagua tokeni kutoka logits. `history` = tokeni za hivi karibuni (penalty).
pub fn sample(logits: &mut [f32], p: &SamplingParams, history: &[u32], rng: &mut Rng) -> u32 {
    let n = logits.len();
    if n == 0 {
        return 0;
    }
    // 1) Repetition penalty.
    if p.repeat_penalty != 1.0 && !history.is_empty() {
        let start = history.len().saturating_sub(p.repeat_last_n);
        let mut seen: HashMap<u32, ()> = HashMap::new();
        for &t in &history[start..] {
            if seen.insert(t, ()).is_none() {
                let i = t as usize;
                if i < n {
                    if logits[i] > 0.0 {
                        logits[i] /= p.repeat_penalty;
                    } else {
                        logits[i] *= p.repeat_penalty;
                    }
                }
            }
        }
    }

    // 2) Greedy kama temperature ~ 0.
    if p.temperature <= 1e-5 {
        return argmax(logits);
    }

    // 3) Temperature.
    for v in logits.iter_mut() {
        *v /= p.temperature;
    }

    // 4) Top-k.
    let k = if p.top_k == 0 || p.top_k > n { n } else { p.top_k };
    let mut idx: Vec<usize> = (0..n).collect();
    if k < n {
        idx.select_nth_unstable_by(k - 1, |&a, &b| {
            logits[b].partial_cmp(&logits[a]).unwrap_or(std::cmp::Ordering::Equal)
        });
        idx.truncate(k);
    }
    idx.sort_unstable_by(|&a, &b| {
        logits[b].partial_cmp(&logits[a]).unwrap_or(std::cmp::Ordering::Equal)
    });

    // 5) Softmax juu ya waliochaguliwa.
    let maxv = logits[idx[0]];
    let mut probs: Vec<f32> = idx.iter().map(|&i| (logits[i] - maxv).exp()).collect();
    let sum: f32 = probs.iter().sum();
    for pr in probs.iter_mut() {
        *pr /= sum;
    }

    // 6) Top-p (nucleus).
    let mut cut = probs.len();
    if p.top_p < 1.0 {
        let mut acc = 0f32;
        for (i, &pr) in probs.iter().enumerate() {
            acc += pr;
            if acc >= p.top_p {
                cut = i + 1;
                break;
            }
        }
        cut = cut.max(1);
    }

    // 7) Sampuli.
    let total: f32 = probs[..cut].iter().sum();
    let mut r = rng.next_f32() * total;
    for i in 0..cut {
        r -= probs[i];
        if r <= 0.0 {
            return idx[i] as u32;
        }
    }
    idx[cut - 1] as u32
}

pub fn argmax(v: &[f32]) -> u32 {
    let mut bi = 0usize;
    let mut bv = f32::NEG_INFINITY;
    for (i, &x) in v.iter().enumerate() {
        if x > bv {
            bv = x;
            bi = i;
        }
    }
    bi as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_huchagua_kubwa() {
        let mut l = vec![0.1, 5.0, 0.2, 3.0];
        let p = SamplingParams {
            temperature: 0.0,
            ..Default::default()
        };
        let mut rng = Rng::new(1);
        assert_eq!(sample(&mut l, &p, &[], &mut rng), 1);
    }

    #[test]
    fn top_k_huzuia_wa_chini() {
        // Tokeni 0 ina uwezekano mkubwa; top_k=1 LAZIMA irudi 0 kila mara.
        let mut l = vec![10.0, 1.0, 1.0, 1.0];
        let p = SamplingParams {
            temperature: 1.0,
            top_k: 1,
            top_p: 1.0,
            repeat_penalty: 1.0,
            ..Default::default()
        };
        for s in 0..20 {
            let mut ll = l.clone();
            let mut rng = Rng::new(s);
            assert_eq!(sample(&mut ll, &p, &[], &mut rng), 0);
        }
        let _ = &mut l;
    }

    #[test]
    fn repetition_penalty_hupunguza() {
        let mut l = vec![10.0, 1.0];
        let p = SamplingParams {
            temperature: 0.0,
            repeat_penalty: 5.0,
            repeat_last_n: 64,
            ..Default::default()
        };
        let mut rng = Rng::new(1);
        // Tokeni 0 ilitokea → adhabu → 10/5 = 2.0 → bado kubwa (1.0)
        assert_eq!(sample(&mut l, &p, &[0], &mut rng), 0);
        // Kama adhabu ni kubwa mno, 0 inashindwa
        let mut l2 = vec![10.0, 1.0];
        let p2 = SamplingParams {
            temperature: 0.0,
            repeat_penalty: 20.0,
            repeat_last_n: 64,
            ..Default::default()
        };
        assert_eq!(sample(&mut l2, &p2, &[0], &mut rng), 1);
    }
}
