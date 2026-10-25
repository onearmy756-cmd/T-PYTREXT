//! rd.rs — Reddit security modules (Modules 248-257).
//!
//! Jukwaa: Reddit. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "reddit";
pub const NAME: &str = "Reddit";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 248, last: 257 };

/// Modules zote za Reddit.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 248,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Reddit",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 249,
            platform: PLATFORM,
            name: "Post Privacy",
            kazi: "Inadhibiti nani anaweza kuona machapisho",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 250,
            platform: PLATFORM,
            name: "Comment Protection",
            kazi: "Inachuja maoni yenye matusi au vitisho",
            twist: "Inaficha maoni ya matusi",
        },
        SocialMod {
            id: 251,
            platform: PLATFORM,
            name: "Subreddit Safety",
            kazi: "Inachambua subreddits kabla ya kujiunga",
            twist: "Inagundua subreddits za ulaghai",
        },
        SocialMod {
            id: 252,
            platform: PLATFORM,
            name: "Bot Detection",
            kazi: "Inagundua na kuzuia bot bandia",
            twist: "Inazuia bot bandia",
        },
        SocialMod {
            id: 253,
            platform: PLATFORM,
            name: "Karma Security",
            kazi: "Inalinda karma ya mteja dhidi ya manipulatio",
            twist: "Inagundua karma manipulation",
        },
        SocialMod {
            id: 254,
            platform: PLATFORM,
            name: "Award Security",
            kazi: "Inachambua awards za ulaghai",
            twist: "Inazuia awards za ulaghai",
        },
        SocialMod {
            id: 255,
            platform: PLATFORM,
            name: "Chat Security",
            kazi: "Inachambua chat kwa viungo hatari",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 256,
            platform: PLATFORM,
            name: "Poll Security",
            kazi: "Inazuia kura bandia kwenye polls",
            twist: "Inagundua kura bandia",
        },
        SocialMod {
            id: 257,
            platform: PLATFORM,
            name: "Prediction Security",
            kazi: "Inalinda predictions kwenye Reddit",
            twist: "Inathibitisha predictions",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rd_ina_modules_10() {
        assert_eq!(modules().len(), 10);
        assert_eq!(modules()[0].id, 248);
        assert_eq!(modules()[9].id, 257);
    }
}

// ---------- INJINI YA REDDIT ----------
pub struct RedditEngine;
const RD_LOOKALIKE: &[&str] = &["reddit-login", "reddit-verify", "reddit-gift", "reddit-premium-free", "redditapp-verify"];
impl PlatformEngine for RedditEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "reddit", &mut r);
        for l in RD_LOOKALIKE { if low.contains(l) { r.hit("rd_lookalike_domain", 70, format!("{url}: domain inaiga Reddit ('{l}')")); } }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("crypto") || low.contains("investment") || low.contains("wekeza"))
            && (low.contains("guaranteed") || low.contains("100%") || low.contains("double your")) {
            r.hit("rd_crypto_scam", 80, "Ulaghai wa uwekezaji wa crypto");
        }
        if (low.contains("free awards") || low.contains("free premium")) && (low.contains("click") || low.contains("link")) {
            r.hit("rd_award_scam", 65, "Ulaghai wa awards/premium bure");
        }
        if low.contains("dm me") && (low.contains("profit") || low.contains("faida") || low.contains("trade")) {
            r.hit("rd_dm_pump", 70, "Bait ya DM yenye ahadi za faida");
        }
        if low.contains("onlyfans") || low.contains("nsfw link") { r.hit("rd_nsfw_bait", 55, "Kiungo cha maudhui hatarishi"); }
        for s in platform::urgency_signals(text) { r.hit("rd_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        let karma = params.get("karma").and_then(|v| v.as_i64()).unwrap_or(0);
        let age = params.get("account_age_days").and_then(|v| v.as_i64()).unwrap_or(999);
        if age < 2 && karma < 1 { r.hit("rd_burner", 55, "Akaunti mpya bila karma — burner/spam"); }
        if params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.7 { r.hit("rd_bot", 80, "Alama ya bot juu"); }
        r
    }
}
pub fn engine() -> RedditEngine { RedditEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn rd_inagundua_crypto_scam() {
        let e = engine();
        assert!(e.text_checks("Guaranteed 100% profit, wekeza crypto").findings.iter().any(|f| f.kind == "rd_crypto_scam"));
        assert!(e.text_checks("Free awards, click link").findings.iter().any(|f| f.kind == "rd_award_scam"));
        assert!(e.url_checks("http://reddit-verify.com").findings.iter().any(|f| f.kind == "rd_lookalike_domain"));
    }
}
