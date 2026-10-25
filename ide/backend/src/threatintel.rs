//! threatintel.rs — THREAT INTELLIGENCE CENTER (defensive, consent-based).
//!
//! Utekelezaji HALISI wa sehemu za ULINZI za "TOTAL SECURITY V4" kama
//! APPLICATION ya server (si plugin kwenye SIM): uchanganuzi wa SMS/ujumbe wa
//! ulaghai, uchanganuzi wa URL (typosquatting/homoglyph/TLD), ugunduzi wa
//! Caller-ID spoofing, usimamizi wa blacklist + scam patterns, risk scoring,
//! na ripoti za admin. Kazi zote ni DEFENSIVE: data inachanganuliwa ikiwa
//! imetumwa na akaunti (consent), hakuna kuingilia mawasiliano ya wengine.
//!
//! Kanuni (kama secops.rs): API hurudisha majina SALAMA ya Kiswahili; DB ni
//! SQLite; kila kazi nzito ni HITL pale inapohitajika (kuzuia namba).
//!
//! R inaunganishwa kwa analytics ya kina kupitia `r_engine` (`risk_score`,
//! `anomaly`, `sec_forecast`, `domain_sim`).

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

// ---------- VIZINGITI ----------
pub const DANGER: i64 = 70;
pub const WARNING: i64 = 40;

/// Daraja la hatari kutoka score (0-100).
pub fn verdict_of(score: i64) -> &'static str {
    if score >= DANGER {
        "hatari"
    } else if score >= WARNING {
        "tahadhari"
    } else {
        "salama"
    }
}

// ---------- SIGNAL ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Signal {
    pub id: String,
    pub weight: i64,
    pub detail: String,
}

fn sig(id: &str, weight: i64, detail: impl Into<String>) -> Signal {
    Signal {
        id: id.into(),
        weight,
        detail: detail.into(),
    }
}

fn sum_weights(signals: &[Signal]) -> i64 {
    signals.iter().map(|s| s.weight).sum::<i64>().clamp(0, 100)
}

// ---------- UCHANGANUZI WA UJUMBE (SMS/scam/phishing) ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageAssessment {
    pub score: i64,
    pub verdict: String,
    pub signals: Vec<Signal>,
}

/// Maneno ya hatari (Kiswahili + Kiingereza) yenye uzito. Defensive heuristics
/// zinazoonekana kwa uwazi — si "black box".
const SCAM_TERMS: &[(&str, i64)] = &[
    ("umeshinda", 25),
    ("umepata zawadi", 25),
    ("umebahatika", 22),
    ("hongera", 15),
    ("tuma pesa", 30),
    ("tuma tsh", 30),
    ("tuma kiasi", 28),
    ("lipa", 12),
    ("kutuma", 10),
    ("akaunti yako imefungwa", 28),
    ("imefungwa", 18),
    ("thibitisha", 15),
    ("verify", 15),
    ("otp", 30),
    ("namba ya siri", 30),
    ("pin", 25),
    ("password", 20),
    ("kitambulisho", 10),
    ("nida", 12),
    ("mkopo", 12),
    ("loan", 12),
    ("faida", 10),
    ("bure", 8),
    ("haraka", 12),
    ("sasa hivi", 8),
    ("bofya", 18),
    ("click here", 18),
    ("click", 12),
    ("link", 10),
    ("tuzo", 18),
    ("bahati", 12),
    ("kila siku", 6),
    ("mteja", 6),
    ("benki", 12),
    ("bank", 10),
    ("mpesa", 10),
    ("m-pesa", 10),
    ("pesa", 8),
    ("shilingi", 8),
    ("taarifa yako", 12),
    ("cancel", 8),
    ("unsubscribe", 6),
    ("delivery", 8),
    ("package", 8),
    ("parcel", 10),
    // --- Ulaghai wa SIM-swap / port-out (hati "SECURITY CODE AGENT J 2") ---
    ("sim swap", 35),
    ("kubadilisha sim", 35),
    ("kubadilisha laini", 30),
    ("kubadilisha namba", 30),
    ("kuhamisha laini", 30),
    ("kuhamisha namba", 30),
    ("uhamisho wa laini", 30),
    ("mabadiliko ya sim", 30),
    ("badilishwa", 28),
    ("port out", 35),
    ("port-out", 35),
    ("imehamishwa", 25),
    ("usajili wa sim", 20),
    ("sim registration", 20),
    // --- Uhandisi wa kijamii: Halohalo / makeya / ndugu (hati J 2) ---
    ("halohalo", 30),
    ("halo halo", 30),
    ("makeya", 25),
    ("mjomba", 22),
    ("nimepata ajali", 25),
    ("nimekamatwa", 22),
    ("niokoe", 22),
    ("hospitalini", 18),
    ("nisaidie", 12),
    ("nitakulipa", 15),
    ("usimwambie mtu", 20),
    ("usiseme mtu", 20),
    // --- Uthibitisho/OTP (nyongeza ya kimataifa) ---
    ("verification code", 25),
    ("code ya uthibitisho", 25),
    ("one time password", 30),
];

