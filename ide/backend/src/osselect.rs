//! OS selector yenye nguvu — data-driven (`/data/os_profiles.json`)
//!
//! Mtiririko (kila hatua ina sababu, hakuna chaguo la bubu):
//!   1. SKIP guard  — SMART/RAM fail → "skip" (msimamizi arekebishe hardware)
//!   2. Rules       — spec zingine → OS yenye score juu zaidi
//!   3. Ollama      — (hiari) inapendekeza; lazima ipitie validation ya profiles
//!   4. Output      — os + confidence + reasons (auditable)
//!
//! Kanuni ya mtaalamu: LLM haiamuzi pekee — inapendekeza; profiles JSON ndizo kanuni.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const OS_CHOICES: [&str; 7] = [
    "win11", "win10", "ubuntu", "debian", "fedora", "kali", "skip",
];

#[derive(Debug, Clone, Deserialize)]
pub struct OsProfile {
    pub id: String,
    pub min_ram_gb: f64,
    pub min_cpu_cores: u32,
    pub min_disk_gb: f64,
    pub tpm_required: Option<bool>,
    pub secure_boot: Option<bool>,
    pub license_cost: Option<String>,
    pub weights: BTreeMap<String, f64>,
    pub best_for: Vec<String>,
    pub notes_sw: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsCatalog {
    #[serde(default)]
    pub profiles: Vec<OsProfile>,
    #[serde(default)]
    pub skip_rules: Vec<String>,
    #[serde(default)]
    pub parse_patterns: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OsDecision {
    pub os: String,
    pub confidence: f64,
    pub method: String, // rules | ollama_assisted | skip_guard
    pub reasons: Vec<String>,
    pub scores: BTreeMap<String, f64>,
}

impl OsCatalog {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let txt = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Haiwezi kusoma {path}: {e}"))?;
        Ok(serde_json::from_str(&txt)?)
    }
}

/// Nukuu za specs → ram_gb, cores, disk_gb (regex huru ya mtiririko wa herufi)
pub fn parse_specs(specs: &str) -> (f64, u32, f64) {
    let s = specs.to_lowercase();
    let ram = grab_num(&s, &["gb ram", "ram gb", "ram", "gig ram"]).unwrap_or(0.0);
    let cores = grab_num(&s, &["core", "cores", "cpu"]).unwrap_or(0.0) as u32;
    let disk = grab_num(&s, &["gb ssd", "gb hdd", "ssd", "hdd", "disk"]).unwrap_or(0.0);
    (ram, cores, disk)
}

fn grab_num(s: &str, keys: &[&str]) -> Option<f64> {
    for k in keys {
        if let Some(i) = s.find(k) {
            let before = &s[..i];
            let after = &s[i + k.len()..];
            // namba inayotangulia au kufuata neno la ufunguo
            for chunk in [tail_num(before), head_num(after)] {
                if let Some(v) = chunk {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn tail_num(s: &str) -> Option<f64> {
    let digits: String = s
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse().ok()
}

fn head_num(s: &str) -> Option<f64> {
    let digits: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
}

/// Chaguo la mwisho (lazy-static-like kwa njia rahisi): catalog kutoka data/
pub fn catalog() -> OsCatalog {
    if let Ok(p) = std::env::var("FUNDI_OS_PROFILES") {
        if let Ok(c) = OsCatalog::load(&p) {
            return c;
        }
    }
    for p in [
        "../../data/os_profiles.json", // repo root — cwd = server/agent (cargo test)
        "../../../data/os_profiles.json",
        "../data/os_profiles.json",
        "data/os_profiles.json", // cwd = /app kwenye container (COPY data /app/data)
        "/os_profiles.json", // docker mount
        "/data/os_profiles.json",
    ] {
        if let Ok(c) = OsCatalog::load(p) {
            return c;
        }
    }
    // Fallback ndogo ndipo data ipo
    OsCatalog::default()
}

/// Kanuni kuu (sync, bila AI): score kila profile dhidi ya specs + mahitaji
pub fn decide(specs: &str, need: &str) -> OsDecision {
    decide_with(&catalog(), specs, need)
}

/// Kamili (async): rules + Ollama assist (lazima ipitie validation ya profiles)
pub async fn decide_full(specs: &str, need: &str) -> OsDecision {
    decide_full_with(&catalog(), specs, need).await
}

pub fn decide_with(cat: &OsCatalog, specs: &str, need: &str) -> OsDecision {
    let s = format!("{specs} {need}").to_lowercase();
    let (ram, cores, disk) = parse_specs(specs);

    // 1. SKIP guard (data-driven kutoka skip_rules)
    for rule in &cat.skip_rules {
        if s.contains(rule) {
            return OsDecision {
                os: "skip".into(),
                confidence: 0.95,
                method: "skip_guard".into(),
                reasons: vec![format!(
                    "SKIP rule iliyolingana: '{rule}' — hardware itarekebishwa kwanza"
                )],
                scores: BTreeMap::new(),
            };
        }
    }

    // 2. Score kila profile
    let mut scores: BTreeMap<String, f64> = BTreeMap::new();
    let mut reasons_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in &cat.profiles {
        let mut score = 0.0;
        let mut why: Vec<String> = Vec::new();
        let pw = parse_patterns_apply(&cat.parse_patterns, &s);

        for (kw, w) in &p.weights {
            let k = kw.to_lowercase();
            if (need.to_lowercase().contains(&k) || s.contains(&k)) && *w > 0.0 {
                score += w;
                why.push(format!("+{w} unakidhi '{kw}'"));
            }
        }
        // Kikomo cha hardware
        if ram >= p.min_ram_gb && ram > 0.0 {
            score += 2.0;
            why.push(format!("+2 RAM {ram}GB ≥ {}GB", p.min_ram_gb));
        } else if ram > 0.0 {
            score -= (p.min_ram_gb - ram).clamp(0.0, 6.0) * 3.0;
            why.push(format!(
                "-{} RAM ndogo ({ram}GB < {}GB)",
                ((p.min_ram_gb - ram) * 3.0).round() / 3.0,
                p.min_ram_gb
            ));
        }
        if cores >= p.min_cpu_cores && cores > 0 {
            score += 1.0;
            why.push(format!("+1 cores {cores} ≥ {}", p.min_cpu_cores));
        }
        if disk >= p.min_disk_gb && disk > 0.0 {
            score += 1.0;
            why.push(format!("+1 disk {disk}GB ≥ {}GB", p.min_disk_gb));
        }
        score += pw * 0.5; // pattern bonus ndogo
        scores.insert(p.id.clone(), score);
        reasons_map.insert(p.id.clone(), why);
    }

    let mut ranked: Vec<(String, f64)> = scores.iter().map(|(k, v)| (k.clone(), *v)).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (top_os, top_score) = ranked.first().cloned().unwrap_or(("win10".into(), 0.0));
    let reasons = reasons_map.get(&top_os).cloned().unwrap_or_default();

    let total: f64 = scores.values().sum();
    let confidence = if total > 0.0 {
        ((top_score / total.max(1.0)) * 2.0).clamp(0.3, 0.99)
    } else {
        0.5
    };

    OsDecision {
        os: top_os,
        confidence,
        method: "rules".into(),
        reasons,
        scores,
    }
}

/// Rules + Ollama assist — Ollama inapendekeza; profiles ndizo kanuni
pub async fn decide_full_with(cat: &OsCatalog, specs: &str, need: &str) -> OsDecision {
    let mut d = decide_with(cat, specs, need);
    if let Ok(oll) = ollama_suggest(specs, need).await {
        if OS_CHOICES.contains(&oll.as_str()) && oll != "skip" {
            if let Some(oscore) = d.scores.get(&oll) {
                if *oscore >= 0.0 {
                    if oll != d.os {
                        d.reasons.push(format!(
                            "Ollama ilipendekeza '{oll}' (score {oscore:.1}); rules zilikuwa '{}'",
                            d.os
                        ));
                    } else {
                        d.reasons
                            .push("Ollama ilithibitisha chaguo la rules".into());
                    }
                    d.os = oll;
                    d.method = "ollama_assisted".into();
                }
            }
        }
    }
    d
}

fn parse_patterns_apply(patterns: &BTreeMap<String, f64>, s: &str) -> f64 {
    let mut bonus = 0.0;
    for (pat, w) in patterns {
        if s.contains(pat) {
            bonus += w;
        }
    }
    bonus
}

async fn ollama_suggest(specs: &str, need: &str) -> anyhow::Result<String> {
    #[derive(serde::Serialize)]
    struct GenReq {
        model: String,
        prompt: String,
        stream: bool,
    }
    #[derive(serde::Deserialize)]
    struct GenResp {
        response: Option<String>,
    }
    let prompt = format!(
        "Wewe ni mtaalamu wa IT. Chagua OS moja tu: win11, win10, ubuntu, debian, fedora, kali.\nSpecs:\n{specs}\nMahitaji: {need}\nJibu: jina la OS pekee."
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp: GenResp = client
        .post("http://127.0.0.1:11434/api/generate")
        .json(&GenReq {
            model: std::env::var("FUNDI_AI_MODEL").unwrap_or_else(|_| "tinyllama".into()),
            prompt,
            stream: false,
        })
        .send()
        .await?
        .json()
        .await?;
    let text = resp.response.unwrap_or_default().to_lowercase();
    for v in OS_CHOICES {
        if v == "skip" {
            continue;
        }
        if text.contains(v) {
            return Ok(v.to_string());
        }
    }
    anyhow::bail!("no valid os")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_guard_hurunisha() {
        let d = decide_with(&catalog(), "RAM error SMART fail 8gb", "office");
        assert_eq!(d.os, "skip");
    }

    #[test]
    fn developer_pata_linux() {
        let d = decide_with(
            &catalog(),
            "16gb ram 8 cores 512 ssd",
            "developer docker linux",
        );
        assert!(
            ["ubuntu", "debian", "fedora"].contains(&d.os.as_str()),
            "got {}",
            d.os
        );
        assert!(d.confidence > 0.3);
    }

    #[test]
    fn office_pata_windows() {
        let d = decide_with(&catalog(), "8gb ram 4 cores 256 ssd", "office windows");
        assert!(["win10", "win11"].contains(&d.os.as_str()), "got {}", d.os);
    }

    #[test]
    fn reasons_zina_maudhui() {
        let d = decide_with(&catalog(), "16gb ram", "office");
        assert!(!d.reasons.is_empty());
    }
}
