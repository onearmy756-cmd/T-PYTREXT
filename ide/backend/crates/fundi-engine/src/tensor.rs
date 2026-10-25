//! tensor.rs — MATEMATIKA ya tensor (matvec, rmsnorm, softmax, rope).
//!
//! Kanuni: RAM ndogo. Hatuhifadhi weights zilizofunguliwa; kila matvec hufungua
//! safu moja kwa wakati kwenye buffer ndogo ya scratch na kuhesabu dot product.
//! Hii inafanya modeli ya 4-bit ifanye kazi kwenye RAM ndogo (mmap ya OS page
//! cache inashughulikia I/O).

use crate::gguf::TensorInfo;
use crate::quant::dequant_row;

/// Tensor iliyounganishwa na mmap (bytes zake ziko kwenye faili).
pub struct Tensor<'a> {
    pub info: TensorInfo,
    pub bytes: &'a [u8],
}

impl<'a> Tensor<'a> {
    pub fn new(info: TensorInfo, bytes: &'a [u8]) -> Self {
        Tensor { info, bytes }
    }

    /// n_in = dims[0] (kanuni ya GGML: ne0 ni mfululizo).
    pub fn n_in(&self) -> usize {
        self.info.dims[0] as usize
    }

    /// n_out = dims[1] (1 kama 1-D).
    pub fn n_out(&self) -> usize {
        if self.info.dims.len() < 2 {
            1
        } else {
            self.info.dims[1] as usize
        }
    }

    /// Fumua safu ya `row` (n_in vipengele) kwenye `out`.
    pub fn dequant_row(&self, row: usize, out: &mut [f32]) {
        let n_in = self.n_in();
        let bs = self.info.ty.block_size();
        let ts = self.info.ty.type_size();
        let row_bytes = n_in / bs * ts;
        let start = row * row_bytes;
        dequant_row(self.info.ty, &self.bytes[start..start + row_bytes], out);
    }

    /// Dot ya safu `row` na `x` kwa **block-wise fusion**: hufungua block MOJA
    /// (≤256 f32 → L1) na kuhesabu dot mara moja. Hii huepusha memory traffic
    /// (hakuna scratch ya n_in nzima) — hii ndiyo "speed" halisi ya CPU.
    /// Kwa quant zisizo-block (F32/F16/BF16, block_size < 32) hurudi kwa safu nzima.
    pub fn dot_row(&self, row: usize, x: &[f32], scratch: &mut [f32]) -> f32 {
        let n_in = self.n_in();
        let bs = self.info.ty.block_size();
        let ts = self.info.ty.type_size();
        let row_bytes = n_in / bs * ts;
        let start = row * row_bytes;
        if bs < 32 {
            // Hakuna faida ya blocking; fumua safu nzima mara moja.
            dequant_row(
                self.info.ty,
                &self.bytes[start..start + row_bytes],
                &mut scratch[..n_in],
            );
            return dot(&scratch[..n_in], x);
        }
        let row_slice = &self.bytes[start..start + row_bytes];
        let nblocks = n_in / bs;
        let mut acc = 0f32;
        for blk in 0..nblocks {
            let bbytes = &row_slice[blk * ts..(blk + 1) * ts];
            dequant_row(self.info.ty, bbytes, &mut scratch[..bs]);
            acc += dot(&scratch[..bs], &x[blk * bs..(blk + 1) * bs]);
        }
        acc
    }

    /// y = W · x  (W: n_out × n_in). `scratch` ni buffer ya n_in f32 (inatumika tena).
    pub fn matvec(&self, x: &[f32], y: &mut [f32], scratch: &mut [f32]) {
        let n_in = self.n_in();
        debug_assert_eq!(x.len(), n_in);
        for (o, yo) in y.iter_mut().enumerate().take(self.n_out()) {
            *yo = self.dot_row(o, x, scratch);
        }
    }

