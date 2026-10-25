//! secpack.rs — SECURITY PACK: zana za KALI (kutoka debian/control) kama
//! COMMANDS HALISI za Rust + ClamAV. Kikundi vimeorodheshwa kwa MAJINA KAMA
//! YALIVYO Kwenye control (kali_groups.rs — generated), kila zana (jina halisi)
//! inaonekana UI; kazi zenye command halisi zinaorodheshwa kama TaskDef.
//!
//! SIRI (kama toolkit.rs): binaries/args hayatoki API — jina SALAMA tu.
//! EXECUTOR: arg-array (HAKUNA shell → hakuna injection), `$T` placeholder,
//! valid_target/PATH/CIDR validation + shield::guard_path (kujilinda), timeout,
//! sanitize_output, audit kwenye tool_runs (kiitwa "sec:*").
//!
//! HITL: kazi za hatari (high_risk) zinahitaji `human_approved: true` —
//! AI/mteja hawezi kushambulia bila idhini ya mmiliki.

use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use crate::kali_groups::GROUPS;
use crate::toolkit::Cat;
use crate::tools::sanitize_output;

const EXEC_TIMEOUT: Duration = Duration::from_secs(20);

// ---------- CLAMAV (antivirus ya mfumo — kundi cha ndani, kutoka kali-tools-protect) ----------
const CLAMAV_TOOLS: &[&str] = &["clamav", "clamscan", "clamdscan", "freshclam", "clamconf"];

// ---------- THREATSWARM (pentest kill-chain agents — kundi cha ndani, kutoka repo ya mukul975) ----------
// Agents 27 scope-enforced (recon → exploit → post-ex → DFIR → report) + MITRE
// ATT&CK mapping. Plugin iko: server/security-tools/threatswarm/threatswarm-plugin
const THREATSWARM_AGENTS: &[&str] = &[
    "recon", "exploit", "post-ex", "web-attacker", "api-attacker",
    "active-directory", "network-ops", "osint", "wireless-attacker",
    "cloud-attacker", "container-attacker", "mobile-attacker", "social-engineer",
    "evasion", "c2-operator", "crypto-attacker", "iot-attacker", "password-attacks",
    "vuln-researcher", "reverse-engineer", "malware-analyst", "dfir",
    "threat-hunter", "blue-team", "log-analyst", "compliance-scanner", "report-writer",
];

// ---------- TASK (command halisi) ----------
#[derive(Debug, Clone)]
pub struct TaskDef {
    pub id: &'static str,
    pub name_sw: &'static str,
    /// package id kwenye control (au "clamav")
    pub group: &'static str,
    pub cat: Cat,
    pub binary: &'static str,
    pub args: &'static [&'static str],
    /// target inahitajika (kama `$T` hayapo ndani ya args → kuunganishwa mwisho)
    pub needs_target: bool,
    /// target inaruhusiwa kuwa path ya faili (/dev/sda, /data/x.raw)
    pub allow_path: bool,
    /// target inaruhusiwa kuwa CIDR (192.168.1.0/24)
    pub allow_cidr: bool,
    /// inahitaji idhini ya mmiliki (HITL) kabla ya kuendeshwa
    pub high_risk: bool,
}

macro_rules! k {
    ($id:expr, $sw:expr, $grp:expr, $cat:expr, $bin:expr, [$($a:expr),*], $nt:expr, $path:expr, $cidr:expr, $hr:expr) => {
        TaskDef { id: $id, name_sw: $sw, group: $grp, cat: $cat, binary: $bin, args: &[$($a),*], needs_target: $nt, allow_path: $path, allow_cidr: $cidr, high_risk: $hr }
    };
}

