//! calculus.rs — CALCULUS kwa vifaa vidogo (pure Rust).
//!
//! Hii ni safu ya kihesabu inayotumiwa na CORTEX (kwa mafunzo/optimization) na
//! na matumizi ya kawaida ya kiinjeni. Inatoa:
//!   - **Dual numbers** (forward-mode autodiff): kwa derivative kamili.
//!   - **Gradient** ya scalar function (central difference, kwa usalama).
//!   - **Integral** (Simpson 1/3 + Romberg) kwa usahihi wa juu.
//!   - **Trapezoidal** cumulative integral kwa mfululizo.
//!
//! Kanuni za MTECHOS: hakuna `.unwrap()`/`panic!`/`..` kwenye code ya runtime;
//! vipimo vibovu hurejesha `None` au `0.0` kwa usalama.

/// Namba ya dual (a + b·ε, ε²=0) kwa forward-mode autodiff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dual {
    /// Thamani halisi (value).
    pub val: f32,
    /// Derivative (tangent) sehemu.
    pub der: f32,
}

impl Dual {
    /// Dual kutoka thamani na derivative.
    #[inline]
    pub fn new(val: f32, der: f32) -> Self {
        Dual { val, der }
    }

    /// Kubadilisha scalar kuwa dual yenye derivative 0.
    #[inline]
    pub fn constant(val: f32) -> Self {
        Dual { val, der: 0.0 }
    }

    /// Kigezo huru x (derivative = 1) kwa tofautishaji.
    #[inline]
    pub fn var(val: f32) -> Self {
        Dual { val, der: 1.0 }
    }

    #[inline]
    pub fn add(self, o: Dual) -> Dual {
        Dual::new(self.val + o.val, self.der + o.der)
    }

    #[inline]
    pub fn sub(self, o: Dual) -> Dual {
        Dual::new(self.val - o.val, self.der - o.der)
    }

    #[inline]
    pub fn mul(self, o: Dual) -> Dual {
        Dual::new(self.val * o.val, self.der * o.val + self.val * o.der)
    }

    /// Mgawanyiko: d(u/v) = (u'v - uv')/v².
    #[inline]
    pub fn div(self, o: Dual) -> Dual {
        if o.val.abs() <= f32::EPSILON {
            return Dual::constant(0.0);
        }
        let inv = 1.0 / o.val;
        Dual::new(self.val * inv, (self.der * o.val - self.val * o.der) * inv * inv)
    }

    /// Dhambi (sin): d = cos·u'.
    #[inline]
    pub fn sin(self) -> Dual {
        Dual::new(self.val.sin(), self.val.cos() * self.der)
    }

    /// Kosini (cos): d = -sin·u'.
    #[inline]
    pub fn cos(self) -> Dual {
        Dual::new(self.val.cos(), -self.val.sin() * self.der)
    }

    /// exp: d = exp·u'.
    #[inline]
    pub fn exp(self) -> Dual {
        let e = self.val.exp();
        Dual::new(e, e * self.der)
    }

    /// ln: d = u'/u (hurejesha 0 kwa u ≤ 0).
    #[inline]
    pub fn ln(self) -> Dual {
        if self.val <= 0.0 {
            return Dual::constant(0.0);
        }
        Dual::new(self.val.ln(), self.der / self.val)
    }

    /// Nguvu ya namba kamili ya kweli: u^n.
    #[inline]
    pub fn powi(self, n: i32) -> Dual {
        let v = self.val.powi(n);
        Dual::new(v, (n as f32) * self.val.powi(n - 1) * self.der)
    }

    /// tanh: d = (1 - tanh²)·u'.
    #[inline]
    pub fn tanh(self) -> Dual {
        let t = self.val.tanh();
        Dual::new(t, (1.0 - t * t) * self.der)
    }
}

/// Tofautisha scalar function `f` kwa `x` (forward-mode, derivative kamili).
pub fn derivative<F: Fn(Dual) -> Dual>(f: F, x: f32) -> f32 {
    f(Dual::var(x)).der
}

/// Gradient ya scalar function kwa nyuzi (central difference, hatua `h`).
/// Inatumika pale function haipokei `Dual` moja kwa moja (mf. coefficients).
pub fn gradient_central<F: Fn(&[f32]) -> f32>(f: F, x: &[f32], h: f32) -> Vec<f32> {
    let h = h.abs().max(1e-6);
    let mut g = vec![0.0f32; x.len()];
    let mut probe = x.to_vec();
    for i in 0..x.len() {
        let orig = probe[i];
        probe[i] = orig + h;
        let fp = f(&probe);
        probe[i] = orig - h;
        let fm = f(&probe);
        probe[i] = orig;
        g[i] = (fp - fm) / (2.0 * h);
    }
    g
}

/// Integral ya `f` juu ya [a, b] kwa Simpson 1/3 (n lazima iwe sawa haiwe kichwa;
/// n=0 husahihishwa kuwa 2). Hurudisha 0.0 kwa n ≤ 0.
pub fn integrate_simpson<F: Fn(f32) -> f32>(f: F, a: f32, b: f32, n: usize) -> f32 {
    let n = if n == 0 { 2 } else if n % 2 == 1 { n + 1 } else { n };
    if n == 0 {
        return 0.0;
    }
    let h = (b - a) / n as f32;
    if h == 0.0 || !h.is_finite() {
        return 0.0;
    }
    let mut s = f(a) + f(b);
    for i in 1..n {
        let x = a + h * i as f32;
        let w = if i % 2 == 0 { 2.0 } else { 4.0 };
        s += w * f(x);
    }
    s * h / 3.0
}

