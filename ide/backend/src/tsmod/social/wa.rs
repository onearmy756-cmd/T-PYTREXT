//! wa.rs — WhatsApp security modules (Modules 113-137).
//!
//! Jukwaa: WhatsApp. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "whatsapp";
pub const NAME: &str = "WhatsApp";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 113, last: 137 };

/// Modules zote za WhatsApp.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 113,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya WhatsApp, inazuia uhamisho usioidhinishwa",
            twist: "Inahitaji fingerprint kabla ya kuhamisha akaunti",
        },
        SocialMod {
            id: 114,
            platform: PLATFORM,
            name: "Message Encryption (Double)",
            kazi: "Inaficha ujumbe mara mbili (WhatsApp + SIM)",
            twist: "WhatsApp + SIM double encryption",
        },
        SocialMod {
            id: 115,
            platform: PLATFORM,
            name: "Group Security",
            kazi: "Inachambua vikundi kabla ya kujiunga",
            twist: "Inagundua vikundi vya Ponzi na crypto scams",
        },
        SocialMod {
            id: 116,
            platform: PLATFORM,
            name: "Status Privacy",
            kazi: "Inadhibiti nani anaweza kuona Status",
            twist: "Inazuia watu kwenye blacklist kuona Status",
        },
        SocialMod {
            id: 117,
            platform: PLATFORM,
            name: "Call Security",
            kazi: "Inalinda simu za sauti kwenye WhatsApp",
            twist: "Inaficha sauti",
        },
        SocialMod {
            id: 118,
            platform: PLATFORM,
            name: "Video Call Protection",
            kazi: "Inalinda simu za video kwenye WhatsApp",
            twist: "Inaficha video",
        },
        SocialMod {
            id: 119,
            platform: PLATFORM,
            name: "Voice Call Protection",
            kazi: "Inalinda simu za sauti za WhatsApp",
            twist: "Inaficha sauti",
        },
        SocialMod {
            id: 120,
            platform: PLATFORM,
            name: "Backup Encryption",
            kazi: "Inaficha backup ya WhatsApp kwenye Google Drive/iCloud",
            twist: "Inaficha backup kabla ya kuweka cloud",
        },
        SocialMod {
            id: 121,
            platform: PLATFORM,
            name: "Cloud Backup Security",
            kazi: "Inalinda backup kwenye cloud",
            twist: "Inathibitisha backup ni halali",
        },
        SocialMod {
            id: 122,
            platform: PLATFORM,
            name: "WhatsApp Web Security",
            kazi: "Inafuatilia vikao vya WhatsApp Web",
            twist: "Inazuia vikao kutoka kifaa kipya bila uthibitisho",
        },
        SocialMod {
            id: 123,
            platform: PLATFORM,
            name: "Desktop Security",
            kazi: "Inalinda WhatsApp Desktop",
            twist: "Inafuatilia vikao vya desktop",
        },
        SocialMod {
            id: 124,
            platform: PLATFORM,
            name: "Business Security",
            kazi: "Inalinda akaunti za WhatsApp Business",
            twist: "Inalinda mawasiliano ya biashara",
        },
        SocialMod {
            id: 125,
            platform: PLATFORM,
            name: "Broadcast List Security",
            kazi: "Inachambua orodha za matangazo",
            twist: "Inazuia matangazo ya ulaghai",
        },
        SocialMod {
            id: 126,
            platform: PLATFORM,
            name: "Forwarded Message Detection",
            kazi: "Inaweka alama kwa ujumbe uliosambazwa mara nyingi",
            twist: "Inaweka alama \\\"Forwarded many times\\\"",
        },
        SocialMod {
            id: 127,
            platform: PLATFORM,
            name: "Spam Protection",
            kazi: "Inazuia ujumbe wa spam",
            twist: "Inazuia ujumbe kutoka namba za spam",
        },
        SocialMod {
            id: 128,
            platform: PLATFORM,
            name: "Scam Detection",
            kazi: "Inagundua ulaghai kwenye ujumbe",
            twist: "Inagundua maneno ya ulaghai (\\\"tuma pesa\\\", \\\"OTP\\\")",
        },
        SocialMod {
            id: 129,
            platform: PLATFORM,
            name: "Phishing Protection",
            kazi: "Inalinda dhidi ya ujumbe bandia",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 130,
            platform: PLATFORM,
            name: "Malware Protection",
            kazi: "Inagundua na kuzuia viungo vya malware",
            twist: "Inazuia viungo vinavyopelekea malware",
        },
        SocialMod {
            id: 131,
            platform: PLATFORM,
            name: "Sticker Security",
            kazi: "Inachambua stickers za hatari",
            twist: "Inazuia stickers zenye matusi",
        },
        SocialMod {
            id: 132,
            platform: PLATFORM,
            name: "Document Sharing Security",
            kazi: "Inachambua hati zinazoshirikiwa",
            twist: "Inachambua hati kwa virus",
        },
        SocialMod {
            id: 133,
            platform: PLATFORM,
            name: "Location Sharing Safety",
            kazi: "Inaonyesha warning kabla ya kushiriki location",
            twist: "Inaonya \\\"Je, una uhakika unataka kushiriki location?\\\"",
        },
        SocialMod {
            id: 134,
            platform: PLATFORM,
            name: "Contact Privacy",
            kazi: "Inadhibiti nani anaweza kuona mawasiliano yako",
            twist: "Inazuia wengine kuona mawasiliano yako",
        },
        SocialMod {
            id: 135,
            platform: PLATFORM,
            name: "Block List Management",
            kazi: "Inasimamia orodha ya watu waliozuiwa",
            twist: "Inaongeza automatically namba za ulaghai",
        },
        SocialMod {
            id: 136,
            platform: PLATFORM,
            name: "Report Feature",
            kazi: "Inaripoti watumiaji na vikundi hatari kwa WhatsApp",
            twist: "Inaripoti automatically kwa WhatsApp",
        },
        SocialMod {
            id: 137,
            platform: PLATFORM,
            name: "Two-Step Verification",
            kazi: "Inaweka na kudhibiti PIN ya WhatsApp",
            twist: "Inahitaji PIN + fingerprint",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wa_ina_modules_25() {
        assert_eq!(modules().len(), 25);
        assert_eq!(modules()[0].id, 113);
        assert_eq!(modules()[24].id, 137);
    }

    #[test]
    fn wa_engine_inagundua_simswap_na_otp() {
        let e = engine();
        let r = e.text_checks("Nimekutumia code ya uthibitisho, nipe OTP yako haraka");
        assert!(r.findings.iter().any(|f| f.kind == "wa_otp_request"));
        let r2 = e.text_checks("Mama, hii ni namba yangu mpya tuma 0712345678 pesa");
        assert!(r2.findings.iter().any(|f| f.kind == "wa_sim_swap"));
        let r3 = e.text_checks("Hongera! Umeshinda zawadi, lipa ada ya usajili kwa M-Pesa");
        assert!(r3.findings.iter().any(|f| f.kind == "wa_scam"));
    }
}