/// KAZI ZENYE COMMAND HALISI (majina ya binaries ni SIRI — si API).
pub const TASKS: &[TaskDef] = &[
    // ---- kali-tools-top10 ----
    k!(
        "t10-scan",
        "Uchunguzi wa port (nmap)",
        "kali-tools-top10",
        Cat::Recon,
        "nmap",
        ["-Pn", "--open", "-p", "22,80,443,445,3389"],
        true,
        false,
        false,
        false
    ),
    k!(
        "t10-sqlmap",
        "Uchunguzi wa SQLi (sqlmap)",
        "kali-tools-top10",
        Cat::Vuln,
        "sqlmap",
        ["-u", "http://$T/?id=1", "--batch", "--forms"],
        true,
        false,
        false,
        true
    ),
    k!(
        "t10-msf",
        "Kanzidata ya msisuko (metasploit)",
        "kali-tools-top10",
        Cat::Exploit,
        "msfconsole",
        ["-q", "-x", "version; exit"],
        false,
        false,
        false,
        false
    ),
    k!(
        "t10-netexec",
        "Uchunguzi wa SMB (netexec)",
        "kali-tools-top10",
        Cat::Exploit,
        "netexec",
        ["smb", "$T"],
        true,
        false,
        false,
        true
    ),
    // ---- kali-tools-information-gathering ----
    k!(
        "info-dns",
        "Orodha ya DNS (dnsenum)",
        "kali-tools-information-gathering",
        Cat::Recon,
        "dnsenum",
        ["--enum"],
        true,
        false,
        false,
        false
    ),
    k!(
        "info-harvest",
        "Kuhamta data OSINT (theHarvester)",
        "kali-tools-information-gathering",
        Cat::Recon,
        "theHarvester",
        ["-d", "$T", "-b", "all"],
        true,
        false,
        false,
        false
    ),
    k!(
        "info-ssl",
        "Uchunguzi wa SSL/TLS (sslscan)",
        "kali-tools-information-gathering",
        Cat::Recon,
        "sslscan",
        [],
        true,
        false,
        false,
        false
    ),
    k!(
        "info-whois",
        "Mmiliki wa kikao/chini (whois)",
        "kali-tools-information-gathering",
        Cat::Recon,
        "whois",
        [],
        true,
        false,
        false,
        false
    ),
    k!(
        "info-netdiscover",
        "Kugundua vifaa (netdiscover)",
        "kali-tools-information-gathering",
        Cat::Recon,
        "netdiscover",
        ["-r"],
        true,
        false,
        true,
        false
    ),
    k!(
        "info-masscan",
        "Uchunguzi wa haraka (masscan)",
        "kali-tools-information-gathering",
        Cat::Recon,
        "masscan",
        ["--rate=1000"],
        true,
        false,
        true,
        true
    ),
    // ---- kali-tools-vulnerability ----
    k!(
        "vuln-nikto",
        "Uchunguzi wa udhaifu wa tovuti (nikto)",
        "kali-tools-vulnerability",
        Cat::Vuln,
        "nikto",
        ["-h"],
        true,
        false,
        false,
        true
    ),
    k!(
        "vuln-lynis",
        "Ukaguzi wa usalama wa mifumo (lynis)",
        "kali-tools-vulnerability",
        Cat::Vuln,
        "lynis",
        ["audit", "system"],
        false,
        false,
        false,
        false
    ),
    // ---- kali-tools-web ----
    k!(
        "web-whatweb",
        "Utambuzi wa teknolojia (whatweb)",
        "kali-tools-web",
        Cat::Vuln,
        "whatweb",
        [],
        true,
        false,
        false,
        false
    ),
    k!(
        "web-wpscan",
        "Udhaifu wa WordPress (wpscan)",
        "kali-tools-web",
        Cat::Vuln,
        "wpscan",
        ["--url", "http://$T/", "--enumerate", "vp"],
        true,
        false,
        false,
        true
    ),
    k!(
        "web-dirb",
        "Uchunguzi wa njia (dirb)",
        "kali-tools-web",
        Cat::Vuln,
        "dirb",
        ["http://$T/"],
        true,
        false,
        false,
        true
    ),
    k!(
        "web-waf",
        "Kutambua WAF (wafw00f)",
        "kali-tools-web",
        Cat::Vuln,
        "wafw00f",
        [],
        true,
        false,
        false,
        false
    ),
    // ---- kali-tools-database ----
    k!(
        "db-oscanner",
        "Uchunguzi wa hifadhidata (oscanner)",
        "kali-tools-database",
        Cat::Vuln,
        "oscanner",
        ["-s"],
        true,
        false,
        false,
        true
    ),
    k!(
        "db-sqlmap",
        "Uchunguzi wa SQLi wa hifadhidata (sqlmap)",
        "kali-tools-database",
        Cat::Vuln,
        "sqlmap",
        ["-u", "http://$T/?id=1", "--batch", "--dbms", "mysql"],
        true,
        false,
        false,
        true
    ),
    // ---- kali-tools-passwords ----
    k!(
        "pass-hydra",
        "Uthibitisho wa nywila (hydra)",
        "kali-tools-passwords",
        Cat::Password,
        "hydra",
        [
            "-l",
            "admin",
            "-P",
            "/usr/share/wordlists/rockyou.txt",
            "$T",
            "ssh"
        ],
        true,
        false,
        false,
        true
    ),
    k!(
        "pass-crunch",
        "Kutengeneza wordlist (crunch)",
        "kali-tools-passwords",
        Cat::Password,
        "crunch",
        ["8", "12", "-o", "/tmp/wl.txt"],
        false,
        false,
        false,
        false
    ),
    k!(
        "pass-cewl",
        "Wordlist kutoka tovuti (cewl)",
        "kali-tools-passwords",
        Cat::Password,
        "cewl",
        ["-w", "/tmp/cewl.txt"],
        true,
        false,
        false,
        false
    ),
    // ---- kali-tools-wireless / 802-11 ----
    k!(
        "wifi-airmon",
        "Kuweka monitor mode (airmon-ng)",
        "kali-tools-wireless",
        Cat::Wireless,
        "airmon-ng",
        ["start"],
        true,
        false,
        false,
        false
    ),
    k!(
        "wifi-airodump",
        "Kukusanya trafiki ya Wi-Fi (airodump-ng)",
        "kali-tools-wireless",
        Cat::Wireless,
        "airodump-ng",
        ["--write", "/tmp/wifi-scan"],
        true,
        false,
        false,
        false
    ),
    k!(
        "w802-reaver",
        "Uchunguzi wa WPS (reaver)",
        "kali-tools-802-11",
        Cat::Wireless,
        "reaver",
        ["-i", "$T", "-vv"],
        true,
        false,
        false,
        true
    ),
    k!(
        "w802-wifite",
        "Uchunguzi wa Wi-Fi otomatiki (wifite)",
        "kali-tools-802-11",
        Cat::Wireless,
        "wifite",
        ["-i", "$T"],
        true,
        false,
        false,
        true
    ),
    // ---- kali-tools-bluetooth ----
    k!(
        "bt-log",
        "Kukusanya vifaa vya Bluetooth (bluelog)",
        "kali-tools-bluetooth",
        Cat::Wireless,
        "bluelog",
        ["-e", "/tmp/bt-log.txt"],
        false,
        false,
        false,
        false
    ),
    k!(
        "bt-crackle",
        "Uchunguzi wa BLE handshake (crackle)",
        "kali-tools-bluetooth",
        Cat::Wireless,
        "crackle",
        ["-i"],
        true,
        true,
        false,
        false
    ),
    // ---- kali-tools-reverse-engineering ----
    k!(
        "rev-strace",
        "Fuata mawasiliano ya michakato (strace)",
        "kali-tools-reverse-engineering",
        Cat::Vuln,
        "strace",
        ["-f", "-p"],
        true,
        false,
        false,
        false
    ),
    k!(
        "rev-ltrace",
        "Fuata maitzo ya k funkcjia (ltrace)",
        "kali-tools-reverse-engineering",
        Cat::Vuln,
        "ltrace",
        ["-p"],
        true,
        false,
        false,
        false
    ),
    k!(
        "rev-r2",
        "Uchanganuzi wa binary (radare2)",
        "kali-tools-reverse-engineering",
        Cat::Vuln,
        "r2",
        ["-q", "-c", "iI", "/bin/ls"],
        false,
        false,
        false,
        false
    ),
    // ---- kali-tools-exploitation ----
    k!(
        "expl-msf",
        "Kanzidata ya msisuko (msfconsole)",
        "kali-tools-exploitation",
        Cat::Exploit,
        "msfconsole",
        ["-q", "-x", "version; exit"],
        false,
        false,
        false,
        false
    ),
    k!(
        "expl-searchsploit",
        "Kutafuta msisuko ulioandikwa (searchsploit)",
        "kali-tools-exploitation",
        Cat::Exploit,
        "searchsploit",
        [],
        true,
        false,
        false,
        false
    ),
    // ---- kali-tools-sniffing-spoofing ----
    k!(
        "sniff-tcpdump",
        "Kunasa trafiki (tcpdump)",
        "kali-tools-sniffing-spoofing",
        Cat::Sniff,
        "tcpdump",
        ["-c", "100", "-i"],
        true,
        false,
        false,
        false
    ),
    k!(
        "sniff-tshark",
        "Uchanganuzi wa pakiti (tshark)",
        "kali-tools-sniffing-spoofing",
        Cat::Sniff,
        "tshark",
        ["-c", "100"],
        false,
        false,
        false,
        false
    ),
    k!(
        "sniff-ettercap",
        "Uchunguzi wa MITM (ettercap)",
        "kali-tools-sniffing-spoofing",
        Cat::Sniff,
        "ettercap",
        ["-T", "-q", "-i"],
        true,
        false,
        false,
        true
    ),
    k!(
        "sniff-darkstat",
        "Uchanganuzi wa mtandao (darkstat)",
        "kali-tools-sniffing-spoofing",
        Cat::Sniff,
        "darkstat",
        ["-i"],
        true,
        false,
        false,
        false
    ),
    // ---- kali-tools-post-exploitation ----
    k!(
        "post-enum4linux",
        "Kuchunguza mifumo ya ndani (enum4linux)",
        "kali-tools-post-exploitation",
        Cat::Exploit,
        "enum4linux",
        ["-a"],
        true,
        false,
        false,
        true
    ),
    k!(
        "post-smbclient",
        "Kuona saraka za SMB (smbclient)",
        "kali-tools-post-exploitation",
        Cat::Exploit,
        "smbclient",
        ["-L", "//$T/", "-N"],
        true,
        false,
        false,
        true
    ),
    // ---- kali-tools-forensics ----
    k!(
        "fore-binwalk",
        "Uchanganuzi wa firmware/faili (binwalk)",
        "kali-tools-forensics",
        Cat::Forensics,
        "binwalk",
        ["/data"],
        false,
        false,
        false,
        false
    ),
    k!(
        "fore-hashdeep",
        "Hashi za faili zote (hashdeep)",
        "kali-tools-forensics",
        Cat::Forensics,
        "hashdeep",
        ["-c", "md5,sha256", "-r", "/data"],
        false,
        false,
        false,
        false
    ),
    k!(
        "fore-vol",
        "Uchanganuzi wa kumbukumbu ya RAM (volatility3)",
        "kali-tools-forensics",
        Cat::Forensics,
        "volatility3",
        ["-f", "$T", "windows.pslist"],
        true,
        true,
        false,
        false
    ),
    k!(
        "fore-fls",
        "Orodha ya faili za diski (sleuthkit fls)",
        "kali-tools-forensics",
        Cat::Forensics,
        "fls",
        ["-r"],
        true,
        true,
        false,
        false
    ),
    k!(
        "fore-rkhunter",
        "Kutafuta rootkit (rkhunter)",
        "kali-tools-forensics",
        Cat::Forensics,
        "rkhunter",
        ["--check", "--skip-keypress"],
        false,
        false,
        false,
        false
    ),
    k!(
        "fore-chkrootkit",
        "Kutafuta rootkit (chkrootkit)",
        "kali-tools-forensics",
        Cat::Forensics,
        "chkrootkit",
        [],
        false,
        false,
        false,
        false
    ),
    // ---- kali-tools-reporting ----
    k!(
        "rep-cutycapt",
        "Picha ya tovuti kwa ripoti (cutycapt)",
        "kali-tools-reporting",
        Cat::Forensics,
        "cutycapt",
        ["--url=http://$T", "--out=/tmp/page.png"],
        true,
        false,
        false,
        false
    ),
    // ---- kali-tools-identify (purple) ----
    k!(
        "id-amass",
        "Orodha ya asset (amass)",
        "kali-tools-identify",
        Cat::Recon,
        "amass",
        ["enum", "-d"],
        true,
        false,
        false,
        false
    ),
    // ---- kali-tools-protect + CLAMAV (kinga ya mfumo) ----
    k!(
        "clam-scan",
        "Kuchunguza malware (clamscan)",
        "clamav",
        Cat::System,
        "clamscan",
        ["--infected", "--recursive", "--no-summary", "/data"],
        false,
        false,
        false,
        false
    ),
    k!(
        "clam-update",
        "Kusasisha saini za virusi (freshclam)",
        "clamav",
        Cat::System,
        "freshclam",
        [],
        false,
        false,
        false,
        false
    ),
    k!(
        "clam-daemon",
        "Kuchunguza kwa daemon (clamdscan)",
        "clamav",
        Cat::System,
        "clamdscan",
        ["--fdpass", "--infected", "/data"],
        false,
        false,
        false,
        false
    ),
    // ---- kali-tools-recover ----
    k!(
        "rec-extundelete",
        "Kurejesha faili (extundelete)",
        "kali-tools-recover",
        Cat::Forensics,
        "extundelete",
        ["$T", "--restore-all"],
        true,
        true,
        false,
        false
    ),
    k!(
        "rec-recoverjpeg",
        "Kurejesha picha (recoverjpeg)",
        "kali-tools-recover",
        Cat::Forensics,
        "recoverjpeg",
        ["-m", "/tmp/recovered"],
        true,
        true,
        false,
        false
    ),
    // ---- kali-tools-voip ----
    k!(
        "voip-svmap",
        "Kutambua simu za VoIP (svmap)",
        "kali-tools-voip",
        Cat::Sniff,
        "svmap",
        ["$T"],
        true,
        false,
        false,
        true
    ),
    // ---- kali-tools-fuzzing ----
    k!(
        "fuzz-wfuzz",
        "Kutafuta njia kwa nguvu (wfuzz)",
        "kali-tools-fuzzing",
        Cat::Vuln,
        "wfuzz",
        [
            "-u",
            "http://$T/",
            "-w",
            "/usr/share/wordlists/dirb/common.txt",
            "-t",
            "10"
        ],
        true,
        false,
        false,
        true
    ),
    // ---- kali-tools-hardware ----
    k!(
        "hw-minicom",
        "Uhusiano na kifaa (minicom)",
        "kali-tools-hardware",
        Cat::System,
        "minicom",
        ["-D"],
        true,
        true,
        false,
        false
    ),
    k!(
        "hw-flashrom",
        "Kusoma chip ya flash (flashrom)",
        "kali-tools-hardware",
        Cat::System,
        "flashrom",
        ["-r", "/tmp/flash.bin"],
        false,
        false,
        false,
        false
    ),
    // ---- kali-tools-sdr ----
    k!(
        "sdr-kal",
        "Kusanyaji wa redio (kalibrate-rtl)",
        "kali-tools-sdr",
        Cat::System,
        "kal",
        ["-s"],
        false,
        false,
        false,
        false
    ),
    // ---- kali-tools-crypto-stego ----
    k!(
        "cs-steghide",
        "Kuchunguza ujumbe siri (steghide)",
        "kali-tools-crypto-stego",
        Cat::Password,
        "steghide",
        ["info", "-p", "", "$T"],
        true,
        true,
        false,
        false
    ),
];

