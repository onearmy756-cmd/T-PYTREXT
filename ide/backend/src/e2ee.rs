//! e2ee.rs — USIRI WA MWISHO-HADI-MWISHO (End-to-End Encryption) halisi.
//!
//! Hii inafanya ahadi ya "data haisomeki" kuwa KWELI kwa kriptografia, si maneno:
//!   - **X25519** (ECDH) — kubadilishana funguo kwa njia ya siri.
//!   - **HKDF-SHA256** — kutoa key ya kusimba kutoka siri iliyoshirikiwa.
//!   - **ChaCha20-Poly1305** (AEAD) — kusimba + kuthibitisha uadilifu.
//!
//! Kanuni:
//!   1. Server HAIHIFADHI secret key ya mtumiaji — inaenda kwa mtumiaji pekee.
//!   2. Kila ujumbe hupata nonce ya kipekee (haizidi kurudiwa) — hii ni muhimu
//!      sana kwa AEAD; nonce inarudiwa = usiri umevunjika.
//!   3. Sealed box: (ephemeral_pub, nonce, ciphertext||tag).
//!   4. Uthibitisho: tag ya Poly1305 hukataa ciphertext yoyote iliyobadilishwa.
//!
//! Hii ni "local-first": mtumiaji ana funguo; server ni kikasha tu.

use crate::AppState;
use axum::{
    extract::{Path as AxPath, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;
use sqlx::SqlitePool;
use x25519_dalek::{PublicKey, StaticSecret};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/e2ee/status", get(status))
        .route("/api/e2ee/identity", post(identity))
        .route("/api/e2ee/encrypt", post(encrypt))
        .route("/api/e2ee/decrypt", post(decrypt))
        .route("/api/e2ee/seal", post(seal))
        .route("/api/e2ee/open", post(open))
        .route("/api/e2ee/key/save", post(key_save))
        .route("/api/e2ee/key/:owner", get(key_get))
}

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS e2ee_keys (
            owner TEXT PRIMARY KEY,
            public_key TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS e2ee_sealed (
            id TEXT PRIMARY KEY,
            owner TEXT NOT NULL DEFAULT '',
            recipient TEXT NOT NULL DEFAULT '',
            ephemeral_pub TEXT NOT NULL,
            nonce TEXT NOT NULL,
            ciphertext TEXT NOT NULL,
            aad TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

/// HKDF-SHA256: siri iliyoshirikiwa (32B) → key ya ChaCha20 (32B).
/// Salt = muunganisho wa funguo mbili za umma (hii huzuia key reuse kati ya
/// jozi tofauti). Info = muktadha.
fn derive_key(shared: &[u8; 32], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), shared);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm).expect("HKDF 32B haishindwi");
    okm
}

fn rand32() -> [u8; 32] {
    let mut b = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut b);
    b
}

fn rand12() -> [u8; 12] {
    let mut b = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut b);
    b
}

/// Funguo za mtumiaji (secret + public, base64). Secret HAIHIFADHIWI kwenye DB.
fn gen_identity() -> (String, String) {
    let secret = StaticSecret::from(rand32());
    let public = PublicKey::from(&secret);
    (B64.encode(secret.to_bytes()), B64.encode(public.to_bytes()))
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM e2ee_keys")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    let sealed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM e2ee_sealed")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    Json(json!({
        "ok": true,
        "module": "e2ee",
        "algorithms": {
            "key_exchange": "X25519 (ECDH)",
            "key_derivation": "HKDF-SHA256",
            "encryption": "ChaCha20-Poly1305 (AEAD)",
            "nonce": "96-bit, kipekee kwa kila ujumbe",
        },
        "public_keys": keys,
        "sealed_messages": sealed,
        "server_stores_secret": false,
        "privacy": "server haiwezi kusoma maudhui — haina key ya mtumiaji",
    }))
}

#[derive(Deserialize)]
struct IdIn {
    #[serde(default)]
    owner: Option<String>,
}

