//! yt.rs — YouTube security modules (Modules 258-272).
//!
//! Jukwaa: YouTube. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "youtube";
pub const NAME: &str = "YouTube";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 258, last: 272 };

/// Modules zote za YouTube.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 258,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya YouTube (Google)",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 259,
            platform: PLATFORM,
            name: "Child Safety (Family)",
            kazi: "Inalinda mtoto kwenye YouTube",
            twist: "Mzazi anaweka sheria za kuchuja",
        },
        SocialMod {
            id: 260,
            platform: PLATFORM,
            name: "Video Privacy",
            kazi: "Inadhibiti nani anaweza kuona video",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 261,
            platform: PLATFORM,
            name: "Comment Filter",
            kazi: "Inachuja maoni yenye matusi au vitisho",
            twist: "Inaficha maoni ya matusi",
        },
        SocialMod {
            id: 262,
            platform: PLATFORM,
            name: "Live Chat Security",
            kazi: "Inachambua live chat kwa maneno machafu",
            twist: "Inazuia maoni ya matusi kwa wakati halisi",
        },
        SocialMod {
            id: 263,
            platform: PLATFORM,
            name: "Restricted Mode",
            kazi: "Inaweka restricted mode kiotomatiki kwa watoto",
            twist: "Inazuia video za watu wazima",
        },
        SocialMod {
            id: 264,
            platform: PLATFORM,
            name: "Channel Security",
            kazi: "Inalinda channel za YouTube dhidi ya kuvunjwa",
            twist: "Inazuia watu wasiofaa kubadilisha channel",
        },
        SocialMod {
            id: 265,
            platform: PLATFORM,
            name: "Playlist Security",
            kazi: "Inadhibiti nani anaweza kuongeza kwenye playlist",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 266,
            platform: PLATFORM,
            name: "Subscription Security",
            kazi: "Inagundua subscriptions bandia",
            twist: "Inazuia subscriptions bandia",
        },
        SocialMod {
            id: 267,
            platform: PLATFORM,
            name: "History Security",
            kazi: "Inalinda historia ya utazamaji",
            twist: "Inaficha historia",
        },
        SocialMod {
            id: 268,
            platform: PLATFORM,
            name: "Studio Security",
            kazi: "Inalinda YouTube Studio kwa waundaji",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 269,
            platform: PLATFORM,
            name: "Ads Security",
            kazi: "Inachambua matangazo yenye maudhui hatari",
            twist: "Inazuia matangazo ya ulaghai",
        },
        SocialMod {
            id: 270,
            platform: PLATFORM,
            name: "Analytics Security",
            kazi: "Inalinda data za analytics za waundaji",
            twist: "Inazuia wengine kuona analytics",
        },
        SocialMod {
            id: 271,
            platform: PLATFORM,
            name: "Copyright Security",
            kazi: "Inasaidia kugundua ukiukaji wa hakimiliki",
            twist: "Inaonya ikiwa video ina copyright infringement",
        },
        SocialMod {
            id: 272,
            platform: PLATFORM,
            name: "Community Post Security",
            kazi: "Inachambua community posts za ulaghai",
            twist: "Inazuia community posts za ulaghai",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yt_ina_modules_15() {
        assert_eq!(modules().len(), 15);
        assert_eq!(modules()[0].id, 258);
        assert_eq!(modules()[14].id, 272);
    }
}

// ---------- INJINI YA YOUTUBE ----------
pub struct YouTubeEngine;
const YT_LOOKALIKE: &[&str] = &["youtube-login", "youtu.be-verify", "youtube-verify", "youtube-monetize-free", "youtube-secure"];
impl PlatformEngine for YouTubeEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "youtube", &mut r);
        for l in YT_LOOKALIKE { if low.contains(l) { r.hit("yt_lookalike_domain", 70, format!("{url}: domain inaiga YouTube ('{l}')")); } }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("monetization") || low.contains("monetize") || low.contains("mapato"))
            && (low.contains("apply") || low.contains("omba") || low.contains("link") || low.contains("fast")) {
            r.hit("yt_monetization_scam", 80, "Ulaghai wa kuwasha mapato (monetization)");
        }
        if (low.contains("subscribers") || low.contains("wafuasi")) && (low.contains("buy") || low.contains("free") || low.contains("nunua")) {
            r.hit("yt_sub_scam", 60, "Kununua/kupata wafuasi bandia");
        }
        if low.contains("copyright strike") && (low.contains("appeal") || low.contains("link") || low.contains("kata rufaa")) {
            r.hit("yt_copyright_phish", 75, "Phishing ya copyright strike");
        }
        if (low.contains("giveaway") || low.contains("zawadi")) && (low.contains("comment") || low.contains("link") || low.contains("click")) {
            r.hit("yt_giveaway_scam", 70, "Giveaway ya YouTube yenye kiungo");
        }
        if low.contains("join my channel") && low.contains("http") { r.hit("yt_channel_bait", 50, "Bait ya kujiunga na channel"); }
        for s in platform::urgency_signals(text) { r.hit("yt_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("child_account").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.hit("yt_child_safety", 60, "Akaunti ya mtoto — Restricted Mode unapendekezwa");
        }
        if params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.7 { r.hit("yt_bot", 80, "Alama ya bot juu"); }
        r
    }
}
pub fn engine() -> YouTubeEngine { YouTubeEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn yt_inagundua_monetization_scam() {
        let e = engine();
        assert!(e.text_checks("Fast monetization! Apply here via link").findings.iter().any(|f| f.kind == "yt_monetization_scam"));
        assert!(e.text_checks("Copyright strike appeal, click link now").findings.iter().any(|f| f.kind == "yt_copyright_phish"));
        assert!(e.url_checks("http://youtube-verify.com").findings.iter().any(|f| f.kind == "yt_lookalike_domain"));
    }
}
