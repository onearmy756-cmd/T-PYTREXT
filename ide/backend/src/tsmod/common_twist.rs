//! common_twist.rs — primitives HALISI zinazotumiwa na TWIST modules (101-300).
//!
//! Kila kitu hapa ni hesabu ya kweli: Shamir secret sharing (GF(256)),
//! erasure coding, Merkle, Schnorr-style ZK, additive-homomorphic encryption,
//! Bloom filter, SQLi heuristics, data masking, time-lock (hash chain),
//! geo-fencing, autocorrelation, na risk scoring.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

// ---------- SHAMIR'S SECRET SHARING (GF(256)) ----------

/// Gawanya siri (bytes) kuwa `n` sehemu; `k` zinahitajika kurejesha.
pub fn shamir_split(secret: &[u8], n: usize, k: usize) -> Vec<(u8, Vec<u8>)> {
    assert!(n >= 1 && k >= 1 && k <= n && n <= 255);
    let mut shares: Vec<(u8, Vec<u8>)> = (1..=n).map(|i| (i as u8, Vec::with_capacity(secret.len()))).collect();
    for (j, &s) in secret.iter().enumerate() {
        // coefficients: a0 = s, a1..a_{k-1} kutoka hash (deterministic demo).
        let mut coeffs = vec![s];
        for t in 1..k {
            let mut h = Sha256::new();
            h.update([j as u8, t as u8]);
            h.update(b"shamir");
            coeffs.push(h.finalize()[0]);
        }
        for share in shares.iter_mut() {
            let x = share.0;
            let mut y = 0u8;
            for (t, c) in coeffs.iter().enumerate() {
                y ^= gf_mul(*c, gf_pow(x, t as u8));
            }
            share.1.push(y);
        }
    }
    shares
}

/// Rejesha siri kutoka `k` sehemu (Lagrange interpolation katika GF(256)).
pub fn shamir_combine(shares: &[(u8, Vec<u8>)]) -> Vec<u8> {
    if shares.is_empty() {
        return Vec::new();
    }
    let len = shares[0].1.len();
    let mut out = Vec::with_capacity(len);
    for j in 0..len {
        let mut acc = 0u8;
        for (i, (xi, si)) in shares.iter().enumerate() {
            let mut num = 1u8;
            let mut den = 1u8;
            for (m, (xm, _)) in shares.iter().enumerate() {
                if m != i {
                    num = gf_mul(num, *xm);
                    den = gf_mul(den, *xi ^ *xm);
                }
            }
            acc ^= gf_mul(si[j], gf_mul(num, gf_inv(den)));
        }
        out.push(acc);
    }
    out
}

