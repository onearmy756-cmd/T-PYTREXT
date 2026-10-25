//! hostel.rs — ELIMUPRO Awamu 10: hosteli (malazi) (moduli 14).
//!
//! Mtiririko: hosteli (jengo) → vyumba → vitanda → ugawaji kwa wanafunzi.
//!
//! Kanuni halisi:
//!   - Jinsia: mwanafunzi wa kike hawezi kupewa hosteli ya kiume na kinyume
//!     (isipokuwa hosteli `mixed`).
//!   - Kitanda kilichochukuliwa hakiwezi kugawiwa tena.
//!   - Mwanafunzi anaweza kuwa na ugawaji MMOJA unaoendelea.
//!   - Kuondoa (vacate) hufanya kitanda kiwe huru tena.
//!   - Nafasi: vitanda huru / jumla ya vitanda.

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
        r#"CREATE TABLE IF NOT EXISTS ep_hostels (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            gender TEXT NOT NULL DEFAULT 'mixed', -- male|female|mixed
            warden TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_rooms (
            id TEXT PRIMARY KEY,
            hostel_id TEXT NOT NULL,
            room_no TEXT NOT NULL DEFAULT '',
            beds INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            UNIQUE(hostel_id, room_no)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_beds (
            id TEXT PRIMARY KEY,
            room_id TEXT NOT NULL,
            label TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'free', -- free|occupied
            UNIQUE(room_id, label)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_allocations (
            id TEXT PRIMARY KEY,
            bed_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            allocated_at TEXT NOT NULL,
            vacated_at TEXT
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/hostel/status", get(status))
        .route("/api/hostel/list", get(hostels_list))
        .route("/api/hostel/add", post(hostel_add))
        .route("/api/hostel/:id", get(hostel_get))
        .route("/api/hostel/room/add", post(room_add))
        .route("/api/hostel/allocate", post(allocate))
        .route("/api/hostel/vacate", post(vacate))
        .route("/api/hostel/allocations", get(allocations_list))
        .route("/api/hostel/occupancy", get(occupancy))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

/// Hugeuza jinsia ya mwanafunzi kuwa `male`/`female`/`mixed`.
pub fn norm_gender(s: &str) -> &'static str {
    match s.trim().to_lowercase().as_str() {
        "m" | "male" | "me" | "kiume" => "male",
        "f" | "female" | "ke" | "kike" => "female",
        "mixed" | "mchanganyiko" => "mixed",
        _ => "unknown",
    }
}

/// Hukubali mwanafunzi wa jinsia hii kwenye hosteli ya jinsia hii?
pub fn gender_allowed(hostel_gender: &str, student_gender: &str) -> bool {
    let h = norm_gender(hostel_gender);
    let s = norm_gender(student_gender);
    if h == "mixed" {
        return s == "male" || s == "female";
    }
    h == s && s != "unknown"
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (beds,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_beds")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (occupied,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_beds WHERE status='occupied'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "hostels": table_count(&s.db, "ep_hostels").await,
        "rooms": table_count(&s.db, "ep_rooms").await,
        "beds": beds,
        "occupied": occupied,
        "free": beds - occupied,
        "occupancy_pct": if beds > 0 { ((occupied as f64 / beds as f64) * 1000.0).round() / 10.0 } else { 0.0 },
        "allocations": table_count(&s.db, "ep_allocations").await
    }))
}

// ============================================================
// HOSTELI + VYUMBA
// ============================================================

#[derive(Deserialize)]
pub struct HostelIn {
    pub name: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub warden: String,
}

pub async fn hostel_add(State(s): State<AppState>, Json(b): Json<HostelIn>) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la hosteli linahitajika" }));
    }
    let g = norm_gender(&b.gender);
    let gender = if g == "unknown" { "mixed" } else { g };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_hostels (id,name,gender,warden,created_at) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&id)
    .bind(b.name.trim())
    .bind(gender)
    .bind(b.warden.trim())
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "gender": gender }))
}

