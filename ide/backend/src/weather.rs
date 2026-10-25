//! weather.rs — Sehemu 4: HALI YA HEWA halisi (bila key, bila gharama).
//!
//! Hutumia Open-Meteo (https://open-meteo.com) — API ya bure yenye data halisi
//! ya utabiri na hali ya sasa kwa Tanzania na dunia nzima. Hakuna key, hakuna
//! usajili. Vyanzo:
//!   - forecast: api.open-meteo.com/v1/forecast
//!   - geocoding: geocoding-api.open-meteo.com/v1/search
//!
//! Hali ya hewa ni data halisi ya wakati huu — hivyo module hii ni "RAG ya
//! wakati": inachukua data, kisha LLM (kama ipo) hufasiri kwa Kiswahili rahisi.

use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/weather/status", get(status))
        .route("/api/weather/geocode", get(geocode))
        .route("/api/weather/current", get(current))
        .route("/api/weather/forecast", get(forecast))
        .route("/api/weather/advice", post(advice))
}

const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODE_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

async fn status() -> Json<Value> {
    Json(json!({
        "ok": true,
        "module": "weather",
        "part": "Sehemu 4 — Hali ya hewa",
        "provider": "Open-Meteo",
        "requires_key": false,
        "cost": "bure",
        "features": ["geocode", "current", "forecast", "advice"],
    }))
}

#[derive(Deserialize)]
struct GeoQuery {
    q: String,
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    lang: Option<String>,
}