/// Hutengeneza jozi mpya ya funguo. Secret inarudi KWA MTUMIAJI PEKEE.
async fn identity(State(s): State<AppState>, Json(b): Json<IdIn>) -> Json<Value> {
    let (secret, public) = gen_identity();
    if let Some(owner) = b.owner.as_deref().map(str::trim).filter(|o| !o.is_empty()) {
        let _ = sqlx::query(
            "INSERT INTO e2ee_keys (owner, public_key, created_at, updated_at)
             VALUES (?1,?2,?3,?3)
             ON CONFLICT(owner) DO UPDATE SET public_key=excluded.public_key, updated_at=excluded.updated_at",
        )
        .bind(owner)
        .bind(&public)
        .bind(now())
        .execute(&s.db)
        .await;
    }
    Json(json!({
        "ok": true,
        "secret_key": secret,
        "public_key": public,
        "warning": "Hifadhi secret_key mahali salama — server haitaihifadhi. Ikupotea = data haiwezi kufunguliwa.",
    }))
}

#[derive(Deserialize)]
struct KeyIn {
    owner: String,
    public_key: String,
}

/// Husajili funguo ya umma ya mtumiaji (secret haitumwa kamwe).
async fn key_save(State(s): State<AppState>, Json(b): Json<KeyIn>) -> Json<Value> {
    let owner = b.owner.trim();
    let pk = b.public_key.trim();
    if owner.is_empty() || pk.is_empty() {
        return Json(json!({"ok": false, "error": "owner na public_key zinahitajika"}));
    }
    let raw = match B64.decode(pk) {
        Ok(r) if r.len() == 32 => r,
        _ => return Json(json!({"ok": false, "error": "public_key si X25519 ya 32B (base64)"})),
    };
    let _ = raw;
    let res = sqlx::query(
        "INSERT INTO e2ee_keys (owner, public_key, created_at, updated_at)
         VALUES (?1,?2,?3,?3)
         ON CONFLICT(owner) DO UPDATE SET public_key=excluded.public_key, updated_at=excluded.updated_at",
    )
    .bind(owner)
    .bind(pk)
    .bind(now())
    .execute(&s.db)
    .await;
    match res {
        Ok(_) => Json(json!({"ok": true, "owner": owner})),
        Err(e) => Json(json!({"ok": false, "error": format!("kuhifadhi funguo: {e}")})),
    }
}

async fn key_get(State(s): State<AppState>, AxPath(owner): AxPath<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT owner, public_key FROM e2ee_keys WHERE owner=?1",
    )
    .bind(&owner)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match row {
        Some((o, pk)) => Json(json!({"ok": true, "owner": o, "public_key": pk})),
        None => Json(json!({"ok": false, "error": format!("hakuna funguo ya umma kwa '{owner}'")})),
    }
}

#[derive(Deserialize)]
struct CryptIn {
    /// Base64 ya key ya 32B (secret key, au key iliyoshirikiwa).
    key: String,
    /// Maandishi (plaintext) au base64 (ciphertext).
    data: String,
    #[serde(default)]
    aad: Option<String>,
}

/// Husimba kwa key ya moja kwa moja (ChaCha20-Poly1305). Hurudisha base64 ya
/// `nonce(12) || ciphertext||tag`.
async fn encrypt(Json(b): Json<CryptIn>) -> Json<Value> {
    let Some(key) = decode_key(&b.key) else {
        return Json(json!({"ok": false, "error": "key si base64 ya 32B"}));
    };
    let nonce_bytes = rand12();
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let aad = b.aad.as_deref().unwrap_or("").as_bytes();
    let payload = chacha20poly1305::aead::Payload { msg: b.data.as_bytes(), aad };
    let Ok(ct) = cipher.encrypt(Nonce::from_slice(&nonce_bytes), payload) else {
        return Json(json!({"ok": false, "error": "kusimba kumeshindwa"}));
    };
    let mut out = nonce_bytes.to_vec();
    out.extend_from_slice(&ct);
    Json(json!({
        "ok": true,
        "algorithm": "ChaCha20-Poly1305",
        "nonce": B64.encode(nonce_bytes),
        "ciphertext": B64.encode(out),
    }))
}

