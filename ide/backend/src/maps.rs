//! maps.rs — MTECHOS Sehemu 3: MAP SERVICE (uelekezaji, mahali, alarm, utafutaji).
//!
//! Hutumia huduma HALISI za OpenStreetMap:
//!   - Nominatim   → kutafuta mahali kwa jina (geocode) na kinyume (reverse).
//!   - OSRM        → njia ya kweli ya barabara + hatua kwa hatua.
//!   - Overpass    → maeneo ya karibu (hospitali, hoteli, benki…).
//!
//! Hakuna key inayohitajika. Bila internet, hurudi kwa hesabu ya mstari
//! ulionyooka (haversine) na kuripoti `offline: true` — hakuna kubuni njia.
//!
//! Njia za Kiswahili hutengenezwa kutoka `maneuver` halisi ya OSRM.
//!
//! Tables: maps_places maps_alarms maps_trips

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS maps_places (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL DEFAULT '',
            query TEXT NOT NULL DEFAULT '',
            lat REAL NOT NULL DEFAULT 0,
            lon REAL NOT NULL DEFAULT 0,
            display_name TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS maps_alarms (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL DEFAULT '',
            lat REAL NOT NULL DEFAULT 0,
            lon REAL NOT NULL DEFAULT 0,
            radius_m REAL NOT NULL DEFAULT 500,
            active INTEGER NOT NULL DEFAULT 1,
            triggered_at TEXT,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS maps_trips (
            id TEXT PRIMARY KEY,
            origin TEXT NOT NULL DEFAULT '',
            destination TEXT NOT NULL DEFAULT '',
            distance_m REAL NOT NULL DEFAULT 0,
            duration_s REAL NOT NULL DEFAULT 0,
            steps INTEGER NOT NULL DEFAULT 0,
            offline INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/maps/status", get(status))
        .route("/api/maps/geocode", get(geocode))
        .route("/api/maps/reverse", get(reverse))
        .route("/api/maps/route", get(route))
        .route("/api/maps/nearby", get(nearby))
        .route("/api/maps/places", get(places_list))
        .route("/api/maps/place/add", post(place_add))
        .route("/api/maps/place/:id/remove", post(place_remove))
        .route("/api/maps/alarm/add", post(alarm_add))
        .route("/api/maps/alarms", get(alarms_list))
        .route("/api/maps/alarm/:id/remove", post(alarm_remove))
        .route("/api/maps/alarm/check", post(alarm_check))
        .route("/api/maps/trips", get(trips_list))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------- Hesabu halisi za kijiografia ----------

/// Umbali wa mstari ulionyooka (mita) kwa fomula ya haversine.
pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

/// Uelekeo wa dira (nyuzi 0–360) kutoka nukta 1 kuelekea nukta 2.
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Jina la mwelekeo kwa Kiswahili.
pub fn compass_sw(deg: f64) -> &'static str {
    let d = (deg % 360.0 + 360.0) % 360.0;
    match d {
        d if d < 22.5 || d >= 337.5 => "kaskazini",
        d if d < 67.5 => "kaskazini-mashariki",
        d if d < 112.5 => "mashariki",
        d if d < 157.5 => "kusini-mashariki",
        d if d < 202.5 => "kusini",
        d if d < 247.5 => "kusini-magharibi",
        d if d < 292.5 => "magharibi",
        _ => "kaskazini-magharibi",
    }
}

/// Maelekezo ya hatua kwa Kiswahili kutoka `maneuver` halisi ya OSRM.
pub fn step_sw(m_type: &str, modifier: &str, road: &str, dist_m: f64) -> String {
    let km = dist_m / 1000.0;
    let umbali = if km >= 1.0 {
        format!("kilomita {:.1}", km)
    } else {
        format!("mita {:.0}", dist_m)
    };
    let barabara = if road.trim().is_empty() {
        String::new()
    } else {
        format!(" kwenye {}", road.trim())
    };
    let modi = modifier.to_lowercase();
    let kuelekea = match modi.as_str() {
        "left" => "kushoto",
        "right" => "kuume",
        "slight left" => "kushoto kidogo",
        "slight right" => "kuume kidogo",
        "sharp left" => "kushoto kwa nguvu",
        "sharp right" => "kuume kwa nguvu",
        "uturn" => "kinyume (U-turn)",
        _ => "",
    };
    match m_type {
        "depart" => format!("Anza safari — endelea {umbali}{barabara}."),
        "arrive" => "Umefika unakoenda.".to_string(),
        "turn" => {
            if kuelekea.is_empty() {
                format!("Endelea {umbali}{barabara}.")
            } else {
                format!("Geuka {kuelekea}{barabara}, kisha endelea {umbali}.")
            }
        }
        "roundabout" | "rotary" => format!("Zunguka kwenye kivuko (roundabout){barabara}."),
        "merge" => format!("Ungana na barabara{barabara}, endelea {umbali}."),
        "fork" => {
            if kuelekea.is_empty() {
                format!("Chagua njia sahihi{barabara}, endelea {umbali}.")
            } else {
                format!("Kwenye njia inayogawanyika, chukua upande wa {kuelekea}{barabara}.")
            }
        }
        "on ramp" => format!("Panda kwenye barabara kuu{barabara}."),
        "off ramp" => format!("Shuka kwenye njia ya kutoka{barabara}."),
        "end of road" => format!("Mwisho wa barabara — geuka {kuelekea}{barabara}."),
        "new name" | "continue" => format!("Endelea moja kwa moja {umbali}{barabara}."),
        _ => format!("Endelea {umbali}{barabara}."),
    }
}

// ---------- Huduma za OSM (halisi) ----------

fn nominatim_base() -> String {
    std::env::var("NOMINATIM_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://nominatim.openstreetmap.org".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn osrm_base() -> String {
    std::env::var("OSRM_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://router.project-osrm.org".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn overpass_base() -> String {
    std::env::var("OVERPASS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://overpass-api.de/api/interpreter".to_string())
        .trim_end_matches('/')
        .to_string()
}

async fn nominatim_search(q: &str, limit: usize) -> Result<Vec<Value>, String> {
    let url = format!(
        "{}/search?q={}&format=jsonv2&limit={}&addressdetails=1",
        nominatim_base(),
        crate::urlenc(q),
        limit.clamp(1, 20)
    );
    let client = crate::http_client(20).ok_or("imeshindwa kujenga client")?;
    let resp = client
        .get(&url)
        .header("Accept-Language", "sw,en")
        .send()
        .await
        .map_err(|e| format!("Nominatim haipatikani: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Nominatim status {}", resp.status()));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|r| {
            let lat = r["lat"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let lon = r["lon"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            json!({
                "label": r["name"].as_str().unwrap_or("").trim(),
                "display_name": r["display_name"].as_str().unwrap_or("").trim(),
                "lat": lat, "lon": lon,
                "type": r["type"].as_str().unwrap_or(""),
                "category": r["category"].as_str().unwrap_or(""),
            })
        })
        .filter(|x| x["lat"].as_f64().unwrap_or(0.0) != 0.0 || x["lon"].as_f64().unwrap_or(0.0) != 0.0)
        .collect())
}

// ---------- Handlers ----------

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "module": "maps",
        "part": "Sehemu 3 — Map Service",
        "providers": {
            "geocode": nominatim_base(),
            "routing": osrm_base(),
            "poi": overpass_base(),
        },
        "places": table_count(&s.db, "maps_places").await,
        "alarms": table_count(&s.db, "maps_alarms").await,
        "alarms_active": sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM maps_alarms WHERE active=1")
            .fetch_one(&s.db).await.map(|r| r.0).unwrap_or(0),
        "trips": table_count(&s.db, "maps_trips").await,
    }))
}

async fn geocode(Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let query = q.get("q").cloned().unwrap_or_default().trim().to_string();
    if query.is_empty() {
        return Json(json!({ "ok": false, "error": "hakuna mahali — tumia ?q=..." }));
    }
    let limit = q.get("limit").and_then(|v| v.parse().ok()).unwrap_or(5);
    match nominatim_search(&query, limit).await {
        Ok(items) => {
            let n = items.len();
            Json(json!({
                "ok": true, "query": query, "count": n, "results": items,
                "source": "OpenStreetMap Nominatim (halisi)",
                "note_sw": if n == 0 { "Hakuna mahali palipopatikana kwa jina hilo." } else { "Matokeo halisi ya OpenStreetMap." }
            }))
        }
        Err(e) => Json(json!({ "ok": false, "error": e, "query": query })),
    }
}

async fn reverse(Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let lat = q.get("lat").and_then(|v| v.parse::<f64>().ok());
    let lon = q.get("lon").and_then(|v| v.parse::<f64>().ok());
    let (Some(lat), Some(lon)) = (lat, lon) else {
        return Json(json!({ "ok": false, "error": "tumia ?lat=..&lon=.." }));
    };
    let url = format!(
        "{}/reverse?lat={lat}&lon={lon}&format=jsonv2&zoom=18&addressdetails=1",
        nominatim_base()
    );
    let Some(client) = crate::http_client(20) else {
        return Json(json!({ "ok": false, "error": "imeshindwa kujenga client" }));
    };
    match client.get(&url).header("Accept-Language", "sw,en").send().await {
        Ok(resp) if resp.status().is_success() => {
            let v: Value = resp.json().await.unwrap_or(json!({}));
            Json(json!({
                "ok": true, "lat": lat, "lon": lon,
                "display_name": v["display_name"].as_str().unwrap_or(""),
                "address": v["address"].clone(),
                "source": "OpenStreetMap Nominatim (halisi)"
            }))
        }
        Ok(resp) => Json(json!({ "ok": false, "error": format!("Nominatim status {}", resp.status()) })),
        Err(e) => Json(json!({ "ok": false, "error": format!("Nominatim haipatikani: {e}") })),
    }
}

#[derive(Deserialize)]
struct RouteQuery {
    from_lat: Option<f64>,
    from_lon: Option<f64>,
    to_lat: Option<f64>,
    to_lon: Option<f64>,
    from: Option<String>,
    to: Option<String>,
    profile: Option<String>,
}

/// Njia halisi ya barabara (OSRM) + hatua kwa Kiswahili. Bila OSRM: mstari
/// ulionyooka (haversine) na `offline: true`.
async fn route(State(s): State<AppState>, Query(q): Query<RouteQuery>) -> Json<Value> {
    // Geocode ikiwa jina limetolewa badala ya kuratibu.
    let mut from = (q.from_lat, q.from_lon);
    let mut to = (q.to_lat, q.to_lon);
    if from.0.is_none() {
        if let Some(name) = q.from.as_deref().filter(|s| !s.trim().is_empty()) {
            if let Ok(items) = nominatim_search(name, 1).await {
                if let Some(f) = items.first() {
                    from = (f["lat"].as_f64(), f["lon"].as_f64());
                }
            }
        }
    }
    if to.0.is_none() {
        if let Some(name) = q.to.as_deref().filter(|s| !s.trim().is_empty()) {
            if let Ok(items) = nominatim_search(name, 1).await {
                if let Some(f) = items.first() {
                    to = (f["lat"].as_f64(), f["lon"].as_f64());
                }
            }
        }
    }
    let (Some(flat), Some(flon), Some(tlat), Some(tlon)) = (from.0, from.1, to.0, to.1) else {
        return Json(json!({
            "ok": false,
            "error": "tumia ?from_lat=..&from_lon=..&to_lat=..&to_lon=.. AU ?from=<jina>&to=<jina>",
        }));
    };

    let profile = match q.profile.as_deref().unwrap_or("driving") {
        "walking" | "foot" => "foot",
        "cycling" | "bike" => "bike",
        _ => "driving",
    };
    let url = format!(
        "{}/route/v1/{profile}/{flon},{flat};{tlon},{tlat}?overview=false&steps=true&geometries=geojson",
        osrm_base()
    );

    if let Some(client) = crate::http_client(25) {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<Value>().await {
                    if v["code"].as_str() == Some("Ok") {
                        if let Some(r) = v["routes"].as_array().and_then(|a| a.first()) {
                            let dist = r["distance"].as_f64().unwrap_or(0.0);
                            let dur = r["duration"].as_f64().unwrap_or(0.0);
                            let mut steps_sw: Vec<String> = Vec::new();
                            if let Some(legs) = r["legs"].as_array() {
                                for leg in legs {
                                    if let Some(steps) = leg["steps"].as_array() {
                                        for st in steps {
                                            let m = &st["maneuver"];
                                            let txt = step_sw(
                                                m["type"].as_str().unwrap_or(""),
                                                m["modifier"].as_str().unwrap_or(""),
                                                st["name"].as_str().unwrap_or(""),
                                                st["distance"].as_f64().unwrap_or(0.0),
                                            );
                                            steps_sw.push(txt);
                                        }
                                    }
                                }
                            }
                            let _ = sqlx::query(
                                "INSERT INTO maps_trips (id,origin,destination,distance_m,duration_s,steps,offline,created_at)
                                 VALUES (?1,?2,?3,?4,?5,?6,0,?7)")
                                .bind(uid())
                                .bind(format!("{flat},{flon}"))
                                .bind(format!("{tlat},{tlon}"))
                                .bind(dist).bind(dur).bind(steps_sw.len() as i64).bind(now())
                                .execute(&s.db).await;
                            return Json(json!({
                                "ok": true, "offline": false,
                                "distance_m": dist, "distance_km": (dist / 1000.0 * 10.0).round() / 10.0,
                                "duration_s": dur, "duration_min": (dur / 60.0).round() as i64,
                                "steps": steps_sw,
                                "source": "OSRM (njia halisi ya barabara)",
                                "profile": profile,
                            }));
                        }
                    }
                    return Json(json!({
                        "ok": false,
                        "error": v["message"].as_str().unwrap_or("OSRM haikupata njia"),
                        "code": v["code"].as_str().unwrap_or("")
                    }));
                }
            }
        }
    }

    // Offline: mstari ulionyooka — HAKUNA kubuni barabara.
    let dist = haversine_m(flat, flon, tlat, tlon);
    let bearing = bearing_deg(flat, flon, tlat, tlon);
    let _ = sqlx::query(
        "INSERT INTO maps_trips (id,origin,destination,distance_m,duration_s,steps,offline,created_at)
         VALUES (?1,?2,?3,?4,?5,1,1,?6)")
        .bind(uid())
        .bind(format!("{flat},{flon}")).bind(format!("{tlat},{tlon}"))
        .bind(dist).bind(0.0).bind(now())
        .execute(&s.db).await;
    Json(json!({
        "ok": true, "offline": true,
        "distance_m": dist, "distance_km": (dist / 1000.0 * 10.0).round() / 10.0,
        "bearing_deg": (bearing * 10.0).round() / 10.0,
        "direction": compass_sw(bearing),
        "steps": [format!("Safari ya mstari ulionyooka: elekea {} kwa kilomita {:.1}.",
            compass_sw(bearing), dist / 1000.0)],
        "note_sw": "OSRM haipatikani — hii ni umbali wa mstari ulionyooka, SI njia ya barabara."
    }))
}

async fn nearby(Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let lat = q.get("lat").and_then(|v| v.parse::<f64>().ok());
    let lon = q.get("lon").and_then(|v| v.parse::<f64>().ok());
    let (Some(lat), Some(lon)) = (lat, lon) else {
        return Json(json!({ "ok": false, "error": "tumia ?lat=..&lon=..&amenity=hospital" }));
    };
    let radius: f64 = q.get("radius").and_then(|v| v.parse().ok()).unwrap_or(2000.0);
    let amenity = q
        .get("amenity")
        .cloned()
        .unwrap_or_else(|| "hospital".to_string())
        .replace(['"', '\\'], "");
    let limit: usize = q.get("limit").and_then(|v| v.parse().ok()).unwrap_or(10);

    let ql = format!(
        "[out:json][timeout:20];node(around:{radius:.0},{lat},{lon})[\"amenity\"=\"{amenity}\"];out body {limit};"
    );
    let Some(client) = crate::http_client(30) else {
        return Json(json!({ "ok": false, "error": "imeshindwa kujenga client" }));
    };
    let resp = client
        .post(overpass_base())
        .form(&[("data", ql.as_str())])
        .send()
        .await;
    let Ok(resp) = resp else {
        return Json(json!({ "ok": false, "error": "Overpass haipatikani" }));
    };
    if !resp.status().is_success() {
        return Json(json!({ "ok": false, "error": format!("Overpass status {}", resp.status()) }));
    }
    let v: Value = resp.json().await.unwrap_or(json!({}));
    let mut out: Vec<Value> = Vec::new();
    if let Some(els) = v["elements"].as_array() {
        for e in els {
            let elat = e["lat"].as_f64().unwrap_or(0.0);
            let elon = e["lon"].as_f64().unwrap_or(0.0);
            let d = haversine_m(lat, lon, elat, elon);
            let b = bearing_deg(lat, lon, elat, elon);
            let tags = &e["tags"];
            out.push(json!({
                "name": tags["name"].as_str().unwrap_or("(bila jina)"),
                "amenity": tags["amenity"].as_str().unwrap_or(&amenity),
                "lat": elat, "lon": elon,
                "distance_m": (d * 10.0).round() / 10.0,
                "direction": compass_sw(b),
                "phone": tags["phone"].as_str().unwrap_or(""),
            }));
        }
    }
    out.sort_by(|a, b| {
        a["distance_m"].as_f64().unwrap_or(0.0)
            .partial_cmp(&b["distance_m"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let n = out.len();
    Json(json!({
        "ok": true, "amenity": amenity, "count": n, "results": out,
        "source": "OpenStreetMap Overpass (halisi)",
        "note_sw": if n == 0 { "Hakuna kilichopatikana karibu na hapo." } else { "Maeneo halisi yenye umbali na mwelekeo." }
    }))
}

async fn places_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, f64, f64, String, String)>(
        "SELECT id,label,query,lat,lon,display_name,created_at FROM maps_places ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let places: Vec<Value> = rows
        .into_iter()
        .map(|(id, label, query, lat, lon, dn, at)| {
            json!({ "id": id, "label": label, "query": query, "lat": lat, "lon": lon, "display_name": dn, "created_at": at })
        })
        .collect();
    Json(json!({ "ok": true, "places": places }))
}

#[derive(Deserialize)]
struct PlaceIn {
    label: Option<String>,
    query: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    display_name: Option<String>,
}

async fn place_add(State(s): State<AppState>, Json(p): Json<PlaceIn>) -> Json<Value> {
    let label = p.label.unwrap_or_default().trim().to_string();
    let query = p.query.unwrap_or_default().trim().to_string();
    let mut lat = p.lat.unwrap_or(0.0);
    let mut lon = p.lon.unwrap_or(0.0);
    let mut dn = p.display_name.unwrap_or_default();
    if label.is_empty() && query.is_empty() {
        return Json(json!({ "ok": false, "error": "weka `label` au `query`" }));
    }
    // Bila kuratibu: tafuta halisi kwa Nominatim.
    if lat == 0.0 && lon == 0.0 {
        let q = if query.is_empty() { label.clone() } else { query.clone() };
        match nominatim_search(&q, 1).await {
            Ok(items) if !items.is_empty() => {
                lat = items[0]["lat"].as_f64().unwrap_or(0.0);
                lon = items[0]["lon"].as_f64().unwrap_or(0.0);
                if dn.is_empty() {
                    dn = items[0]["display_name"].as_str().unwrap_or("").to_string();
                }
            }
            Ok(_) => return Json(json!({ "ok": false, "error": "mahali hapapatikani kwa jina hilo" })),
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        }
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO maps_places (id,label,query,lat,lon,display_name,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id).bind(&label).bind(&query).bind(lat).bind(lon).bind(&dn).bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "lat": lat, "lon": lon, "display_name": dn }))
}

async fn place_remove(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query("DELETE FROM maps_places WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": r.is_ok(), "removed": r.map(|x| x.rows_affected()).unwrap_or(0) }))
}

#[derive(Deserialize)]
struct AlarmIn {
    label: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    radius_m: Option<f64>,
    query: Option<String>,
}

async fn alarm_add(State(s): State<AppState>, Json(a): Json<AlarmIn>) -> Json<Value> {
    let label = a.label.unwrap_or_default().trim().to_string();
    let mut lat = a.lat.unwrap_or(0.0);
    let mut lon = a.lon.unwrap_or(0.0);
    let radius = a.radius_m.unwrap_or(500.0).max(10.0);
    if lat == 0.0 && lon == 0.0 {
        let q = a.query.unwrap_or_else(|| label.clone());
        if q.trim().is_empty() {
            return Json(json!({ "ok": false, "error": "weka `lat`+`lon` AU `query`/`label`" }));
        }
        match nominatim_search(&q, 1).await {
            Ok(items) if !items.is_empty() => {
                lat = items[0]["lat"].as_f64().unwrap_or(0.0);
                lon = items[0]["lon"].as_f64().unwrap_or(0.0);
            }
            Ok(_) => return Json(json!({ "ok": false, "error": "mahali hapapatikani" })),
            Err(e) => return Json(json!({ "ok": false, "error": e })),
        }
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO maps_alarms (id,label,lat,lon,radius_m,active,created_at) VALUES (?1,?2,?3,?4,?5,1,?6)",
    )
    .bind(&id).bind(&label).bind(lat).bind(lon).bind(radius).bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "lat": lat, "lon": lon, "radius_m": radius }))
}

