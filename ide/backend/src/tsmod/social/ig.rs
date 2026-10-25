//! ig.rs — Instagram security modules (Modules 88-112).
//!
//! Jukwaa: Instagram. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "instagram";
pub const NAME: &str = "Instagram";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 88, last: 112 };

/// Modules zote za Instagram.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 88,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Instagram",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 89,
            platform: PLATFORM,
            name: "DM Security",
            kazi: "Inachuja DM kwa viungo hatari na ulaghai",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 90,
            platform: PLATFORM,
            name: "Story Security",
            kazi: "Inadhibiti nani anaweza kuona Story",
            twist: "Inazuia watu kwenye blacklist",
        },
        SocialMod {
            id: 91,
            platform: PLATFORM,
            name: "Post Security",
            kazi: "Inachambua maoni kwenye post",
            twist: "Inaficha maoni ya matusi",
        },
        SocialMod {
            id: 92,
            platform: PLATFORM,
            name: "Comment Filter",
            kazi: "Inachuja maoni yenye maneno machafu",
            twist: "Inaficha automatically",
        },
        SocialMod {
            id: 93,
            platform: PLATFORM,
            name: "Hashtag Security",
            kazi: "Inaonyesha warning kwa hashtag hatari",
            twist: "Inaonya ikiwa hashtag inatumiwa kwa ulaghai",
        },
        SocialMod {
            id: 94,
            platform: PLATFORM,
            name: "Explore Security",
            kazi: "Inachuja maudhui kwenye sehemu ya Explore",
            twist: "Inaficha maudhui hatari",
        },
        SocialMod {
            id: 95,
            platform: PLATFORM,
            name: "IGTV Security",
            kazi: "Inachambua video kwenye IGTV",
            twist: "Inazuia video zenye maudhui hatari",
        },
        SocialMod {
            id: 96,
            platform: PLATFORM,
            name: "Reel Security",
            kazi: "Inachambua Reels zenye maudhui hatari",
            twist: "Inazuia Reels za ulaghai",
        },
        SocialMod {
            id: 97,
            platform: PLATFORM,
            name: "Live Security",
            kazi: "Inachambua maoni kwenye Live na kuzuia matusi",
            twist: "Inazuia maoni ya matusi kwa wakati halisi",
        },
        SocialMod {
            id: 98,
            platform: PLATFORM,
            name: "Shopping Security",
            kazi: "Inalinda ununuzi kwenye Instagram Shop",
            twist: "Inathibitisha muuzaji ni halali",
        },
        SocialMod {
            id: 99,
            platform: PLATFORM,
            name: "Business Security",
            kazi: "Inalinda akaunti za biashara",
            twist: "Inafuatilia wafanyakazi",
        },
        SocialMod {
            id: 100,
            platform: PLATFORM,
            name: "Creator Security",
            kazi: "Inalinda waundaji wa maudhui",
            twist: "Inalinda mapato ya waundaji",
        },
        SocialMod {
            id: 101,
            platform: PLATFORM,
            name: "Close Friends Security",
            kazi: "Inadhibiti nani yuko kwenye Close Friends list",
            twist: "Inazuia watu wasiofaa kuona Close Friends story",
        },
        SocialMod {
            id: 102,
            platform: PLATFORM,
            name: "Guide Security",
            kazi: "Inachambua miongozo (Guides) yenye maudhui hatari",
            twist: "Inazuia Guides za ulaghai",
        },
        SocialMod {
            id: 103,
            platform: PLATFORM,
            name: "Filter Security",
            kazi: "Inalinda filters za AR",
            twist: "Inagundua filters za kuficha utambulisho",
        },
        SocialMod {
            id: 104,
            platform: PLATFORM,
            name: "AR Effect Security",
            kazi: "Inachambua athari za AR",
            twist: "Inazuia AR effects za hatari",
        },
        SocialMod {
            id: 105,
            platform: PLATFORM,
            name: "Music Security",
            kazi: "Inahakikisha muziki unaotumiwa una leseni",
            twist: "Inazuia matumizi ya muziki bila leseni",
        },
        SocialMod {
            id: 106,
            platform: PLATFORM,
            name: "Question Sticker Security",
            kazi: "Inachuja maswali yenye maneno machafu",
            twist: "Inaficha maswali ya matusi",
        },
        SocialMod {
            id: 107,
            platform: PLATFORM,
            name: "Poll Sticker Security",
            kazi: "Inazuia kura bandia",
            twist: "Inagundua kura kutoka akaunti bandia",
        },
        SocialMod {
            id: 108,
            platform: PLATFORM,
            name: "Quiz Sticker Security",
            kazi: "Inalinda majibu ya quiz",
            twist: "Inazuia kuiba majibu",
        },
        SocialMod {
            id: 109,
            platform: PLATFORM,
            name: "Countdown Sticker Security",
            kazi: "Inaonyesha warning kwa countdown za ulaghai",
            twist: "Inaonya ikiwa countdown inatumiwa kwa ulaghai",
        },
        SocialMod {
            id: 110,
            platform: PLATFORM,
            name: "Emoji Slider Security",
            kazi: "Inachambua emoji za hatari",
            twist: "Inazuia emoji za matusi",
        },
        SocialMod {
            id: 111,
            platform: PLATFORM,
            name: "Fundraiser Sticker Security",
            kazi: "Inahakikisha uchangishaji ni halali",
            twist: "Inathibitisha kampeni",
        },
        SocialMod {
            id: 112,
            platform: PLATFORM,
            name: "Link Sticker Security",
            kazi: "Inachambua viungo kwenye stickers",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ig_ina_modules_25() {
        assert_eq!(modules().len(), 25);
        assert_eq!(modules()[0].id, 88);
        assert_eq!(modules()[24].id, 112);
    }

    #[test]
    fn ig_engine_inagundua_bio_scam_na_story_phish() {
        let e = engine();
        let r = e.text_checks("Link in bio 🔥🔥 free followers hack, click now");
        assert!(r.findings.iter().any(|f| f.kind == "ig_bio_fraud"));
        let r2 = e.url_checks("http://instagram-verify.secure-login.com/verify");
        assert!(r2.findings.iter().any(|f| f.kind == "ig_lookalike_domain"));
        let r3 = e.text_checks("Nimeku-tag kwenye story, angalia link hii");
        assert!(r3.findings.iter().any(|f| f.kind == "ig_story_mention_bait"));
    }
}

