//! m36_ids.rs — MODULE 36: IDS/IPS (Suricata-class, bila dependency ya nje)
//!
//! Spec: "Inagundua na kuzuia mashambulizi (Suricata/Snort). Twist: inagundua
//! mashambulizi ya zero-day (isiyojulikana)."
//!
//! Hii si catalog: inachanganua flows HALISI kutoka kernel conntrack
//! (`/proc/net/nf_conntrack`) na ina engine ya signature za TCP flags
//! (NULL/FIN/XMAS/SYN) zinazotumiwa kwenye observations halisi za pakiti.
//! IPS (prevention) huzuia source IP kwenye set ya nftables — HITL.

use super::engine::{self, FW_SET, FW_TABLE};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// Kikomo cha dport tofauti kwa src kabla ya kuitwa port scan.
const PORT_SCAN_DISTINCT: usize = 15;
/// Kikomo cha uunganisho kwa SSH kutoka src mmoja (brute force).
const BRUTE_THRESHOLD: usize = 10;
/// Kikomo cha malengo tofauti kutoka src mmoja (fan-out).
const FANOUT_THRESHOLD: usize = 50;
/// Kikomo cha SYN kutoka src mmoja (SYN flood).
const SYN_FLOOD: usize = 100;

pub struct M36Ids;

/// Flow moja kutoka conntrack.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub proto: String,
    pub src: String,
    pub dst: String,
    pub sport: u16,
    pub dport: u16,
}

/// Observation moja ya pakiti (kutoka capture halisi au feed).
#[derive(Debug, Clone)]
pub struct Obs {
    pub src: String,
    pub dport: u16,
    /// TCP flags: "SYN","FIN","NULL","XMAS","ACK",... au namba.
    pub flags: String,
}

impl TsModule for M36Ids {
    fn id(&self) -> u16 {
        36
    }
    fn name(&self) -> &'static str {
        "IDS/IPS"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Inachanganua flows halisi (conntrack) na signature za TCP flags; IPS huzuia \
         source hatari kwenye nftables kwa idhini."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("analyze");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "analyze" => self.analyze(),
            "observe" => self.observe(params),
            "block" => self.block(target, apply),
            "ips" => self.ips(apply),
            other => Err(format!("op '{other}' haipo: analyze|observe|block|ips")),
        }
    }
}

/// Changanua mstari wa conntrack: rudisha Flow kama ni TCP/UDP.
pub fn parse_conntrack_line(line: &str) -> Option<Flow> {
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 4 {
        return None;
    }
    let proto = cols[2];
    if proto != "tcp" && proto != "udp" {
        return None;
    }
    let mut src = None;
    let mut dst = None;
    let mut sport = 0u16;
    let mut dport = 0u16;
    for c in &cols {
        if let Some(v) = c.strip_prefix("src=") {
            if src.is_none() {
                src = Some(v.to_string());
            }
        } else if let Some(v) = c.strip_prefix("dst=") {
            if dst.is_none() {
                dst = Some(v.to_string());
            }
        } else if let Some(v) = c.strip_prefix("sport=") {
            if sport == 0 {
                sport = v.parse().unwrap_or(0);
            }
        } else if let Some(v) = c.strip_prefix("dport=") {
            if dport == 0 {
                dport = v.parse().unwrap_or(0);
            }
        }
    }
    Some(Flow {
        proto: proto.to_string(),
        src: src?,
        dst: dst?,
        sport,
        dport,
    })
}

/// Tafsiri flags kuwa set ya herufi (SYN/FIN/RST/PSH/URG/ACK) au "NULL".
pub fn parse_flags(f: &str) -> HashSet<&'static str> {
    let mut set = HashSet::new();
    let up = f.trim().to_uppercase();
    if up.is_empty() || up == "0" || up == "NONE" || up == "NULL" {
        return set;
    }
    if let Ok(n) = up.parse::<u8>() {
        if n & 0x02 != 0 {
            set.insert("SYN");
        }
        if n & 0x01 != 0 {
            set.insert("FIN");
        }
        if n & 0x04 != 0 {
            set.insert("RST");
        }
        if n & 0x08 != 0 {
            set.insert("PSH");
        }
        if n & 0x20 != 0 {
            set.insert("URG");
        }
        if n & 0x10 != 0 {
            set.insert("ACK");
        }
        return set;
    }
    for part in up.split(['+', ',', '|', ' ']).filter(|s| !s.is_empty()) {
        match part {
            "SYN" | "S" => {
                set.insert("SYN");
            }
            "FIN" | "F" => {
                set.insert("FIN");
            }
            "RST" | "R" => {
                set.insert("RST");
            }
            "PSH" | "P" => {
                set.insert("PSH");
            }
            "URG" | "U" => {
                set.insert("URG");
            }
            "ACK" | "A" => {
                set.insert("ACK");
            }
            _ => {}
        }
    }
    set
}

