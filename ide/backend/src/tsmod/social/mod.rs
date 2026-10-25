//! social/mod.rs — SEHEMU YA 13: SOCIAL MEDIA SECURITY (Modules 58-272).
//!
//! Faili moja kwa kila jukwaa (Facebook, Instagram, WhatsApp, Twitter/X, TikTok,
//! Snapchat, LinkedIn, Telegram, Signal, Discord, Reddit, YouTube). Kila
//! jukwaa lina module zake halisi zinazotumia injini ya HIFADHIDATA
//! (`super::store` + `engine`): sheria zinasomwa kutoka SQLite, hujifunza
//! kutoka feedback, na IOC zinashirikiwa kwenye majukwaa yote.

pub mod engine;
pub mod ai;
pub mod platform;
pub mod fb;
pub mod ig;
pub mod wa;
pub mod tw;
pub mod tt;
pub mod sc;
pub mod li;
pub mod tg;
pub mod sg;
pub mod dc;
pub mod rd;
pub mod yt;
pub mod runner;

use super::{ModReport, TsModule};
use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::OnceLock;

/// Jukwaa la kijamii.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub key: &'static str,
    pub name: &'static str,
    pub first: u16,
    pub last: u16,
}

pub const PLATFORMS: &[Platform] = &[
    Platform { key: "facebook", name: "Facebook", first: 58, last: 87 },
    Platform { key: "instagram", name: "Instagram", first: 88, last: 112 },
    Platform { key: "whatsapp", name: "WhatsApp", first: 113, last: 137 },
    Platform { key: "twitterx", name: "TwitterX", first: 138, last: 157 },
    Platform { key: "tiktok", name: "TikTok", first: 158, last: 177 },
    Platform { key: "snapchat", name: "Snapchat", first: 178, last: 192 },
    Platform { key: "linkedin", name: "LinkedIn", first: 193, last: 207 },
    Platform { key: "telegram", name: "Telegram", first: 208, last: 222 },
    Platform { key: "signal", name: "Signal", first: 223, last: 232 },
    Platform { key: "discord", name: "Discord", first: 233, last: 247 },
    Platform { key: "reddit", name: "Reddit", first: 248, last: 257 },
    Platform { key: "youtube", name: "YouTube", first: 258, last: 272 },
];

/// Jukwaa la module kwa id.
pub fn platform_of(id: u16) -> Option<Platform> {
    PLATFORMS.iter().copied().find(|p| id >= p.first && id <= p.last)
}

/// Kundi la module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cat {
    Content,
    Link,
    Account,
    Privacy,
    Fraud,
    Encryption,
    Bot,
    Moderation,
}

impl Cat {
    pub fn key(self) -> &'static str {
        match self {
            Cat::Content => "content",
            Cat::Link => "link",
            Cat::Account => "account",
            Cat::Privacy => "privacy",
            Cat::Fraud => "fraud",
            Cat::Encryption => "encryption",
            Cat::Bot => "bot",
            Cat::Moderation => "moderation",
        }
    }
}

/// Ainisha module kwa jina (spec: kazi + twist).
pub fn category(name: &str) -> Cat {
    let n = name.to_lowercase();
    if n.contains("encryption") || n.contains("secret") || n.contains("cloud") || n.contains("backup") {
        return Cat::Encryption;
    }
    if n.contains("phishing") || n.contains("link") || n.contains("qr") || n.contains("snapcode")
        || n.contains("webhook") || n.contains("inmail") || n.contains("newsletter")
    {
        return Cat::Link;
    }
    if n.contains("account") || n.contains("takeover") || n.contains("login") || n.contains("pin")
        || n.contains("two-step") || n.contains("registration") || n.contains("verification")
    {
        return Cat::Account;
    }
    if n.contains("privacy") || n.contains("story") || n.contains("audience") || n.contains("list")
        || n.contains("circle") || n.contains("folder") || n.contains("bookmark") || n.contains("history")
        || n.contains("map") || n.contains("close friends") || n.contains("profile")
    {
        return Cat::Privacy;
    }
    if n.contains("scam") || n.contains("fraud") || n.contains("marketplace") || n.contains("shop")
        || n.contains("commerce") || n.contains("job") || n.contains("recruiter") || n.contains("fundrais")
        || n.contains("crowdfund") || n.contains("ads") || n.contains("promotion") || n.contains("tip jar")
        || n.contains("super follow") || n.contains("premium") || n.contains("payment")
    {
        return Cat::Fraud;
    }
    if n.contains("bot") || n.contains("follower") || n.contains("avatar") || n.contains("fake")
        || n.contains("karma") || n.contains("subscription") || n.contains("poll") || n.contains("quiz")
    {
        return Cat::Bot;
    }
    if n.contains("comment") || n.contains("filter") || n.contains("mention") || n.contains("chat")
        || n.contains("dm") || n.contains("message") || n.contains("post") || n.contains("live")
        || n.contains("quote") || n.contains("reel") || n.contains("watch") || n.contains("video")
        || n.contains("snap") || n.contains("spotlight") || n.contains("discover") || n.contains("sound")
    {
        return Cat::Content;
    }
    Cat::Moderation
}

