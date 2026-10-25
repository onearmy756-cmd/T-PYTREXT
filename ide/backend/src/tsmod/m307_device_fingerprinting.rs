//! m307_device_fingerprinting.rs — MODULE 307: DEVICE FINGERPRINTING
//!
//! Spec: "Inachukua alama za kipekee za SIM na kifaa. Twist: Inachukua IMEI, MAC, OS, screen, fonts."
//!
//! Injini halisi: hukusanya sifa (IMEI, MAC/OUI, OS, screen, fonts) na kutengeneza
//! fingerprint ya kipekee (SHA-256); hulinganisha na fingerprint ya DB kugundua
//! kifaa kilichobadilishwa/kibandia.

use super::common_device::{device_fingerprint, imei_valid, oui_vendor, screen_ratio_class, shannon_entropy};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Sifa za kifaa.
#[derive(Debug, Clone, Default)]
pub struct DeviceTraits {
    pub imei: String,
    pub mac: String,
    pub os: String,
    pub screen_w: u32,
    pub screen_h: u32,
    pub fonts: Vec<String>,
    pub iccid: String,
}

pub fn fingerprint_of(t: &DeviceTraits) -> String {
    device_fingerprint(&[
        &t.imei,
        &t.mac,
        &t.os,
        &format!("{}x{}", t.screen_w, t.screen_h),
        &t.fonts.join(","),
        &t.iccid,
    ])
}

pub struct M307DeviceFingerprinting;

#[async_trait::async_trait]
impl TsModule for M307DeviceFingerprinting {
    fn id(&self) -> u16 {
        307
    }
    fn name(&self) -> &'static str {
        "Device Fingerprinting"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hukusanya IMEI, MAC/OUI, OS, screen na fonts na kutengeneza fingerprint \
         ya kipekee (SHA-256); hulinganisha na DB kugundua kifaa kibandia."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(307, self.name());
        let t = traits_from(p);
        let fp = fingerprint_of(&t);
        r.finding("fingerprint", format!("Fingerprint: {fp}"), 1);
        let imei_ok = imei_valid(&t.imei);
        r.findings.push(json!({ "kind": "imei", "severity": if imei_ok { 1 } else { 70 },
            "detail": { "imei": mask(&t.imei, 4), "valid": imei_ok } }));
        let vendor = oui_vendor(&t.mac);
        r.findings.push(json!({ "kind": "mac", "severity": 1,
            "detail": { "mac": t.mac, "vendor": vendor, "oui_known": vendor.is_some() } }));
        r.findings.push(json!({ "kind": "os_screen", "severity": 1,
            "detail": { "os": t.os, "screen": format!("{}x{}", t.screen_w, t.screen_h),
                        "ratio_class": screen_ratio_class(t.screen_w, t.screen_h) } }));
        let ent = shannon_entropy(&t.fonts);
        r.findings.push(json!({ "kind": "fonts", "severity": 1,
            "detail": { "count": t.fonts.len(), "entropy_bits": format!("{ent:.2}"),
                        "note": if t.fonts.len() < 5 { "fonts chache — kifaa kibandia/uwezekano" } else { "fonts za kawaida" } } }));
        r.summary = format!("Device Fingerprinting: fp {}.", &fp[..16]);
        Ok(r)
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, p: &Value) -> Result<ModReport, String> {
        let mut rep = self.run(target, p)?;
        let fp = fingerprint_of(&traits_from(p));
        let known: Option<(i64,)> = sqlx::query_as("SELECT id FROM ts_iocs WHERE kind='device_fp' AND value=? LIMIT 1")
            .bind(&fp)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
        let seen = known.is_some();
        rep.findings.push(json!({ "kind": "db_match", "severity": if seen { 1 } else { 50 },
            "detail": { "known_device": seen, "note": if seen { "kifaa kinajulikana" } else { "kifaa kipya" } } }));
        rep.summary = format!("Device Fingerprinting: fp {}, kifaa {}.", &fp[..16], if seen { "kinajulikana" } else { "kipya" });
        Ok(rep)
    }
}

fn traits_from(p: &Value) -> DeviceTraits {
    DeviceTraits {
        imei: p.get("imei").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        mac: p.get("mac").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        os: p.get("os").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        screen_w: p.get("screen_w").and_then(|v| v.as_u64()).unwrap_or(1080) as u32,
        screen_h: p.get("screen_h").and_then(|v| v.as_u64()).unwrap_or(2340) as u32,
        fonts: p.get("fonts").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default(),
        iccid: p.get("iccid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
}

fn mask(s: &str, keep: usize) -> String {
    if s.len() <= keep {
        return "*".repeat(s.len());
    }
    format!("{}{}", "*".repeat(s.len() - keep), &s[s.len() - keep..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_inatofautiana() {
        let mut a = DeviceTraits::default();
        a.imei = "490154203237518".into();
        let mut b = a.clone();
        b.os = "iOS".into();
        assert_ne!(fingerprint_of(&a), fingerprint_of(&b));
        assert!(imei_valid(&a.imei));
    }
}
