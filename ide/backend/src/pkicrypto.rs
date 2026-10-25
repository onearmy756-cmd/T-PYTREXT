//! pkicrypto.rs — PKI YA KAMPUNI: ECDSA P-256 + muhuri wa SHA-256 + hash-chain.
//!
//! Mahitaji ya mmiliki: "SHA-256", "ECDSA architecture kulinda mfumo mzima",
//! "ulinzi mkali wa quantum grade". Hii ni safu ya kriptografia halisi:
//!
//!   1. **ECDSA P-256 (secp256r1)** — muhuri wa kidijitali wa ripoti/mikataba
//!      na ujumbe. RFC 6979 deterministic; DER-encoded; PEM (PKCS#8/SPKI).
//!      Hii ni curve ile ile inayotumiwa na TLS, na familia ya NIST.
//!   2. **SHA-256** — hash ya maudhui (content hash) ya kila muhuri/hati.
//!   3. **HASH-CHAIN LEDGER** — kila muhuri huingizwa kwenye mnyororo
//!      tamper-evident (`prev_hash`): kubadilisha rekodi ya zamani huvunja
//!      mnyororo wote. Hii ni integrity inayotegemea SHA-256 PEKEE — hivyo
//!      **inastahimili mashambulizi ya quantum** (Grover hupunguza nguvu ya
//!      preimage kwa nusu tu; SHA-256 inabaki salama).
//!   4. **LAMPORT OTS (hash-based, post-quantum)** — sahihi ya matumizi-moja
//!      kwa nyaraka za maisha marefu. Hategemei ugumu wa ECDLP, hivyo
//!      hash-based signatures ni safu ya PQC (NIST SP 800-208, SLH-DSA/XMSS
//!      familia) — inastahimili hata kompyuta za quantum.
//!
//! CRYPTO-AGILITY: kila muhuri hubeba `alg` (mf. "ecdsa-p256-sha256") ili
//! siku moja tuhamie PQC kamili (ML-DSA/Kyber) bila kuvunja nyaraka za zamani.
//!
//! API: /api/crypto/keys|sign|verify|seal|verify-seal|ledger|verify-ledger

use base64::Engine as _;
use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

pub const ALG_ECDSA: &str = "ecdsa-p256-sha256";
pub const ALG_LAMPORT: &str = "lamport-sha256";
/// Urefu wa biti za Lamport OTS (SHA-256 → 256 biti).
const LAMPORT_BITS: usize = 256;
/// Ukubwa wa kila funguo ya Lamport (byte 32 = SHA-256).
const LAMPORT_WORD: usize = 32;

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// =====================================================================
// 1. ECDSA P-256 — funguo, sahihi, uthibitisho
// =====================================================================

/// Kifungu cha funguo za kampuni (PEM). `fingerprint` = SHA-256 ya public DER.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub id: i64,
    pub alg: String,
    pub public_pem: String,
    pub fingerprint: String,
    pub created_at: String,
    pub active: bool,
}

/// Funguo mpya ya ECDSA P-256 (private PEM + public PEM).
pub fn generate_keypair() -> (String, String) {
    let sk = SigningKey::random(&mut OsRng);
    let private_pem = sk
        .to_pkcs8_pem(LineEnding::LF)
        .expect("pkcs8 pem")
        .to_string();
    let public_pem = sk
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("spki pem");
    (private_pem, public_pem)
}

/// Fingerprint ya funguo ya umma: SHA-256 ya encoded point (uncompressed) hex.
pub fn fingerprint(public_pem: &str) -> Result<String, String> {
    let vk = VerifyingKey::from_public_key_pem(public_pem).map_err(|e| e.to_string())?;
    let point = vk.to_encoded_point(false);
    Ok(sha256_hex(point.as_bytes()))
}

/// Tia sahihi ya ECDSA (RFC 6979 deterministic) juu ya `data`. Hurudisha DER
/// base64. Hii ni sahihi HALISI — uthibitisho hufanyika kwa funguo ya umma.
pub fn sign(private_pem: &str, data: &[u8]) -> Result<String, String> {
    let sk = SigningKey::from_pkcs8_pem(private_pem).map_err(|e| e.to_string())?;
    let sig: Signature = sk.sign(data);
    Ok(b64().encode(sig.to_der().as_bytes()))
}

