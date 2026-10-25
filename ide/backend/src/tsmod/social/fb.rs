//! fb.rs — Facebook security modules (Modules 58-87).
//!
//! Jukwaa: Facebook. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "facebook";
pub const NAME: &str = "Facebook";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 58, last: 87 };

/// Modules zote za Facebook.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 58,
            platform: PLATFORM,
            name: "Account Takeover Protection",
            kazi: "Inalinda akaunti ya Facebook",
            twist: "Inagundua kuingia kutoka location isiyo ya kawaida",
        },
        SocialMod {
            id: 59,
            platform: PLATFORM,
            name: "Phishing Detection",
            kazi: "Inagundua barua pepe na tovuti bandia za Facebook",
            twist: "Inabonyeza link kwenye sandbox kabla ya mteja",
        },
        SocialMod {
            id: 60,
            platform: PLATFORM,
            name: "Marketplace Security",
            kazi: "Inalinda wanunuzi na wauzaji",
            twist: "Inaonyesha warning ikiwa muuzaji ameripotiwa",
        },
        SocialMod {
            id: 61,
            platform: PLATFORM,
            name: "Group Security",
            kazi: "Inachambua vikundi kabla ya kujiunga",
            twist: "Inagundua vikundi vya Ponzi na crypto scams",
        },
        SocialMod {
            id: 62,
            platform: PLATFORM,
            name: "Page Security",
            kazi: "Inalinda kurasa za Facebook",
            twist: "Inahitaji uthibitisho kabla ya kuongeza admin mpya",
        },
        SocialMod {
            id: 63,
            platform: PLATFORM,
            name: "Event Security",
            kazi: "Inalinda matukio na walioalikwa",
            twist: "Inagundua matukio bandia ya ulaghai",
        },
        SocialMod {
            id: 64,
            platform: PLATFORM,
            name: "Live Security",
            kazi: "Inachambua maoni kwenye live na kuzuia matusi",
            twist: "Inazuia maoni ya matusi kwa wakati halisi",
        },
        SocialMod {
            id: 65,
            platform: PLATFORM,
            name: "Messenger Security",
            kazi: "Inachambua DM kwa viungo hatari",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 66,
            platform: PLATFORM,
            name: "Messenger Encryption",
            kazi: "Inaficha ujumbe wa Messenger mara mbili",
            twist: "WhatsApp + SIM double encryption",
        },
        SocialMod {
            id: 67,
            platform: PLATFORM,
            name: "Messenger Call Security",
            kazi: "Inalinda simu za sauti na video kwenye Messenger",
            twist: "Inaficha sauti na video",
        },
        SocialMod {
            id: 68,
            platform: PLATFORM,
            name: "Story Security",
            kazi: "Inazuia watu wasiofaa kuona Story",
            twist: "Inazuia watu kwenye blacklist kuona Story",
        },
        SocialMod {
            id: 69,
            platform: PLATFORM,
            name: "Comment Security",
            kazi: "Inachuja maoni yenye matusi au ulaghai",
            twist: "Inaficha maoni ya matusi automatically",
        },
        SocialMod {
            id: 70,
            platform: PLATFORM,
            name: "Friend Request Filter",
            kazi: "Inachuja maombi ya urafiki kutoka kwa watu hatari",
            twist: "Inazuia maombi kutoka kwa namba za ulaghai",
        },
        SocialMod {
            id: 71,
            platform: PLATFORM,
            name: "Data Leak Detection",
            kazi: "Inagundua kama data ya mteja imevuja kwenye dark web",
            twist: "Inatahadharisha mteja mara moja",
        },
        SocialMod {
            id: 72,
            platform: PLATFORM,
            name: "Ad Security",
            kazi: "Inachambua matangazo yenye ulaghai",
            twist: "Inazuia matangazo ya ulaghai",
        },
        SocialMod {
            id: 73,
            platform: PLATFORM,
            name: "Business Suite Security",
            kazi: "Inalinda biashara kwenye Facebook",
            twist: "Inafuatilia wafanyakazi wanaosimamia ukurasa",
        },
        SocialMod {
            id: 74,
            platform: PLATFORM,
            name: "Commerce Security",
            kazi: "Inalinda ununuzi na mauzo kwenye Facebook Shop",
            twist: "Inathibitisha muuzaji ni halali",
        },
        SocialMod {
            id: 75,
            platform: PLATFORM,
            name: "Gaming Security",
            kazi: "Inalinda wachezaji na miamala kwenye michezo",
            twist: "Inazuia ulaghai wa in-game currency",
        },
        SocialMod {
            id: 76,
            platform: PLATFORM,
            name: "Watch Security",
            kazi: "Inachuja video zisizofaa",
            twist: "Inazuia video zenye maudhui hatari",
        },
        SocialMod {
            id: 77,
            platform: PLATFORM,
            name: "Dating Security",
            kazi: "Inalinda watumiaji wa Facebook Dating",
            twist: "Inagundua fake profiles",
        },
        SocialMod {
            id: 78,
            platform: PLATFORM,
            name: "Job Security",
            kazi: "Inagundua matangazo bandia ya kazi",
            twist: "Inazuia matangazo ya kazi yanayoomba pesa",
        },
        SocialMod {
            id: 79,
            platform: PLATFORM,
            name: "Fundraising Security",
            kazi: "Inahakikisha uchangishaji ni halali",
            twist: "Inathibitisha kampeni kabla ya kuruhusu",
        },
        SocialMod {
            id: 80,
            platform: PLATFORM,
            name: "Crowdfunding Security",
            kazi: "Inalinda wachangiaji na wachangishaji",
            twist: "Inafuatilia fedha zilizochangishwa",
        },
        SocialMod {
            id: 81,
            platform: PLATFORM,
            name: "Poll Security",
            kazi: "Inazuia kura bandia kwenye Polls",
            twist: "Inagundua kura kutoka akaunti bandia",
        },
        SocialMod {
            id: 82,
            platform: PLATFORM,
            name: "Quiz Security",
            kazi: "Inalinda maswali na majibu ya Quiz",
            twist: "Inazuia kuiba majibu",
        },
        SocialMod {
            id: 83,
            platform: PLATFORM,
            name: "Avatar Security",
            kazi: "Inalinda picha bandia za watumiaji",
            twist: "Inagundua ikiwa avatar inaiga mtu mwingine",
        },
        SocialMod {
            id: 84,
            platform: PLATFORM,
            name: "Reel Security",
            kazi: "Inachambua Reels zenye maudhui hatari",
            twist: "Inazuia Reels zenye vurugu au ngono",
        },
        SocialMod {
            id: 85,
            platform: PLATFORM,
            name: "Sound Collection Security",
            kazi: "Inalinda sauti na muziki uliopakiwa",
            twist: "Inahakikisha muziki una leseni",
        },
        SocialMod {
            id: 86,
            platform: PLATFORM,
            name: "Professional Dashboard Security",
            kazi: "Inalinda dashibodi ya wataalamu",
            twist: "Inahitaji uthibitisho wa ziada",
        },
        SocialMod {
            id: 87,
            platform: PLATFORM,
            name: "Facebook Ads Manager Security",
            kazi: "Inalinda matangazo ya biashara",
            twist: "Inazuia matangazo yasiyoidhinishwa",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fb_ina_modules_30() {
        assert_eq!(modules().len(), 30);
        assert_eq!(modules()[0].id, 58);
        assert_eq!(modules()[29].id, 87);
    }

    #[test]
    fn fb_engine_inagundua_phishing_na_ulaghai() {
        let e = engine();
        // URL ya kuiga Facebook
        let r = e.url_checks("http://facebook-login.verify-secure.com/login");
        assert!(r.findings.iter().any(|f| f.kind == "url_brand_subdomain"));
        assert!(r.findings.iter().any(|f| f.kind == "url_phish_path"));
        assert!(r.max_sev() >= 50);
        // Ulaghai wa Marketplace
        let r2 = e.text_checks("Bei rahisi sana, tuma pesa kwa M-Pesa 0712345678 upate bidhaa");
        assert!(r2.findings.iter().any(|f| f.kind == "fb_marketplace_scam"));
        // Kiungo halali hakitatoa brand-subdomain
        let r3 = e.url_checks("https://www.facebook.com/groups/12345");
        assert!(!r3.findings.iter().any(|f| f.kind == "url_brand_subdomain"));
    }
}

