//! diarize.rs — KUGUNDUA WAZUNGUMZAJI (speaker diarization) HALISI, bila GPU.
//!
//! Pyannote/NeMo zinahitaji GPU na modeli za ~1GB. Njia hii ni ya ndani kabisa:
//! hutumia **sifa za akustika** (acoustic features) kwa kila fremu, kisha
//! **k-means** kuweka fremu katika makundi ya wazungumzaji. Hii ni njia ya
//! kawaida ya "unsupervised speaker clustering" — inafanya kazi kwa Kiswahili
//! bila training yoyote.
//!
//! Sifa zinazotumika (zote halisi, kutoka FFT):
//!   - **RMS** (nguvu ya sauti) — nani anazungumza kwa nguvu
//!   - **ZCR** (zero-crossing rate) — sauti za "s" vs "a" (fricative vs vowel)
//!   - **spectral centroid** — "mwangaza" wa sauti (pitch-related)
//!   - **spectral rolloff (85%)** — mgawanyo wa nishati
//!   - **spectral flux** — kasi ya mabadiliko
//!   - **band energies** (4 bendi) — umbo la spektra (voiceprint rahisi)
//!
//! Kwa mtu mmoja: sifa zake zinaunda kundi moja. Watu wawili: makundi mawili.
//! Kisha tunarudisha zamu: "Mzungumzaji 1: 0.0s–4.2s", "Mzungumzaji 2: …".

use crate::scribe::{fft_inplace, hann, rms};

/// Vipimo vya vekta ya sifa (MFCC 8 + bendi 4 + RMS/ZCR/centroid/rolloff 4 + F0).
pub const DIM: usize = 17;
/// Idadi ya vichujio vya mel.
const N_MELS: usize = 26;
/// Idadi ya mgawo wa MFCC.
const N_MFCC: usize = 8;

/// Sifa za fremu moja.
#[derive(Clone, Debug)]
pub struct FrameFeat {
    pub t_start: f64,
    pub t_end: f64,
    pub rms: f64,
    pub zcr: f64,
    pub centroid: f64,
    pub rolloff: f64,
    pub flux: f64,
    pub bands: [f64; 4],
    /// MFCC 0–7 — hutambua **mzungumzaji** (umbo la mfereji wa sauti), si
    /// maudhui ya maneno. Hii ndiyo sifa kuu ya diarization.
    pub mfcc: [f64; N_MFCC],
    /// Pitch (F0) kwa Hz — 0.0 kama haipatikani. Tofauti ya F0 ndiyo kipimo
    /// chenye nguvu zaidi cha kutofautisha wazungumzaji (mf. mwanaume/mwanamke).
    pub f0: f64,
}

impl FrameFeat {
    /// Vekta ya sifa kwa clustering. MFCC huchukua nafasi kuu (utambulisho wa
    /// mzungumzaji); bendi + sifa za muda ni nyongeza.
    fn vector(&self) -> [f64; DIM] {
        let mut v = [0.0f64; DIM];
        v[0..N_MFCC].copy_from_slice(&self.mfcc);
        v[8] = self.bands[0];
        v[9] = self.bands[1];
        v[10] = self.bands[2];
        v[11] = self.bands[3];
        v[12] = self.rms * 10.0;
        v[13] = self.zcr;
        v[14] = self.centroid / 1000.0;
        v[15] = self.rolloff / 1000.0;
        v[16] = self.f0 / 100.0;
        v
    }
}

fn hz_to_mel(hz: f64) -> f64 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(m: f64) -> f64 {
    700.0 * (10f64.powf(m / 2595.0) - 1.0)
}

/// Hukadiria pitch (F0) kwa autocorrelation yenye kizingiti cha unadhifu.
/// Hurudisha Hz (0.0 kama fremu haionyeshi mwangwi wazi). Hii hutumika
/// kutofautisha wazungumzaji (F0 ya mwanaume ≈ 85–180Hz, mwanamke ≈ 165–255Hz).
fn estimate_f0(frame: &[f32], sample_rate: u32) -> f64 {
    let n = frame.len();
    if n < 64 {
        return 0.0;
    }
    let fmin = 60.0f64;
    let fmax = 400.0f64;
    let min_lag = (sample_rate as f64 / fmax).floor() as usize;
    let max_lag = (sample_rate as f64 / fmin).ceil() as usize;
    if max_lag >= n {
        return 0.0;
    }
    // Ondoa wastani (DC).
    let mean: f64 = frame.iter().map(|s| *s as f64).sum::<f64>() / n as f64;
    let x: Vec<f64> = frame.iter().map(|s| *s as f64 - mean).collect();
    let energy: f64 = x.iter().map(|v| v * v).sum::<f64>();
    if energy < 1e-6 {
        return 0.0;
    }
    // Autocorrelation (normalized) kwa lags zinazowezekana.
    let mut best_lag = 0usize;
    let mut best_corr = 0.0f64;
    let mut corr = vec![0.0f64; max_lag + 1];
    for lag in min_lag..=max_lag {
        let mut s = 0.0f64;
        for i in 0..(n - lag) {
            s += x[i] * x[i + lag];
        }
        let c = s / energy;
        corr[lag] = c;
        if c > best_corr {
            best_corr = c;
            best_lag = lag;
        }
    }
    // Unadhifu: kama kilele ni dhaifu, hakuna F0 ya kuaminika.
    if best_corr < 0.35 || best_lag == 0 {
        return 0.0;
    }
    // Uboreshaji wa parabolic (sub-sample) kwa F0 laini zaidi.
    let (mut lag_f) = best_lag as f64;
    if best_lag > min_lag && best_lag < max_lag {
        let a = corr[best_lag - 1];
        let b = corr[best_lag];
        let c = corr[best_lag + 1];
        let denom = a - 2.0 * b + c;
        if denom.abs() > 1e-12 {
            lag_f += 0.5 * (a - c) / denom;
        }
    }
    sample_rate as f64 / lag_f
}

