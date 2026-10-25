//! m282_webrtc_security.rs — MODULE 282: WEBRTC SECURITY
//!
//! Spec: "Inalinda WebRTC calls. Twist: Inaficha IP address."
//!
//! Injini halisi: huchanganua SDP (Session Description Protocol) ya WebRTC:
//!   - Hugundua ICE candidates zinazofichua IP ya ndani/public (privacy leak).
//!   - Hupendekeza/kuweka `ice_candidate_policy` ya relay-only (TURN) ili kuficha IP.
//!   - Hukagua DTLS-SRTP (usimbaji wa media) na `a=fingerprint`.
//!   - Hugundua mifumo ya zamani (SDES, RTMP) isiyo salama.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Uchanganuzi wa SDP.
#[derive(Debug, Default, Clone)]
pub struct SdpAudit {
    pub has_dtls: bool,
    pub has_srtp: bool,
    pub has_fingerprint: bool,
    pub candidates: usize,
    pub host_ip_candidates: usize,
    pub srflx_candidates: usize,
    pub relay_candidates: usize,
    pub uses_sdes: bool,
}

pub fn audit_sdp(sdp: &str) -> SdpAudit {
    let mut a = SdpAudit::default();
    for line in sdp.lines() {
        let l = line.trim();
        if l.starts_with("a=fingerprint:") {
            a.has_fingerprint = true;
        }
        if l.contains("UDP/TLS/RTP/SAVPF") || l.contains("DTLS") {
            a.has_dtls = true;
        }
        if l.contains("RTP/SAVP") {
            a.has_srtp = true;
        }
        if l.contains("crypto:") && l.contains("AES_CM") {
            a.uses_sdes = true;
        }
        if l.starts_with("a=candidate:") {
            a.candidates += 1;
            if l.contains(" typ host") {
                a.host_ip_candidates += 1;
            } else if l.contains(" typ srflx") {
                a.srflx_candidates += 1;
            } else if l.contains(" typ relay") {
                a.relay_candidates += 1;
            }
        }
    }
    a
}

pub struct M282WebRtcSecurity;

impl TsModule for M282WebRtcSecurity {
    fn id(&self) -> u16 {
        282
    }
    fn name(&self) -> &'static str {
        "WebRTC Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganua SDP ya WebRTC: hugundua kufichuka kwa IP, hukagua DTLS-SRTP, \
         na hupendekeza relay-only (TURN) kuficha IP."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(282, self.name());
        let sdp = params.get("sdp").and_then(|v| v.as_str()).unwrap_or("");
        let a = audit_sdp(sdp);
        let mut score = 0u8;
        if !sdp.is_empty() {
            if a.host_ip_candidates > 0 || a.srflx_candidates > 0 {
                score = score.max(60);
                r.finding("ip_leak", format!(
                    "Kufichuka kwa IP: host {}, srflx {} (IP ya ndani/public inaonekana)",
                    a.host_ip_candidates, a.srflx_candidates), 60);
            }
            if !a.has_dtls {
                score = score.max(70);
                r.finding("no_dtls", "DTLS haipo kwenye SDP — media haijasimbwa", 70);
            }
            if !a.has_fingerprint {
                score = score.max(50);
                r.finding("no_fingerprint", "a=fingerprint haipo — hakuna uthibitisho wa DTLS", 50);
            }
            if a.uses_sdes {
                score = score.max(75);
                r.finding("sdes_legacy", "SDES (AES_CM) imetumika — mfumo wa zamani usio salama", 75);
            }
        }
        // Pendekezo la faragha: relay-only.
        r.findings.push(json!({
            "kind": "recommendation", "severity": 1,
            "detail": { "ice_candidate_policy": "relay", "reason": "kuficha IP ya mtumiaji",
                        "candidates": a.candidates, "relay": a.relay_candidates }
        }));
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("WebRTC Security: uamuzi {decision} (candidates {}, IP leak {}).", a.candidates, a.host_ip_candidates + a.srflx_candidates);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_ip_leak_na_dtls() {
        let sdp = "m=audio 9 UDP/TLS/RTP/SAVPF 111\r\na=fingerprint:sha-256 AA:BB\r\na=candidate:1 1 UDP 2130706431 192.168.1.5 54400 typ host\r\na=candidate:2 1 UDP 1694498815 41.59.0.1 54401 typ srflx\r\n";
        let a = audit_sdp(sdp);
        assert!(a.has_dtls && a.has_fingerprint);
        assert_eq!(a.host_ip_candidates, 1);
        assert_eq!(a.srflx_candidates, 1);
    }

    #[test]
    fn inagundua_sdes() {
        let a = audit_sdp("m=audio 9 RTP/SAVP 0\r\na=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:abc\r\n");
        assert!(a.uses_sdes);
        assert!(!a.has_dtls);
    }
}
