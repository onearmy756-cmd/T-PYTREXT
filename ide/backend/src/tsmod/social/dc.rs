//! dc.rs — Discord security modules (Modules 233-247).
//!
//! Jukwaa: Discord. Kila module inatumia injini ya HIFADHIDATA
//! (`super::engine` + `crate::tsmod::store`): sheria zinasomwa kutoka SQLite,
//! hujifunza kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

use super::{Platform, SocialMod};
use super::platform::{self, PReport, PlatformEngine};
use serde_json::Value;

pub const KEY: &str = "discord";
pub const NAME: &str = "Discord";

pub const PLATFORM: Platform = Platform { key: KEY, name: NAME, first: 233, last: 247 };

/// Modules zote za Discord.
pub fn modules() -> Vec<SocialMod> {
    vec![
        SocialMod {
            id: 233,
            platform: PLATFORM,
            name: "Account Protection",
            kazi: "Inalinda akaunti ya Discord",
            twist: "Inagundua kuingia kutoka kifaa kipya",
        },
        SocialMod {
            id: 234,
            platform: PLATFORM,
            name: "Server Security",
            kazi: "Inachambua server za Discord kabla ya kujiunga",
            twist: "Inagundua server za ulaghai",
        },
        SocialMod {
            id: 235,
            platform: PLATFORM,
            name: "Channel Security",
            kazi: "Inadhibiti nani anaweza kuona na kuandika kwenye channel",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 236,
            platform: PLATFORM,
            name: "DM Protection",
            kazi: "Inachambua Direct Messages kwa viungo hatari",
            twist: "Inabonyeza viungo kwenye sandbox",
        },
        SocialMod {
            id: 237,
            platform: PLATFORM,
            name: "Voice Channel Security",
            kazi: "Inalinda voice channels",
            twist: "Inaficha sauti",
        },
        SocialMod {
            id: 238,
            platform: PLATFORM,
            name: "Video Call Security",
            kazi: "Inalinda video calls",
            twist: "Inaficha video",
        },
        SocialMod {
            id: 239,
            platform: PLATFORM,
            name: "Screen Share Security",
            kazi: "Inaonyesha warning kabla ya kushiriki skrini",
            twist: "Inaonya \\\"Je, una uhakika?\\\"",
        },
        SocialMod {
            id: 240,
            platform: PLATFORM,
            name: "Bot Security",
            kazi: "Inachambua bot za Discord",
            twist: "Inazuia bot hatari",
        },
        SocialMod {
            id: 241,
            platform: PLATFORM,
            name: "Webhook Security",
            kazi: "Inalinda webhooks dhidi ya matumizi mabaya",
            twist: "Inathibitisha webhooks",
        },
        SocialMod {
            id: 242,
            platform: PLATFORM,
            name: "Integration Security",
            kazi: "Inalinda integrations (Spotify, GitHub, n.k.)",
            twist: "Inathibitisha integrations",
        },
        SocialMod {
            id: 243,
            platform: PLATFORM,
            name: "Role Security",
            kazi: "Inadhibiti nani anaweza kutoa na kuondoa roles",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 244,
            platform: PLATFORM,
            name: "Permission Security",
            kazi: "Inahakikisha permissions hazitumwi vibaya",
            twist: "Inathibitisha permissions",
        },
        SocialMod {
            id: 245,
            platform: PLATFORM,
            name: "Audit Log Security",
            kazi: "Inalinda audit logs dhidi ya kufutwa",
            twist: "Inahifadhi audit logs kwenye blockchain",
        },
        SocialMod {
            id: 246,
            platform: PLATFORM,
            name: "Thread Security",
            kazi: "Inalinda threads kwenye Discord",
            twist: "Inazuia watu wasiofaa",
        },
        SocialMod {
            id: 247,
            platform: PLATFORM,
            name: "Stage Channel Security",
            kazi: "Inalinda stage channels (majadiliano ya sauti)",
            twist: "Inaficha sauti",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_ina_modules_15() {
        assert_eq!(modules().len(), 15);
        assert_eq!(modules()[0].id, 233);
        assert_eq!(modules()[14].id, 247);
    }
}

// ---------- INJINI YA DISCORD ----------
pub struct DiscordEngine;
const DC_LOOKALIKE: &[&str] = &["discord-login", "discord-nitro-free", "discord-verify", "discord-gift", "discordapp-verify"];
impl PlatformEngine for DiscordEngine {
    fn key(&self) -> &'static str { KEY }
    fn name(&self) -> &'static str { NAME }
    fn url_checks(&self, url: &str) -> PReport {
        let mut r = PReport::default();
        let low = url.to_lowercase();
        platform::generic_url_checks(url, "discord", &mut r);
        for l in DC_LOOKALIKE { if low.contains(l) { r.hit("dc_lookalike_domain", 75, format!("{url}: domain inaiga Discord ('{l}')")); } }
        if low.contains("discord.gift") { r.note(format!("{url}: kiungo cha zawadi cha Discord")); }
        if low.contains("discord.com/api/webhooks") { r.hit("dc_webhook_exposed", 70, "Webhook ya Discord imefichuka"); }
        r
    }
    fn text_checks(&self, text: &str) -> PReport {
        let mut r = PReport::default();
        let low = text.to_lowercase();
        if (low.contains("nitro") || low.contains("free gift") || low.contains("zawadi"))
            && (low.contains("click") || low.contains("claim") || low.contains("link") || low.contains("bofya")) {
            r.hit("dc_nitro_scam", 80, "Ulaghai wa Discord Nitro bure");
        }
        if low.contains("steam") && (low.contains("free") || low.contains("gift")) && low.contains("link") {
            r.hit("dc_steam_phish", 75, "Ulaghai wa Steam gift");
        }
        if low.contains("token") && (low.contains("send") || low.contains("tuma") || low.contains("paste")) {
            r.hit("dc_token_theft", 90, "Ombi la token ya Discord — wizi wa akaunti");
        }
        if (low.contains("mod") || low.contains("admin")) && (low.contains("apply") || low.contains("link")) {
            r.hit("dc_mod_bait", 55, "Bait ya 'kuwa mod/admin'");
        }
        if low.contains("webhook") && (low.contains("spam") || low.contains("send")) { r.hit("dc_webhook_abuse", 65, "Matumizi mabaya ya webhook"); }
        for s in platform::urgency_signals(text) { r.hit("dc_social_eng", 40, format!("Mbinu ya ushawishi: {s}")); }
        r
    }
    fn account_checks(&self, params: &Value) -> PReport {
        let mut r = PReport::default();
        if params.get("bot_score").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.7 { r.hit("dc_bot", 80, "Alama ya bot juu"); }
        if params.get("new_webhook").and_then(|v| v.as_bool()).unwrap_or(false) { r.hit("dc_new_webhook", 55, "Webhook mpya imeongezwa"); }
        r
    }
}
pub fn engine() -> DiscordEngine { DiscordEngine }
#[cfg(test)]
mod engine_tests {
    use super::*;
    #[test]
    fn dc_inagundua_nitro_na_token() {
        let e = engine();
        assert!(e.text_checks("Free Nitro! Click this link to claim").findings.iter().any(|f| f.kind == "dc_nitro_scam"));
        assert!(e.text_checks("Paste your token here to verify").findings.iter().any(|f| f.kind == "dc_token_theft"));
        assert!(e.url_checks("http://discord-nitro-free.com").findings.iter().any(|f| f.kind == "dc_lookalike_domain"));
    }
}
