//! snn.rs — SPIKING NEURAL NETWORK (SNN) — matumizi ya nishati NDOGO + speed.
//!
//! Huu ni mseto halisi wa SNN juu ya injini yetu ya transformer (`model.rs`).
//! Badala ya kuhesabu kwa namba endelevu (multiplications nyingi), neuron hutoka
//! **spikes** (0/1) na matumizi makubwa hufanyika kwa **addition pekee** (AC)
//! badala ya **multiply-accumulate** (MAC) — hii ndiyo "speed + ubora" ya SNN.
//!
//! Vipengele:
//!   - **Neuron ya LIF** (Leaky Integrate-and-Fire): integrate → leak → fire → reset.
//!   - **Surrogate gradient** (arctan) kwa mafunzo (spike haiwezi kutofautishwa).
//!   - **Encodings**: rate (Bernoulli), TTFS (time-to-first-spike), direct.
//!   - **Shiftmax**: badala ya softmax (hakuna `exp`) — huokoa nishati.
//!   - **Bipolar self-attention** (BSA): inaruhusu membrane potential ±.
//!   - **Spiking pooling attention**.
//!   - **A2OS2A / addition-only** matvec kwa spikes (XNOR/ternary, hakuna mult).
//!   - **STDP**: kanuni ya kujifunza ya kibaolojia (pre→post = potentiation).
//!   - **Energy accounting**: hesabu AC vs MAC + pJ (Horowitz, ISSCC 2014).
//!
//! Kanuni za MTECHOS: hakuna unwrap/panic/'..'/md5 kwenye code ya runtime.
//! Fail-safe: hucheshimu `Result` au kurejesha sufuri.

use crate::tensor::Tensor;

// ── Nishati kwa operesheni (picojoules, ~45nm, Horowitz ISSCC 2014) ──────────
/// Ongezeko la 32-bit (adder).
pub const ADD_ENERGY_PJ: f32 = 0.9;
/// Kuzidisha kwa 32-bit (multiplier).
pub const MULT_ENERGY_PJ: f32 = 3.7;
/// MAC = mult + add.
pub const MAC_ENERGY_PJ: f32 = MULT_ENERGY_PJ + ADD_ENERGY_PJ;
/// `exp` kwenye kitengo cha DSP (tunadhani mara ~2 ya mult; bado ni taarifa).
pub const EXP_ENERGY_PJ: f32 = 8.0;
/// Accumulate ya neuromorphic (event-driven, 8-bit AC) — nafuu sana.
pub const AC_NEURO_PJ: f32 = 0.1;

/// Config ya SNN.
#[derive(Clone, Debug)]
pub struct SnnConfig {
    /// Hatua za muda (time steps) kwa kila encoding/kuhesabu.
    pub t_steps: usize,
    /// Muda wa membrane (τ); beta = exp(-1/τ). Kubwa = "kumbukumbu" ndefu.
    pub tau_mem: f32,
    /// Kizingiti cha kutoka spike.
    pub v_th: f32,
    /// β ya Shiftmax (kiasi cha shift ya wastani).
    pub shiftmax_beta: f32,
    /// Target ya sparsity (dimu kwa kuripoti; [0,1)).
    pub sparsity_target: f32,
    /// Mbegu ya RNG (determinism kwa tests).
    pub seed: u64,
}

impl Default for SnnConfig {
    fn default() -> Self {
        SnnConfig {
            t_steps: 4,
            tau_mem: 2.0,
            v_th: 1.0,
            shiftmax_beta: 0.5,
            sparsity_target: 0.9,
            seed: 0x2545_F491_4F6C_DD1D,
        }
    }
}

impl SnnConfig {
    /// Sababu ya kuvuja (leak): u ← β·u. β = exp(-1/τ) ∈ (0,1).
    pub fn beta(&self) -> f32 {
        (-1.0f32 / self.tau_mem.max(1e-3)).exp()
    }
}

// ── Takwimu za spikes + nishati ──────────────────────────────────────────────

/// Takwimu za spikes na operesheni (AC/MAC) kwa hesabu ya nishati.
#[derive(Clone, Debug, Default)]
pub struct SpikeStats {
    pub spikes: u64,
    pub neurons: u64,
    /// Addition (accumulate) — operesheni nafuu (spike-driven).
    pub acs: u64,
    /// Multiply-accumulate — operesheni ghali zilizobaki kwenye SNN.
    pub macs: u64,
    /// MAC ambazo baseline DENSE ingefanya (kwa ulinganisho wa haki).
    pub dense_macs: u64,
    /// Hesabu za `exp` zilizookolewa (softmax → shiftmax).
    pub exps_saved: u64,
}

