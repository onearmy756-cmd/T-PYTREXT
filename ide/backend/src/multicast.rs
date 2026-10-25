//! Multicast deploy (P2) — picha moja kwa PCs nyingi kwa mtando mmoja.
//!
//! Server (agent) : UDP sender — inatuma image kwa group 239.255.0.1:5007
//! Client (PC)    : `mtechos-mcrecv` (PXE initramfs/live) — inapokea na kuandika disk
//! Ukadiriaji    : ACK counts kutoka kwa clients (`/api/mc/ack` kutoka recv script)
//!
//! Njia salama:
//!   - Manifest (jina, bytes, sha256) inatumwa kwanza (JSON line)
//!   - Baada ya kutuma: clients ripoti sha256 yao → server inalinganisha
//!   - HITL kazi kama kawaida — multicast ni stage moja ya pipeline

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::path::Path;
use std::sync::Arc;

pub const MC_GROUP: &str = "239.255.0.1";
pub const MC_PORT: u16 = 5007;
const CHUNK: usize = 1400; // chini ya MTU kwa usalama
const PACKETS_PER_SEC: usize = 400;
/// Pause kati ya packets (µs) — huzuia kufurika buffer ya receiver (drops za UDP).
const PACING_US: u64 = 100;

#[derive(Serialize, Deserialize, Clone)]
pub struct McManifest {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    pub total_chunks: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct McAck {
    pub mac: String,
    pub sha256: String,
    pub received_bytes: u64,
    pub ok: bool,
}

/// Tuma file kwa multicast group (blocking call ndani ya spawn).
/// Huru­dua mara `FUNDI_MC_REPEAT` (default 2) ili clients wanaokosa chunks
/// (UDP haina retransmission) waweze kuzipata; client dedupe kwa seq.
pub fn send_file(path: &Path, session: &str) -> anyhow::Result<McManifest> {
    let repeat = std::env::var("FUNDI_MC_REPEAT")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(2);
    send_file_repeat(path, session, repeat)
}

pub fn send_file_repeat(path: &Path, session: &str, repeat: u32) -> anyhow::Result<McManifest> {
    let data = std::fs::read(path).with_context(|| format!("soma {}", path.display()))?;
    let sha = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&data);
        format!("{:x}", h.finalize())
    };
    let total_chunks = ((data.len() + CHUNK - 1) / CHUNK) as u64;

    let manifest = McManifest {
        name: path
            .file_name()
            .map(|s| s.to_string_lossy().into())
            .unwrap_or_default(),
        bytes: data.len() as u64,
        sha256: sha.clone(),
        total_chunks,
    };

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    let addr = format!("{MC_GROUP}:{MC_PORT}");

    // TTL ya multicast (scope ya LAN pekee)
    let _ = sock.set_multicast_loop_v4(true);

    // 1. Manifest (JSON + newline) — mara ya kwanza
    let mline = format!("{}\n", serde_json::to_string(&manifest)?);
    sock.send_to(mline.as_bytes(), &addr)?;

    // 2. Data chunks — rudia `repeat` mara; client dedupe kwa seq (bitmap).
    //    UDP haina retransmission, hivyo marudio ndiyo njia ya kuaminika LAN.
    let mut sent = 0u64;
    for pass in 0..repeat {
        if pass > 0 {
            // manifest ya marudio: client aliyechelewa ajiunge
            sock.send_to(mline.as_bytes(), &addr)?;
        }
        for (i, chunk) in data.chunks(CHUNK).enumerate() {
            // header: session|seq(4 bytes BE)|payload
            // LAZIMA 4 bytes (u32) — mc_recv.c/mtechos-mcrecv zinasoma 4.
            let mut pkt = Vec::with_capacity(CHUNK + 32);
            pkt.extend_from_slice(session.as_bytes());
            pkt.push(b'|');
            pkt.extend_from_slice(&(i as u32).to_be_bytes());
            pkt.extend_from_slice(chunk);
            sock.send_to(&pkt, &addr)?;
            sent += 1;
            // Pacing: bila hii, burst inaweza kufurika buffer ya receiver (drops).
            std::thread::sleep(std::time::Duration::from_micros(PACING_US));
            if sent % PACKETS_PER_SEC as u64 == 0 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        // END baada ya kila pass: client anayemaliza mapema anaacha kusikiliza.
        let end = format!("END|{session}|{sha}\n");
        sock.send_to(end.as_bytes(), &addr)?;
    }

    Ok(manifest)
}

/// Hifadhi ACK kutoka client (inaitwa na route /api/mc/ack)
#[derive(Default)]
pub struct AckBoard {
    pub acks: std::sync::Mutex<Vec<McAck>>,
}

impl AckBoard {
    pub fn record(&self, ack: McAck) {
        self.acks.lock().unwrap().push(ack);
    }
    pub fn count(&self) -> usize {
        self.acks.lock().unwrap().len()
    }
    pub fn verified(&self, sha: &str) -> usize {
        self.acks
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.ok && a.sha256 == sha)
            .count()
    }
    /// Jumla ya ACKs zilizothibitishwa (sha yoyote)
    pub fn ok_count(&self) -> usize {
        self.acks.lock().unwrap().iter().filter(|a| a.ok).count()
    }
}