/// Viungo/URL zilizofichwa ndani ya ujumbe.
fn extract_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in text.split_whitespace() {
        let t = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '/' && c != '.' && c != ':' && c != '-' && c != '?' && c != '=' && c != '&' && c != '%' && c != '_' && c != '~' && c != '#');
        let low = t.to_lowercase();
        if low.starts_with("http://") || low.starts_with("https://") || low.starts_with("www.") {
            out.push(low);
        }
    }
    out
}

/// Namba ya simu iliyopo ndani ya ujumbe (>= 9 tarakimu mfululizo au +255...).
fn contains_phone(text: &str) -> bool {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    // mlolongo wa tarakimu 9+ ndani ya token moja
    let mut run = 0usize;
    for c in text.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= 9 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    let _ = digits;
    false
}

/// Alama za brand zinazotajwa (impersonation).
const BRANDS: &[&str] = &[
    "crdb", "nbc", "nmb", "absa", "stanbic", "exim", "mpesa", "m-pesa", "tigo",
    "vodacom", "airtel", "halotel", "ttcl", "tcra", "nida", "tra", "voda", "paypal",
    "whatsapp", "instagram", "facebook", "netflix", "amazon", "google",
];

fn brands_mentioned(low: &str) -> Vec<String> {
    BRANDS
        .iter()
        .filter(|b| low.contains(*b))
        .map(|b| b.to_string())
        .collect()
}

/// Uchanganuzi kamili wa ujumbe (SMS/scam). Heuristics za uwazi + uzito.
pub fn score_message(text: &str) -> MessageAssessment {
    let low = text.to_lowercase();
    let mut signals: Vec<Signal> = Vec::new();

    for (term, w) in SCAM_TERMS {
        if low.contains(term) {
            signals.push(sig("scam_term", *w, format!("Neno la hatari: '{term}'")));
        }
    }

    let urls = extract_urls(text);
    if !urls.is_empty() {
        signals.push(sig("has_link", 15, format!("Ujumbe una kiungo ({})", urls.len())));
        for u in &urls {
            let ua = analyze_url(u);
            if ua.score >= WARNING {
                signals.push(sig(
                    "bad_link",
                    ua.score / 2,
                    format!("Kiungo hatari: {}", ua.reasons.join("; ")),
                ));
            }
        }
    }

    if contains_phone(text) {
        signals.push(sig("has_phone", 8, "Ujumbe una namba ya simu"));
    }

    let brands = brands_mentioned(&low);
    if !brands.is_empty() {
        signals.push(sig(
            "brand_mention",
            12,
            format!("Inataja taasisi: {}", brands.join(", ")),
        ));
    }

    // Mchanganyiko hatari: brand + ombi la pesa/OTP
    let money = low.contains("tuma") || low.contains("lipa") || low.contains("send money");
    let otp = low.contains("otp") || low.contains("pin") || low.contains("namba ya siri");
    if !brands.is_empty() && (money || otp) {
        signals.push(sig(
            "brand_plus_request",
            25,
            "Inataja taasisi na inaomba pesa/OTP (mtindo wa ulaghai)",
        ));
    }
    if otp {
        signals.push(sig(
            "otp_request",
            20,
            "Taasisi halali HAWAOMBI OTP/PIN kwa ujumbe",
        ));
    }

    // Mchanganyiko wa SIM-swap/port-out: telco + mabadiliko ya laini (+ OTP)
    let sim_change = low.contains("sim swap")
        || low.contains("port out")
        || low.contains("port-out")
        || low.contains("kubadilisha sim")
        || low.contains("kubadilisha laini")
        || low.contains("kuhamisha laini")
        || low.contains("kuhamisha namba")
        || low.contains("uhamisho wa laini")
        || low.contains("imehamishwa")
        || ((low.contains("badilishwa") || low.contains("inabadilishwa"))
            && (low.contains("sim") || low.contains("laini") || low.contains("namba")));
    let telco = brands
        .iter()
        .any(|b| matches!(b.as_str(), "vodacom" | "tigo" | "airtel" | "halotel" | "ttcl" | "mpesa" | "m-pesa"));
    if sim_change {
        signals.push(sig(
            "sim_swap_attempt",
            35,
            "Inataja mabadiliko/port-out ya SIM — mtindo wa SIM-swap",
        ));
    }
    if sim_change && telco {
        signals.push(sig(
            "sim_swap_telco",
            40,
            "Mabadiliko ya SIM yanayotajwa na mtandao — hatari ya kuchukua laini",
        ));
    }

    let score = sum_weights(&signals);
    MessageAssessment {
        score,
        verdict: verdict_of(score).to_string(),
        signals,
    }
}

