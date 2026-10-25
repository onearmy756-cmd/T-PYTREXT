//! quant.rs — KUFUNGUA QUANTIZATION za GGML → f32.
//!
//! Kila aina ina "block": F32/F16/BF16 (1 kipengele), Q4_0/Q8_0 (32), na
//! K-quants (256). Namba za block zinafuata llama.cpp (block_q4_0, block_q4_K…)
//! ili faili zilizopakuliwa kutoka HuggingFace zifunguke BILA kubadilishwa.
//!
//! Kila kazi ni PURE (bytes → f32) — hivyo inajaribiwa kwa vitengo bila modeli.

use crate::gguf::GgmlType;

/// f16 → f32 (bit manipulation, hakuna crate ya nje).
#[inline]
pub fn f16_to_f32(h: u16) -> f32 {
    let h = h as u32;
    let sign = (h & 0x8000) << 16;
    let exp = (h >> 10) & 0x1F;
    let mant = h & 0x3FF;
    let bits = if exp == 0 {
        if mant == 0 {
            sign
        } else {
            let mut e = exp as i32;
            let mut m = mant;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3FF;
            sign | (((e + 127 - 15 + 1) as u32) << 23) | (m << 13)
        }
    } else if exp == 0x1F {
        sign | 0x7F80_0000 | (mant << 13)
    } else {
        sign | ((exp + 127 - 15) << 23) | (mant << 13)
    };
    f32::from_bits(bits)
}

/// bf16 → f32 (kuhama bits 16 juu).
#[inline]
pub fn bf16_to_f32(b: u16) -> f32 {
    f32::from_bits((b as u32) << 16)
}

#[inline]
fn rd_f16(b: &[u8], off: usize) -> f32 {
    f16_to_f32(u16::from_le_bytes([b[off], b[off + 1]]))
}

/// Fumua safu MOJA ya tensor (elements nyingi) kutoka bytes → f32.
/// `out.len()` LAZIMA ilingane na `n_elements` ya tensor.
pub fn dequant_row(ty: GgmlType, bytes: &[u8], out: &mut [f32]) {
    match ty {
        GgmlType::F32 => dequant_f32(bytes, out),
        GgmlType::F16 => dequant_f16(bytes, out),
        GgmlType::Bf16 => dequant_bf16(bytes, out),
        GgmlType::Q4_0 => dequant_q4_0(bytes, out),
        GgmlType::Q4_1 => dequant_q4_1(bytes, out),
        GgmlType::Q5_0 => dequant_q5_0(bytes, out),
        GgmlType::Q5_1 => dequant_q5_1(bytes, out),
        GgmlType::Q8_0 => dequant_q8_0(bytes, out),
        GgmlType::Q8_1 => dequant_q8_1(bytes, out),
        GgmlType::Q2K => dequant_q2_k(bytes, out),
        GgmlType::Q3K => dequant_q3_k(bytes, out),
        GgmlType::Q4K => dequant_q4_k(bytes, out),
        GgmlType::Q5K => dequant_q5_k(bytes, out),
        GgmlType::Q6K => dequant_q6_k(bytes, out),
        GgmlType::Q8K => dequant_q8_k(bytes, out),
    }
}

fn dequant_f32(b: &[u8], out: &mut [f32]) {
    for (i, o) in out.iter_mut().enumerate() {
        *o = f32::from_le_bytes([b[i * 4], b[i * 4 + 1], b[i * 4 + 2], b[i * 4 + 3]]);
    }
}

fn dequant_f16(b: &[u8], out: &mut [f32]) {
    for (i, o) in out.iter_mut().enumerate() {
        *o = rd_f16(b, i * 2);
    }
}

fn dequant_bf16(b: &[u8], out: &mut [f32]) {
    for (i, o) in out.iter_mut().enumerate() {
        *o = bf16_to_f32(u16::from_le_bytes([b[i * 2], b[i * 2 + 1]]));
    }
}

fn dequant_q4_0(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(32).enumerate() {
        let base = blk * 18;
        let d = rd_f16(b, base);
        let qs = &b[base + 2..base + 18];
        for j in 0..16 {
            let x0 = (qs[j] & 0x0F) as i32 - 8;
            let x1 = (qs[j] >> 4) as i32 - 8;
            chunk[j] = x0 as f32 * d;
            chunk[j + 16] = x1 as f32 * d;
        }
    }
}