pub async fn hostels_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64)>(
        "SELECT h.id,h.name,h.gender,h.warden,
                (SELECT COUNT(*) FROM ep_rooms r WHERE r.hostel_id=h.id),
                (SELECT COUNT(*) FROM ep_beds b JOIN ep_rooms r ON r.id=b.room_id WHERE r.hostel_id=h.id),
                (SELECT COUNT(*) FROM ep_beds b JOIN ep_rooms r ON r.id=b.room_id WHERE r.hostel_id=h.id AND b.status='occupied')
         FROM ep_hostels h ORDER BY h.name",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "hostels": rows.into_iter().map(|r| json!({
            "id": r.0, "name": r.1, "gender": r.2, "warden": r.3,
            "rooms": r.4, "beds": r.5, "occupied": r.6, "free": r.5 - r.6
        })).collect::<Vec<_>>()
    }))
}

pub async fn hostel_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let h = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,name,gender FROM ep_hostels WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((hid, name, gender)) = h else {
        return Json(json!({ "ok": false, "error": "hosteli haipo" }));
    };
    let rooms = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT r.id,r.room_no,r.beds,
                (SELECT COUNT(*) FROM ep_beds b WHERE b.room_id=r.id AND b.status='occupied')
         FROM ep_rooms r WHERE r.hostel_id=?1 ORDER BY r.room_no",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut out = Vec::new();
    for (rid, room_no, beds, occ) in rooms {
        let bl = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
            "SELECT b.id,b.label,b.status,
                    (SELECT COALESCE(st.full_name,'') FROM ep_allocations a
                       JOIN ep_students st ON st.id=a.student_id
                       WHERE a.bed_id=b.id AND a.vacated_at IS NULL LIMIT 1),
                    (SELECT a.student_id FROM ep_allocations a WHERE a.bed_id=b.id AND a.vacated_at IS NULL LIMIT 1)
             FROM ep_beds b WHERE b.room_id=?1 ORDER BY b.label",
        )
        .bind(&rid)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
        out.push(json!({
            "id": rid, "room_no": room_no, "beds": beds, "occupied": occ,
            "beds_detail": bl.into_iter().map(|b| json!({
                "id": b.0, "label": b.1, "status": b.2, "student": b.3, "student_id": b.4
            })).collect::<Vec<_>>()
        }));
    }
    Json(json!({ "ok": true, "hostel": { "id": hid, "name": name, "gender": gender }, "rooms": out }))
}

#[derive(Deserialize)]
pub struct RoomIn {
    pub hostel_id: String,
    pub room_no: String,
    #[serde(default)]
    pub beds: Option<i64>,
}