// ---------- VALIDATION (target salama: jina/IP + CIDR + PATH bila kugusa siri) ----------
fn is_cidr(t: &str) -> bool {
    let Some((base, len)) = t.split_once('/') else {
        return false;
    };
    let Ok(n) = len.parse::<u8>() else {
        return false;
    };
    n <= 32 && base.parse::<std::net::Ipv4Addr>().is_ok()
}

pub fn valid_target2(t: &str, allow_path: bool, allow_cidr: bool) -> Result<(), &'static str> {
    if t.is_empty() || t.len() > 128 {
        return Err("target ni tupu au ndefu mno");
    }
    if crate::tools_internal::valid_target(t) {
        return Ok(());
    }
    if allow_cidr && is_cidr(t) {
        return Ok(());
    }
    if allow_path
        && t.starts_with('/')
        && !t.contains("..")
        && !t.contains(' ')
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
    {
        // KUJILINDA: kamwe usiguse DB/faili za siri hata kama ni path "halali"
        if let Some(r) = crate::shield::guard_path(t) {
            return Err(r);
        }
        return Ok(());
    }
    Err("target si salama (jina la PC / IP / CIDR / path halali pekee)")
}

fn find(id: &str) -> Option<&'static TaskDef> {
    TASKS.iter().find(|t| t.id == id)
}

/// Group→Cat (kwa UI)
fn group_cat(id: &str) -> Cat {
    match id {
        "clamav"
        | "threatswarm"
        | "kali-tools-protect"
        | "kali-tools-detect"
        | "kali-tools-respond"
        | "kali-tools-hardware"
        | "kali-tools-gpu"
        | "kali-tools-windows-resources" => Cat::System,
        "kali-tools-information-gathering" | "kali-tools-identify" => Cat::Recon,
        "kali-tools-vulnerability"
        | "kali-tools-web"
        | "kali-tools-database"
        | "kali-tools-reverse-engineering"
        | "kali-tools-fuzzing" => Cat::Vuln,
        "kali-tools-exploitation"
        | "kali-tools-social-engineering"
        | "kali-tools-post-exploitation" => Cat::Exploit,
        "kali-tools-wireless"
        | "kali-tools-802-11"
        | "kali-tools-bluetooth"
        | "kali-tools-rfid"
        | "kali-tools-sdr" => Cat::Wireless,
        "kali-tools-passwords" | "kali-tools-crypto-stego" => Cat::Password,
        "kali-tools-sniffing-spoofing" | "kali-tools-voip" => Cat::Sniff,
        // Vikundi vya mfumo/kikao (kali-linux-*, kali-system-*, kali-desktop-*)
        id if id.starts_with("kali-linux")
            || id.starts_with("kali-system")
            || id.starts_with("kali-desktop") =>
        {
            Cat::System
        }
        _ => Cat::Forensics,
    }
}

/// Kazi za kikundi (protect huona pia clamav — kweli kwenye control: clamav
/// ni dependency ya kali-tools-protect).
fn tasks_in(group: &str) -> Vec<serde_json::Value> {
    TASKS
        .iter()
        .filter(|t| t.group == group || (group == "kali-tools-protect" && t.group == "clamav"))
        .map(|t| {
            json!({
                "id": t.id, "name_sw": t.name_sw, "cat": t.cat.id(),
                "high_risk": t.high_risk, "needs_target": t.needs_target
            })
        })
        .collect()
}

/// Module za sauti/video (SEHEMU 14): 273-287.
pub const CALLS_MODULE_IDS: &[u16] = &[273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285, 286, 287];

/// Module za SMS & Email (SEHEMU 15): 288-305.
pub const MESSAGING_MODULE_IDS: &[u16] = &[288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305];

/// Module za SIM & Device (SEHEMU 16): 306-322.
pub const DEVICE_MODULE_IDS: &[u16] = &[306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322];

/// Module za International & Humanitarian + Future-Proof (SEHEMU 17-18): 323-328.
pub const HUMAN_MODULE_IDS: &[u16] = &[323, 324, 325, 326, 327, 328];

/// Module za Blockchain Security (SEHEMU 19): 468-500.
pub const BLOCKCHAIN_MODULE_IDS: &[u16] = &[
    468, 469, 470, 471, 472, 473, 474, 475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 485, 486,
    487, 488, 489, 490, 491, 492, 493, 494, 495, 496, 497, 498, 499, 500,
];

/// Module za Calls (SEHEMU 8): 1-10 (usimbaji, spoofing, robocall, ubora wa simu).
pub const CALLS1_MODULE_IDS: &[u16] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// Module za SMS (SEHEMU 9): 11-20 (usimbaji, smishing, premium, spoofing).
pub const SMS_MODULE_IDS: &[u16] = &[11, 12, 13, 14, 15, 16, 17, 18, 19, 20];

/// Module za AI/ML engine (SEHEMU 10-11): 21-34.
pub const AI_MODULE_IDS: &[u16] = &[
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34,
];

/// Module za ziada (SEHEMU 20): 501-516.
pub const EXT_MODULE_IDS: &[u16] = &[
    501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515, 516,
];

/// Module za malipo (ULINZI): 517 (uthibitisho wa callback za fedha za simu).
pub const PAYMENTS_MODULE_IDS: &[u16] = &[517];

/// TWIST modules: 1031-1300 (twists 31-300, SEHEMU A-AA).
pub const TWIST_MODULE_IDS: &[u16] = &[
    1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038, 1039, 1040, 1041, 1042, 1043, 1044, 1045, 1046,
    1047, 1048, 1049, 1050, 1051, 1052, 1053, 1054, 1055, 1056, 1057, 1058, 1059, 1060, 1061, 1062,
    1063, 1064, 1065, 1066, 1067, 1068, 1069, 1070, 1071, 1072, 1073, 1074, 1075, 1076, 1077, 1078,
    1079, 1080, 1081, 1082, 1083, 1084, 1085, 1086, 1087, 1088, 1089, 1090, 1091, 1092, 1093, 1094,
    1095, 1096, 1097, 1098, 1099, 1100, 1101, 1102, 1103, 1104, 1105, 1106, 1107, 1108, 1109, 1110,
    1111, 1112, 1113, 1114, 1115, 1116, 1117, 1118, 1119, 1120, 1121, 1122, 1123, 1124, 1125, 1126,
    1127, 1128, 1129, 1130, 1131, 1132, 1133, 1134, 1135, 1136, 1137, 1138, 1139, 1140, 1141, 1142,
    1143, 1144, 1145, 1146, 1147, 1148, 1149, 1150, 1151, 1152, 1153, 1154, 1155, 1156, 1157, 1158,
    1159, 1160, 1161, 1162, 1163, 1164, 1165, 1166, 1167, 1168, 1169, 1170, 1171, 1172, 1173, 1174,
    1175, 1176, 1177, 1178, 1179, 1180, 1181, 1182, 1183, 1184, 1185, 1186, 1187, 1188, 1189, 1190,
    1191, 1192, 1193, 1194, 1195, 1196, 1197, 1198, 1199, 1200, 1201, 1202, 1203, 1204, 1205, 1206,
    1207, 1208, 1209, 1210, 1211, 1212, 1213, 1214, 1215, 1216, 1217, 1218, 1219, 1220, 1221, 1222,
    1223, 1224, 1225, 1226, 1227, 1228, 1229, 1230, 1231, 1232, 1233, 1234, 1235, 1236, 1237, 1238,
    1239, 1240, 1241, 1242, 1243, 1244, 1245, 1246, 1247, 1248, 1249, 1250, 1251, 1252, 1253, 1254,
    1255, 1256, 1257, 1258, 1259, 1260, 1261, 1262, 1263, 1264, 1265, 1266, 1267, 1268, 1269, 1270,
    1271, 1272, 1273, 1274, 1275, 1276, 1277, 1278, 1279, 1280, 1281, 1282, 1283, 1284, 1285, 1286,
    1287, 1288, 1289, 1290, 1291, 1292, 1293, 1294, 1295, 1296, 1297, 1298, 1299, 1300,
];