/// Kichujio cha mel (triangular) kwa bins za FFT. Hurudisha [n_mels][n_bins].
fn mel_filterbank(n_mels: usize, n_bins: usize, n_fft: usize, sample_rate: u32) -> Vec<Vec<f64>> {
    let fmin = 80.0;
    let fmax = sample_rate as f64 / 2.0;
    let (mmin, mmax) = (hz_to_mel(fmin), hz_to_mel(fmax));
    let points: Vec<f64> = (0..n_mels + 2)
        .map(|i| mel_to_hz(mmin + (mmax - mmin) * i as f64 / (n_mels + 1) as f64))
        .collect();
    let mut fb = vec![vec![0.0f64; n_bins]; n_mels];
    for (b, row) in fb.iter_mut().enumerate() {
        let (lo, c, hi) = (points[b], points[b + 1], points[b + 2]);
        for (i, w) in row.iter_mut().enumerate() {
            let f = i as f64 * sample_rate as f64 / n_fft as f64;
            *w = if f >= lo && f <= c {
                if c > lo { (f - lo) / (c - lo) } else { 0.0 }
            } else if f > c && f <= hi {
                if hi > c { (hi - f) / (hi - c) } else { 0.0 }
            } else {
                0.0
            };
        }
    }
    fb
}

/// DCT-II ya orthonormal — hubadilisha nishati za mel (log) kuwa MFCC.
fn dct(log_mel: &[f64], n_out: usize) -> Vec<f64> {
    let m = log_mel.len();
    (0..n_out)
        .map(|k| {
            let s: f64 = (0..m)
                .map(|n| log_mel[n] * (std::f64::consts::PI * k as f64 * (n as f64 + 0.5) / m as f64).cos())
                .sum();
            let norm = if k == 0 {
                (1.0 / m as f64).sqrt()
            } else {
                (2.0 / m as f64).sqrt()
            };
            s * norm
        })
        .collect()
}

/// Zamu ya mzungumzaji mmoja.
#[derive(Clone, Debug)]
pub struct Turn {
    pub speaker: usize,
    pub t_start: f64,
    pub t_end: f64,
    pub mean_rms: f64,
}

/// Hutoa sifa kwa kila fremu ya sauti. `n_fft` = 512 (32ms @ 16kHz).
pub fn frame_features(samples: &[f32], sample_rate: u32, n_fft: usize, hop: usize) -> Vec<FrameFeat> {
    if samples.len() < n_fft || n_fft < 2 {
        return Vec::new();
    }
    let window = hann(n_fft);
    let half = n_fft / 2;
    let fb = mel_filterbank(N_MELS, half, n_fft, sample_rate);
    // Pre-emphasis (0.97) — huongeza nishati ya juu, muhimu kwa MFCC.
    let preemph = |i: usize| -> f64 {
        let cur = samples[i] as f64;
        if i == 0 {
            cur
        } else {
            cur - 0.97 * samples[i - 1] as f64
        }
    };
    let mut out: Vec<FrameFeat> = Vec::new();
    let mut prev_spec: Vec<f64> = Vec::new();
    let mut pos = 0usize;

    while pos + n_fft <= samples.len() {
        let frame = &samples[pos..pos + n_fft];

        // RMS + ZCR kwenye uwanja wa muda.
        let energy = rms(frame);
        let mut zc = 0usize;
        for w in frame.windows(2) {
            if (w[0] >= 0.0) != (w[1] >= 0.0) {
                zc += 1;
            }
        }
        let zcr = zc as f64 / n_fft as f64;

        // FFT (radix-2, halisi) → spektra ya ukubwa (kwa pre-emphasis + window).
        let mut re: Vec<f64> = (0..n_fft).map(|i| preemph(pos + i) * window[i]).collect();
        let mut im = vec![0.0f64; n_fft];
        fft_inplace(&mut re, &mut im);
        let mag: Vec<f64> = (0..half)
            .map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt())
            .collect();
        let mag_sum: f64 = mag.iter().sum::<f64>().max(1e-12);

        // Spectral centroid.
        let centroid = (0..half)
            .map(|i| {
                let f = i as f64 * sample_rate as f64 / n_fft as f64;
                f * mag[i]
            })
            .sum::<f64>()
            / mag_sum;

        // Spectral rolloff (85% ya nishati).
        let target = mag_sum * 0.85;
        let mut acc = 0.0;
        let mut rolloff = 0.0;
        for i in 0..half {
            acc += mag[i];
            if acc >= target {
                rolloff = i as f64 * sample_rate as f64 / n_fft as f64;
                break;
            }
        }

        // Spectral flux (mabadiliko kati ya fremu).
        let flux = if prev_spec.len() == half {
            let d: f64 = (0..half)
                .map(|i| {
                    let x = mag[i] - prev_spec[i];
                    if x > 0.0 { x * x } else { 0.0 }
                })
                .sum();
            (d / half as f64).sqrt()
        } else {
            0.0
        };
        prev_spec = mag.clone();

        // Nishati kwa bendi 4: 0–500, 500–1k, 1k–2k, 2k–4k+ Hz.
        let edges = [500.0, 1000.0, 2000.0, 4000.0];
        let mut bands = [0.0f64; 4];
        for i in 0..half {
            let f = i as f64 * sample_rate as f64 / n_fft as f64;
            let b = edges.iter().position(|e| f < *e).unwrap_or(3);
            bands[b] += mag[i];
        }
        for b in bands.iter_mut() {
            *b /= mag_sum;
        }

        // MFCC: nishati ya mel (log) → DCT. Hii hutambua mzungumzaji.
        let mut log_mel = vec![0.0f64; N_MELS];
        for (m, row) in fb.iter().enumerate() {
            let e: f64 = row.iter().zip(mag.iter()).map(|(w, x)| w * x).sum();
            log_mel[m] = (e.max(1e-10)).ln();
        }
        let ceps = dct(&log_mel, N_MFCC);
        let mut mfcc = [0.0f64; N_MFCC];
        mfcc.copy_from_slice(&ceps);

        // Pitch (F0) — kipimo chenye nguvu zaidi cha kutofautisha wazungumzaji.
        let f0 = estimate_f0(frame, sample_rate);

        out.push(FrameFeat {
            t_start: pos as f64 / sample_rate as f64,
            t_end: (pos + n_fft) as f64 / sample_rate as f64,
            rms: energy,
            zcr,
            centroid,
            rolloff,
            flux,
            bands,
            mfcc,
            f0,
        });
        pos += hop;
    }
    out
}

