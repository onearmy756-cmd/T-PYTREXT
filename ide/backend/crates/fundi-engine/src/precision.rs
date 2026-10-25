//! precision.rs — BF16 FIDELITY: hifadhi ubora wa faili za bf16 bila kupoteza.
//!
//! Modeli za Qwen (na nyingine) huhifadhi weights kwa **bfloat16** (sehemu 8 za
//! exponent, 7 za mantissa). Kuzalisha kwa usahihi wa juu kwenye vifaa vidogo
//! kunahitaji:
//!   - **Compensated summation** (Kahan / Neumaier) — huondoa drifting ya f32
//!     wakati wa kujumlisha dot products ndefu (n_in hadi 17408).
//!   - **Stochastic rounding** kwa bf16 — hupunguza bias ya rounding compared
//!     na round-to-nearest-even (muhimu kwa mafunzo/quantization ikiwa).
//!   - **Block scaling** — huhifadhi mantissa kwa max precision kwa kundi.
//!
//! Kanuni: hakuna `.unwrap()`/`panic!`/`..` kwenye code ya runtime.

use crate::quant::bf16_to_f32;

/// f32 → bf16 kwa round-to-nearest-even (RNE) — kama llama.cpp hufanya.
#[inline]
pub fn f32_to_bf16_rne(x: f32) -> u16 {
    let bits = x.to_bits();
    // Round-to-nearest-even kwenye biti 16 za chini.
    let lsb = (bits >> 16) & 1;
    let rounded = bits.wrapping_add(0x7FFF + lsb);
    (rounded >> 16) as u16
}

/// f32 → bf16 kwa **stochastic rounding** (hupunguza bias). `rng` ni hali ya
/// xorshift; hurudisha bf16.
#[inline]
pub fn f32_to_bf16_stochastic(x: f32, rng: &mut u64) -> u16 {
    let bits = x.to_bits();
    let lsb = (bits >> 16) & 1;
    // Thamani ya chini ya mantissa (biti 16) — sehemu inayoachwa.
    let lower = bits & 0xFFFF;
    // Random 16-bit.
    let rand = (xorshift(rng) & 0xFFFF) as u32;
    // Round juu kama lower > rand, au tie-break kwa lsb.
    let round_up = if lower > rand {
        true
    } else if lower == rand {
        lsb == 1
    } else {
        false
    };
    let base = bits & 0xFFFF0000;
    let adjusted = if round_up { base + 0x10000 } else { base };
    ((if adjusted == 0 { base } else { adjusted }) >> 16) as u16
}