// ---------- UCHANGANUZI WA URL ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UrlAssessment {
    pub host: String,
    pub score: i64,
    pub verdict: String,
    pub brand: Option<String>,
    pub reasons: Vec<String>,
}

const SHORTENERS: &[&str] = &[
    "bit.ly", "tinyurl.com", "t.co", "goo.gl", "is.gd", "cutt.ly", "rb.gy",
    "ow.ly", "shorturl.at", "t.ly", "bit.do",
];

const BAD_TLDS: &[&str] = &[
    "zip", "mov", "xyz", "top", "tk", "ml", "ga", "cf", "gq", "click", "link",
    "work", "rest", "cam", "sbs", "lol", "icu",
];

/// Brand → domains halali (suffix). URL inayoishia hizi ni salama.
const OFFICIAL: &[(&str, &[&str])] = &[
    ("crdb", &["crdb.co.tz", "crdbbank.co.tz"]),
    ("nbc", &["nbc.co.tz", "nbctz.com"]),
    ("nmb", &["nmbbank.co.tz"]),
    ("absa", &["absa.co.tz", "absafrica.com"]),
    ("stanbic", &["stanbicbank.co.tz"]),
    ("exim", &["eximbank.co.tz"]),
    ("mpesa", &["mpesa.co.tz", "vodacom.co.tz"]),
    ("m-pesa", &["mpesa.co.tz", "vodacom.co.tz"]),
    ("tigo", &["tigo.co.tz"]),
    ("vodacom", &["vodacom.co.tz"]),
    ("airtel", &["airtel.co.tz"]),
    ("halotel", &["halotel.co.tz"]),
    ("ttcl", &["ttcl.co.tz"]),
    ("paypal", &["paypal.com"]),
    ("whatsapp", &["whatsapp.com"]),
    ("facebook", &["facebook.com", "fb.com"]),
    ("instagram", &["instagram.com"]),
    ("netflix", &["netflix.com"]),
    ("amazon", &["amazon.com"]),
    ("google", &["google.com"]),
];

/// Toa host kutoka URL (bila crate ya ziada).
pub fn host_of(url: &str) -> String {
    let mut s = url.trim().to_lowercase();
    for p in ["https://", "http://", "ftp://", "//"] {
        if let Some(rest) = s.strip_prefix(p) {
            s = rest.to_string();
            break;
        }
    }
    if let Some(at) = s.rfind('@') {
        s = s[at + 1..].to_string();
    }
    let end = s.find(['/', '?', '#']).unwrap_or(s.len());
    s.truncate(end);
    // port (IPv6 imepuuzwa — hati hii inalenga host za IPv4/domain)
    if let Some(colon) = s.find(':') {
        if !s[..colon].contains(']') {
            s.truncate(colon);
        }
    }
    s
}

fn registrable_label(host: &str) -> String {
    let h = host.strip_prefix("www.").unwrap_or(host);
    h.split('.').next().unwrap_or(h).to_string()
}

fn is_ipv4_literal(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| !p.is_empty() && p.parse::<u8>().is_ok())
}

fn normalize_homoglyphs(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => 'o',
            '1' => 'l',
            '3' => 'e',
            '4' => 'a',
            '5' => 's',
            '7' => 't',
            '$' => 's',
            '2' => 'z',
            '8' => 'b',
            '9' => 'g',
            _ => c,
        })
        .collect()
}

/// Levenshtein (edit distance) — DP ya kawaida, O(n*m).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

/// Uchanganuzi wa URL: IP-literal, punycode, shortener, TLD hatari,
/// na typosquatting dhidi ya brand zilizolindwa.
pub fn analyze_url(url: &str) -> UrlAssessment {
    let host = host_of(url);
    let mut reasons: Vec<String> = Vec::new();
    let mut score: i64 = 0;
    let mut brand: Option<String> = None;

    if host.is_empty() {
        return UrlAssessment {
            host,
            score: 0,
            verdict: "salama".into(),
            brand: None,
            reasons: vec!["host tupu".into()],
        };
    }

    // 1) IP literal
    if is_ipv4_literal(&host) {
        score += 30;
        reasons.push("URL inatumia anwani ya IP moja kwa moja".into());
    }

    // 2) punycode / homoglyph
    if host.contains("xn--") {
        score += 30;
        reasons.push("Jina la kikoa la punycode (homoglyph/IDN) — linaweza kufanana na brand".into());
    }

    // 3) shortener
    let reg = format!("{}.{}", registrable_label(&host), host.rsplit_once('.').map(|x| x.1).unwrap_or(""));
    if SHORTENERS.iter().any(|s| host == *s || host.ends_with(&format!(".{s}"))) || SHORTENERS.contains(&reg.as_str()) {
        score += 20;
        reasons.push("Kiungo kifupi (shortener) — huvalia lengo halisi".into());
    }

    // 4) TLD hatari
    if let Some(tld) = host.rsplit_once('.').map(|x| x.1) {
        if BAD_TLDS.contains(&tld) {
            score += 15;
            reasons.push(format!("Kikoa cha juu cha hatari (.{tld})"));
        }
    }

    // 5) typosquatting dhidi ya brand zilizolindwa
    let label = registrable_label(&host);
    let label_norm = normalize_homoglyphs(&label);
    for (b, official) in OFFICIAL {
        // official exact?
        let is_official = official.iter().any(|d| host == *d || host.ends_with(&format!(".{d}")));
        if is_official {
            break;
        }
        let bnorm = normalize_homoglyphs(b);
        let d = levenshtein(&label_norm, &bnorm);
        let contains = label_norm.contains(&bnorm) && label_norm != bnorm;
        let hyphen_variant = label.contains(b) && label != *b;
        if (d > 0 && d <= 2) || contains || hyphen_variant {
            score += 45;
            brand = Some((*b).to_string());
            reasons.push(format!(
                "Inafanana na '{b}' lakini si kikoa halali ({}) — typosquatting",
                official.join(", ")
            ));
            break;
        }
    }

    let score = score.clamp(0, 100);
    UrlAssessment {
        host,
        score,
        verdict: verdict_of(score).to_string(),
        brand,
        reasons,
    }
}