/// Cepstral Mean Normalization — huondoa mfereji (channel) na kelele ya
/// kudumu. Hii ni LAZIMA kwa diarization: bila CMN, sauti moja inaweza
/// kugawanyika kwa sababu ya tofauti za kelele/umbali.
fn cmn(data: &mut [[f64; DIM]]) {
    let n = data.len();
    if n == 0 {
        return;
    }
    let mut mean = [0.0f64; DIM];
    for d in data.iter() {
        for j in 0..DIM {
            mean[j] += d[j];
        }
    }
    for m in mean.iter_mut() {
        *m /= n as f64;
    }
    for d in data.iter_mut() {
        for j in 0..DIM {
            d[j] -= mean[j];
        }
    }
}

fn dist2(a: &[f64; DIM], b: &[f64; DIM]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum()
}

/// Z-score kwa kila kipimo (dimension). Bila hii, kipimo chenye kiwango kikubwa
/// (mf. centroid yenye maelfu) kingeifunika ZCR (0–1) — clustering ingekuwa
/// ya uongo. Hurudisha data iliyosawazishwa + (mean, std) kwa kila kipimo.
fn standardize(data: &[[f64; DIM]]) -> Vec<[f64; DIM]> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }
    let mut mean = [0.0f64; DIM];
    for d in data {
        for j in 0..DIM {
            mean[j] += d[j];
        }
    }
    for m in mean.iter_mut() {
        *m /= n as f64;
    }
    let mut var = [0.0f64; DIM];
    for d in data {
        for j in 0..DIM {
            var[j] += (d[j] - mean[j]) * (d[j] - mean[j]);
        }
    }
    let std: [f64; DIM] = std::array::from_fn(|j| (var[j] / n as f64).sqrt().max(1e-9));
    data.iter()
        .map(|d| std::array::from_fn(|j| (d[j] - mean[j]) / std[j]))
        .collect()
}

/// LCG rahisi kwa seeding inayoweza kurudiwa (deterministic tests).
fn lcg(seed: &mut u64) -> usize {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as usize
}

