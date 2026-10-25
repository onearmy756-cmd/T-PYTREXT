//! sg.rs — Signal security modules (Modules 223-232).
//!
//! Jukwaa: Signal. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "signal";
pub const NAME: &str = "Signal";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 223, last: 232 };

/// Modules zote za Signal.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 223,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Signal",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 224,
            platform: PLATFORM,
            name: "Message Encryption",
            kazi: "Inaficha ujumbe wa Signal (tayari encrypted, lakini inaongeza safu nyingine)",
            twist: "Double encryption",
        },
        SocialMod {
            id: 225,
            platform: PLATFORM,
            name: "Call Security",
            kazi: "Inalinda simu za sauti na video",
            twist: "Inaficha sauti na video",
        },
        SocialMod {
            id: 226,
            platform: PLATFORM,
            name: "Group Security",
            kazi: "Inachambua vikundi kabla ya kujiunga",
            twist: "Inagundua vikundi vya ulaghai",
        },
        SocialMod {
            id: 227,
            platform: PLATFORM,
            name: "Disappearing Message Security",
            kazi: "Inahakikisha ujumbe unaojiathiri unafutwa kwa usalama",
            twist: "Inathibitisha ujumbe umefutwa",
        },
        SocialMod {
            id: 228,
            platform: PLATFORM,
            name: "Note to Self Security",
            kazi: "Inalinda notes za mteja mwenyewe",
            twist: "Inaficha notes",
        },
        SocialMod {
            id: 229,
            platform: PLATFORM,
            name: "Payment Security (MobileCoin)",
            kazi: "Inalinda miamala ya MobileCoin kwenye Signal",
            twist: "Inathibitisha malipo",
        },
        SocialMod {
            id: 230,
            platform: PLATFORM,
            name: "Profile Security",
            kazi: "Inadhibiti nani anaweza kuona profile",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 231,
            platform: PLATFORM,
            name: "PIN Security",
            kazi: "Inalinda PIN ya Signal",
            twist: "Inahitaji PIN + fingerprint",
        },
        SocialMod {
            id: 232,
            platform: PLATFORM,
            name: "Registration Lock",
            kazi: "Inazuia usajili upya wa akaunti bila idhini",
            twist: "Inahitaji uthibitisho",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sg_ina_modules_10() {
        assert_eq!(modules().len(), 10);
        assert_eq!(modules()[0].id, 223);
        assert_eq!(modules()[9].id, 232);
    }
}

// ---------- INJINI YA SIGNAL ----------
pub struct SignalEngine;
const SG_LOOKALIKE: &[&str] = &["signal-login", "signal-verify", "signal-secure", "signalapp-verify"];
impl PlatformEngine for SignalEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "signal", &mut r);
        for l in SG_LOOKALIKE { if low.contains(l) { r.hit("sg_lookalike_domain", 70, format!("{url}: domain inaiga Signal ('{l}')")); } }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("safety number") || low.contains("namba ya usalama")) && (low.contains("verify") || low.contains("thibitisha")) {
            r.note("Uthibitisho wa Safety Number unapendekezwa");
        }
        if (low.contains("registration lock") || low.contains("pin")) && (low.contains("weka") || low.contains("set")) {
            r.hit("sg_pin_advice", 30, "Weka Registration Lock (PIN) kwa usalama");
        }
        if (low.contains("otp") || low.contains("code")) && (low.contains("nipe") || low.contains("tuma") || low.contains("send")) {
            r.hit("sg_otp_request", 90, "Ombi la code/OTP — hatari ya kuchukua akaunti");
        }
        if low.contains("mobilecoin") || low.contains("signal pay") {
            if low.contains("tuma") || low.contains("send") || low.contains("link") { r.hit("sg_payment_scam", 70, "Malipo ya Signal yanayotiliwa shaka"); }
        }
        if (low.contains("disappearing") || low.contains("kutoweka")) && low.contains("off") { r.hit("sg_no_disappear", 35, "Ujumbe wa kutoweka haujawashwa"); }
        for s in platform::urgency_signals(text) { r.hit("sg_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if !params.get("registration_lock").and_then(|v| v.as_bool()).unwrap_or(true) {
            r.hit("sg_no_reg_lock", 50, "Registration Lock haijawekwa — hatari ya SIM-swap");
        }
        if params.get("new_device").and_then(|v| v.as_bool()).unwrap_or(false) { r.hit("sg_new_device", 55, "Kifaa kipya kimeunganishwa"); }
        r
    }
}
pub fn engine() -> SignalEngine { SignalEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn sg_inagundua_otp_na_reg_lock() {
        let e = engine();
        assert!(e.text_checks("Nipe code yako ya Signal haraka").findings.iter().any(|f| f.kind == "sg_otp_request"));
        let p = serde_json::json!({"registration_lock": false});
        assert!(e.account_checks(&p).findings.iter().any(|f| f.kind == "sg_no_reg_lock"));
        assert!(e.url_checks("http://signal-verify.com").findings.iter().any(|f| f.kind == "sg_lookalike_domain"));
    }
}