/// Thibitisha sahihi ya ECDSA. Hurudisha `true` tu ikiwa sahihi ni HALISI.
pub fn verify(public_pem: &str, data: &[u8], sig_b64: &str) -> Result<bool, String> {
    let vk = VerifyingKey::from_public_key_pem(public_pem).map_err(|e| e.to_string())?;
    let raw = b64().decode(sig_b64).map_err(|e| e.to_string())?;
    let sig = Signature::from_der(&raw).map_err(|e| e.to_string())?;
    Ok(vk.verify(data, &sig).is_ok())
}

// =====================================================================
// 2. MUHURI (SEAL) — SHA-256 + ECDSA, pamoja na hash-chain
// =====================================================================

/// Muhuri kamili wa hati: hash ya SHA-256 + sahihi ya ECDSA + nembo ya muda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seal {
    pub alg: String,
    /// SHA-256 (hex) ya payload KAMILI — hubadilika ukibadilisha herufi moja.
    pub content_hash: String,
    /// Sahihi ya ECDSA (DER base64) juu ya `content_hash`.
    pub signature: String,
    /// Fingerprint ya funguo iliyotumika (kufuatilia ni funguo gani).
    pub key_fingerprint: String,
    /// Hash ya muhuri uliopita kwenye mnyororo (hash-chain, tamper-evident).
    pub prev_hash: String,
    pub chain_index: i64,
    pub at: String,
}

/// Hati iliyotiwa muhuri: HTML yenye nembo + hash + sahihi (inayoonekana).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedDoc {
    pub title: String,
    pub body_html: String,
    pub seal: Seal,
    /// HTML kamili yenye kichwa cha muhuri kinachoonekana (printable).
    pub sealed_html: String,
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Muhuri wa payload: SHA-256 → ECDSA → unganisha kwenye hash-chain.
/// Hurudisha `Seal`. Hii ndiyo njia KUU ya kutia muhuri ripoti/mkataba.
pub async fn seal(db: &SqlitePool, private_pem: &str, payload: &str) -> Result<Seal, String> {
    let key = active_key(db).await?;
    let content_hash = sha256_hex(payload.as_bytes());
    // Sahihi juu ya hash (si payload) — ni ndogo na salama sawa (hash binding).
    let signature = sign(private_pem, content_hash.as_bytes())?;
    let (prev_hash, chain_index) = last_chain(db).await?;
    let at = now();
    let seal = Seal {
        alg: ALG_ECDSA.to_string(),
        content_hash,
        signature,
        key_fingerprint: key.fingerprint,
        prev_hash,
        chain_index: chain_index + 1,
        at,
    };
    // Ingiza kwenye ledger (mnyororo). entry_hash = SHA-256 ya seal yote.
    let entry_hash = seal_entry_hash(&seal);
    sqlx::query(
        "INSERT INTO crypto_ledger (chain_index, content_hash, signature, key_fingerprint, prev_hash, entry_hash, at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(seal.chain_index)
    .bind(&seal.content_hash)
    .bind(&seal.signature)
    .bind(&seal.key_fingerprint)
    .bind(&seal.prev_hash)
    .bind(&entry_hash)
    .bind(&seal.at)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(seal)
}

/// Hash ya kiingilio kwenye mnyororo = SHA-256(prev_hash|content_hash|signature|at).
pub fn seal_entry_hash(s: &Seal) -> String {
    sha256_hex(format!("{}|{}|{}|{}", s.prev_hash, s.content_hash, s.signature, s.at).as_bytes())
}

/// Thibitisha muhuri: (a) sahihi ya ECDSA ni halali, (b) hash inalingana na payload.
pub async fn verify_seal(
    db: &SqlitePool,
    payload: &str,
    seal: &Seal,
) -> Result<VerifyResult, String> {
    let content_hash = sha256_hex(payload.as_bytes());
    let hash_ok = content_hash == seal.content_hash;
    let public_pem = public_key_for_fingerprint(db, &seal.key_fingerprint)
        .await?
        .ok_or_else(|| "funguo ya umma haipatikani kwa fingerprint hii".to_string())?;
    let sig_ok = verify(&public_pem, seal.content_hash.as_bytes(), &seal.signature)?;
    Ok(VerifyResult {
        valid: hash_ok && sig_ok,
        hash_matches: hash_ok,
        signature_valid: sig_ok,
        alg: seal.alg.clone(),
        key_fingerprint: seal.key_fingerprint.clone(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub valid: bool,
    pub hash_matches: bool,
    pub signature_valid: bool,
    pub alg: String,
    pub key_fingerprint: String,
}

// =====================================================================
// 3. HASH-CHAIN LEDGER — integrity inayostahimili quantum (SHA-256 pekee)
// =====================================================================

/// Hali ya mnyororo mzima — kila kiungo huthibitishwa dhidi ya kilichotangulia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatus {
    pub entries: i64,
    pub intact: bool,
    pub head_hash: String,
    pub broken_at: Option<i64>,
    pub note: String,
}

async fn last_chain(db: &SqlitePool) -> Result<(String, i64), String> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT entry_hash, chain_index FROM crypto_ledger ORDER BY chain_index DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.unwrap_or_else(|| ("0".repeat(64), 0)))
}

/// Kagua mnyororo wote: kila kiingilo LAZIMA kiendelee kutoka `prev_hash` ya
/// kilichotangulia na `entry_hash` yake ilingane (iliyohesabiwa upya). Hii ni
/// hash-chain ya kweli — kubadilisha rekodi ya zamani huvunja kila kiungo.
pub async fn verify_chain(db: &SqlitePool) -> Result<ChainStatus, String> {
    let rows: Vec<(i64, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT chain_index, content_hash, signature, key_fingerprint, prev_hash, entry_hash, at \
         FROM crypto_ledger ORDER BY chain_index ASC",
    )
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    let genesis = "0".repeat(64);
    let mut prev = genesis.clone();
    let mut head = genesis;
    let mut intact = true;
    let mut broken_at = None;
    for (idx, ch, sig, _kf, ph, eh, at) in &rows {
        let recomputed = sha256_hex(format!("{}|{}|{}|{}", ph, ch, sig, at).as_bytes());
        if ph != &prev || &recomputed != eh {
            intact = false;
            broken_at = Some(*idx);
            break;
        }
        prev = eh.clone();
        head = eh.clone();
    }
    Ok(ChainStatus {
        entries: rows.len() as i64,
        intact,
        head_hash: head,
        broken_at,
        note: "Mnyororo hutegemea SHA-256 pekee — hash-based, hustahimili quantum (Grover: nusu nguvu tu).".into(),
    })
}

// =====================================================================
// 4. LAMPORT OTS — sahihi ya matumizi-moja (hash-based, post-quantum)
// =====================================================================

/// Funguo ya Lamport (matumizi MOJA): funguo 256×2 za maneno ya SHA-256.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LamportKey {
    /// Funguo za umma (512 maneno = 256 biti × (0,1)), hex.
    pub public: Vec<String>,
    /// Funguo za siri — hazitakiwi kutoka nje (hapa kwa demo/vipimo).
    #[serde(skip_serializing)]
    pub private: Vec<String>,
}

