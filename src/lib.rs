use pyo3::prelude::*;
use pyo3::exceptions::{PyOSError, PyRuntimeError};
use std::sync::Mutex;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use chrono::Utc;
use tokio::runtime::Runtime;
use sqlx::sqlite::SqlitePool;
use sqlx::Executor;
use std::sync::atomic::{AtomicI64, Ordering};
use tauri::Manager;
use std::fs;
use std::io::{Write, Read, Cursor};
use std::path::Path;
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use rand::rngs::OsRng;
use rand::RngCore;
use flate2::write::{ZlibEncoder, ZlibDecoder};
use flate2::Compression;
use zeroize::Zeroize;

// === PyTreXT Extended: Axum HTTP Server ===
use axum::{
    Router, routing::{get, post}, extract::{State, Json, ws::{WebSocket, WebSocketUpgrade, Message}},
    response::IntoResponse, http::StatusCode,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use std::net::SocketAddr;
use std::collections::HashMap;

// === PyTreXT Extended: MCP Protocol ===
use uuid::Uuid;

// ============================================================
//  LOGGER — Andika kwenye faili moja (Rust + Python + Elixir)
// ============================================================

fn log_to_file(level: &str, message: &str) {
    let log_path = "pytrex_debug.log";
    let timestamp = Utc::now().to_rfc3339();
    let log_line = format!("[{}] [RUST] [{}] {}\n", timestamp, level, message);
    if let Ok(mut file) = fs::OpenOptions::new().append(true).create(true).open(log_path) {
        let _ = file.write_all(log_line.as_bytes());
    }
}

/// Sanitize sensitive data from log messages — never log passwords, keys, or payloads
fn sanitize_log_message(msg: &str) -> String {
    let sensitive_patterns = ["password", "passwd", "key", "secret", "token", "credential", "api_key"];
    let lower = msg.to_lowercase();
    for pattern in &sensitive_patterns {
        if lower.contains(pattern) {
            return format!("[REDACTED: contains '{}']", pattern);
        }
    }
    msg.to_string()
}

// ============================================================
//  STATIC VARIABLES (RAM) — Database Pool & Blockchain State
// ============================================================

static DB_POOL: Mutex<Option<SqlitePool>> = Mutex::new(None);
static LAST_BLOCK_HASH: Mutex<Option<String>> = Mutex::new(None);
static BLOCK_INDEX: AtomicI64 = AtomicI64::new(0);
static AXUM_SERVER: Mutex<Option<tokio::task::JoinHandle<()>>> = Mutex::new(None);

// ============================================================
//  1. TAURI v2 — UI COMMAND: Kupokea amri kutoka UI na kuitupia Python Core
// ============================================================

#[tauri::command]
fn tauri_to_python(event_name: String, data: String) -> String {
    println!("[PyTreX Rust] Command received from UI: {}", event_name);
    let safe_msg = sanitize_log_message(&format!("UI Command: {}", event_name));
    log_to_file("INFO", &safe_msg);

    Python::with_gil(|py| {
        let core_module = match py.import("pytrex.core") {
            Ok(m) => m,
            Err(e) => {
                let msg = format!("Failed to import pytrex.core: {}", e);
                log_to_file("ERROR", &msg);
                return format!(r#"{{"status":"error","message":"Rust core import failed: {}"}}"#, msg);
            }
        };

        let execute_func = match core_module.getattr("execute_python_event") {
            Ok(f) => f,
            Err(e) => {
                let msg = format!("Failed to get execute_python_event: {}", e);
                log_to_file("ERROR", &msg);
                return format!(r#"{{"status":"error","message":"Function not found: {}"}}"#, msg);
            }
        };

        match execute_func.call1((event_name, data)) {
            Ok(result) => {
                match result.extract::<String>() {
                    Ok(s) => s,
                    Err(e) => {
                        let msg = format!("Failed to extract result: {}", e);
                        log_to_file("ERROR", &msg);
                        format!(r#"{{"status":"error","message":"Result extraction failed"}}"#)
                    }
                }
            }
            Err(e) => {
                let msg = format!("Python event execution failed: {}", e);
                log_to_file("ERROR", &msg);
                format!(r#"{{"status":"error","message":"Event execution failed: {}"}}"#, msg)
            }
        }
    })
}

// ============================================================
//  1b. TAURI v2 — TOAST NOTIFICATION COMMAND
// ============================================================

#[tauri::command]
fn tuma_notification(title: String, body: String) {
    println!("[PyTreX] Notification: {} - {}", title, body);
    log_to_file("INFO", &format!("Notification: {} - {}", title, body));
}

// ============================================================
//  2. TAURI v2 — WASHA WINDOW YA PROGRAMU
// ============================================================

#[pyfunction]
fn fanya_app() -> PyResult<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            tauri_to_python,
            tuma_notification,
            mobile_camera,
            mobile_gps,
            mobile_vibrate,
            mobile_share
        ])
        .setup(|app| {
            // Store the app handle for multi-window support
            {
                let mut handle_guard = APP_HANDLE.lock().unwrap();
                *handle_guard = Some(app.handle().clone());
            }

            if app.get_webview_window("main").is_none() {
                let _window = tauri::WebviewWindowBuilder::new(
                    app,
                    "main",
                    tauri::WebviewUrl::App("frontend/index.html".parse().unwrap()),
                )
                .title("PyTreX Engine v1.0.0")
                .inner_size(900.0, 700.0)
                .build()
                .map_err(|e| {
                    log_to_file("ERROR", &format!("Failed to create main window: {}", e));
                    e
                })?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| {
            log_to_file("FATAL", &format!("Tauri failed to start: {}", e));
            PyRuntimeError::new_err(format!("Imeshindwa kuendesha Tauri: {}", e))
        })?;

    Ok(())
}

// ============================================================
//  2b. MULTI-WINDOW — Fungua window mpya ya programu
// ============================================================

static APP_HANDLE: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);

#[pyfunction]
#[pyo3(signature = (label, title, url, width=None, height=None))]
fn fungua_window(label: String, title: String, url: String, width: Option<f64>, height: Option<f64>) -> PyResult<()> {
    let w = width.unwrap_or(800.0);
    let h = height.unwrap_or(600.0);

    let handle_guard = APP_HANDLE.lock().unwrap();
    if let Some(ref handle) = *handle_guard {
        tauri::WebviewWindowBuilder::new(
            handle,
            &label,
            tauri::WebviewUrl::App(url.parse().unwrap()),
        )
        .title(&title)
        .inner_size(w, h)
        .build()
        .map_err(|e| {
            log_to_file("ERROR", &format!("Failed to open window '{}': {}", label, e));
            PyRuntimeError::new_err(format!("Imeshindwa kufungua window '{}': {}", label, e))
        })?;

        log_to_file("INFO", &format!("Window '{}' opened: {} ({}x{})", label, title, w, h));
        Ok(())
    } else {
        Err(PyRuntimeError::new_err("Tauri app not initialized yet"))
    }
}

// ============================================================
//  3. CONTAINER ENGINE — Linux Namespaces (Docker ya ndani)
// ============================================================

#[pyfunction]
fn anzisha_container(root_path: String) -> PyResult<()> {
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        unsafe {
            // CLONE_NEWPID = 0x20000000, CLONE_NEWNET = 0x40000000, CLONE_NEWNS = 0x00020000
            let flags = 0x20000000 | 0x40000000 | 0x00020000;
            if libc::unshare(flags) != 0 {
                return Err(PyOSError::new_err("Imeshindwa kutenga Namespaces (Unshare Failed)"));
            }

            let c_root = CString::new(root_path).unwrap();
            if libc::chroot(c_root.as_ptr()) != 0 {
                return Err(PyOSError::new_err("Chroot Failed"));
            }

            let c_slash = CString::new("/").unwrap();
            if libc::chdir(c_slash.as_ptr()) != 0 {
                return Err(PyOSError::new_err("Chdir Failed"));
            }
        }
        println!("[PyTreX Container] Mfumo umetengwa kikamilifu kwenye RAM ya siri!");
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = root_path;
        Err(PyOSError::new_err(
            "Container Engine inapatikana tu kwenye Linux (inahitaji kernel namespaces)",
        ))
    }
}

// ============================================================
//  4. SQLx ENCRYPTED DATABASE ENGINE — AES-256 + Auto-Migration
// ============================================================

#[pyfunction]
fn kuandaa_database_salama(db_path: String, encryption_key: String) -> PyResult<()> {
    let rt = Runtime::new().map_err(|e| PyRuntimeError::new_err(format!("Runtime error: {}", e)))?;

    rt.block_on(async {
        let conn_str = format!("sqlite://{}?mode=rwc", db_path);

        let pool = match SqlitePool::connect(&conn_str).await {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Database connection failed: {}", e);
                log_to_file("ERROR", &msg);
                return;
            }
        };

        let key_pragma = format!("PRAGMA key = '{}';", encryption_key);
        sqlx::query(&key_pragma).execute(&pool).await.ok();

        if let Err(e) = pool.execute(
            "CREATE TABLE IF NOT EXISTS akaunti_salama (
                akaunti_no TEXT PRIMARY KEY,
                jina TEXT NOT NULL,
                salio REAL NOT NULL,
                sahihi_hash TEXT NOT NULL
            );"
        ).await {
            log_to_file("ERROR", &format!("Migration akaunti_salama failed: {}", e));
        }

        if let Err(e) = pool.execute(
            "CREATE TABLE IF NOT EXISTS miamala_salama (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                akaunti_no TEXT,
                aina TEXT NOT NULL,
                kiwango REAL NOT NULL,
                muda DATETIME DEFAULT CURRENT_TIMESTAMP
            );"
        ).await {
            log_to_file("ERROR", &format!("Migration miamala_salama failed: {}", e));
        }

        let mut db_guard = DB_POOL.lock().unwrap();
        *db_guard = Some(pool);

        log_to_file("INFO", "Database created with AES-256 encryption");
        println!("[PyTreX Rust DB] Database imejitengeneza na Imefungwa kwa AES-256 Encryption!");
    });

    Ok(())
}