    /// y = W · x kwa **safu NYINGI kwa wakati mmoja** (row-parallel).
    ///
    /// Hii ni "speed" ya MTECHOS kwenye CPU: kila core hushughulikia kipande cha
    /// safu, na kila thread ina `scratch` yake (hakuna kushirikiana state).
    /// Hurudi kwa `matvec` ya serial kama cores ≤ 1 au matrix ni ndogo (overhead).
    pub fn matvec_par(&self, x: &[f32], y: &mut [f32]) {
        let n_in = self.n_in();
        let n_out = self.n_out();
        if x.len() != n_in {
            return;
        }
        let nthreads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(n_out.max(1));
        // Matrix ndogo → overhead ya kuunda thread haifai.
        if nthreads <= 1 || n_out < 256 {
            let mut scratch = vec![0f32; n_in];
            self.matvec(x, y, &mut scratch);
            return;
        }
        let chunk = n_out.div_ceil(nthreads);
        let bs = self.info.ty.block_size();
        std::thread::scope(|s| {
            for (ti, ychunk) in y.chunks_mut(chunk).enumerate() {
                let start = ti * chunk;
                s.spawn(move || {
                    let mut scratch = vec![0f32; n_in.max(bs)];
                    for (o, yo) in ychunk.iter_mut().enumerate() {
                        let row = start + o;
                        if row >= n_out {
                            break;
                        }
                        *yo = self.dot_row(row, x, &mut scratch);
                    }
                });
            }
        });
    }
}

/// Dot product ya f32 — AVX2+FMA kama inapatikana (runtime detect), vinginevyo
/// scalar iliyofunguliwa 4×. Hii ndiyo kernel ya moto ya matvec; ndiyo "speed".
#[inline]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            return unsafe { dot_avx512(a, b) };
        }
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return unsafe { dot_avx2(a, b) };
        }
    }
    dot_scalar(a, b)
}

/// Scalar fallback: 4 accumulators (ILP) — hupunguza latency ya FMA chain.
fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let mut acc0 = 0f32;
    let mut acc1 = 0f32;
    let mut acc2 = 0f32;
    let mut acc3 = 0f32;
    let chunks = n / 4;
    for c in 0..chunks {
        let i = c * 4;
        acc0 += a[i] * b[i];
        acc1 += a[i + 1] * b[i + 1];
        acc2 += a[i + 2] * b[i + 2];
        acc3 += a[i + 3] * b[i + 3];
    }
    let mut acc = (acc0 + acc1) + (acc2 + acc3);
    for i in chunks * 4..n {
        acc += a[i] * b[i];
    }
    acc
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
unsafe fn dot_avx2(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    let n = a.len();
    let mut acc = _mm256_setzero_ps();
    let mut i = 0usize;
    while i + 8 <= n {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        acc = _mm256_fmadd_ps(va, vb, acc);
        i += 8;
    }
    // Horizontal sum ya 8 lanes.
    let lo = _mm256_castps256_ps128(acc);
    let hi = _mm256_extractf128_ps(acc, 1);
    let mut s = _mm_add_ps(lo, hi);
    s = _mm_hadd_ps(s, s);
    s = _mm_hadd_ps(s, s);
    let mut sum = _mm_cvtss_f32(s);
    while i < n {
        sum += *a.get_unchecked(i) * *b.get_unchecked(i);
        i += 1;
    }
    sum
}

/// AVX-512 kernel (16 f32 kwa mzunguko, 2 accumulators kwa ILP). Hutumika
/// kwenye CPU zinazounga mkono avx512f (runtime detect) — hadi ~2× juu ya AVX2.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn dot_avx512(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    let n = a.len();
    let mut acc0 = _mm512_setzero_ps();
    let mut acc1 = _mm512_setzero_ps();
    let mut i = 0usize;
    while i + 32 <= n {
        let a0 = _mm512_loadu_ps(a.as_ptr().add(i));
        let b0 = _mm512_loadu_ps(b.as_ptr().add(i));
        acc0 = _mm512_fmadd_ps(a0, b0, acc0);
        let a1 = _mm512_loadu_ps(a.as_ptr().add(i + 16));
        let b1 = _mm512_loadu_ps(b.as_ptr().add(i + 16));
        acc1 = _mm512_fmadd_ps(a1, b1, acc1);
        i += 32;
    }
    while i + 16 <= n {
        let a0 = _mm512_loadu_ps(a.as_ptr().add(i));
        let b0 = _mm512_loadu_ps(b.as_ptr().add(i));
        acc0 = _mm512_fmadd_ps(a0, b0, acc0);
        i += 16;
    }
    let mut sum = _mm512_reduce_add_ps(_mm512_add_ps(acc0, acc1));
    while i < n {
        sum += *a.get_unchecked(i) * *b.get_unchecked(i);
        i += 1;
    }
    sum
}

/// RMSNorm: y = x / sqrt(mean(x²) + eps) * w
pub fn rmsnorm(x: &[f32], w: &[f32], eps: f32, out: &mut [f32]) {
    let n = x.len();
    let mut ss = 0f32;
    for v in x {
        ss += v * v;
    }
    let inv = 1.0 / (ss / n as f32 + eps).sqrt();
    for i in 0..n {
        out[i] = x[i] * inv * w[i];
    }
}