async fn alarms_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, f64, f64, f64, i64, Option<String>, String)>(
        "SELECT id,label,lat,lon,radius_m,active,triggered_at,created_at FROM maps_alarms ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let alarms: Vec<Value> = rows
        .into_iter()
        .map(|(id, label, lat, lon, r, active, trig, at)| {
            json!({ "id": id, "label": label, "lat": lat, "lon": lon, "radius_m": r,
                    "active": active != 0, "triggered_at": trig, "created_at": at })
        })
        .collect();
    Json(json!({ "ok": true, "alarms": alarms }))
}

async fn alarm_remove(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query("DELETE FROM maps_alarms WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": r.is_ok(), "removed": r.map(|x| x.rows_affected()).unwrap_or(0) }))
}

#[derive(Deserialize)]
struct AlarmCheck {
    lat: f64,
    lon: f64,
}

/// Hupima mahali pa mtumiaji dhidi ya alarm zote — hurudisha zilizoamilika.
async fn alarm_check(State(s): State<AppState>, Json(c): Json<AlarmCheck>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, f64, f64, f64)>(
        "SELECT id,label,lat,lon,radius_m FROM maps_alarms WHERE active=1",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut fired: Vec<Value> = Vec::new();
    let mut nearest: Option<(f64, String)> = None;
    for (id, label, lat, lon, radius) in rows {
        let d = haversine_m(c.lat, c.lon, lat, lon);
        if nearest.as_ref().map(|(nd, _)| d < *nd).unwrap_or(true) {
            nearest = Some((d, label.clone()));
        }
        if d <= radius {
            let _ = sqlx::query("UPDATE maps_alarms SET triggered_at=?1, active=0 WHERE id=?2")
                .bind(now()).bind(&id).execute(&s.db).await;
            fired.push(json!({ "id": id, "label": label, "distance_m": (d * 10.0).round() / 10.0 }));
        }
    }
    let (nd, nl) = match nearest {
        Some((d, l)) => (json!((d * 10.0).round() / 10.0), json!(l)),
        None => (json!(null), json!("")),
    };
    Json(json!({
        "ok": true,
        "fired": fired,
        "fired_count": fired.len(),
        "nearest_label": nl,
        "nearest_m": nd,
        "note_sw": if fired.is_empty() {
            "Hakuna alarm iliyoamilika bado.".to_string()
        } else {
            format!("Alarm {} zimeamilika.", fired.len())
        }
    }))
}

