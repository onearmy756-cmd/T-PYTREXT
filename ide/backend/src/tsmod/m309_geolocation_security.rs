//! m309_geolocation_security.rs — MODULE 309: GEOLOCATION SECURITY
//!
//! Spec: "Inaangalia kama location inalingana na tabia. Twist: Inazuia kuingia kutoka location isiyo ya kawaida."
//!
//! Injini halisi: hujifunza tabia ya location (home/work) na kuzuia kuingia kutoka
//! location isiyo ya kawaida au mwendo usiowezekana ("impossible travel").

use super::common_device::haversine_km;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Tabia ya location ya mtumiaji.
#[derive(Debug, Clone, Default)]
pub struct GeoProfile {
    pub home_lat: f64,
    pub home_lon: f64,
    pub known_radius_km: f64,
    pub last_lat: f64,
    pub last_lon: f64,
    pub last_ts: i64,
}

/// Impossible travel: umbali/saa zinazowezekana?
pub fn impossible_travel(prev: &GeoProfile, lat: f64, lon: f64, ts: i64, max_kmh: f64) -> (bool, f64) {
    let dt_h = (ts - prev.last_ts).abs() as f64 / 3600.0;
    if dt_h <= 0.0 {
        return (false, 0.0);
    }
    let km = haversine_km(prev.last_lat, prev.last_lon, lat, lon);
    let kmh = km / dt_h;
    (kmh > max_kmh, kmh)
}

/// Umbali kutoka home.
pub fn distance_from_home(p: &GeoProfile, lat: f64, lon: f64) -> f64 {
    haversine_km(p.home_lat, p.home_lon, lat, lon)
}

pub struct M309GeolocationSecurity;

impl TsModule for M309GeolocationSecurity {
    fn id(&self) -> u16 {
        309
    }
    fn name(&self) -> &'static str {
        "Geolocation Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hujifunza tabia ya location na kuzuia kuingia kutoka location isiyo ya \
         kawaida au mwendo usiowezekana (impossible travel)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(309, self.name());
        let prof = GeoProfile {
            home_lat: p.get("home_lat").and_then(|v| v.as_f64()).unwrap_or(-6.8),
            home_lon: p.get("home_lon").and_then(|v| v.as_f64()).unwrap_or(39.2),
            known_radius_km: p.get("known_radius_km").and_then(|v| v.as_f64()).unwrap_or(50.0),
            last_lat: p.get("last_lat").and_then(|v| v.as_f64()).unwrap_or(-6.8),
            last_lon: p.get("last_lon").and_then(|v| v.as_f64()).unwrap_or(39.2),
            last_ts: p.get("last_ts").and_then(|v| v.as_i64()).unwrap_or(0),
        };
        let lat = p.get("lat").and_then(|v| v.as_f64()).unwrap_or(-6.8);
        let lon = p.get("lon").and_then(|v| v.as_f64()).unwrap_or(39.2);
        let ts = p.get("ts").and_then(|v| v.as_i64()).unwrap_or(3600);
        let max_kmh = p.get("max_kmh").and_then(|v| v.as_f64()).unwrap_or(900.0);

        let dhome = distance_from_home(&prof, lat, lon);
        let unusual = dhome > prof.known_radius_km;
        let (impossible, kmh) = impossible_travel(&prof, lat, lon, ts, max_kmh);

        let mut score = 0u8;
        if unusual {
            score = score.max(60);
            r.finding("unusual_location", format!("Location iko km {dhome:.1} kutoka nyumbani (kikomo {:.0})", prof.known_radius_km), 60);
        }
        if impossible {
            score = score.max(85);
            r.finding("impossible_travel", format!("Mwendo usiowezekana: {kmh:.0} km/h (kikomo {max_kmh:.0})"), 85);
        }
        let block = score >= 60;
        r.findings.push(json!({ "kind": "decision", "severity": score,
            "detail": { "block": block, "distance_home_km": format!("{dhome:.1}"), "speed_kmh": format!("{kmh:.0}") } }));
        r.summary = format!("Geolocation Security: {} (km {dhome:.1}, kasi {kmh:.0}).", if block { "imezuiwa" } else { "imeruhusiwa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impossible_travel_inagundulika() {
        let p = GeoProfile { last_lat: -6.8, last_lon: 39.2, last_ts: 0, ..Default::default() };
        // Dar → London (≈7000 km) kwa saa 1.
        let (imp, kmh) = impossible_travel(&p, 51.5, -0.12, 3600, 900.0);
        assert!(imp, "kmh={kmh}");
    }
}