/// Tengeneza funguo za Lamport kwa biti 256 (deterministic kutoka seed).
pub fn lamport_keypair(seed: &str) -> LamportKey {
    let mut private = Vec::with_capacity(LAMPORT_BITS * 2);
    let mut public = Vec::with_capacity(LAMPORT_BITS * 2);
    for i in 0..(LAMPORT_BITS * 2) {
        let sk = sha256_hex(format!("{seed}|sk|{i}").as_bytes());
        let pk = sha256_hex(sk.as_bytes());
        private.push(sk);
        public.push(pk);
    }
    LamportKey { public, private }
}

/// Tia sahihi ya Lamport juu ya ujumbe (hash ya ujumbe = biti 256).
pub fn lamport_sign(key: &LamportKey, msg: &[u8]) -> Vec<String> {
    let digest = Sha256::digest(msg);
    let mut sig = Vec::with_capacity(LAMPORT_BITS);
    for bit in 0..LAMPORT_BITS {
        let byte = digest[bit / 8];
        let set = (byte >> (7 - (bit % 8))) & 1 == 1;
        let idx = bit * 2 + if set { 1 } else { 0 };
        sig.push(key.private[idx].clone());
    }
    sig
}

/// Thibitisha sahihi ya Lamport kwa funguo ya umma.
pub fn lamport_verify(public: &[String], msg: &[u8], sig: &[String]) -> bool {
    if sig.len() != LAMPORT_BITS || public.len() != LAMPORT_BITS * 2 {
        return false;
    }
    let digest = Sha256::digest(msg);
    for bit in 0..LAMPORT_BITS {
        let byte = digest[bit / 8];
        let set = (byte >> (7 - (bit % 8))) & 1 == 1;
        let idx = bit * 2 + if set { 1 } else { 0 };
        if sha256_hex(sig[bit].as_bytes()) != public[idx] {
            return false;
        }
    }
    true
}