fn dequant_q4_1(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(32).enumerate() {
        let base = blk * 20;
        let d = rd_f16(b, base);
        let m = rd_f16(b, base + 2);
        let qs = &b[base + 4..base + 20];
        for j in 0..16 {
            let x0 = (qs[j] & 0x0F) as f32;
            let x1 = (qs[j] >> 4) as f32;
            chunk[j] = x0 * d + m;
            chunk[j + 16] = x1 * d + m;
        }
    }
}

fn dequant_q5_0(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(32).enumerate() {
        let base = blk * 22;
        let d = rd_f16(b, base);
        let qh = u32::from_le_bytes([b[base + 2], b[base + 3], b[base + 4], b[base + 5]]);
        let qs = &b[base + 6..base + 22];
        for j in 0..16 {
            let xh0 = ((qh >> j) & 1) as u8;
            let xh1 = ((qh >> (j + 16)) & 1) as u8;
            let x0 = ((qs[j] & 0x0F) | (xh0 << 4)) as i32 - 16;
            let x1 = ((qs[j] >> 4) | (xh1 << 4)) as i32 - 16;
            chunk[j] = x0 as f32 * d;
            chunk[j + 16] = x1 as f32 * d;
        }
    }
}

fn dequant_q5_1(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(32).enumerate() {
        let base = blk * 24;
        let d = rd_f16(b, base);
        let m = rd_f16(b, base + 2);
        let qh = u32::from_le_bytes([b[base + 4], b[base + 5], b[base + 6], b[base + 7]]);
        let qs = &b[base + 8..base + 24];
        for j in 0..16 {
            let xh0 = ((qh >> j) & 1) as u8;
            let xh1 = ((qh >> (j + 16)) & 1) as u8;
            let x0 = ((qs[j] & 0x0F) | (xh0 << 4)) as f32;
            let x1 = ((qs[j] >> 4) | (xh1 << 4)) as f32;
            chunk[j] = x0 * d + m;
            chunk[j + 16] = x1 * d + m;
        }
    }
}

fn dequant_q8_0(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(32).enumerate() {
        let base = blk * 34;
        let d = rd_f16(b, base);
        for j in 0..32 {
            chunk[j] = (b[base + 2 + j] as i8) as f32 * d;
        }
    }
}

fn dequant_q8_1(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(32).enumerate() {
        let base = blk * 36;
        let d = rd_f16(b, base);
        for j in 0..32 {
            chunk[j] = (b[base + 4 + j] as i8) as f32 * d;
        }
    }
}

/// Soma (scale, min) za K-quants 6-bit kutoka `scales[12]` (get_scale_min_k4).
#[inline]
fn get_scale_min_k4(j: usize, q: &[u8]) -> (u8, u8) {
    if j < 4 {
        (q[j] & 63, q[j + 4] & 63)
    } else {
        (
            (q[j + 4] & 0x0F) | ((q[j - 4] >> 6) << 4),
            (q[j + 4] >> 4) | ((q[j] >> 6) << 4),
        )
    }
}

fn dequant_q2_k(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 84;
        let scales = &b[base..base + 16];
        let qs = &b[base + 16..base + 80];
        let d = rd_f16(b, base + 80);
        let dmin = rd_f16(b, base + 82);
        let mut y = 0usize;
        let mut is = 0usize;
        for n in (0..256).step_by(128) {
            let mut shift = 0u32;
            for _j in 0..4 {
                let sc = scales[is];
                is += 1;
                let dl = d * (sc & 0x0F) as f32;
                let ml = dmin * (sc >> 4) as f32;
                for l in 0..16 {
                    let q = ((qs[n / 2 + l] >> shift) & 3) as f32;
                    chunk[y] = dl * q - ml;
                    y += 1;
                }
                let sc = scales[is];
                is += 1;
                let dl = d * (sc & 0x0F) as f32;
                let ml = dmin * (sc >> 4) as f32;
                for l in 0..16 {
                    let q = ((qs[n / 2 + 16 + l] >> shift) & 3) as f32;
                    chunk[y] = dl * q - ml;
                    y += 1;
                }
                shift += 2;
            }
        }
    }
}

