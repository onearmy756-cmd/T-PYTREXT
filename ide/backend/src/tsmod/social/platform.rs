//! platform.rs — TRAIT YA INJINI YA KILA JUKWAA (code halisi, sio catalog).
//!
//! Kila jukwaa (Facebook, Instagram, WhatsApp, ...) lina sheria zake za
//! kimuundo ambazo ni za jukwaa hilo PEKEE — mfano Instagram ina Reels/Stories
//! na "link in bio", WhatsApp ina SIM-swap na namba za simu, Telegram ina
//! channel/forward, n.k. Tabaka hili linahifadhi mantiki hiyo halisi.
//!
//! Injini ya DB (`engine.rs`) inashughulikia sheria zinazoweza kubadilishwa;
//! hapa ni mantiki ya kimuundo isiyobadilika ya jukwaa (inayoendana na DB).

use serde_json::Value;

/// Ugunduzi mmoja wa jukwaa.
#[derive(Debug, Clone)]
pub struct PFind {
    pub kind: &'static str,
    pub severity: u8,
    pub detail: String,
}

/// Ripoti ya uchanganuzi wa jukwaa.
#[derive(Debug, Clone, Default)]
pub struct PReport {
    pub findings: Vec<PFind>,
    pub notes: Vec<String>,
}

impl PReport {
    pub fn hit(&mut self, kind: &'static str, severity: u8, detail: impl Into<String>) -> &mut Self {
        self.findings.push(PFind { kind, severity, detail: detail.into() });
        self
    }
    pub fn note(&mut self, n: impl Into<String>) -> &mut Self {
        self.notes.push(n.into());
        self
    }
    pub fn max_sev(&self) -> u8 {
        self.findings.iter().map(|f| f.severity).max().unwrap_or(0)
    }
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Injini ya jukwaa moja. Kila jukwaa linajibu kinachohusika nalo pekee.
pub trait PlatformEngine: Send + Sync {
    fn key(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// Uchanganuzi wa maudhui (maandishi: post, DM, comment, caption...).
    fn text_checks(&self, _text: &str) -> PReport {
        PReport::default()
    }
    /// Uchanganuzi wa kiungo/URL (pamoja na fake domain za jukwaa hili).
    fn url_checks(&self, _url: &str) -> PReport {
        PReport::default()
    }
    /// Uchanganuzi wa tabia ya akaunti (params zilizotolewa).
    fn account_checks(&self, _params: &Value) -> PReport {
        PReport::default()
    }
}

// ---------- VISAIDIZI VINAVYOSHIRIKIWA ----------

/// Umbali wa duara (km) kati ya nukta mbili (Haversine).
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0_f64;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Hesabu kasi inayowezekana (km/h) kati ya kuingia mbili.
pub fn speed_kmh(km: f64, secs: i64) -> f64 {
    if secs <= 0 {
        return f64::INFINITY;
    }
    km / (secs as f64 / 3600.0)
}

/// Kanuni za jumla za kimuundo kwa URL yoyote (hutumika na majukwaa yote).
pub fn generic_url_checks(url: &str, brand: &str, out: &mut PReport) {
    let low = url.to_lowercase();
    if !low.starts_with("https://") {
        out.hit("url_no_https", 20, format!("{url}: si HTTPS"));
    }
    let after = low.split("://").nth(1).unwrap_or(&low);
    let host = after.split(['/', '?', '#']).next().unwrap_or("");
    let host_no_port = host.split(':').next().unwrap_or(host);
    if host_no_port.parse::<std::net::Ipv4Addr>().is_ok() {
        out.hit("url_ip_literal", 35, format!("{url}: IP-literal badala ya domain"));
    }
    if host_no_port.contains("xn--") {
        out.hit("url_punycode", 45, format!("{url}: punycode/homoglyph"));
    }
    if after.contains('@') {
        out.hit("url_userinfo", 35, format!("{url}: '@' kwenye URL (userinfo trick)"));
    }
    // Uwepo wa jina la brand kwenye host kama sehemu isiyo ya mwisho (subdomain trick).
    if !brand.is_empty() {
        let labels: Vec<&str> = host_no_port.split('.').collect();
        if labels.len() >= 3 && labels[..labels.len() - 2].iter().any(|l| l.contains(brand)) {
            out.hit("url_brand_subdomain", 50,
                format!("{url}: '{brand}' kwenye subdomain — uwezekano wa kuiga"));
        }
    }
    // Njia za kawaida za phishing.
    for p in ["/login", "/signin", "/verify", "/account/confirm", "/password", "/wallet"] {
        if after.contains(p) {
            out.hit("url_phish_path", 30, format!("{url}: njia ya phishing ({p})"));
        }
    }
}

/// Ugunduzi wa namba za simu za Tanzania (+255/0[67]x) kwenye maandishi.
pub fn tz_phone_numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'+' || bytes[i] == b' ') {
                i += 1;
            }
            let raw: String = text[start..i].chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
            let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            let norm = if digits.starts_with("255") {
                Some(digits.clone())
            } else if digits.starts_with('0') && digits.len() == 10 {
                Some(format!("255{}", &digits[1..]))
            } else {
                None
            };
            if let Some(n) = norm {
                if n.len() == 12 && matches!(n.as_bytes()[3], b'6' | b'7') {
                    out.push(n);
                }
            }
        } else {
            i += 1;
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Je, maandishi yana mbinu za uharaka/kushinikiza (social engineering)?
pub fn urgency_signals(text: &str) -> Vec<&'static str> {
    let low = text.to_lowercase();
    let mut out = Vec::new();
    for (sig, words) in [
        ("uharaka", &["haraka", "urgent", "sasa hivi", "immediately", "within 24", "mwisho wa leo"][..]),
        ("vitisho", &["utafungwa", "account itafungwa", "suspended", "legal action", "polisi"][..]),
        ("zawadi", &["umeshinda", "you won", "zawadi", "prize", "lottery", "bahati"][..]),
        ("pesa", &["tuma pesa", "send money", "lipa", "deposit", "malipo ya awali", "activation fee"][..]),
        ("siri", &["usimwambie", "don't tell", "siri", "confidential", "kwa siri"][..]),
    ] {
        if words.iter().any(|w| low.contains(w)) {
            out.push(sig);
        }
    }
    out
}