// ---------- INJINI YA INSTAGRAM ----------

pub struct InstagramEngine;

const IG_LOOKALIKE: &[&str] = &["instagram-login", "instagrarn", "instagram-verify", "ig-verify", "instagram-secure", "insta-login"];

impl PlatformEngine for InstagramEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }

    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "instagram", &mut r);
        for l in IG_LOOKALIKE {
            if low.contains(l) {
                r.hit("ig_lookalike_domain", 70, format!("{url}: domain inaiga Instagram ('{l}')"));
            }
        }
        if low.contains("linktr.ee") || low.contains("linkin.bio") {
            r.note(format!("{url}: kiungo cha 'link in bio' — hakikisha mmiliki"));
        }
        r
    }

    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        // "link in bio" + ahadi za uongo (followers/hack)
        let bait = ["free followers", "followers hack", "gain followers", "verified badge", "blue tick", "follow back fast"];
        if (low.contains("link in bio") || low.contains("bio link") || low.contains("link kwenye bio"))
            && (bait.iter().any(|w| low.contains(w)) || low.contains("bofya"))
        {
            r.hit("ig_bio_fraud", 65, "Ahadi ya uongo yenye 'link in bio'");
        }
        if bait.iter().any(|w| low.contains(w)) {
            r.hit("ig_follower_scam", 60, "Ulaghai wa kuongeza wafuasi");
        }
        // Bait ya DM ("tagged you", "see this")
        if (low.contains("tagged you") || low.contains("nimeku-tag") || low.contains("mentioned you"))
            && (low.contains("link") || low.contains("angalia"))
        {
            r.hit("ig_story_mention_bait", 55, "Ujumbe wa kudanganya kuhusu kutajwa kwenye story");
        }
        // Giveaway bandia
        if (low.contains("giveaway") || low.contains("mchanganuo") || low.contains("zawadi"))
            && (low.contains("dm") || low.contains("inbox") || low.contains("tuma"))
        {
            r.hit("ig_giveaway_scam", 65, "Giveaway inayotaka DM/kutuma");
        }
        // Sticker za kuchangisha fedha
        if (low.contains("fundraiser") || low.contains("donate") || low.contains("changisha"))
            && (low.contains("pesa") || low.contains("money") || low.contains("m-pesa"))
        {
            r.hit("ig_fundraiser_susp", 60, "Fundraiser inahitaji uthibitisho");
        }
        for s in platform::urgency_signals(text) {
            r.hit("ig_social_eng", 40, format!("Mbinu ya ushawishi: {s}"));
        }
        r
    }

    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        // Akaunti mpya yenye wafuasi wengi + hakuna posts = bandia.
        let followers = params.get("new_followers").and_then(|v| v.as_u64()).unwrap_or(0);
        let age = params.get("account_age_days").and_then(|v| v.as_i64()).unwrap_or(999);
        let posts = params.get("posts").and_then(|v| v.as_u64()).unwrap_or(1);
        if followers > 1000 && age < 7 && posts == 0 {
            r.hit("ig_bot_account", 75, format!("Wafuasi {followers}, siku {age}, posts {posts} — bandia"));
        }
        if params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.7 {
            r.hit("ig_bot_score", 80, "Alama ya bot juu — inazuiwa");
        }
        r
    }
}

pub fn engine() -> InstagramEngine { InstagramEngine }