fn dequant_q3_k(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 110;
        let hm = &b[base..base + 32];
        let qs = &b[base + 32..base + 96];
        let raw = &b[base + 96..base + 108];
        let d_all = rd_f16(b, base + 108);

        // Fungua scales 12 bytes → 16 × 6-bit signed (kanuni ya ggml).
        let mut aux = [0u32; 4];
        for (i, a) in aux.iter_mut().enumerate().take(3) {
            *a = u32::from_le_bytes([
                raw[i * 4],
                raw[i * 4 + 1],
                raw[i * 4 + 2],
                raw[i * 4 + 3],
            ]);
        }
        let tmp = aux[2];
        aux[2] = ((aux[0] >> 4) & 0x0F0F_0F0F) | (((tmp >> 4) & 0x0303_0303) << 4);
        aux[3] = ((aux[1] >> 4) & 0x0F0F_0F0F) | (((tmp >> 6) & 0x0303_0303) << 4);
        aux[0] = (aux[0] & 0x0F0F_0F0F) | ((tmp & 0x0303_0303) << 4);
        aux[1] = (aux[1] & 0x0F0F_0F0F) | (((tmp >> 2) & 0x0303_0303) << 4);
        let scales: Vec<i8> = aux
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .map(|b| b as i8)
            .collect();

        let mut y = 0usize;
        let mut is = 0usize;
        let mut m: u8 = 1;
        for n in (0..256).step_by(128) {
            let mut shift = 0u32;
            for _j in 0..4 {
                let dl = d_all * (scales[is] as f32 - 32.0);
                is += 1;
                for l in 0..16 {
                    let hi = if hm[n / 2 + l] & m != 0 { 0 } else { 4 };
                    let q = (((qs[n / 2 + l] >> shift) & 3) as i32) - hi;
                    chunk[y] = dl * q as f32;
                    y += 1;
                }
                let dl = d_all * (scales[is] as f32 - 32.0);
                is += 1;
                for l in 0..16 {
                    let hi = if hm[n / 2 + 16 + l] & m != 0 { 0 } else { 4 };
                    let q = (((qs[n / 2 + 16 + l] >> shift) & 3) as i32) - hi;
                    chunk[y] = dl * q as f32;
                    y += 1;
                }
                shift += 2;
                m <<= 1;
            }
        }
    }
}

fn dequant_q4_k(b: &[u8], out: &mut [f32]) {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { dequant_q4_k_avx2(b, out) };
            return;
        }
    }
    dequant_q4_k_scalar(b, out);
}

/// AVX2 kernel ya Q4_K: huchakata nibbles 8 kwa wakati (bit-exact na scalar).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn dequant_q4_k_avx2(b: &[u8], out: &mut [f32]) {
    use std::arch::x86_64::*;
    let mask = _mm256_set1_epi32(0x0F);
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 144;
        let d = rd_f16(b, base);
        let dmin = rd_f16(b, base + 2);
        let scales = &b[base + 4..base + 16];
        let qs = &b[base + 16..base + 144];
        let mut y = 0usize;
        let mut is = 0usize;
        let mut qoff = 0usize;
        for _ in 0..4 {
            let (sc1, m1) = get_scale_min_k4(is, scales);
            let (sc2, m2) = get_scale_min_k4(is + 1, scales);
            let d1 = _mm256_set1_ps(d * sc1 as f32);
            let mm1 = _mm256_set1_ps(dmin * m1 as f32);
            let d2 = _mm256_set1_ps(d * sc2 as f32);
            let mm2 = _mm256_set1_ps(dmin * m2 as f32);
            let p = qs.as_ptr().add(qoff);
            // Nibbles za chini: chunk[y..y+32].
            for t in 0..4 {
                let v = _mm_loadl_epi64(p.add(t * 8) as *const __m128i);
                let iv = _mm256_cvtepu8_epi32(v);
                let lo = _mm256_and_si256(iv, mask);
                let f = _mm256_cvtepi32_ps(lo);
                let r = _mm256_fmsub_ps(f, d1, mm1);
                _mm256_storeu_ps(chunk.as_mut_ptr().add(y + t * 8), r);
            }
            y += 32;
            // Nibbles za juu: chunk[y..y+32].
            for t in 0..4 {
                let v = _mm_loadl_epi64(p.add(t * 8) as *const __m128i);
                let iv = _mm256_cvtepu8_epi32(v);
                let hi = _mm256_and_si256(_mm256_srli_epi32(iv, 4), mask);
                let f = _mm256_cvtepi32_ps(hi);
                let r = _mm256_fmsub_ps(f, d2, mm2);
                _mm256_storeu_ps(chunk.as_mut_ptr().add(y + t * 8), r);
            }
            y += 32;
            qoff += 32;
            is += 2;
        }
    }
}