impl SpikeStats {
    pub fn activity(&self) -> f32 {
        if self.neurons == 0 {
            0.0
        } else {
            self.spikes as f32 / self.neurons as f32
        }
    }
    pub fn sparsity(&self) -> f32 {
        1.0 - self.activity()
    }
    pub fn absorb(&mut self, o: &SpikeStats) {
        self.spikes += o.spikes;
        self.neurons += o.neurons;
        self.acs += o.acs;
        self.macs += o.macs;
        self.dense_macs += o.dense_macs;
        self.exps_saved += o.exps_saved;
    }

    /// Nishati ya SNN (AC nafuu + MAC zilizobaki) kwa pJ.
    pub fn energy_snn_pj(&self) -> f32 {
        self.acs as f32 * AC_NEURO_PJ + self.macs as f32 * MAC_ENERGY_PJ
    }

    /// Nishati ya baseline DENSE (MAC zote + `exp` za softmax) kwa pJ.
    pub fn energy_dense_pj(&self) -> f32 {
        self.dense_macs as f32 * MAC_ENERGY_PJ + self.exps_saved as f32 * EXP_ENERGY_PJ
    }

    /// Kipimo cha uokoaji (mara ngapi SNN ni nafuu kuliko dense).
    pub fn savings_factor(&self) -> f32 {
        let d = self.energy_dense_pj();
        if d <= 0.0 {
            return 1.0;
        }
        d / self.energy_snn_pj().max(1e-9)
    }
}

// ── Neuron ya LIF ────────────────────────────────────────────────────────────

/// Tabaka la neurons za LIF (membrane potential + spikes).
pub struct LifLayer {
    pub cfg: SnnConfig,
    /// Membrane potential u kwa kila neuron.
    pub v: Vec<f32>,
    /// Spikes za hatua ya sasa (0.0 au 1.0).
    pub out: Vec<f32>,
    /// Ratiba ya spikes kwa hatua zote (rate coding output).
    rate: Vec<f32>,
}

impl LifLayer {
    pub fn new(n: usize, cfg: SnnConfig) -> Self {
        LifLayer {
            cfg,
            v: vec![0.0; n],
            out: vec![0.0; n],
            rate: vec![0.0; n],
        }
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }
    pub fn is_empty(&self) -> bool {
        self.v.is_empty()
    }

    /// Hatua MOJA: integrate → leak → fire → reset (subtract-threshold).
    /// u = β·u + x; spike kama u ≥ v_th; reset u -= v_th.
    pub fn step(&mut self, x: &[f32]) -> &[f32] {
        let beta = self.cfg.beta();
        let th = self.cfg.v_th;
        let n = self.v.len().min(x.len());
        for i in 0..n {
            let u = beta * self.v[i] + x[i];
            if u >= th {
                self.v[i] = u - th; // relative reset
                self.out[i] = 1.0;
            } else {
                self.v[i] = u;
                self.out[i] = 0.0;
            }
        }
        for o in self.out.iter_mut().skip(n) {
            *o = 0.0;
        }
        &self.out
    }

