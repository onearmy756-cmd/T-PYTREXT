//! m42_wifi.rs — MODULE 42: WI-FI SECURITY
//!
//! Spec: "Inalinda uunganisho wa Wi-Fi, inagundua honeypot. Twist: inagundua
//! Wi-Fi honeypot (bandia) na kuonya."
//!
//! Hii si catalog: inasoma hali halisi ya wireless (/proc/net/wireless), na
//! ina engine ya evil-twin/honeypot: SSID moja yenye BSSID nyingi au encryption
//! iliyoshushwa (WPA2 → OPEN) = honeypot. Uamuzi halisi kwenye scan data.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct M42Wifi;

/// AP moja kutoka scan.
#[derive(Debug, Clone)]
pub struct Ap {
    pub ssid: String,
    pub bssid: String,
    pub enc: String, // "open" | "wep" | "wpa" | "wpa2" | "wpa3"
    pub rssi: i32,
}

fn enc_rank(e: &str) -> u8 {
    match e.to_ascii_lowercase().as_str() {
        "wpa3" => 4,
        "wpa2" => 3,
        "wpa" => 2,
        "wep" => 1,
        _ => 0,
    }
}

impl TsModule for M42Wifi {
    fn id(&self) -> u16 {
        42
    }
    fn name(&self) -> &'static str {
        "Wi-Fi Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Husoma hali halisi ya Wi-Fi na kugundua honeypot/evil-twin (SSID moja, \
         BSSID nyingi au encryption iliyoshushwa)."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "scan" => self.scan(params),
            other => Err(format!("op '{other}' haipo: audit|scan")),
        }
    }
}

impl M42Wifi {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(42, self.name());
        match std::fs::read_to_string("/proc/net/wireless") {
            Ok(txt) => {
                let lines: Vec<&str> = txt.lines().filter(|l| l.contains(':')).collect();
                r.finding(
                    "wireless_ifaces",
                    format!("Interfaces za wireless: {}", lines.len()),
                    2,
                );
                for l in &lines {
                    let iface = l.split(':').next().unwrap_or("").trim();
                    r.finding("iface", format!("Wireless interface: {iface}"), 2);
                }
            }
            Err(_) => {
                r.finding(
                    "no_wireless",
                    "Hakuna interface ya wireless kwenye host hii (/proc/net/wireless haipo)",
                    5,
                );
            }
        }
        r.summary = "Hali ya Wi-Fi imechunguzwa.".into();
        Ok(r)
    }

    /// Changanua scan: gundua evil-twin/honeypot.
    fn scan(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(42, self.name());
        let arr = params
            .get("aps")
            .and_then(|v| v.as_array())
            .ok_or("params.aps (array) inahitajika")?;
        let aps: Vec<Ap> = arr
            .iter()
            .map(|a| Ap {
                ssid: a.get("ssid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                bssid: a.get("bssid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                enc: a.get("enc").and_then(|v| v.as_str()).unwrap_or("open").to_string(),
                rssi: a.get("rssi").and_then(|v| v.as_i64()).unwrap_or(-100) as i32,
            })
            .collect();
        let mut by_ssid: HashMap<String, Vec<&Ap>> = HashMap::new();
        for a in &aps {
            by_ssid.entry(a.ssid.clone()).or_default().push(a);
        }
        let mut alerts = 0;
        for (ssid, list) in &by_ssid {
            let bssids: std::collections::HashSet<&str> =
                list.iter().map(|a| a.bssid.as_str()).collect();
            if bssids.len() > 1 {
                let max_rank = list.iter().map(|a| enc_rank(&a.enc)).max().unwrap_or(0);
                let min_rank = list.iter().map(|a| enc_rank(&a.enc)).min().unwrap_or(0);
                alerts += 1;
                r.finding(
                    "evil_twin",
                    format!(
                        "SSID '{ssid}' ina BSSID {} tofauti — evil-twin/honeypot inawezekana",
                        bssids.len()
                    ),
                    75,
                );
                if min_rank < max_rank {
                    alerts += 1;
                    r.finding(
                        "enc_downgrade",
                        format!(
                            "SSID '{ssid}': encryption imeshushwa ({} hadi {}) — honeypot",
                            max_rank, min_rank
                        ),
                        85,
                    );
                }
            }
            for a in list {
                if enc_rank(&a.enc) == 0 {
                    r.finding(
                        "open_network",
                        format!("SSID '{}' ({}) ni OPEN — hatari", a.ssid, a.bssid),
                        40,
                    );
                }
            }
        }
        r.summary = format!("APs {} zimechanganuliwa; alerts {alerts}.", aps.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evil_twin_inagundulika() {
        let m = M42Wifi;
        let aps = json!([
            {"ssid":"Cafe","bssid":"aa:bb:cc:00:00:01","enc":"wpa2","rssi":-40},
            {"ssid":"Cafe","bssid":"aa:bb:cc:00:00:02","enc":"open","rssi":-50}
        ]);
        let r = m.run("", &json!({ "op": "scan", "aps": aps })).unwrap();
        assert!(r.findings.iter().any(|f| f["kind"] == "evil_twin"));
        assert!(r.findings.iter().any(|f| f["kind"] == "enc_downgrade"));
    }

    #[test]
    fn ssid_moja_haishtuki() {
        let m = M42Wifi;
        let aps = json!([{"ssid":"Home","bssid":"aa:bb:cc:00:00:01","enc":"wpa2"}]);
        let r = m.run("", &json!({ "op": "scan", "aps": aps })).unwrap();
        assert!(!r.findings.iter().any(|f| f["kind"] == "evil_twin"));
    }

    #[test]
    fn enc_rank_inapanga() {
        assert!(enc_rank("wpa3") > enc_rank("wpa2"));
        assert!(enc_rank("open") == 0);
    }
}
