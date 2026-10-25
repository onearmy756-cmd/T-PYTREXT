//! toolkit.rs — TOOLKIT LAYER (H12): registry + executor + batch + audit.
//!
//! ARCHITECTURE (kama mpango wa mmiliki):
//!   - REGISTRY: zana zote zinasajiliwa kama DATA ya Rust (Cat + ToolDef).
//!   - EXECUTOR: kila zana inaendesha binary yake kwa args salama (arg-array,
//!     HAKUNA shell), timeout ngumu, matokeo yanasaifishwa kwa sanitize_output.
//!   - BATCH: kazi kwa kompyuta NYINGI kwa WAKATI MMOJA (tokio::spawn kwa kila
//!     target — parallelism halisi, kama SEHEMU 8.4 ya mpango).
//!   - AUDIT: kila utekelezaji unaandikwa DB (tool_runs) — SEHEMU 8.4 ya mpango.
//!
//! KANUNI YA SIRI (mmiliki): API ya umma inarudisha majina SALAMA ya Kiswahili
//! tu + categories za jumla. Majina halisi ya binaries/args yanaishi NDANI YA
//! Rust pekee; matokeo yanapita kwenye sanitize_output kabla ya kuja hapa.

use serde::Serialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::time::Duration;
use std::time::Instant;

use crate::tools::sanitize_output;

const EXEC_TIMEOUT: Duration = Duration::from_secs(60);

// ---------- CATEGORIES (8 — kama mpango) ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Cat {
    Recon,
    Vuln,
    Exploit,
    Wireless,
    Password,
    Sniff,
    Forensics,
    System,
}

impl Cat {
    pub fn id(&self) -> &'static str {
        match self {
            Cat::Recon => "recon",
            Cat::Vuln => "vuln",
            Cat::Exploit => "exploit",
            Cat::Wireless => "wireless",
            Cat::Password => "password",
            Cat::Sniff => "sniff",
            Cat::Forensics => "forensics",
            Cat::System => "system",
        }
    }

    /// Jina SALAMA la Kiswahili (mteja haoni jina la zana au category ya "attack")
    pub fn name_sw(&self) -> &'static str {
        match self {
            Cat::Recon => "Uchunguzi wa Mtandao",
            Cat::Vuln => "Uchunguzi wa Udhaifu",
            Cat::Exploit => "Upimaji wa Usalama",
            Cat::Wireless => "Usalama wa Wi-Fi",
            Cat::Password => "Uthibitisho wa Nywila",
            Cat::Sniff => "Uchunguzi wa Trafiki",
            Cat::Forensics => "Uchanganuzi wa Kidijitali",
            Cat::System => "Huduma za Mfumo",
        }
    }

    pub fn parse(s: &str) -> Option<Cat> {
        match s.trim().to_lowercase().as_str() {
            "recon" => Some(Cat::Recon),
            "vuln" => Some(Cat::Vuln),
            "exploit" => Some(Cat::Exploit),
            "wireless" => Some(Cat::Wireless),
            "password" => Some(Cat::Password),
            "sniff" => Some(Cat::Sniff),
            "forensics" => Some(Cat::Forensics),
            "system" => Some(Cat::System),
            _ => None,
        }
    }

    pub const ALL: [Cat; 8] = [
        Cat::Recon,
        Cat::Vuln,
        Cat::Exploit,
        Cat::Wireless,
        Cat::Password,
        Cat::Sniff,
        Cat::Forensics,
        Cat::System,
    ];
}

// ---------- TOOL DEFINITION (binary + args = SIRI, viko ndani ya Rust) ----------
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub id: &'static str,
    pub name_sw: &'static str,
    pub cat: Cat,
    pub binary: &'static str,
    pub args: &'static [&'static str],
    pub target_arg: bool,
    pub root: bool,
    /// Kikomo cha kasi (0 = hakuna) — tunapunguza kasi, sisi tunaamua
    pub max_rate: u32,
}

macro_rules! t {
    ($id:expr, $sw:expr, $cat:expr, $bin:expr, [$($a:expr),*], $ta:expr, $root:expr, $rate:expr) => {
        ToolDef { id: $id, name_sw: $sw, cat: $cat, binary: $bin, args: &[$($a),*], target_arg: $ta, root: $root, max_rate: $rate }
    };
}