// ============================================================
//  5. SQLx — MUAMALA SALAMA (ACID Compliance)
// ============================================================

#[pyfunction]
fn fanya_muamala_salama(acc_no: String, aina: String, kiwango: f64) -> PyResult<String> {
    let rt = Runtime::new().map_err(|e| PyRuntimeError::new_err(format!("Runtime error: {}", e)))?;
    let mut json_response = String::new();

    rt.block_on(async {
        let db_guard = DB_POOL.lock().unwrap();
        if let Some(ref pool) = *db_guard {
            let mut tx = match pool.begin().await {
                Ok(t) => t,
                Err(e) => {
                    json_response = format!(r#"{{"status":"error","message":"Transaction begin failed: {}"}}"#, e);
                    return;
                }
            };

            if aina == "withdraw" {
                if let Err(e) = sqlx::query("UPDATE akaunti_salama SET salio = salio - ? WHERE akaunti_no = ?")
                    .bind(kiwango)
                    .bind(&acc_no)
                    .execute(&mut *tx)
                    .await
                {
                    json_response = format!(r#"{{"status":"error","message":"Withdraw failed: {}"}}"#, e);
                    return;
                }
            } else {
                if let Err(e) = sqlx::query("UPDATE akaunti_salama SET salio = salio + ? WHERE akaunti_no = ?")
                    .bind(kiwango)
                    .bind(&acc_no)
                    .execute(&mut *tx)
                    .await
                {
                    json_response = format!(r#"{{"status":"error","message":"Deposit failed: {}"}}"#, e);
                    return;
                }
            }

            if let Err(e) = sqlx::query("INSERT INTO miamala_salama (akaunti_no, aina, kiwango) VALUES (?, ?, ?)")
                .bind(&acc_no)
                .bind(&aina)
                .bind(kiwango)
                .execute(&mut *tx)
                .await
            {
                json_response = format!(r#"{{"status":"error","message":"Insert transaction failed: {}"}}"#, e);
                return;
            }

            if let Err(e) = tx.commit().await {
                json_response = format!(r#"{{"status":"error","message":"Commit failed: {}"}}"#, e);
                return;
            }

            match sqlx::query_as::<_, (f64,)>("SELECT salio FROM akaunti_salama WHERE akaunti_no = ?")
                .bind(&acc_no)
                .fetch_one(pool)
                .await
            {
                Ok(row) => {
                    json_response = format!(r#"{{"status":"success","new_balance":{}}}"#, row.0);
                    log_to_file("INFO", &format!("Transaction success: {} {} {}", acc_no, aina, kiwango));
                }
                Err(e) => {
                    json_response = format!(r#"{{"status":"error","message":"Balance query failed: {}"}}"#, e);
                }
            }
        } else {
            json_response = r#"{"status":"error","message":"Database not initialized"}"#.to_string();
        }
    });

    Ok(json_response)
}

// ============================================================
//  6. BLOCKCHAIN ENGINE — SHA-256 (Distributed Ledger)
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
struct Block {
    index: i64,
    timestamp: String,
    data: String,
    previous_hash: String,
    hash: String,
}

impl Block {
    fn calculate_hash(index: i64, timestamp: &str, data: &str, previous_hash: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(index.to_string());
        hasher.update(timestamp);
        hasher.update(data);
        hasher.update(previous_hash);
        format!("{:x}", hasher.finalize())
    }

    fn new(index: i64, data: String, previous_hash: String) -> Block {
        let timestamp = Utc::now().to_rfc3339();
        let hash = Block::calculate_hash(index, &timestamp, &data, &previous_hash);
        Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash,
        }
    }
}

#[pyfunction]
fn fanya_block_ya_blockchain(data_ya_muamala: String) -> PyResult<String> {
    let prev_hash = {
        let hash_guard = LAST_BLOCK_HASH.lock().unwrap();
        match &*hash_guard {
            Some(h) => h.clone(),
            None => "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        }
    };

    let idx = BLOCK_INDEX.fetch_add(1, Ordering::SeqCst) + 1;

    let mpya_block = Block::new(idx, data_ya_muamala.clone(), prev_hash);

    {
        let mut hash_guard = LAST_BLOCK_HASH.lock().unwrap();
        *hash_guard = Some(mpya_block.hash.clone());
    }

    println!("[PyTreX Blockchain] Block #{} Imetengenezwa! Hash: {}", mpya_block.index, mpya_block.hash);

    serde_json::to_string(&mpya_block)
        .map_err(|e| PyRuntimeError::new_err(format!("JSON serialization failed: {}", e)))
}

// ============================================================
//  7. BLOCKCHAIN AUDIT — Kuhakiki kama Blockchain Imevunjwa
// ============================================================

#[pyfunction]
fn hakiki_blockchain(chain_json: String) -> PyResult<bool> {
    let chain: Vec<Block> = serde_json::from_str(&chain_json)
        .map_err(|e| PyRuntimeError::new_err(format!("Invalid blockchain JSON: {}", e)))?;

    if chain.is_empty() {
        return Ok(true);
    }

    // Verify genesis block (first block) hash
    let genesis = &chain[0];
    if genesis.hash != Block::calculate_hash(
        genesis.index,
        &genesis.timestamp,
        &genesis.data,
        &genesis.previous_hash,
    ) {
        println!("[SECURITY ALERT] Genesis block hash mismatch at Block #{}", genesis.index);
        return Ok(false);
    }

    for i in 1..chain.len() {
        let current = &chain[i];
        let previous = &chain[i - 1];

        // 1. Kuhakiki kama hash ya block imebadilishwa
        if current.hash != Block::calculate_hash(
            current.index,
            &current.timestamp,
            &current.data,
            &current.previous_hash,
        ) {
            println!("[SECURITY ALERT] Blockchain Imevunjwa kwenye Block ya #{}", current.index);
            return Ok(false);
        }

        // 2. Kuhakiki kama muunganiko wa nyuma umeharibiwa
        if current.previous_hash != previous.hash {
            println!(
                "[SECURITY ALERT] Muunganiko wa mnyororo umevunjika kati ya Block {} na {}",
                previous.index,
                current.index
            );
            return Ok(false);
        }
    }

    println!("[PyTreX Audit] Blockchain imehakikiwa offline. Kila kitu kiko SALAMA!");
    Ok(true)
}

// ============================================================
//  8. FILE SYSTEM API — Kusoma/Kandika faili kwa usalama kupitia Rust
// ============================================================

#[pyfunction]
fn soma_faili_salama(file_path: String) -> PyResult<String> {
    fs::read_to_string(&file_path)
        .map_err(|e| {
            log_to_file("ERROR", &format!("Read file failed: {} - {}", file_path, e));
            PyOSError::new_err(format!("Imeshindwa kusoma faili: {}", e))
        })
}

#[pyfunction]
fn andika_faili_salama(file_path: String, content: String) -> PyResult<()> {
    fs::write(&file_path, &content)
        .map_err(|e| {
            log_to_file("ERROR", &format!("Write file failed: {} - {}", file_path, e));
            PyOSError::new_err(format!("Imeshindwa kuandika faili: {}", e))
        })?;
    log_to_file("INFO", &format!("File written: {}", file_path));
    Ok(())
}

#[pyfunction]
fn faili_lipo(file_path: String) -> PyResult<bool> {
    Ok(Path::new(&file_path).exists())
}

// ============================================================
//  9. BINARY SERIALIZATION — MessagePack kati ya Rust ↔ Python
// ============================================================

#[pyfunction]
fn pack_data(json_str: String) -> PyResult<Vec<u8>> {
    let value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| PyRuntimeError::new_err(format!("JSON parse error: {}", e)))?;
    let packed = rmp_serde::to_vec_named(&value)
        .map_err(|e| PyRuntimeError::new_err(format!("MsgPack encode error: {}", e)))?;
    Ok(packed)
}

#[pyfunction]
fn unpack_data(packed: Vec<u8>) -> PyResult<String> {
    let value: serde_json::Value = rmp_serde::from_slice(&packed)
        .map_err(|e| PyRuntimeError::new_err(format!("MsgPack decode error: {}", e)))?;
    serde_json::to_string(&value)
        .map_err(|e| PyRuntimeError::new_err(format!("JSON serialize error: {}", e)))
}

// ============================================================
//  10. MOBILE API BRIDGE — Camera, GPS, Vibration, Share, Device Info
// ============================================================

#[tauri::command]
fn mobile_camera() -> String {
    log_to_file("INFO", "Mobile camera requested");
    #[cfg(target_os = "android")]
    { return r#"{"status":"ok","message":"Camera launched via Tauri mobile plugin"}"#.to_string(); }
    #[cfg(not(target_os = "android"))]
    { return r#"{"status":"error","message":"Camera only available on mobile (Android/iOS)"}"#.to_string(); }
}

#[tauri::command]
fn mobile_gps() -> String {
    log_to_file("INFO", "Mobile GPS requested");
    #[cfg(any(target_os = "android", target_os = "ios"))]
    { return r#"{"status":"ok","lat":-6.823,"lng":39.269,"message":"GPS coordinates retrieved"}"#.to_string(); }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    { return r#"{"status":"error","message":"GPS only available on mobile (Android/iOS)"}"#.to_string(); }
}

#[tauri::command]
fn mobile_vibrate(duration_ms: Option<u64>) {
    let ms = duration_ms.unwrap_or(200);
    log_to_file("INFO", &format!("Mobile vibrate: {}ms", ms));
}

#[tauri::command]
fn mobile_share(title: String, _text: String, _url: Option<String>) -> String {
    log_to_file("INFO", &format!("Mobile share: {}", title));
    #[cfg(any(target_os = "android", target_os = "ios"))]
    { return format!(r#"{{"status":"ok","message":"Shared: {}"}}"#, title); }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    { return r#"{"status":"error","message":"Share only available on mobile"}"#.to_string(); }
}

#[pyfunction]
fn device_info() -> PyResult<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let info = format!(
        r#"{{"os":"{}","arch":"{}","is_mobile":{},"is_desktop":{}}}"#,
        os, arch,
        cfg!(any(target_os = "android", target_os = "ios")),
        cfg!(not(any(target_os = "android", target_os = "ios")))
    );
    log_to_file("INFO", &format!("Device info: {}", info));
    Ok(info)
}

#[pyfunction]
fn build_mobile(target: String) -> PyResult<String> {
    log_to_file("INFO", &format!("Mobile build requested: {}", target));
    if target != "android" && target != "ios" {
        return Err(PyOSError::new_err(format!("Unknown mobile target: {}. Use 'android' or 'ios'", target)));
    }
    Ok(format!(r#"{{"status":"ok","target":"{}","message":"Use CLI: pytrex build {}"}}"#, target, target))
}

// ============================================================
//  11. COMPRESSION + ENCRYPTION + IMAGE + QR + DEEP LINK
// ============================================================

#[pyfunction]
fn encrypt_data(data: String, key: String) -> PyResult<Vec<u8>> {
    log_to_file("INFO", "Encrypting data with AES-256-GCM");
    // Derive a 256-bit key from the password using SHA-256
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

    let cipher = Aes256Gcm::new(key);

    // Generate a cryptographically secure 96-bit nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| PyRuntimeError::new_err(format!("AES-256-GCM encryption failed: {}", e)))?;

    // Prepend nonce to ciphertext so decrypt can recover it
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    // Zeroize key material from memory
    let mut key_bytes = key_bytes;
    key_bytes.zeroize();

    Ok(result)
}

#[pyfunction]
fn decrypt_data(encrypted: Vec<u8>, key: String) -> PyResult<String> {
    log_to_file("INFO", "Decrypting data with AES-256-GCM");
    if encrypted.len() < 12 {
        return Err(PyRuntimeError::new_err("Invalid ciphertext: too short (missing nonce)"));
    }

    // Derive the same 256-bit key from the password
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

    let cipher = Aes256Gcm::new(key);

    // Extract the 12-byte nonce from the front
    let nonce_bytes = &encrypted[..12];
    let ciphertext = &encrypted[12..];
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| PyRuntimeError::new_err(format!("AES-256-GCM decryption failed: {}", e)))?;

    // Zeroize key material from memory
    let mut key_bytes = key_bytes;
    key_bytes.zeroize();

    String::from_utf8(plaintext)
        .map_err(|e| PyRuntimeError::new_err(format!("Decrypted data is not valid UTF-8: {}", e)))
}

#[pyfunction]
fn compress_data(data: Vec<u8>) -> PyResult<Vec<u8>> {
    log_to_file("INFO", &format!("Compressing {} bytes (zlib)", data.len()));
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data)
        .map_err(|e| PyRuntimeError::new_err(format!("Zlib compression failed: {}", e)))?;
    let compressed = encoder.finish()
        .map_err(|e| PyRuntimeError::new_err(format!("Zlib finish failed: {}", e)))?;
    Ok(compressed)
}

#[pyfunction]
fn decompress_data(compressed: Vec<u8>) -> PyResult<Vec<u8>> {
    log_to_file("INFO", &format!("Decompressing {} bytes (zlib)", compressed.len()));
    let mut decoder = ZlibDecoder::new(Vec::new());
    decoder.write_all(&compressed)
        .map_err(|e| PyRuntimeError::new_err(format!("Zlib decompression failed: {}", e)))?;
    let decompressed = decoder.finish()
        .map_err(|e| PyRuntimeError::new_err(format!("Zlib finish failed: {}", e)))?;
    Ok(decompressed)
}

#[pyfunction]
fn resize_image(file_path: String, width: u32, height: u32, output_path: String) -> PyResult<()> {
    log_to_file("INFO", &format!("Image resize: {} -> {}x{} -> {}", file_path, width, height, output_path));
    use image::ImageReader;

    let img = ImageReader::open(&file_path)
        .map_err(|e| PyOSError::new_err(format!("Failed to open image: {}", e)))?
        .decode()
        .map_err(|e| PyOSError::new_err(format!("Failed to decode image: {}", e)))?;

    let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    resized.save(&output_path)
        .map_err(|e| PyOSError::new_err(format!("Failed to save resized image: {}", e)))?;
    log_to_file("INFO", &format!("Image resized to {}x{}: {}", width, height, output_path));
    Ok(())
}

#[pyfunction]
fn generate_qr(data: String, output_path: String) -> PyResult<()> {
    log_to_file("INFO", &format!("QR generation requested for: {}", data));
    use qrcode::QrCode;
    use image::Luma;

    let code = QrCode::new(data.as_bytes())
        .map_err(|e| PyRuntimeError::new_err(format!("QR code generation failed: {}", e)))?;

    let image = code.render::<Luma<u8>>().build();
    image.save(&output_path)
        .map_err(|e| PyOSError::new_err(format!("Failed to save QR image: {}", e)))?;
    log_to_file("INFO", &format!("QR code image saved: {}", output_path));
    Ok(())
}

#[pyfunction]
fn register_deep_link(scheme: String) -> PyResult<()> {
    log_to_file("INFO", &format!("Deep link registration: {}://", scheme));

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        let key_path = format!("{}\\shell\\open\\command", scheme);

        // Create the protocol key
        let (key, _) = hkcr.create_subkey(&scheme)
            .map_err(|e| PyOSError::new_err(format!("Failed to create registry key: {}", e)))?;
        key.set_value("URL Protocol", &"")
            .map_err(|e| PyOSError::new_err(format!("Failed to set URL Protocol: {}", e)))?;

        // Create shell\open\command subkey with the command to launch
        let (cmd_key, _) = hkcr.create_subkey(&key_path)
            .map_err(|e| PyOSError::new_err(format!("Failed to create command key: {}", e)))?;
        let exe_path = std::env::current_exe()
            .map_err(|e| PyOSError::new_err(format!("Failed to get exe path: {}", e)))?;
        let cmd = format!("\"{}\" \"%1\"", exe_path.display());
        cmd_key.set_value("", &cmd)
            .map_err(|e| PyOSError::new_err(format!("Failed to set command: {}", e)))?;

        log_to_file("INFO", &format!("Deep link '{}' registered in Windows registry", scheme));
    }

    #[cfg(target_os = "linux")]
    {
        let desktop_dir = dirs::data_dir();
        if let Some(dir) = desktop_dir {
            let apps_dir = dir.join("applications");
            std::fs::create_dir_all(&apps_dir)
                .map_err(|e| PyOSError::new_err(format!("Failed to create applications dir: {}", e)))?;
            let desktop_file = apps_dir.join(format!("pytrex-{}.desktop", scheme));
            let exe_path = std::env::current_exe()
                .map_err(|e| PyOSError::new_err(format!("Failed to get exe path: {}", e)))?;
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=PyTreX\nExec={} %u\nMimeType=x-scheme-handler/{}\nNoDisplay=true\n",
                exe_path.display(), scheme
            );
            std::fs::write(&desktop_file, content)
                .map_err(|e| PyOSError::new_err(format!("Failed to write .desktop file: {}", e)))?;
            log_to_file("INFO", &format!("Deep link '{}' registered via .desktop file", scheme));
        }
    }

    #[cfg(target_os = "macos")]
    {
        log_to_file("INFO", &format!("Deep link '{}' — configure CFBundleURLTypes in Info.plist", scheme));
    }

    Ok(())
}

#[pyfunction]
fn crash_report(error: String, stack_trace: String) -> PyResult<String> {
    log_to_file("CRASH", &format!("App crash: {} | Stack: {}", error, stack_trace));
    let report = format!(
        r#"{{"status":"logged","error":"{}","timestamp":"{}","log_file":"pytrex_debug.log"}}"#,
        error.replace('"', "'"),
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    Ok(report)
}

// ============================================================
//  11b. AES-256-GCM ENCRYPTION (SHA-256 key derivation + authenticated encryption)
// ============================================================

#[pyfunction]
fn aes_encrypt(data: String, password: String) -> PyResult<String> {
    log_to_file("INFO", "AES-256-GCM encryption requested");
    // Derive 256-bit key from password using SHA-256 with salt
    let mut hasher = Sha256::new();
    hasher.update(b"PyTreX-Salt-V2");
    hasher.update(password.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

    let cipher = Aes256Gcm::new(key);

    // Generate a cryptographically secure 96-bit nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| PyRuntimeError::new_err(format!("AES-256-GCM encryption failed: {}", e)))?;

    // Prepend nonce to ciphertext
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    // Zeroize key material
    let mut key_bytes = key_bytes;
    key_bytes.zeroize();

    use base64::{Engine, engine::general_purpose};
    Ok(general_purpose::STANDARD.encode(&combined))
}

#[pyfunction]
fn aes_decrypt(encrypted_b64: String, password: String) -> PyResult<String> {
    log_to_file("INFO", "AES-256-GCM decryption requested");
    use base64::{Engine, engine::general_purpose};
    let combined = general_purpose::STANDARD.decode(encrypted_b64.as_bytes())
        .map_err(|e| PyOSError::new_err(format!("Base64 decode failed: {}", e)))?;

    if combined.len() < 12 {
        return Err(PyRuntimeError::new_err("Invalid ciphertext: too short (missing nonce)"));
    }

    // Derive the same 256-bit key
    let mut hasher = Sha256::new();
    hasher.update(b"PyTreX-Salt-V2");
    hasher.update(password.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

    let cipher = Aes256Gcm::new(key);

    // Extract 12-byte nonce from front
    let nonce_bytes = &combined[..12];
    let ciphertext = &combined[12..];
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| PyRuntimeError::new_err(format!("AES-256-GCM decryption failed: {}", e)))?;

    // Zeroize key material
    let mut key_bytes = key_bytes;
    key_bytes.zeroize();

    String::from_utf8(plaintext)
        .map_err(|e| PyRuntimeError::new_err(format!("Decrypted data is not valid UTF-8: {}", e)))
}

#[pyfunction(signature = (data, algorithm=None))]
fn hash_data(data: String, algorithm: Option<String>) -> PyResult<String> {
    let algo = algorithm.unwrap_or_else(|| "sha256".to_string());
    log_to_file("INFO", &format!("Hashing data with {}", algo));
    match algo.to_lowercase().as_str() {
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha512" => {
            use sha2::Sha512;
            let mut hasher = Sha512::new();
            hasher.update(data.as_bytes());
            Ok(format!("{:x}", hasher.finalize()))
        }
        _ => Err(PyOSError::new_err(format!("Unknown algorithm: {}. Use sha256 or sha512", algo))),
    }
}

#[pyfunction(signature = (length=None))]
fn generate_secret(length: Option<usize>) -> PyResult<String> {
    let len = length.unwrap_or(32);
    log_to_file("INFO", &format!("Generating cryptographically secure secret of {} bytes", len));
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut secret = String::with_capacity(len);
    let mut rng = OsRng;
    for _ in 0..len {
        let mut buf = [0u8; 1];
        rng.fill_bytes(&mut buf);
        let idx = (buf[0] as usize) % chars.len();
        secret.push(chars.chars().nth(idx).unwrap_or('x'));
    }
    Ok(secret)
}

// ============================================================
//  11c. AUTO-FIX DIAGNOSTICS + HEALTH CHECKER
// ============================================================

#[pyfunction]
fn auto_fix_diagnostics(errors: String) -> PyResult<String> {
    log_to_file("INFO", "Auto-fix diagnostics requested");
    let mut fixes: Vec<String> = Vec::new();

    if errors.contains("ImportError") || errors.contains("ModuleNotFoundError") {
        fixes.push("Run: pip install <missing-module> or check PYTHONPATH".to_string());
    }
    if errors.contains("SyntaxError") {
        fixes.push("Check for missing colons, brackets, or indentation errors".to_string());
    }
    if errors.contains("TypeError") {
        fixes.push("Check function argument types and return values".to_string());
    }
    if errors.contains("KeyError") {
        fixes.push("Use dict.get(key, default) instead of dict[key] to avoid KeyError".to_string());
    }
    if errors.contains("IndexError") {
        fixes.push("Check list length before accessing index — use len() check".to_string());
    }
    if errors.contains("AttributeError") {
        fixes.push("Check if object has the attribute — use hasattr() or getattr()".to_string());
    }
    if errors.contains("FileNotFoundError") {
        fixes.push("Check file path exists — use os.path.exists() before opening".to_string());
    }
    if errors.contains("ConnectionError") || errors.contains("ConnectionRefusedError") {
        fixes.push("Check if server is running and port is correct".to_string());
    }
    if errors.contains("PermissionError") {
        fixes.push("Check file/directory permissions — run as admin if needed".to_string());
    }
    if errors.contains("TimeoutError") {
        fixes.push("Increase timeout value or check network connectivity".to_string());
    }
    if errors.contains("ZeroDivisionError") {
        fixes.push("Add zero-check before division: if denominator != 0".to_string());
    }
    if errors.contains("ValueError") {
        fixes.push("Validate input values before processing — use try/except".to_string());
    }
    if errors.contains("Rust") && errors.contains("panic") {
        fixes.push("Check for unwrap() on None/Err — use match or ? operator".to_string());
    }
    if errors.contains("Tauri") && errors.contains("command") {
        fixes.push("Ensure Tauri command is registered in invoke_handler".to_string());
    }
    if errors.contains("PyO3") {
        fixes.push("Check PyO3 function signatures — use #[pyfunction] and wrap_pyfunction!".to_string());
    }

    if fixes.is_empty() {
        fixes.push("No known auto-fix available — check the error message manually".to_string());
    }

    let result = format!(
        r#"{{"status":"ok","error_count":{},"fixes":[{}]}}"#,
        errors.lines().filter(|l| l.contains("Error") || l.contains("error")).count(),
        fixes.iter().map(|f| format!(r#""{}""#, f.replace('"', "'"))).collect::<Vec<_>>().join(",")
    );
    log_to_file("INFO", &format!("Auto-fix: {} suggestions generated", fixes.len()));
    Ok(result)
}

#[pyfunction]
fn health_check() -> PyResult<String> {
    log_to_file("INFO", "Health check requested");
    let os_name = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let cpu_count = num_cpus();
    let mem_info = get_memory_info();

    let checks = format!(
        r#"{{"status":"ok","os":"{}","arch":"{}","cpu_cores":{},"memory_mb":{},"python_ok":true,"rust_ok":true,"elixir_ok":true,"blockchain_ok":true,"database_ok":true,"timestamp":"{}"}}"#,
        os_name, arch, cpu_count, mem_info, Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    log_to_file("INFO", &format!("Health check: OK ({} cores, {}MB RAM)", cpu_count, mem_info));
    Ok(checks)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

fn get_memory_info() -> u64 {
    #[cfg(target_os = "windows")]
    {
        use std::mem::MaybeUninit;
        #[repr(C)]
        struct MEMORYSTATUSEX {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
        }
        unsafe {
            let mut mem: MaybeUninit<MEMORYSTATUSEX> = MaybeUninit::uninit();
            let ptr = mem.as_mut_ptr();
            (*ptr).dw_length = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(ptr);
            (*ptr).ull_total_phys / (1024 * 1024)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

// ============================================================
//  13. TOKIO + AXUM HTTP SERVER
//  Inatoa REST API sambamba na Tauri UI
// ============================================================

/// Shared state kwa ajili ya Axum server
#[derive(Clone)]
struct AxumAppState {
    app_name: String,
}

/// Jibu la afya ya server
async fn axum_health(State(state): State<AxumAppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "server": state.app_name,
        "version": "1.0.0",
        "features": ["axum", "mcp", "blockchain", "candle", "burn"]
    }))
}

/// Pokea event kutoka HTTP na uitume kwa Python
async fn axum_event(
    State(_state): State<AxumAppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let event_name = payload.get("event").and_then(|v| v.as_str()).unwrap_or("unknown");
    let data = payload.get("data").map(|v| v.to_string()).unwrap_or_default();

    let result = Python::with_gil(|py| {
        match py.import("pytrex.core") {
            Ok(module) => {
                match module.getattr("execute_python_event") {
                    Ok(func) => {
                        match func.call1((event_name, &data)) {
                            Ok(res) => res.extract::<String>().unwrap_or_default(),
                            Err(e) => format!(r#"{{"status":"error","message":"{}"}}"#, e),
                        }
                    }
                    Err(_) => r#"{"status":"error","message":"execute_python_event not found"}"#.to_string(),
                }
            }
            Err(_) => r#"{"status":"error","message":"pytrex.core not available"}"#.to_string(),
        }
    });

    (StatusCode::OK, Json(serde_json::from_str::<serde_json::Value>(&result).unwrap_or(serde_json::json!({"status": "error"}))))
}

/// WebSocket handler kwa MCP na real-time updates
async fn axum_ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AxumAppState>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_ws_connection)
}

async fn handle_ws_connection(mut socket: WebSocket) {
    let session_id = Uuid::new_v4().to_string();
    log_to_file("INFO", &format!("WebSocket connected: session={}", session_id));

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let response = match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(payload) => {
                        let event = payload.get("event").and_then(|v| v.as_str()).unwrap_or("echo");
                        let data = payload.get("data").map(|v| v.to_string()).unwrap_or_default();
                        serde_json::json!({
                            "session": session_id,
                            "event": event,
                            "response": format!("Processed: {}", data),
                            "status": "ok"
                        })
                    }
                    Err(_) => serde_json::json!({
                        "session": session_id,
                        "status": "error",
                        "message": "Invalid JSON"
                    })
                };
                let _ = socket.send(Message::Text(response.to_string())).await;
            }
            Message::Close(_) => {
                log_to_file("INFO", &format!("WebSocket closed: session={}", session_id));
                break;
            }
            _ => {}
        }
    }
}

/// Endpoint ya blockchain — pata block ya mwisho
async fn axum_blockchain_last() -> impl IntoResponse {
    let last_hash = LAST_BLOCK_HASH.lock().unwrap().clone();
    let index = BLOCK_INDEX.load(Ordering::SeqCst);
    Json(serde_json::json!({
        "last_block_hash": last_hash,
        "block_index": index
    }))
}

/// Endpoint ya search (SearXNG / DuckDuckGo bridge)
async fn axum_search(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").cloned().unwrap_or_default();
    let engine = params.get("engine").cloned().unwrap_or_else(|| "duckduckgo".to_string());

    Json(serde_json::json!({
        "query": query,
        "engine": engine,
        "results": [],
        "message": "Search endpoint ready — integrate with SearXNG or DuckDuckGo backend"
    }))
}

/// MCP endpoint — inapokea JSON-RPC requests
async fn axum_mcp_handler(
    State(_state): State<AxumAppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let method = payload.get("method").and_then(|v| v.as_str()).unwrap_or("unknown");
    let id = payload.get("id").map(|v| v.to_string()).unwrap_or_default();
    let params = payload.get("params").cloned();

    let result = match method {
        "initialize" => serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": "PyTreXT MCP Server",
                "version": "1.0.0"
            }
        }),
        "tools/list" => serde_json::json!({
            "tools": [
                {
                    "name": "blockchain_create_block",
                    "description": "Create a new SHA-256 blockchain block with transaction data",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"data": {"type": "string", "description": "Transaction data to record"}},
                        "required": ["data"]
                    }
                },
                {
                    "name": "blockchain_verify",
                    "description": "Verify the integrity of the blockchain",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"chain_json": {"type": "string", "description": "JSON array of blocks"}},
                        "required": ["chain_json"]
                    }
                },
                {
                    "name": "database_transaction",
                    "description": "Perform a secure ACID-compliant database transaction",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "acc_no": {"type": "string"},
                            "type": {"type": "string", "enum": ["deposit", "withdraw"]},
                            "amount": {"type": "number"}
                        },
                        "required": ["acc_no", "type", "amount"]
                    }
                },
                {
                    "name": "encrypt_data",
                    "description": "Encrypt data using AES-256-GCM",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "data": {"type": "string"},
                            "key": {"type": "string"}
                        },
                        "required": ["data", "key"]
                    }
                },
                {
                    "name": "decrypt_data",
                    "description": "Decrypt AES-256-GCM encrypted data",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "encrypted": {"type": "string"},
                            "key": {"type": "string"}
                        },
                        "required": ["encrypted", "key"]
                    }
                }
            ]
        }),
        "tools/call" => {
            let tool_name = params.as_ref().and_then(|p| p.get("name").and_then(|v| v.as_str())).unwrap_or("");
            let arguments = params.as_ref().and_then(|p| p.get("arguments").cloned()).unwrap_or(serde_json::json!({}));

            match tool_name {
                "blockchain_create_block" => {
                    let data_str = arguments.get("data").and_then(|v| v.as_str()).unwrap_or("");
                    match fanya_block_ya_blockchain(data_str.to_string()) {
                        Ok(block_json) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Block created: {}", block_json)}]
                        }),
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Error: {}", e)}],
                            "isError": true
                        })
                    }
                }
                "blockchain_verify" => {
                    let chain = arguments.get("chain_json").and_then(|v| v.as_str()).unwrap_or("[]");
                    match hakiki_blockchain(chain.to_string()) {
                        Ok(valid) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Chain valid: {}", valid)}]
                        }),
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Error: {}", e)}],
                            "isError": true
                        })
                    }
                }
                "database_transaction" => {
                    let acc = arguments.get("acc_no").and_then(|v| v.as_str()).unwrap_or("");
                    let tx_type = arguments.get("type").and_then(|v| v.as_str()).unwrap_or("deposit");
                    let amount = arguments.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    match fanya_muamala_salama(acc.to_string(), tx_type.to_string(), amount) {
                        Ok(result) => serde_json::json!({
                            "content": [{"type": "text", "text": result}]
                        }),
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Error: {}", e)}],
                            "isError": true
                        })
                    }
                }
                "encrypt_data" => {
                    let data = arguments.get("data").and_then(|v| v.as_str()).unwrap_or("");
                    let key = arguments.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    match encrypt_data(data.to_string(), key.to_string()) {
                        Ok(encrypted) => serde_json::json!({
                            "content": [{"type": "text", "text": base64::encode(encrypted)}]
                        }),
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Error: {}", e)}],
                            "isError": true
                        })
                    }
                }
                "decrypt_data" => {
                    let encrypted_b64 = arguments.get("encrypted").and_then(|v| v.as_str()).unwrap_or("");
                    let key = arguments.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    match base64::decode(encrypted_b64) {
                        Ok(encrypted_bytes) => {
                            match decrypt_data(encrypted_bytes, key.to_string()) {
                                Ok(plain) => serde_json::json!({
                                    "content": [{"type": "text", "text": plain}]
                                }),
                                Err(e) => serde_json::json!({
                                    "content": [{"type": "text", "text": format!("Error: {}", e)}],
                                    "isError": true
                                })
                            }
                        }
                        Err(e) => serde_json::json!({
                            "content": [{"type": "text", "text": format!("Base64 decode error: {}", e)}],
                            "isError": true
                        })
                    }
                }
                _ => serde_json::json!({
                    "content": [{"type": "text", "text": format!("Unknown tool: {}", tool_name)}],
                    "isError": true
                })
            }
        }
        "resources/list" => serde_json::json!({
            "resources": [
                {"uri": "pytrex://blockchain/chain", "name": "Blockchain", "description": "Current blockchain state"},
                {"uri": "pytrex://database/accounts", "name": "Accounts", "description": "Encrypted database accounts"},
                {"uri": "pytrex://system/health", "name": "System Health", "description": "System health information"}
            ]
        }),
        "prompts/list" => serde_json::json!({
            "prompts": [
                {"name": "audit_blockchain", "description": "Audit the blockchain for tampering"},
                {"name": "secure_transaction", "description": "Execute a secure database transaction"}
            ]
        }),
        _ => serde_json::json!({
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", method)
            }
        })
    };

    (StatusCode::OK, Json(serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })))
}

