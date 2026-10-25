//! li.rs — LinkedIn security modules (Modules 193-207).
//!
//! Jukwaa: LinkedIn. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "linkedin";
pub const NAME: &str = "LinkedIn";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 193, last: 207 };

/// Modules zote za LinkedIn.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 193,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya LinkedIn",
            twist: "Inagundua kuingia kutoka location mpya",
        },
        SocialMod {
            id: 194,
            platform: PLATFORM,
            name: "Job Scam Detection",
            kazi: "Inagundua matangazo bandia ya kazi",
            twist: "Inaonya \\\"Hii ni kazi bandia\\\"",
        },
        SocialMod {
            id: 195,
            platform: PLATFORM,
            name: "Profile Privacy",
            kazi: "Inadhibiti nani anaweza kuona profile",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 196,
            platform: PLATFORM,
            name: "Connection Analysis",
            kazi: "Inachambua maombi ya kuunganisha",
            twist: "Inaonya ikiwa mtumaji ni bandia",
        },
        SocialMod {
            id: 197,
            platform: PLATFORM,
            name: "Recruiter Verification",
            kazi: "Inathibitisha kama recruiters ni halali",
            twist: "Inathibitisha recruiter ni halali",
        },
        SocialMod {
            id: 198,
            platform: PLATFORM,
            name: "InMail Security",
            kazi: "Inachambua InMail kwa viungo hatari",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 199,
            platform: PLATFORM,
            name: "Company Page Security",
            kazi: "Inalinda kurasa za makampuni",
            twist: "Inathibitisha ukurasa ni halali",
        },
        SocialMod {
            id: 200,
            platform: PLATFORM,
            name: "Sales Navigator Security",
            kazi: "Inalinda watumiaji wa Sales Navigator",
            twist: "Inalinda data za mauzo",
        },
        SocialMod {
            id: 201,
            platform: PLATFORM,
            name: "Talent Hub Security",
            kazi: "Inalinda watafuta wafanyakazi",
            twist: "Inathibitisha waombaji kazi",
        },
        SocialMod {
            id: 202,
            platform: PLATFORM,
            name: "Learning Security",
            kazi: "Inalinda kozi na cheti kwenye LinkedIn Learning",
            twist: "Inathibitisha vyeti",
        },
        SocialMod {
            id: 203,
            platform: PLATFORM,
            name: "Ads Security",
            kazi: "Inachambua matangazo yenye ulaghai",
            twist: "Inazuia matangazo ya ulaghai",
        },
        SocialMod {
            id: 204,
            platform: PLATFORM,
            name: "Analytics Security",
            kazi: "Inalinda data za analytics za makampuni",
            twist: "Inazuia wengine kuona analytics",
        },
        SocialMod {
            id: 205,
            platform: PLATFORM,
            name: "Publisher Security",
            kazi: "Inachambua makala zilizochapishwa",
            twist: "Inazuia makala za ulaghai",
        },
        SocialMod {
            id: 206,
            platform: PLATFORM,
            name: "Groups Security",
            kazi: "Inalinda vikundi vya LinkedIn",
            twist: "Inazuia watu wasiofaa kujiunga",
        },
        SocialMod {
            id: 207,
            platform: PLATFORM,
            name: "Events Security",
            kazi: "Inalinda matukio yaliyopangwa kwenye LinkedIn",
            twist: "Inathibitisha matukio ni halali",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn li_ina_modules_15() {
        assert_eq!(modules().len(), 15);
        assert_eq!(modules()[0].id, 193);
        assert_eq!(modules()[14].id, 207);
    }
}

// ---------- INJINI YA LINKEDIN ----------
pub struct LinkedInEngine;
const LI_LOOKALIKE: &[&str] = &["linkedin-login", "linked-in", "linkedin-verify", "linkedin-careers", "linkedin-secure"];
impl PlatformEngine for LinkedInEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "linkedin", &mut r);
        for l in LI_LOOKALIKE { if low.contains(l) { r.hit("li_lookalike_domain", 70, format!("{url}: domain inaiga LinkedIn ('{l}')")); } }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        // Kazi bandia zinazoomba malipo — tatizo kubwa la LinkedIn.
        if (low.contains("kazi") || low.contains("job") || low.contains("position") || low.contains("nafasi"))
            && (low.contains("registration fee") || low.contains("ada") || low.contains("malipo") || low.contains("training fee") || low.contains("pay")) {
            r.hit("li_job_scam", 80, "Tangazo la kazi linaomba malipo — ulaghai");
        }
        if (low.contains("recruiter") || low.contains("recruitment"))
            && (low.contains("whatsapp") || low.contains("telegram") || low.contains("personal email") || low.contains("gmail")) {
            r.hit("li_recruiter_offplatform", 60, "Recruiter anahamisha mazungumzo nje ya LinkedIn");
        }
        if low.contains("inmail") && (low.contains("verify") || low.contains("link") || low.contains("account")) {
            r.hit("li_inmail_phish", 65, "InMail yenye kiungo cha uthibitisho");
        }
        if low.contains("view my profile") && low.contains("http") && !low.contains("linkedin.com") {
            r.hit("li_profile_phish", 55, "Kiungo cha 'view profile' kisicho cha LinkedIn");
        }
        for s in platform::urgency_signals(text) { r.hit("li_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        let conns = params.get("connections").and_then(|v| v.as_u64()).unwrap_or(0);
        let age = params.get("account_age_days").and_then(|v| v.as_i64()).unwrap_or(999);
        if age < 30 && conns < 5 && params.get("recruiter").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.hit("li_fake_recruiter", 70, "Akaunti mpya inajifanya recruiter — uwezekano wa bandia");
        }
        r
    }
}
pub fn engine() -> LinkedInEngine { LinkedInEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn li_inagundua_job_scam() {
        let e = engine();
        assert!(e.text_checks("Nafasi ya kazi! Lipa ada ya usajili TZS 50,000").findings.iter().any(|f| f.kind == "li_job_scam"));
        assert!(e.text_checks("Recruiter here, contact me on WhatsApp for the job").findings.iter().any(|f| f.kind == "li_recruiter_offplatform"));
        assert!(e.url_checks("http://linkedin-verify.com").findings.iter().any(|f| f.kind == "li_lookalike_domain"));
    }
}
