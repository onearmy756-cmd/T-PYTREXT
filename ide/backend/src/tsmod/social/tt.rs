//! tt.rs — TikTok security modules (Modules 158-177).
//!
//! Jukwaa: TikTok. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "tiktok";
pub const NAME: &str = "TikTok";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 158, last: 177 };

/// Modules zote za TikTok.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 158,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya TikTok",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 159,
            platform: PLATFORM,
            name: "Child Safety (Family)",
            kazi: "Inalinda mtoto kwenye TikTok",
            twist: "Mzazi anaweka sheria za kuchuja",
        },
        SocialMod {
            id: 160,
            platform: PLATFORM,
            name: "DM Protection",
            kazi: "Inachambua Direct Messages kwenye TikTok",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 161,
            platform: PLATFORM,
            name: "Comment Filter",
            kazi: "Inachuja maoni yenye matusi au vitisho",
            twist: "Inaficha maoni ya matusi",
        },
        SocialMod {
            id: 162,
            platform: PLATFORM,
            name: "Follower Analysis",
            kazi: "Inachambua wafuasi wapya (bandia au hatari)",
            twist: "Inazuia wafuasi bandia",
        },
        SocialMod {
            id: 163,
            platform: PLATFORM,
            name: "Live Security",
            kazi: "Inachambua maoni kwenye Live na kuzuia matusi",
            twist: "Inazuia maoni ya matusi kwa wakati halisi",
        },
        SocialMod {
            id: 164,
            platform: PLATFORM,
            name: "Family Pairing",
            kazi: "Inaunganisha akaunti ya mtoto na mzazi",
            twist: "Mzazi anaweza kuona shughuli za mtoto",
        },
        SocialMod {
            id: 165,
            platform: PLATFORM,
            name: "Duet Security",
            kazi: "Inadhibiti nani anaweza kufanya Duet na mtoto",
            twist: "Inazuia watu wasiofaa kufanya Duet",
        },
        SocialMod {
            id: 166,
            platform: PLATFORM,
            name: "Stitch Security",
            kazi: "Inadhibiti nani anaweza kufanya Stitch na video za mtoto",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 167,
            platform: PLATFORM,
            name: "QR Code Security",
            kazi: "Inachambua QR codes za ulaghai",
            twist: "Inabonyeza QR code kwenye sandbox",
        },
        SocialMod {
            id: 168,
            platform: PLATFORM,
            name: "Sound Security",
            kazi: "Inachambua sauti zenye maneno machafu",
            twist: "Inazuia sauti zenye matusi",
        },
        SocialMod {
            id: 169,
            platform: PLATFORM,
            name: "Effect Security",
            kazi: "Inalinda filters na effects za hatari",
            twist: "Inazuia effects za hatari",
        },
        SocialMod {
            id: 170,
            platform: PLATFORM,
            name: "Hashtag Challenge Security",
            kazi: "Inaonyesha warning kwa challenges za hatari",
            twist: "Inaonya ikiwa challenge ni hatari",
        },
        SocialMod {
            id: 171,
            platform: PLATFORM,
            name: "Business Account Security",
            kazi: "Inalinda akaunti za biashara kwenye TikTok",
            twist: "Inalinda mawasiliano ya biashara",
        },
        SocialMod {
            id: 172,
            platform: PLATFORM,
            name: "Creator Marketplace Security",
            kazi: "Inalinda waundaji kwenye marketplace",
            twist: "Inathibitisha wateja ni halali",
        },
        SocialMod {
            id: 173,
            platform: PLATFORM,
            name: "TikTok Shop Security",
            kazi: "Inalinda ununuzi kwenye TikTok Shop",
            twist: "Inathibitisha muuzaji ni halali",
        },
        SocialMod {
            id: 174,
            platform: PLATFORM,
            name: "Ads Manager Security",
            kazi: "Inalinda matangazo ya biashara",
            twist: "Inazuia matangazo yasiyoidhinishwa",
        },
        SocialMod {
            id: 175,
            platform: PLATFORM,
            name: "Analytics Security",
            kazi: "Inalinda data za analytics za waundaji",
            twist: "Inazuia wengine kuona analytics",
        },
        SocialMod {
            id: 176,
            platform: PLATFORM,
            name: "Promotion Security",
            kazi: "Inachambua matangazo ya kulipia",
            twist: "Inazuia matangazo ya ulaghai",
        },
        SocialMod {
            id: 177,
            platform: PLATFORM,
            name: "Series Security",
            kazi: "Inalinda maudhui ya kulipia (Series)",
            twist: "Inalinda malipo",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tt_ina_modules_20() {
        assert_eq!(modules().len(), 20);
        assert_eq!(modules()[0].id, 158);
        assert_eq!(modules()[19].id, 177);
    }
}

// ---------- INJINI YA TIKTOK ----------
pub struct TikTokEngine;
const TT_LOOKALIKE: &[&str] = &["tiktok-verify", "tik-tok", "tiktok-gift", "tiktok-secure", "tt-verify", "tiktokshop-free"];
impl PlatformEngine for TikTokEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "tiktok", &mut r);
        for l in TT_LOOKALIKE { if low.contains(l) { r.hit("tt_lookalike_domain", 70, format!("{url}: domain inaiga TikTok ('{l}')")); } }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("gift") || low.contains("zawadi") || low.contains("free coins") || low.contains("diamonds"))
            && (low.contains("claim") || low.contains("dai") || low.contains("link") || low.contains("bofya")) {
            r.hit("tt_gift_scam", 75, "Ulaghai wa zawadi/coins za TikTok");
        }
        if (low.contains("tiktok shop") || low.contains("shop")) && (low.contains("tuma pesa") || low.contains("m-pesa") || low.contains("deposit")) {
            r.hit("tt_shop_scam", 70, "Ulaghai wa TikTok Shop (malipo ya awali)");
        }
        if low.contains("scan qr") || low.contains("skani qr") || low.contains("scan the code") {
            r.hit("tt_qr_bait", 55, "Ushawishi wa kuskani QR");
        }
        if low.contains("family pairing") || low.contains("parental") { r.note("Udhibiti wa wazazi (Family Pairing) unapendekezwa"); }
        if (low.contains("duet") || low.contains("stitch")) && low.contains("link") { r.hit("tt_duet_bait", 45, "Bait ya Duet/Stitch yenye kiungo"); }
        for s in platform::urgency_signals(text) { r.hit("tt_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("child_account").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.hit("tt_child_safety", 60, "Akaunti ya mtoto — udhibiti wa wazazi unahitajika");
        }
        if params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.7 { r.hit("tt_bot", 80, "Alama ya bot juu"); }
        r
    }
}
pub fn engine() -> TikTokEngine { TikTokEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn tt_inagundua_gift_na_shop_scam() {
        let e = engine();
        assert!(e.text_checks("Free coins! Claim your gift now, click link").findings.iter().any(|f| f.kind == "tt_gift_scam"));
        assert!(e.text_checks("TikTok Shop deal, tuma pesa kwa M-Pesa kwanza").findings.iter().any(|f| f.kind == "tt_shop_scam"));
        assert!(e.url_checks("http://tiktok-verify.com/claim").findings.iter().any(|f| f.kind == "tt_lookalike_domain"));
    }
}
