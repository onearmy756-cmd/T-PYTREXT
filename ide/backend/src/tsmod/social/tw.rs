//! tw.rs — TwitterX security modules (Modules 138-157).
//!
//! Jukwaa: TwitterX. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "twitterx";
pub const NAME: &str = "TwitterX";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 138, last: 157 };

/// Modules zote za TwitterX.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 138,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Twitter",
            twist: "Inagundua kuingia kutoka location mpya",
        },
        SocialMod {
            id: 139,
            platform: PLATFORM,
            name: "DM Protection",
            kazi: "Inachuja DM kwa viungo hatari",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 140,
            platform: PLATFORM,
            name: "Mention Safety",
            kazi: "Inachambua mention zenye maneno machafu",
            twist: "Inaficha mention za matusi",
        },
        SocialMod {
            id: 141,
            platform: PLATFORM,
            name: "Hashtag Safety",
            kazi: "Inaonyesha warning kwa hashtag hatari",
            twist: "Inaonya ikiwa hashtag inatumiwa kwa uchochezi",
        },
        SocialMod {
            id: 142,
            platform: PLATFORM,
            name: "Trend Safety",
            kazi: "Inachambua trending topics zenye ulaghai",
            twist: "Inazuia trending topics za ulaghai",
        },
        SocialMod {
            id: 143,
            platform: PLATFORM,
            name: "Space Security",
            kazi: "Inalinda Twitter Spaces (majadiliano ya sauti)",
            twist: "Inafuatilia majadiliano ya sauti",
        },
        SocialMod {
            id: 144,
            platform: PLATFORM,
            name: "Quote Tweet Safety",
            kazi: "Inachambua quote tweets zenye maudhui hatari",
            twist: "Inaficha quote tweets za matusi",
        },
        SocialMod {
            id: 145,
            platform: PLATFORM,
            name: "List Security",
            kazi: "Inadhibiti nani anaweza kuona au kuongeza kwenye Lists",
            twist: "Inazuia watu wasiofaa kuona Lists",
        },
        SocialMod {
            id: 146,
            platform: PLATFORM,
            name: "Bookmark Security",
            kazi: "Inalinda bookmarks za mteja",
            twist: "Inaficha bookmarks za mteja",
        },
        SocialMod {
            id: 147,
            platform: PLATFORM,
            name: "Topic Security",
            kazi: "Inachambua topics zinazofuatwa",
            twist: "Inaonya ikiwa topic ina maudhui hatari",
        },
        SocialMod {
            id: 148,
            platform: PLATFORM,
            name: "Community Security",
            kazi: "Inalinda vikundi kwenye Twitter Communities",
            twist: "Inazuia watu wasiofaa kujiunga",
        },
        SocialMod {
            id: 149,
            platform: PLATFORM,
            name: "Newsletter Security",
            kazi: "Inachambua newsletters za ulaghai",
            twist: "Inazuia newsletters za ulaghai",
        },
        SocialMod {
            id: 150,
            platform: PLATFORM,
            name: "Tip Jar Security",
            kazi: "Inahakikisha Tip Jar haitumiwi kwa ulaghai",
            twist: "Inathibitisha tip jar ni halali",
        },
        SocialMod {
            id: 151,
            platform: PLATFORM,
            name: "Super Follow Security",
            kazi: "Inalinda wateja wanaolipia Super Follow",
            twist: "Inalinda malipo",
        },
        SocialMod {
            id: 152,
            platform: PLATFORM,
            name: "Twitter Blue Security",
            kazi: "Inalinda akaunti za Twitter Blue",
            twist: "Inathibitisha akaunti ni halali",
        },
        SocialMod {
            id: 153,
            platform: PLATFORM,
            name: "Ads Security",
            kazi: "Inachambua matangazo yenye ulaghai",
            twist: "Inazuia matangazo ya ulaghai",
        },
        SocialMod {
            id: 154,
            platform: PLATFORM,
            name: "Analytics Security",
            kazi: "Inalinda data za analytics za waundaji",
            twist: "Inazuia wengine kuona analytics",
        },
        SocialMod {
            id: 155,
            platform: PLATFORM,
            name: "Developer Portal Security",
            kazi: "Inalinda API keys za wasanidi programu",
            twist: "Inazuia matumizi mabaya ya API keys",
        },
        SocialMod {
            id: 156,
            platform: PLATFORM,
            name: "Circle Security",
            kazi: "Inadhibiti nani yuko kwenye Twitter Circle",
            twist: "Inazuia watu wasiofaa kwenye Circle",
        },
        SocialMod {
            id: 157,
            platform: PLATFORM,
            name: "Fleet Security (archive)",
            kazi: "Inalinda Fleets (zilizokwisha)",
            twist: "Inahifadhi Fleets kwenye archive salama",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tw_ina_modules_20() {
        assert_eq!(modules().len(), 20);
        assert_eq!(modules()[0].id, 138);
        assert_eq!(modules()[19].id, 157);
    }
}