/// Integral kwa Romberg (Richardson extrapolation) — usahihi wa juu kwa function
/// laini. `levels` ni idadi ya tu kwenye triangle (>=1).
pub fn integrate_romberg<F: Fn(f32) -> f32>(f: F, a: f32, b: f32, levels: usize) -> f32 {
    let lv = levels.clamp(1, 16);
    let mut r = vec![vec![0.0f32; lv]; lv];
    // Trapezoidal (h col moja) kwa kiwango cha 0.
    r[0][0] = (b - a) * 0.5 * (f(a) + f(b));
    for i in 1..lv {
        let n = 1usize << i; // 2^i intervals
        let h = (b - a) / n as f32;
        let mut sum = 0.0f32;
        // Nukta mpya tu (odd indices).
        let mut k = 1usize;
        while k < n {
            sum += f(a + h * k as f32);
            k += 2;
        }
        r[i][0] = 0.5 * r[i - 1][0] + h * sum;
        let mut j = 1usize;
        while j <= i {
            let factor = 4f32.powi(j as i32);
            r[i][j] = r[i][j - 1] + (r[i][j - 1] - r[i - 1][j - 1]) / (factor - 1.0);
            j += 1;
        }
    }
    r[lv - 1][lv - 1]
}

/// Integral ya kimkusanyiko (cumulative trapezoid) kwa mfululizo.
/// Hurudisha vector yenye urefu sawa na `y` (element ya kwanza = 0.0).
pub fn cumulative_trapezoid(y: &[f32], dx: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; y.len()];
    for i in 1..y.len() {
        out[i] = out[i - 1] + 0.5 * (y[i] + y[i - 1]) * dx;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn dual_powi_na_lna_ni_sahihi() {
        // f(x) = x³ ; f'(2) = 12
        let d = derivative(|x| x.powi(3), 2.0);
        assert!(approx(d, 12.0, 1e-3), "d={d}");
        // f(x) = ln(x) ; f'(e) = 1/e
        let d2 = derivative(|x| x.ln(), std::f32::consts::E);
        assert!(approx(d2, 1.0 / std::f32::consts::E, 1e-3), "d2={d2}");
    }

    #[test]
    fn derivative_ya_mraba_ni_mstari() {
        let d = derivative(|x| x.mul(x), 3.0);
        assert!(approx(d, 6.0, 1e-4), "d={d}");
    }

    #[test]
    fn derivative_ya_sin_ni_cos() {
        let d = derivative(|x| x.sin(), 0.0);
        assert!(approx(d, 1.0, 1e-4), "d={d}");
    }

    #[test]
    fn derivative_ya_exp_ni_exp() {
        let d = derivative(|x| x.exp(), 1.0);
        assert!(approx(d, std::f32::consts::E, 1e-3));
    }

    #[test]
    fn chain_rule_inashikamana() {
        // f(x) = sin(x²) ; f'(x) = 2x·cos(x²). Kwa x=1: 2·cos(1).
        let d = derivative(|x| x.mul(x).sin(), 1.0);
        let want = 2.0 * 1.0f32.cos();
        assert!(approx(d, want, 1e-3), "d={d} want={want}");
    }

    #[test]
    fn gradient_hupata_mwelekeo_kwa_juu() {
        // f(x,y) = x² + 3y² ; grad = (2x, 6y)
        let g = gradient_central(|v| v[0] * v[0] + 3.0 * v[1] * v[1], &[2.0, 1.0], 1e-3);
        assert!(approx(g[0], 4.0, 1e-2), "g0={}", g[0]);
        assert!(approx(g[1], 6.0, 1e-2), "g1={}", g[1]);
    }

    #[test]
    fn simpson_ya_x_kwamba_mbili_kutoka_0_hadi_3() {
        // ∫ x² dx = 9 (0..3)
        let v = integrate_simpson(|x| x * x, 0.0, 3.0, 100);
        assert!(approx(v, 9.0, 1e-2), "v={v}");
    }

    #[test]
    fn romberg_ni_sahihi_kwa_sin() {
        // ∫ sin(x) dx = 2 (0..π)
        let v = integrate_romberg(|x| x.sin(), 0.0, std::f32::consts::PI, 8);
        assert!(approx(v, 2.0, 1e-4), "v={v}");
    }

    #[test]
    fn cumulative_trapezoid_ya_mstari() {
        // y = x, dx=1 → integral = n(n-1)/2
        let y = [0.0f32, 1.0, 2.0, 3.0];
        let c = cumulative_trapezoid(&y, 1.0);
        assert!(approx(c[3], 4.5, 1e-4), "c3={}", c[3]);
    }

    #[test]
    fn integrate_hurudisha_0_kwa_kipimo_bovu() {
        assert_eq!(integrate_simpson(|x| x, 1.0, 1.0, 10), 0.0);
    }
}