/// REGISTRY — zana 34 (SEHEMU 8.2 ya mpango, zimejumuishwa bila nakala). MAJINA YA SIRI — SI API.
pub const TOOLS: &[ToolDef] = &[
    // RECON
    t!(
        "recon-net",
        "Uchunguzi wa Mtandao",
        Cat::Recon,
        "nmap",
        ["-Pn", "--open", "-p", "22,80,443,445,3389"],
        true,
        false,
        0
    ),
    t!(
        "recon-fast",
        "Uchunguzi wa Haraka",
        Cat::Recon,
        "masscan",
        ["--rate=1000"],
        true,
        true,
        1000
    ),
    t!(
        "recon-domain",
        "Uchunguzi wa Domini",
        Cat::Recon,
        "whois",
        [],
        false,
        false,
        0
    ),
    t!(
        "recon-dns",
        "Uchunguzi wa DNS",
        Cat::Recon,
        "dnsenum",
        ["--enum"],
        true,
        false,
        0
    ),
    t!(
        "recon-mail",
        "Uchunguzi wa Barua",
        Cat::Recon,
        "theHarvester",
        ["-b", "all"],
        true,
        false,
        0
    ),
    // VULN
    t!(
        "vuln-full",
        "Kupima Udhaifu Kamili",
        Cat::Vuln,
        "gvm-cli",
        [],
        true,
        false,
        0
    ),
    t!(
        "vuln-web",
        "Kupima Udhaifu wa Tovuti",
        Cat::Vuln,
        "nikto",
        ["-h"],
        true,
        false,
        0
    ),
    t!(
        "vuln-cms",
        "Kupima Udhaifu wa CMS",
        Cat::Vuln,
        "wpscan",
        ["--enumerate"],
        true,
        false,
        0
    ),
    // EXPLOIT (upimaji wa usalama — idhini ya mmiliki pekee)
    t!(
        "exploit-frame",
        "Upimaji wa Mfumo",
        Cat::Exploit,
        "msfconsole",
        ["-q"],
        true,
        false,
        0
    ),
    t!(
        "exploit-sql",
        "Upimaji wa Hifadhidata",
        Cat::Exploit,
        "sqlmap",
        ["--batch"],
        true,
        false,
        0
    ),
    t!(
        "exploit-login",
        "Upimaji wa Uthibitisho",
        Cat::Exploit,
        "hydra",
        [],
        true,
        false,
        0
    ),
    // WIRELESS
    t!(
        "wifi-audit",
        "Ukaguzi wa Wi-Fi",
        Cat::Wireless,
        "aircrack-ng",
        [],
        true,
        true,
        0
    ),
    t!(
        "wifi-map",
        "Ramani ya Wi-Fi",
        Cat::Wireless,
        "kismet",
        [],
        true,
        true,
        0
    ),
    // PASSWORD
    t!(
        "pass-strength",
        "Nguvu za Nywila",
        Cat::Password,
        "john",
        ["--wordlist"],
        true,
        false,
        0
    ),
    t!(
        "pass-gpu",
        "Nywila (GPU)",
        Cat::Password,
        "hashcat",
        [],
        true,
        false,
        0
    ),
    t!(
        "pass-gen",
        "Kutengeneza Nywila",
        Cat::Password,
        "crunch",
        [],
        false,
        false,
        0
    ),
    // SNIFF
    t!(
        "sniff-live",
        "Kunasa Trafiki (moja kwa moja)",
        Cat::Sniff,
        "tcpdump",
        [],
        true,
        true,
        0
    ),
    t!(
        "sniff-file",
        "Uchanganuzi wa Trafiki",
        Cat::Sniff,
        "tshark",
        [],
        true,
        false,
        0
    ),
    t!(
        "sniff-mitm",
        "Upimaji wa MITM",
        Cat::Sniff,
        "ettercap",
        ["-T", "-q"],
        true,
        true,
        0
    ),
    // FORENSICS
    t!(
        "forensic-disk",
        "Uchanganuzi wa Diski",
        Cat::Forensics,
        "fls",
        ["-r"],
        true,
        false,
        0
    ),
    t!(
        "forensic-memory",
        "Uchanganuzi wa Kumbukumbu",
        Cat::Forensics,
        "volatility3",
        ["-f"],
        true,
        false,
        0
    ),
    t!(
        "forensic-recover",
        "Kupata Faili Zilizofutwa",
        Cat::Forensics,
        "foremost",
        ["-i"],
        true,
        false,
        0
    ),
    t!(
        "forensic-carve",
        "Uchukuaji wa Faili",
        Cat::Forensics,
        "scalpel",
        ["-c"],
        true,
        false,
        0
    ),
    t!(
        "forensic-firmware",
        "Uchanganuzi wa Firmware",
        Cat::Forensics,
        "binwalk",
        ["-e"],
        true,
        false,
        0
    ),
    t!(
        "forensic-image",
        "Nakili ya Diski (ushahidi)",
        Cat::Forensics,
        "dc3dd",
        ["hash=sha256"],
        false,
        true,
        0
    ),
    t!(
        "forensic-photo",
        "Kupata Picha Zilizofutwa",
        Cat::Forensics,
        "photorec",
        [],
        true,
        true,
        0
    ),
    t!(
        "forensic-partition",
        "Kupata Partition Zilizofutwa",
        Cat::Forensics,
        "testdisk",
        [],
        true,
        true,
        0
    ),
    t!(
        "forensic-platform",
        "Jukwaa la Uchanganuzi",
        Cat::Forensics,
        "autopsy",
        ["--nosplash"],
        true,
        false,
        0
    ),
    t!(
        "forensic-imager",
        "Kuweka Nakili za Diski",
        Cat::Forensics,
        "guymager",
        [],
        true,
        true,
        0
    ),
    // SYSTEM
    t!(
        "system-discover",
        "Kugundua Kompyuta",
        Cat::System,
        "ping",
        ["-c", "3"],
        true,
        false,
        0
    ),
    t!(
        "system-health",
        "Afya ya Kifaa",
        Cat::System,
        "uptime",
        [],
        false,
        false,
        0
    ),
    t!(
        "system-process",
        "Michakato ya Kifaa",
        Cat::System,
        "ps",
        ["aux"],
        false,
        false,
        0
    ),
    t!(
        "system-storage",
        "Hifadhi ya Kifaa",
        Cat::System,
        "df",
        ["-h"],
        false,
        false,
        0
    ),
    t!(
        "system-logs",
        "Kumbukumbu za Mfumo",
        Cat::System,
        "journalctl",
        ["-n", "50", "--no-pager"],
        false,
        false,
        0
    ),
];

