//! cortex.rs — CORTEX: safu ya cortical yenye tabia ya ubongo wa binadamu.
//!
//! Hii ni safu inayojenga juu ya `snn.rs` (LIF neurons) na kuongeza tabia za
//! cortex ya kibinadamu kwa **matumizi ya nishati NDOGO** — bila kuunda modeli
//! mpya na bila kupoteza uwezo wa code:
//!
//!   - **Minicolumns**: kikundi cha neurons kinachoshiriki pembejeo.
//!   - **Lateral inhibition** (k-WTA): column yenye nguvu hukandamiza wenzake —
//!     hutengeneza sparse coding (kama cortex halisi).
//!   - **Recurrent excitation**: msisimko wa kurudia kwa muda (working memory).
//!   - **Homeostasis**: kila neuron husahihisha kizingiti chake kufikia target
//!     ya sparsity (kama synaptic scaling).
//!   - **Predictive coding**: tofauti (residual) kati ya pembejeo na ubashiri —
//!     cortex hutuma "error signal" tu, si signal kamili.
//!
//! Kanuni: hakuna `.unwrap()`/`panic!`/`..` kwenye code ya runtime.

use crate::snn::{SnnConfig, SpikeStats};

/// Tabaka la neurons lenye lateral inhibition + homeostasis.
pub struct CortexColumn {
    pub cfg: SnnConfig,
    /// Membrane potential kwa kila neuron.
    v: Vec<f32>,
    /// Trace ya recurrent excitation (working memory).
    recur: Vec<f32>,
    /// Kizingiti cha adaptive kwa kila neuron (homeostasis).
    th: Vec<f32>,
    /// Kiwango cha spike cha kila neuron (kwa homeostasis).
    rate: Vec<f32>,
    /// Uzito wa lateral inhibition (0 = hakuna; 1 = kamili).
    pub inhibition: f32,
    /// Uzito wa recurrent excitation (working memory).
    pub recurrence: f32,
    /// Kiwango cha kujifunza cha homeostasis.
    pub homeo_lr: f32,
}

impl CortexColumn {
    /// Tengeneza column ya `n` neurons.
    pub fn new(n: usize, cfg: SnnConfig) -> Self {
        CortexColumn {
            v: vec![0.0; n],
            recur: vec![0.0; n],
            th: vec![cfg.v_th; n],
            rate: vec![0.0; n],
            inhibition: 0.5,
            recurrence: 0.2,
            homeo_lr: 0.01,
            cfg,
        }
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }
    pub fn is_empty(&self) -> bool {
        self.v.is_empty()
    }

