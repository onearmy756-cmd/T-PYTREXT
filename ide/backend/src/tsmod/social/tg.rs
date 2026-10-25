//! tg.rs — Telegram security modules (Modules 208-222).
//!
//! Jukwaa: Telegram. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "telegram";
pub const NAME: &str = "Telegram";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 208, last: 222 };

/// Modules zote za Telegram.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 208,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Telegram",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 209,
            platform: PLATFORM,
            name: "Secret Chat Security",
            kazi: "Inalinda secret chats za Telegram",
            twist: "Inaficha secret chats",
        },
        SocialMod {
            id: 210,
            platform: PLATFORM,
            name: "Group Security",
            kazi: "Inachambua vikundi kabla ya kujiunga",
            twist: "Inagundua vikundi vya ulaghai",
        },
        SocialMod {
            id: 211,
            platform: PLATFORM,
            name: "Channel Security",
            kazi: "Inachambua channel zenye ulaghai",
            twist: "Inazuia channel za ulaghai",
        },
        SocialMod {
            id: 212,
            platform: PLATFORM,
            name: "Bot Security",
            kazi: "Inachambua bot za Telegram",
            twist: "Inazuia bot hatari",
        },
        SocialMod {
            id: 213,
            platform: PLATFORM,
            name: "Cloud Security",
            kazi: "Inalinda data kwenye Telegram cloud",
            twist: "Inaficha data kabla ya kuweka cloud",
        },
        SocialMod {
            id: 214,
            platform: PLATFORM,
            name: "Call Security",
            kazi: "Inalinda simu za sauti na video",
            twist: "Inaficha sauti na video",
        },
        SocialMod {
            id: 215,
            platform: PLATFORM,
            name: "Video Chat Security",
            kazi: "Inalinda video chats",
            twist: "Inaficha video",
        },
        SocialMod {
            id: 216,
            platform: PLATFORM,
            name: "Poll Security",
            kazi: "Inazuia kura bandia kwenye polls",
            twist: "Inagundua kura bandia",
        },
        SocialMod {
            id: 217,
            platform: PLATFORM,
            name: "Quiz Security",
            kazi: "Inalinda majibu ya quiz",
            twist: "Inazuia kuiba majibu",
        },
        SocialMod {
            id: 218,
            platform: PLATFORM,
            name: "Voice Chat Security",
            kazi: "Inalinda voice chats kwenye vikundi",
            twist: "Inaficha sauti",
        },
        SocialMod {
            id: 219,
            platform: PLATFORM,
            name: "Folder Security",
            kazi: "Inalinda folder za mazungumzo",
            twist: "Inaficha folder",
        },
        SocialMod {
            id: 220,
            platform: PLATFORM,
            name: "Sticker Security",
            kazi: "Inachambua stickers za hatari",
            twist: "Inazuia stickers zenye matusi",
        },
        SocialMod {
            id: 221,
            platform: PLATFORM,
            name: "GIF Security",
            kazi: "Inachambua GIF zenye maudhui hatari",
            twist: "Inazuia GIF za matusi",
        },
        SocialMod {
            id: 222,
            platform: PLATFORM,
            name: "Premium Security",
            kazi: "Inalinda akaunti za Telegram Premium",
            twist: "Inalinda malipo",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tg_ina_modules_15() {
        assert_eq!(modules().len(), 15);
        assert_eq!(modules()[0].id, 208);
        assert_eq!(modules()[14].id, 222);
    }
}

// ---------- INJINI YA TELEGRAM ----------
pub struct TelegramEngine;
const TG_LOOKALIKE: &[&str] = &["telegram-login", "telegram-verify", "telegrarn", "telegram-premium-free", "tg-verify"];
impl PlatformEngine for TelegramEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "telegram", &mut r);
        for l in TG_LOOKALIKE { if low.contains(l) { r.hit("tg_lookalike_domain", 70, format!("{url}: domain inaiga Telegram ('{l}')")); } }
        if low.contains("t.me/") { r.note(format!("{url}: kiungo cha Telegram")); }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("crypto") || low.contains("investment") || low.contains("wekeza") || low.contains("faida"))
            && (low.contains("guaranteed") || low.contains("100%") || low.contains("double") || low.contains("daily profit")) {
            r.hit("tg_investment_scam", 80, "Ulaghai wa uwekezaji wa crypto (faida ya uhakika)");
        }
        if (low.contains("premium") || low.contains("stars")) && (low.contains("free") || low.contains("bure") || low.contains("claim")) {
            r.hit("tg_premium_scam", 65, "Ulaghai wa Telegram Premium bure");
        }
        if low.contains("tap to earn") || low.contains("tap game") || low.contains("clicker") {
            r.hit("tg_tap_scam", 55, "Mchezo wa 'tap to earn' — uwezekano wa ulaghai");
        }
        if (low.contains("bot") && low.contains("join")) || low.contains("add bot") {
            r.hit("tg_bot_bait", 50, "Bait ya kuongeza bot");
        }
        if low.contains("forward to") || low.contains("share to 5 groups") { r.hit("tg_chain", 35, "Ujumbe wa mnyororo"); }
        if low.contains("secret chat") { r.note("Secret Chat inapendekezwa kwa faragha"); }
        for s in platform::urgency_signals(text) { r.hit("tg_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("new_device").and_then(|v| v.as_bool()).unwrap_or(false) { r.hit("tg_new_device", 50, "Kuingia kifaa kipya — uthibitisho"); }
        if !params.get("two_step").and_then(|v| v.as_bool()).unwrap_or(true) { r.hit("tg_no_2fa", 45, "Two-step verification haijawekwa"); }
        r
    }
}
pub fn engine() -> TelegramEngine { TelegramEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn tg_inagundua_investment_scam() {
        let e = engine();
        assert!(e.text_checks("Guaranteed 100% daily profit, wekeza crypto sasa").findings.iter().any(|f| f.kind == "tg_investment_scam"));
        assert!(e.text_checks("Free Telegram Premium, claim now").findings.iter().any(|f| f.kind == "tg_premium_scam"));
        assert!(e.url_checks("http://telegram-premium-free.com").findings.iter().any(|f| f.kind == "tg_lookalike_domain"));
    }
}