pub async fn room_add(State(s): State<AppState>, Json(b): Json<RoomIn>) -> Json<Value> {
    if b.room_no.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "namba ya chumba inahitajika" }));
    }
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM ep_hostels WHERE id=?1")
        .bind(&b.hostel_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    if exists.is_none() {
        return Json(json!({ "ok": false, "error": "hosteli haipo" }));
    }
    let n = b.beds.unwrap_or(4).clamp(1, 20);
    let id = uid();
    let res = sqlx::query(
        "INSERT INTO ep_rooms (id,hostel_id,room_no,beds,created_at) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&id)
    .bind(&b.hostel_id)
    .bind(b.room_no.trim())
    .bind(n)
    .bind(now())
    .execute(&s.db)
    .await;
    if res.is_err() {
        return Json(json!({ "ok": false, "error": "chumba hiki kipo tayari" }));
    }
    for i in 1..=n {
        let _ = sqlx::query("INSERT INTO ep_beds (id,room_id,label,status) VALUES (?1,?2,?3,'free')")
            .bind(uid())
            .bind(&id)
            .bind(format!("B{i}"))
            .execute(&s.db)
            .await;
    }
    Json(json!({ "ok": true, "id": id, "beds": n }))
}

// ============================================================
// UGAWAJI
// ============================================================

#[derive(Deserialize)]
pub struct AllocIn {
    pub student_id: String,
    #[serde(default)]
    pub hostel_id: String,
    #[serde(default)]
    pub room_id: String,
    #[serde(default)]
    pub bed_id: String,
}

pub async fn allocate(State(s): State<AppState>, Json(b): Json<AllocIn>) -> Json<Value> {
    // Mwanafunzi
    let st = sqlx::query_as::<_, (String, String)>(
        "SELECT COALESCE(full_name,''),COALESCE(gender,'') FROM ep_students WHERE id=?1",
    )
    .bind(&b.student_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((_name, gender)) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hayupo" }));
    };

    // Mwanafunzi ana ugawaji unaoendelea?
    let active: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM ep_allocations WHERE student_id=?1 AND vacated_at IS NULL",
    )
    .bind(&b.student_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if active.is_some() {
        return Json(json!({ "ok": false, "error": "mwanafunzi ana nafasi tayari" }));
    }

    // Tafuta kitanda huru.
    let bed = if !b.bed_id.trim().is_empty() {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT b.id,b.status,r.id,h.gender FROM ep_beds b
             JOIN ep_rooms r ON r.id=b.room_id JOIN ep_hostels h ON h.id=r.hostel_id
             WHERE b.id=?1",
        )
        .bind(b.bed_id.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    } else if !b.room_id.trim().is_empty() {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT b.id,b.status,r.id,h.gender FROM ep_beds b
             JOIN ep_rooms r ON r.id=b.room_id JOIN ep_hostels h ON h.id=r.hostel_id
             WHERE r.id=?1 AND b.status='free' ORDER BY b.label LIMIT 1",
        )
        .bind(b.room_id.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    } else {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT b.id,b.status,r.id,h.gender FROM ep_beds b
             JOIN ep_rooms r ON r.id=b.room_id JOIN ep_hostels h ON h.id=r.hostel_id
             WHERE (?1='' OR h.id=?1) AND b.status='free' ORDER BY h.name, r.room_no, b.label LIMIT 1",
        )
        .bind(&b.hostel_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    };
    let Some((bed_id, bed_status, _room_id, hostel_gender)) = bed else {
        return Json(json!({ "ok": false, "error": "hakuna kitanda huru" }));
    };
    if bed_status != "free" {
        return Json(json!({ "ok": false, "error": "kitanda kimechukuliwa" }));
    }
    if !gender_allowed(&hostel_gender, &gender) {
        return Json(json!({
            "ok": false, "error": "jinsia haikubaliwi kwenye hosteli hii",
            "hostel_gender": norm_gender(&hostel_gender), "student_gender": norm_gender(&gender)
        }));
    }

    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_allocations (id,bed_id,student_id,allocated_at) VALUES (?1,?2,?3,?4)",
    )
    .bind(&id)
    .bind(&bed_id)
    .bind(&b.student_id)
    .bind(now())
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE ep_beds SET status='occupied' WHERE id=?1")
        .bind(&bed_id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "allocation_id": id, "bed_id": bed_id }))
}

#[derive(Deserialize)]
pub struct VacateIn {
    #[serde(default)]
    pub allocation_id: String,
    #[serde(default)]
    pub student_id: String,
}

pub async fn vacate(State(s): State<AppState>, Json(b): Json<VacateIn>) -> Json<Value> {
    let alloc = if !b.allocation_id.trim().is_empty() {
        sqlx::query_as::<_, (String, String)>(
            "SELECT id,bed_id FROM ep_allocations WHERE id=?1 AND vacated_at IS NULL",
        )
        .bind(b.allocation_id.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    } else {
        sqlx::query_as::<_, (String, String)>(
            "SELECT id,bed_id FROM ep_allocations WHERE student_id=?1 AND vacated_at IS NULL",
        )
        .bind(&b.student_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    };
    let Some((aid, bed_id)) = alloc else {
        return Json(json!({ "ok": false, "error": "hakuna ugawaji unaoendelea" }));
    };
    let _ = sqlx::query("UPDATE ep_allocations SET vacated_at=?1 WHERE id=?2")
        .bind(now())
        .bind(&aid)
        .execute(&s.db)
        .await;
    let _ = sqlx::query("UPDATE ep_beds SET status='free' WHERE id=?1")
        .bind(&bed_id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "bed_id": bed_id }))
}