// ---------- UGUNDUZI WA CALLER SPOOFING ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallAssessment {
    pub spoofed: bool,
    pub score: i64,
    pub verdict: String,
    pub reasons: Vec<String>,
}

/// Safisha namba (weka + na tarakimu; geuza 0-prefix kuwa +255).
pub fn normalize_phone(p: &str) -> String {
    let has_plus = p.trim_start().starts_with('+');
    let digits: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut d = digits;
    if d.starts_with("255") {
        // tayari kimataifa
    } else if d.starts_with('0') {
        d = format!("255{}", d.trim_start_matches('0'));
    }
    if has_plus || !d.is_empty() {
        format!("+{d}")
    } else {
        String::new()
    }
}

/// Linganisha Caller-ID iliyoonyeshwa na chanzo halisi (kutoka MSC/operator).
/// Spoofing = haziendani. Defensive: data hii hutoka kwa operator aliyeidhinishwa.
pub fn check_caller(presented: &str, actual: &str) -> CallAssessment {
    let p = normalize_phone(presented);
    let a = normalize_phone(actual);
    let mut reasons = Vec::new();
    let mut spoofed = false;
    let mut score = 0i64;

    if p.is_empty() || a.is_empty() {
        return CallAssessment {
            spoofed: false,
            score: 0,
            verdict: "salama".into(),
            reasons: vec!["data haitoshi kuhukumu".into()],
        };
    }
    if p != a {
        spoofed = true;
        score += 70;
        reasons.push(format!("Caller-ID ({p}) HAIFANANI na chanzo halisi ({a})"));
    }
    if p.len() < 10 || a.len() < 10 {
        score += 10;
        reasons.push("Namba fupi au isiyo ya kawaida".into());
    }
    let score = score.clamp(0, 100);
    CallAssessment {
        spoofed,
        score,
        verdict: verdict_of(score).to_string(),
        reasons,
    }
}

// ---------- UTHIBITISHO WA CALLBACK ZA MALIPO (defensive) ----------
// Hati "SECURITY CODE AGENT J 2" inaeleza jinsi callback za M-Pesa zinavyoweza
// kughushiwa (kutuma callback bandia kabla ya malipo halisi) na CheckoutRequestID
// kutumika tena. Hii NI ULINZI: hutathmini callback inayoingia kabla ya kuamini
// malipo. Hailingii mfumo wa mtu mwingine — inalinda mfumo WAKO.

/// Matokeo ya uthibitisho wa callback ya malipo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallbackAssessment {
    pub trusted: bool,
    pub score: i64,
    pub verdict: String,
    pub reasons: Vec<String>,
}

/// Data ya callback inayoingia (inaweza kuwa ghushi). Timestamps ni epoch seconds.
#[derive(Debug, Clone, Default)]
pub struct CallbackInput {
    pub checkout_request_id: String,
    pub amount: f64,
    pub expected_amount: f64,
    /// Saa ambapo ombi HALISI la malipo liliundwa (kutoka leja yako).
    pub request_created_at: i64,
    /// Saa ya callback yenyewe.
    pub callback_at: i64,
    pub signature_ok: bool,
    pub ip_whitelisted: bool,
    /// CheckoutRequestID hii ilishatumika/imeshalipwa awali.
    pub already_settled: bool,
}