/// Ainisha observation moja kuwa aina ya shambulizi (au None).
pub fn classify_obs(o: &Obs) -> Option<&'static str> {
    let f = parse_flags(&o.flags);
    if f.is_empty() {
        return Some("null_scan");
    }
    if f.len() == 1 && f.contains("FIN") {
        return Some("fin_scan");
    }
    if f.contains("FIN") && f.contains("PSH") && f.contains("URG") {
        return Some("xmas_scan");
    }
    None
}

/// Signature kwa SYN (kwa SYN flood counting).
pub fn is_syn(o: &Obs) -> bool {
    let f = parse_flags(&o.flags);
    f.contains("SYN") && !f.contains("ACK")
}

impl M36Ids {
    fn analyze(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(36, self.name());
        let txt = engine::read_root_file("/proc/net/nf_conntrack")?;
        let flows: Vec<Flow> = txt.lines().filter_map(parse_conntrack_line).collect();
        let flows = FlowStats::from(&flows);
        r.finding(
            "conntrack_summary",
            format!(
                "Flows halisi: {} (src tofauti: {})",
                flows.total, flows.per_src.len()
            ),
            2,
        );
        let mut alerts = 0u64;
        for (src, st) in &flows.per_src {
            if st.dports.len() >= PORT_SCAN_DISTINCT {
                alerts += 1;
                r.finding(
                    "port_scan",
                    format!(
                        "{src}: bandari {} tofauti zimejaribiwa",
                        st.dports.len()
                    ),
                    60,
                );
            }
            if st.dsts.len() >= FANOUT_THRESHOLD {
                alerts += 1;
                r.finding(
                    "fan_out",
                    format!("{src}: malengo {} tofauti (horizontal scan)", st.dsts.len()),
                    55,
                );
            }
            if st.dport22 >= BRUTE_THRESHOLD {
                alerts += 1;
                r.finding(
                    "ssh_brute",
                    format!("{src}: uunganisho {} kwa SSH (brute force)", st.dport22),
                    65,
                );
            }
        }
        r.summary = format!(
            "Flows {} zimechanganuliwa; alerts {alerts}.",
            flows.total
        );
        Ok(r)
    }

