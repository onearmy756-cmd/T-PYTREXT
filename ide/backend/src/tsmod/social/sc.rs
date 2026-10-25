//! sc.rs — Snapchat security modules (Modules 178-192).
//!
//! Jukwaa: Snapchat. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "snapchat";
pub const NAME: &str = "Snapchat";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 178, last: 192 };

/// Modules zote za Snapchat.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 178,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Snapchat",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 179,
            platform: PLATFORM,
            name: "Snap Security",
            kazi: "Inachambua Snap (picha/video) kabla ya kuonyeshwa",
            twist: "Inazuia Snaps zenye maudhui hatari",
        },
        SocialMod {
            id: 180,
            platform: PLATFORM,
            name: "Story Security",
            kazi: "Inadhibiti nani anaweza kuona Story",
            twist: "Inazuia watu kwenye blacklist",
        },
        SocialMod {
            id: 181,
            platform: PLATFORM,
            name: "Chat Protection",
            kazi: "Inachambua chat kwa maneno hatari na viungo",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 182,
            platform: PLATFORM,
            name: "Map Privacy",
            kazi: "Inadhibiti nani anaweza kuona location kwenye Snap Map",
            twist: "Inazuia watu wasiofaa kuona location",
        },
        SocialMod {
            id: 183,
            platform: PLATFORM,
            name: "Location Safety",
            kazi: "Inaonyesha warning kabla ya kushiriki location",
            twist: "Inaonya \\\"Je, una uhakika?\\\"",
        },
        SocialMod {
            id: 184,
            platform: PLATFORM,
            name: "Memories Security",
            kazi: "Inalinda picha na video zilizohifadhiwa",
            twist: "Inaficha Memories",
        },
        SocialMod {
            id: 185,
            platform: PLATFORM,
            name: "Spotlight Security",
            kazi: "Inachambua video kwenye Spotlight",
            twist: "Inazuia video zenye maudhui hatari",
        },
        SocialMod {
            id: 186,
            platform: PLATFORM,
            name: "Discover Security",
            kazi: "Inachuja maudhui kwenye Discover",
            twist: "Inaficha maudhui hatari",
        },
        SocialMod {
            id: 187,
            platform: PLATFORM,
            name: "Lens Security",
            kazi: "Inalinda lenses za AR",
            twist: "Inazuia lenses za hatari",
        },
        SocialMod {
            id: 188,
            platform: PLATFORM,
            name: "Filter Security",
            kazi: "Inachambua filters za hatari",
            twist: "Inazuia filters za kuficha utambulisho",
        },
        SocialMod {
            id: 189,
            platform: PLATFORM,
            name: "Bitmoji Security",
            kazi: "Inalinda Bitmoji ya mteja",
            twist: "Inazuia watu kuiga Bitmoji",
        },
        SocialMod {
            id: 190,
            platform: PLATFORM,
            name: "Snapcode Security",
            kazi: "Inachambua Snapcodes za ulaghai",
            twist: "Inabonyeza Snapcode kwenye sandbox",
        },
        SocialMod {
            id: 191,
            platform: PLATFORM,
            name: "Friendship Profile Security",
            kazi: "Inadhibiti nani anaweza kuona Friendship Profile",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 192,
            platform: PLATFORM,
            name: "Streak Security",
            kazi: "Inalinda Streaks za mteja",
            twist: "Inaonya kabla ya Streak kufa",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sc_ina_modules_15() {
        assert_eq!(modules().len(), 15);
        assert_eq!(modules()[0].id, 178);
        assert_eq!(modules()[14].id, 192);
    }
}

// ---------- INJINI YA SNAPCHAT ----------
pub struct SnapchatEngine;
const SC_LOOKALIKE: &[&str] = &["snapchat-login", "snap-chat", "snapchat-verify", "snapcode-free", "snapchat-secure"];
impl PlatformEngine for SnapchatEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "snapchat", &mut r);
        for l in SC_LOOKALIKE { if low.contains(l) { r.hit("sc_lookalike_domain", 70, format!("{url}: domain inaiga Snapchat ('{l}')")); } }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("snapcode") || low.contains("snap code")) && (low.contains("scan") || low.contains("skani") || low.contains("link")) {
            r.hit("sc_snapcode_phish", 65, "Snapcode inayoelekeza kwenye kiungo");
        }
        if low.contains("streak") && (low.contains("keep") || low.contains("save") || low.contains("restore")) {
            r.hit("sc_streak_bait", 45, "Bait ya kuokoa streak");
        }
        if (low.contains("snap me") || low.contains("add me")) && (low.contains("link") || low.contains("http")) {
            r.hit("sc_add_bait", 50, "Mwaliko wa kuongeza rafiki wenye kiungo");
        }
        if low.contains("spotlight") && (low.contains("earn") || low.contains("money") || low.contains("pesa")) {
            r.hit("sc_spotlight_scam", 65, "Ulaghai wa Spotlight (mapato)");
        }
        for s in platform::urgency_signals(text) { r.hit("sc_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("location_sharing").and_then(|v| v.as_bool()).unwrap_or(false)
            && params.get("public").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.hit("sc_location_risk", 55, "Kushiriki eneo hadharani — hatari ya usalama");
        }
        if params.get("new_device").and_then(|v| v.as_bool()).unwrap_or(false) { r.hit("sc_new_device", 50, "Kuingia kifaa kipya"); }
        r
    }
}
pub fn engine() -> SnapchatEngine { SnapchatEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn sc_inagundua_snapcode_na_spotlight() {
        let e = engine();
        assert!(e.text_checks("Scan this snapcode to get free snap score link").findings.iter().any(|f| f.kind == "sc_snapcode_phish"));
        assert!(e.text_checks("Earn money on Spotlight, click link").findings.iter().any(|f| f.kind == "sc_spotlight_scam"));
        assert!(e.url_checks("http://snapchat-verify.com").findings.iter().any(|f| f.kind == "sc_lookalike_domain"));
    }
}
