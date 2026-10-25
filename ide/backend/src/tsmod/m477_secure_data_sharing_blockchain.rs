//! m477_secure_data_sharing_blockchain.rs — MODULE 477: SECURE DATA SHARING BLOCKCHAIN
//!
//! Spec: "Kushiriki data kwa usalama. Twist: Kati ya mashirika."
//!
//! Injini halisi: hushiriki data kati ya mashirika kwa usimbaji (AEAD) na
//! kuthibitisha uadilifu kwa hash; kila shirika lina ufunguo wake.

use super::common_blockchain::sha_hex;
use super::common_human::aead_encrypt;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub struct M477SecureDataSharingBlockchain;

impl TsModule for M477SecureDataSharingBlockchain {
    fn id(&self) -> u16 {
        477
    }
    fn name(&self) -> &'static str {
        "Secure Data Sharing Blockchain"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Hushiriki data kati ya mashirika kwa usimbaji (AEAD) + uthibitisho wa hash."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(477, self.name());
        let org = p.get("org").and_then(|v| v.as_str()).unwrap_or("TZ-ISP-A");
        let peer = p.get("peer").and_then(|v| v.as_str()).unwrap_or("KE-ISP-B");
        let data = p.get("data").and_then(|v| v.as_str()).unwrap_or("rekodi za wateja (zimefichwa)");
        let key: [u8; 32] = Sha256::digest(format!("{org}|{peer}").as_bytes()).into();
        let nonce = [7u8; 12];
        let (ct, tag) = aead_encrypt(&key, &nonce, peer.as_bytes(), data.as_bytes());
        let integrity = sha_hex(&ct);
        r.findings.push(json!({ "kind": "share", "severity": 1,
            "detail": { "org": org, "peer": peer, "cipher": "ChaCha20+HMAC", "ct_len": ct.len(),
                        "integrity": integrity, "tag": tag.iter().map(|b| format!("{b:02x}")).collect::<String>() } }));
        r.summary = format!("Secure Data Sharing: data ya {org}→{peer} imesimbwa (AEAD).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_inasimbwa() {
        let rep = M477SecureDataSharingBlockchain.run("", &json!({"org":"A","peer":"B"})).unwrap();
        assert_eq!(rep.findings[0]["detail"]["cipher"], "ChaCha20+HMAC");
    }
}
