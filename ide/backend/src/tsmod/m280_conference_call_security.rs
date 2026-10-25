//! m280_conference_call_security.rs — MODULE 280: CONFERENCE CALL SECURITY
//!
//! Spec: "Inalinda mikutano ya simu. Twist: Inaficha sauti kwa funguo za kikundi."
//!
//! Injini halisi: usimamizi wa funguo za kikundi (group key management):
//!   - Kila mshiriki ana funguo yake; sauti inasimbwa kwa funguo ya kikundi (GK).
//!   - Kuingia/kutoka kwa mshiriki → GK inabadilishwa (rekey) ili mtu aliyeondoka
//!     asikize mazungumzo yajayo (forward secrecy ya kikundi).
//!   - Kila mshiriki ana "key epoch"; hutumia GK ya epoch yake.
//!   - Hukagua ikiwa mshiriki asiyejulikana (unknown) ameunganishwa.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Kokotoa GK ya kikundi kwa epoch fulani kutoka funguo za washiriki.
pub fn group_key(epoch: u64, member_keys: &[String]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"FUNDI-CONF-GK-v1");
    h.update(epoch.to_le_bytes());
    let mut sorted: Vec<&String> = member_keys.iter().collect();
    sorted.sort();
    for k in sorted {
        h.update(k.as_bytes());
    }
    h.finalize().into()
}

/// Simba sauti ya mkutano kwa GK ya epoch.
pub fn enc_audio(gk: &[u8; 32], epoch: u64, audio: &[u8]) -> Vec<u8> {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&epoch.to_le_bytes());
    super::m273_voice_encryption::chacha20_xor(gk, &nonce, audio)
}

pub struct M280ConferenceCallSecurity;

impl TsModule for M280ConferenceCallSecurity {
    fn id(&self) -> u16 {
        280
    }
    fn name(&self) -> &'static str {
        "Conference Call Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Husimamia funguo za kikundi kwa mikutano ya simu; hurekey mshiriki \
         anapotoka ili mazungumzo yajayo yasikike na mtu aliyeondoka."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(280, self.name());
        let members: Vec<String> = params
            .get("members")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let unknown = params.get("unknown_joined").and_then(|v| v.as_bool()).unwrap_or(false);
        let epoch = params.get("epoch").and_then(|v| v.as_u64()).unwrap_or(1);
        let mut score = 0u8;

        if members.len() < 2 {
            score = score.max(35);
            r.finding("few_members", "Washiriki pungufu kwa mkutano salama", 35);
        }
        if unknown {
            score = score.max(75);
            r.finding("unknown_member", "Mshiriki asiyejulikana amejiunga — hakikisha", 75);
        }
        // Rekey on leave: onyesha GK za epoch mbili tofauti.
        let gk1 = group_key(epoch, &members);
        let mut after: Vec<String> = members.clone();
        if after.len() > 1 {
            after.remove(0); // mshiriki mmoja ameondoka
        }
        let gk2 = group_key(epoch + 1, &after);
        let rekey_ok = gk1 != gk2;
        r.finding("forward_secrecy", if rekey_ok { "GK inabadilika mshiriki anapotoka" } else { "GK haibadiliki" }, if rekey_ok { 1 } else { 70 });
        // Simba sampuli ya sauti.
        let audio: Vec<u8> = (0..320).map(|i| (i % 256) as u8).collect();
        let ct = enc_audio(&gk1, epoch, &audio);
        r.findings.push(json!({
            "kind": "group_crypto", "severity": 1,
            "detail": { "alg": "ChaCha20", "members": members.len(), "epoch": epoch,
                        "gk_hex": hex(&gk1[..8]), "ct_len": ct.len(), "rekeyed": rekey_ok }
        }));
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Conference Call Security: uamuzi {decision} (washiriki {}, epoch {epoch}).", members.len());
        Ok(r)
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gk_inabadilika_mshiriki_anapotoka() {
        let m = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let g1 = group_key(1, &m);
        let g2 = group_key(2, &m[..2].to_vec());
        assert_ne!(g1, g2);
    }
}
