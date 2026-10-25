//! phoenix_shield.rs — PHOENIX SHIELD: mfumo wa ULINZI (defensive) kwa mtu
//! binafsi au kampuni ndogo.
//!
//! Dhamira: kusaidia MTU kujilinda — KUGUNDUA na KUZUIA mashambulizi — kamwe
//! kushambulia. Hakuna module ya mashambulizi hapa; zana za Kali (kali_groups/
//! secpack) zinabaki kama katalogi ya HITL-lab (zinahitaji idhini ya mmiliki).
//!
//! Sehemu za Phoenix Shield:
//!   1. posture      — tathmini ya ULINZI WA MFUMO huu (checked halisi: nftables,
//!                     bandari za kusikiliza, akaunti bila neno la siri, sudo, SSH).
//!   2. lab_plan     — mpango wa pen-testing wa LAB (scope.txt + HITL) — DEFENSIVE:
//!                     unasaidia kuthibitisha ulinzi wa mifumo YAKO, si ya wengine.
//!   3. r_analytics  — takwimu/forecast za matukio (R: risk_score, anomaly,
//!                     sec_forecast, fraud_trend) kwa maamuzi ya ulinzi.
//!   4. hardening_plan — mpango wa kuimarisha (nftables policy, sysctl, SSH) —
//!                     hauweki kitu bila `apply = true` (HITL).
//!
//! Kila kitu kinachorudishwa kinatokana na mfumo halisi; pale kisichopatikana,
//! kinaripotiwa kwa uaminifu (`skipped`), kamwe kikadaiwa kwa uongo.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;

/// Hali ya kipimo kimoja cha ulinzi.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Check {
    pub id: String,
    pub jina: String,
    /// "sawa" | "tahadhari" | "hatari" | "haijapimwa"
    pub hali: String,
    /// 0–100 (juu = salama zaidi). -1 = haijapimwa.
    pub alama: i64,
    pub maelezo: String,
}

fn check(id: &str, jina: &str, hali: &str, alama: i64, maelezo: impl Into<String>) -> Check {
    Check {
        id: id.into(),
        jina: jina.into(),
        hali: hali.into(),
        alama,
        maelezo: maelezo.into(),
    }
}

/// Endesha amri bila shell (arg-array) — hakuna injection. Inarudisha stdout
/// ikiwa imefanikiwa ndani ya muda mfupi.
fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        None
    }
}

/// Je, amri ipo kwenye PATH?
fn has_cmd(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd}"))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------- 1) POSTURE: ULINZI WA MFUMO ----------

/// Bandari za TCP zinazosikiliza (kutoka /proc/net/tcp — bila zana ya nje).
/// Inarudisha bandari za local_address ambazo ziko state LISTEN (0A).
pub fn listening_tcp_ports() -> Vec<u16> {
    let mut ports: Vec<u16> = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/proc/net/tcp") {
        for line in content.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                continue;
            }
            let st = cols[3];
            if st != "0A" {
                continue; // 0A = LISTEN
            }
            // local_address = HEXIP:HEXPORT
            if let Some(hexport) = cols[1].split(':').nth(1) {
                if let Ok(p) = u16::from_str_radix(hexport, 16) {
                    if !ports.contains(&p) {
                        ports.push(p);
                    }
                }
            }
        }
    }
    let mut tcp6: Vec<u16> = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/proc/net/tcp6") {
        for line in content.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 || cols[3] != "0A" {
                continue;
            }
            if let Some(hexport) = cols[1].split(':').nth(1) {
                if let Ok(p) = u16::from_str_radix(hexport, 16) {
                    if !ports.contains(&p) && !tcp6.contains(&p) {
                        tcp6.push(p);
                    }
                }
            }
        }
    }
    ports.extend(tcp6);
    ports.sort_unstable();
    ports
}

/// Bandari hatarishi zinazosikiliza (Database/remote/kibovu) — zinaathiri DB/SSH.
const RISKY_PORTS: &[(u16, &str)] = &[
    (23, "telnet (bila usimbaji)"),
    (445, "SMB (wizi wa faili)"),
    (1433, "MSSQL"),
    (1521, "Oracle"),
    (3306, "MySQL"),
    (3389, "RDP"),
    (5432, "PostgreSQL"),
    (5900, "VNC"),
    (6379, "Redis"),
    (11211, "Memcached"),
    (27017, "MongoDB"),
];