// ---------- INJINI YA FACEBOOK (mantiki halisi ya jukwaa) ----------

pub struct FacebookEngine;

/// Majina ya kuiga (lookalike) ya Facebook kwenye host.
const FB_LOOKALIKE: &[&str] = &["faceb00k", "face-book", "facebook-login", "fb-verify", "facebook-secure", "facebk"];

impl PlatformEngine for FacebookEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }

    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "facebook", &mut r);
        for l in FB_LOOKALIKE {
            if low.contains(l) {
                r.hit("fb_lookalike_domain", 70, format!("{url}: domain inaiga Facebook ('{l}')"));
            }
        }
        if low.contains("fb.me") && !low.contains("https://fb.me") {
            r.hit("fb_shortener", 25, format!("{url}: kiungo kifupi cha Facebook"));
        }
        if low.contains("/checkpoint") || low.contains("/confirm") {
            r.hit("fb_checkpoint_phish", 60, format!("{url}: ukurasa wa uthibitisho unaoigwa"));
        }
        r
    }

    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        // Ulaghai wa Marketplace / Shop
        let pays = ["tuma pesa", "lipa kwanza", "deposit", "m-pesa", "mpesa", "malipo ya awali", "toa pesa"];
        let cheap = ["bei rahisi", "bei poa", "punguzo", "offer", "flash sale", "last price"];
        if pays.iter().any(|w| low.contains(w)) && cheap.iter().any(|w| low.contains(w)) {
            r.hit("fb_marketplace_scam", 75, "Mbinu ya ulaghai wa Marketplace (pesa + bei ya kupotosha)");
        }
        // Matangazo bandia ya kazi
        if (low.contains("kazi") || low.contains("ajira") || low.contains("job"))
            && (low.contains("usajili") || low.contains("registration fee") || low.contains("malipo"))
        {
            r.hit("fb_job_scam", 70, "Tangazo la kazi linaomba malipo ya usajili");
        }
        // Uchangishaji wa fedha (fundraising) wa kutiliwa shaka
        if (low.contains("changisha") || low.contains("mchango") || low.contains("donate"))
            && pays.iter().any(|w| low.contains(w))
        {
            r.hit("fb_fundraising_susp", 65, "Kampeni ya kuchangisha inahitaji uthibitisho");
        }
        // Namba za simu za TZ kwenye ujumbe wa biashara
        let phones = platform::tz_phone_numbers(text);
        if !phones.is_empty() && (low.contains("tuma") || low.contains("lipa") || low.contains("whatsapp")) {
            r.hit("fb_phone_payment", 55, format!("Namba za malipo: {}", phones.join(", ")));
        }
        for s in platform::urgency_signals(text) {
            r.hit("fb_social_eng", 40, format!("Mbinu ya ushawishi: {s}"));
        }
        r
    }

    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("new_device").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.hit("fb_new_device", 50, "Kuingia kutoka kifaa kipya — uthibitisho unahitajika");
        }
        if let Some(logins) = params.get("logins").and_then(|v| v.as_array()) {
            if logins.len() >= 2 {
                let a = &logins[0];
                let b = &logins[1];
                let (la, lo) = (a["lat"].as_f64().unwrap_or(0.0), a["lon"].as_f64().unwrap_or(0.0));
                let (lb, lob) = (b["lat"].as_f64().unwrap_or(0.0), b["lon"].as_f64().unwrap_or(0.0));
                let km = platform::haversine_km(la, lo, lb, lob);
                let dt = (b["ts"].as_i64().unwrap_or(0) - a["ts"].as_i64().unwrap_or(0)).abs();
                let v = platform::speed_kmh(km, dt);
                if v > 900.0 {
                    r.hit("fb_impossible_travel", 85,
                        format!("Safari isiyowezekana: {km:.0} km kwa {dt}s ({v:.0} km/h)"));
                }
            }
        }
        r
    }
}

pub fn engine() -> FacebookEngine { FacebookEngine }