fn dequant_q4_k_scalar(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 144;
        let d = rd_f16(b, base);
        let dmin = rd_f16(b, base + 2);
        let scales = &b[base + 4..base + 16];
        let qs = &b[base + 16..base + 144];
        let mut y = 0usize;
        let mut is = 0usize;
        let mut qoff = 0usize;
        for _ in 0..4 {
            let (sc1, m1) = get_scale_min_k4(is, scales);
            let (sc2, m2) = get_scale_min_k4(is + 1, scales);
            let d1 = d * sc1 as f32;
            let mm1 = dmin * m1 as f32;
            let d2 = d * sc2 as f32;
            let mm2 = dmin * m2 as f32;
            for l in 0..32 {
                chunk[y] = d1 * (qs[qoff + l] & 0x0F) as f32 - mm1;
                y += 1;
            }
            for l in 0..32 {
                chunk[y] = d2 * (qs[qoff + l] >> 4) as f32 - mm2;
                y += 1;
            }
            qoff += 32;
            is += 2;
        }
    }
}

fn dequant_q5_k(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 176;
        let d = rd_f16(b, base);
        let dmin = rd_f16(b, base + 2);
        let scales = &b[base + 4..base + 16];
        let qh = &b[base + 16..base + 48];
        let ql = &b[base + 48..base + 176];
        let mut y = 0usize;
        let mut is = 0usize;
        let mut qoff = 0usize;
        let mut u1: u8 = 1;
        let mut u2: u8 = 2;
        for _ in 0..4 {
            let (sc1, m1) = get_scale_min_k4(is, scales);
            let (sc2, m2) = get_scale_min_k4(is + 1, scales);
            let d1 = d * sc1 as f32;
            let mm1 = dmin * m1 as f32;
            let d2 = d * sc2 as f32;
            let mm2 = dmin * m2 as f32;
            for l in 0..32 {
                let hi = if qh[qoff + l] & u1 != 0 { 16 } else { 0 };
                chunk[y] = d1 * ((ql[qoff + l] & 0x0F) + hi) as f32 - mm1;
                y += 1;
            }
            for l in 0..32 {
                let hi = if qh[qoff + l] & u2 != 0 { 16 } else { 0 };
                chunk[y] = d2 * ((ql[qoff + l] >> 4) + hi) as f32 - mm2;
                y += 1;
            }
            qoff += 32;
            is += 2;
            u1 <<= 2;
            u2 <<= 2;
        }
    }
}

fn dequant_q6_k(b: &[u8], out: &mut [f32]) {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { dequant_q6_k_avx2(b, out) };
            return;
        }
    }
    dequant_q6_k_scalar(b, out);
}