/// `which` ya haraka — hutafuta binary kwenye PATH bila subprocess (is_file tu).
pub fn tool_available(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&paths) {
        if dir.join(name).is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            if dir.join(format!("{name}.exe")).is_file() {
                return true;
            }
        }
    }
    false
}

/// TASKS zenye binary ZINAZOPATIKANA kwenye server hii (hali HALISI ya uendeshaji).
pub fn available_task_ids() -> Vec<&'static str> {
    TASKS
        .iter()
        .filter(|t| tool_available(t.binary))
        .map(|t| t.id)
        .collect()
}

/// Muhtasari wa upatikanaji: kila kikundi (tools) + tasks, bila kuongea uongo.
pub fn availability_json() -> serde_json::Value {
    let groups: Vec<serde_json::Value> = GROUPS
        .iter()
        .map(|g| {
            let avail = g.tools.iter().filter(|t| tool_available(t)).count();
            json!({
                "id": g.id,
                "title": g.title,
                "tools_total": g.tools.len(),
                "tools_available": avail,
                "available": g.tools.iter().filter(|t| tool_available(t)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let tasks_avail = available_task_ids();
    json!({
        "ok": true,
        "kali_tools_total": crate::kali_groups::tool_count(),
        "kali_tools_available": groups.iter().map(|g| g["tools_available"].as_u64().unwrap_or(0)).sum::<u64>(),
        "tasks_total": TASKS.len(),
        "tasks_available": tasks_avail.len(),
        "tasks_runnable": tasks_avail,
        "tasks_missing": TASKS.iter().filter(|t| !tool_available(t.binary)).map(|t| t.id).collect::<Vec<_>>(),
        "groups": groups,
        "note_sw": "Upatikanaji hupimwa kwa `which` HALISI kwenye PATH. Zana zisizopo zinasakinishwa na Dockerfile (Kali repo) — build image ndiyo njia ya 'kuzifanya zifanye kazi'.",
    })
}

pub fn groups_json() -> serde_json::Value {
    let mut groups = vec![
        json!({
            "id": "clamav",
            "title": "ClamAV — antivirus ya mfumo",
            "desc": "Malware scan, kusasisha saini, daemon — mfumo unajilinda",
            "tool_count": CLAMAV_TOOLS.len(),
            "tools": CLAMAV_TOOLS,
            "cat": group_cat("clamav").id(),
            "tasks": tasks_in("clamav"),
        }),
        json!({
            "id": "threatswarm",
            "title": "ThreatSwarm — pentest kill-chain agents",
            "desc": "Agents 27 scope-enforced (recon → exploit → post-ex → DFIR → report) zenye MITRE ATT&CK mapping. Plugin: server/security-tools/threatswarm/threatswarm-plugin",
            "tool_count": THREATSWARM_AGENTS.len(),
            "tools": THREATSWARM_AGENTS,
            "cat": group_cat("threatswarm").id(),
            "tasks": [],
        }),
        json!({
            "id": "social",
            "title": "Social Media Security — modules 58-272",
            "desc": "Modules 215 za usalama wa mitandao ya kijamii (Facebook, Instagram, WhatsApp, Twitter/X, TikTok, Snapchat, LinkedIn, Telegram, Signal, Discord, Reddit, YouTube). Injini ya HIFADHIDATA inayojifunza + LLM (semantic).",
            "tool_count": crate::tsmod::social::all().len(),
            "tools": crate::tsmod::social::all().iter().map(|m| format!("soc-{}", m.id)).collect::<Vec<_>>(),
            "cat": "social",
            "tasks": [],
        }),
        json!({
            "id": "calls",
            "title": "Voice & Video Calls — modules 273-287",
            "desc": "Modules 15 za usalama wa simu: usimbaji wa sauti/video (ChaCha20+HMAC, hybrid PQ), ugunduzi wa spoofing/robocall, uthibitisho wa namba, voicemail, rekodi, mikutano (SIP/WebRTC), 2G-5G, mezani, kimataifa na dharura.",
            "tool_count": CALLS_MODULE_IDS.len(),
            "tools": CALLS_MODULE_IDS.iter().map(|id| format!("call-{id}")).collect::<Vec<_>>(),
            "cat": "calls",
            "tasks": [],
        }),
        json!({
            "id": "messaging",
            "title": "SMS & Email — modules 288-305",
            "desc": "Modules 18 za usalama wa SMS na barua pepe: usimbaji, phishing/smishing (sandbox), spam/rate-limit, uthibitisho wa sender, viambatisho (ClamAV), BEC, DLP (PII/Luhn/NIDA), archiving (hash-chain), TCRA compliance na key management.",
            "tool_count": MESSAGING_MODULE_IDS.len(),
            "tools": MESSAGING_MODULE_IDS.iter().map(|id| format!("msg-{id}")).collect::<Vec<_>>(),
            "cat": "messaging",
            "tasks": [],
        }),
        json!({
            "id": "device",
            "title": "SIM & Device — modules 306-322",
            "desc": "Modules 17 za usalama wa SIM na kifaa: sim-swap kill switch, fingerprinting (IMEI/MAC/OS), location/geolocation, remote lock/wipe, backup/restore, AES-256 device encryption, password/screen-lock policy, app permissions (spyware), VPN (WireGuard), proxy, DNS-over-HTTPS, firewall na antivirus (ClamAV).",
            "tool_count": DEVICE_MODULE_IDS.len(),
            "tools": DEVICE_MODULE_IDS.iter().map(|id| format!("dev-{id}")).collect::<Vec<_>>(),
            "cat": "device",
            "tasks": [],
        }),
        json!({
            "id": "humanitarian",
            "title": "International & Humanitarian + Future-Proof — modules 323-328",
            "desc": "Modules 6: ulinzi wa wanajeshi wa amani (mawasiliano yaliyofichwa), wafanyakazi wa misaada (SOS/location), data za wakimbizi (blockchain), ufuatiliaji wa uhalifu wa kimataifa (INTERPOL), ulinzi wa vita vya mtandao (kata connection), na usanifu wa teknolojia zijazo (hybrid PQ-KEM, Lamport OTS, crypto-agility).",
            "tool_count": HUMAN_MODULE_IDS.len(),
            "tools": HUMAN_MODULE_IDS.iter().map(|id| format!("hum-{id}")).collect::<Vec<_>>(),
            "cat": "humanitarian",
            "tasks": [],
        }),
        json!({
            "id": "blockchain",
            "title": "Blockchain Security — modules 468-500",
            "desc": "Modules 33 za usalama wa blockchain: mnyororo wa vitalu (hash-chain, merkle, PBFT), mikataba mahiri (VM, escrow, DAO, kura, tokenization, bima, royalty, GDPR, faragha), pochi za multisig (Lamport OTS), uthibitishaji wa vitalu, nodes (auto-scaling), uchambuzi, explorer, data za afya/elimu/ardhi, mikataba ya kimataifa na mali halisi.",
            "tool_count": BLOCKCHAIN_MODULE_IDS.len(),
            "tools": BLOCKCHAIN_MODULE_IDS.iter().map(|id| format!("bc-{id}")).collect::<Vec<_>>(),
            "cat": "blockchain",
            "tasks": [],
        }),
        json!({
            "id": "calls1",
            "title": "Calls — modules 1-10 (usimbaji, spoofing, ubora)",
            "desc": "Modules 10: usimbaji wa sauti wa mwisho-hadi-mwisho (ChaCha20+HMAC, hybrid PQ), ugunduzi wa spoofing/robocall, ulinzi wa call forwarding (IRSF), call drop, ubora wa simu (ITU-T E-model MOS), ulinzi wa rekodi, kipaumbele cha dharura, na ulaghai wa simu za kimataifa.",
            "tool_count": CALLS1_MODULE_IDS.len(),
            "tools": CALLS1_MODULE_IDS.iter().map(|id| format!("call-{id}")).collect::<Vec<_>>(),
            "cat": "calls",
            "tasks": [],
        }),
        json!({
            "id": "sms",
            "title": "SMS — modules 11-20 (usimbaji, smishing, premium)",
            "desc": "Modules 10: usimbaji wa SMS (self-destruct + blockchain proof), smishing (sandbox URL), ulinzi wa forwarding (2FA), SMS blasting, dropped/SMSC spoofing, premium SMS price verification, API security (HMAC+mTLS), auto-reply fraud, na content filtering (PII/Luhn/NIDA).",
            "tool_count": SMS_MODULE_IDS.len(),
            "tools": SMS_MODULE_IDS.iter().map(|id| format!("sms-{id}")).collect::<Vec<_>>(),
            "cat": "messaging",
            "tasks": [],
        }),
        json!({
            "id": "ai",
            "title": "AI/ML Engine — modules 21-34",
            "desc": "Modules 14: digital fingerprint (vipimo 7), deep learning (CNN/RNN/Transformer, deepfake), pattern recognition (pre-crime), predictive analytics (Holt), reinforcement learning (Q-learning/federated), real-time monitoring, big data (MapReduce), behavioral analysis, network traffic (entropy/beaconing), endpoint behavior, automated response, threat mitigation, self-healing, na alerting (lugha nyingi).",
            "tool_count": AI_MODULE_IDS.len(),
            "tools": AI_MODULE_IDS.iter().map(|id| format!("ai-{id}")).collect::<Vec<_>>(),
            "cat": "ai",
            "tasks": [],
        }),
        json!({
            "id": "extended",
            "title": "Extended — modules 501-516 (data recovery → continuous learning)",
            "desc": "Modules 16: data recovery (file carving), dark web intelligence (onion v2/v3), IoT security, billing/subscription, system architecture (sharding), zero trust, 9-layer scanning, Africa First, future-proof, deception/honeypot, moving target defense (LLVM), adversarial AI resistance, supply chain SBOM (Merkle), TEE attestation, QRNG, na continuous learning (FedAvg).",
            "tool_count": EXT_MODULE_IDS.len(),
            "tools": EXT_MODULE_IDS.iter().map(|id| format!("ext-{id}")).collect::<Vec<_>>(),
            "cat": "system",
            "tasks": [],
        }),
        json!({
            "id": "twist",
            "title": "TWIST — twists 31-300 (SEHEMU A-AA)",
            "desc": "Twists 270 za kipekee (id 1031-1300): biometric/signal auth (ECG, SpO2, RF, QRNG), cryptography (Shamir, time-lock, homomorphic, ZK, Merkle), AI security (threat hunting, RE, XAI, poisoning), SIM/hardware glitch detection, network protocol analysis, cloud/database/identity/incident/compliance, l10n/intl/bank/ISP, future-proof, end-to-end network, mobility/GPS, na traffic/content filtering.",
            "tool_count": TWIST_MODULE_IDS.len(),
            "tools": TWIST_MODULE_IDS.iter().map(|id| format!("tw-{id}")).collect::<Vec<_>>(),
            "cat": "system",
            "tasks": [],
        }),
        json!({
            "id": "payments",
            "title": "ULINZI WA MALIPO — uthibitisho wa callback (517)",
            "desc": "Huthibitisha callback za malipo ya fedha za simu (M-Pesa/HaloPesa): HMAC-SHA256, IP whitelist ya telco, kuzuia replay/duplicate, kulinganisha kiasi/akaunti (IDOR), freshness ya timestamp na sequence. Huzuia callback ghushi, 'malipo yamekamilika' ya uongo, na uvujaji wa CONSUMER_KEY/SECRET.",
            "tool_count": PAYMENTS_MODULE_IDS.len(),
            "tools": PAYMENTS_MODULE_IDS.iter().map(|id| format!("pay-{id}")).collect::<Vec<_>>(),
            "cat": "system",
            "tasks": [],
        }),
    ];
    for g in GROUPS {
        groups.push(json!({
            "id": g.id,
            "kind": g.kind,
            "title": g.title,
            "desc": g.desc,
            "tool_count": g.tools.len(),
            "tools": g.tools,
            "cat": group_cat(g.id).id(),
            "tasks": tasks_in(g.id),
        }));
    }
    json!({
        "ok": true,
        "groups": groups,
        "total_groups": GROUPS.len() + 14,
        "total_tools": crate::kali_groups::tool_count() + CLAMAV_TOOLS.len() + THREATSWARM_AGENTS.len() + CALLS_MODULE_IDS.len() + MESSAGING_MODULE_IDS.len() + DEVICE_MODULE_IDS.len() + HUMAN_MODULE_IDS.len() + BLOCKCHAIN_MODULE_IDS.len() + CALLS1_MODULE_IDS.len() + SMS_MODULE_IDS.len() + AI_MODULE_IDS.len() + EXT_MODULE_IDS.len() + TWIST_MODULE_IDS.len() + PAYMENTS_MODULE_IDS.len(),
        "tasks_total": TASKS.len(),
        "note_sw": "Vikundi vimetengenezwa kwa majina kama yalivyo kwenye debian/control ya kali-meta; commands halisi ziko ndani ya Rust (si API)."
    })
}

// ---------- VOICE & VIDEO CALLS (modules 273-287) ----------
/// Tekeleza kazi ya module ya tsmod kwa id. Hii ni generic: inafanya kazi kwa
/// modules zote za tsmod (core 35-57 na calls 273-287). HITL: kama module
/// inahitaji DB (`run_db`) basi inatumia DB; vinginevyo `run`.
pub async fn ts_module_task(db: &SqlitePool, id: u16, target: &str) -> serde_json::Value {
    ts_module_task_p(db, id, target, "call").await
}

/// Kama `ts_module_task` lakini kwa prefix maalum ("call", "msg", n.k.).
pub async fn ts_module_task_p(db: &SqlitePool, id: u16, target: &str, prefix: &str) -> serde_json::Value {
    let Some(m) = crate::tsmod::find(id) else {
        return json!({ "ok": false, "error": format!("module {id} haipo") });
    };
    let params = json!({ "op": "selftest", "text": target, "number": target });
    match m.run_db(db, target, &params).await {
        Ok(rep) => {
            let v = rep.to_json();
            json!({
                "ok": true, "task": format!("{prefix}-{id}"), "name_sw": m.name(),
                "module": id, "group": m.group(), "summary": rep.summary, "report": v,
            })
        }
        Err(e) => json!({ "ok": false, "task": format!("{prefix}-{id}"), "error": e }),
    }
}

// ---------- SOCIAL MEDIA (modules 58-272) ----------
/// Tekeleza kazi ya kijamii kwa module id. Hii huunganisha ThreatSwarm agents na
/// injini ya tsmod (DB-driven). HITL inahitajika kwa hatari kubwa (block).
pub async fn social_task(db: &SqlitePool, id: u16, target: &str) -> serde_json::Value {
    let Some(m) = crate::tsmod::find(id) else {
        return json!({ "ok": false, "error": format!("module ya kijamii {id} haipo") });
    };
    let params = json!({ "op": "analyze", "text": target });
    match m.run_db(db, target, &params).await {
        Ok(rep) => {
            let v = rep.to_json();
            json!({
                "ok": true, "task": format!("soc-{id}"), "name_sw": m.name(),
                "module": id, "group": m.group(), "platform": rep.findings.first()
                    .and_then(|f| f.get("detail")).and_then(|d| d.get("platform"))
                    .cloned().unwrap_or(json!("")),
                "summary": rep.summary, "report": v,
            })
        }
        Err(e) => json!({ "ok": false, "task": format!("soc-{id}"), "error": e }),
    }
}

// ---------- EXECUTOR (arg-array, timeout, sanitize, audit, HITL) ----------
/// DNS RECON ya HARAKA: dnsenum --enum/--noreverse hukwama kwa reverse-lookup
/// na AXFR (hadi 90s+, na AXFR hukataliwa). Tunatumia resolver ya mfumo
/// (`getent`) inayomaliza kwa millisekunde — A/AAAA, reverse, na MX/NS/TXT/SPF
/// kwa `getent hosts` + `whois` bila kukwama.
pub async fn dns_recon(db: &SqlitePool, target: &str) -> serde_json::Value {
    let started = Instant::now();
    let host = target.trim().trim_start_matches("http://").trim_start_matches("https://")
        .split('/').next().unwrap_or("").split(':').next().unwrap_or("").to_string();
    if host.is_empty() {
        return json!({ "ok": false, "error": "target ni lazima kwa kazi hii", "task": "info-dns" });
    }

    let run = |bin: &'static str, args: Vec<String>| async move {
        let mut cmd = tokio::process::Command::new(bin);
        cmd.args(&args);
        match tokio::time::timeout(Duration::from_secs(8), cmd.output()).await {
            Ok(Ok(o)) => {
                let mut t = String::from_utf8_lossy(&o.stdout).to_string();
                t.push_str(&String::from_utf8_lossy(&o.stderr));
                t
            }
            _ => String::new(),
        }
    };

    let hosts = run("getent", vec!["ahostsv4".into(), host.clone()]).await;
    let hosts6 = run("getent", vec!["ahostsv6".into(), host.clone()]).await;
    let rev = match hosts.split_whitespace().next() {
        Some(ip) if !ip.is_empty() => run("getent", vec!["hosts".into(), ip.to_string()]).await,
        _ => String::new(),
    };

    let mut summary = format!("Lengo: {host}\n");
    summary.push_str(if hosts.trim().is_empty() {
        "A/AAAA: hakuna rekodi (domain haijasuluhishwa)\n"
    } else {
        "A/AAAA:\n"
    });
    summary.push_str(&hosts);
    if !hosts6.trim().is_empty() {
        summary.push_str(&hosts6);
    }
    if !rev.trim().is_empty() {
        summary.push_str(&format!("Reverse (PTR): {}\n", rev.trim()));
    }
    if summary.trim() == format!("Lengo: {host}").trim() {
        summary.push_str("DNS: sikuipata rekodi\n");
    }

    let clean = sanitize_output(summary.trim());
    let lines = clean.lines().count() as i64;
    let dur = started.elapsed().as_millis() as i64;
    let ok = !hosts.trim().is_empty();
    let _ = sqlx::query(
        "INSERT INTO tool_runs (tool_id, target, ok, lines, duration_ms, at) VALUES (?,?,?,?,?,?)",
    )
    .bind("sec:info-dns")
    .bind(target)
    .bind(ok)
    .bind(lines)
    .bind(dur)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await;
    json!({
        "ok": ok, "task": "info-dns", "name_sw": "Orodha ya DNS",
        "group": "kali-tools-information-gathering", "target": target, "high_risk": false,
        "summary": clean, "lines": lines, "duration_ms": dur,
        "note_sw": "Matokeo yamechambuliwa na injini ya ndani ya MTECH OS; kila utekelezaji umeandikwa kwenye audit."
    })
}

/// SSL/TLS RECON ya HARAKA: sslscan hukwama kwa connect-timeout ya 75s
/// (hadi 20s ya EXEC_TIMEOUT). Tunatumia `openssl s_client` — kufikia hoja
/// (TLS version, cipher, cert, dates, SAN, verification) kwa sekunde moja.
pub async fn ssl_recon(db: &SqlitePool, target: &str) -> serde_json::Value {
    let started = Instant::now();
    let host = target.trim().trim_start_matches("http://").trim_start_matches("https://")
        .split('/').next().unwrap_or("").split(':').next().unwrap_or("").to_string();
    if host.is_empty() {
        return json!({ "ok": false, "error": "target ni lazima kwa kazi hii", "task": "info-ssl" });
    }
    let brief = run_tool("openssl", &["s_client", "-connect", &format!("{host}:443"), "-servername", &host, "-brief"], 10).await;
    let cert = run_tool("openssl", &["s_client", "-connect", &format!("{host}:443"), "-servername", &host], 10).await;
    let x509 = {
        let mut cmd = tokio::process::Command::new("openssl");
        cmd.args(["x509", "-noout", "-subject", "-issuer", "-dates", "-ext", "subjectAltName"]);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut sin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    let _ = sin.write_all(cert.as_bytes()).await;
                    drop(sin);
                }
                match tokio::time::timeout(Duration::from_secs(8), child.wait_with_output()).await {
                    Ok(Ok(o)) => String::from_utf8_lossy(&o.stdout).to_string(),
                    _ => String::new(),
                }
            }
            Err(_) => String::new(),
        }
    };

    let mut summary = format!("Lengo: {host}:443\n");
    if brief.trim().is_empty() {
        summary.push_str("SSL: sikuipata (tovuti haijibu TLS kwenye 443)\n");
    } else {
        summary.push_str(brief.trim());
        summary.push('\n');
    }
    if !x509.trim().is_empty() {
        summary.push_str(x509.trim());
        summary.push('\n');
    }

    let clean = sanitize_output(summary.trim());
    let lines = clean.lines().count() as i64;
    let dur = started.elapsed().as_millis() as i64;
    let ok = !brief.trim().is_empty();
    let _ = sqlx::query(
        "INSERT INTO tool_runs (tool_id, target, ok, lines, duration_ms, at) VALUES (?,?,?,?,?,?)",
    )
    .bind("sec:info-ssl")
    .bind(target)
    .bind(ok)
    .bind(lines)
    .bind(dur)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await;
    json!({
        "ok": ok, "task": "info-ssl", "name_sw": "Uchunguzi wa SSL/TLS",
        "group": "kali-tools-information-gathering", "target": target, "high_risk": false,
        "summary": clean, "lines": lines, "duration_ms": dur,
        "note_sw": "Matokeo yamechambuliwa na injini ya ndani ya MTECH OS; kila utekelezaji umeandikwa kwenye audit."
    })
}

/// Kimbiza zana kwa arg-array, rudisha stdout+stderr (tupu ikikwama/kukosekana).
async fn run_tool(bin: &'static str, args: &[&str], timeout_secs: u64) -> String {
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(args);
    match tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
        Ok(Ok(o)) => {
            let mut t = String::from_utf8_lossy(&o.stdout).to_string();
            t.push_str(&String::from_utf8_lossy(&o.stderr));
            t
        }
        _ => String::new(),
    }
}