/// Module moja ya jukwaa.
#[derive(Clone, Copy)]
pub struct SocialMod {
    pub id: u16,
    pub platform: Platform,
    pub name: &'static str,
    pub kazi: &'static str,
    pub twist: &'static str,
}

impl SocialMod {
    pub fn cat(&self) -> Cat {
        category(self.name)
    }
}

#[async_trait]
impl TsModule for SocialMod {
    fn id(&self) -> u16 {
        self.id
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn group(&self) -> &'static str {
        "social"
    }
    fn about(&self) -> &'static str {
        self.kazi
    }
    fn run(&self, _target: &str, _params: &Value) -> Result<ModReport, String> {
        Err("module hii ni async — tumia run_db (handler ya HTTP huiita run_db)".into())
    }
    async fn run_db(&self, db: &SqlitePool, target: &str, params: &Value) -> Result<ModReport, String> {
        runner::run(self, db, target, params).await
    }
}

/// Rejista ya modules zote za kijamii (58..272).
pub fn all() -> &'static [SocialMod] {
    static V: OnceLock<Vec<SocialMod>> = OnceLock::new();
    V.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(fb::modules());
        v.extend(ig::modules());
        v.extend(wa::modules());
        v.extend(tw::modules());
        v.extend(tt::modules());
        v.extend(sc::modules());
        v.extend(li::modules());
        v.extend(tg::modules());
        v.extend(sg::modules());
        v.extend(dc::modules());
        v.extend(rd::modules());
        v.extend(yt::modules());
        v.sort_by_key(|m| m.id);
        v
    })
}

/// Tafuta module kwa id.
pub fn find(id: u16) -> Option<SocialMod> {
    all().iter().copied().find(|m| m.id == id)
}

/// Injini HALISI ya jukwaa (code ya kila jukwaa) — sio catalog.
pub fn engine_for(key: &str) -> Option<Box<dyn platform::PlatformEngine>> {
    match key {
        "facebook" => Some(Box::new(fb::engine())),
        "instagram" => Some(Box::new(ig::engine())),
        "whatsapp" => Some(Box::new(wa::engine())),
        "twitterx" => Some(Box::new(tw::engine())),
        "tiktok" => Some(Box::new(tt::engine())),
        "snapchat" => Some(Box::new(sc::engine())),
        "linkedin" => Some(Box::new(li::engine())),
        "telegram" => Some(Box::new(tg::engine())),
        "signal" => Some(Box::new(sg::engine())),
        "discord" => Some(Box::new(dc::engine())),
        "reddit" => Some(Box::new(rd::engine())),
        "youtube" => Some(Box::new(yt::engine())),
        _ => None,
    }
}

/// Catalog ya modules za kijamii.
pub fn catalog() -> Vec<Value> {
    all()
        .iter()
        .map(|m| {
            json!({
                "module": m.id, "name": m.name, "group": "social",
                "platform": m.platform.name, "platform_key": m.platform.key,
                "category": m.cat().key(), "about": m.kazi, "twist": m.twist,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_zote_215_zipo() {
        assert_eq!(all().len(), 215);
        assert_eq!(all()[0].id, 58);
        assert_eq!(all()[214].id, 272);
        // kila id 58..=272 ipo
        for id in 58..=272u16 {
            assert!(find(id).is_some(), "module {id} haipo");
        }
    }

    #[test]
    fn platform_lookup() {
        assert_eq!(platform_of(58).unwrap().name, "Facebook");
        assert_eq!(platform_of(272).unwrap().name, "YouTube");
        assert!(platform_of(57).is_none());
        assert!(platform_of(273).is_none());
    }

    #[test]
    fn jukwaa_la_kila_module_linapatikana() {
        for m in all() {
            assert!(m.id >= m.platform.first && m.id <= m.platform.last);
        }
    }
}