/// AVX2 kernel ya Q6_K: huchakata group ya 16 `l` kwa wakati (bit-exact).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn dequant_q6_k_avx2(b: &[u8], out: &mut [f32]) {
    use std::arch::x86_64::*;
    let m0f = _mm_set1_epi8(0x0F);
    let m03 = _mm_set1_epi8(0x03);
    let m32 = _mm256_set1_epi32(32);
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 210;
        let ql = b.as_ptr().add(base);
        let qh = b.as_ptr().add(base + 128);
        let sc = b.as_ptr().add(base + 192);
        let d = rd_f16(b, base + 208);
        for n in (0..256).step_by(128) {
            let qho = (n / 128) * 32;
            for g in 0..2 {
                let l0 = g * 16;
                let is = l0 / 16;
                let s = (n / 128) * 8;
                let ql_lo = _mm_loadu_si128(ql.add(n / 2 + l0) as *const __m128i);
                let ql_lo2 = _mm_loadu_si128(ql.add(n / 2 + 32 + l0) as *const __m128i);
                let qh_g = _mm_loadu_si128(qh.add(qho + l0) as *const __m128i);

                // q1 = (ql_lo & 0x0F) | ((qh>>0 & 3) << 4)
                let b0 = _mm_slli_epi16(_mm_and_si128(qh_g, m03), 4);
                let q1 = _mm_or_si128(_mm_and_si128(ql_lo, m0f), b0);
                // q2 = (ql_lo2 & 0x0F) | ((qh>>2 & 3) << 4)
                let b2 = _mm_slli_epi16(_mm_and_si128(_mm_srli_epi16(qh_g, 2), m03), 4);
                let q2 = _mm_or_si128(_mm_and_si128(ql_lo2, m0f), b2);
                // q3 = (ql_lo >> 4) | ((qh>>4 & 3) << 4)
                let b4 = _mm_slli_epi16(_mm_and_si128(_mm_srli_epi16(qh_g, 4), m03), 4);
                let q3 = _mm_or_si128(_mm_and_si128(_mm_srli_epi16(ql_lo, 4), m0f), b4);
                // q4 = (ql_lo2 >> 4) | ((qh>>6 & 3) << 4)
                let b6 = _mm_slli_epi16(_mm_and_si128(_mm_srli_epi16(qh_g, 6), m03), 4);
                let q4 = _mm_or_si128(_mm_and_si128(_mm_srli_epi16(ql_lo2, 4), m0f), b6);

                let sc1 = d * (*sc.add(s + is) as i8) as f32;
                let sc2 = d * (*sc.add(s + is + 2) as i8) as f32;
                let sc3 = d * (*sc.add(s + is + 4) as i8) as f32;
                let sc4 = d * (*sc.add(s + is + 6) as i8) as f32;

                emit16(&q1, sc1, l0, n, chunk, m32);
                emit16(&q2, sc2, l0 + 32, n, chunk, m32);
                emit16(&q3, sc3, l0 + 64, n, chunk, m32);
                emit16(&q4, sc4, l0 + 96, n, chunk, m32);
            }
        }
    }
}

/// Panua bytes 16 (0..63) → f32 16: (q-32)*scale; andika kwenye `chunk`.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn emit16(
    q: &std::arch::x86_64::__m128i,
    scale: f32,
    off: usize,
    n: usize,
    chunk: &mut [f32],
    m32: std::arch::x86_64::__m256i,
) {
    use std::arch::x86_64::*;
    let s = _mm256_set1_ps(scale);
    let lo = _mm256_cvtepu8_epi32(*q);
    let fl = _mm256_cvtepi32_ps(_mm256_sub_epi32(lo, m32));
    let rl = _mm256_mul_ps(fl, s);
    _mm256_storeu_ps(chunk.as_mut_ptr().add(n + off), rl);

    let hi8 = _mm_srli_si128(*q, 8);
    let h = _mm256_cvtepu8_epi32(hi8);
    let fh = _mm256_cvtepi32_ps(_mm256_sub_epi32(h, m32));
    let rh = _mm256_mul_ps(fh, s);
    _mm256_storeu_ps(chunk.as_mut_ptr().add(n + off + 8), rh);
}