async fn trips_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, f64, f64, i64, i64, String)>(
        "SELECT id,origin,destination,distance_m,duration_s,steps,offline,created_at FROM maps_trips ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let trips: Vec<Value> = rows
        .into_iter()
        .map(|(id, o, d, dist, dur, steps, off, at)| {
            json!({ "id": id, "origin": o, "destination": d, "distance_m": dist,
                    "duration_s": dur, "steps": steps, "offline": off != 0, "created_at": at })
        })
        .collect();
    Json(json!({ "ok": true, "trips": trips }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_ni_sahihi_kwa_dar_na_mwanza() {
        // Dar es Salaam → Mwanza ≈ km 850 (mstari ulionyooka).
        let d = haversine_m(-6.7924, 39.2083, -2.5164, 32.9175);
        let km = d / 1000.0;
        assert!((800.0..900.0).contains(&km), "umbali ulikuwa {km} km");
    }

    #[test]
    fn haversine_ni_sifuri_kwa_nukta_ile_ile() {
        assert_eq!(haversine_m(-6.8, 39.2, -6.8, 39.2), 0.0);
    }

    #[test]
    fn compass_hurudisha_kiswahili_sahihi() {
        assert_eq!(compass_sw(0.0), "kaskazini");
        assert_eq!(compass_sw(90.0), "mashariki");
        assert_eq!(compass_sw(180.0), "kusini");
        assert_eq!(compass_sw(270.0), "magharibi");
        assert_eq!(compass_sw(359.0), "kaskazini");
    }

    #[test]
    fn bearing_kaskazini_ni_sifuri() {
        // Kusonga kaskazini (lat inaongezeka, lon ile ile) → 0°.
        let b = bearing_deg(-6.0, 39.0, -5.0, 39.0);
        assert!(b < 1.0 || b > 359.0, "bearing ulikuwa {b}");
    }

    #[test]
    fn hatua_za_osrm_zinatafsiriwa_kwa_kiswahili() {
        let s = step_sw("turn", "left", "Nyerere Road", 1500.0);
        assert!(s.contains("Geuka kushoto"), "ilikuwa: {s}");
        assert!(s.contains("kilomita 1.5"), "ilikuwa: {s}");
        assert!(s.contains("Nyerere Road"), "ilikuwa: {s}");
        assert!(step_sw("arrive", "", "", 0.0).contains("Umefika"));
        assert!(step_sw("roundabout", "", "", 0.0).contains("roundabout"));
        assert!(step_sw("depart", "", "", 300.0).contains("mita 300"));
    }

    #[test]
    fn hatua_bila_barabara_haina_nafasi_ya_ziada() {
        let s = step_sw("continue", "", "", 200.0);
        assert!(!s.contains("  "), "nafasi maradufu: {s}");
        assert!(!s.contains(" kwenye ."), "barabara tupu: {s}");
    }

    #[test]
    fn jina_la_barabara_hutumiwa_kama_lipo() {
        let s = step_sw("turn", "right", "Ali Hassan Mwinyi Rd", 800.0);
        assert!(s.contains("kuume"), "ilikuwa: {s}");
        assert!(s.contains("Ali Hassan Mwinyi Rd"), "ilikuwa: {s}");
    }
}