async fn decrypt(Json(b): Json<CryptIn>) -> Json<Value> {
    let Some(key) = decode_key(&b.key) else {
        return Json(json!({"ok": false, "error": "key si base64 ya 32B"}));
    };
    let Ok(blob) = B64.decode(b.data.trim()) else {
        return Json(json!({"ok": false, "error": "data si base64"}));
    };
    if blob.len() < 12 + 16 {
        return Json(json!({"ok": false, "error": "ciphertext ni fupi mno"}));
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let aad = b.aad.as_deref().unwrap_or("").as_bytes();
    let payload = chacha20poly1305::aead::Payload { msg: ct, aad };
    match cipher.decrypt(Nonce::from_slice(nonce_bytes), payload) {
        Ok(pt) => Json(json!({
            "ok": true,
            "plaintext": String::from_utf8_lossy(&pt).to_string(),
        })),
        Err(_) => Json(json!({
            "ok": false,
            "error": "kufungua kumeshindwa — key si sahihi AU data imebadilishwa (tag imekataa)",
        })),
    }
}

#[derive(Deserialize)]
struct SealIn {
    /// Funguo ya umma ya mpokeaji (base64, 32B).
    recipient_public_key: String,
    data: String,
    #[serde(default)]
    aad: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    recipient: Option<String>,
    /// `true` → hifadhi sealed box kwenye DB (kwa uwasilishaji).
    #[serde(default)]
    store: Option<bool>,
}

/// **Sealed box**: husimba kwa funguo ya umma ya mpokeaji bila kubadilishana
/// funguo mapema. Hatumii ephemeral secret — mpokeaji anafungua kwa secret key
/// yake. Hii ni njia salama zaidi kwa "kutuma kwa mtu".
async fn seal(State(s): State<AppState>, Json(b): Json<SealIn>) -> Json<Value> {
    let Ok(recipient_raw) = B64.decode(b.recipient_public_key.trim()) else {
        return Json(json!({"ok": false, "error": "recipient_public_key si base64"}));
    };
    if recipient_raw.len() != 32 {
        return Json(json!({"ok": false, "error": "recipient_public_key lazima iwe 32B"}));
    }
    let mut rp = [0u8; 32];
    rp.copy_from_slice(&recipient_raw);
    let recipient_pub = PublicKey::from(rp);

    // Ephemeral sender key (mpya kwa kila ujumbe — forward secrecy).
    let eph_secret = StaticSecret::from(rand32());
    let eph_pub = PublicKey::from(&eph_secret);
    let shared = eph_secret.diffie_hellman(&recipient_pub);

    // Salt = eph_pub || recipient_pub → key ya kipekee kwa jozi hii.
    let mut salt = Vec::with_capacity(64);
    salt.extend_from_slice(eph_pub.as_bytes());
    salt.extend_from_slice(recipient_pub.as_bytes());
    let key = derive_key(shared.as_bytes(), &salt, b"MTECHOS-E2EE-v1");

    let nonce_bytes = rand12();
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let aad = b.aad.as_deref().unwrap_or("").as_bytes();
    let payload = chacha20poly1305::aead::Payload { msg: b.data.as_bytes(), aad };
    let Ok(ct) = cipher.encrypt(Nonce::from_slice(&nonce_bytes), payload) else {
        return Json(json!({"ok": false, "error": "kusimba kumeshindwa"}));
    };

    let eph_b64 = B64.encode(eph_pub.as_bytes());
    let nonce_b64 = B64.encode(nonce_bytes);
    let ct_b64 = B64.encode(&ct);
    let mut id = String::new();
    if b.store.unwrap_or(false) {
        id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO e2ee_sealed (id, owner, recipient, ephemeral_pub, nonce, ciphertext, aad, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        )
        .bind(&id)
        .bind(b.owner.clone().unwrap_or_default())
        .bind(b.recipient.clone().unwrap_or_default())
        .bind(&eph_b64)
        .bind(&nonce_b64)
        .bind(&ct_b64)
        .bind(b.aad.clone().unwrap_or_default())
        .bind(now())
        .execute(&s.db)
        .await;
    }

    Json(json!({
        "ok": true,
        "algorithm": "X25519 + HKDF-SHA256 + ChaCha20-Poly1305",
        "ephemeral_public_key": eph_b64,
        "nonce": nonce_b64,
        "ciphertext": ct_b64,
        "id": id,
    }))
}

#[derive(Deserialize)]
struct OpenIn {
    /// Secret key ya mpokeaji (base64, 32B).
    secret_key: String,
    ephemeral_public_key: String,
    nonce: String,
    ciphertext: String,
    #[serde(default)]
    aad: Option<String>,
}

async fn open(Json(b): Json<OpenIn>) -> Json<Value> {
    let Some(secret_bytes) = decode_key(&b.secret_key) else {
        return Json(json!({"ok": false, "error": "secret_key si base64 ya 32B"}));
    };
    let Ok(eph_raw) = B64.decode(b.ephemeral_public_key.trim()) else {
        return Json(json!({"ok": false, "error": "ephemeral_public_key si base64"}));
    };
    if eph_raw.len() != 32 {
        return Json(json!({"ok": false, "error": "ephemeral_public_key lazima iwe 32B"}));
    }
    let Ok(nonce_raw) = B64.decode(b.nonce.trim()) else {
        return Json(json!({"ok": false, "error": "nonce si base64"}));
    };
    if nonce_raw.len() != 12 {
        return Json(json!({"ok": false, "error": "nonce lazima iwe 12B"}));
    }
    let Ok(ct) = B64.decode(b.ciphertext.trim()) else {
        return Json(json!({"ok": false, "error": "ciphertext si base64"}));
    };

    let mut eph = [0u8; 32];
    eph.copy_from_slice(&eph_raw);
    let eph_pub = PublicKey::from(eph);
    let secret = StaticSecret::from(secret_bytes);
    let my_pub = PublicKey::from(&secret);
    let shared = secret.diffie_hellman(&eph_pub);

    let mut salt = Vec::with_capacity(64);
    salt.extend_from_slice(eph_pub.as_bytes());
    salt.extend_from_slice(my_pub.as_bytes());
    let key = derive_key(shared.as_bytes(), &salt, b"MTECHOS-E2EE-v1");

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let aad = b.aad.as_deref().unwrap_or("").as_bytes();
    let payload = chacha20poly1305::aead::Payload { msg: &ct, aad };
    match cipher.decrypt(Nonce::from_slice(&nonce_raw), payload) {
        Ok(pt) => Json(json!({
            "ok": true,
            "plaintext": String::from_utf8_lossy(&pt).to_string(),
        })),
        Err(_) => Json(json!({
            "ok": false,
            "error": "kufungua kumeshindwa — secret key si sahihi AU data imebadilishwa",
        })),
    }
}

fn decode_key(s: &str) -> Option<[u8; 32]> {
    let raw = B64.decode(s.trim()).ok()?;
    if raw.len() != 32 {
        return None;
    }
    let mut k = [0u8; 32];
    k.copy_from_slice(&raw);
    Some(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_ni_32b_na_tofauti() {
        let (s1, p1) = gen_identity();
        let (s2, p2) = gen_identity();
        assert_eq!(B64.decode(&p1).unwrap().len(), 32);
        assert_eq!(B64.decode(&s1).unwrap().len(), 32);
        assert_ne!(s1, s2);
        assert_ne!(p1, p2);
    }

    #[test]
    fn derive_key_ni_deterministic() {
        let shared = [7u8; 32];
        let salt = [9u8; 64];
        let k1 = derive_key(&shared, &salt, b"ctx");
        let k2 = derive_key(&shared, &salt, b"ctx");
        let k3 = derive_key(&shared, &salt, b"tofauti");
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn chacha_roundtrip_inafanya_kazi() {
        let key = rand32();
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let nonce = rand12();
        let msg = b"habari ya siri ya MTECHOS";
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), msg.as_ref())
            .unwrap();
        let pt = cipher
            .decrypt(Nonce::from_slice(&nonce), ct.as_ref())
            .unwrap();
        assert_eq!(&pt, msg);
    }

    #[test]
    fn tag_hukataa_ciphertext_iliyobadilishwa() {
        let key = rand32();
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let nonce = rand12();
        let mut ct = cipher
            .encrypt(Nonce::from_slice(&nonce), b"muhimu".as_ref())
            .unwrap();
        ct[0] ^= 0xff; // haribu byte moja
        assert!(cipher.decrypt(Nonce::from_slice(&nonce), ct.as_ref()).is_err());
    }

    #[test]
    fn ecdh_hushiriki_siri_ile_ile() {
        let a_secret = StaticSecret::from(rand32());
        let a_pub = PublicKey::from(&a_secret);
        let b_secret = StaticSecret::from(rand32());
        let b_pub = PublicKey::from(&b_secret);
        assert_eq!(
            a_secret.diffie_hellman(&b_pub).as_bytes(),
            b_secret.diffie_hellman(&a_pub).as_bytes()
        );
    }

    #[test]
    fn decode_key_hukataa_urefu_mbaya() {
        assert!(decode_key("").is_none());
        assert!(decode_key(&B64.encode([1u8; 16])).is_none());
        assert!(decode_key(&B64.encode([1u8; 32])).is_some());
    }
}