/// K-means mmoja kwa kianzio fulani. Hurudisha (labels, centroids, inertia).
fn kmeans_once(data: &[[f64; DIM]], k: usize, iters: usize, seed0: u64) -> (Vec<usize>, Vec<[f64; DIM]>, f64) {
    let n = data.len();
    let mut seed = seed0;
    // Kianzio cha bahati (random distinct points) — rahisi na thabiti zaidi ya
    // k-means++ yenye outliers (flux ya fremu za mpito).
    let mut centroids: Vec<[f64; DIM]> = Vec::with_capacity(k);
    let mut guard = 0;
    while centroids.len() < k && guard < n * 4 {
        let idx = lcg(&mut seed) % n;
        if !centroids.iter().any(|c| c == &data[idx]) {
            centroids.push(data[idx]);
        }
        guard += 1;
    }
    while centroids.len() < k {
        centroids.push(data[centroids.len() % n]);
    }

    let mut labels = vec![0usize; n];
    for _ in 0..iters {
        let mut changed = false;
        for (i, d) in data.iter().enumerate() {
            let mut bi = 0;
            let mut bd = f64::MAX;
            for (ci, c) in centroids.iter().enumerate() {
                let dd = dist2(d, c);
                if dd < bd {
                    bd = dd;
                    bi = ci;
                }
            }
            if labels[i] != bi {
                labels[i] = bi;
                changed = true;
            }
        }
        let mut sums = vec![[0.0f64; DIM]; k];
        let mut counts = vec![0usize; k];
        for (i, d) in data.iter().enumerate() {
            let l = labels[i];
            counts[l] += 1;
            for j in 0..DIM {
                sums[l][j] += d[j];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..DIM {
                    centroids[c][j] = sums[c][j] / counts[c] as f64;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let inertia: f64 = data
        .iter()
        .enumerate()
        .map(|(i, d)| dist2(d, &centroids[labels[i]]))
        .sum();
    (labels, centroids, inertia)
}

/// K-means imara: standardize + restarts N, chagua inertia ndogo kabisa.
/// Hurudisha (labels, centroids, inertia) kwenye nafasi iliyosawazishwa.
fn kmeans(data: &[[f64; DIM]], k: usize, iters: usize) -> (Vec<usize>, Vec<[f64; DIM]>) {
    let n = data.len();
    if n == 0 || k == 0 {
        return (vec![0; n], vec![]);
    }
    let k = k.min(n);
    let std_data = standardize(data);
    let mut best: Option<(Vec<usize>, Vec<[f64; DIM]>, f64)> = None;
    for r in 0..12u64 {
        let (l, c, inertia) = kmeans_once(&std_data, k, iters, 7919 + r * 104729);
        if best.as_ref().map(|(_, _, bi)| inertia < *bi).unwrap_or(true) {
            best = Some((l, c, inertia));
        }
    }
    let (labels, centroids, _) = best.unwrap();
    (labels, centroids)
}

/// Silhouette score halisi (Rousseeuw 1987): hupima jinsi kila nukta inavyofaa
/// kundi lake kuliko kundi la jirani. Hii ndiyo njia ya kawaida ya kuchagua k.
/// Hupimwa kwenye nafasi iliyosawazishwa (standardized).
fn silhouette(data: &[[f64; DIM]], labels: &[usize], k: usize) -> f64 {
    let n = data.len();
    if k < 2 || n < 2 {
        return 0.0;
    }
    let mut total = 0.0f64;
    let mut counted = 0usize;
    for i in 0..n {
        let li = labels[i];
        let mut sum_same = 0.0f64;
        let mut cnt_same = 0usize;
        let mut sum_other = vec![0.0f64; k];
        let mut cnt_other = vec![0usize; k];
        for j in 0..n {
            if i == j {
                continue;
            }
            let d = dist2(&data[i], &data[j]).sqrt();
            let lj = labels[j];
            if lj == li {
                sum_same += d;
                cnt_same += 1;
            } else {
                sum_other[lj] += d;
                cnt_other[lj] += 1;
            }
        }
        let a = if cnt_same > 0 { sum_same / cnt_same as f64 } else { 0.0 };
        let b = (0..k)
            .filter(|c| *c != li && cnt_other[*c] > 0)
            .map(|c| sum_other[c] / cnt_other[c] as f64)
            .fold(f64::MAX, f64::min);
        if b == f64::MAX || (a == 0.0 && b == 0.0) {
            continue;
        }
        total += (b - a) / a.max(b);
        counted += 1;
    }
    if counted == 0 {
        0.0
    } else {
        total / counted as f64
    }
}

/// Inertia bora (ndogo) kwa k, kwenye nafasi iliyosawazishwa.
fn best_inertia(std_data: &[[f64; DIM]], k: usize, iters: usize, restarts: u64) -> f64 {
    let mut best = f64::MAX;
    for r in 0..restarts {
        let (_, _, i) = kmeans_once(std_data, k, iters, 7919 + r * 104729);
        if i < best {
            best = i;
        }
    }
    best.max(1e-12)
}

/// **Gap statistic** (Tibshirani, Walther & Hastie 2001) — njia halisi ya
/// kuchagua k. Hulinganisha kupungua kwa inertia kwenye data YETU na kupungua
/// kunakotarajiwa kwenye data ya NASIBU (bila muundo). Hii huzuia "kila data
/// inagawanyika" — tatizo la silhouette pekee.
///
/// Huchagua k ndogo ambapo Gap(k) >= Gap(k+1) - s_{k+1}.
fn gap_statistic(data: &[[f64; DIM]], kmax: usize, b_ref: usize) -> usize {
    let n = data.len();
    if n < 8 {
        return 1;
    }
    let kmax = kmax.clamp(1, 6).min(n / 4).max(1);
    if kmax < 2 {
        return 1;
    }
    let std_data = standardize(data);

    // Kisanduku cha data (bounding box) kwa data ya nasibu.
    let mut lo = [f64::MAX; DIM];
    let mut hi = [f64::MIN; DIM];
    for d in &std_data {
        for j in 0..DIM {
            lo[j] = lo[j].min(d[j]);
            hi[j] = hi[j].max(d[j]);
        }
    }

    let mut seed = 20261020u64;
    let mut uniform_point = || -> [f64; DIM] {
        std::array::from_fn(|j| {
            let u = (lcg(&mut seed) as f64) / (usize::MAX as f64);
            lo[j] + u * (hi[j] - lo[j])
        })
    };

    let log_w = |k: usize| -> f64 { best_inertia(&std_data, k, 20, 3).ln() };

    // Gap(k) = mean_ref(log W*_k) - log W_k, na sd_k.
    let mut gaps = vec![0.0f64; kmax + 1];
    let mut sds = vec![0.0f64; kmax + 1];
    for k in 1..=kmax {
        let wk = log_w(k);
        let mut refs: Vec<f64> = Vec::with_capacity(b_ref);
        for _ in 0..b_ref {
            let rdata: Vec<[f64; DIM]> = (0..n).map(|_| uniform_point()).collect();
            let rst = standardize(&rdata);
            refs.push(best_inertia(&rst, k, 20, 3).ln());
        }
        let mean: f64 = refs.iter().sum::<f64>() / refs.len() as f64;
        let var: f64 = refs.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / refs.len() as f64;
        gaps[k] = mean - wk;
        sds[k] = (var.sqrt()) * (1.0 + 1.0 / b_ref as f64).sqrt();
    }

    // k ndogo yenye Gap(k) >= Gap(k+1) - s_{k+1}.
    for k in 1..kmax {
        if gaps[k] >= gaps[k + 1] - sds[k + 1] {
            return k;
        }
    }
    kmax
}

/// Uzito wa vipimo. F0 na sifa za spektra (centroid/rolloff) ni za
/// **mzungumzaji**; MFCC za fremu moja hufuata **fonimu**, hivyo uzito mdogo
/// (vinginevyo tofauti za vokali ndani ya mzungumzaji mmoja hugawanya kundi).
fn feature_weights() -> [f64; DIM] {
    let mut w = [1.0f64; DIM];
    for x in w.iter_mut().take(N_MFCC) {
        *x = 0.5;
    }
    w[8] = 0.7;
    w[9] = 0.7;
    w[10] = 0.7;
    w[11] = 0.7;
    w[12] = 0.3;
    w[13] = 0.3;
    w[14] = 1.2;
    w[15] = 1.0;
    w[16] = 2.5;
    w
}

/// Hupima uzito kwa data iliyosawazishwa (kwa kila kipimo).
fn weight_space(data: &[[f64; DIM]]) -> Vec<[f64; DIM]> {
    let w = feature_weights();
    data.iter()
        .map(|d| std::array::from_fn(|j| d[j] * w[j]))
        .collect()
}

/// K-means yenye restarts kwenye nafasi iliyotolewa (tayari imesawazishwa
/// na kupimwa uzito). Hurudisha (labels, centroids, inertia).
fn kmeans_space(space: &[[f64; DIM]], k: usize, iters: usize) -> (Vec<usize>, Vec<[f64; DIM]>, f64) {
    let mut best: Option<(Vec<usize>, Vec<[f64; DIM]>, f64)> = None;
    for r in 0..12u64 {
        let (l, c, inertia) = kmeans_once(space, k, iters, 7919 + r * 104729);
        if best.as_ref().map(|(_, _, bi)| inertia < *bi).unwrap_or(true) {
            best = Some((l, c, inertia));
        }
    }
    best.unwrap()
}

/// Huchagua idadi ya wazungumzaji kwa **silhouette** kwenye vekta za kanda
/// (window-averaged, zenye uzito). Kwa kanda za ~1.5s sifa hutawaliwa na
/// mzungumzaji. Makundi yenye hisa ndogo (<20%) hukataliwa (ni kelele/fonimu,
/// si mzungumzaji). Kama silhouette bora iko chini ya kizingiti, huchukulia
/// kundi moja (k=1) — hii ni muhimu kwa sauti ya mzungumzaji mmoja.
fn estimate_k(data: &[[f64; DIM]], kmax: usize) -> usize {
    let n = data.len();
    if n < 6 {
        return 1;
    }
    let kmax = kmax.clamp(1, 6).min(n / 3).max(1);
    if kmax < 2 {
        return 1;
    }
    let std_data = standardize(data);
    let space = weight_space(&std_data);
    let mut best_k = 1usize;
    let mut best_sil = 0.50f64;
    for k in 2..=kmax {
        let (labels, _, _) = kmeans_space(&space, k, 25);
        let mut counts = vec![0usize; k];
        for l in &labels {
            counts[*l] += 1;
        }
        let min_share = counts.iter().min().copied().unwrap_or(0) as f64 / n as f64;
        if min_share < 0.20 {
            continue;
        }
        let sil = silhouette(&space, &labels, k);
        if sil > best_sil {
            best_sil = sil;
            best_k = k;
        }
    }
    best_k
}

/// Diarization kamili: sauti → zamu za wazungumzaji.
///
/// `expected` = idadi ya wazungumzaji inayotarajiwa (0 = igundue yenyewe).
pub fn diarize(samples: &[f32], sample_rate: u32, expected: usize) -> Vec<Turn> {
    let n_fft = 512usize;
    let hop = n_fft / 2;
    let feats = frame_features(samples, sample_rate, n_fft, hop);
    if feats.is_empty() {
        return Vec::new();
    }

    // 1) Chuja fremu za kimya (RMS chini ya kizingiti) — hazina mzungumzaji.
    let rms_sorted = {
        let mut v: Vec<f64> = feats.iter().map(|f| f.rms).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    };
    let median = rms_sorted[rms_sorted.len() / 2];
    let thresh = (median * 0.35).max(1e-4);
    let voiced: Vec<usize> = (0..feats.len()).filter(|i| feats[*i].rms >= thresh).collect();
    if voiced.is_empty() {
        return Vec::new();
    }

    // 2) Gawa fremu zenye sauti kuwa **kanda za usemi** (~1.5s, zinazoingiliana
    //    kwa hatua ya 0.5s). K-means kwa fremu moja (32ms) hufuata fonimu, si
    //    mzungumzaji. Kwa wastani wa kanda ya 1.5s, sifa hutawaliwa na
    //    mzungumzaji — njia ya kawaida ya diarization (utterance-level clustering).
    let frames_per_win = ((sample_rate as f64 * 1.5) / hop as f64).round().max(1.0) as usize;
    let win_step = ((sample_rate as f64 * 0.5) / hop as f64).round().max(1.0) as usize;
    // Vekta za fremu (MFCC), kisha CMN juu ya fremu zote zenye sauti.
    let mut frame_vecs: Vec<[f64; DIM]> = voiced.iter().map(|i| feats[*i].vector()).collect();
    cmn(&mut frame_vecs);

    // Makundi ya fremu mfululizo (bila mpaka wa kimya) → kanda.
    let mut regions: Vec<(usize, usize)> = Vec::new(); // (start,end) kwa indeksi za `voiced`
    {
        let mut s = 0usize;
        for w in 1..=voiced.len() {
            let brk = w == voiced.len() || voiced[w] != voiced[w - 1] + 1;
            if brk {
                regions.push((s, w));
                s = w;
            }
        }
    }
    // Dirisha linaloingiliana ndani ya kila kanda (len 1.5s, hatua 0.5s).
    let mut windows: Vec<Vec<usize>> = Vec::new();
    for (s, e) in regions {
        let len = e - s;
        if len <= frames_per_win {
            windows.push((s..e).collect());
            continue;
        }
        let mut i = s;
        loop {
            let j = (i + frames_per_win).min(e);
            windows.push((i..j).collect());
            if j == e {
                break;
            }
            i += win_step;
        }
    }
    if windows.is_empty() {
        return Vec::new();
    }

    // Wastani wa sifa kwa kila kanda (kipimo cha mzungumzaji).
    let win_vecs: Vec<[f64; DIM]> = windows
        .iter()
        .map(|idxs| {
            let mut m = [0.0f64; DIM];
            for &i in idxs {
                for j in 0..DIM {
                    m[j] += frame_vecs[i][j];
                }
            }
            for v in m.iter_mut() {
                *v /= idxs.len() as f64;
            }
            m
        })
        .collect();

    // 3) Amua k na cluster kanda (si fremu), kwa nafasi yenye uzito.
    let k = if expected >= 1 {
        expected.clamp(1, 6)
    } else {
        estimate_k(&win_vecs, 4)
    };
    let win_space = weight_space(&standardize(&win_vecs));
    let (win_labels, _, _) = kmeans_space(&win_space, k, 25);

    // 4) Weka lebo ya kila fremu kutoka kanda yake.
    let mut full = vec![usize::MAX; feats.len()];
    for (wi, idxs) in windows.iter().enumerate() {
        for &i in idxs {
            full[voiced[i]] = win_labels[wi];
        }
    }

    // 5) Unganisha fremu mfululizo za mzungumzaji yule yule → zamu.
    //    Zamu fupi (<0.3s) huunganishwa na jirani (hupunguza "kuruka").
    let mut turns: Vec<Turn> = Vec::new();
    let mut i = 0usize;
    while i < full.len() {
        let cur = full[i];
        if cur == usize::MAX {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        while j < full.len() && full[j] == cur {
            j += 1;
        }
        let seg = &feats[start..j];
        let mean_rms = seg.iter().map(|f| f.rms).sum::<f64>() / seg.len() as f64;
        turns.push(Turn {
            speaker: cur,
            t_start: seg[0].t_start,
            t_end: seg[seg.len() - 1].t_end,
            mean_rms,
        });
        i = j;
    }

    // 6) Unganisha zamu fupi sana na jirani yenye nguvu zaidi.
    let min_len = 0.3;
    let mut merged: Vec<Turn> = Vec::new();
    for t in turns {
        let too_short = (t.t_end - t.t_start) < min_len;
        if too_short {
            if let Some(last) = merged.last_mut() {
                last.t_end = t.t_end;
                continue;
            }
        }
        if let Some(last) = merged.last_mut() {
            if last.speaker == t.speaker {
                last.t_end = t.t_end;
                last.mean_rms = (last.mean_rms + t.mean_rms) / 2.0;
                continue;
            }
        }
        merged.push(t);
    }

    // 7) Rekebisha namba za wazungumzaji kuwa 1..N kwa mpangilio wa kuonekana.
    let mut mapping: Vec<usize> = Vec::new();
    for t in merged.iter_mut() {
        let pos = mapping.iter().position(|m| *m == t.speaker);
        let new = match pos {
            Some(p) => p + 1,
            None => {
                mapping.push(t.speaker);
                mapping.len()
            }
        };
        t.speaker = new;
    }
    merged
}

/// Hutoa muhtasari wa diarization (kwa JSON).
pub fn summary(turns: &[Turn]) -> serde_json::Value {
    let speakers: Vec<usize> = {
        let mut s: Vec<usize> = turns.iter().map(|t| t.speaker).collect();
        s.sort_unstable();
        s.dedup();
        s
    };
    let total: f64 = turns.iter().map(|t| t.t_end - t.t_start).sum();
    let items: Vec<serde_json::Value> = turns
        .iter()
        .map(|t| {
            serde_json::json!({
                "speaker": t.speaker,
                "t_start": (t.t_start * 100.0).round() / 100.0,
                "t_end": (t.t_end * 100.0).round() / 100.0,
                "duration_s": ((t.t_end - t.t_start) * 100.0).round() / 100.0,
                "mean_rms": (t.mean_rms * 10000.0).round() / 10000.0,
            })
        })
        .collect();
    let per_speaker: Vec<serde_json::Value> = speakers
        .iter()
        .map(|s| {
            let dur: f64 = turns
                .iter()
                .filter(|t| t.speaker == *s)
                .map(|t| t.t_end - t.t_start)
                .sum();
            serde_json::json!({
                "speaker": s,
                "turns": turns.iter().filter(|t| t.speaker == *s).count(),
                "total_s": (dur * 100.0).round() / 100.0,
                "share_pct": if total > 0.0 { ((dur / total) * 1000.0).round() / 10.0 } else { 0.0 },
            })
        })
        .collect();
    serde_json::json!({
        "speakers": speakers.len(),
        "turns": turns.len(),
        "total_speech_s": (total * 100.0).round() / 100.0,
        "segments": items,
        "per_speaker": per_speaker,
        "method": "MFCC (mel 26 + DCT-II) + CMN + bendi + centroid/rolloff + F0 (autocorrelation), k-means kwa kanda za usemi 1.5s zenye uzito",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tengeneza toni (sine) ya muda na freq.
    fn tone(freq: f64, dur_s: f64, rate: u32, amp: f32) -> Vec<f32> {
        let n = (dur_s * rate as f64) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / rate as f64;
                (amp as f64 * (2.0 * std::f64::consts::PI * freq * t).sin()) as f32
            })
            .collect()
    }

    #[test]
    fn frame_features_ni_halisi() {
        let s = tone(440.0, 1.0, 16000, 0.5);
        let f = frame_features(&s, 16000, 512, 256);
        assert!(!f.is_empty(), "lazima kuna fremu");
        let first = &f[0];
        assert!(first.rms > 0.1, "RMS: {}", first.rms);
        assert!(first.centroid > 100.0 && first.centroid < 2000.0, "centroid: {}", first.centroid);
        let band_sum: f64 = first.bands.iter().sum();
        assert!((band_sum - 1.0).abs() < 0.01, "bendi zina jumla 1.0: {}", band_sum);
    }

    #[test]
    fn centroid_hutofautisha_toni_za_juu_na_chini() {
        let low = frame_features(&tone(200.0, 0.5, 16000, 0.5), 16000, 512, 256);
        let high = frame_features(&tone(3000.0, 0.5, 16000, 0.5), 16000, 512, 256);
        let lc = low.iter().map(|f| f.centroid).sum::<f64>() / low.len() as f64;
        let hc = high.iter().map(|f| f.centroid).sum::<f64>() / high.len() as f64;
        assert!(hc > lc * 3.0, "centroid ya juu ({hc}) lazima iwe > 3x ya chini ({lc})");
    }

    #[test]
    fn kmeans_hutenganisha_makundi_mawili_wazi() {
        let mut data: Vec<[f64; DIM]> = Vec::new();
        for i in 0..20 {
            { let mut a=[0.1f64;DIM]; a[0]=i as f64*0.01; data.push(a); }
        }
        for i in 0..20 {
            { let mut a=[0.9f64;DIM]; a[0]=5.0+i as f64*0.01; data.push(a); }
        }
        let (labels, _) = kmeans(&data, 2, 25);
        let first: Vec<usize> = labels[..20].to_vec();
        let second: Vec<usize> = labels[20..].to_vec();
        assert!(first.iter().all(|l| *l == first[0]), "kundi 1 linafanana");
        assert!(second.iter().all(|l| *l == second[0]), "kundi 2 linafanana");
        assert_ne!(first[0], second[0], "makundi yanatofautiana");
    }

    #[test]
    fn diarize_hugundua_wazungumzaji_wawili() {
        // Mzungumzaji A: toni ya chini (200Hz), B: toni ya juu (2500Hz).
        let mut audio = tone(200.0, 1.5, 16000, 0.5);
        audio.extend(tone(2500.0, 1.5, 16000, 0.5));
        audio.extend(tone(200.0, 1.5, 16000, 0.5));
        let turns = diarize(&audio, 16000, 2);
        assert!(!turns.is_empty(), "lazima kuna zamu");
        let speakers: std::collections::HashSet<usize> = turns.iter().map(|t| t.speaker).collect();
        assert!(speakers.len() >= 2, "lazima wagundulike wazungumzaji 2+, wamepatikana: {speakers:?}");
    }

    #[test]
    fn diarize_hukataa_sauti_tupu() {
        assert!(diarize(&[], 16000, 2).is_empty());
    }

    #[test]
    fn diarize_hurudisha_zamu_zenye_muda_sahihi() {
        let audio = tone(440.0, 2.0, 16000, 0.5);
        let turns = diarize(&audio, 16000, 1);
        for t in &turns {
            assert!(t.t_end > t.t_start, "muda wa mwisho > mwanzo");
            assert!(t.t_end <= 2.1, "hauzidi muda wa sauti");
        }
    }

    #[test]
    fn summary_ina_sehemu_zote() {
        let audio = tone(440.0, 1.0, 16000, 0.5);
        let turns = diarize(&audio, 16000, 1);
        let s = summary(&turns);
        assert!(s["speakers"].is_number());
        assert!(s["segments"].is_array());
        assert!(s["per_speaker"].is_array());
        assert!(s["method"].is_string());
    }

    #[test]
    fn standardize_hufanya_mean_0_na_std_1() {
        let data: Vec<[f64; DIM]> = (0..50)
            .map(|i| {
                let v = i as f64;
                { let mut a = [v; DIM]; a[1] = v*2.0; a[2] = v*0.1; a }
            })
            .collect();
        let z = standardize(&data);
        for j in 0..DIM {
            let mean: f64 = z.iter().map(|d| d[j]).sum::<f64>() / z.len() as f64;
            assert!(mean.abs() < 1e-9, "kipimo {j} mean={mean}");
            let var: f64 = z.iter().map(|d| d[j] * d[j]).sum::<f64>() / z.len() as f64;
            assert!((var - 1.0).abs() < 1e-6, "kipimo {j} var={var}");
        }
    }

    /// RNG rahisi ya test (deterministic).
    fn rnd(seed: &mut u64) -> f64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 33) as f64) / (usize::MAX as f64)
    }

    #[test]
    fn estimate_k_hugundua_kamili_ya_kweli() {
        // Makundi 2 yenye vipimo 8 VYENYE KUTOFautiana (independent noise).
        let mut seed = 42u64;
        let mut data: Vec<[f64; DIM]> = Vec::new();
        for _ in 0..40 {
            data.push(std::array::from_fn(|_| rnd(&mut seed) * 0.3));
        }
        for _ in 0..40 {
            data.push(std::array::from_fn(|_| 5.0 + rnd(&mut seed) * 0.3));
        }
        assert_eq!(estimate_k(&data, 4), 2, "makundi mawili yanagundulika");
    }

    #[test]
    fn estimate_k_hurudisha_1_kwa_kundi_moja() {
        // Kundi moja la kweli (unimodal, vipimo huru) — gap statistic → 1.
        let mut seed = 7u64;
        let data: Vec<[f64; DIM]> = (0..80)
            .map(|_| std::array::from_fn(|_| rnd(&mut seed) * 2.0))
            .collect();
        assert_eq!(estimate_k(&data, 4), 1, "kundi moja haligawanyiki");
    }

    #[test]
    fn gap_statistic_ni_deterministic() {
        let mut seed = 99u64;
        let data: Vec<[f64; DIM]> = (0..40)
            .map(|_| std::array::from_fn(|_| rnd(&mut seed)))
            .collect();
        assert_eq!(gap_statistic(&data, 4, 8), gap_statistic(&data, 4, 8));
    }

    #[test]
    fn kmeans_ni_deterministic() {
        let data: Vec<[f64; DIM]> = (0..40)
            .map(|i| {
                let v = (i % 2) as f64 * 5.0 + i as f64 * 0.01;
                [v; DIM]
            })
            .collect();
        let (l1, _) = kmeans(&data, 2, 25);
        let (l2, _) = kmeans(&data, 2, 25);
        assert_eq!(l1, l2, "k-means hurudia matokeo yale yale");
    }

    /// Sauti yenye hamoni (kama usemi) — F0 + hamoni 6 zenye fade.
    fn voice(f0: f64, dur_s: f64, rate: u32, amp: f32) -> Vec<f32> {
        let n = (dur_s * rate as f64) as usize;
        let fade = (rate as f64 * 0.03) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / rate as f64;
                let mut v = 0.0f64;
                for h in 1..=6 {
                    v += (1.0 / h as f64) * (2.0 * std::f64::consts::PI * f0 * h as f64 * t).sin();
                }
                let env = if i < fade {
                    i as f64 / fade as f64
                } else if i + fade > n {
                    (n - i) as f64 / fade as f64
                } else {
                    1.0
                };
                (amp as f64 * v * 0.35 * env) as f32
            })
            .collect()
    }

    #[test]
    fn f0_hutofautisha_toni_za_juu_na_chini() {
        // F0 ya toni 120Hz ni ndogo; ya 300Hz ni kubwa.
        let low = frame_features(&tone(120.0, 1.0, 16000, 0.6), 16000, 512, 256);
        let high = frame_features(&tone(300.0, 1.0, 16000, 0.6), 16000, 512, 256);
        let lm: Vec<f64> = low.iter().map(|f| f.f0).filter(|v| *v > 0.0).collect();
        let hm: Vec<f64> = high.iter().map(|f| f.f0).filter(|v| *v > 0.0).collect();
        assert!(!lm.is_empty(), "F0 ya toni ya chini ilipatikana");
        assert!(!hm.is_empty(), "F0 ya toni ya juu ilipatikana");
        let la = lm.iter().sum::<f64>() / lm.len() as f64;
        let ha = hm.iter().sum::<f64>() / hm.len() as f64;
        assert!(ha > la * 1.5, "F0 ya juu ({ha:.0}Hz) > 1.5x ya chini ({la:.0}Hz)");
    }

    #[test]
    fn auto_detect_hurudisha_mmoja_kwa_mzungumzaji_mmoja() {
        // Sauti ya mzungumzaji MMOJA (sauti yenye hamoni, F0 moja) — auto-detect = 1.
        let audio = voice(180.0, 5.0, 16000, 0.6);
        let turns = diarize(&audio, 16000, 0);
        let sp: std::collections::HashSet<usize> = turns.iter().map(|t| t.speaker).collect();
        assert_eq!(sp.len(), 1, "mzungumzaji mmoja asigawanywe: {sp:?}");
    }

    #[test]
    fn estimate_k_hurudisha_1_kwa_kanda_za_mzungumzaji_mmoja() {
        // Kanda 12 za mzungumzaji mmoja (keleteleke ndogo) — k=1.
        let mut seed = 2024u64;
        let data: Vec<[f64; DIM]> = (0..12)
            .map(|_| std::array::from_fn(|_| rnd(&mut seed) * 0.2))
            .collect();
        assert_eq!(estimate_k(&data, 4), 1, "kanda za mtu mmoja hazigawanyiki");
    }

    #[test]
    fn auto_detect_hugundua_wazungumzaji_wawili_bila_kwambia() {
        let mut audio = tone(200.0, 1.5, 16000, 0.5);
        audio.extend(tone(2500.0, 1.5, 16000, 0.5));
        audio.extend(tone(200.0, 1.5, 16000, 0.5));
        let turns = diarize(&audio, 16000, 0); // 0 = igundue yenyewe
        let sp: std::collections::HashSet<usize> = turns.iter().map(|t| t.speaker).collect();
        assert!(sp.len() >= 2, "auto-detect ilipata {sp:?}");
    }


}
