//! mtechos-mcrecv — mpokeaji wa multicast
//! (Rust — hakuna Python).
//!
//! Huendeshwa kwenye PC ya lengo (initramfs/live):
//!   1. Soma manifest (JSON line) kutoka group 239.255.0.1:5007
//!   2. Poka chunks (`session|seq_be32|payload`) hadi `END|session`
//!   3. Thibitisha sha256 + urefu
//!   4. Andika kwenye disk (`OF`, default `/dev/sda`) au faili (test)
//!   5. Tuma ACK kwa `FUNDI_API` (`/api/mc/ack`) — best-effort
//!
//! Env: `FUNDI_MC_GROUP`, `FUNDI_MC_PORT`, `OF`, `FUNDI_API`, `MAC`,
//! `FUNDI_MC_TEST`, `FUNDI_MC_TEST_OF`.

use std::fs::OpenOptions;
use std::io::Write;
use std::net::UdpSocket;
use std::process::ExitCode;
use std::time::Duration;

use sha2::{Digest, Sha256};

const BUFSZ: usize = 65536;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// ACK ni best-effort — kosa halizuii uandikaji wa disk.
fn send_ack(api: &str, mac: &str, sha: &str, received: usize, ok: bool) {
    let body = serde_json::json!({
        "mac": mac,
        "sha256": sha,
        "received_bytes": received,
        "ok": ok,
    })
    .to_string();
    let req = ureq_post(&format!("{api}/api/mc/ack"), &body);
    if let Err(e) = req {
        eprintln!("ACK fail: {e}");
    }
}

/// POST nyepesi kwa `std::net::TcpStream` (hakuna runtime ya async kwenye
/// initramfs; `reqwest` ni ya server, si ya client hii ndogo).
fn ureq_post(url: &str, body: &str) -> std::io::Result<String> {
    use std::io::{Read, Write as _};
    use std::net::TcpStream;

    let rest = url.strip_prefix("http://").unwrap_or(url);
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let mut stream = TcpStream::connect(authority)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes())?;
    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;
    Ok(resp)
}

fn main() -> ExitCode {
    let group = env_or("FUNDI_MC_GROUP", "239.255.0.1");
    let port: u16 = env_or("FUNDI_MC_PORT", "5007").parse().unwrap_or(5007);
    let of = env_or("OF", "/dev/sda");
    let api = env_or("FUNDI_API", "http://127.0.0.1:8080");
    let mac = env_or("MAC", "00:00:00:00:00:00");

    let sock = match UdpSocket::bind(("0.0.0.0", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("bind imeshindwa: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = sock.join_multicast_v4(
        &group.parse().expect("group si IPv4 sahihi"),
        &"0.0.0.0".parse().unwrap(),
    ) {
        eprintln!("join_multicast imeshindwa: {e}");
        return ExitCode::FAILURE;
    }
    println!("[fundi_recv] listeners on {group}:{port} → {of}");

    let mut buf = vec![0u8; BUFSZ];

    // 1. Manifest
    let (name, expect_sha, expect_bytes) = loop {
        let n = match sock.recv(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf[..n]) else {
            continue;
        };
        let Some(name) = v["name"].as_str() else { continue };
        let Some(sha) = v["sha256"].as_str() else { continue };
        break (
            name.to_string(),
            sha.to_string(),
            v["bytes"].as_u64().unwrap_or(0) as usize,
        );
    };
    println!("[fundi_recv] image={name} bytes={expect_bytes}");

    // 2. Chunks — `session|seq_be32|payload`
    let mut session: Option<String> = None;
    let mut chunks: std::collections::BTreeMap<u32, Vec<u8>> = std::collections::BTreeMap::new();
    loop {
        let n = match sock.recv(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let data = &buf[..n];
        if data.starts_with(b"END|") {
            break;
        }
        let Some(sep) = data.iter().position(|&b| b == b'|') else {
            continue;
        };
        let head = String::from_utf8_lossy(&data[..sep]).to_string();
        if data.len() < sep + 1 + 4 {
            continue;
        }
        if session.is_none() {
            session = Some(head.clone());
        }
        if session.as_deref() != Some(head.as_str()) {
            continue;
        }
        let seq = u32::from_be_bytes([
            data[sep + 1],
            data[sep + 2],
            data[sep + 3],
            data[sep + 4],
        ]);
        chunks.insert(seq, data[sep + 5..].to_vec());
    }

    let blob: Vec<u8> = chunks.values().flatten().copied().collect();
    let sha = format!("{:x}", Sha256::digest(&blob));
    let ok = sha == expect_sha && blob.len() == expect_bytes;
    println!("[fundi_recv] received={} sha_ok={ok}", blob.len());

    if !ok {
        send_ack(&api, &mac, &expect_sha, blob.len(), false);
        return ExitCode::FAILURE;
    }

    // 3. Andika (disk au faili la majaribio)
    let is_device = of.starts_with("/dev/") && std::env::var("FUNDI_MC_TEST").is_err();
    if is_device {
        match OpenOptions::new().write(true).open(&of) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(&blob).and_then(|_| f.sync_all()) {
                    eprintln!("disk write imeshindwa: {e}");
                    send_ack(&api, &mac, &expect_sha, blob.len(), false);
                    return ExitCode::FAILURE;
                }
                println!("[fundi_recv] disk {of} imeandikwa");
            }
            Err(e) => {
                eprintln!("disk {of} haifunguki: {e}");
                send_ack(&api, &mac, &expect_sha, blob.len(), false);
                return ExitCode::FAILURE;
            }
        }
    } else {
        let out = env_or("FUNDI_MC_TEST_OF", "/tmp/fundi_mc_out.bin");
        if let Err(e) = std::fs::write(&out, &blob) {
            eprintln!("write imeshindwa: {e}");
            return ExitCode::FAILURE;
        }
        println!("[fundi_recv] test output: {out}");
    }

    send_ack(&api, &mac, &expect_sha, blob.len(), true);
    ExitCode::SUCCESS
}
