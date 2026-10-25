//! auth.rs — LOGIN + ROLES (SEHEMU 9.2 + 4.3): credentials za default, tokens za session,
//! admin mkuu na wasaidizi.
//!
//! Mahitaji ya mmiliki:
//!   - "Sehemu ya login kwa watu wote kwa credentials za default"
//!   - "Admin mkuu na wasaidizi — admin mkuu anagawa kazi mwenyewe kwa wahusika"
//!
//! DEFAULT CREDENTIALS (kila mtu anaingia kwa hizi, kisha anabadilisha):
//!   MBILINYI / MKINGAHALISI → role: AdminMkuu (madaraka yote)
//!   fundi / fundi2026 → role: Msaidizi  (kazi zilizopewa)
//!
//! SEED FORMAT (legacy): password_hash = legacy_sha256(password, username),
//! salt="" — login ya kwanza inapandisha mtumiaji kutoka SHA-256 → Argon2id
//! moja kwa moja (transparent upgrade). Hakuna migration ya schema.

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Role {
    AdminMkuu,
    Msaidizi,
}

impl Role {
    pub fn id(&self) -> &'static str {
        match self {
            Role::AdminMkuu => "admin_mkuu",
            Role::Msaidizi => "msaidizi",
        }
    }

    /// Kutoka rollu iliyohifadhiwa (sessions.role) → enum.
    pub fn from_id(id: &str) -> Role {
        if id == "admin_mkuu" {
            Role::AdminMkuu
        } else {
            Role::Msaidizi
        }
    }

    pub fn name_sw(&self) -> &'static str {
        match self {
            Role::AdminMkuu => "Admin Mkuu",
            Role::Msaidizi => "Msaidizi",
        }
    }

    /// Admin Mkuu anagawa kazi; Msaidizi anafanya zilizopewa.
    pub fn can_assign_tasks(&self) -> bool {
        matches!(self, Role::AdminMkuu)
    }

    /// High-risk actions (install/reboot/wipe) — Admin Mkuu PEKEE anaidhinisha.
    pub fn can_approve_high_risk(&self) -> bool {
        matches!(self, Role::AdminMkuu)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub token: String,
    pub username: String,
    pub role: String,
    pub created_at: String,
}

/// Username ya ndani ya server (agent/chat bridge) — HAIONEKANI kwa mtumiaji
/// na haiguswi na login/logout za watu halisi.
pub const AGENT_USER: &str = "__agent__";

pub async fn init_tables(db: &SqlitePool) {
    for ddl in [
        r#"CREATE TABLE IF NOT EXISTS users (
            username TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL,
            salt TEXT NOT NULL,
            role TEXT NOT NULL,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            role TEXT NOT NULL,
            created_at TEXT NOT NULL
        )"#,
    ] {
        let _ = sqlx::query(ddl).execute(db).await;
    }
    // Seed default users (SEHEMU 9.2)
    // MBILINYI = Admin Mkuu (madaraka yote); password mpya ya kazi halisi
    for (u, p, r) in [
        ("MBILINYI", "MKINGAHALISI", "admin_mkuu"),
        ("fundi", "fundi2026", "msaidizi"),
    ] {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO users (username, password_hash, salt, role, created_at) VALUES (?,?,?,?,?)",
        )
        .bind(u)
        .bind(legacy_sha256(p, u))
        .bind("")
        .bind(r)
        .bind(chrono::Local::now().to_rfc3339())
        .execute(db)
        .await;
    }
}

/// Params za Argon2id v19 (RFC 9106 recommended balance): m=19MiB, t=2, p=1.
/// Miliseconds 1–2 kwa laptop ya kawaida — inafaa kwa login ya web app bila
/// kukatiza mtiririko wa dashboard.
fn argon2_instance() -> Argon2<'static> {
    Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(19456, 2, 1, None).expect("argon2 params halali"),
    )
}

/// Hash ya Argon2id kwa salt ya hex — PHC string kamili (`$argon2id$v=19$…`).
fn hash_password_argon2id(password: &str, salt_hex: &str) -> Result<String, String> {
    let salt = salt_string_from_hex(salt_hex)?;
    let hash = argon2_instance()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("argon2 hash: {e}"))?;
    Ok(hash.to_string())
}