/// Tathmini callback ya malipo. `trusted` = kweli TU ikiwa hakuna alama.
/// Defensive: inaongeza score kwa kila hatari inayogundulika.
pub fn verify_payment_callback(c: &CallbackInput) -> CallbackAssessment {
    let mut reasons: Vec<String> = Vec::new();
    let mut score: i64 = 0;

    if c.checkout_request_id.trim().is_empty() {
        score += 40;
        reasons.push("CheckoutRequestID haipo".into());
    }
    if !c.signature_ok {
        score += 45;
        reasons.push("Sahihi (HMAC) ya callback HAITHIBITIKI".into());
    }
    if !c.ip_whitelisted {
        score += 25;
        reasons.push("Callback imetoka IP isiyo kwenye whitelist ya telco".into());
    }
    if c.already_settled {
        score += 40;
        reasons.push("CheckoutRequestID ilishalipwa/imetumika tena (replay)".into());
    }
    if c.request_created_at > 0 && c.callback_at > 0 && c.callback_at < c.request_created_at {
        score += 50;
        reasons.push("Callback ilifika KABLA ya ombi halisi (sequence ya ughushi)".into());
    }
    // Freshness: callback ya zamani sana au ya mbeleni isiyowezekana
    if c.request_created_at > 0 && c.callback_at > 0 {
        let age = c.callback_at - c.request_created_at;
        if age > 3600 {
            score += 20;
            reasons.push("Callback imechelewa zaidi ya saa 1 (freshness)".into());
        }
    }
    // Kiasi kisicholingana
    if c.expected_amount > 0.0 && (c.amount - c.expected_amount).abs() > 0.01 {
        score += 35;
        reasons.push(format!(
            "Kiasi ({:.2}) HAILINGANI na ombi halisi ({:.2})",
            c.amount, c.expected_amount
        ));
    }

    let score = score.clamp(0, 100);
    CallbackAssessment {
        trusted: score < WARNING,
        score,
        verdict: verdict_of(score).to_string(),
        reasons,
    }
}

// ---------- THREATSWARM: AGENTS ZA ULINZI ----------
/// Agents za ThreatSwarm zinazohusiana na ULINZI (defensive) — hizi ndizo
/// zinaunganishwa na Threat Intelligence. Kazi zao (secpack/Kali) zinaendeshwa
/// kwa lengo lililo kwenye `scope.txt` (HITL) kupitia `threatswarm::run_agent`.
pub const DEFENSIVE_AGENTS: &[&str] = &[
    "blue-team",
    "dfir",
    "threat-hunter",
    "log-analyst",
    "compliance-scanner",
    "malware-analyst",
    "vuln-researcher",
    "report-writer",
];

/// Katalogi ya agents za ulinzi (kutoka ThreatSwarm).
pub fn defensive_agents() -> serde_json::Value {
    let entries = crate::threatswarm::agent_entries();
    let items: Vec<serde_json::Value> = entries
        .iter()
        .filter(|(a, _)| DEFENSIVE_AGENTS.contains(a))
        .map(|(a, t)| json!({ "agent": a, "tasks": t, "task_count": t.len() }))
        .collect();
    json!({
        "ok": true,
        "agents": items,
        "count": items.len(),
        "note_sw": "Agents za ULINZI kutoka ThreatSwarm — zinaendeshwa kwa lengo lililo kwenye scope.txt (HITL).",
    })
}

// ---------- DB ----------
pub async fn init_tables(db: &SqlitePool) {
    for ddl in [
        r#"CREATE TABLE IF NOT EXISTS ti_blacklist (
            number TEXT PRIMARY KEY,
            reason TEXT,
            added_by TEXT,
            created_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ti_patterns (
            pattern TEXT PRIMARY KEY,
            description TEXT,
            created_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ti_reports (
            id TEXT PRIMARY KEY,
            at TEXT,
            account TEXT,
            kind TEXT,
            target TEXT,
            score INTEGER,
            verdict TEXT,
            signals TEXT,
            note TEXT
        )"#,
        // Leja ya callback za malipo zilizoshughulikiwa — huzuia replay
        // (CheckoutRequestID moja kulipwa/kuthibitishwa mara mbili).
        r#"CREATE TABLE IF NOT EXISTS ti_callbacks (
            checkout_request_id TEXT PRIMARY KEY,
            first_seen TEXT,
            trusted INTEGER,
            score INTEGER
        )"#,
    ] {
        let _ = sqlx::query(ddl).execute(db).await;
    }
}