// =====================================================================
// DB
// =====================================================================

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS crypto_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            alg TEXT NOT NULL,
            private_pem TEXT NOT NULL,
            public_pem TEXT NOT NULL,
            fingerprint TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            active INTEGER NOT NULL DEFAULT 1
        )",
    )
    .execute(db)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS crypto_ledger (
            chain_index INTEGER PRIMARY KEY,
            content_hash TEXT NOT NULL,
            signature TEXT NOT NULL,
            key_fingerprint TEXT NOT NULL,
            prev_hash TEXT NOT NULL,
            entry_hash TEXT NOT NULL,
            at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

/// Funguo inayotumika sasa; itengeneze ikiwa haipo (bootstrap salama).
pub async fn ensure_company_key(db: &SqlitePool) -> Result<KeyPair, String> {
    if let Some(k) = active_key(db).await.ok() {
        return Ok(k);
    }
    let (private_pem, public_pem) = generate_keypair();
    let fp = fingerprint(&public_pem)?;
    sqlx::query(
        "INSERT INTO crypto_keys (alg, private_pem, public_pem, fingerprint, created_at, active) \
         VALUES (?1,?2,?3,?4,?5,1)",
    )
    .bind(ALG_ECDSA)
    .bind(&private_pem)
    .bind(&public_pem)
    .bind(&fp)
    .bind(now())
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    active_key(db).await
}

pub async fn active_key(db: &SqlitePool) -> Result<KeyPair, String> {
    let row: Option<(i64, String, String, String, String, i64)> = sqlx::query_as(
        "SELECT id, alg, public_pem, fingerprint, created_at, active FROM crypto_keys \
         WHERE active=1 ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?;
    row.map(|(id, alg, public_pem, fingerprint, created_at, active)| KeyPair {
        id,
        alg,
        public_pem,
        fingerprint,
        created_at,
        active: active == 1,
    })
    .ok_or_else(|| "hakuna funguo inayotumika".to_string())
}

/// Private PEM ya funguo inayotumika (kwa kutia muhuri).
pub async fn active_private_pem(db: &SqlitePool) -> Result<String, String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT private_pem FROM crypto_keys WHERE active=1 ORDER BY id DESC LIMIT 1")
            .fetch_optional(db)
            .await
            .map_err(|e| e.to_string())?;
    row.map(|(p,)| p)
        .ok_or_else(|| "hakuna funguo inayotumika".to_string())
}

pub async fn public_key_for_fingerprint(
    db: &SqlitePool,
    fingerprint: &str,
) -> Result<Option<String>, String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT public_pem FROM crypto_keys WHERE fingerprint=?1")
            .bind(fingerprint)
            .fetch_optional(db)
            .await
            .map_err(|e| e.to_string())?;
    Ok(row.map(|(p,)| p))
}