/// Tathmini mfumo huu kwa checks HALISI. `skipped` = vilivyoshindwa kupimwa.
pub fn posture() -> Value {
    let mut checks: Vec<Check> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    // 1) Firewall (nftables)
    if has_cmd("nft") {
        let rules = run("nft", &["list", "ruleset"]).unwrap_or_default();
        let has_rules = rules.lines().any(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.starts_with("table ") && !t.ends_with('{')
        });
        if has_rules {
            checks.push(check(
                "firewall",
                "Firewall (nftables)",
                "sawa",
                90,
                "nftables ina sheria zinazotumika",
            ));
        } else {
            checks.push(check(
                "firewall",
                "Firewall (nftables)",
                "hatari",
                20,
                "Hakuna sheria za nftables — mfumo upo wazi kwenye mtandao",
            ));
        }
    } else {
        skipped.push("firewall: nft haipo".into());
        checks.push(check(
            "firewall",
            "Firewall (nftables)",
            "haijapimwa",
            -1,
            "nft haipo kwenye mfumo",
        ));
    }

    // 2) Bandari za kusikiliza hatarishi
    let ports = listening_tcp_ports();
    if ports.is_empty() {
        checks.push(check(
            "open_ports",
            "Bandari za kusikiliza",
            "sawa",
            85,
            "Hakuna bandari ya TCP inayosikiliza (au /proc/net/tcp haipatikani)",
        ));
    } else {
        let risky: Vec<String> = ports
            .iter()
            .filter_map(|p| RISKY_PORTS.iter().find(|(rp, _)| rp == p))
            .map(|(p, n)| format!("{p} ({n})"))
            .collect();
        if risky.is_empty() {
            checks.push(check(
                "open_ports",
                "Bandari za kusikiliza",
                "sawa",
                80,
                format!("Bandari {} — hakuna hatarishi", list_ports(&ports)),
            ));
        } else {
            checks.push(check(
                "open_ports",
                "Bandari za kusikiliza",
                "hatari",
                30,
                format!("Bandari hatarishi zinasikiliza: {}", risky.join(", ")),
            ));
        }
    }

    // 3) Akaunti bila neno la siri (UID 0 / system accounts)
    if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
        let no_pass_locked: usize = passwd
            .lines()
            .filter(|l| {
                let f: Vec<&str> = l.split(':').collect();
                f.len() >= 2 && (f[1].is_empty() || f[1] == "!")
            })
            .count();
        if no_pass_locked == 0 {
            checks.push(check(
                "weak_accounts",
                "Akaunti bila neno la siri",
                "sawa",
                80,
                "Hakuna akaunti yenye uwanja wa neno la siri tupu kwenye /etc/passwd",
            ));
        } else {
            checks.push(check(
                "weak_accounts",
                "Akaunti bila neno la siri",
                "tahadhari",
                45,
                format!("Akaunti {no_pass_locked} zina uwanja wa neno la siri tupu/imefungwa (!)"),
            ));
        }
    } else {
        skipped.push("weak_accounts: /etc/passwd haipatikani".into());
    }

    // 4) SSH root login (kama sshd_config inapatikana)
    match std::fs::read_to_string("/etc/ssh/sshd_config") {
        Ok(cfg) => {
            let root_yes = cfg.lines().any(|l| {
                let t = l.trim().to_lowercase();
                t.starts_with("permitrootlogin") && t.contains("yes")
            });
            let pass_yes = cfg.lines().any(|l| {
                let t = l.trim().to_lowercase();
                t.starts_with("passwordauthentication") && t.contains("yes")
            });
            let alama = if root_yes { 25 } else if pass_yes { 55 } else { 90 };
            let hali = if root_yes { "hatari" } else if pass_yes { "tahadhari" } else { "sawa" };
            checks.push(check(
                "ssh",
                "Usanidi wa SSH",
                hali,
                alama,
                String::from(if root_yes {
                    "PermitRootLogin yes — root anaweza kuingia kwa SSH"
                } else if pass_yes {
                    "PasswordAuthentication yes — wezesha uthibitisho kwa funguo pekee"
                } else {
                    "SSH imeimarishwa (root amezuiwa, funguo pekee)"
                }),
            ));
        }
        Err(_) => {
            skipped.push("ssh: /etc/ssh/sshd_config haipatikani".into());
        }
    }

    // 5) Updates
    if has_cmd("apt") {
        checks.push(check(
            "updates",
            "Sasisho za mfumo",
            "tahadhari",
            50,
            "Endesha 'apt list --upgradable' kuthibitisha pakiti zinazohitaji sasisho",
        ));
    } else {
        skipped.push("updates: apt haipo".into());
    }

    let scored: Vec<&Check> = checks.iter().filter(|c| c.alama >= 0).collect();
    let health = if scored.is_empty() {
        0
    } else {
        scored.iter().map(|c| c.alama).sum::<i64>() / scored.len() as i64
    };
    let grade = if health >= 80 {
        "sawa"
    } else if health >= 50 {
        "tahadhari"
    } else {
        "hatari"
    };

    json!({
        "ok": true,
        "health": health,
        "grade": grade,
        "checks": checks,
        "skipped": skipped,
        "note_sw": "Ulinzi wa mfumo huu PEKEE. Hakuna kugusa mfumo wa mtu mwingine.",
    })
}