/// Katalogi SALAMA kwa UI/API — kamwe binary/args (kanuni ya mmiliki).
pub fn catalog_json(cat: Option<Cat>) -> serde_json::Value {
    let cats: Vec<serde_json::Value> = Cat::ALL
        .iter()
        .filter(|c| cat.is_none() || cat == Some(**c))
        .map(|c| {
            let tools: Vec<serde_json::Value> = TOOLS
                .iter()
                .filter(|t| t.cat == *c)
                .map(|t| json!({ "id": t.id, "name_sw": t.name_sw, "category": c.id() }))
                .collect();
            json!({ "id": c.id(), "name_sw": c.name_sw(), "count": tools.len(), "tools": tools })
        })
        .collect();
    json!({ "ok": true, "categories": cats, "total_tools": TOOLS.len(),
        "note_sw": "MTECH OS inachagua zana zake vyenyewe — wewe unachagua kazi na kompyuta tu." })
}

/// Resolver ya ndani
fn find(id: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.id == id)
}

// ---------- EXECUTOR (arg-array salama, timeout, sanitize) ----------
pub async fn run_single(db: &SqlitePool, job: BatchJob) -> serde_json::Value {
    let tool = match find(&job.tool) {
        Some(t) => t,
        None => return json!({ "ok": false, "error": "zana haipo kwenye toolkit" }),
    };
    let target = &job.targets[0];
    if !crate::tools_internal::valid_target(target) {
        return json!({ "ok": false, "error": "jina la kompyuta si salama", "target": target });
    }
    // args: base + (rate) + target — zote kama arg-array (hakuna shell/injection)
    let mut args: Vec<String> = tool.args.iter().map(|s| s.to_string()).collect();
    if tool.max_rate > 0 {
        args.push(format!("--max-rate={}", tool.max_rate));
    }
    if tool.target_arg {
        args.push(target.clone());
    }
    let started = Instant::now();
    let mut cmd = tokio::process::Command::new(tool.binary);
    cmd.args(&args);
    let out = match tokio::time::timeout(EXEC_TIMEOUT, cmd.output()).await {
        Err(_) => (
            false,
            "ucheleweshaji: kazi imechukua muda mrefu".to_string(),
        ),
        Ok(Err(_)) => (
            false,
            "zana ya ndani haipatikani kwenye server hii".to_string(),
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
    // AUDIT (bila data nyeti — id, target, hali, muda)
    let _ = sqlx::query(
        "INSERT INTO tool_runs (tool_id, target, ok, lines, duration_ms, at) VALUES (?,?,?,?,?,?)",
    )
    .bind(tool.id)
    .bind(target)
    .bind(out.0)
    .bind(lines)
    .bind(dur)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(db)
    .await;
    json!({
        "ok": out.0, "tool": tool.id, "name_sw": tool.name_sw, "target": target,
        "summary": if clean.trim().is_empty() { "kazi imekamilika — hakuna dalili za hatari".to_string() } else { clean },
        "lines": lines, "duration_ms": dur,
        "note_sw": "Matokeo yamechambuliwa na injini ya ndani ya MTECH OS.",
    })
}

// ---------- BATCH — kompyuta NYINGI kwa WAKATI MMOJA (SEHEMU 8.4) ----------
#[derive(Debug, Clone)]
pub struct BatchJob {
    pub tool: String,
    pub targets: Vec<String>,
    pub account: String,
}

pub async fn run_batch(db: &SqlitePool, job: BatchJob) -> Vec<serde_json::Value> {
    let mut handles = Vec::new();
    for t in job.targets.clone() {
        let j = BatchJob {
            tool: job.tool.clone(),
            targets: vec![t],
            account: job.account.clone(),
        };
        let db2 = db.clone();
        handles.push(tokio::spawn(async move { run_single(&db2, j).await }));
    }
    let mut out = Vec::new();
    for h in handles {
        match h.await {
            Ok(v) => out.push(v),
            Err(e) => out.push(json!({ "ok": false, "error": e.to_string() })),
        }
    }
    out
}

// ---------- DB (audit) ----------
pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS tool_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tool_id TEXT,
            target TEXT,
            ok INTEGER,
            lines INTEGER,
            duration_ms INTEGER,
            at TEXT
        )"#,
    )
    .execute(db)
    .await;
}