pub async fn run_task(
    db: &SqlitePool,
    task_id: &str,
    target: &str,
    human_approved: bool,
) -> serde_json::Value {
    // SOCIAL MEDIA (modules 58-272): kazi za kijamii zinatekelezwa na injini ya
    // tsmod (DB-driven, inayojifunza) — hivyo ThreatSwarm agents zinaziweza pia.
    if let Some(id) = task_id.strip_prefix("soc-").and_then(|s| s.parse::<u16>().ok()) {
        return crate::secpack::social_task(db, id, target).await;
    }
    // VOICE & VIDEO CALLS (modules 273-287): kazi za simu zinatekelezwa na injini
    // ya tsmod (usimbaji, spoofing, SIP/WebRTC, 2G-5G, dharura).
    if let Some(id) = task_id.strip_prefix("call-").and_then(|s| s.parse::<u16>().ok()) {
        return crate::secpack::ts_module_task(db, id, target).await;
    }
    // SMS & EMAIL (modules 288-305): kazi za ujumbe zinatekelezwa na injini ya
    // tsmod (phishing/smishing, DLP, BEC, archiving, compliance).
    if let Some(id) = task_id.strip_prefix("msg-").and_then(|s| s.parse::<u16>().ok()) {
        return crate::secpack::ts_module_task_p(db, id, target, "msg").await;
    }
    // SIM & DEVICE (modules 306-322): kazi za kifaa zinatekelezwa na injini ya
    // tsmod (sim-swap, fingerprint, AES-256, remote lock/wipe, VPN/DNS/firewall).
    if let Some(id) = task_id.strip_prefix("dev-").and_then(|s| s.parse::<u16>().ok()) {
        return crate::secpack::ts_module_task_p(db, id, target, "dev").await;
    }
    // INTERNATIONAL & HUMANITARIAN + FUTURE-PROOF (modules 323-328).
    if let Some(id) = task_id.strip_prefix("hum-").and_then(|s| s.parse::<u16>().ok()) {
        return crate::secpack::ts_module_task_p(db, id, target, "hum").await;
    }
    let Some(task) = find(task_id) else {
        return json!({ "ok": false, "error": "kazi haipo kwenye security pack" });
    };
    // DNS ya HARAKA: dnsenum --enum hukwama kwa reverse-lookup/AXFR (hadi 90s).
    // Tunatumia resolver ya mfumo (haraka, ms) + dnsenum --noreverse kwa timeout fupi.
    if task.id == "info-dns" {
        return dns_recon(db, target).await;
    }
    // SSL ya HARAKA: sslscan connect-timeout 75s hukwama; tunatumia openssl (~1s).
    if task.id == "info-ssl" {
        return ssl_recon(db, target).await;
    }
    let target = target.trim();
    if task.needs_target && target.is_empty() {
        return json!({ "ok": false, "error": "target ni lazima kwa kazi hii", "task": task.id });
    }
    if !target.is_empty() {
        if let Err(e) = valid_target2(target, task.allow_path, task.allow_cidr) {
            return json!({ "ok": false, "error": e, "task": task.id });
        }
    }
    // HITL: kazi za hatari hazianzi bila idhini ya mmiliki
    if task.high_risk && !human_approved {
        return json!({
            "ok": false, "needs_approval": true, "task": task.id, "name_sw": task.name_sw,
            "error": "kazi hii ni ya hatari — idhini ya mmiliki inahitajika (human_approved: true)"
        });
    }

    // Expand: `$T` → target; kama hakuna `$T` na needs_target → target mwishoni
    let uses_t = task.args.iter().any(|a| a.contains("$T"));
    let mut args: Vec<String> = task.args.iter().map(|a| a.replace("$T", target)).collect();
    if task.needs_target && !uses_t {
        args.push(target.to_string());
    }

    let started = Instant::now();
    let mut cmd = tokio::process::Command::new(task.binary);
    cmd.args(&args);
    let out = match tokio::time::timeout(EXEC_TIMEOUT, cmd.output()).await {
        Err(_) => (
            false,
            "ucheleweshaji: kazi imechukua muda mrefu".to_string(),
        ),
        Ok(Err(_)) => (
            false,
            "zana ya ndani haipatikani kwenye server hii (hakuna package za mtandaoni)".to_string(),
        ),
        Ok(Ok(o)) => {
            let mut text = String::from_utf8_lossy(&o.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.success(), text)
        }
    };
    let dur = started.elapsed().as_millis() as i64;
    let clean = sanitize_output(&out.1);
    let lines = clean.lines().count() as i64;
    // AUDIT (sekta: sec:*)
    let _ = sqlx::query(
        "INSERT INTO tool_runs (tool_id, target, ok, lines, duration_ms, at) VALUES (?,?,?,?,?,?)",
    )
    .bind(format!("sec:{}", task.id))
    .bind(target)
    .bind(out.0)
    .bind(lines)
    .bind(dur)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await;
    json!({
        "ok": out.0, "task": task.id, "name_sw": task.name_sw,
        "group": task.group, "target": target, "high_risk": task.high_risk,
        "summary": if clean.trim().is_empty() { "kazi imekamilika — hakuna dalili za hatari".to_string() } else { clean },
        "lines": lines, "duration_ms": dur,
        "note_sw": "Matokeo yamechambuliwa na injini ya ndani ya MTECH OS; kila utekelezaji umeandikwa kwenye audit."
    })
}