/// Je, CheckoutRequestID hii ilishashughulikiwa awali? (replay detection)
pub async fn callback_seen(db: &SqlitePool, checkout_request_id: &str) -> bool {
    if checkout_request_id.trim().is_empty() {
        return false;
    }
    let row: Option<(String,)> =
        sqlx::query_as("SELECT checkout_request_id FROM ti_callbacks WHERE checkout_request_id=?")
            .bind(checkout_request_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
    row.is_some()
}

/// Rekodi callback iliyoshughulikiwa (idempotent — ya kwanza inabaki).
pub async fn callback_record(
    db: &SqlitePool,
    checkout_request_id: &str,
    trusted: bool,
    score: i64,
) -> Result<(), String> {
    if checkout_request_id.trim().is_empty() {
        return Ok(());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO ti_callbacks (checkout_request_id, first_seen, trusted, score) VALUES (?,?,?,?)",
    )
    .bind(checkout_request_id)
    .bind(&now)
    .bind(if trusted { 1i64 } else { 0i64 })
    .bind(score)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Ongeza namba kwenye blacklist (idempotent).
pub async fn blacklist_add(
    db: &SqlitePool,
    number: &str,
    reason: &str,
    added_by: &str,
) -> Result<serde_json::Value, String> {
    let n = normalize_phone(number);
    if n.is_empty() {
        return Err("namba tupu".into());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query("INSERT OR REPLACE INTO ti_blacklist (number, reason, added_by, created_at) VALUES (?,?,?,?)")
        .bind(&n)
        .bind(reason)
        .bind(added_by)
        .bind(&now)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "number": n, "added": true }))
}

/// Toa namba kwenye blacklist.
pub async fn blacklist_remove(db: &SqlitePool, number: &str) -> Result<serde_json::Value, String> {
    let n = normalize_phone(number);
    let res = sqlx::query("DELETE FROM ti_blacklist WHERE number=?")
        .bind(&n)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "number": n, "removed": res.rows_affected() > 0 }))
}

/// Je, namba ipo kwenye blacklist?
pub async fn blacklist_has(db: &SqlitePool, number: &str) -> bool {
    let n = normalize_phone(number);
    let row: Option<(i64,)> = sqlx::query_as("SELECT COUNT(1) FROM ti_blacklist WHERE number=?")
        .bind(&n)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    row.map(|(c,)| c > 0).unwrap_or(false)
}

/// Orodha ya blacklist.
pub async fn blacklist_list(db: &SqlitePool, limit: i64) -> serde_json::Value {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT number, reason, added_by, created_at FROM ti_blacklist ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(limit.clamp(1, 1000))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|(n, r, by, at)| json!({ "number": n, "reason": r, "added_by": by, "created_at": at }))
        .collect();
    json!({ "ok": true, "count": items.len(), "items": items })
}

/// Ongeza scam pattern.
pub async fn pattern_add(
    db: &SqlitePool,
    pattern: &str,
    description: &str,
) -> Result<serde_json::Value, String> {
    let p = pattern.trim().to_lowercase();
    if p.is_empty() {
        return Err("pattern tupu".into());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query("INSERT OR REPLACE INTO ti_patterns (pattern, description, created_at) VALUES (?,?,?)")
        .bind(&p)
        .bind(description)
        .bind(&now)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "pattern": p }))
}

/// Orodha ya scam patterns.
pub async fn pattern_list(db: &SqlitePool, limit: i64) -> serde_json::Value {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT pattern, description, created_at FROM ti_patterns ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(limit.clamp(1, 1000))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|(p, d, at)| json!({ "pattern": p, "description": d, "created_at": at }))
        .collect();
    json!({ "ok": true, "count": items.len(), "items": items })
}