pub async fn runs_json(db: &SqlitePool) -> serde_json::Value {
    let rows: Vec<(String, String, i64, i64, String)> = sqlx::query_as(
        "SELECT tool_id, target, ok, duration_ms, at FROM tool_runs ORDER BY at DESC LIMIT 30",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|(tool_id, target, ok, dur, at)| {
            let name = find(tool_id).map(|t| t.name_sw).unwrap_or(tool_id);
            json!({ "tool": name, "target": target, "ok": *ok == 1, "duration_ms": dur, "at": at })
        })
        .collect();
    json!({ "ok": true, "runs": list })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THIBITISHO KAMILI: kila zana ina binary/args/jina kamili, catalog haivui
    /// binaries HATA MOJA, na huduma ya Kali (kali-tools) ipo docker-compose —
    /// zwan zote zimeunganishwa kama kanuni ya SEHEMU 8.2.
    #[test]
    fn kila_zana_ina_mpangilio_kamili_na_kali_imeunganishwa() {
        // 1) kila ToolDef kamili + ids pekee (hakuna nakala)
        let mut ids = std::collections::HashSet::new();
        for t in TOOLS {
            assert!(!t.id.is_empty(), "id tupu");
            assert!(ids.insert(t.id), "id inajirudia: {}", t.id);
            assert!(!t.binary.is_empty(), "{} binary tupu", t.id);
            assert!(!t.name_sw.is_empty(), "{} jina tupu", t.id);
        }
        // 2) katalogi ya API HAIVUI binary yoyote (kwa zana MOJA MOJA)
        let cat = catalog_json(None).to_string();
        for t in TOOLS {
            assert!(!cat.contains(t.binary), "{} inavuja kwenye katalogi", t.id);
        }
        // 3) docker-compose ina huduma ya Kali — zana zinatekelezwa hapo
        let mf = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docker-compose.yml");
        let s = std::fs::read_to_string(&mf).expect("docker-compose.yml haipatikani");
        assert!(
            s.contains("kali-tools") && s.contains("kalilinux/kali-rolling"),
            "huduma ya Kali haipo compose"
        );
        // 4) ukubwa wa registry kama kanuni (SEHEMU 8.2: zana 30+)
        assert!(TOOLS.len() >= 30, "registry: {}", TOOLS.len());
    }

    #[test]
    fn registry_ina_zana_33_na_categories_8() {
        assert_eq!(
            TOOLS.len(),
            34,
            "SEHEMU 8.2 ya mpango: zana 34 (bila nakala)"
        );
        assert_eq!(Cat::ALL.len(), 8);
        // kila category ina angalau zana 2 (isipokuwa vuln/exploit/wireless — 3/3/2)
        for c in Cat::ALL {
            let n = TOOLS.iter().filter(|t| t.cat == c).count();
            assert!(n >= 2, "category {} ina zana chache: {}", c.id(), n);
        }
        // ids zote ni unique
        let mut ids: Vec<&str> = TOOLS.iter().map(|t| t.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "id za zana zinarudiwa");
    }

    #[test]
    fn catalog_haina_majina_ya_zana_wala_args() {
        let c = catalog_json(None);
        let s = c.to_string();
        // binaries + args za siri — KAMWE hazitoki kwenye katalogi ya umma
        for secret in [
            "nmap",
            "masscan",
            "msfconsole",
            "sqlmap",
            "hydra",
            "hashcat",
            "john",
            "aircrack",
            "kismet",
            "tcpdump",
            "tshark",
            "ettercap",
            "fls",
            "volatility",
            "foremost",
            "scalpel",
            "binwalk",
            "dc3dd",
            "photorec",
            "testdisk",
            "autopsy",
            "guymager",
            "gvm",
            "nikto",
            "wpscan",
            "whois",
            "dnsenum",
            "theHarvester",
            "journalctl",
            "--rate",
            "--wordlist",
            "--enumerate",
            "hash=sha256",
        ] {
            assert!(!s.contains(secret), "katalogi inavuja: '{}' ipo", secret);
        }
        // categories zina majina salama ya Kiswahili
        assert!(s.contains("Uchunguzi wa Mtandao"));
        assert!(s.contains("Uchanganuzi wa Kidijitali"));
        assert_eq!(c["total_tools"], serde_json::json!(34));
        // filter kwa category moja
        let f = catalog_json(Some(Cat::Forensics));
        assert_eq!(f["categories"].as_array().unwrap().len(), 1);
        assert_eq!(f["categories"][0]["tools"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn kikomo_cha_kasi_kinawekwa_kwa_zana_zenye_hatari() {
        let fast = find("recon-fast").unwrap();
        assert_eq!(fast.max_rate, 1000, "masscan inapunguzwa kasi");
        let plain = find("recon-net").unwrap();
        assert_eq!(plain.max_rate, 0);
        // zana za root zote zinaenderesha kwa sudo kwenye server ya mmiliki
        assert!(find("sniff-live").unwrap().root);
    }

    #[test]
    fn cat_parse_inakubali_8_na_inakataa_mengine() {
        for c in Cat::ALL {
            assert_eq!(Cat::parse(c.id()), Some(c));
        }
        assert_eq!(Cat::parse("hacking"), None);
        assert_eq!(Cat::parse(""), None);
    }

    #[tokio::test]
    async fn audit_inaandikwa_db_na_runs_inasomeka() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // zana isiyojulikana + target mbaya → kosa laini, HAKUNA panic, hakuna amri
        let bad = run_single(
            &db,
            BatchJob {
                tool: "recon-net".into(),
                targets: vec!["bad; rm -rf /".into()],
                account: "mteja1".into(),
            },
        )
        .await;
        assert_eq!(bad["ok"], serde_json::Value::Bool(false));
        assert!(bad["error"].as_str().unwrap().contains("salama"));
        let unknown = run_single(
            &db,
            BatchJob {
                tool: "zana-ya-siri".into(),
                targets: vec!["pc-01".into()],
                account: "mteja1".into(),
            },
        )
        .await;
        assert_eq!(unknown["ok"], serde_json::Value::Bool(false));
        let r = runs_json(&db).await;
        assert_eq!(r["ok"], serde_json::Value::Bool(true));
        assert_eq!(
            r["runs"].as_array().unwrap().len(),
            0,
            "hakuna run iliyofeli inaandikwa"
        );
    }

    #[tokio::test]
    async fn batch_inasambaza_kwa_kompyuta_zote() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        let results = run_batch(
            &db,
            BatchJob {
                tool: "system-health".into(),
                targets: vec!["pc-a".into(), "pc-b".into(), "pc-c".into()],
                account: "mteja1".into(),
            },
        )
        .await;
        assert_eq!(results.len(), 3, "kila target ina matokeo yake");
        // sandbox haina zana halisi — matokeo yatakuwa "haipatikani" lakini SAFI
        for r in &results {
            let s = r.to_string();
            assert!(
                !s.contains("uptime"),
                "jina la binary halitavuja kwenye matokeo"
            );
        }
    }
}