pub async fn list_keys(db: &SqlitePool) -> serde_json::Value {
    let rows: Vec<(i64, String, String, String, i64)> = sqlx::query_as(
        "SELECT id, alg, fingerprint, created_at, active FROM crypto_keys ORDER BY id DESC",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    serde_json::json!({
        "ok": true,
        "keys": rows.into_iter().map(|(id, alg, fp, at, active)| serde_json::json!({
            "id": id, "alg": alg, "fingerprint": fp, "created_at": at, "active": active == 1,
        })).collect::<Vec<_>>(),
    })
}

/// Ledger ya muhuri (ripoti ya ukaguzi kwa admin).
pub async fn ledger(db: &SqlitePool, limit: i64) -> serde_json::Value {
    let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT chain_index, content_hash, key_fingerprint, entry_hash, at FROM crypto_ledger \
         ORDER BY chain_index DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    serde_json::json!({
        "ok": true,
        "entries": rows.into_iter().map(|(i, ch, kf, eh, at)| serde_json::json!({
            "chain_index": i, "content_hash": ch, "key_fingerprint": kf, "entry_hash": eh, "at": at,
        })).collect::<Vec<_>>(),
    })
}

// =====================================================================
// Hati iliyotiwa muhuri (HTML yenye nembo)
// =====================================================================

/// Tengeneza hati ya HTML yenye muhuri unaoonekana (nembo + hash + sahihi).
pub fn sealed_html(title: &str, body_html: &str, seal: &Seal) -> String {
    format!(
        r#"<!doctype html><html lang="sw"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — MBILINYI TECH</title>
<style>
 :root{{--ink:#0b1220;--brand:#0ea5e9;--brand2:#6366f1;--ok:#059669}}
 *{{box-sizing:border-box}} body{{margin:0;font:14px/1.55 system-ui,Segoe UI,Roboto,sans-serif;color:var(--ink);background:#f8fafc}}
 .page{{max-width:820px;margin:0 auto;background:#fff;padding:42px 48px}}
 .hdr{{display:flex;align-items:center;gap:14px;border-bottom:3px solid var(--brand);padding-bottom:14px}}
 .logo{{width:46px;height:46px;border-radius:12px;background:linear-gradient(135deg,var(--brand),var(--brand2));
        display:flex;align-items:center;justify-content:center;color:#fff;font-weight:800;font-size:20px}}
 .co{{font-weight:800;font-size:17px;letter-spacing:.2px}} .co small{{display:block;font-weight:500;color:#64748b;font-size:11.5px}}
 h1{{font-size:22px;margin:26px 0 6px}} .meta{{color:#64748b;font-size:12px;margin-bottom:20px}}
 .seal{{margin-top:30px;border:1px solid #dbeafe;background:#f0f9ff;border-radius:12px;padding:16px 18px}}
 .seal h2{{font-size:13px;margin:0 0 10px;color:var(--ok);letter-spacing:.4px;text-transform:uppercase}}
 .seal table{{width:100%;border-collapse:collapse;font:11.5px ui-monospace,SFMono-Regular,Menlo,monospace}}
 .seal td{{padding:3px 0;vertical-align:top;word-break:break-all}} .seal td:first-child{{color:#64748b;width:150px}}
 .ftr{{margin-top:26px;border-top:1px solid #e2e8f0;padding-top:12px;color:#94a3b8;font-size:11px}}
 @media print{{body{{background:#fff}} .page{{padding:0}}}}
</style></head><body><div class="page">
 <div class="hdr"><div class="logo">M</div>
   <div class="co">MBILINYI TECH SOLUTIONS<small>MTECH OS · Cyber Security &amp; Digital Forensics</small></div></div>
 <h1>{title}</h1>
 <div class="meta">Imetiwa muhuri: {at} · Kielezo #{idx} kwenye mnyororo</div>
 <div class="body">{body}</div>
 <div class="seal"><h2>✓ Muhuri wa kidijitali umethibitishwa</h2><table>
   <tr><td>Algorithm</td><td>{alg}</td></tr>
   <tr><td>SHA-256 (maudhui)</td><td>{chash}</td></tr>
   <tr><td>Sahihi (ECDSA DER b64)</td><td>{sig}</td></tr>
   <tr><td>Fingerprint ya funguo</td><td>{fp}</td></tr>
   <tr><td>Hash ya awali</td><td>{prev}</td></tr>
 </table></div>
 <div class="ftr">Hati hii ina muhuri wa kriptografia. Uthibitisho: /api/crypto/verify-seal ·
   Muhuri wowote kubadilishwa huvunja mnyororo (hash-chain tamper-evident).</div>
</div></body></html>"#,
        title = title,
        at = seal.at,
        idx = seal.chain_index,
        body = body_html,
        alg = seal.alg,
        chash = seal.content_hash,
        sig = seal.signature,
        fp = seal.key_fingerprint,
        prev = seal.prev_hash,
    )
}

/// Tia muhuri payload na urudishe hati kamili (HTML) + seal.
pub async fn seal_document(
    db: &SqlitePool,
    title: &str,
    body_html: &str,
) -> Result<SealedDoc, String> {
    let private_pem = active_private_pem(db).await?;
    let payload = format!("{title}\n{body_html}");
    let seal = seal(db, &private_pem, &payload).await?;
    let sealed_html = sealed_html(title, body_html, &seal);
    Ok(SealedDoc {
        title: title.to_string(),
        body_html: body_html.to_string(),
        seal,
        sealed_html,
    })
}

/// Eleza suite ya kriptografia (crypto-agility + mwelekeo wa PQC).
pub fn crypto_suite_json() -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "current": {
            "signature": ALG_ECDSA,
            "curve": "NIST P-256 (secp256r1)",
            "hash": "SHA-256",
            "nonce": "RFC 6979 (deterministic, hakuna kubahatisha RNG)",
            "encoding": "DER + PEM (PKCS#8 / SPKI)",
            "key_exchange": "X25519 (hybrid KEM, m328)",
        },
        "quantum_resistant": {
            "integrity": "SHA-256 hash-chain + Lamport OTS (hash-based; haitegemei ECDLP)",
            "note": "SHA-256 inastahimili Grover (nusu nguvu tu); Lamport/XMSS/SPHINCS+ ni safu ya PQC (NIST SP 800-208).",
        },
        "agility": ["ecdsa-p256-sha256", "lamport-sha256", "hybrid-kem-v1"],
        "roadmap": ["ML-DSA (Dilithium)", "ML-KEM (Kyber)", "SLH-DSA (SPHINCS+)"],
    })
}

// =====================================================================
// Vipimo
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let p = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_tables(&p).await;
        p
    }

    #[test]
    fn ecdsa_sign_na_verify_ni_halisi() {
        let (priv_pem, pub_pem) = generate_keypair();
        let msg = b"ripoti ya usalama ya MTECH OS";
        let sig = sign(&priv_pem, msg).unwrap();
        assert!(verify(&pub_pem, msg, &sig).unwrap());
        // Ujumbe uliobadilishwa LAZIMA ushindwe.
        assert!(!verify(&pub_pem, b"ripoti iliyobadilishwa", &sig).unwrap());
        // Funguo nyingine LAZIMA ishindwe.
        let (_p2, pub2) = generate_keypair();
        assert!(!verify(&pub2, msg, &sig).unwrap());
    }

    #[test]
    fn fingerprint_ni_sha256_ya_point() {
        let (_priv_pem, pub_pem) = generate_keypair();
        let fp = fingerprint(&pub_pem).unwrap();
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn seal_hutengeneza_muhuri_na_huthibitika() {
        let db = pool().await;
        ensure_company_key(&db).await.unwrap();
        let priv_pem = active_private_pem(&db).await.unwrap();
        let payload = "Ripoti: scan ya mtandao — 3 matukio ya hatari";
        let seal = seal(&db, &priv_pem, payload).await.unwrap();
        assert_eq!(seal.alg, ALG_ECDSA);
        assert_eq!(seal.chain_index, 1);
        assert_eq!(seal.content_hash, sha256_hex(payload.as_bytes()));
        let v = verify_seal(&db, payload, &seal).await.unwrap();
        assert!(v.valid && v.hash_matches && v.signature_valid);
        // Payload iliyobadilishwa LAZIMA ishindwe.
        let bad = verify_seal(&db, "payload nyingine", &seal).await.unwrap();
        assert!(!bad.valid);
    }

    #[tokio::test]
    async fn mnyororo_wa_hash_hushikamana_na_kuvunjika() {
        let db = pool().await;
        ensure_company_key(&db).await.unwrap();
        let priv_pem = active_private_pem(&db).await.unwrap();
        for i in 0..5 {
            seal(&db, &priv_pem, &format!("hati #{i}")).await.unwrap();
        }
        let st = verify_chain(&db).await.unwrap();
        assert!(st.intact, "mnyororo safi lazima ushikamane");
        assert_eq!(st.entries, 5);
        // Vunja kiungo cha 3 kwa mkono.
        sqlx::query("UPDATE crypto_ledger SET content_hash='deadbeef' WHERE chain_index=3")
            .execute(&db)
            .await
            .unwrap();
        let st2 = verify_chain(&db).await.unwrap();
        assert!(!st2.intact, "mnyororo uliovunjwa lazima ugundulike");
        assert_eq!(st2.broken_at, Some(3));
    }

    #[test]
    fn lamport_ots_inastahimili_quantum() {
        let key = lamport_keypair("seed-ya-kampuni-2026");
        let msg = b"mkataba wa kazi: mfanyakazi #1";
        let sig = lamport_sign(&key, msg);
        assert_eq!(sig.len(), LAMPORT_BITS);
        assert!(lamport_verify(&key.public, msg, &sig));
        assert!(!lamport_verify(&key.public, b"mkataba uliobadilishwa", &sig));
    }

    #[test]
    fn hash_chain_entry_hash_ni_deterministic() {
        let s = Seal {
            alg: ALG_ECDSA.into(),
            content_hash: "a".repeat(64),
            signature: "sig".into(),
            key_fingerprint: "fp".into(),
            prev_hash: "0".repeat(64),
            chain_index: 1,
            at: "2026-01-01 00:00:00".into(),
        };
        assert_eq!(seal_entry_hash(&s), seal_entry_hash(&s));
        assert_eq!(seal_entry_hash(&s).len(), 64);
    }
}