fn dequant_q6_k_scalar(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 210;
        let ql = &b[base..base + 128];
        let qh = &b[base + 128..base + 192];
        let sc = &b[base + 192..base + 208];
        let d = rd_f16(b, base + 208);
        for n in (0..256).step_by(128) {
            let qho = n / 128 * 32; // qh huendelea 32 kwa iteration
            for l in 0..32 {
                let is = l / 16;
                let q1 = (((ql[n / 2 + l] & 0x0F) as u8 | (((qh[qho + l] >> 0) & 3) << 4)) as i32)
                    - 32;
                let q2 = (((ql[n / 2 + 32 + l] & 0x0F) as u8
                    | (((qh[qho + l] >> 2) & 3) << 4)) as i32)
                    - 32;
                let q3 = (((ql[n / 2 + l] >> 4) as u8
                    | (((qh[qho + l] >> 4) & 3) << 4)) as i32)
                    - 32;
                let q4 = (((ql[n / 2 + 32 + l] >> 4) as u8
                    | (((qh[qho + l] >> 6) & 3) << 4)) as i32)
                    - 32;
                let s = n / 128 * 8;
                chunk[n + l] = d * (sc[s + is] as i8) as f32 * q1 as f32;
                chunk[n + 32 + l] = d * (sc[s + is + 2] as i8) as f32 * q2 as f32;
                chunk[n + 64 + l] = d * (sc[s + is + 4] as i8) as f32 * q3 as f32;
                chunk[n + 96 + l] = d * (sc[s + is + 6] as i8) as f32 * q4 as f32;
            }
        }
    }
}