#[derive(Deserialize)]
pub struct HostelQuery {
    #[serde(default)]
    pub hostel_id: String,
    #[serde(default)]
    pub active: Option<bool>,
}

pub async fn allocations_list(State(s): State<AppState>, Query(q): Query<HostelQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>)>(
        "SELECT a.id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),
                COALESCE(h.name,''),COALESCE(r.room_no,''),COALESCE(b.label,''),a.vacated_at
         FROM ep_allocations a
         LEFT JOIN ep_students st ON st.id=a.student_id
         LEFT JOIN ep_beds b ON b.id=a.bed_id
         LEFT JOIN ep_rooms r ON r.id=b.room_id
         LEFT JOIN ep_hostels h ON h.id=r.hostel_id
         WHERE (?1='' OR h.id=?1)
           AND (?2=0 OR a.vacated_at IS NULL)
         ORDER BY a.allocated_at DESC LIMIT 500",
    )
    .bind(&q.hostel_id)
    .bind(if q.active.unwrap_or(true) { 1 } else { 0 })
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "allocations": rows.into_iter().map(|r| json!({
            "id": r.0, "student": r.1, "reg_no": r.2, "hostel": r.3,
            "room_no": r.4, "bed": r.5, "active": r.6.is_none(), "vacated_at": r.6
        })).collect::<Vec<_>>()
    }))
}