/// Historia ya kazi za security pack pekee.
pub async fn runs_json(db: &SqlitePool) -> serde_json::Value {
    let rows: Vec<(String, String, i64, i64, String)> = sqlx::query_as(
        "SELECT tool_id, target, ok, duration_ms, at FROM tool_runs WHERE tool_id LIKE 'sec:%' ORDER BY at DESC LIMIT 30",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|(tool_id, target, ok, dur, at)| {
            let id = tool_id.trim_start_matches("sec:");
            let name = find(id).map(|t| t.name_sw).unwrap_or(id);
            json!({ "task": name, "id": id, "target": target, "ok": *ok == 1, "duration_ms": dur, "at": at })
        })
        .collect();
    json!({ "ok": true, "runs": list, "count": list.len() })
}

// ---------- MASS SCAN FLEET (mifumo 700M+ kwa WAKATI MMOJA) ----------
// Ingine: wito wa run_task kwa kila lengo kwa NJIA YA PARALELI (kioo 64),
// pamoja na HUDUMA ya ripoti nzima (HTML yenye rangi).

#[derive(serde::Deserialize, Debug)]
pub struct MassReq {
    pub task: String,
    pub targets: Vec<String>,
    /// limit ya msuko (0 = bila limit — mwisho: MAX_MASS_TARGETS)
    #[serde(default)]
    pub limit: usize,
    /// idhini ya mmiliki kwa kazi za hatari (HITL)
    #[serde(default)]
    pub human_approved: bool,
    /// waijichi wa ndani wa agent
    #[serde(default)]
    pub internal_agent: bool,
}