/// Thibitisha password dhidi ya PHC string ya Argon2id — salt inatumika
/// moja kwa moja kutoka ndani ya PHC string (kama PasswordVerifier inavyoelekeza).
fn matches_argon2id(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => argon2_instance()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Legacy SHA-256+salt ( sichanga na kemikali ya auth mpya; inabaki kwa rows za awali).
fn legacy_sha256(password: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(b":mtech:");
    h.update(password.as_bytes());
    format!("{:x}", h.finalize())
}

/// Random salt ya hex 16-byte (32 chars) — Argon2 PHC inaitaji base64.
fn new_salt_hex() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// JE, salt hex ("aabb…") kwa SaltString ya PHC (base64, bila padding).
fn salt_string_from_hex(hex: &str) -> Result<SaltString, String> {
    if hex.len() % 2 != 0 || hex.is_empty() {
        return Err("salt si hexadecimal halali".to_string());
    }
    let bytes = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("salt si hex: {e}"))?;
    SaltString::encode_b64(&bytes)        .map_err(|e| format!("salt b64: {e}"))
}

/// Login halisi: anza kwa default credentials, kisha password iliyobadilishwa.
pub async fn login(db: &SqlitePool, username: &str, password: &str) -> Result<Session, String> {
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT username, password_hash, salt, role FROM users WHERE username = ?")
            .bind(username.trim())
            .fetch_optional(db)
            .await
            .unwrap_or(None);

    let (user, stored_hash, salt, role) = match row {
        Some(r) => r,
        None => return Err("Username au password si sahihi".into()),
    };

    // Format halisi: PHC string inaanza na '$' = Argon2id (mpya).
    // vinginevyo = legacy SHA-256 (seed rows: salt="" → salt=username).
    let verify = |pw: &str, hash_salt: &str| -> bool {
        if stored_hash.starts_with('$') {
            matches_argon2id(pw, &stored_hash)
        } else {
            legacy_sha256(pw, hash_salt) == stored_hash
        }
    };
    let matches = if salt.is_empty() {
        verify(password, &user)
    } else {
        verify(password, &salt)
    };
    if !matches {
        return Err("Username au password si sahihi".into());
    }

    // Transparent upgrade: legacy row iliyothibitishwa inapandishwa kwenye
    // Argon2id + salt mpya ya kipekee. Hakuna kosa kwa mtumiaji — kama DB
    // imekataa (read-only n.k.), tunarudi kawaida bila kuvunja login.
    if !stored_hash.starts_with('$') {
        let upgrade_salt = new_salt_hex();
        if let Ok(h) = hash_password_argon2id(password, &upgrade_salt) {
            let _ = sqlx::query("UPDATE users SET password_hash=?, salt=? WHERE username=?")
                .bind(&h)
                .bind(&upgrade_salt)
                .bind(&user)
                .execute(db)
                .await;
        }
    }

    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let created = chrono::Local::now().to_rfc3339();
    sqlx::query("INSERT INTO sessions (token, username, role, created_at) VALUES (?,?,?,?)")
        .bind(&token)
        .bind(&user)
        .bind(&role)
        .bind(&created)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Session {
        token,
        username: user,
        role,
        created_at: created,
    })
}

pub async fn verify_token(db: &SqlitePool, token: &str) -> Option<Session> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT token, username, role FROM sessions WHERE token = ?")
            .bind(token)
            .fetch_optional(db)
            .await
            .unwrap_or(None);
    row.map(|(token, username, role)| Session {
        token,
        username,
        role,
        created_at: String::new(),
    })
}