fn list_ports(ports: &[u16]) -> String {
    ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
}

// ---------- 2) LAB PLAN: PEN-TESTING YA ULINZI (HITL) ----------
/// Mpango wa pen-testing wa LAB — DEFENSIVE. Unahitaji `scope.txt` (mifumo
/// YAKO mwenyewe) + idhini ya mmiliki. Unasaidia kuthibitisha ulinzi.
pub fn lab_plan(target: &str, authorized: bool) -> Value {
    let phases = json!([
        { "phase": 1, "jina": "Upelelezi wa ULINZI", "zana": ["nmap -sV (kwenye mfumo WAKO)"],
          "lengo": "Kuthibitisha bandari zipi zinasikiliza kwenye mifumo yako" },
        { "phase": 2, "jina": "Uthibitisho wa udhaifu", "zana": ["nuclei", "nikto (mfumo wako)"],
          "lengo": "Kugundua udhaifu kabla ya mshambuliaji" },
        { "phase": 3, "jina": "Kupima ulinzi wa uthibitisho", "zana": ["hydra (kwenye mfumo wako, funguo zako)"],
          "lengo": "Kuthibitisha sera za neno la siri" },
        { "phase": 4, "jina": "Ripoti + kurekebisha", "zana": ["reports.rs (kind=cyber)"],
          "lengo": "Ripoti yenye ushahidi (hash) + hatua za kurekebisha" },
    ]);
    json!({
        "ok": true,
        "lengo": target,
        "authorized": authorized,
        "hitl": true,
        "phases": phases,
        "kanuni_sw": "Pen-testing INAFANYIKA kwenye mifumo YAKO mwenyewe PEKEE, kwa idhini \
                      iliyoandikwa (scope.txt). Hii si zana ya kushambulia wengine.",
        "note_sw": if authorized {
            "Idhini imetolewa — unaweza kuendelea kwa kifaa chako/mfumo wako."
        } else {
            "HAIJARUHUSIWA — weka authorized=true THAMANI ikiwa unamiliki mfumo huu."
        },
    })
}

// ---------- 3) HARDENING PLAN ----------
/// Mpango wa kuimarisha mfumo. Haubadilishi kitu bila `apply = true` (HITL).
pub fn hardening_plan(apply: bool) -> Value {
    let steps = json!([
        { "id": "fw_policy", "jina": "nftables: policy DROP + ruhusu loopback/established" },
        { "id": "fw_ports", "jina": "nftables: funga bandari zote isipokuwa huduma zinazohitajika" },
        { "id": "ssh_hard", "jina": "SSH: PermitRootLogin no + PasswordAuthentication no" },
        { "id": "sysctl", "jina": "Sysctl: rp_filter, SYN cookies, ruka redirects" },
        { "id": "updates", "jina": "Sasisho za usalama (unattended-upgrades)" },
    ]);
    json!({
        "ok": true,
        "apply": apply,
        "steps": steps,
        "note_sw": if apply {
            "apply=true — hatua zitawekwa (bado zinahitaji uthibitisho wa mmiliki wa mfumo)."
        } else {
            "dry-run: hakuna kilichobadilika. Weka apply=true kutekeleza."
        },
    })
}

// ---------- 4) R ANALYTICS ----------
/// Tathmini data ya matukio (R: risk_score + anomaly + sec_forecast + fraud_trend).
/// Inatumia r_engine kwa kweli; ikiwa R haipo, inarudisha kwa uaminifu.
pub async fn r_analytics(events: &[f64], severities: &[f64]) -> Value {
    let mut out = json!({ "ok": true, "engine": "R" });

    if !events.is_empty() {
        let anomaly = crate::r_engine::run_req("anomaly", &json!({ "data": events })).await;
        out["anomaly"] = anomaly["stdout"].clone();
        let trend = crate::r_engine::run_req("fraud_trend", &json!({ "data": events })).await;
        out["trend"] = trend["stdout"].clone();
    }
    if !severities.is_empty() {
        let risk = crate::r_engine::run_req(
            "risk_score",
            &json!({ "data": severities, "max": 100 }),
        )
        .await;
        out["risk_score"] = risk["stdout"].clone();
    }
    out["note_sw"] = json!(
        "Takwimu za R (Rscript) — ikiwa 'stdout' ni tupu, R haijasakinishwa kwenye mfumo."
    );
    out
}

