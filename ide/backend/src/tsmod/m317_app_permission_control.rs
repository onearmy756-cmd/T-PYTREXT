//! m317_app_permission_control.rs — MODULE 317: APP PERMISSION CONTROL
//!
//! Spec: "Inadhibiti ni app gani zina ruhusa gani. Twist: Inazuia apps za spyware."
//!
//! Injini halisi: hukagua ruhusa za app (camera, mic, location, contacts, SMS,
//! storage, accessibility) na kutambua michanganyiko ya spyware:
//!   - Recorder: mic + storage + internet bila UI.
//!   - Tracker: location + contacts + internet (background).
//!   - Accessibility abuse (inasoma skrini).

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Ruhusa za app.
#[derive(Debug, Clone, Default)]
pub struct Perms {
    pub camera: bool,
    pub mic: bool,
    pub location: bool,
    pub contacts: bool,
    pub sms: bool,
    pub storage: bool,
    pub internet: bool,
    pub accessibility: bool,
    pub background: bool,
    pub has_ui: bool,
}

/// Michanganyiko ya spyware.
pub fn spyware_score(p: &Perms) -> (u8, Vec<String>) {
    let mut score = 0u8;
    let mut why = Vec::new();
    if p.mic && p.storage && p.internet && !p.has_ui {
        score = score.max(90);
        why.push("Recorder: mic+storage+internet bila UI".into());
    }
    if p.location && p.contacts && p.internet && p.background {
        score = score.max(75);
        why.push("Tracker: location+contacts+internet nyuma".into());
    }
    if p.accessibility && p.internet {
        score = score.max(85);
        why.push("Accessibility abuse: inaweza kusoma skrini".into());
    }
    if p.sms && p.internet {
        score = score.max(80);
        why.push("SMS+internet: uwezekano wa kuingilia OTP".into());
    }
    (score, why)
}

pub struct M317AppPermissionControl;

impl TsModule for M317AppPermissionControl {
    fn id(&self) -> u16 {
        317
    }
    fn name(&self) -> &'static str {
        "App Permission Control"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hukagua ruhusa za app na kutambua michanganyiko ya spyware (recorder, \
         tracker, accessibility abuse, SMS interception)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(317, self.name());
        let perms = Perms {
            camera: p.get("camera").and_then(|v| v.as_bool()).unwrap_or(false),
            mic: p.get("mic").and_then(|v| v.as_bool()).unwrap_or(false),
            location: p.get("location").and_then(|v| v.as_bool()).unwrap_or(false),
            contacts: p.get("contacts").and_then(|v| v.as_bool()).unwrap_or(false),
            sms: p.get("sms").and_then(|v| v.as_bool()).unwrap_or(false),
            storage: p.get("storage").and_then(|v| v.as_bool()).unwrap_or(false),
            internet: p.get("internet").and_then(|v| v.as_bool()).unwrap_or(true),
            accessibility: p.get("accessibility").and_then(|v| v.as_bool()).unwrap_or(false),
            background: p.get("background").and_then(|v| v.as_bool()).unwrap_or(false),
            has_ui: p.get("has_ui").and_then(|v| v.as_bool()).unwrap_or(true),
        };
        let (score, why) = spyware_score(&perms);
        for w in &why {
            r.finding("spyware_pattern", w.clone(), score);
        }
        let action = if score >= 80 { "zuia app" } else if score >= 60 { "onya + dhibiti ruhusa" } else { "ruhusu" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "score": score, "action": action } }));
        r.summary = format!("App Permission Control: score {score} → {action}.");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spyware_recorder_inagundulika() {
        let p = Perms { mic: true, storage: true, internet: true, has_ui: false, ..Default::default() };
        let (s, why) = spyware_score(&p);
        assert!(s >= 80);
        assert!(why.iter().any(|w| w.contains("Recorder")));
    }
}