    /// Endesha hatua `t` kwa pembejeo ile ile, kurudisha **rate** (wastani wa spike).
    /// Hii ndiyo encoding/activation ya "rate": pato ∈ [0,1].
    pub fn run(&mut self, x: &[f32], t: usize, stats: &mut SpikeStats) -> &[f32] {
        let t = t.max(1);
        for r in self.rate.iter_mut() {
            *r = 0.0;
        }
        for _ in 0..t {
            self.step(x);
            let n = self.rate.len().min(self.out.len());
            for i in 0..n {
                self.rate[i] += self.out[i];
            }
        }
        let inv = 1.0 / t as f32;
        let mut nspk = 0u64;
        for r in self.rate.iter_mut() {
            *r *= inv;
            if *r > 0.0 {
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
        for o in self.out.iter_mut() {
            *o = 0.0;
        }
    }
}

// ── Surrogate gradient (kwa mafunzo / BPTT) ──────────────────────────────────

/// Surrogate gradient ya arctan: σ(x) = (1/π)·arctan(π/2·α·x) + ½,
/// derivative σ'(x) = (α/2) / (1 + (π/2·α·x)²). Hurudisha 0 kwa x isiyo na kikomo.
#[inline]
pub fn arctan_surrogate(x: f32, alpha: f32) -> f32 {
    let a = std::f32::consts::FRAC_PI_2 * alpha * x;
    (alpha / 2.0) / (1.0 + a * a)
}

/// Surrogate ya FastSigmoid: σ(x)=1/(1+α|x|); σ'(x)= (α/2)/(1+α|x|)² kwa x≠0.
#[inline]
pub fn fast_sigmoid_surrogate(x: f32, alpha: f32) -> f32 {
    let d = 1.0 + alpha * x.abs();
    (alpha / 2.0) / (d * d)
}

// ── Encodings ────────────────────────────────────────────────────────────────

/// Xorshift64 — deterministic RNG (bila crate ya nje).
#[inline]
pub fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Rate encoding: pembejeo ∈ [0,1] → spike (0/1) kwa hatua `t` kwa Bernoulli.
/// Hurudisha mpangilio [`t`][`n`]. Wastani wa spikes ≈ value.
pub fn rate_encode(x: &[f32], t: usize, seed: u64) -> Vec<Vec<f32>> {
    let t = t.max(1);
    let n = x.len();
    let mut out = vec![vec![0.0f32; n]; t];
    let mut st = seed | 1;
    for frame in out.iter_mut() {
        for i in 0..n {
            let p = x[i].clamp(0.0, 1.0);
            let r = (xorshift64(&mut st) >> 11) as f32 / (1u64 << 53) as f32;
            frame[i] = if r < p { 1.0 } else { 0.0 };
        }
    }
    out
}

/// TTFS (time-to-first-spike): value kubwa → spike MAPEMA.
/// Hurudisha wakati wa spike kwa kila neuron (0 = mapema zaidi), au `None` (t)
/// kama neuron haiweki spike (value ≤ 0).
pub fn ttfs_encode(x: &[f32], t: usize) -> Vec<Option<usize>> {
    let t = t.max(1);
    x.iter()
        .map(|&v| {
            let p = v.clamp(0.0, 1.0);
            if p <= 0.0 {
                None
            } else {
                // p=1 → 0 (mapema), p=0 → t-1 (chelewa).
                let step = ((1.0 - p) * (t - 1) as f32).round() as usize;
                Some(step.min(t - 1))
            }
        })
        .collect()
}

// ── Shiftmax (badala ya softmax; huokoa exp) ─────────────────────────────────

/// Shiftmax: z = max(0, x - β·mean(x)); out = z / Σz. Hakuna `exp`.
/// Hurudisha idadi ya `exp` zilizoookolewa (moja kwa kila kipengele) + AC/MAC.
pub fn shiftmax_inplace(x: &mut [f32], beta: f32, stats: &mut SpikeStats) {
    let n = x.len();
    if n == 0 {
        return;
    }
    let mean = x.iter().sum::<f32>() / n as f32;
    let shift = beta * mean;
    let mut sum = 0.0f32;
    for v in x.iter_mut() {
        let z = (*v - shift).max(0.0);
        *v = z;
        sum += z;
    }
    if sum > 1e-12 {
        for v in x.iter_mut() {
            *v /= sum;
        }
    } else {
        // Fail-safe: mgawanyo sawa (spikes zote sifuri).
        let u = 1.0 / n as f32;
        for v in x.iter_mut() {
            *v = u;
        }
    }
    // Shiftmax huhifadhi `exp` ya kila kipengele + mgawanyo.
    stats.exps_saved += n as u64;
    stats.acs += (2 * n) as u64;
}

// ── Attention ya spiking ─────────────────────────────────────────────────────

/// Bipolar attention output kwa head moja: out = Shiftmax((Q·K)·scale) · V.
///
/// "Bipolar" = scores zinaweza kuwa ± ; Shiftmax huzibadilisha kwa `max(0, x-βμ)`,
/// hivyo vichocheo hasi hukandamizwa (bila `exp`/softmax).
pub fn attention_out(
    q: &[f32],
    k: &[f32],
    v: &[f32],
    npos: usize,
    d: usize,
    scale: f32,
    beta: f32,
    stats: &mut SpikeStats,
) -> Vec<f32> {
    let mut scores = vec![0.0f32; npos];
    for t in 0..npos {
        let kt = &k[t * d..t * d + d];
        let mut acc = 0.0f32;
        for i in 0..d {
            acc += q[i] * kt[i];
        }
        scores[t] = acc * scale;
        stats.macs += d as u64;
        stats.dense_macs += d as u64;
    }
    shiftmax_inplace(&mut scores, beta, stats);
    let mut out = vec![0.0f32; d];
    for t in 0..npos {
        let vt = &v[t * d..t * d + d];
        for i in 0..d {
            out[i] += scores[t] * vt[i];
            stats.macs += 1;
            stats.dense_macs += 1;
        }
    }
    out
}

/// Spiking pooling attention: max-pool spikes kwa dirisha `pool` kwa muda.
/// `spikes` = [t][n] (rate/time-major). Hurudisha [n/pool] baada ya max-pool.
pub fn spiking_pool(spikes: &[Vec<f32>], pool: usize) -> Vec<f32> {
    if spikes.is_empty() {
        return Vec::new();
    }
    let n = spikes[0].len();
    let pool = pool.max(1);
    let nout = n.div_ceil(pool);
    let mut out = vec![0.0f32; nout];
    for (o, slot) in out.iter_mut().enumerate() {
        let lo = o * pool;
        let hi = (lo + pool).min(n);
        let mut m = 0.0f32;
        for frame in spikes {
            for i in lo..hi {
                if i < frame.len() && frame[i] > m {
                    m = frame[i];
                }
            }
        }
        *slot = m;
    }
    out
}

// ── A2OS2A / addition-only matvec ────────────────────────────────────────────

/// Multiplication ya binary kwa XNOR + popcount (0 multiplications).
/// `a`, `b` ni bitmask za 64-bit; `bits` = upana wa bits unaozingatiwa (≤64).
/// Hurudisha idadi ya bits zinazolingana kwenye upana huo.
#[inline]
pub fn xnor_popcount(a: u64, b: u64, bits: u32) -> u32 {
    let bits = bits.min(64);
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    (!(a ^ b) & mask).count_ones()
}

/// Addition-only matvec: y = W·s, ambapo `s` ni spikes (0/1).
/// Badala ya kuzidisha, tunaongeza safu ya W pale spike ipo (`s[i] > 0.5`).
/// Hii ni **AC pekee** (hakuna MAC). `stats.acs` huongezwa kwa #spikes kwa row;
/// `stats.dense_macs` huongezwa n_in kwa row (baseline dense yenye MAC zote).
pub fn addition_only_matvec(
    w: &Tensor<'_>,
    spikes: &[f32],
    y: &mut [f32],
    scratch: &mut [f32],
    stats: &mut SpikeStats,
) {
    let n_in = w.n_in().min(spikes.len());
    for (o, yo) in y.iter_mut().enumerate().take(w.n_out()) {
        w.dequant_row(o, &mut scratch[..w.n_in()]);
        let mut acc = 0.0f32;
        let mut used = 0u64;
        for i in 0..n_in {
            if spikes[i] > 0.5 {
                acc += scratch[i];
                used += 1;
            }
        }
        *yo = acc;
        stats.acs += used;
        stats.dense_macs += w.n_in() as u64; // dense baseline: MAC kwa kila kiungo
    }
}

/// Ternary matvec (A2OS2A): uzito ∈ {-1,0,+1}. `s` binary spikes.
/// y[o] = Σ_i w[o,i]·s[i] — huhesabiwa kwa addition/subtraction pekee.
/// `w` hupangwa row-major [n_out][n_in] kama i8.
pub fn ternary_matvec(w: &[i8], n_out: usize, n_in: usize, s: &[f32], y: &mut [f32], stats: &mut SpikeStats) {
    for o in 0..n_out {
        let base = o * n_in;
        let mut acc = 0.0f32;
        for i in 0..n_in.min(s.len()) {
            if s[i] > 0.5 {
                match w[base + i] {
                    1 => {
                        acc += 1.0;
                        stats.acs += 1;
                    }
                    -1 => {
                        acc -= 1.0;
                        stats.acs += 1;
                    }
                    _ => {}
                }
            }
        }
        y[o] = acc;
    }
}

// ── STDP ─────────────────────────────────────────────────────────────────────

/// STDP: pre→post (dt>0) = potentiation; post→pre (dt<0) = depression.
/// Hurudisha mabadiliko ya uzito Δw.
#[inline]
pub fn stdp_delta(dt_steps: i32, a_plus: f32, a_minus: f32, tau: f32) -> f32 {
    let tau = tau.max(1e-3);
    if dt_steps > 0 {
        a_plus * (-(dt_steps as f32) / tau).exp()
    } else if dt_steps < 0 {
        -a_minus * ((dt_steps as f32) / tau).exp()
    } else {
        0.0
    }
}

// ── Muhtasari wa nishati (kwa API) ───────────────────────────────────────────

/// Muhtasari wa nishati kwa UI/API.
pub fn energy_json(stats: &SpikeStats) -> serde_json::Value {
    serde_json::json!({
        "spikes": stats.spikes,
        "neurons": stats.neurons,
        "activity": stats.activity(),
        "sparsity": stats.sparsity(),
        "acs": stats.acs,
        "macs": stats.macs,
        "exps_saved": stats.exps_saved,
        "energy_snn_pj": stats.energy_snn_pj(),
        "energy_dense_pj": stats.energy_dense_pj(),
        "savings_factor": stats.savings_factor(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lif_hutoka_spike_kwa_kizingiti() {
        let cfg = SnnConfig {
            tau_mem: 1e9, // β ≈ 1 (bila kuvuja) kwa test
            v_th: 1.0,
            ..Default::default()
        };
        let mut lif = LifLayer::new(3, cfg);
        let sp = lif.step(&[0.5, 1.0, 2.0]).to_vec();
        // neuron 0: 0.5 < 1 → 0; neuron 1: 1.0 ≥ 1 → 1; neuron 2: 2.0 ≥ 1 → 1
        assert_eq!(sp, vec![0.0, 1.0, 1.0]);
        // Reset: neuron 2 alikuwa 2.0 → 1.0 (relative reset).
        assert!((lif.v[2] - 1.0).abs() < 1e-4, "v={}", lif.v[2]);
    }

    #[test]
    fn lif_rate_iko_kati_0_na_1() {
        let mut lif = LifLayer::new(4, SnnConfig::default());
        let mut st = SpikeStats::default();
        let r = lif.run(&[0.9, 0.5, 0.1, 0.0], 8, &mut st);
        for v in r {
            assert!((0.0..=1.0).contains(v), "rate={v}");
        }
        assert!(r[0] >= r[2], "rate ya pembejeo kubwa iwe kubwa");
        assert!(st.spikes > 0);
        assert!(st.neurons > 0);
    }

    #[test]
    fn shiftmax_hujumlisha_moja_na_haifanyi_exp() {
        let mut x = vec![1.0f32, 2.0, 3.0, -5.0];
        let mut st = SpikeStats::default();
        shiftmax_inplace(&mut x, 0.5, &mut st);
        let s: f32 = x.iter().sum();
        assert!((s - 1.0).abs() < 1e-5, "sum={s}");
        assert!(x.iter().all(|v| *v >= 0.0));
        assert!(x[3] == 0.0, "thamani hasi ikandamizwe: {}", x[3]);
        assert_eq!(st.exps_saved, 4);
    }

    #[test]
    fn surrogate_gradient_ina_umbo_la_kengele() {
        // Kilele kwa x=0; hupungua kwa |x| kubwa.
        let g0 = arctan_surrogate(0.0, 2.0);
        let g1 = arctan_surrogate(1.0, 2.0);
        let g10 = arctan_surrogate(10.0, 2.0);
        assert!(g0 > g1 && g1 > g10);
        assert!(g0 > 0.0 && g10 >= 0.0);
        // FastSigmoid pia ina kilele kwa 0.
        assert!(fast_sigmoid_surrogate(0.0, 2.0) > fast_sigmoid_surrogate(2.0, 2.0));
    }

    #[test]
    fn rate_encode_wastani_unaokaribia_value() {
        let x = [1.0f32, 0.0, 0.5, 0.5];
        let frames = rate_encode(&x, 200, 7);
        assert_eq!(frames.len(), 200);
        let n = x.len();
        let mut mean = vec![0.0f32; n];
        for f in &frames {
            for i in 0..n {
                mean[i] += f[i];
            }
        }
        for m in mean.iter_mut() {
            *m /= 200.0;
        }
        assert!((mean[0] - 1.0).abs() < 1e-6, "{}", mean[0]);
        assert!((mean[1] - 0.0).abs() < 1e-6);
        assert!((mean[2] - 0.5).abs() < 0.1, "{}", mean[2]);
    }

    #[test]
    fn ttfs_value_kubwa_hutoka_mapema() {
        let s = ttfs_encode(&[1.0, 0.0, 0.5], 10);
        assert_eq!(s[0], Some(0), "value 1 → mapema zaidi");
        assert_eq!(s[1], None, "value 0 → haitoi spike");
        assert!(s[2].is_some_and(|v| v > 0));
    }

    #[test]
    fn attention_out_hujumlisha_uzito_moja() {
        // npos=2, d=2; Q·K itoe scores; Shiftmax → weights; × V.
        let q = [1.0f32, 0.0];
        let k = [1.0f32, 0.0, 0.0, 1.0];
        let v = [10.0f32, 0.0, 0.0, 20.0];
        let mut st = SpikeStats::default();
        let out = attention_out(&q, &k, &v, 2, 2, 1.0, 0.0, &mut st);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|x| x.is_finite()));
        // score[0] = q·k0 = 1.0 > score[1] = q·k1 = 0.0 → out inaelekea v0.
        assert!(out[0] >= out[1], "out={out:?}");
        assert!(st.macs > 0 && st.exps_saved > 0);
    }

    #[test]
    fn spiking_pool_huchukua_max() {
        let frames = vec![vec![0.0f32, 1.0, 0.0, 0.0], vec![1.0, 0.0, 0.0, 1.0]];
        let p = spiking_pool(&frames, 2);
        assert_eq!(p, vec![1.0, 1.0]);
    }

    #[test]
    fn addition_only_ni_nafuu_kuliko_mac() {
        // Tensor ndogo ya 1-D badala ya full: tunajaribu ternary_matvec.
        let w: Vec<i8> = vec![1, -1, 0, 1];
        let s = [1.0f32, 1.0, 1.0, 0.0]; // spikes
        let mut y = vec![0.0f32; 2];
        let mut st = SpikeStats::default();
        ternary_matvec(&w, 2, 2, &s, &mut y, &mut st);
        assert_eq!(y, vec![0.0, 1.0]);
        assert!(st.acs > 0);
        assert_eq!(st.macs, 0, "addition-only haifanyi MAC");
    }

    #[test]
    fn xnor_popcount_inalingana() {
        // 0b1010 vs 0b1000 → bits 3 za chini: 1,2,3 sawa; bit0 tofauti → 3.
        assert_eq!(xnor_popcount(0b1010, 0b1000, 4), 3);
        assert_eq!(xnor_popcount(0xFFFF, 0xFFFF, 16), 16);
        // Upana mdogo hupuuza bits za juu.
        assert_eq!(xnor_popcount(0b1100, 0b0000, 2), 2);
    }

    #[test]
    fn stdp_potentiation_na_depression() {
        // pre→post (dt>0) chanya; post→pre (dt<0) hasi; dt=0 = 0.
        assert!(stdp_delta(2, 0.1, 0.12, 2.0) > 0.0);
        assert!(stdp_delta(-2, 0.1, 0.12, 2.0) < 0.0);
        assert_eq!(stdp_delta(0, 0.1, 0.12, 2.0), 0.0);
    }

    #[test]
    fn nishati_ya_snn_ni_ndogo_kuliko_dense() {
        let st = SpikeStats {
            spikes: 10,
            neurons: 100,
            acs: 1000,
            macs: 100,
            dense_macs: 100000,
            exps_saved: 500,
        };
        assert!(st.sparsity() > 0.85);
        assert!(
            st.savings_factor() > 1.0,
            "savings={} (snn={} dense={})",
            st.savings_factor(),
            st.energy_snn_pj(),
            st.energy_dense_pj()
        );
    }

    #[test]
    fn config_beta_iko_kati_0_na_1() {
        for tau in [0.5f32, 1.0, 2.0, 10.0] {
            let b = SnnConfig {
                tau_mem: tau,
                ..Default::default()
            }
            .beta();
            assert!(b > 0.0 && b < 1.0, "tau={tau} beta={b}");
        }
    }
}