// ---------- 5) COVERAGE: hati → defensive engine ----------
/// Ramani ya kila mbinu/zana iliyotajwa kwenye hati tatu za "SECURITY CODE AGENT J"
/// → engine ya ULINZI inayoifanya kazi HALISI. Hii inaonyesha wazi ninacho-
/// linga (defense) na kile nilichokataa (mashambulizi — siyo halali).
pub fn coverage() -> Value {
    // (mbinu, aina, engine, path/module, hali)
    let rows = json!([
        // --- Ulaghai wa fedha za simu (hati J 2) ---
        { "mbinu": "SIM Swap / port-out", "aina": "defense", "engine": "m306 + threatintel",
          "sehemu": "tsmod/m306_sim_swap_detection.rs; threatintel sim_swap_attempt/telco", "hali": "halisi" },
        { "mbinu": "Ughushi wa callback ya malipo (M-Pesa)", "aina": "defense", "engine": "m517 + threatintel",
          "sehemu": "tsmod/m517_payment_callback_verify.rs; threatintel::verify_payment_callback", "hali": "halisi" },
        { "mbinu": "Replay ya CheckoutRequestID", "aina": "defense", "engine": "threatintel",
          "sehemu": "ti_callbacks leja (callback_seen/callback_record)", "hali": "halisi" },
        { "mbinu": "Phishing ya SMS / smishing", "aina": "defense", "engine": "m12, m289 + threatintel",
          "sehemu": "tsmod/m12_sms_phishing.rs; threatintel::score_message", "hali": "halisi" },
        { "mbinu": "Uhandisi wa kijamii (Halohalo/makeya)", "aina": "defense", "engine": "threatintel",
          "sehemu": "SCAM_TERMS (halohalo, makeya, niokoe...)", "hali": "halisi" },
        { "mbinu": "Caller-ID spoofing", "aina": "defense", "engine": "m2 + threatintel",
          "sehemu": "tsmod/m2_call_spoofing_detection.rs; threatintel::check_caller", "hali": "halisi" },
        { "mbinu": "Robocall / premium SMS", "aina": "defense", "engine": "m3, m16, m277",
          "sehemu": "tsmod/m3_call_blocking.rs; m16_premium_sms_blocking.rs", "hali": "halisi" },
        { "mbinu": "SS7/Diameter interceptions", "aina": "defense", "engine": "m284, m286",
          "sehemu": "tsmod/m284_mobile_network_call_security.rs", "hali": "halisi" },
        { "mbinu": "WhatsApp GhostPairing (device link)", "aina": "defense", "engine": "m307, m317",
          "sehemu": "tsmod/m307_device_fingerprinting.rs; m317_app_permission_control.rs", "hali": "halisi" },
        { "mbinu": "IDOR / Broken Access Control", "aina": "defense", "engine": "code_audit + threatintel",
          "sehemu": "code_audit.rs (rules); m18_sms_api_security.rs", "hali": "halisi" },
        { "mbinu": "DEBUG mode / uvujaji wa funguo", "aina": "defense", "engine": "code_audit + shield",
          "sehemu": "code_audit.rs secrets; shield.rs self-protection", "hali": "halisi" },
        { "mbinu": "Phishing kits / URL za uongo", "aina": "defense", "engine": "threatintel",
          "sehemu": "analyze_url (typosquat/homoglyph/TLD)", "hali": "halisi" },
        { "mbinu": "Phishing ya barua pepe / BEC", "aina": "defense", "engine": "m291, m299, m300",
          "sehemu": "tsmod/m300_email_bec_protection.rs", "hali": "halisi" },
        // --- Malware / spyware / rootkit (hati J, J 2, J 3) ---
        { "mbinu": "Spyware (Pegasus/Predator)", "aina": "defense", "engine": "m322, m317, m503",
          "sehemu": "tsmod/m322_antivirus_integration.rs; m503_iot_security.rs", "hali": "halisi" },
        { "mbinu": "Rootkit / process hiding", "aina": "defense", "engine": "m30, m28",
          "sehemu": "tsmod/m30_endpoint_behavior.rs; m28_behavioral_analysis.rs", "hali": "halisi" },
        { "mbinu": "Anti-forensics (kufuta logs)", "aina": "defense", "engine": "m303, m304, m472",
          "sehemu": "tsmod/m303_email_compliance.rs; m472_audit_blockchain.rs", "hali": "halisi" },
        { "mbinu": "Deepfakes (sauti/uso)", "aina": "defense", "engine": "m281, m512",
          "sehemu": "tsmod/m512_adversarial_ai.rs; m281_video_conference_security.rs", "hali": "halisi" },
        { "mbinu": "Adversarial AI / prompt injection", "aina": "defense", "engine": "shield + m512",
          "sehemu": "shield.rs guard(); m512_adversarial_ai.rs", "hali": "halisi" },
        // --- Mtandao / mfumo (hati J) ---
        { "mbinu": "Upelelezi wa mtandao (recon)", "aina": "defense", "engine": "m29, m35, m36",
          "sehemu": "tsmod/m35_firewall.rs; m36_ids.rs; m29_network_traffic_analysis.rs", "hali": "halisi" },
        { "mbinu": "Kuvunja firewall/IDS", "aina": "defense", "engine": "m35, m36, m511",
          "sehemu": "tsmod/m35_firewall.rs (harden); m511_moving_target_defense.rs", "hali": "halisi" },
        { "mbinu": "Kuvunja Active Directory", "aina": "defense", "engine": "m506, m315",
          "sehemu": "tsmod/m506_zero_trust.rs; m315_password_policy.rs", "hali": "halisi" },
        { "mbinu": "Supply chain (SBOM)", "aina": "defense", "engine": "m513",
          "sehemu": "tsmod/m513_supply_chain_sbom.rs", "hali": "halisi" },
        { "mbinu": "Metadata / ufuatiliaji wa eneo", "aina": "defense", "engine": "m308, m309",
          "sehemu": "tsmod/m308_location_tracking.rs; m309_geolocation_security.rs", "hali": "halisi" },
        { "mbinu": "Nywila za udhaifu (credential stuffing)", "aina": "defense", "engine": "m315, m316",
          "sehemu": "tsmod/m315_password_policy.rs; m316_screen_lock_security.rs", "hali": "halisi" },
        { "mbinu": "Zero Trust (ulinzi wa kina)", "aina": "defense", "engine": "m506",
          "sehemu": "tsmod/m506_zero_trust.rs", "hali": "halisi" },
        { "mbinu": "Honeypot / deception", "aina": "defense", "engine": "m510",
          "sehemu": "tsmod/m510_deception_honeypot.rs", "hali": "halisi" },
        // --- Pen-testing (HITL: mifumo YAKO) ---
        { "mbinu": "Pen-testing (lab, mfumo wako)", "aina": "pen-test (HITL)", "engine": "secpack + kali_groups",
          "sehemu": "secpack.rs (zana halisi); kali_groups.rs (katalogi); phoenix_shield::lab_plan", "hali": "halisi (HITL)" },
        { "mbinu": "Uchanganuzi wa code (USALAMA/BUGS)", "aina": "defense", "engine": "code_audit",
          "sehemu": "code_audit.rs", "hali": "halisi" },
        { "mbinu": "Takwimu/Forecast (R)", "aina": "defense", "engine": "r_engine",
          "sehemu": "risk_score, anomaly, sec_forecast, fraud_trend, domain_sim", "hali": "halisi (R)" },
        // --- Quantum / baada-ya-quantum (hati J, J 3) ---
        { "mbinu": "Finvu ya quantum (post-quantum crypto)", "aina": "defense", "engine": "m509 + m328",
          "sehemu": "tsmod/m509_future_proof.rs; m328_future_proof_architecture.rs (post_quantum)", "hali": "halisi" },
        { "mbinu": "QRNG (quantum random number generator)", "aina": "defense", "engine": "m515",
          "sehemu": "tsmod/m515_qrng.rs", "hali": "halisi" },
        { "mbinu": "Usimbaji wa quantum-safe kwa simu/blockchain", "aina": "defense", "engine": "m1, m273, m468",
          "sehemu": "tsmod/m1_voice_call_encryption.rs; m468_blockchain_core.rs", "hali": "halisi" },
        { "mbinu": "Homomorphic / ZK proofs / federated learning", "aina": "defense", "engine": "m509",
          "sehemu": "tsmod/m509_future_proof.rs (homomorphic, zero_knowledge, federated_learning, did)", "hali": "halisi" },
        { "mbinu": "Kuvunja usimbaji wa quantum (Shor/Grover)", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        // --- Zilizokataa ---
        { "mbinu": "SIM swap KUSHAMBULIA / kununua SIM za wengine", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        { "mbinu": "SS7 exploitation (Rodan/SWITCHSNIPER)", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        { "mbinu": "WhatsApp GhostPairing KUSHAMBULIA", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        { "mbinu": "Kughushi callback ya malipo", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        { "mbinu": "Pegasus/Predator kupeleleza", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        { "mbinu": "Rootkit / kufuta alama kushambulia", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
        { "mbinu": "Kufuatilia wanasiasa / deepfake za kuiba", "aina": "OFFENSIVE", "engine": "—",
          "sehemu": "HAIKUJENGWA", "hali": "imekataliwa (haramu)" },
    ]);
    let defense = rows.as_array().map(|a| a.iter().filter(|r| r["aina"] == "defense").count()).unwrap_or(0);
    let pentest = rows.as_array().map(|a| a.iter().filter(|r| r["aina"] == "pen-test (HITL)").count()).unwrap_or(0);
    let refused = rows.as_array().map(|a| a.iter().filter(|r| r["aina"] == "OFFENSIVE").count()).unwrap_or(0);
    json!({
        "ok": true,
        "defense_engines": defense,
        "pentest_hitl": pentest,
        "refused": refused,
        "coverage": rows,
        "kanuni_sw": "Kila mbinu ya hati imelingwa (defense) kwa engine halisi, au imekataliwa \
                      kama mashambulizi. Pen-testing inapatikana kwa mifumo YAKO pekee (HITL).",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posture_inarudisha_health_na_checks() {
        let v = posture();
        assert_eq!(v["ok"], serde_json::json!(true));
        assert!(v["health"].as_i64().unwrap() >= 0);
        let checks = v["checks"].as_array().unwrap();
        assert!(!checks.is_empty(), "kuna checks");
        // id za checks ni za kipekee
        let mut ids: Vec<&str> = checks.iter().filter_map(|c| c["id"].as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), checks.len());
    }

    #[test]
    fn bandari_za_tcp_ni_halali() {
        // Hii ni imani kwa mazingira: inarudisha Vec (inaweza kuwa tupu)
        let ports = listening_tcp_ports();
        for p in ports {
            assert!(p > 0);
        }
    }

    #[test]
    fn lab_plan_inahitaji_idhini() {
        let denied = lab_plan("10.0.0.5", false);
        assert_eq!(denied["hitl"], serde_json::json!(true));
        assert_eq!(denied["authorized"], serde_json::json!(false));
        assert!(denied["note_sw"].as_str().unwrap().contains("HAIJARUHUSIWA"));
        let ok = lab_plan("10.0.0.5", true);
        assert_eq!(ok["authorized"], serde_json::json!(true));
    }

    #[test]
    fn hardening_ni_dry_run_kwa_default() {
        let v = hardening_plan(false);
        assert_eq!(v["apply"], serde_json::json!(false));
        assert!(v["note_sw"].as_str().unwrap().contains("dry-run"));
        let a = hardening_plan(true);
        assert_eq!(a["apply"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn r_analytics_haipaniki_bila_data() {
        let v = r_analytics(&[], &[]).await;
        assert_eq!(v["ok"], serde_json::json!(true));
        assert_eq!(v["engine"], serde_json::json!("R"));
    }

    #[test]
    fn coverage_inagawanya_defense_na_offensive() {
        let v = coverage();
        assert_eq!(v["ok"], serde_json::json!(true));
        assert!(v["defense_engines"].as_u64().unwrap() >= 25, "defense engines nyingi");
        assert!(v["refused"].as_u64().unwrap() >= 5, "offensive zimekataliwa");
        let rows = v["coverage"].as_array().unwrap();
        // kila defense row iwe na engine isiyo "—"
        for r in rows.iter().filter(|r| r["aina"] == "defense") {
            assert_ne!(r["engine"], serde_json::json!("—"), "{}", r["mbinu"]);
        }
        // kila offensive row iwe HAIKUJENGWA
        for r in rows.iter().filter(|r| r["aina"] == "OFFENSIVE") {
            assert_eq!(r["sehemu"], serde_json::json!("HAIKUJENGWA"));
        }
    }
}
