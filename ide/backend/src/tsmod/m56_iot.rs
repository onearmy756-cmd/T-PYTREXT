//! m56_iot.rs — MODULE 56: IOT SECURITY
//!
//! Spec: "Inalinda vifaa vya IoT (smart cameras, locks, bulbs). Twist: inagundua
//! IoT device inayofanya DDoS."
//!
//! Hii si catalog: ina engine ya kugundua IoT DDoS kwa kuchanganua flows halisi
//! (conntrack) — kifaa kimoja kinachofungua uunganisho mwingi kwa malengo mengi
//! (fan-out) au SYN nyingi = bot. Pia husoma /proc/net/arp kwa vifaa vya IoT.

use super::engine;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// Kikomo cha malengo kwa kifaa kabla ya kuitwa DDoS bot.
const BOT_FANOUT: usize = 30;

pub struct M56Iot;

impl TsModule for M56Iot {
    fn id(&self) -> u16 {
        56
    }
    fn name(&self) -> &'static str {
        "IoT Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Husoma flows halisi (conntrack) na kugundua IoT device inayofanya DDoS \
         (fan-out kubwa kwa malengo mengi)."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "audit" => self.audit(),
            "ddos" => self.ddos(apply),
            other => Err(format!("op '{other}' haipo: audit|ddos")),
        }
    }
}

impl M56Iot {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(56, self.name());
        let arp = std::fs::read_to_string("/proc/net/arp").unwrap_or_default();
        let n = arp.lines().skip(1).filter(|l| !l.trim().is_empty()).count();
        r.finding("lan_devices", format!("Vifaa kwenye LAN (ARP): {n}"), 2);
        // OUI za kawaida za IoT (mifano) — onyesha vifaa vinavyowezekana.
        for line in arp.lines().skip(1) {
            let c: Vec<&str> = line.split_whitespace().collect();
            if c.len() >= 4 {
                let mac = c[3].to_lowercase();
                let oui = mac.split(':').take(3).collect::<Vec<_>>().join(":");
                if ["b8:27:eb", "dc:a6:32", "ec:fa:bc"].contains(&oui.as_str()) {
                    r.finding("iot_like", format!("{} ({}) — Raspberry Pi-class", c[0], mac), 10);
                }
            }
        }
        r.summary = format!("Vifaa {n} vimechunguzwa.");
        Ok(r)
    }

    fn ddos(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(56, self.name());
        let txt = engine::read_root_file("/proc/net/nf_conntrack")?;
        let mut dsts: HashMap<String, HashSet<String>> = HashMap::new();
        let mut syn: HashMap<String, usize> = HashMap::new();
        for line in txt.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                continue;
            }
            let proto = cols[2];
            let mut src = "";
            let mut dst = "";
            for c in &cols {
                if let Some(v) = c.strip_prefix("src=") {
                    if src.is_empty() {
                        src = v;
                    }
                } else if let Some(v) = c.strip_prefix("dst=") {
                    if dst.is_empty() {
                        dst = v;
                    }
                }
            }
            if !src.is_empty() && !dst.is_empty() {
                dsts.entry(src.to_string()).or_default().insert(dst.to_string());
                if proto == "tcp" && line.contains("SYN_SENT") {
                    *syn.entry(src.to_string()).or_insert(0) += 1;
                }
            }
        }
        let mut bots: Vec<String> = Vec::new();
        for (src, d) in &dsts {
            if d.len() >= BOT_FANOUT {
                bots.push(src.clone());
                r.finding(
                    "iot_ddos",
                    format!("{src}: malengo {} tofauti — IoT bot inawezekana", d.len()),
                    80,
                );
            }
        }
        for (src, n) in &syn {
            if *n >= 20 {
                if !bots.contains(src) {
                    bots.push(src.clone());
                }
                r.finding("syn_flood", format!("{src}: SYN {n} — flood"), 75);
            }
        }
        if apply && !bots.is_empty() {
            let mut done = 0;
            for b in &bots {
                if engine::fw_block(b).is_ok() {
                    done += 1;
                }
            }
            r.applied = true;
            r.dry_run = false;
            r.action("iot_quarantine", format!("Vifaa {done} vimezuiliwa (quarantine)"), true);
            r.summary = format!("IoT quarantine: vifaa {done} vimezuiwa.");
        } else {
            r.summary = format!("Vifaa {} vinavyowezekana kuwa bots.", bots.len());
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_inafanya_kazi() {
        let m = M56Iot;
        let r = m.run("", &json!({ "op": "audit" })).unwrap();
        assert_eq!(r.module, 56);
    }
}