/// Jina la mahali → (lat, lon, jina kamili). Open-Meteo geocoding (bure).
async fn geocode(Query(p): Query<GeoQuery>) -> Json<Value> {
    let q = p.q.trim();
    if q.is_empty() {
        return Json(json!({"ok": false, "error": "q inahitajika"}));
    }
    let count = p.count.unwrap_or(5).clamp(1, 20);
    let lang = p.lang.clone().unwrap_or_else(|| "sw".into());
    let client = reqwest::Client::new();
    let resp = client
        .get(GEOCODE_URL)
        .query(&[
            ("name", q),
            ("count", &count.to_string()),
            ("language", &lang),
            ("format", "json"),
        ])
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await;
    let Ok(resp) = resp else {
        return Json(json!({"ok": false, "error": "geocoding haikufikiwa (mtandao)"}));
    };
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    let results: Vec<Value> = v["results"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| {
                    json!({
                        "name": r["name"],
                        "country": r["country"],
                        "admin1": r["admin1"],
                        "latitude": r["latitude"],
                        "longitude": r["longitude"],
                        "elevation": r["elevation"],
                        "timezone": r["timezone"],
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Json(json!({ "ok": true, "count": results.len(), "results": results }))
}

#[derive(Deserialize)]
struct PlaceQuery {
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    days: Option<usize>,
}

/// Hutatua (lat, lon) kutoka `lat`/`lon` au `place` (kwa geocoding).
async fn resolve_place(p: &PlaceQuery) -> Result<(f64, f64, String), Value> {
    if let (Some(lat), Some(lon)) = (p.lat, p.lon) {
        return Ok((lat, lon, format!("{lat:.4},{lon:.4}")));
    }
    let Some(place) = p.place.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(json!({"ok": false, "error": "toa lat+lon au place"}));
    };
    let client = reqwest::Client::new();
    let resp = client
        .get(GEOCODE_URL)
        .query(&[("name", place), ("count", "1"), ("language", "sw"), ("format", "json")])
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| json!({"ok": false, "error": "geocoding haikufikiwa"}))?;
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    let r = v["results"].as_array().and_then(|a| a.first()).cloned();
    let Some(r) = r else {
        return Err(json!({"ok": false, "error": format!("mahali '{place}' haipatikani")}));
    };
    let lat = r["latitude"].as_f64().unwrap_or(0.0);
    let lon = r["longitude"].as_f64().unwrap_or(0.0);
    let name = format!(
        "{}{}",
        r["name"].as_str().unwrap_or(place),
        r["country"]
            .as_str()
            .map(|c| format!(", {c}"))
            .unwrap_or_default()
    );
    Ok((lat, lon, name))
}

/// Hali ya sasa halisi (joto, upepo, mvua, unyevu).
async fn current(Query(p): Query<PlaceQuery>) -> Json<Value> {
    let (lat, lon, name) = match resolve_place(&p).await {
        Ok(v) => v,
        Err(e) => return Json(e),
    };
    let client = reqwest::Client::new();
    let resp = client
        .get(FORECAST_URL)
        .query(&[
            ("latitude", lat.to_string()),
            ("longitude", lon.to_string()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,precipitation,weather_code,wind_speed_10m,wind_direction_10m,apparent_temperature,is_day".to_string(),
            ),
            ("timezone", "Africa/Dar_es_Salaam".to_string()),
        ])
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await;
    let Ok(resp) = resp else {
        return Json(json!({"ok": false, "error": "Open-Meteo haikufikiwa (mtandao)"}));
    };
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    let c = &v["current"];
    let code = c["weather_code"].as_i64().unwrap_or(-1);
    Json(json!({
        "ok": true,
        "place": name,
        "latitude": lat,
        "longitude": lon,
        "time": c["time"],
        "temperature_c": c["temperature_2m"],
        "feels_like_c": c["apparent_temperature"],
        "humidity_pct": c["relative_humidity_2m"],
        "precipitation_mm": c["precipitation"],
        "wind_kmh": c["wind_speed_10m"],
        "wind_direction_deg": c["wind_direction_10m"],
        "is_day": c["is_day"],
        "weather_code": code,
        "condition_sw": wmo_sw(code),
        "provider": "Open-Meteo",
    }))
}

/// Utabiri wa siku N (default 7) — joto la juu/chini, mvua, upepo.
async fn forecast(Query(p): Query<PlaceQuery>) -> Json<Value> {
    let (lat, lon, name) = match resolve_place(&p).await {
        Ok(v) => v,
        Err(e) => return Json(e),
    };
    let days = p.days.unwrap_or(7).clamp(1, 16);
    let client = reqwest::Client::new();
    let resp = client
        .get(FORECAST_URL)
        .query(&[
            ("latitude", lat.to_string()),
            ("longitude", lon.to_string()),
            (
                "daily",
                "weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max".to_string(),
            ),
            ("timezone", "Africa/Dar_es_Salaam".to_string()),
            ("forecast_days", days.to_string()),
        ])
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await;
    let Ok(resp) = resp else {
        return Json(json!({"ok": false, "error": "Open-Meteo haikufikiwa (mtandao)"}));
    };
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    let d = &v["daily"];
    let dates = d["time"].as_array().cloned().unwrap_or_default();
    let mut out: Vec<Value> = Vec::new();
    for (i, date) in dates.iter().enumerate() {
        let code = d["weather_code"][i].as_i64().unwrap_or(-1);
        out.push(json!({
            "date": date,
            "temp_max_c": d["temperature_2m_max"][i],
            "temp_min_c": d["temperature_2m_min"][i],
            "precipitation_mm": d["precipitation_sum"][i],
            "rain_chance_pct": d["precipitation_probability_max"][i],
            "wind_max_kmh": d["wind_speed_10m_max"][i],
            "weather_code": code,
            "condition_sw": wmo_sw(code),
        }));
    }
    Json(json!({
        "ok": true,
        "place": name,
        "latitude": lat,
        "longitude": lon,
        "days": out.len(),
        "forecast": out,
        "provider": "Open-Meteo",
    }))
}

#[derive(Deserialize)]
struct AdviceIn {
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    place: Option<String>,
    /// Nyanja: kilimo|afya|usafiri|jumla
    #[serde(default)]
    sector: Option<String>,
}

/// Ushauri wa kitaalamu kwa Kiswahili kulingana na hali halisi ya hewa.
/// Data ni halisi (Open-Meteo); LLM (kama ipo) hufasiri kwa muktadha wa nyanja.
async fn advice(State(s): State<AppState>, Json(b): Json<AdviceIn>) -> Json<Value> {
    let pq = PlaceQuery {
        lat: b.lat,
        lon: b.lon,
        place: b.place.clone(),
        days: Some(5),
    };
    let (lat, lon, name) = match resolve_place(&pq).await {
        Ok(v) => v,
        Err(e) => return Json(e),
    };
    let sector = b.sector.clone().unwrap_or_else(|| "jumla".into());

    let client = reqwest::Client::new();
    let resp = client
        .get(FORECAST_URL)
        .query(&[
            ("latitude", lat.to_string()),
            ("longitude", lon.to_string()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,precipitation,weather_code,wind_speed_10m".to_string(),
            ),
            (
                "daily",
                "weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max".to_string(),
            ),
            ("timezone", "Africa/Dar_es_Salaam".to_string()),
            ("forecast_days", "5".to_string()),
        ])
        .timeout(std::time::Duration::from_secs(25))
        .send()
        .await;
    let Ok(resp) = resp else {
        return Json(json!({"ok": false, "error": "Open-Meteo haikufikiwa (mtandao)"}));
    };
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    let cur = &v["current"];
    let code = cur["weather_code"].as_i64().unwrap_or(-1);
    let temp = cur["temperature_2m"].as_f64().unwrap_or(0.0);
    let rain5: f64 = v["daily"]["precipitation_sum"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_f64()).sum())
        .unwrap_or(0.0);

    let summary = format!(
        "Mahali: {name}. Sasa: {} (joto {:.1}°C, unyevu {}%, upepo {} km/h, mvua {} mm). \
         Mvua inayotarajiwa siku 5: {:.1} mm.",
        wmo_sw(code),
        temp,
        cur["relative_humidity_2m"].as_i64().unwrap_or(0),
        cur["wind_speed_10m"].as_f64().unwrap_or(0.0).round(),
        cur["precipitation"].as_f64().unwrap_or(0.0),
        rain5
    );

    let prompt = format!(
        "Wewe ni mtaalamu wa MTECHOS. Nyanja: {sector}. Lugha: Kiswahili rahisi.\n\
         DATA HALISI YA HALI YA HEWA:\n{summary}\n\n\
         Toa ushauri wa hatua kwa hatua unaotegemea data hii pekee (usibuni namba). \
         Rudisha JSON PEKEE: {{\"muhtasari\":\"...\",\"ushauri\":[\"...\"],\"tahadhari\":[\"...\"]}}"
    );
    let out = crate::education::llm_long(&s.db, &prompt, 60).await;
    let advice = match &out {
        Some(t) => serde_json::from_str::<Value>(t.trim())
            .ok()
            .filter(|x| x.is_object())
            .unwrap_or_else(|| json!({ "muhtasari": t.trim() })),
        None => json!({
            "muhtasari": summary,
            "ushauri": [],
            "note": "LLM haipatikani — hii ni data halisi pekee."
        }),
    };
    Json(json!({
        "ok": true,
        "place": name,
        "latitude": lat,
        "longitude": lon,
        "sector": sector,
        "engine": if out.is_some() { "llm + open-meteo" } else { "open-meteo pekee" },
        "data": {
            "current": cur,
            "condition_sw": wmo_sw(code),
            "rain_5d_mm": rain5,
        },
        "advice": advice,
        "provider": "Open-Meteo",
    }))
}

/// Msimbo wa WMO → maelezo ya Kiswahili.
pub fn wmo_sw(code: i64) -> &'static str {
    match code {
        0 => "angavu",
        1 => "karibu angavu",
        2 => "mawingu kidogo",
        3 => "mawingu mengi",
        45 | 48 => "ukungu",
        51 | 53 | 55 => "manyunyu",
        56 | 57 => "manyunyu yanayoganda",
        61 => "mvua kidogo",
        63 => "mvua ya wastani",
        65 => "mvua kubwa",
        66 | 67 => "mvua inayoganda",
        71 => "theluji kidogo",
        73 => "theluji ya wastani",
        75 => "theluji kubwa",
        77 => "theluji ya punje",
        80 => "mvua za ghafla kidogo",
        81 => "mvua za ghafla za wastani",
        82 => "mvua za ghafla kubwa",
        85 | 86 => "theluji za ghafla",
        95 => "radi na mvua",
        96 | 99 => "radi na mvua ya mawe",
        _ => "haijulikani",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wmo_hurudisha_kiswahili() {
        assert_eq!(wmo_sw(0), "angavu");
        assert_eq!(wmo_sw(3), "mawingu mengi");
        assert_eq!(wmo_sw(65), "mvua kubwa");
        assert_eq!(wmo_sw(95), "radi na mvua");
        assert_eq!(wmo_sw(9999), "haijulikani");
    }

    #[test]
    fn wmo_haiachi_code_bila_maelezo() {
        for c in [0, 1, 2, 3, 45, 51, 61, 71, 80, 95, 99] {
            assert_ne!(wmo_sw(c), "haijulikani", "code {c} haijafafanuliwa");
        }
    }

    #[tokio::test]
    async fn resolve_place_hukataa_bila_hoja() {
        let p = PlaceQuery { lat: None, lon: None, place: None, days: None };
        assert!(resolve_place(&p).await.is_err());
    }

    #[tokio::test]
    async fn resolve_place_hukubali_lat_lon() {
        let p = PlaceQuery { lat: Some(-3.38), lon: Some(36.68), place: None, days: None };
        let (lat, lon, _) = resolve_place(&p).await.unwrap();
        assert!((lat + 3.38).abs() < 0.001);
        assert!((lon - 36.68).abs() < 0.001);
    }
}