/// Hifadhi ripoti ya uchanganuzi.
pub async fn save_report(
    db: &SqlitePool,
    account: &str,
    kind: &str,
    target: &str,
    score: i64,
    verdict: &str,
    signals: &[Signal],
    note: &str,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let at = chrono::Local::now().to_rfc3339();
    let sj = serde_json::to_string(signals).unwrap_or_else(|_| "[]".into());
    sqlx::query(
        "INSERT INTO ti_reports (id, at, account, kind, target, score, verdict, signals, note) VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&at)
    .bind(account)
    .bind(kind)
    .bind(target)
    .bind(score)
    .bind(verdict)
    .bind(&sj)
    .bind(note)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Muhtasari wa threat intel kwa account (data HALISI pekee).
pub async fn summary(db: &SqlitePool, account: &str) -> serde_json::Value {
    let rows: Vec<(String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT at, kind, target, score, verdict, signals FROM ti_reports WHERE account=?1 ORDER BY at DESC",
    )
    .bind(account)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut kinds: std::collections::HashMap<String, i64> = Default::default();
    let mut verdicts: std::collections::HashMap<String, i64> = Default::default();
    let mut reports: Vec<serde_json::Value> = Vec::new();
    for (i, (at, kind, target, score, verdict, sj)) in rows.iter().enumerate() {
        *kinds.entry(kind.clone()).or_insert(0) += 1;
        *verdicts.entry(verdict.clone()).or_insert(0) += 1;
        if i < 20 {
            let signals: Vec<Signal> = serde_json::from_str(sj).unwrap_or_default();
            reports.push(json!({
                "at": at, "kind": kind, "target": target, "score": score,
                "verdict": verdict, "signals_count": signals.len(),
            }));
        }
    }

    let bl: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ti_blacklist")
        .fetch_one(db)
        .await
        .unwrap_or((0,));
    let pat: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM ti_patterns")
        .fetch_one(db)
        .await
        .unwrap_or((0,));

    json!({
        "ok": true,
        "account": account,
        "total_scans": rows.len(),
        "kinds": kinds,
        "verdicts": verdicts,
        "blacklist_count": bl.0,
        "patterns_count": pat.0,
        "reports": reports,
        "note_sw": "Uchanganuzi ni wa DEFENSIVE: unachanganua data uliyotuma (consent), hukuingilia mawasiliano ya wengine.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_vizingiti() {
        assert_eq!(verdict_of(0), "salama");
        assert_eq!(verdict_of(39), "salama");
        assert_eq!(verdict_of(40), "tahadhari");
        assert_eq!(verdict_of(69), "tahadhari");
        assert_eq!(verdict_of(70), "hatari");
        assert_eq!(verdict_of(100), "hatari");
    }

    #[test]
    fn agents_za_ulinzi_zinaunganishwa_na_threatswarm() {
        let v = defensive_agents();
        assert_eq!(v["ok"], serde_json::Value::Bool(true));
        let arr = v["agents"].as_array().unwrap();
        assert!(arr.len() >= 5, "agents za ulinzi zipo: {}", arr.len());
        let names: Vec<&str> = arr.iter().filter_map(|a| a["agent"].as_str()).collect();
        assert!(names.contains(&"blue-team"));
        assert!(names.contains(&"dfir"));
        assert!(names.contains(&"threat-hunter"));
        // agents za mashambulizi HAZIWI hapa (defensive pekee)
        assert!(!names.contains(&"exploit"));
        assert!(!names.contains(&"c2-operator"));
    }

    #[test]
    fn sms_ulaghai_wa_kawaida_unagundulika_hatari() {
        let a = score_message(
            "Hongera! Umeshinda TSh 5,000,000. Tuma TSh 50,000 kwa +25570000123 kupata zawadi yako.",
        );
        assert!(a.score >= DANGER, "score={} ", a.score);
        assert_eq!(a.verdict, "hatari");
        assert!(a.signals.iter().any(|s| s.id == "scam_term"));
        assert!(a.signals.iter().any(|s| s.id == "has_phone"));
    }

    #[test]
    fn sms_ya_kawaida_ni_salama() {
        let a = score_message("Habari, tutakutana saa tatu alasiri kwenye ofisi.");
        assert!(a.score < WARNING, "score={}", a.score);
        assert_eq!(a.verdict, "salama");
    }

    #[test]
    fn ombi_la_otp_ni_hatari_kubwa() {
        let a = score_message("CRDB: Thibitisha akaunti yako. Tuma OTP yako sasa.");
        assert!(a.score >= DANGER);
        assert!(a.signals.iter().any(|s| s.id == "otp_request"));
        assert!(a.signals.iter().any(|s| s.id == "brand_plus_request"));
    }

    #[test]
    fn url_ya_kikoa_halali_ni_salama() {
        let u = analyze_url("https://www.crdbbank.co.tz/login");
        assert_eq!(u.host, "www.crdbbank.co.tz");
        assert!(u.score < WARNING, "score={} reasons={:?}", u.score, u.reasons);
        assert!(u.brand.is_none());
    }

    #[test]
    fn typosquatting_ya_brand_inagundulika() {
        let u = analyze_url("http://crdb-bank.com/verify");
        assert!(u.score >= WARNING, "score={} reasons={:?}", u.score, u.reasons);
        assert_eq!(u.brand.as_deref(), Some("crdb"));
    }

    #[test]
    fn homoglyph_ya_brand_inagundulika() {
        // "cr0db" (0 kwa o) → homoglyph
        let u = analyze_url("http://cr0db.co.ke/");
        assert!(u.score >= WARNING, "score={} reasons={:?}", u.score, u.reasons);
    }

    #[test]
    fn ip_literal_na_shortener_zinagundulika() {
        let ip = analyze_url("http://192.168.1.10/pay");
        assert!(ip.reasons.iter().any(|r| r.contains("IP")));
        let sh = analyze_url("https://bit.ly/x9k2");
        assert!(sh.score >= 20, "shortener score={}", sh.score);
    }

    #[test]
    fn tld_hatari_inagundulika() {
        let u = analyze_url("http://free-prize.xyz/claim");
        assert!(u.reasons.iter().any(|r| r.contains(".xyz")), "{:?}", u.reasons);
    }

    #[test]
    fn caller_spoofing_inagundulika() {
        let c = check_caller("+255712345678", "+255700000000");
        assert!(c.spoofed);
        assert!(c.score >= DANGER);
        let ok = check_caller("+255712345678", "0712345678");
        assert!(!ok.spoofed, "namba hiyo hiyo kwa miundo tofauti si spoofing");
    }

    #[test]
    fn normalize_phone_inashughulikia_miundo() {
        assert_eq!(normalize_phone("0712345678"), "+255712345678");
        assert_eq!(normalize_phone("+255 712 345 678"), "+255712345678");
        assert_eq!(normalize_phone("255712345678"), "+255712345678");
    }

    #[test]
    fn levenshtein_sahihi() {
        assert_eq!(levenshtein("crdb", "crdb"), 0);
        assert_eq!(levenshtein("crdb", "crdbb"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn sms_sim_swap_na_telco_ni_hatari() {
        let a = score_message(
            "Vodacom: SIM yako inabadilishwa leo. Kubadilisha SIM kunahitaji OTP yako. Thibitisha sasa.",
        );
        assert!(a.score >= DANGER, "score={}", a.score);
        assert!(a.signals.iter().any(|s| s.id == "sim_swap_attempt"));
        assert!(a.signals.iter().any(|s| s.id == "sim_swap_telco"));
        // Aina nyingine ya kitenzi: "laini yako imebadilishwa"
        let b = score_message("Airtel: laini yako imebadilishwa, thibitisha namba yako.");
        assert!(b.signals.iter().any(|s| s.id == "sim_swap_attempt"), "{:?}", b.signals);
    }

    #[test]
    fn sms_halohalo_ya_ndugu_inagundulika() {
        let a = score_message("Nisaidie! Nimepata ajali, niokoe. Tuma pesa haraka, usimwambie mtu.");
        assert!(a.score >= WARNING, "score={}", a.score);
        assert!(a.signals.iter().any(|s| s.id == "scam_term"));
    }

    #[test]
    fn callback_halisi_inaaminika() {
        let c = CallbackInput {
            checkout_request_id: "ws_CO_123".into(),
            amount: 50000.0,
            expected_amount: 50000.0,
            request_created_at: 1_000_000,
            callback_at: 1_000_060,
            signature_ok: true,
            ip_whitelisted: true,
            already_settled: false,
        };
        let a = verify_payment_callback(&c);
        assert!(a.trusted, "score={} reasons={:?}", a.score, a.reasons);
        assert_eq!(a.verdict, "salama");
    }

    #[test]
    fn callback_ya_ughushi_haitumiki() {
        // Bila sahihi, IP isiyoruhusiwa, ilishalipwa, na kiasi tofauti
        let c = CallbackInput {
            checkout_request_id: "ws_CO_999".into(),
            amount: 100000.0,
            expected_amount: 50000.0,
            request_created_at: 1_000_000,
            callback_at: 1_000_030,
            signature_ok: false,
            ip_whitelisted: false,
            already_settled: true,
        };
        let a = verify_payment_callback(&c);
        assert!(!a.trusted);
        assert!(a.score >= DANGER, "score={}", a.score);
    }

    #[test]
    fn callback_iliyofika_kabla_ya_ombi_ni_hatari() {
        let c = CallbackInput {
            checkout_request_id: "ws_CO_777".into(),
            amount: 50000.0,
            expected_amount: 50000.0,
            request_created_at: 2_000_000,
            callback_at: 1_999_000, // kabla ya ombi
            signature_ok: true,
            ip_whitelisted: true,
            already_settled: false,
        };
        let a = verify_payment_callback(&c);
        assert!(!a.trusted);
        assert!(a.reasons.iter().any(|r| r.contains("KABLA ya ombi")));
    }

    #[tokio::test]
    async fn blacklist_na_ripoti_zinafanya_kazi() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        blacklist_add(&db, "0712345678", "spam", "admin").await.unwrap();
        assert!(blacklist_has(&db, "+255712345678").await);
        let list = blacklist_list(&db, 10).await;
        assert_eq!(list["count"], serde_json::json!(1));
        blacklist_remove(&db, "0712345678").await.unwrap();
        assert!(!blacklist_has(&db, "0712345678").await);

        let a = score_message("Umeshinda zawadi, tuma pesa");
        let id = save_report(&db, "mteja1", "sms", "+25570000123", a.score, &a.verdict, &a.signals, "")
            .await
            .unwrap();
        assert!(!id.is_empty());
        let s = summary(&db, "mteja1").await;
        assert_eq!(s["total_scans"], serde_json::json!(1));
        assert_eq!(s["ok"], serde_json::Value::Bool(true));
        let s2 = summary(&db, "mwingine").await;
        assert_eq!(s2["total_scans"], serde_json::json!(0));
    }

    #[tokio::test]
    async fn leja_ya_callback_inazuia_replay() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        assert!(!callback_seen(&db, "ws_CO_replay").await);
        callback_record(&db, "ws_CO_replay", true, 0).await.unwrap();
        assert!(callback_seen(&db, "ws_CO_replay").await);
        // Kurekodi tena ni idempotent
        callback_record(&db, "ws_CO_replay", false, 90).await.unwrap();
        assert!(callback_seen(&db, "ws_CO_replay").await);
        // ID tupu hairekodiwi
        assert!(!callback_seen(&db, "").await);
    }
}