pub async fn logout(db: &SqlitePool, token: &str) -> bool {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(db)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

/// RBAC KWA AGENT NDANI: chat-agent ndani ya server hii hupiga fundi_api —
/// anapata session halisi ya Admin Mkuu kila kuanzishwa kwa server.
///
/// MUHIMU: token hii ni ya `__agent__` (si ya `MBILINYI`). Kwa hiyo
/// kuanzishwa/kurestart server HAIFUTI session za mtumiaji halisi aliyeingia.
/// (Kabuli ya awali iliyokuwa `DELETE ... WHERE username='MBILINYI'` ilikuwa
///  inaua login ya Admin Mkuu kwenye kila deploy/restart — dashboard nzima
/// ikarudi kwenye 401/prompt.)
pub async fn agent_session(db: &SqlitePool) -> String {
    // Safisha token ya zamani ya agent PEKEE (hakuna clutter, hakuna kuua login ya mtu).
    let _ = sqlx::query("DELETE FROM sessions WHERE username = ?")
        .bind(AGENT_USER)
        .execute(db)
        .await;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let created = chrono::Local::now().to_rfc3339();
    let _ = sqlx::query("INSERT INTO sessions (token, username, role, created_at) VALUES (?,?,?,?)")
        .bind(&token)
        .bind(AGENT_USER)
        .bind(Role::AdminMkuu.id())
        .bind(&created)
        .execute(db)
        .await;
    token
}

/// Token ya agent IMARA (inadumu kwenye restart): Canvas (/ide) huipata
/// kupitia `window.__AGENT_CANVAS_SESSION_API_KEY__`, hivyo lazima ibaki sawa
/// ili mtumiaji asionekane "ametoka" kila server inapoanzishwa upya. Token
/// huhifadhiwa kwenye `<dir>/agent_token` (mode 0600) na kuhuishwa kwenye DB.
pub async fn agent_session_stable(db: &SqlitePool, dir: &str) -> String {
    let path = std::path::Path::new(dir).join("agent_token");
    let existing = std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(tok) = existing {
        // Huisha session kwenye DB (inaweza kuwa imefutwa na restart ya awali).
        let _ = sqlx::query("DELETE FROM sessions WHERE username = ?")
            .bind(AGENT_USER)
            .execute(db)
            .await;
        let created = chrono::Local::now().to_rfc3339();
        if sqlx::query("INSERT INTO sessions (token, username, role, created_at) VALUES (?,?,?,?)")
            .bind(&tok)
            .bind(AGENT_USER)
            .bind(Role::AdminMkuu.id())
            .bind(&created)
            .execute(db)
            .await
            .is_ok()
        {
            return tok;
        }
    }

    // Hakuna token imara — tengeneza mpya na uihifadhi.
    let tok = agent_session(db).await;
    let _ = std::fs::create_dir_all(dir);
    if std::fs::write(&path, &tok).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
    tok
}

/// Badilisha password (salt mpya ya kipekee).
pub async fn change_password(
    db: &SqlitePool,
    token: &str,
    new_password: &str,
) -> Result<(), String> {
    let sess = verify_token(db, token).await.ok_or("Session si sahihi")?;
    if new_password.chars().count() < 6 {
        return Err("Password lazima iwe na herufi 6+".into());
    }
    let salt: String = new_salt_hex();
    let new_hash = hash_password_argon2id(new_password, &salt)
        .map_err(|e| format!("hash mpya: {e}"))?;
    sqlx::query("UPDATE users SET password_hash=?, salt=? WHERE username=?")
        .bind(&new_hash)
        .bind(&salt)
        .bind(&sess.username)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_credentials_zinafanya_kazi() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // MBILINYI / MKINGAHALISI → AdminMkuu
        let s = login(&db, "MBILINYI", "MKINGAHALISI").await.unwrap();
        assert_eq!(s.role, "admin_mkuu");
        assert!(!s.token.is_empty());
        // fundi / fundi2026 → Msaidizi
        let s2 = login(&db, "fundi", "fundi2026").await.unwrap();
        assert_eq!(s2.role, "msaidizi");
        // password mbaya inakataa
        assert!(login(&db, "MBILINYI", "mbaya").await.is_err());
        assert!(login(&db, "hakuna", "MKINGAHALISI").await.is_err());
        // admin ya zamani huendelee kufanya kazi — hakuna tena default yake
        assert!(login(&db, "admin", "mtech2026").await.is_err());
    }

    #[tokio::test]
    async fn token_verify_na_logout() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        let s = login(&db, "MBILINYI", "MKINGAHALISI").await.unwrap();
        let v = verify_token(&db, &s.token).await.unwrap();
        assert_eq!(v.username, "MBILINYI");
        assert!(logout(&db, &s.token).await);
        assert!(verify_token(&db, &s.token).await.is_none());
    }

    #[tokio::test]
    async fn agent_session_haifutii_mtumiaji_session() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // Mtumiaji ameingia tayari
        let user = login(&db, "MBILINYI", "MKINGAHALISI").await.unwrap();
        // Agent ya ndani inaanzishwa (kama kwenye boot ya server)
        let tok = agent_session(&db).await;
        assert_eq!(tok.len(), 64);
        // Login ya mtumiaji BADO inafanya kazi — restart haimuondoi
        assert!(
            verify_token(&db, &user.token).await.is_some(),
            "boot ya server haipaswi kuua session ya Admin Mkuu"
        );
        // Token ya agent ina role ya admin_mkuu (RBAC ya fundi_api)
        let s = verify_token(&db, &tok).await.unwrap();
        assert_eq!(s.username, AGENT_USER);
        assert_eq!(s.role, "admin_mkuu");
        // Kuanzisha tena HAITOJI hali ya kutengeneza taka: moja tu ya agent
        let tok2 = agent_session(&db).await;
        assert!(verify_token(&db, &tok).await.is_none(), "token ya zamani ya agent inafutwa");
        assert!(verify_token(&db, &tok2).await.is_some());
        assert!(verify_token(&db, &user.token).await.is_some());
    }

    #[test]
    fn roles_na_madaraka() {
        assert!(Role::AdminMkuu.can_assign_tasks());
        assert!(!Role::Msaidizi.can_assign_tasks());
        assert!(Role::AdminMkuu.can_approve_high_risk());
        assert!(!Role::Msaidizi.can_approve_high_risk());
    }

    #[tokio::test]
    async fn agent_session_stable_hudumu_kwenye_restart() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("fundi_stable_{}", std::process::id()));
        let dir_s = dir.to_string_lossy().to_string();

        let t1 = agent_session_stable(&db, &dir_s).await;
        assert_eq!(t1.len(), 64);
        assert!(verify_token(&db, &t1).await.is_some());

        // "Restart": simulia kufutwa kwa DB; kwa token IMARA inahuishwa.
        let _ = sqlx::query("DELETE FROM sessions WHERE username = ?")
            .bind(AGENT_USER)
            .execute(&db)
            .await;
        let t2 = agent_session_stable(&db, &dir_s).await;
        assert_eq!(t2, t1, "token imara haipaswi kubadilika kwenye restart");
        assert!(verify_token(&db, &t2).await.is_some());
        assert_eq!(verify_token(&db, &t2).await.unwrap().role, "admin_mkuu");

        let _ = std::fs::remove_file(dir.join("agent_token"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn argon2id_hash_na_thibitisho() {
        // PHC string inajumuisha algo+params+salt inayojulikana kwa Argon2.
        let salt = new_salt_hex();
        let hash = hash_password_argon2id("siri-ndogo", &salt).unwrap();
        assert!(hash.starts_with("$argon2id$"), "PHC mba: {hash}");
        // Thibitisho: password sahihi inafanana, mbaya hainafanana.
        assert!(matches_argon2id("siri-ndogo", &hash));
        assert!(!matches_argon2id("siyo-ndogo", &hash));
    }

    #[tokio::test]
    async fn legacy_login_inapandisha_kwenye_argon2id() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // Seed row: legacy SHA-256 (salt="")
        let s = login(&db, "fundi", "fundi2026").await.unwrap();
        assert_eq!(s.role, "msaidizi");
        // Baada ya login, row imepandishwa: hash = PHC, salt = hex ya kipekee.
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT password_hash, salt FROM users WHERE username='fundi'")
                .fetch_optional(&db)
                .await
                .unwrap();
        let (hash, salt) = row.expect("fundi yupo");
        assert!(hash.starts_with("$argon2id$"), "hash haijapandishwa: {hash}");
        assert!(!salt.is_empty(), "salt mpya inatakiwa");
        // Login ya pili (Argon2id row) bado inafanya kazi.
        assert!(login(&db, "fundi", "fundi2026").await.is_ok());
        assert!(login(&db, "fundi", "mbaya").await.is_err());
    }

    #[tokio::test]
    async fn change_password_inafanya_kazi() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        let s = login(&db, "fundi", "fundi2026").await.unwrap();
        change_password(&db, &s.token, "password-mpya-1")
            .await
            .unwrap();
        // default haifanyi kazi tena; mpya inafanya
        assert!(login(&db, "fundi", "fundi2026").await.is_err());
        assert!(login(&db, "fundi", "password-mpya-1").await.is_ok());
    }
}
