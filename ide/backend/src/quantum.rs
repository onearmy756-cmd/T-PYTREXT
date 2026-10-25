//! quantum.rs — QUANTUM ENGINE YETU (Rust mtupu, hakuna library za nje).
//!
//! Statevector simulator: n qubits (≤14 → 16,384 amplitudes), gates:
//! h x y z s t rx ry rz cnot cz swap, p + measurement (sampling).
//! Pia QRNG: vipande vya biti kutoka matokeo ya measurement ya |+>.
//! API: POST /api/quantum/run {qubits, ops, shots} + /api/quantum/random {bits}.

use rand::Rng;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const MAX_QUBITS: usize = 14;
pub const MAX_SHOTS: usize = 4096;

// ---------- NAMBA TARAKAWIA (complex) ----------
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct C64 {
    pub re: f64,
    pub im: f64,
}

impl C64 {
    pub fn new(re: f64, im: f64) -> Self {
        C64 { re, im }
    }

    fn add(self, o: C64) -> C64 {
        C64::new(self.re + o.re, self.im + o.im)
    }

    fn mul(self, o: C64) -> C64 {
        C64::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }

    fn scale(self, s: f64) -> C64 {
        C64::new(self.re * s, self.im * s)
    }

    pub fn abs2(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

// ---------- HALI (statevector) ----------
pub struct State {
    pub nq: usize,
    pub amp: Vec<C64>,
}

type G2 = [[C64; 2]; 2];

impl State {
    pub fn new(nq: usize) -> Result<Self, String> {
        if nq == 0 || nq > MAX_QUBITS {
            return Err(format!("qubits lazima ziwe 1..={}", MAX_QUBITS));
        }
        let mut amp = vec![C64::new(0.0, 0.0); 1usize << nq];
        amp[0] = C64::new(1.0, 0.0); // |00...0>
        Ok(State { nq, amp })
    }

    /// Bit ya qubit q kwa index ya msingi: qubit0 = MSB.
    fn mask(&self, q: usize) -> usize {
        1usize << (self.nq - 1 - q)
    }

    /// Gate moja (2x2) kwenye target — kila pair (base, base|mask).
    fn apply1(&mut self, g: G2, t: usize) {
        let m = self.mask(t);
        let len = self.amp.len();
        for base in 0..len {
            if base & m != 0 {
                continue;
            }
            let i0 = base;
            let i1 = base | m;
            let a0 = self.amp[i0];
            let a1 = self.amp[i1];
            self.amp[i0] = g[0][0].mul(a0).add(g[0][1].mul(a1));
            self.amp[i1] = g[1][0].mul(a0).add(g[1][1].mul(a1));
        }
    }

    pub fn h(&mut self, t: usize) {
        let s = 1.0 / 2.0f64.sqrt();
        self.apply1(
            [
                [C64::new(s, 0.0), C64::new(s, 0.0)],
                [C64::new(s, 0.0), C64::new(-s, 0.0)],
            ],
            t,
        );
    }

    pub fn pauli_x(&mut self, t: usize) {
        self.apply1(
            [
                [C64::new(0.0, 0.0), C64::new(1.0, 0.0)],
                [C64::new(1.0, 0.0), C64::new(0.0, 0.0)],
            ],
            t,
        );
    }

    pub fn pauli_y(&mut self, t: usize) {
        self.apply1(
            [
                [C64::new(0.0, 0.0), C64::new(0.0, -1.0)],
                [C64::new(0.0, 1.0), C64::new(0.0, 0.0)],
            ],
            t,
        );
    }

    pub fn pauli_z(&mut self, t: usize) {
        self.apply1(
            [
                [C64::new(1.0, 0.0), C64::new(0.0, 0.0)],
                [C64::new(0.0, 0.0), C64::new(-1.0, 0.0)],
            ],
            t,
        );
    }

    pub fn phase_s(&mut self, t: usize) {
        self.apply1(
            [
                [C64::new(1.0, 0.0), C64::new(0.0, 0.0)],
                [C64::new(0.0, 0.0), C64::new(0.0, 1.0)],
            ],
            t,
        );
    }

    pub fn phase_t(&mut self, t: usize) {
        let s = 1.0 / 2.0f64.sqrt();
        self.apply1(
            [
                [C64::new(1.0, 0.0), C64::new(0.0, 0.0)],
                [C64::new(0.0, 0.0), C64::new(s, s)],
            ],
            t,
        );
    }

    pub fn rx(&mut self, t: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        self.apply1(
            [
                [C64::new(c, 0.0), C64::new(0.0, -s)],
                [C64::new(0.0, -s), C64::new(c, 0.0)],
            ],
            t,
        );
    }

    pub fn ry(&mut self, t: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        self.apply1(
            [
                [C64::new(c, 0.0), C64::new(-s, 0.0)],
                [C64::new(s, 0.0), C64::new(c, 0.0)],
            ],
            t,
        );
    }

    pub fn rz(&mut self, t: usize, theta: f64) {
        // Rz = diag(e^{-iθ/2}, e^{iθ/2})
        let (c, s) = ((theta / 2.0).cos(), (theta / 2.0).sin());
        self.apply1(
            [
                [C64::new(c, -s), C64::new(0.0, 0.0)],
                [C64::new(0.0, 0.0), C64::new(c, s)],
            ],
            t,
        );
    }

    /// CNOT: kama control = 1 → PINDULI target PEKEE (`i ^ tm`).
    /// (Kutupa `^ cm` pia kulibadilisha control qubit — Bell state ikaharibika:
    ///  |00>+|10> ilitoa prob kwenye 0 na 1 badala ya 0 na 3.)
    pub fn cnot(&mut self, c: usize, t: usize) {
        let cm = self.mask(c);
        let tm = self.mask(t);
        let len = self.amp.len();
        for i in 0..len {
            if i & cm != 0 && i & tm == 0 {
                let j = i ^ tm;
                self.amp.swap(i, j);
            }
        }
    }

    pub fn cz(&mut self, c: usize, t: usize) {
        let cm = self.mask(c);
        let tm = self.mask(t);
        for i in 0..self.amp.len() {
            if i & cm != 0 && i & tm != 0 {
                self.amp[i] = self.amp[i].scale(-1.0);
            }
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        let am = self.mask(a);
        let bm = self.mask(b);
        for i in 0..self.amp.len() {
            if i & am != 0 && i & bm == 0 {
                let j = i ^ am ^ bm;
                self.amp.swap(i, j);
            }
        }
    }

    /// Wasifu wa probabilities (index → p, zisizo chini ya 1e-9).
    pub fn probs(&self) -> Vec<(usize, f64)> {
        self.amp
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.abs2()))
            .filter(|(_, p)| *p > 1e-9)
            .collect()
    }

    pub fn norm_ok(&self) -> bool {
        let s: f64 = self.amp.iter().map(|a| a.abs2()).sum();
        (s - 1.0).abs() < 1e-9
    }

    /// Sampuli ya `shots` matokeo (measurement ya qubit zote).
    pub fn measure_counts(&self, shots: usize) -> BTreeMap<String, usize> {
        let probs = self.amp.iter().map(|a| a.abs2()).collect::<Vec<_>>();
        let mut rng = rand::thread_rng();
        let mut out: BTreeMap<String, usize> = BTreeMap::new();
        for _ in 0..shots {
            let r: f64 = rng.gen();
            let mut acc = 0.0;
            let mut idx = probs.len() - 1;
            for (i, p) in probs.iter().enumerate() {
                acc += p;
                if r < acc {
                    idx = i;
                    break;
                }
            }
            let bits: String = (0..self.nq)
                .map(|q| if idx & self.mask(q) != 0 { '1' } else { '0' })
                .collect();
            *out.entry(bits).or_insert(0) += 1;
        }
        out
    }
}

// ---------- DISPATCHER WA OPS (JSON) ----------
fn qcheck(q: usize, nq: usize) -> Result<(), String> {
    if q >= nq {
        return Err(format!("qubit {} haipo (0..{})", q, nq - 1));
    }
    Ok(())
}

/// Endesha mfululizo wa gates: ops = [{gate, target, control?, angle?}]
pub fn run(nq: usize, ops: &[Value], _shots: usize) -> Result<State, String> {
    let mut st = State::new(nq)?;
    for (k, op) in ops.iter().enumerate() {
        let gate = op["gate"].as_str().unwrap_or("");
        let target = op["target"].as_u64().unwrap_or(u64::MAX) as usize;
        let control = op["control"].as_u64().map(|c| c as usize);
        let angle = op["angle"].as_f64().unwrap_or(0.0);
        let ctx = |e: String| format!("gate #{} ({}): {}", k + 1, gate, e);
        qcheck(target, nq).map_err(|e| ctx(e))?;
        match gate {
            "h" => st.h(target),
            "x" => st.pauli_x(target),
            "y" => st.pauli_y(target),
            "z" => st.pauli_z(target),
            "s" => st.phase_s(target),
            "t" => st.phase_t(target),
            "rx" => st.rx(target, angle),
            "ry" => st.ry(target, angle),
            "rz" => st.rz(target, angle),
            "cnot" | "cx" => {
                let c = control.ok_or_else(|| ctx("inahitaji control".into()))?;
                qcheck(c, nq).map_err(|e| ctx(e))?;
                st.cnot(c, target);
            }
            "cz" => {
                let c = control.ok_or_else(|| ctx("inahitaji control".into()))?;
                qcheck(c, nq).map_err(|e| ctx(e))?;
                st.cz(c, target);
            }
            "swap" => {
                let b =
                    control.ok_or_else(|| ctx("swap inahitaji qubit ya pili (control)".into()))?;
                qcheck(b, nq).map_err(|e| ctx(e))?;
                st.swap(target, b);
            }
            _ => {
                return Err(ctx(
                    "gate isiyojulikani (h x y z s t rx ry rz cnot cz swap)".into(),
                ))
            }
        }
    }
    Ok(st)
}

// ---------- API ----------
pub fn api_run(p: &Value) -> Value {
    let nq = p["qubits"].as_u64().unwrap_or(2) as usize;
    let shots = p["shots"]
        .as_u64()
        .unwrap_or(256)
        .clamp(1, MAX_SHOTS as u64) as usize;
    let ops = match p["ops"] {
        Value::Array(ref a) => a.clone(),
        _ => return json!({ "ok": false, "error": "ops lazima iwe orodha ya gates" }),
    };
    let st = match run(nq, &ops, shots) {
        Ok(s) => s,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let probs: Vec<Value> = st
        .probs()
        .iter()
        .map(|(i, pr)| {
            let bits: String = (0..st.nq)
                .map(|q| if i & st.mask(q) != 0 { '1' } else { '0' })
                .collect();
            json!({ "state": format!("|{}>", bits), "p": pr })
        })
        .collect();
    let counts = st.measure_counts(shots);
    let measured: Vec<Value> = counts
        .iter()
        .map(|(b, c)| json!({ "state": format!("|{}>", b), "count": c }))
        .collect();
    json!({
        "ok": true,
        "qubits": nq,
        "gates": ops.len(),
        "normalized": st.norm_ok(),
        "probabilities": probs,
        "measured": measured,
        "shots": shots,
        "note_sw": "Injini ya Quantum ya MTECH OS — statevector halisi + measurement sampling."
    })
}

/// QRNG: biti kutoka measurement ya |+> (kila biti = Hadamard + sampuli).
pub fn api_qrng(p: &Value) -> Value {
    let bits = p["bits"].as_u64().unwrap_or(64).clamp(1, 8192) as usize;
    let mut rng = rand::thread_rng();
    let mut out = String::with_capacity(bits);
    let mut ones = 0usize;
    for _ in 0..bits {
        // |0> → H → sampuli ya measurement (p=0.5 kila upande)
        let mut st = State::new(1).expect("nq=1 ni halali");
        st.h(0);
        let r: f64 = rng.gen();
        let p1 = st.amp[1].abs2();
        let b = if r < p1 { '1' } else { '0' };
        if b == '1' {
            ones += 1;
        }
        out.push(b);
    }
    let hex: String = out
        .chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| {
            format!(
                "{:x}",
                usize::from_str_radix(c.iter().collect::<String>().as_str(), 2).unwrap_or(0)
            )
        })
        .collect();
    json!({
        "ok": true,
        "bits": out,
        "count": bits,
        "ones": ones,
        "zeros": bits - ones,
        "hex": hex,
        "note_sw": "Namba za nasibu kutoka quantum measurement (Hadamard + sampuli)."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bell_state_ipatikana() {
        let st = run(
            2,
            &[
                json!({ "gate": "h", "target": 0 }),
                json!({ "gate": "cnot", "control": 0, "target": 1 }),
            ],
            100,
        )
        .unwrap();
        assert!(st.norm_ok());
        let probs: BTreeMap<usize, f64> = st.probs().into_iter().collect();
        // |00> = 0.5, |11> = 0.5 (qubit0 = MSB → index 0 na 3)
        assert!((probs[&0] - 0.5).abs() < 1e-9);
        assert!((probs[&3] - 0.5).abs() < 1e-9);
        assert_eq!(probs.len(), 2);
    }

    #[test]
    fn gates_x_na_h_vina_sahihi() {
        let st = run(1, &[json!({ "gate": "x", "target": 0 })], 10).unwrap();
        let probs: BTreeMap<usize, f64> = st.probs().into_iter().collect();
        assert!((probs[&1] - 1.0).abs() < 1e-9); // |1>
        let st2 = run(1, &[json!({ "gate": "h", "target": 0 })], 10).unwrap();
        let p2: BTreeMap<usize, f64> = st2.probs().into_iter().collect();
        assert!((p2[&0] - 0.5).abs() < 1e-9 && (p2[&1] - 0.5).abs() < 1e-9);
        // z gate haibadilishi probability
        let st3 = run(
            1,
            &[
                json!({ "gate": "h", "target": 0 }),
                json!({ "gate": "z", "target": 0 }),
            ],
            10,
        )
        .unwrap();
        let p3: BTreeMap<usize, f64> = st3.probs().into_iter().collect();
        assert!((p3[&0] - 0.5).abs() < 1e-9 && (p3[&1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn measurement_hifanya_jumla_kuwa_shots() {
        let st = run(2, &[json!({ "gate": "h", "target": 0 })], 500).unwrap();
        let counts = st.measure_counts(500);
        let total: usize = counts.values().sum();
        assert_eq!(total, 500);
    }

    #[test]
    fn errors_vinazoshikilia() {
        assert!(run(1, &[json!({ "gate": "evil", "target": 0 })], 10).is_err());
        assert!(run(1, &[json!({ "gate": "h", "target": 5 })], 10).is_err());
        assert!(run(1, &[json!({ "gate": "cnot", "target": 0 })], 10).is_err()); // control inahitajika
        assert!(State::new(0).is_err());
        assert!(State::new(MAX_QUBITS + 1).is_err());
        let v = api_qrng(&json!({ "bits": 16 }));
        assert_eq!(v["ok"], true);
        assert_eq!(v["bits"].as_str().unwrap().len(), 16);
    }
}