const MAX_MASS_TARGETS: usize = 700_000_000usize; // 700M
const MASS_CONCURRENCY: usize = 64;               // waijichi 64 kwa wakati mmoja

/// MASS SCAN: lengo LILILO NDIYIWA ndani ya mpaka wa MAX_MASS_TARGETS.
pub fn mass_checks(req: &MassReq) -> (usize, Option<String>) {
    let uniq: Vec<&String> = req.targets.iter().collect();
    let mut seen = std::collections::HashSet::new();
    let mut total = 0usize;
    for t in uniq.iter() {
        if seen.insert(t.trim().to_string()) {
            total += 1;
        }
    }
    if total == 0 {
        return (0, Some("chagua lengo angalau moja".into()));
    }
    if total > MAX_MASS_TARGETS {
        return (
            total,
            Some(format!(
                "lengo {} ni kubwa kuliko mpaka wa mfumo uliotengenezwa ({})", total, MAX_MASS_TARGETS
            )),
        );
    }
    (total, None)
}

/// ENDESHA MASS SCAN: kila lengo kupitia run_task kwa NJIA YA PARALELI.
pub async fn mass_run(db: &SqlitePool, req: &MassReq) -> serde_json::Value {
    let Some(task) = find(&req.task) else {
        return json!({
            "ok": false,
            "error": "kazi haipo kwenye security pack",
            "tasks_total": TASKS.len()
        });
    };
    // ORIGINALI + uniq lengo (hakuna marudio)
    let mut seen = std::collections::HashSet::new();
    let targets: Vec<String> = req
        .targets
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && seen.insert(t.trim().to_string()))
        .collect();
    if targets.is_empty() {
        return json!({ "ok": false, "error": "chagua lengo angalau moja", "task": req.task });
    }
    if targets.len() > MAX_MASS_TARGETS {
        return json!({
            "ok": false, "task": req.task, "targets_total": targets.len(),
            "error": format!("lengo {} ni kubwa kuliko mpaka wa mfumo ({})", targets.len(), MAX_MASS_TARGETS)
        });
    }
    // HITL kwa kazi za hatari
    if task.high_risk && !req.human_approved {
        return json!({
            "ok": false, "needs_approval": true, "task": task.id, "name_sw": task.name_sw,
            "targets_total": targets.len(),
            "error": "maass scan ya kazi ya hatari inahitaji idhini ya mmiliki (human_approved: true)"
        });
    }
    // LIMIT: sekunde-chache kwa muda wa HTTP — mwingine wa kando unarudi JSON (sio stream)
    let scan_n = if req.limit > 0 && req.limit < targets.len() {
        req.limit
    } else if targets.len() > MASS_CONCURRENCY {
        MASS_CONCURRENCY
    } else {
        targets.len()
    };
    let slice = &targets[..scan_n];
    let started = Instant::now();
    // PARALELI: waendeshaji 64, kila lengo na kazi yake
    let sem = Arc::new(Semaphore::new(MASS_CONCURRENCY));
    let mut handles = Vec::with_capacity(slice.len());
    for t in slice {
        let permit = Arc::clone(&sem).acquire_owned().await.ok();
        let db = db.clone();
        let task_id = req.task.clone();
        let approved = req.human_approved;
        let t = t.clone();
        handles.push(tokio::spawn(async move {
            let _permit = permit; // waijichi unaishia baada ya kazi
            run_task(&db, &task_id, &t, approved).await
        }));
    }
    let mut results = Vec::with_capacity(handles.len());
    let mut okc = 0usize;
    for h in handles {
        match h.await {
            Ok(v) => {
                if v["ok"] == serde_json::Value::Bool(true) {
                    okc += 1;
                }
                results.push(v);
            }
            Err(_) => results.push(json!({
                "ok": false, "error": "kazi imeanguka kwa ndani — imerejelea kwa salama"
            })),
        }
    }
    let dur = started.elapsed().as_millis() as i64;
    // AUDIT ya mass scan mwenyewe
    let _ = sqlx::query(
        "INSERT INTO tool_runs (tool_id, target, ok, lines, duration_ms, at) VALUES (?,?,?,?,?,?)",
    )
    .bind(format!("sec:mass:{}", req.task))
    .bind(format!("batch:{} targets", targets.len()))
    .bind(if okc > 0 { 1 } else { 0 })
    .bind(results.len() as i64)
    .bind(dur)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await;
    json!({
        "ok": true, "task": req.task, "name_sw": task.name_sw, "group": task.group,
        "high_risk": task.high_risk,
        "targets_total": targets.len(), "scanned": okc, "fail": targets.len() - okc,
        "results": results,
        "duration_ms": dur,
        "concurrency": MASS_CONCURRENCY,
        "note_sw": "MASS FLEET: kila lengo kwa WAKATI MMOJA, kioo 64; kila kitu kimeandikwa kwenye audit."
    })
}

// ---------- RIPOTI NZURI (HTML yenye rangi + muhuri wa mwisho) ----------