// ---------- INJINI YA WHATSAPP ----------

pub struct WhatsAppEngine;

const WA_LOOKALIKE: &[&str] = &["whatsapp-login", "whatsapp-verify", "whatsap-web", "whatsapp-secure", "wa-verify", "web-whatsapp"];

impl PlatformEngine for WhatsAppEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }

    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "whatsapp", &mut r);
        for l in WA_LOOKALIKE {
            if low.contains(l) {
                r.hit("wa_lookalike_domain", 75, format!("{url}: domain inaiga WhatsApp ('{l}')"));
            }
        }
        // "chat.whatsapp.com" ni halali; vingine vyenye whatsapp-web ni hatari.
        if low.contains("chat.whatsapp.com") {
            r.note(format!("{url}: mwaliko halali wa kikundi"));
        }
        r
    }

    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        // Ombi la OTP/code — classic SIM-swap/account takeover.
        if (low.contains("otp") || low.contains("code ya uthibitisho") || low.contains("verification code")
            || low.contains("code ya sita") || low.contains("6-digit"))
            && (low.contains("nipe") || low.contains("tuma") || low.contains("send") || low.contains("share"))
        {
            r.hit("wa_otp_request", 90, "Ombi la OTP/code ya uthibitisho — SIM-swap/account takeover");
        }
        // Namba mpya / simu imeibiwa.
        if (low.contains("namba yangu mpya") || low.contains("new number") || low.contains("simu yangu imeibiwa")
            || low.contains("my phone was stolen") || low.contains("namba imebadilika"))
            && (low.contains("tuma") || low.contains("pesa") || low.contains("money") || low.contains("send"))
        {
            r.hit("wa_sim_swap", 88, "Mbinu ya 'namba mpya' + ombi la pesa (SIM-swap/uhadaa)");
        }
        // Ulaghai wa zawadi/ada.
        let scam = ["umeshinda", "you won", "zawadi", "ada ya usajili", "activation fee", "lipa"];
        if scam.iter().any(|w| low.contains(w))
            && (low.contains("m-pesa") || low.contains("mpesa") || low.contains("tuma") || low.contains("lipa"))
        {
            r.hit("wa_scam", 75, "Ulaghai wa zawadi/ada kwa M-Pesa");
        }
        // Ujumbe uliosambazwa (forwarded) mara nyingi.
        if low.contains("forwarded") || low.contains("imepelekwa") || low.contains("share to 10") {
            r.hit("wa_chain_message", 35, "Ujumbe wa mnyororo/forwarded");
        }
        // Viungo vya kikundi vya kutiliwa shaka.
        if low.contains("chat.whatsapp.com/") && (low.contains("join") || low.contains("jiunge") || low.contains("fast money")) {
            r.hit("wa_group_invite_susp", 50, "Mwaliko wa kikundi wa kutiliwa shaka");
        }
        // Malware/APK.
        if low.contains(".apk") || low.contains("download app") || low.contains("pakua app") {
            r.hit("wa_malware_apk", 80, "Kiungo cha APK/app — hatari ya malware");
        }
        // Ujumbe unaotaka kubadilisha mipangilio.
        if (low.contains("two-step") || low.contains("pin")) && (low.contains("weka") || low.contains("set") || low.contains("badilisha")) {
            r.hit("wa_config_change", 60, "Ombi la kubadilisha two-step/PIN");
        }
        let phones = platform::tz_phone_numbers(text);
        if !phones.is_empty() && (low.contains("tuma") || low.contains("lipa")) {
            r.hit("wa_phone_payment", 55, format!("Namba za malipo: {}", phones.join(", ")));
        }
        for s in platform::urgency_signals(text) {
            r.hit("wa_social_eng", 40, format!("Mbinu ya ushawishi: {s}"));
        }
        r
    }

    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        // Ufungaji upya wa WhatsApp (re-registration) ni ishara ya SIM-swap.
        if params.get("reregistration").and_then(|v| v.as_bool()).unwrap_or(false) {
            r.hit("wa_reregistration", 70, "Usajili upya wa WhatsApp — hatari ya SIM-swap");
        }
        if params.get("new_device").and_then(|v| v.as_bool()).unwrap_or(false)
            && params.get("whatsapp_web").and_then(|v| v.as_bool()).unwrap_or(false)
        {
            r.hit("wa_web_login", 55, "WhatsApp Web imeunganishwa kwenye kifaa kipya");
        }
        if !params.get("has_key").and_then(|v| v.as_bool()).unwrap_or(true) {
            r.hit("wa_backup_no_key", 50, "Backup haina ufunguo wa SIM (usimbaji wa pili)");
        }
        r
    }
}

pub fn engine() -> WhatsAppEngine { WhatsAppEngine }