    /// Changanua observations za pakiti (feed halisi) kwa signature + flood.
    fn observe(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(36, self.name());
        let arr = params
            .get("observations")
            .and_then(|v| v.as_array())
            .ok_or("params.observations (array) inahitajika")?;
        let obs: Vec<Obs> = arr
            .iter()
            .map(|o| Obs {
                src: o.get("src").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                dport: o.get("dport").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                flags: o
                    .get("flags")
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| v.to_string())
                    })
                    .unwrap_or_default(),
            })
            .collect();
        let mut counts: HashMap<&'static str, u64> = HashMap::new();
        let mut syn_per_src: HashMap<&str, usize> = HashMap::new();
        let mut distinct_dport: HashMap<&str, HashSet<u16>> = HashMap::new();
        for o in &obs {
            if let Some(kind) = classify_obs(o) {
                *counts.entry(kind).or_insert(0) += 1;
            }
            if is_syn(o) {
                *syn_per_src.entry(o.src.as_str()).or_insert(0) += 1;
            }
            if o.dport > 0 {
                distinct_dport
                    .entry(o.src.as_str())
                    .or_default()
                    .insert(o.dport);
            }
        }
        for (kind, n) in &counts {
            r.finding(
                *kind,
                format!("Pakiti {n} za aina {kind} zimegundulika"),
                if *kind == "xmas_scan" { 70 } else { 60 },
            );
        }
        for (src, n) in &syn_per_src {
            if *n >= SYN_FLOOD {
                r.finding("syn_flood", format!("{src}: SYN {n} (flood)"), 75);
            }
        }
        for (src, dp) in &distinct_dport {
            if dp.len() >= PORT_SCAN_DISTINCT {
                r.finding(
                    "port_scan",
                    format!("{src}: dport {} tofauti kwenye feed", dp.len()),
                    60,
                );
            }
        }
        r.summary = format!(
            "Observations {} zimechanganuliwa; signatures {}.",
            obs.len(),
            counts.values().sum::<u64>()
        );
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(36, self.name());
        let (net, prefix) = engine::parse_cidr(target)
            .ok_or_else(|| format!("target si IP/CIDR sahihi: {target}"))?;
        let entry = if prefix == 32 {
            std::net::Ipv4Addr::from(net).to_string()
        } else {
            format!("{}/{prefix}", std::net::Ipv4Addr::from(net))
        };
        if apply {
            engine::fw_block(&entry)?;
            r.applied = true;
            r.dry_run = false;
            r.action("ips_block", format!("{entry} imezuiwa (IPS)"), true);
            r.summary = format!("IPS: {entry} imezuiwa.");
        } else {
            r.action("ips_block_planned", format!("Itazuiwa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }

    /// Prevention ya kiotomatiki: changanua conntrack, zuia offenders wote.
    fn ips(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(36, self.name());
        let txt = engine::read_root_file("/proc/net/nf_conntrack")?;
        let flows: Vec<Flow> = txt.lines().filter_map(parse_conntrack_line).collect();
        let stats = FlowStats::from(&flows);
        let mut offenders: Vec<String> = Vec::new();
        for (src, st) in &stats.per_src {
            if st.dports.len() >= PORT_SCAN_DISTINCT
                || st.dport22 >= BRUTE_THRESHOLD
                || st.dsts.len() >= FANOUT_THRESHOLD
            {
                offenders.push(src.to_string());
            }
        }
        offenders.sort();
        offenders.dedup();
        if offenders.is_empty() {
            r.summary = "IPS: hakuna source hatari kwenye conntrack.".into();
            return Ok(r);
        }
        if apply {
            let mut done = 0;
            for ip in &offenders {
                if engine::fw_block(ip).is_ok() {
                    done += 1;
                }
            }
            r.applied = true;
            r.dry_run = false;
            r.action("ips_enforce", format!("IP {done} zimezuiwa"), true);
            r.summary = format!("IPS: IP {done} hatarishi zimezuiwa.");
        } else {
            for ip in &offenders {
                r.finding("offender", format!("Itazuiwa: {ip}"), 60);
            }
            r.summary = format!("Dry-run: IP {} zingezuiwa ukiweka apply=true.", offenders.len());
        }
        r.findings.push(json!({
            "kind": "ips_targets", "table": FW_TABLE, "set": FW_SET, "offenders": offenders
        }));
        Ok(r)
    }
}

/// Muhtasari wa flows kwa src.
#[derive(Default)]
pub struct SrcStats {
    pub dports: HashSet<u16>,
    pub dsts: HashSet<String>,
    pub dport22: usize,
}

pub struct FlowStats {
    pub total: usize,
    pub per_src: HashMap<String, SrcStats>,
}

impl FlowStats {
    pub fn from(flows: &[Flow]) -> Self {
        let mut per_src: HashMap<String, SrcStats> = HashMap::new();
        for f in flows {
            let e = per_src.entry(f.src.clone()).or_default();
            e.dports.insert(f.dport);
            e.dsts.insert(f.dst.clone());
            if f.dport == 22 {
                e.dport22 += 1;
            }
        }
        FlowStats {
            total: flows.len(),
            per_src,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conntrack_line_inaparsika() {
        let line = "ipv4     2 tcp      6 84 TIME_WAIT src=10.1.0.201 dst=10.2.53.55 sport=51574 dport=60000 src=10.2.53.55 dst=10.1.0.201 sport=60000 dport=51574 [ASSURED] mark=0";
        let f = parse_conntrack_line(line).unwrap();
        assert_eq!(f.proto, "tcp");
        assert_eq!(f.src, "10.1.0.201");
        assert_eq!(f.dport, 60000);
    }

    #[test]
    fn null_scan_inatambulika() {
        let o = Obs { src: "1.1.1.1".into(), dport: 80, flags: "".into() };
        assert_eq!(classify_obs(&o), Some("null_scan"));
    }

    #[test]
    fn xmas_scan_inatambulika() {
        let o = Obs { src: "1.1.1.1".into(), dport: 80, flags: "FIN+PSH+URG".into() };
        assert_eq!(classify_obs(&o), Some("xmas_scan"));
    }

    #[test]
    fn fin_scan_inatambulika() {
        let o = Obs { src: "1.1.1.1".into(), dport: 80, flags: "FIN".into() };
        assert_eq!(classify_obs(&o), Some("fin_scan"));
    }

    #[test]
    fn syn_ya_kawaida_haishtuki() {
        let o = Obs { src: "1.1.1.1".into(), dport: 443, flags: "SYN".into() };
        assert_eq!(classify_obs(&o), None);
        assert!(is_syn(&o));
    }

    #[test]
    fn flags_za_namba_zinaparsika() {
        // 0x29 = FIN+PSH+URG
        let o = Obs { src: "x".into(), dport: 1, flags: "41".into() };
        assert_eq!(classify_obs(&o), Some("xmas_scan"));
    }

    #[test]
    fn flowstats_inakusanya_bandari() {
        let flows = vec![
            Flow { proto: "tcp".into(), src: "9.9.9.9".into(), dst: "a".into(), sport: 1, dport: 22 },
            Flow { proto: "tcp".into(), src: "9.9.9.9".into(), dst: "a".into(), sport: 2, dport: 22 },
            Flow { proto: "tcp".into(), src: "9.9.9.9".into(), dst: "b".into(), sport: 3, dport: 80 },
        ];
        let s = FlowStats::from(&flows);
        let e = s.per_src.get("9.9.9.9").unwrap();
        assert_eq!(e.dport22, 2);
        assert_eq!(e.dports.len(), 2);
        assert_eq!(e.dsts.len(), 2);
    }

    #[test]
    fn observe_inagundua_null_scan() {
        let m = M36Ids;
        let obs = json!([
            {"src":"1.2.3.4","dport":80,"flags":""},
            {"src":"1.2.3.4","dport":443,"flags":"NULL"},
        ]);
        let r = m.run("", &json!({ "op": "observe", "observations": obs })).unwrap();
        assert!(r.findings.iter().any(|f| f["kind"] == "null_scan"));
    }
}