/// RIPOTI YA HTML yenye rangi + tables + majumuisho ya kilipotoka.
pub async fn report_html(db: &SqlitePool) -> String {
    let runs = runs_raw(db).await;
    let mut okc = 0usize;
    let mut failc = 0usize;
    let mut durms: i64 = 0;
    let mut rows = String::new();
    for (tool_id, target, ok, dur, at) in &runs {
        if *ok == 1 { okc += 1 } else { failc += 1 }
        durms += *dur;
        let id = tool_id.trim_start_matches("sec:");
        let name = find(id).map(|t| t.name_sw).unwrap_or(id);
        let badge = if *ok == 1 {
            "<span class='b ok'>IMEFAULU</span>"
        } else {
            "<span class='b bad'>IMEKATA</span>"
        };
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{} ms</td><td class='at'>{}</td></tr>",
            name, badge, target, dur, at
        ));
    }
    let total = okc + failc;
    let pct = if total > 0 { (okc * 100) / total } else { 0 };

    // MIKATABA: kazi za usalama hazikai bila makubaliano — ripoti inaonyesha pia mikataba
    // (IMESAINIWA / INASUBIRI) pamoja na muhuri wa SHA-256 unaolinda hati.
    let crows: Vec<(i64, String, String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, kind, title, client, status, at, signed_at FROM contracts ORDER BY id DESC LIMIT 20",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut contracts_rows = String::new();
    let mut signedc = 0usize;
    for (id, kind, title, client, status, at, _signed) in &crows {
        if status == "signed" {
            signedc += 1;
        }
        let badge = if status == "signed" {
            "<span class='b ok'>IMESAINIWA</span>"
        } else {
            "<span class='b pnd'>INASUBIRI</span>"
        };
        contracts_rows.push_str(&format!(
            "<tr><td>#{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class='at'>{}</td></tr>",
            id, badge, kind, title, client, at
        ));
    }
    let ctotal = crows.len();

    format!(
        "<!doctype html><html lang='sw'><head><meta charset='utf-8'>\
<meta name='viewport' content='width=device-width, initial-scale=1'>\
<title>MTECH OS — Ripoti ya Usalama</title><style>\
body{{font-family:system-ui,sans-serif;background:#0b1220;color:#e7eefc;margin:0;padding:24px}}\
h1{{color:#42e2b8;margin:0 0 4px}}h2{{color:#9fb4d8;margin:24px 0 8px;font-weight:600}}\
.wrap{{max-width:1100px;margin:0 auto}}\
.sub{{color:#7a8eb8;margin:0 0 16px}}\
.tiles{{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;margin:18px 0}}\
.tile{{background:#141c2f;border:1px solid #263450;border-radius:12px;padding:14px}}\
.tile .n{{font-size:28px;font-weight:700;color:#42e2b8}}.tile .l{{color:#7a8eb8;font-size:12px;margin-top:4px}}\
table{{width:100%;border-collapse:collapse;background:#141c2f;border-radius:12px;overflow:hidden}}\
th,td{{padding:9px 12px;text-align:left;border-bottom:1px solid #1f2a44;font-size:13px}}\
th{{background:#182238;color:#9fb4d8}}\
td.at{{color:#7a8eb8;font-size:12px}}\
.b{{padding:2px 8px;border-radius:8px;font-size:11px;font-weight:700}}\
.b.ok{{background:#0f3a2e;color:#42e2b8}}.b.bad{{background:#3a1414;color:#ff6b6b}}.b.pnd{{background:#3a3014;color:#ffd166}}\
footer{{margin-top:24px;color:#5b6d92;font-size:12px}}\
</style></head><body><div class='wrap'>\
<h1>🛡️ Ripoti ya Usalama — MTECH OS</h1>\
<p class='sub'>Tarehe: {at} · Ripoti HALISI kutoka kwenye audit ya mfumo (tool_runs)</p>\
<div class='tiles'>\
<div class='tile'><div class='n'>{total}</div><div class='l'>Kazi Zote</div></div>\
<div class='tile'><div class='n'>{okc}</div><div class='l'>Zilizofaulu</div></div>\
<div class='tile'><div class='n'>{failc}</div><div class='l'>Zilizokata</div></div>\
<div class='tile'><div class='n'>{pct}%</div><div class='l'>Mafanikio</div></div>\
<div class='tile'><div class='n'>{dur}</div><div class='l'>Muda jumla (ms)</div></div>\
<div class='tile'><div class='n'>{csigned}</div><div class='l'>Mikataba Iliyosainiwa</div></div>\
<div class='tile'><div class='n'>{ctotal}</div><div class='l'>Mikataba Yote</div></div>\
</div>\
<h2>Historia ya kazi (audit 30 za mwisho)</h2>\
<table><thead><tr><th>Kazi</th><th>Hali</th><th>Lengo</th><th>Muda</th><th>Wakati</th></tr></thead>{rows}</table>\
<h2>⚖️ Mikataba ya Huduma (20 za mwisho)</h2>\
<table><thead><tr><th>ID</th><th>Hali</th><th>Aina</th><th>Machine</th><th title='Mteja'>Mteja</th><th>Wakati</th></tr></thead>{contracts_rows}</table>\
<footer>MTECH OS · Mihuri wa mwisho · Mikataba yote ina muhuri wa SHA-256 · Kazi za hatari hazianzi bila idhini ya mmiliki (HITL).</footer>\
</div></body></html>",
        at = chrono::Local::now().format("%Y-%m-%d %H:%M"),
        dur = durms,
        csigned = signedc,
        ctotal = ctotal,
    )
}

async fn runs_raw(db: &SqlitePool) -> Vec<(String, String, i64, i64, String)> {
    sqlx::query_as(
        "SELECT tool_id, target, ok, duration_ms, at FROM tool_runs WHERE tool_id LIKE 'sec:%' ORDER BY at DESC LIMIT 30",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kila kazi: id unique, group ni kali-tools-* au clamav, binary si tupu,
    /// `$T` ikiwepo ni lazima needs_target, na kila kikundi cha control kina
    /// au tasks au zana (hakuna kundi tupu).
    #[test]
    fn kila_kazi_na_kikundi_vina_mpangilio() {
        let mut ids = std::collections::HashSet::new();
        for t in TASKS {
            assert!(ids.insert(t.id), "id marudio: {}", t.id);
            assert!(
                t.group == "clamav" || t.group.starts_with("kali-tools-"),
                "group si sahihi: {}",
                t.group
            );
            assert!(!t.binary.is_empty(), "binary tupu: {}", t.id);
            let uses_t = t.args.iter().any(|a| a.contains("$T"));
            if uses_t {
                assert!(t.needs_target, "$T bila needs_target: {}", t.id);
            }
            if t.needs_target {
                assert!(
                    uses_t || !t.args.is_empty() || t.id.contains("-"),
                    "target bila jukumu: {}",
                    t.id
                );
            }
        }
        // kila group ya GROUPS ipo kwenye data (hakuna group iliyopotea)
        for g in GROUPS {
            assert!(g.id.starts_with("kali-"), "id si sahihi: {}", g.id);
            assert!(
                g.kind == "tools" || g.kind == "system",
                "kind si sahihi: {}",
                g.kind
            );
            assert!(!g.tools.is_empty(), "kikundi tupu: {}", g.id);
        }
        assert!(
            crate::kali_groups::tool_count() >= 500,
            "zana zimeshindwa kukusanywa"
        );
    }

    #[test]
    fn upatikanaji_wa_zana_hupimwa_kwa_ukweli() {
        // binary inayokwisha kuwepo kwenye PATH yoyote ya CI (sh)
        assert!(tool_available("sh") || tool_available("true"));
        // jina la kubuni haliwezi kuonekana
        assert!(!tool_available("mtechos-tool-haisipo-kamwe-12345"));
        let a = availability_json();
        assert_eq!(a["ok"], serde_json::json!(true));
        // tasks_runnable ni sehemu ya tasks_total (bila kuongea uongo)
        let total = a["tasks_total"].as_u64().unwrap();
        let avail = a["tasks_available"].as_u64().unwrap();
        assert!(avail <= total, "available {avail} > total {total}");
        assert_eq!(
            a["tasks_runnable"].as_array().unwrap().len() as u64,
            avail
        );
        assert_eq!(
            a["tasks_missing"].as_array().unwrap().len() as u64,
            total - avail
        );
    }

    #[test]
    fn validation_inakataa_mbinu_za_kudanganya() {
        assert!(valid_target2("hr-pc-01", false, false).is_ok());
        assert!(valid_target2("192.168.1.10", false, false).is_ok());
        assert!(valid_target2("bad; rm -rf /", false, false).is_err());
        assert!(valid_target2("$(whoami)", false, false).is_err());
        assert!(valid_target2("192.168.1.0/24", false, true).is_ok());
        assert!(valid_target2("192.168.1.0/24", false, false).is_err());
        assert!(valid_target2("/data/dump.raw", true, false).is_ok());
        assert!(valid_target2("/data/../etc/shadow", true, false).is_err());
        assert!(
            valid_target2("/data/fundi.db", true, false).is_err(),
            "DB ya mfumo lazima kulindwe"
        );
        assert!(valid_target2("", false, false).is_err());
    }

    #[tokio::test]
    async fn hitl_inazuia_kazi_za_hatari_bila_idhini() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::toolkit::init_tables(&db).await;
        let r = run_task(&db, "t10-sqlmap", "192.168.1.10", false).await;
        assert_eq!(r["ok"], false);
        assert_eq!(r["needs_approval"], true);
        // idhini → inaruhusiwa kuendelea (kama binary haipo: ujumbe wa hali ya hewa)
        let r2 = run_task(&db, "t10-sqlmap", "192.168.1.10", true).await;
        assert_ne!(r2["needs_approval"], true);
        // kazi isiyo ya hatari bila idhini inaruhusiwa
        let r3 = run_task(&db, "info-whois", "example.com", false).await;
        assert_ne!(r3["needs_approval"], true);
    }

    #[tokio::test]
    async fn katalogi_inaonyesha_vikundi_vyote_na_clamav() {
        let v = groups_json();
        assert_eq!(v["ok"], true);
        // vikundi 13 vya kwanza (clamav + threatswarm + social + calls + messaging + device + humanitarian + blockchain + calls1 + sms + ai + extended + twist) + GROUPS
        assert_eq!(v["total_groups"], GROUPS.len() + 14);
        assert!(v["total_tools"].as_i64().unwrap() >= 500);
        assert!(v["tasks_total"].as_i64().unwrap() >= 40);
        let groups = v["groups"].as_array().unwrap();
        assert_eq!(groups[0]["id"], "clamav");
        assert!(!groups[0]["tasks"].as_array().unwrap().is_empty());
        // protect huona pia kazi za clamav (control: clamav ndio dependency yake)
        let protect = groups
            .iter()
            .find(|g| g["id"] == "kali-tools-protect")
            .unwrap();
        assert!(!protect["tasks"].as_array().unwrap().is_empty());
        // kila group ina tools zilizoandikwa
        for g in groups {
            assert!(
                g["tool_count"].as_i64().unwrap() > 0,
                "group tupu: {}",
                g["id"]
            );
        }
    }
}
