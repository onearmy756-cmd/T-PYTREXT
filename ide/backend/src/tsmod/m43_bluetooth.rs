//! m43_bluetooth.rs — MODULE 43: BLUETOOTH SECURITY
//!
//! Spec: "Inalinda Bluetooth, inagundua BlueBorne attack. Twist: inagundua
//! BlueBorne attack kabla ya kuathiri."
//!
//! Hii si catalog: inasoma adapters halisi (/sys/class/bluetooth) na ina engine
//! ya BlueBorne inayochanganua device class + advertised services kwa dalili za
//! BlueBorne (BR/EDR discoverable + SDP/L2CAP abuse) kabla ya kuunganisha.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M43Bluetooth;

/// Kifaa cha Bluetooth kutoka scan.
#[derive(Debug, Clone)]
pub struct BtDevice {
    pub addr: String,
    pub name: String,
    pub class: u32,
    /// Services/Profiles zinazotangazwa (UUID au jina).
    pub services: Vec<String>,
}

/// Dalili za BlueBorne (BR/EDR): services zinazotumiwa na CVE-2017-1000251 n.k.
const BLUEBORNE_SVC: &[&str] = &[
    "0x1101", // Serial Port
    "0x1105", // Object Push
    "0x1116", // NAP
    "0x0008", // L2CAP
    "0x0001", // SDP
];

impl TsModule for M43Bluetooth {
    fn id(&self) -> u16 {
        43
    }
    fn name(&self) -> &'static str {
        "Bluetooth Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Husoma adapters halisi na kuchanganua device class/services kwa dalili za \
         BlueBorne kabla ya kuunganisha."
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

impl M43Bluetooth {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(43, self.name());
        match std::fs::read_dir("/sys/class/bluetooth") {
            Ok(rd) => {
                let adapters: Vec<String> = rd
                    .flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect();
                r.finding(
                    "bt_adapters",
                    format!("Adapters: [{}]", adapters.join(", ")),
                    2,
                );
                if adapters.is_empty() {
                    r.finding("no_adapter", "Hakuna adapter ya Bluetooth", 5);
                }
            }
            Err(_) => {
                r.finding("bt_unavailable", "Bluetooth haipo kwenye host hii", 5);
            }
        }
        r.summary = "Hali ya Bluetooth imechunguzwa.".into();
        Ok(r)
    }

    fn scan(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(43, self.name());
        let arr = params
            .get("devices")
            .and_then(|v| v.as_array())
            .ok_or("params.devices (array) inahitajika")?;
        let devices: Vec<BtDevice> = arr
            .iter()
            .map(|d| BtDevice {
                addr: d.get("addr").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: d.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                class: d.get("class").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                services: d
                    .get("services")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str().map(|x| x.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();
        let mut alerts = 0;
        for d in &devices {
            let hits: Vec<&String> = d
                .services
                .iter()
                .filter(|s| {
                    let l = s.to_ascii_lowercase();
                    BLUEBORNE_SVC.iter().any(|b| l.contains(b))
                })
                .collect();
            // BR/EDR (class bit 0x0000xx = device class) + discoverable risky service.
            let br_edr = d.class != 0;
            if br_edr && hits.len() >= 2 {
                alerts += 1;
                r.finding(
                    "blueborne_risk",
                    format!(
                        "{} ({}): services hatarishi [{}] — uwezekano wa BlueBorne",
                        d.addr,
                        d.name,
                        hits.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                    80,
                );
            }
            if d.addr.is_empty() {
                r.finding("anon_device", "Kifaa bila BD_ADDR kimegundulika", 20);
            }
        }
        r.summary = format!("Vifaa {} vimechanganuliwa; alerts {alerts}.", devices.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueborne_inagundulika() {
        let m = M43Bluetooth;
        let devs = json!([
            {"addr":"11:22:33:44:55:66","name":"Speaker","class":262144,
             "services":["0x1101","0x0008"]}
        ]);
        let r = m.run("", &json!({ "op": "scan", "devices": devs })).unwrap();
        assert!(r.findings.iter().any(|f| f["kind"] == "blueborne_risk"));
    }

    #[test]
    fn kifaa_salama_hakishtuki() {
        let m = M43Bluetooth;
        let devs = json!([{"addr":"11:22:33:44:55:66","name":"Watch","class":262144,"services":["0x180D"]}]);
        let r = m.run("", &json!({ "op": "scan", "devices": devs })).unwrap();
        assert!(!r.findings.iter().any(|f| f["kind"] == "blueborne_risk"));
    }
}