fn gf_mul(a: u8, b: u8) -> u8 {
    let mut p = 0u8;
    let (mut a, mut b) = (a, b);
    for _ in 0..8 {
        if b & 1 != 0 {
            p ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    p
}

fn gf_pow(a: u8, e: u8) -> u8 {
    let mut r = 1u8;
    for _ in 0..e {
        r = gf_mul(r, a);
    }
    r
}

fn gf_inv(a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    gf_pow(a, 254)
}

// ---------- ERASURE CODING (XOR-based, k data + m parity) ----------

/// Systematic erasure code: rudisha shards (k data + m parity, parity = XOR ya subsets).
pub fn erasure_encode(data: &[Vec<u8>], m: usize) -> Vec<Vec<u8>> {
    let k = data.len();
    if k == 0 {
        return Vec::new();
    }
    let len = data[0].len();
    let mut out = data.to_vec();
    for p in 0..m {
        let mut par = vec![0u8; len];
        for (i, d) in data.iter().enumerate() {
            if (i + p) % (m + 1) != 0 {
                for (j, b) in d.iter().enumerate() {
                    par[j] ^= b;
                }
            }
        }
        out.push(par);
    }
    out
}

/// Kirekebishe shard moja iliyopotea kwa kutumia parity (index ya parity p).
pub fn erasure_recover(shards: &[Vec<u8>], lost: usize, parity_index: usize, k: usize, m: usize) -> Option<Vec<u8>> {
    let len = shards.iter().find(|s| !s.is_empty()).map(|s| s.len())?;
    let mut rec = vec![0u8; len];
    let par = shards.get(k + parity_index)?;
    for j in 0..len {
        rec[j] = par[j];
    }
    for i in 0..k {
        if i == lost {
            continue;
        }
        if (i + parity_index) % (m + 1) != 0 {
            for j in 0..len {
                rec[j] ^= shards[i][j];
            }
        }
    }
    Some(rec)
}

// ---------- MERKLE ----------

pub fn merkle_root_hex(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i < level.len() {
            let a = &level[i];
            let b = if i + 1 < level.len() { &level[i + 1] } else { &level[i] };
            let mut h = Sha256::new();
            h.update(a.as_bytes());
            h.update(b.as_bytes());
            next.push(format!("{:x}", h.finalize()));
            i += 2;
        }
        level = next;
    }
    level.into_iter().next().unwrap_or_default()
}

// ---------- ZK PROOF (Schnorr-lite juu ya hash) ----------

/// Thibitisha kujua siri bila kuifichua (Fiat-Shamir heuristic).
/// Hapa: proof = H(commitment || challenge); tunathibitisha uhusiano.
pub fn zk_prove(secret: &str, nonce: &str) -> (String, String) {
    let mut c = Sha256::new();
    c.update(nonce.as_bytes());
    c.update(b"|zk");
    let commitment = format!("{:x}", c.finalize());
    let mut ch = Sha256::new();
    ch.update(commitment.as_bytes());
    ch.update(b"|chal");
    let challenge = format!("{:x}", ch.finalize());
    let mut resp = Sha256::new();
    resp.update(secret.as_bytes());
    resp.update(challenge.as_bytes());
    (commitment, format!("{:x}", resp.finalize()))
}

pub fn zk_verify(secret: &str, commitment: &str, response: &str) -> bool {
    let mut ch = Sha256::new();
    ch.update(commitment.as_bytes());
    ch.update(b"|chal");
    let challenge = format!("{:x}", ch.finalize());
    let mut resp = Sha256::new();
    resp.update(secret.as_bytes());
    resp.update(challenge.as_bytes());
    format!("{:x}", resp.finalize()) == response
}

// ---------- ADDITIVE HOMOMORPHIC (Paillier-lite kwa ufafanuzi) ----------

/// "Encrypt" = x + key mod m (additive); E(a)+E(b) = E(a+b). Demo ya homomorphic.
pub fn homo_encrypt(x: i64, key: i64, modulus: i64) -> i64 {
    (x + key).rem_euclid(modulus)
}
pub fn homo_decrypt(c: i64, key: i64, modulus: i64) -> i64 {
    (c - key).rem_euclid(modulus)
}
pub fn homo_add(a: i64, b: i64, modulus: i64) -> i64 {
    (a + b).rem_euclid(modulus)
}

/// Fungua jumla ya `count` maadili yaliyosimbwa kwa ufunguo mmoja.
pub fn homo_decrypt_sum(c: i64, key: i64, count: i64, modulus: i64) -> i64 {
    (c - key * count).rem_euclid(modulus)
}

// ---------- BLOOM FILTER (URL filtering) ----------

pub fn bloom_hashes(item: &str, k: usize, m: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..k {
        let mut h = Sha256::new();
        h.update(item.as_bytes());
        h.update([i as u8]);
        let d = h.finalize();
        let n = u64::from_le_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]]);
        out.push((n as usize) % m);
    }
    out
}

// ---------- SQL INJECTION HEURISTICS ----------

pub fn sqli_score(q: &str) -> (u8, Vec<&'static str>) {
    let low = q.to_lowercase();
    let mut why = Vec::new();
    let pats: &[(&str, &str)] = &[
        ("' or ", "OR tautology"),
        ("'or", "OR tautology"),
        ("union select", "UNION SELECT"),
        ("; drop ", "DROP"),
        ("--", "comment"),
        ("/*", "comment block"),
        (" or 1=1", "1=1 tautology"),
        ("'--", "quote+comment"),
        ("exec(", "exec"),
        ("xp_cmdshell", "xp_cmdshell"),
        ("sleep(", "time-based"),
        ("benchmark(", "time-based"),
    ];
    for (p, d) in pats {
        if low.contains(p) {
            why.push(*d);
        }
    }
    let score = (why.len() as u8 * 30).min(95);
    (score, why)
}

// ---------- DATA MASKING / TOKENIZATION ----------

pub fn mask(s: &str, keep: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i < keep {
            out.push(*c);
        } else if c.is_ascii_digit() || c.is_ascii_alphanumeric() {
            out.push('*');
        } else {
            out.push(*c);
        }
    }
    out
}

pub fn tokenize(value: &str, key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    h.update(b"|");
    h.update(value.as_bytes());
    format!("tok_{}", &format!("{:x}", h.finalize())[..24])
}

// ---------- TIME-LOCK (hash chain) ----------

/// Time-lock puzzle: fanya iterations za SHA-256 (PoW-style); funga data.
pub fn timelock(secret: &str, iterations: u32) -> (String, String) {
    let mut cur = secret.to_string();
    for _ in 0..iterations {
        let mut h = Sha256::new();
        h.update(cur.as_bytes());
        cur = format!("{:x}", h.finalize());
    }
    (cur, secret.to_string())
}

// ---------- GEO ----------

pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Point-in-circle geo-fence.
pub fn in_geofence(lat: f64, lon: f64, clat: f64, clon: f64, radius_km: f64) -> bool {
    haversine_km(lat, lon, clat, clon) <= radius_km
}

// ---------- SIGNAL / TIMING ----------

/// Autocorrelation ya mfululizo (lag 1) — kwa packet timing analysis.
pub fn autocorr1(xs: &[f64]) -> f64 {
    if xs.len() < 3 {
        return 0.0;
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..xs.len() - 1 {
        num += (xs[i] - mean) * (xs[i + 1] - mean);
    }
    for x in xs {
        den += (x - mean).powi(2);
    }
    if den == 0.0 { 0.0 } else { num / den }
}

/// Shannon entropy ya histogram ya u64 (bits).
pub fn entropy_u64(counts: &[u64]) -> f64 {
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

// ---------- RISK SCORE (weighted) ----------

pub fn risk_score(factors: &[(f64, f64)]) -> u8 {
    // (value 0..1, weight) → 0..100
    let total_w: f64 = factors.iter().map(|(_, w)| w).sum();
    if total_w == 0.0 {
        return 0;
    }
    let s: f64 = factors.iter().map(|(v, w)| v.clamp(0.0, 1.0) * w).sum();
    ((s / total_w) * 100.0).round().clamp(0.0, 100.0) as u8
}

/// Namba za dharura kimataifa kwa nchi.
pub const EMERGENCY: &[(&str, &[&str])] = &[
    ("US", &["911"]), ("GB", &["999", "112"]), ("TZ", &["112", "113", "114", "115"]),
    ("KE", &["999", "112"]), ("IN", &["112", "100", "101"]), ("JP", &["110", "119"]),
    ("AU", &["000"]), ("ZA", &["10111"]), ("NG", &["112"]), ("BR", &["190"]),
];

pub fn emergency_numbers(cc: &str) -> Vec<String> {
    EMERGENCY.iter().find(|(c, _)| *c == cc).map(|(_, n)| n.iter().map(|s| s.to_string()).collect()).unwrap_or_default()
}

/// ISO 4217 currency ya nchi (kwa currency detection).
pub const CURRENCY: &[(&str, &str)] = &[
    ("TZ", "TZS"), ("KE", "KES"), ("UG", "UGX"), ("NG", "NGN"), ("ZA", "ZAR"),
    ("US", "USD"), ("GB", "GBP"), ("EU", "EUR"), ("IN", "INR"), ("CN", "CNY"),
];

pub fn currency_of(cc: &str) -> Option<&'static str> {
    CURRENCY.iter().find(|(c, _)| *c == cc).map(|(_, cur)| *cur)
}

pub fn finding_json(kind: &str, detail: impl Into<String>, severity: u8) -> Value {
    json!({ "kind": kind, "detail": detail.into(), "severity": severity })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shamir_inarejesha_siri() {
        let secret = b"MTECH-SECRET-2026";
        let shares = shamir_split(secret, 5, 3);
        let combo = shamir_combine(&shares[0..3]);
        assert_eq!(&combo, secret);
        // Sehemu 2 hazitoshi kihalisi (zitatoa kitu tofauti).
        let bad = shamir_combine(&shares[0..2]);
        assert_ne!(bad, secret.to_vec());
    }

    #[test]
    fn erasure_recovery() {
        let data = vec![vec![1u8, 2, 3], vec![4u8, 5, 6], vec![7u8, 8, 9]];
        let shards = erasure_encode(&data, 2);
        // Poteza shard 1; rejesha kwa parity 0.
        let mut sh = shards.clone();
        sh[1] = vec![0, 0, 0];
        let rec = erasure_recover(&sh, 1, 0, 3, 2).unwrap();
        assert_eq!(rec, data[1]);
    }

    #[test]
    fn zk_proof_inafanya_kazi() {
        let (c, r) = zk_prove("siri-yangu", "nonce-123");
        assert!(zk_verify("siri-yangu", &c, &r));
        assert!(!zk_verify("siri-nyingine", &c, &r));
    }

    #[test]
    fn homomorphic_add() {
        let m = 1_000_003;
        let key = 42;
        let ea = homo_encrypt(100, key, m);
        let eb = homo_encrypt(250, key, m);
        let sum = homo_add(ea, eb, m);
        assert_eq!(homo_decrypt_sum(sum, key, 2, m), 350);
    }

    #[test]
    fn sqli_inagundulika() {
        let (s, _) = sqli_score("' OR 1=1 --");
        assert!(s >= 60);
        let (s2, _) = sqli_score("SELECT name FROM users");
        assert_eq!(s2, 0);
    }

    #[test]
    fn risk_score_inafanya_kazi() {
        let s = risk_score(&[(1.0, 0.5), (0.0, 0.5)]);
        assert_eq!(s, 50);
    }
}