// ---------- INJINI YA TWITTER/X ----------
pub struct TwitterEngine;
const TW_LOOKALIKE: &[&str] = &["twiter", "twitter-verify", "x-verify", "twitter-secure", "twitter-blue", "x-login"];
impl PlatformEngine for TwitterEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "twitter", &mut r);
        for l in TW_LOOKALIKE { if low.contains(l) { r.hit("tw_lookalike_domain", 70, format!("{url}: domain inaiga Twitter/X ('{l}')")); } }
        if low.contains("t.co/") { r.note(format!("{url}: kiungo kifupi cha X")); }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("giveaway") || low.contains("airdrop") || low.contains("zawadi"))
            && (low.contains("send") || low.contains("tuma") || low.contains("wallet") || low.contains("eth") || low.contains("btc")) {
            r.hit("tw_crypto_giveaway", 80, "Giveaway/airdrop ya crypto inayotaka kutuma");
        }
        if (low.contains("verified") || low.contains("blue tick") || low.contains("blue badge"))
            && (low.contains("apply") || low.contains("click") || low.contains("link") || low.contains("omba")) {
            r.hit("tw_blue_phish", 70, "Ulaghai wa 'Verified/Blue' yenye kiungo");
        }
        if low.contains("dm me") || low.contains("dm for") { r.hit("tw_dm_bait", 45, "Bait ya DM"); }
        if (low.contains("tip jar") || low.contains("cash app") || low.contains("send funds")) && low.contains("http") {
            r.hit("tw_tipjar_phish", 65, "Kiungo cha malipo kwenye Tip Jar");
        }
        if low.contains("crypto pump") || low.contains("pump and dump") || low.contains("100x") {
            r.hit("tw_pump_dump", 70, "Ishara za pump-and-dump ya crypto");
        }
        for s in platform::urgency_signals(text) { r.hit("tw_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.7 { r.hit("tw_bot", 80, "Alama ya bot juu"); }
        let age = params.get("account_age_days").and_then(|v| v.as_i64()).unwrap_or(999);
        let follows = params.get("following").and_then(|v| v.as_u64()).unwrap_or(0);
        if age < 7 && follows > 2000 { r.hit("tw_follow_spam", 60, "Akaunti mpya inafuata maelfu — spam"); }
        r
    }
}
pub fn engine() -> TwitterEngine { TwitterEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn tw_inagundua_giveaway_na_blue_phish() {
        let e = engine();
        assert!(e.text_checks("Huge giveaway! Send ETH to wallet 0xabc").findings.iter().any(|f| f.kind == "tw_crypto_giveaway"));
        assert!(e.text_checks("Apply for blue tick here: http://x-verify.com").findings.iter().any(|f| f.kind == "tw_blue_phish"));
        assert!(e.url_checks("http://twitter-verify.com/login").findings.iter().any(|f| f.kind == "tw_lookalike_domain"));
    }
}