pub type SharedAcks = Arc<AckBoard>;

/// Chunk pacing info kwa UI (packets sent)
pub fn stats_line(sent: usize, total: usize) -> String {
    format!("multicast: {sent}/{total} packets (group {MC_GROUP}:{MC_PORT})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let m = McManifest {
            name: "os.iso".into(),
            bytes: 123,
            sha256: "abc".into(),
            total_chunks: 1,
        };
        let line = serde_json::to_string(&m).unwrap();
        let back: McManifest = serde_json::from_str(&line).unwrap();
        assert_eq!(back.name, "os.iso");
        assert_eq!(back.bytes, 123);
    }

    #[test]
    fn ack_board_inakusanya() {
        let b = AckBoard::default();
        b.record(McAck {
            mac: "aa".into(),
            sha256: "x".into(),
            received_bytes: 1,
            ok: true,
        });
        b.record(McAck {
            mac: "bb".into(),
            sha256: "x".into(),
            received_bytes: 1,
            ok: false,
        });
        assert_eq!(b.count(), 2);
        assert_eq!(b.verified("x"), 1);
        assert_eq!(b.ok_count(), 1);
    }

    /// End-to-end HALISI: send_file → receiver ya UDP multicast (loopback) →
    /// reassembly → sha256. Inathibitisha protocol (manifest/chunks/END) inafanya kazi.
    #[test]
    fn send_file_multicast_roundtrip_halisi() {
        use std::net::UdpSocket;
        use std::sync::mpsc;
        use std::time::Duration;

        // data ya kutosha kwa chunks kadhaa (mfano 5000 bytes → 4 chunks)
        let dir = std::env::temp_dir().join(format!("fundi_mc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("img.bin");
        let payload: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload).unwrap();

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let group = MC_GROUP.to_string();
        let expect_payload = payload.clone();

        let handle = std::thread::spawn(move || {
            let sock = UdpSocket::bind(format!("0.0.0.0:{MC_PORT}")).expect("bind recv");
            sock.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
            let mreq = multicast_join(&group);
            sock.join_multicast_v4(&mreq.0, &mreq.1).expect("join");
            let mut expected: u64 = 0;
            let mut name = String::new();
            let mut sha_manifest = String::new();
            let mut chunks: Vec<Vec<u8>> = Vec::new();
            let mut session = String::new();
            let mut got_manifest = false;
            let mut buf = vec![0u8; 65536];
            let deadline = std::time::Instant::now() + Duration::from_secs(6);
            // Endelea kusikiliza hadi chunks ZOTE zipokee (au deadline). END ya pass
            // ya kwanza haitoshi — tunasubiri marudio yajaze mapengo.
            while std::time::Instant::now() < deadline {
                let (n, _) = match sock.recv_from(&mut buf) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let pkt = &buf[..n];
                if !got_manifest {
                    let line = String::from_utf8_lossy(pkt).to_string();
                    if line.contains("\"sha256\"") && line.contains("\"bytes\"") {
                        let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
                        expected = v["bytes"].as_u64().unwrap();
                        name = v["name"].as_str().unwrap().to_string();
                        sha_manifest = v["sha256"].as_str().unwrap().to_string();
                        let total = v["total_chunks"].as_u64().unwrap() as usize;
                        chunks = vec![Vec::new(); total];
                        got_manifest = true;
                    }
                    continue;
                }
                if pkt.starts_with(b"END|") {
                    if chunks.iter().all(|c| !c.is_empty()) {
                        break;
                    }
                    continue;
                }
                if let Some(pos) = pkt.iter().position(|&b| b == b'|') {
                    let sess = String::from_utf8_lossy(&pkt[..pos]).to_string();
                    if session.is_empty() {
                        session = sess;
                    } else if session != sess {
                        continue;
                    }
                    if pkt.len() < pos + 5 {
                        continue;
                    }
                    let seq = u32::from_be_bytes([
                        pkt[pos + 1], pkt[pos + 2], pkt[pos + 3], pkt[pos + 4],
                    ]) as usize;
                    if seq < chunks.len() && chunks[seq].is_empty() {
                        chunks[seq] = pkt[pos + 5..].to_vec();
                    }
                }
            }
            let joined: Vec<u8> = chunks.concat();
            assert_eq!(joined.len() as u64, expected, "bytes zote");
            assert_eq!(&joined[..], &expect_payload[..], "data inalingana");
            let sha = {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&joined);
                format!("{:x}", h.finalize())
            };
            assert_eq!(sha, sha_manifest, "sha256 inalingana");
            assert_eq!(name, "img.bin");
            tx.send(joined).unwrap();
        });

        // subiri receiver ijiunge kisha tuma
        std::thread::sleep(Duration::from_millis(800));
        let man = send_file_repeat(&path, "session-e2e", 2).expect("send_file");
        assert_eq!(man.bytes, payload.len() as u64);
        let got = rx.recv_timeout(Duration::from_secs(20)).expect("receiver data");
        assert_eq!(got.len(), payload.len());
        handle.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn multicast_join(group: &str) -> (std::net::Ipv4Addr, std::net::Ipv4Addr) {
        (
            group.parse().unwrap(),
            std::net::Ipv4Addr::UNSPECIFIED,
        )
    }
}