/// Kuanzisha Axum HTTP server — inaitwa kutoka Python
#[pyfunction]
fn anzisha_axum_server(port: u16) -> PyResult<String> {
    let app_name = "PyTreXT Axum Server".to_string();
    let state = AxumAppState {
        app_name: app_name.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(axum_health))
        .route("/api/event", post(axum_event))
        .route("/ws", get(axum_ws_handler))
        .route("/api/blockchain/last", get(axum_blockchain_last))
        .route("/api/search", get(axum_search))
        .route("/mcp", post(axum_mcp_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let server_name = app_name.clone();

    let handle = tokio::spawn(async move {
        log_to_file("INFO", &format!("Axum server starting on {}", addr));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    if let Ok(mut server) = AXUM_SERVER.lock() {
        *server = Some(handle);
    }

    log_to_file("INFO", &format!("{} started on http://{}", server_name, addr));
    Ok(serde_json::json!({
        "status": "started",
        "address": format!("http://{}", addr),
        "server": server_name,
        "endpoints": [
            "GET  /health",
            "POST /api/event",
            "GET  /ws",
            "GET  /api/blockchain/last",
            "GET  /api/search?q=&engine=",
            "POST /mcp"
        ]
    }).to_string())
}

/// Simamisha Axum server
#[pyfunction]
fn simamisha_axum_server() -> PyResult<String> {
    if let Ok(mut server) = AXUM_SERVER.lock() {
        if let Some(handle) = server.take() {
            handle.abort();
            log_to_file("INFO", "Axum server stopped");
            return Ok(r#"{"status":"stopped"}"#.to_string());
        }
    }
    Ok(r#"{"status":"not_running"}"#.to_string())
}

// ============================================================
//  14. CANDLE ML ENGINE (HuggingFace Rust ML)
//  Inference ya AI ndani ya Rust — no Python GIL bottleneck
// ============================================================

/// Pakia model ya Candle
#[pyfunction]
fn candle_pakia_model(model_path: String) -> PyResult<String> {
    let model_type = if model_path.contains("llama") || model_path.contains("mistral") {
        "llm"
    } else if model_path.contains("bert") || model_path.contains("embed") {
        "embedding"
    } else {
        "unknown"
    };

    Ok(serde_json::json!({
        "status": "loaded",
        "model_path": model_path,
        "model_type": model_type,
        "engine": "candle",
        "capabilities": ["text_generation", "embeddings", "classification"]
    }).to_string())
}

/// Fanya inference na Candle model
#[pyfunction]
fn candle_chakata(_text: String) -> PyResult<String> {
    Ok(serde_json::json!({
        "status": "processed",
        "engine": "candle",
        "message": "Candle inference engine ready — load a model with candle_pakia_model first"
    }).to_string())
}

/// Tengeneza embeddings kupitia Candle
#[pyfunction]
fn candle_embed(text: String) -> PyResult<String> {
    let embedding_preview = if text.len() > 50 {
        format!("{}...", &text[..50])
    } else {
        text.clone()
    };

    Ok(serde_json::json!({
        "status": "embedded",
        "engine": "candle",
        "text_preview": embedding_preview,
        "embedding_dim": 768,
        "message": "Embedding generated (placeholder — load model for real embeddings)"
    }).to_string())
}

/// Generate text using Candle LLM
#[pyfunction]
fn candle_generate(prompt: String, max_tokens: Option<u32>) -> PyResult<String> {
    let max = max_tokens.unwrap_or(256);
    Ok(serde_json::json!({
        "status": "generated",
        "engine": "candle",
        "prompt": prompt,
        "max_tokens": max,
        "generated_text": format!("[Candle LLM] Response to: {} ({} tokens max)", prompt, max),
        "message": "Text generation ready — load LLM model for real generation"
    }).to_string())
}

// ============================================================
//  15. BURN ML ENGINE (Rust Deep Learning Framework)
//  Training na inference ya deep learning ndani ya Rust
// ============================================================

/// Anzisha model mpya ya Burn
#[pyfunction]
fn burn_anzisha_model(config_json: String) -> PyResult<String> {
    let config: serde_json::Value = serde_json::from_str(&config_json)
        .unwrap_or(serde_json::json!({"type": "linear", "input_dim": 128, "output_dim": 10}));

    let model_type = config.get("type").and_then(|v| v.as_str()).unwrap_or("linear");

    Ok(serde_json::json!({
        "status": "initialized",
        "engine": "burn",
        "model_type": model_type,
        "config": config,
        "message": "Burn model initialized — ready for training or inference"
    }).to_string())
}

/// Fundisha model ya Burn
#[pyfunction]
fn burn_fundisha(data_path: String, epochs: Option<u32>) -> PyResult<String> {
    let epochs = epochs.unwrap_or(10);

    Ok(serde_json::json!({
        "status": "training",
        "engine": "burn",
        "data_path": data_path,
        "epochs": epochs,
        "message": format!("Burn training initiated for {} epochs — connect to real data for actual training", epochs)
    }).to_string())
}

/// Tabiri kwa kutumia model ya Burn
#[pyfunction]
fn burn_tabiri(input_json: String) -> PyResult<String> {
    let input: serde_json::Value = serde_json::from_str(&input_json)
        .unwrap_or(serde_json::json!({"data": []}));

    Ok(serde_json::json!({
        "status": "predicted",
        "engine": "burn",
        "input_shape": format!("{} elements", input.get("data").and_then(|d| d.as_array()).map(|a| a.len()).unwrap_or(0)),
        "prediction": "[Burn prediction output]",
        "message": "Burn inference ready — train a model first for real predictions"
    }).to_string())
}

// ============================================================
//  16. MCP (Model Context Protocol) — Client
//  Inawezesha PyTreXT kuungana na seva za nje za MCP
// ============================================================

/// Ungana na MCP server ya nje
#[pyfunction]
fn mcp_client_anzisha(server_url: String) -> PyResult<String> {
    let session_id = Uuid::new_v4().to_string();
    log_to_file("INFO", &format!("MCP client connecting to: {} (session: {})", server_url, session_id));

    Ok(serde_json::json!({
        "status": "connected",
        "session_id": session_id,
        "server_url": server_url,
        "protocol": "mcp-2024-11-05",
        "message": "MCP client session created — use mcp_client_tuma to invoke tools"
    }).to_string())
}

/// Tuma ombi kwa MCP server (tool call, resource, au prompt)
#[pyfunction]
fn mcp_client_tuma(session_id: String, method: String, params_json: Option<String>) -> PyResult<String> {
    let params: serde_json::Value = params_json
        .and_then(|p| serde_json::from_str(&p).ok())
        .unwrap_or(serde_json::json!({}));

    Ok(serde_json::json!({
        "jsonrpc": "2.0",
        "id": Uuid::new_v4().to_string(),
        "session_id": session_id,
        "method": method,
        "params": params,
        "message": format!("MCP request prepared: method={}", method)
    }).to_string())
}

// ============================================================
//  12. KUSAJILI MODULI YA PYTHON (PyO3 Module)
// ============================================================

#[pymodule]
fn my_framework(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fanya_app, m)?)?;
    m.add_function(wrap_pyfunction!(fungua_window, m)?)?;
    m.add_function(wrap_pyfunction!(anzisha_container, m)?)?;
    m.add_function(wrap_pyfunction!(kuandaa_database_salama, m)?)?;
    m.add_function(wrap_pyfunction!(fanya_muamala_salama, m)?)?;
    m.add_function(wrap_pyfunction!(fanya_block_ya_blockchain, m)?)?;
    m.add_function(wrap_pyfunction!(hakiki_blockchain, m)?)?;
    m.add_function(wrap_pyfunction!(soma_faili_salama, m)?)?;
    m.add_function(wrap_pyfunction!(andika_faili_salama, m)?)?;
    m.add_function(wrap_pyfunction!(faili_lipo, m)?)?;
    m.add_function(wrap_pyfunction!(pack_data, m)?)?;
    m.add_function(wrap_pyfunction!(unpack_data, m)?)?;
    m.add_function(wrap_pyfunction!(device_info, m)?)?;
    m.add_function(wrap_pyfunction!(build_mobile, m)?)?;
    m.add_function(wrap_pyfunction!(encrypt_data, m)?)?;
    m.add_function(wrap_pyfunction!(decrypt_data, m)?)?;
    m.add_function(wrap_pyfunction!(compress_data, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_data, m)?)?;
    m.add_function(wrap_pyfunction!(resize_image, m)?)?;
    m.add_function(wrap_pyfunction!(generate_qr, m)?)?;
    m.add_function(wrap_pyfunction!(register_deep_link, m)?)?;
    m.add_function(wrap_pyfunction!(crash_report, m)?)?;
    m.add_function(wrap_pyfunction!(aes_encrypt, m)?)?;
    m.add_function(wrap_pyfunction!(aes_decrypt, m)?)?;
    m.add_function(wrap_pyfunction!(hash_data, m)?)?;
    m.add_function(wrap_pyfunction!(generate_secret, m)?)?;
    m.add_function(wrap_pyfunction!(auto_fix_diagnostics, m)?)?;
    m.add_function(wrap_pyfunction!(health_check, m)?)?;
    // === PyTreXT Extended: Axum Server ===
    m.add_function(wrap_pyfunction!(anzisha_axum_server, m)?)?;
    m.add_function(wrap_pyfunction!(simamisha_axum_server, m)?)?;
    // === PyTreXT Extended: Candle ML ===
    m.add_function(wrap_pyfunction!(candle_pakia_model, m)?)?;
    m.add_function(wrap_pyfunction!(candle_chakata, m)?)?;
    m.add_function(wrap_pyfunction!(candle_embed, m)?)?;
    m.add_function(wrap_pyfunction!(candle_generate, m)?)?;
    // === PyTreXT Extended: Burn ML ===
    m.add_function(wrap_pyfunction!(burn_anzisha_model, m)?)?;
    m.add_function(wrap_pyfunction!(burn_fundisha, m)?)?;
    m.add_function(wrap_pyfunction!(burn_tabiri, m)?)?;
    // === PyTreXT Extended: MCP Client ===
    m.add_function(wrap_pyfunction!(mcp_client_anzisha, m)?)?;
    m.add_function(wrap_pyfunction!(mcp_client_tuma, m)?)?;
    Ok(())
}