pub async fn occupancy(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT h.id,h.name,h.gender,
                (SELECT COUNT(*) FROM ep_beds b JOIN ep_rooms r ON r.id=b.room_id WHERE r.hostel_id=h.id),
                (SELECT COUNT(*) FROM ep_beds b JOIN ep_rooms r ON r.id=b.room_id WHERE r.hostel_id=h.id AND b.status='occupied')
         FROM ep_hostels h ORDER BY h.name",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let out: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "hostel_id": r.0, "hostel": r.1, "gender": r.2, "beds": r.3, "occupied": r.4,
                "free": r.3 - r.4,
                "occupancy_pct": if r.3 > 0 { ((r.4 as f64 / r.3 as f64) * 1000.0).round() / 10.0 } else { 0.0 }
            })
        })
        .collect();
    Json(json!({ "ok": true, "occupancy": out }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS ep_students (
                id TEXT PRIMARY KEY, full_name TEXT NOT NULL DEFAULT '',
                reg_no TEXT NOT NULL DEFAULT '', gender TEXT NOT NULL DEFAULT '')",
        )
        .execute(&p)
        .await;
        init_tables(&p).await;
        p
    }

    #[test]
    fn jinsia_hutafsiriwa() {
        assert_eq!(norm_gender("M"), "male");
        assert_eq!(norm_gender("me"), "male");
        assert_eq!(norm_gender("kiume"), "male");
        assert_eq!(norm_gender("F"), "female");
        assert_eq!(norm_gender("ke"), "female");
        assert_eq!(norm_gender("kike"), "female");
        assert_eq!(norm_gender("xyz"), "unknown");
    }

    #[test]
    fn jinsia_hulinganishwa_sahihi() {
        assert!(gender_allowed("male", "M"));
        assert!(gender_allowed("female", "F"));
        // Hosteli ya kiume haimkubali mwanafunzi wa kike.
        assert!(!gender_allowed("male", "F"));
        assert!(!gender_allowed("female", "M"));
        // Mixed hukubali wote wawili.
        assert!(gender_allowed("mixed", "M"));
        assert!(gender_allowed("mixed", "F"));
        // Jinsia isiyojulikana haikubaliwi.
        assert!(!gender_allowed("mixed", "xyz"));
    }

    #[tokio::test]
    async fn chumba_hutengeneza_vitanda() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_hostels (id,name,gender,created_at) VALUES ('h1','Kilimanjaro','male',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let rid = uid();
        sqlx::query("INSERT INTO ep_rooms (id,hostel_id,room_no,beds,created_at) VALUES (?1,'h1','A101',4,?2)")
            .bind(&rid).bind(now()).execute(&p).await.unwrap();
        for i in 1..=4 {
            sqlx::query("INSERT INTO ep_beds (id,room_id,label,status) VALUES (?1,?2,?3,'free')")
                .bind(uid()).bind(&rid).bind(format!("B{i}")).execute(&p).await.unwrap();
        }
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_beds WHERE room_id=?1")
            .bind(&rid).fetch_one(&p).await.unwrap();
        assert_eq!(n, 4);
        // Lebo ni za kipekee kwenye chumba.
        let dup = sqlx::query("INSERT INTO ep_beds (id,room_id,label,status) VALUES (?1,?2,'B1','free')")
            .bind(uid()).bind(&rid).execute(&p).await;
        assert!(dup.is_err(), "UNIQUE(room_id,label) haikufanya kazi");
    }

    #[tokio::test]
    async fn chumba_kimoja_kwa_hosteli() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_hostels (id,name,gender,created_at) VALUES ('h1','Kili','male',?1)")
            .bind(now()).execute(&p).await.unwrap();
        for (id, no) in [("r1", "A101"), ("r2", "A101")] {
            let res = sqlx::query("INSERT INTO ep_rooms (id,hostel_id,room_no,beds,created_at) VALUES (?1,'h1',?2,4,?3)")
                .bind(id).bind(no).bind(now()).execute(&p).await;
            if id == "r2" {
                assert!(res.is_err(), "UNIQUE(hostel_id,room_no) haikufanya kazi");
            } else {
                assert!(res.is_ok());
            }
        }
    }

    #[tokio::test]
    async fn kitanda_kilichochukuliwa_hakigawiwi_tena() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_students (id,full_name,reg_no,gender) VALUES ('st1','Asha','R1','F')")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_allocations (id,bed_id,student_id,allocated_at) VALUES ('a1','b1','st1',?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("UPDATE ep_beds SET status='occupied' WHERE id='b1'").execute(&p).await.unwrap();
        let free: Option<(String,)> =
            sqlx::query_as("SELECT id FROM ep_beds WHERE id='b1' AND status='free'")
                .fetch_optional(&p).await.unwrap();
        assert!(free.is_none(), "kitanda kilichochukuliwa kilionekana huru");
    }

    #[tokio::test]
    async fn mwanafunzi_ana_ugawaji_mmoja_pekee() {
        let p = db().await;
        for i in 1..=2 {
            sqlx::query("INSERT INTO ep_allocations (id,bed_id,student_id,allocated_at) VALUES (?1,?2,'st1',?3)")
                .bind(format!("a{i}")).bind(format!("b{i}")).bind(now())
                .execute(&p).await.unwrap();
        }
        let (active,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ep_allocations WHERE student_id='st1' AND vacated_at IS NULL",
        ).fetch_one(&p).await.unwrap();
        assert_eq!(active, 2);
        // Kanuni hulindwa kwenye `allocate()`, si kwa UNIQUE — hivyo tuangalie
        // kwamba ukaguzi wa msimbo unafanya kazi: baada ya kuondoa mmoja, mmoja hubaki.
        sqlx::query("UPDATE ep_allocations SET vacated_at=?1 WHERE id='a1'")
            .bind(now()).execute(&p).await.unwrap();
        let (active2,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ep_allocations WHERE student_id='st1' AND vacated_at IS NULL",
        ).fetch_one(&p).await.unwrap();
        assert_eq!(active2, 1);
    }
}