fn dequant_q8_k(b: &[u8], out: &mut [f32]) {
    for (blk, chunk) in out.chunks_mut(256).enumerate() {
        let base = blk * 292;
        let d = f32::from_le_bytes([b[base], b[base + 1], b[base + 2], b[base + 3]]);
        for j in 0..256 {
            chunk[j] = (b[base + 4 + j] as i8) as f32 * d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_na_bf16_sawa() {
        assert_eq!(f16_to_f32(0x3C00), 1.0);
        assert_eq!(f16_to_f32(0x4000), 2.0);
        assert_eq!(f16_to_f32(0xC000), -2.0);
        assert_eq!(f16_to_f32(0x0000), 0.0);
        assert_eq!(bf16_to_f32(0x3F80), 1.0);
        assert_eq!(bf16_to_f32(0x4000), 2.0);
    }

    #[test]
    fn q4_0_hufungua_sawa() {
        // block moja: d = 0.5 (f16 0x3800), nibbles → (q-8)*d
        let d: u16 = 0x3800; // 0.5
        let mut b = Vec::new();
        b.extend_from_slice(&d.to_le_bytes());
        // low nibbles 0..15 → q-8 = -8..7 ; high nibbles 15..0
        let mut qs = [0u8; 16];
        for j in 0..16 {
            let lo = j as u8; // 0..15
            let hi = (15 - j) as u8;
            qs[j] = lo | (hi << 4);
        }
        b.extend_from_slice(&qs);
        let mut out = [0f32; 32];
        dequant_row(GgmlType::Q4_0, &b, &mut out);
        assert_eq!(out[0], -4.0); // (0-8)*0.5
        assert_eq!(out[15], 3.5); // (15-8)*0.5
        assert_eq!(out[16], 3.5); // (15-8)*0.5
        assert_eq!(out[31], -4.0); // (0-8)*0.5
    }

    #[test]
    fn q8_0_hufungua_sawa() {
        let d: u16 = 0x3C00; // 1.0
        let mut b = Vec::new();
        b.extend_from_slice(&d.to_le_bytes());
        for j in 0..32i8 {
            b.push((j as i8).to_le_bytes()[0]);
        }
        let mut out = [0f32; 32];
        dequant_row(GgmlType::Q8_0, &b, &mut out);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[5], 5.0);
        assert_eq!(out[31], 31.0);
    }

    #[test]
    fn q4_k_hufungua_256() {
        // d=1, dmin=0, scales → sc=1, m=0 kwa sub-block zote; nibbles = l & 0xF
        let mut b = vec![0u8; 144];
        b[0..2].copy_from_slice(&0x3C00u16.to_le_bytes()); // d = 1.0
        b[2..4].copy_from_slice(&0x0000u16.to_le_bytes()); // dmin = 0
        // scales: j<4 → q[j]&63 = 1 (sc), q[j+4]&63 = 0 (min)
        for j in 0..4 {
            b[4 + j] = 1; // sc
            b[4 + j + 4] = 0; // min
        }
        for j in 4..8 {
            // sc = (q[j+4]&0xF) | ((q[j-4]>>6)<<4) ; min = (q[j+4]>>4) | ((q[j]>>6)<<4)
            b[4 + j + 4] = 0x01; // low nibble 1 → sc=1, min high nibble 0
        }
        for j in 0..128 {
            b[16 + j] = (j as u8 & 0x0F) | (((j as u8) & 0x0F) << 4);
        }
        let mut out = [0f32; 256];
        dequant_row(GgmlType::Q4K, &b, &mut out);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 1.0);
        assert_eq!(out[15], 15.0);
    }

    #[test]
    fn type_size_za_k_quants_ni_sahihi() {
        // Ukubwa halisi kutoka llama.cpp (block_* structs).
        assert_eq!(GgmlType::Q2K.type_size(), 84);
        assert_eq!(GgmlType::Q3K.type_size(), 110);
        assert_eq!(GgmlType::Q4K.type_size(), 144);
        assert_eq!(GgmlType::Q5K.type_size(), 176);
        assert_eq!(GgmlType::Q6K.type_size(), 210);
        assert_eq!(GgmlType::Q8K.type_size(), 292);
        // Q6K: block moja = 256 vipengele → 210 bytes.
        assert_eq!(GgmlType::Q6K.n_bytes_per(256), 210);
    }

    #[test]
    fn q6_k_hufungua_na_scale_sahihi() {
        // ql=0, qh=0 → q1..q4 = -32; scales[0..8] = 1 → out = d * 1 * -32
        let mut b = vec![0u8; 210];
        b[208..210].copy_from_slice(&0x3C00u16.to_le_bytes()); // d = 1.0
        for s in b.iter_mut().take(208).skip(192) {
            *s = 1; // scales = 1
        }
        let mut out = [0f32; 256];
        dequant_row(GgmlType::Q6K, &b, &mut out);
        assert_eq!(out[0], -32.0);
        assert_eq!(out[255], -32.0);
    }

    #[test]
    fn q4_k_avx2_ni_bit_exact_na_scalar() {
        // Data ya random kwa blocks 4 → linganisha scalar vs AVX2.
        let mut state: u64 = 0x1234_5678_9ABC_DEF0;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut b = vec![0u8; 144 * 4];
        for x in b.iter_mut() {
            *x = (next() & 0xFF) as u8;
        }
        // d/dmin ziwe f16 halali (eponential ndogo) ili thamani ziwe finite.
        for blk in 0..4 {
            let base = blk * 144;
            b[base..base + 2].copy_from_slice(&0x3000u16.to_le_bytes()); // ~0.125
            b[base + 2..base + 4].copy_from_slice(&0x2800u16.to_le_bytes()); // ~0.0625
        }
        let mut scalar = vec![0f32; 256 * 4];
        dequant_q4_k_scalar(&b, &mut scalar);
        let mut simd = vec![0f32; 256 * 4];
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe { dequant_q4_k_avx2(&b, &mut simd) };
                assert_eq!(scalar, simd, "AVX2 hailingani na scalar");
                return;
            }
        }
        // Bila AVX2, rudi kwa dispatch (itakuwa scalar) — angalia uthabiti.
        dequant_row(GgmlType::Q4K, &b, &mut simd);
        assert_eq!(scalar, simd);
    }

    #[test]
    fn q6_k_avx2_ni_bit_exact_na_scalar() {
        let mut state: u64 = 0xDEAD_BEEF_1234_5678;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut b = vec![0u8; 210 * 3];
        for x in b.iter_mut() {
            *x = (next() & 0xFF) as u8;
        }
        for blk in 0..3 {
            let base = blk * 210;
            b[base + 208..base + 210].copy_from_slice(&0x3000u16.to_le_bytes());
        }
        let mut scalar = vec![0f32; 256 * 3];
        dequant_q6_k_scalar(&b, &mut scalar);
        let mut simd = vec![0f32; 256 * 3];
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe { dequant_q6_k_avx2(&b, &mut simd) };
                assert_eq!(scalar, simd, "Q6_K AVX2 hailingani na scalar");
                return;
            }
        }
        dequant_row(GgmlType::Q6K, &b, &mut simd);
        assert_eq!(scalar, simd);
    }
}