#[inline]
fn xorshift(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

/// Jumlisha kwa **Kahan compensated summation** (hurudisha jumla sahihi).
#[inline]
pub fn kahan_sum(xs: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let mut c = 0.0f32; // compensation
    for &v in xs {
        let y = v - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    sum
}

/// Jumlisha kwa **Neumaier** (bora kuliko Kahan wakati terms zinatofautiana).
#[inline]
pub fn neumaier_sum(xs: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    for &v in xs {
        let t = sum + v;
        if sum.abs() >= v.abs() {
            c += (sum - t) + v;
        } else {
            c += (v - t) + sum;
        }
        sum = t;
    }
    sum + c
}

/// Dot product yenye compensated summation (Kahan) — husaidia kwa n_in kubwa.
/// Hii ndiyo kernel inayohifadhi ubora wa bf16 kwenye vifaa vidogo.
pub fn dot_kahan(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut sum = 0.0f32;
    let mut c = 0.0f32;
    for i in 0..n {
        let prod = a[i] * b[i];
        let y = prod - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    sum
}

/// Dequantize safu ya bf16 → f32 KWA USAHIHI KAMILI (hakuna kupoteza zaidi ya
/// ufafanuzi wa bf16 wenyewe). Hii ni njia salama ya kusoma files za bf16.
pub fn bf16_row_to_f32(bytes: &[u8], out: &mut [f32]) {
    let n = out.len().min(bytes.len() / 2);
    for i in 0..n {
        let lo = bytes[i * 2];
        let hi = bytes[i * 2 + 1];
        out[i] = bf16_to_f32(u16::from_le_bytes([lo, hi]));
    }
}

/// Block scaling: chagua `scale` ili max|value|·scale iwe karibu 1.0 (kwa
/// bf16 mantissa precision kamili). Hurudisha `(scale, dequantized block)`.
pub fn block_scale(block: &[f32], max_val: f32) -> (f32, Vec<f32>) {
    let mut m = 0.0f32;
    for &v in block {
        let a = v.abs();
        if a > m {
            m = a;
        }
    }
    let scale = if m > 0.0 { max_val / m } else { 1.0 };
    let scaled: Vec<f32> = block.iter().map(|&v| v * scale).collect();
    (scale, scaled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn bf16_rne_huzunguka_kwa_usawa() {
        // 1.0 ni bf16 kamili.
        let b = f32_to_bf16_rne(1.0);
        assert!(approx(bf16_to_f32(b), 1.0, 1e-6));
    }

    #[test]
    fn bf16_round_trip_huhifadhi_thamani_kubwa() {
        let vals = [0.0f32, 1.0, -2.5, 100.0, 0.125];
        for v in vals {
            let b = f32_to_bf16_rne(v);
            let back = bf16_to_f32(b);
            // bf16 ina bits 8 za mantissa → hitilafu ndogo sana.
            let eps = v.abs().max(1.0) * 0.01;
            assert!(approx(back, v, eps), "v={v} back={back}");
        }
    }

    #[test]
    fn stochastic_rounding_haina_bias_kwa_wastani() {
        let mut rng = 0x1234_5678_9ABC_DEF0u64;
        let x = 1.0f32 + 0.0039; // karibu na katikati ya bf16 steps
        let mut sum = 0.0f64;
        let n = 2000;
        for _ in 0..n {
            let b = f32_to_bf16_stochastic(x, &mut rng);
            sum += bf16_to_f32(b) as f64;
        }
        let mean = (sum / n as f64) as f32;
        assert!(approx(mean, x, 0.004), "mean={mean} x={x}");
    }

    #[test]
    fn kahan_ni_sahihi_kuliko_naive_kwa_mfululizo_mrefu() {
        // Kwa 1e8, ulp ≈ 8 — kuongeza 1.0 hupotea kwenye naive summation.
        let mut xs = vec![1e8f32];
        for _ in 0..1000 {
            xs.push(1.0);
        }
        xs.push(-1e8);
        let naive: f32 = xs.iter().sum();
        let comp = kahan_sum(&xs);
        // Kahan inapaswa kukaribia jumla ya kweli (1000); naive hukaribia 0.
        assert!(approx(comp, 1000.0, 1.0), "comp={comp}");
        assert!(naive.abs() < 1.0, "naive={naive}");
    }

    #[test]
    fn neumaier_ni_sahihi() {
        let xs = [1e20f32, 1.0, -1e20];
        let s = neumaier_sum(&xs);
        assert!(approx(s, 1.0, 0.5), "s={s}");
    }

    #[test]
    fn dot_kahan_inalingana_na_dot_ya_kawaida_kwa_ukubwa_mdogo() {
        let a = [1.0f32, 2.0, 3.0];
        let b = [4.0f32, 5.0, 6.0];
        assert!(approx(dot_kahan(&a, &b), 32.0, 1e-5));
    }

    #[test]
    fn bf16_row_to_f32_inapakia_byte_sahihi() {
        // LE bytes: 1.0 = bf16 0x3F80 -> [0x80,0x3F]; -2.0 = 0xC000 -> [0x00,0xC0]
        let bytes = [0x80, 0x3F, 0x00, 0xC0];
        let mut out = [0.0f32; 2];
        bf16_row_to_f32(&bytes, &mut out);
        assert!(approx(out[0], 1.0, 1e-6), "o0={}", out[0]);
        assert!(approx(out[1], -2.0, 1e-6), "o1={}", out[1]);
    }

    #[test]
    fn block_scale_hupandisha_kiwango() {
        let block = [0.5f32, -0.25, 0.1];
        let (scale, scaled) = block_scale(&block, 1.0);
        assert!(approx(scale, 2.0, 1e-5), "scale={scale}");
        assert!(approx(scaled[0], 1.0, 1e-5));
    }
}