/// Softmax in-place (max-shifted kwa uthabiti wa namba).
pub fn softmax(x: &mut [f32]) {
    let mut m = f32::NEG_INFINITY;
    for &v in x.iter() {
        if v > m {
            m = v;
        }
    }
    let mut sum = 0f32;
    for v in x.iter_mut() {
        *v = (*v - m).exp();
        sum += *v;
    }
    if sum > 0.0 {
        for v in x.iter_mut() {
            *v /= sum;
        }
    }
}

/// SiLU: x * sigmoid(x)
#[inline]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// GeLU (tanh approximation — inatumika kwenye baadhi ya modeli).
#[inline]
pub fn gelu(x: f32) -> f32 {
    let c = (2.0f32 / std::f32::consts::PI).sqrt();
    0.5 * x * (1.0 + (c * (x + 0.044715 * x * x * x)).tanh())
}

/// RoPE: zungusha jozi kwa pembe θ. `neox` = chaguo la mpangilio wa jozi:
///   - false (NORM/Llama): jozi (i, i + head_dim/2)
///   - true  (NEOX/Qwen2/Gemma/Phi3): jozi (2i, 2i+1)
pub fn rope(
    q: &mut [f32],
    k: &mut [f32],
    n_head: usize,
    n_head_kv: usize,
    head_dim: usize,
    pos: usize,
    base: f32,
    neox: bool,
) {
    fn apply(v: &mut [f32], head_dim: usize, pos: usize, base: f32, neox: bool) {
        let half = head_dim / 2;
        for i in 0..half {
            let theta = pos as f32 / base.powf(2.0 * i as f32 / head_dim as f32);
            let (s, c) = theta.sin_cos();
            let (i0, i1) = if neox { (2 * i, 2 * i + 1) } else { (i, i + half) };
            let x0 = v[i0];
            let x1 = v[i1];
            v[i0] = x0 * c - x1 * s;
            v[i1] = x0 * s + x1 * c;
        }
    }
    for h in 0..n_head {
        apply(
            &mut q[h * head_dim..(h + 1) * head_dim],
            head_dim,
            pos,
            base,
            neox,
        );
    }
    for h in 0..n_head_kv {
        apply(
            &mut k[h * head_dim..(h + 1) * head_dim],
            head_dim,
            pos,
            base,
            neox,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rmsnorm_hupunguza_kwa_wastani() {
        let x = [3.0f32, 4.0];
        let w = [1.0f32, 1.0];
        let mut out = [0f32; 2];
        rmsnorm(&x, &w, 0.0, &mut out);
        // mean(x²)=12.5 → sqrt=3.5355 → [0.8485, 1.1314]
        assert!((out[0] - 0.8485).abs() < 1e-3, "{:?}", out);
        assert!((out[1] - 1.1314).abs() < 1e-3, "{:?}", out);
    }

    #[test]
    fn softmax_hujumlisha_moja() {
        let mut x = [1.0f32, 2.0, 3.0];
        softmax(&mut x);
        let s: f32 = x.iter().sum();
        assert!((s - 1.0).abs() < 1e-5);
        assert!(x[2] > x[1] && x[1] > x[0]);
    }

    #[test]
    fn silu_na_gelu_huhesabu() {
        assert!((silu(0.0) - 0.0).abs() < 1e-6);
        assert!((silu(1.0) - 0.7311).abs() < 1e-3);
        assert!((gelu(0.0)).abs() < 1e-6);
    }

    #[test]
    fn rope_huhifadhi_norm() {
        let mut q = vec![1.0f32, 0.0, 0.0, 1.0];
        let mut k = vec![0f32; 0];
        let n0: f32 = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        rope(&mut q, &mut k, 1, 0, 4, 3, 10000.0, false);
        let n1: f32 = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((n0 - n1).abs() < 1e-5);
    }

    #[test]
    fn dot_inalingana_na_scalar_kwa_urefu_mbalimbali() {
        // Urefu unaojumuisha tail (n%8 != 0) — AVX2 path + scalar path zinakubaliana.
        for n in [1usize, 3, 7, 8, 9, 15, 16, 17, 100, 127] {
            let a: Vec<f32> = (0..n).map(|i| (i as f32 * 0.37).sin()).collect();
            let b: Vec<f32> = (0..n).map(|i| (i as f32 * 0.11).cos()).collect();
            let got = dot(&a, &b);
            let want = dot_scalar(&a, &b);
            assert!(
                (got - want).abs() < 1e-3 * (1.0 + want.abs()),
                "n={n}: got={got} want={want}"
            );
        }
    }
}
