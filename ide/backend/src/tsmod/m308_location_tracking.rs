//! m308_location_tracking.rs — MODULE 308: LOCATION TRACKING
//!
//! Spec: "Inafuatilia location ya kifaa kwa GPS. Twist: Inaweza kutuma location kwa polisi."
//!
//! Injini halisi: hufuatilia mfuatano wa location (GPS) na kutambua safari/mwendo;
//! ikiwa kifaa kimeripotiwa kuibwa, inaweza kutuma location ya mwisho kwa polisi
//! (kwa idhini/HITL — hii ni module yenye uwezo wa dharura).

use super::common_device::haversine_km;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Pointi moja ya location.
#[derive(Debug, Clone)]
pub struct Loc {
    pub lat: f64,
    pub lon: f64,
    pub ts: i64,
}

/// Jumla ya umbali wa mfuatano (km).
pub fn track_distance(pts: &[Loc]) -> f64 {
    pts.windows(2).map(|w| haversine_km(w[0].lat, w[0].lon, w[1].lat, w[1].lon)).sum()
}

/// Kasi ya wastani (km/h).
pub fn avg_speed(pts: &[Loc]) -> f64 {
    if pts.len() < 2 {
        return 0.0;
    }
    let dt = (pts.last().unwrap().ts - pts[0].ts).abs() as f64 / 3600.0;
    if dt <= 0.0 {
        return 0.0;
    }
    track_distance(pts) / dt
}

pub struct M308LocationTracking;

impl TsModule for M308LocationTracking {
    fn id(&self) -> u16 {
        308
    }
    fn name(&self) -> &'static str {
        "Location Tracking"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hufuatilia location ya kifaa (GPS) na mwendo; kifaa kilichoibwa kinaweza \
         kutuma location ya mwisho kwa polisi (HITL)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(308, self.name());
        let pts: Vec<Loc> = p.get("points").and_then(|v| v.as_array()).map(|a| {
            a.iter().filter_map(|x| {
                Some(Loc {
                    lat: x.get("lat")?.as_f64()?,
                    lon: x.get("lon")?.as_f64()?,
                    ts: x.get("ts").and_then(|v| v.as_i64()).unwrap_or(0),
                })
            }).collect()
        }).unwrap_or_default();
        let stolen = p.get("stolen").and_then(|v| v.as_bool()).unwrap_or(false);
        let dist = track_distance(&pts);
        let spd = avg_speed(&pts);
        r.findings.push(json!({ "kind": "track", "severity": 1,
            "detail": { "points": pts.len(), "distance_km": format!("{dist:.3}"), "avg_speed_kmh": format!("{spd:.1}") } }));
        if let Some(last) = pts.last() {
            r.findings.push(json!({ "kind": "last_location", "severity": 1,
                "detail": { "lat": last.lat, "lon": last.lon, "ts": last.ts } }));
        }
        if stolen {
            r.findings.push(json!({ "kind": "police_alert", "severity": 90,
                "detail": { "action": "tuma location ya mwisho kwa polisi (HITL)", "dispatch": true } }));
        }
        r.summary = format!("Location Tracking: pointi {}, km {dist:.3}, kasi {spd:.1} km/h{}.", pts.len(), if stolen { ", POLISI" } else { "" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umbali_na_kasi() {
        let pts = vec![
            Loc { lat: -6.8, lon: 39.2, ts: 0 },
            Loc { lat: -6.9, lon: 39.3, ts: 3600 },
        ];
        let d = track_distance(&pts);
        assert!(d > 10.0 && d < 20.0, "d={d}");
        assert!(avg_speed(&pts) > 10.0);
    }
}