    /// Hatua moja: integrate (+recurrent) → leak → lateral inhibition →
    /// fire → reset → homeostasis.
    pub fn step(&mut self, x: &[f32], k_winners: usize) -> &[f32] {
        let beta = self.cfg.beta();
        let n = self.v.len().min(x.len());
        // 1) Integrate + leak + recurrent excitation.
        for i in 0..n {
            let u = beta * self.v[i] + x[i] + self.recurrence * self.recur[i];
            self.v[i] = u;
        }
        for i in n..self.v.len() {
            self.v[i] = beta * self.v[i];
        }
        // 2) Lateral inhibition: k-WTA kwa membrane potential (>0).
        let mut idx: Vec<usize> = (0..n).filter(|&i| self.v[i] > 0.0).collect();
        idx.sort_by(|&a, &b| {
            self.v[b]
                .partial_cmp(&self.v[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let k = if k_winners == 0 || k_winners > n { n } else { k_winners };
        let mut allowed = vec![false; n];
        for &i in idx.iter().take(k) {
            allowed[i] = true;
        }
        // 3) Fire kwa wale waliopita kizingiti cha adaptive NA k-WTA.
        let mut fired = 0u64;
        for i in 0..n {
            let t = self.th[i];
            if allowed[i] && self.v[i] >= t {
                self.v[i] -= t; // reset (subtract threshold)
                self.rate[i] = 1.0;
                self.recur[i] = (self.recur[i] + 1.0).min(1.0);
                fired += 1;
            } else {
                self.rate[i] = 0.0;
                self.recur[i] *= 0.9; // recurrent decay
                // Lateral inhibition: kandamiza wengine kwa uwiano wa inhibition.
                if !allowed[i] {
                    self.v[i] *= 1.0 - self.inhibition.clamp(0.0, 1.0);
                }
            }
            // 4) Homeostasis: sahihisha kizingiti kufikia sparsity target.
            let target = 1.0 - self.cfg.sparsity_target.clamp(0.0, 1.0);
            self.th[i] += self.homeo_lr * (self.rate[i] - target);
            self.th[i] = self.th[i].clamp(1e-3, 100.0);
        }
        let _ = fired;
        &self.rate
    }

    /// Endesha hatua `t` na kurudisha rate ya wastani (kama snn.rs `run`).
    pub fn run(&mut self, x: &[f32], t: usize, k_winners: usize, stats: &mut SpikeStats) -> &[f32] {
        let t = t.max(1);
        let mut acc = vec![0.0f32; self.rate.len()];
        for _ in 0..t {
            let r = self.step(x, k_winners);
            let n = acc.len().min(r.len());
            for i in 0..n {
                acc[i] += r[i];
            }
        }
        let inv = 1.0 / t as f32;
        let mut nspk = 0u64;
        for i in 0..acc.len() {
            self.rate[i] = acc[i] * inv;
            if self.rate[i] > 0.0 {
                nspk += 1;
            }
        }
        stats.neurons += self.rate.len() as u64;
        stats.spikes += nspk;
        &self.rate
    }

    pub fn reset(&mut self) {
        for v in self.v.iter_mut() {
            *v = 0.0;
        }
        for r in self.recur.iter_mut() {
            *r = 0.0;
        }
        for r in self.rate.iter_mut() {
            *r = 0.0;
        }
    }

    /// Kizingiti cha sasa (kwa ukaguzi/telemetry).
    pub fn thresholds(&self) -> &[f32] {
        &self.th
    }
}

/// Predictive coding: residual (error signal) kati ya input na prediction.
/// Cortical neurons hutuma tofauti hii, si signal kamili — hivyo kuokoa nishati.
pub fn predictive_residual(input: &[f32], prediction: &[f32]) -> Vec<f32> {
    let n = input.len().min(prediction.len());
    let mut r = vec![0.0f32; n];
    for i in 0..n {
        r[i] = input[i] - prediction[i];
    }
    r
}

/// Grid ya minicolumns (hypercolumns) — kila column hushughulikia kipande cha
/// pembejeo; hii hutegemeza tabia ya "kugawanya kazi" ya cortex.
pub struct CortexSheet {
    pub columns: Vec<CortexColumn>,
    /// Idadi ya neurons kwa column.
    pub col_size: usize,
}

impl CortexSheet {
    /// Tengeneza sheet ya `n_cols` columns, kila moja na `col_size` neurons.
    pub fn new(n_cols: usize, col_size: usize, cfg: SnnConfig) -> Self {
        let mut columns = Vec::with_capacity(n_cols);
        for _ in 0..n_cols {
            columns.push(CortexColumn::new(col_size, cfg.clone()));
        }
        CortexSheet { columns, col_size }
    }

    /// Endesha sheet: kila column hupokea kipande chake cha pembejeo.
    pub fn run(&mut self, x: &[f32], t: usize, stats: &mut SpikeStats) -> Vec<f32> {
        let n_cols = self.columns.len();
        if n_cols == 0 || self.col_size == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(n_cols * self.col_size);
        for (c, col) in self.columns.iter_mut().enumerate() {
            let start = c * self.col_size;
            let end = (start + self.col_size).min(x.len());
            if start >= x.len() {
                out.extend(std::iter::repeat(0.0f32).take(self.col_size));
                continue;
            }
            let slice = &x[start..end.max(start + 1).min(x.len())];
            let k = (self.col_size / 10).max(1);
            let r = col.run(slice, t, k, stats);
            out.extend_from_slice(r);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_hutoka_spikes_kwa_pembejeo_kubwa() {
        let mut c = CortexColumn::new(8, SnnConfig::default());
        let x = vec![2.0f32; 8];
        let out = c.step(&x, 4);
        let fired = out.iter().filter(|&&v| v > 0.0).count();
        assert!(fired > 0, "hakuna spike");
        assert!(fired <= 4, "k-WTA haikuzuia (fired={fired})");
    }

    #[test]
    fn kwta_inazuia_wingi_wa_spikes() {
        let mut c = CortexColumn::new(16, SnnConfig::default());
        let x = vec![3.0f32; 16];
        let out = c.step(&x, 4);
        let fired = out.iter().filter(|&&v| v > 0.0).count();
        assert!(fired <= 4, "k-WTA iliruhusu {fired}");
    }

    #[test]
    fn homeostasis_hupandisha_kizingiti() {
        let mut c = CortexColumn::new(4, SnnConfig::default());
        let before = c.thresholds().to_vec();
        for _ in 0..50 {
            c.step(&[5.0f32; 4], 4);
        }
        let after = c.thresholds();
        let sum_before: f32 = before.iter().sum();
        let sum_after: f32 = after.iter().sum();
        assert!(sum_after > sum_before, "kizingiti hakikupanda");
    }

    #[test]
    fn rate_ya_column_iko_ndani_ya_mbili() {
        let mut c = CortexColumn::new(8, SnnConfig::default());
        let mut stats = SpikeStats::default();
        let out = c.run(&[1.0f32; 8], 6, 3, &mut stats).to_vec();
        assert!(out.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }

    #[test]
    fn predictive_residual_ni_tofauti() {
        let r = predictive_residual(&[1.0, 2.0, 3.0], &[0.5, 2.0, 4.0]);
        assert_eq!(r, vec![0.5, 0.0, -1.0]);
    }

    #[test]
    fn sheet_huzalisha_ukubwa_sahihi() {
        let mut s = CortexSheet::new(3, 4, SnnConfig::default());
        let mut stats = SpikeStats::default();
        let out = s.run(&[1.0f32; 12], 4, &mut stats);
        assert_eq!(out.len(), 12);
    }
}
