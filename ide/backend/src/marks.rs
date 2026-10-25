//! marks.rs — ELIMUPRO Awamu 6: alama za mtindo wa Excel (moduli 15).
//!
//! Mwalimu huona jedwali (grid) la alama: wanafunzi kwa safu, vipimo kwa
//! nguzo (T1, T2, A1, A2, Final...). Kila nguzo ina uzito wake. Jumla na
//! daraja (NECTA: A/B/C/D/F) huhesabiwa kiotomatiki. Huweza kuhamisha
//! (export) kwenda CSV kwa Excel/Sheets.
//!
//! Tables: ep_marks_columns ep_marks

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_marks_columns (
            id TEXT PRIMARY KEY,
            level_id TEXT NOT NULL DEFAULT '',
            subject_id TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'other',  -- t1|t2|a1|a2|final|other
            label TEXT NOT NULL,
            max_marks REAL NOT NULL DEFAULT 100,
            weight REAL NOT NULL DEFAULT 1,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_marks (
            id TEXT PRIMARY KEY,
            column_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            score REAL NOT NULL DEFAULT -1,
            entered_by TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL,
            UNIQUE(column_id, student_id)
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/marks/status", get(status))
        .route("/api/marks/columns", get(columns_list))
        .route("/api/marks/column/add", post(column_add))
        .route("/api/marks/column/delete", post(column_delete))
        .route("/api/marks/column/edit", post(column_edit))
        .route("/api/marks/gradebook", get(gradebook))
        .route("/api/marks/set", post(mark_set))
        .route("/api/marks/bulk", post(mark_bulk))
        .route("/api/marks/student/:id", get(student_report))
        .route("/api/marks/export.csv", get(export_csv))
}

// ============================================================
// HELPERS
// ============================================================

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

/// Daraja la NECTA kutoka wastani wa asilimia.
pub fn necta_grade(pct: f64) -> &'static str {
    match pct {
        p if p >= 75.0 => "A",
        p if p >= 65.0 => "B",
        p if p >= 45.0 => "C",
        p if p >= 30.0 => "D",
        p if p >= 0.0 => "F",
        _ => "-",
    }
}

/// Aina za nguzo zinazokubalika (T1,T2,A1,A2,Final).
fn norm_kind(k: &str) -> &'static str {
    match k.trim().to_lowercase().as_str() {
        "t1" | "term1" | "muhula1" => "t1",
        "t2" | "term2" | "muhula2" => "t2",
        "a1" | "annual1" => "a1",
        "a2" | "annual2" => "a2",
        "final" | "mwisho" => "final",
        _ => "other",
    }
}

fn kind_label(k: &str) -> &'static str {
    match k {
        "t1" => "T1",
        "t2" => "T2",
        "a1" => "A1",
        "a2" => "A2",
        "final" => "Final",
        _ => "Nyingine",
    }
}

/// Hukokotoa jumla kwa uzito. Hurudisha (total, max, pct, grade):
///   total = jumla ya alama halisi zilizoingizwa
///   max   = jumla ya alama za juu za nguzo zenye alama
///   pct   = asilimia yenye uzito (hii huamua daraja)
pub fn compute_total(
    cols: &[(String, f64, f64)], // (column_id, max_marks, weight)
    scores: &std::collections::HashMap<String, f64>,
) -> (f64, f64, f64, String) {
    let mut total = 0.0;
    let mut max = 0.0;
    let mut wsum = 0.0;
    let mut wmax = 0.0;
    for (cid, m, w) in cols {
        let sc = scores.get(cid).copied().unwrap_or(-1.0);
        if sc < 0.0 {
            continue;
        }
        let pct = if *m > 0.0 {
            (sc / m * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        total += sc;
        max += m;
        wsum += pct * w;
        wmax += w;
    }
    if wmax <= 0.0 {
        return (total, max, -1.0, "-".into());
    }
    let pct = wsum / wmax;
    (total, max, pct, necta_grade(pct).to_string())
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "columns": table_count(&s.db, "ep_marks_columns").await,
        "marks": table_count(&s.db, "ep_marks").await,
        "levels": sqlx::query_as::<_, (i64,)>("SELECT COUNT(DISTINCT level_id) FROM ep_marks_columns")
            .fetch_one(&s.db).await.map(|r| r.0).unwrap_or(0),
    }))
}

// ============================================================
// NGUZO (columns)
// ============================================================

#[derive(Deserialize)]
pub struct ColIn {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub max_marks: f64,
    #[serde(default)]
    pub weight: f64,
}

pub async fn column_add(State(s): State<AppState>, Json(b): Json<ColIn>) -> Json<Value> {
    let kind = norm_kind(&b.kind);
    let label = if b.label.trim().is_empty() {
        kind_label(kind).to_string()
    } else {
        b.label.trim().to_string()
    };
    let pos: i64 = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(MAX(position),0)+1 FROM ep_marks_columns WHERE level_id=?1 AND subject_id=?2",
    )
    .bind(&b.level_id)
    .bind(&b.subject_id)
    .fetch_one(&s.db)
    .await
    .map(|r| r.0)
    .unwrap_or(1);
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight,position,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&id)
    .bind(&b.level_id)
    .bind(&b.subject_id)
    .bind(kind)
    .bind(&label)
    .bind(if b.max_marks > 0.0 { b.max_marks } else { 100.0 })
    .bind(if b.weight > 0.0 { b.weight } else { 1.0 })
    .bind(pos)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "kind": kind, "label": label, "position": pos }))
}

#[derive(Deserialize)]
pub struct ColQuery {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub subject_id: String,
}

pub async fn columns_list(State(s): State<AppState>, Query(q): Query<ColQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, f64, f64, i64)>(
        "SELECT id,kind,label,subject_id,max_marks,weight,position FROM ep_marks_columns
         WHERE (?1='' OR level_id=?1) AND (?2='' OR subject_id=?2)
         ORDER BY position",
    )
    .bind(&q.level_id)
    .bind(&q.subject_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "columns": rows.into_iter().map(|r| json!({
            "id": r.0, "kind": r.1, "label": r.2, "subject_id": r.3,
            "max_marks": r.4, "weight": r.5, "position": r.6
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct ColDel {
    pub id: String,
}

pub async fn column_delete(State(s): State<AppState>, Json(b): Json<ColDel>) -> Json<Value> {
    let _ = sqlx::query("DELETE FROM ep_marks WHERE column_id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    let _ = sqlx::query("DELETE FROM ep_marks_columns WHERE id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct ColEdit {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub max_marks: Option<f64>,
    #[serde(default)]
    pub weight: Option<f64>,
}

/// Hubadilisha jina/uzito/kiwango cha juu cha nguzo — bila kugusa alama.
pub async fn column_edit(State(s): State<AppState>, Json(b): Json<ColEdit>) -> Json<Value> {
    if b.id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "id ya nguzo inahitajika" }));
    }
    let cur = sqlx::query_as::<_, (String, f64, f64)>(
        "SELECT label,max_marks,weight FROM ep_marks_columns WHERE id=?1",
    )
    .bind(b.id.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((cur_label, cur_max, cur_w)) = cur else {
        return Json(json!({ "ok": false, "error": "nguzo haipo" }));
    };
    let label = if b.label.trim().is_empty() { cur_label } else { b.label.trim().to_string() };
    let max_marks = match b.max_marks {
        Some(m) if m > 0.0 => m,
        Some(_) => return Json(json!({ "ok": false, "error": "max_marks lazima > 0" })),
        None => cur_max,
    };
    let weight = match b.weight {
        Some(w) if w > 0.0 => w,
        Some(_) => return Json(json!({ "ok": false, "error": "weight lazima > 0" })),
        None => cur_w,
    };
    let _ = sqlx::query("UPDATE ep_marks_columns SET label=?1,max_marks=?2,weight=?3 WHERE id=?4")
        .bind(&label)
        .bind(max_marks)
        .bind(weight)
        .bind(b.id.trim())
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "label": label, "max_marks": max_marks, "weight": weight }))
}

// ============================================================
// JEDWALI (gradebook)
// ============================================================

async fn columns_for(db: &SqlitePool, level_id: &str, subject_id: &str) -> Vec<(String, String, String, f64, f64)> {
    sqlx::query_as::<_, (String, String, String, f64, f64)>(
        "SELECT id,kind,label,max_marks,weight FROM ep_marks_columns
         WHERE level_id=?1 AND subject_id=?2 ORDER BY position",
    )
    .bind(level_id)
    .bind(subject_id)
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

async fn students_for(db: &SqlitePool, level_id: &str, subject_id: &str) -> Vec<(String, String, String)> {
    // Wanafunzi wa level, walioandikishwa kwenye somo (au wote wa level kama
    // hakuna usajili) — hii huzuia jedwali tupu.
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT s.id, s.full_name, s.reg_no FROM ep_students s
         WHERE s.level_id=?1
           AND (NOT EXISTS (SELECT 1 FROM ep_enrollments e WHERE e.subject_id=?2)
                OR EXISTS (SELECT 1 FROM ep_enrollments e WHERE e.student_id=s.id AND e.subject_id=?2))
         ORDER BY s.full_name",
    )
    .bind(level_id)
    .bind(subject_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    rows
}

pub async fn gradebook(State(s): State<AppState>, Query(q): Query<ColQuery>) -> Json<Value> {
    let cols = columns_for(&s.db, &q.level_id, &q.subject_id).await;
    let students = students_for(&s.db, &q.level_id, &q.subject_id).await;
    let col_specs: Vec<(String, f64, f64)> = cols.iter().map(|c| (c.0.clone(), c.3, c.4)).collect();

    let mut rows: Vec<Value> = Vec::new();
    for (sid, name, reg) in &students {
        let marks = sqlx::query_as::<_, (String, f64)>(
            "SELECT column_id,score FROM ep_marks WHERE student_id=?1",
        )
        .bind(sid)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
        let mut scores = std::collections::HashMap::new();
        for (cid, sc) in &marks {
            scores.insert(cid.clone(), *sc);
        }
        let (total, max, pct, grade) = compute_total(&col_specs, &scores);
        let mut cell: Vec<Value> = Vec::new();
        for c in &cols {
            cell.push(json!({
                "column_id": c.0,
                "score": scores.get(&c.0).copied().unwrap_or(-1.0)
            }));
        }
        rows.push(json!({
            "student_id": sid, "student": name, "reg_no": reg,
            "marks": cell, "total": total, "max": max, "percent": pct, "grade": grade
        }));
    }

    Json(json!({
        "ok": true,
        "columns": cols.into_iter().map(|c| json!({
            "id": c.0, "kind": c.1, "label": c.2, "max_marks": c.3, "weight": c.4
        })).collect::<Vec<_>>(),
        "rows": rows
    }))
}

// ============================================================
// KUWEKA ALAMA
// ============================================================

#[derive(Deserialize)]
pub struct MarkIn {
    pub column_id: String,
    pub student_id: String,
    pub score: f64,
    #[serde(default)]
    pub entered_by: String,
}

pub async fn mark_set(State(s): State<AppState>, Json(b): Json<MarkIn>) -> Json<Value> {
    let col = sqlx::query_as::<_, (f64,)>("SELECT max_marks FROM ep_marks_columns WHERE id=?1")
        .bind(&b.column_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((max,)) = col else {
        return Json(json!({ "ok": false, "error": "nguzo haipo" }));
    };
    // -1 huruhusiwa (haijafanywa); vinginevyo hukatwa kwenye max.
    let score = if b.score < 0.0 { -1.0 } else { b.score.min(max) };
    let _ = sqlx::query(
        "INSERT INTO ep_marks (id,column_id,student_id,score,entered_by,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(column_id, student_id) DO UPDATE SET score=excluded.score, updated_at=excluded.updated_at",
    )
    .bind(uid())
    .bind(&b.column_id)
    .bind(&b.student_id)
    .bind(score)
    .bind(&b.entered_by)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "score": score }))
}

#[derive(Deserialize)]
pub struct BulkIn {
    pub level_id: String,
    #[serde(default)]
    pub subject_id: String,
    pub marks: Vec<MarkIn>,
}

pub async fn mark_bulk(State(s): State<AppState>, Json(b): Json<BulkIn>) -> Json<Value> {
    let mut saved = 0;
    for m in &b.marks {
        let col = sqlx::query_as::<_, (f64,)>("SELECT max_marks FROM ep_marks_columns WHERE id=?1")
            .bind(&m.column_id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten();
        let Some((max,)) = col else { continue };
        let score = if m.score < 0.0 { -1.0 } else { m.score.min(max) };
        let _ = sqlx::query(
            "INSERT INTO ep_marks (id,column_id,student_id,score,entered_by,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(column_id, student_id) DO UPDATE SET score=excluded.score, updated_at=excluded.updated_at",
        )
        .bind(uid())
        .bind(&m.column_id)
        .bind(&m.student_id)
        .bind(score)
        .bind(&m.entered_by)
        .bind(now())
        .execute(&s.db)
        .await;
        saved += 1;
    }
    Json(json!({ "ok": true, "saved": saved }))
}

// ============================================================
// RIPOTI YA MWANAFUNZI
// ============================================================

pub async fn student_report(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let st = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT full_name, reg_no, level_id, course_id FROM ep_students WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((name, reg, level_id, course_id)) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hapo" }));
    };
    let marks = sqlx::query_as::<_, (String, String, String, f64, f64, f64)>(
        "SELECT c.id, c.subject_id, c.label, m.score, c.max_marks, c.weight
         FROM ep_marks m JOIN ep_marks_columns c ON c.id = m.column_id
         WHERE m.student_id=?1 ORDER BY c.subject_id, c.position",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    // Kwa kila somo: jumla na daraja.
    let mut by_subject: std::collections::BTreeMap<String, Vec<(String, f64, f64)>> = Default::default();
    let mut entries: Vec<Value> = Vec::new();
    for (cid, subject_id, label, score, max, weight) in &marks {
        by_subject
            .entry(subject_id.clone())
            .or_default()
            .push((cid.clone(), *max, *weight));
        entries.push(json!({
            "column_id": cid, "subject_id": subject_id, "label": label,
            "score": score, "max_marks": max, "weight": weight
        }));
    }
    let mut subjects: Vec<Value> = Vec::new();
    for (subject_id, cols) in &by_subject {
        let mut scores = std::collections::HashMap::new();
        for (cid, _, _) in cols {
            let sc = marks.iter().find(|m| &m.0 == cid).map(|m| m.3).unwrap_or(-1.0);
            scores.insert(cid.clone(), sc);
        }
        let (_, _, pct, grade) = compute_total(cols, &scores);
        let subj_name = sqlx::query_as::<_, (String,)>("SELECT name FROM ep_subjects WHERE id=?1")
            .bind(subject_id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
            .map(|r| r.0)
            .unwrap_or_else(|| subject_id.clone());
        subjects.push(json!({
            "subject_id": subject_id, "subject": subj_name,
            "percent": pct, "grade": grade
        }));
    }
    Json(json!({
        "ok": true,
        "student": { "id": id, "name": name, "reg_no": reg, "level_id": level_id, "course_id": course_id },
        "entries": entries,
        "subjects": subjects
    }))
}

// ============================================================
// EXPORT CSV
// ============================================================

/// Huzuia CSV injection (formula kwenye Excel) kwa kuweka apostrofi.
fn csv_cell(s: &str) -> String {
    let needs_quote = s.contains(',') || s.contains('"') || s.contains('\n');
    let safe = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    if needs_quote || safe != s {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}

pub async fn export_csv(State(s): State<AppState>, Query(q): Query<ColQuery>) -> Response {
    let cols = columns_for(&s.db, &q.level_id, &q.subject_id).await;
    let students = students_for(&s.db, &q.level_id, &q.subject_id).await;
    let col_specs: Vec<(String, f64, f64)> = cols.iter().map(|c| (c.0.clone(), c.3, c.4)).collect();

    let mut out = String::from("Reg No,Jina");
    for c in &cols {
        out.push(',');
        out.push_str(&csv_cell(&format!("{} (/{})", c.2, c.3)));
    }
    out.push_str(",Jumla,Daraja\n");

    for (sid, name, reg) in &students {
        let marks = sqlx::query_as::<_, (String, f64)>("SELECT column_id,score FROM ep_marks WHERE student_id=?1")
            .bind(sid)
            .fetch_all(&s.db)
            .await
            .unwrap_or_default();
        let mut scores = std::collections::HashMap::new();
        for (cid, sc) in &marks {
            scores.insert(cid.clone(), *sc);
        }
        let (total, _max, _pct, grade) = compute_total(&col_specs, &scores);
        out.push_str(&csv_cell(reg));
        out.push(',');
        out.push_str(&csv_cell(name));
        for c in &cols {
            out.push(',');
            let sc = scores.get(&c.0).copied().unwrap_or(-1.0);
            out.push_str(&if sc < 0.0 { String::new() } else { format!("{sc}") });
        }
        out.push(',');
        out.push_str(&format!("{total:.1}"));
        out.push(',');
        out.push_str(&grade);
        out.push('\n');
    }

    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"alama.csv\"",
            ),
        ],
        out,
    )
        .into_response()
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    fn cols(v: &[(&str, f64, f64)]) -> Vec<(String, f64, f64)> {
        v.iter().map(|(a, b, c)| (a.to_string(), *b, *c)).collect()
    }

    #[tokio::test]
    async fn init_tables_huunda_majedwali_yote() {
        let p = db().await;
        for t in ["ep_marks_columns", "ep_marks"] {
            assert_eq!(table_count(&p, t).await, 0, "{t} haipo");
        }
    }

    #[test]
    fn daraja_la_necta_ni_sahihi() {
        assert_eq!(necta_grade(80.0), "A");
        assert_eq!(necta_grade(75.0), "A");
        assert_eq!(necta_grade(70.0), "B");
        assert_eq!(necta_grade(50.0), "C");
        assert_eq!(necta_grade(35.0), "D");
        assert_eq!(necta_grade(10.0), "F");
        assert_eq!(necta_grade(-1.0), "-");
    }

    #[test]
    fn aina_za_nguzo_hutafsiriwa() {
        assert_eq!(norm_kind("T1"), "t1");
        assert_eq!(norm_kind("term2"), "t2");
        assert_eq!(norm_kind("Annual1"), "a1");
        assert_eq!(norm_kind("A2"), "a2");
        assert_eq!(norm_kind("FINAL"), "final");
        assert_eq!(norm_kind("kitu"), "other");
        assert_eq!(kind_label("t1"), "T1");
        assert_eq!(kind_label("final"), "Final");
    }

    #[test]
    fn jumla_hupima_kwa_uzito() {
        // T1 (100, uzito 1) na Final (100, uzito 2).
        let c = cols(&[("t1", 100.0, 1.0), ("final", 100.0, 2.0)]);
        let mut sc = HashMap::new();
        sc.insert("t1".to_string(), 60.0);
        sc.insert("final".to_string(), 90.0);
        let (_t, _m, pct, grade) = compute_total(&c, &sc);
        // (60*1 + 90*2) / 3 = 80
        assert!((pct - 80.0).abs() < 0.01, "pct={pct}");
        assert_eq!(grade, "A");
    }

    #[test]
    fn alama_zisizojulikana_hazijumuishwi() {
        let c = cols(&[("t1", 100.0, 1.0), ("t2", 100.0, 1.0)]);
        let mut sc = HashMap::new();
        sc.insert("t1".to_string(), 50.0);
        // t2 haipo → -1, haijumuishwi.
        let (_, _, pct, grade) = compute_total(&c, &sc);
        assert!((pct - 50.0).abs() < 0.01);
        assert_eq!(grade, "C");
    }

    #[test]
    fn max_marks_tofauti_hurekebisha_asilimia() {
        // T1 ni /20, Final ni /100, uzito sawa.
        let c = cols(&[("t1", 20.0, 1.0), ("final", 100.0, 1.0)]);
        let mut sc = HashMap::new();
        sc.insert("t1".to_string(), 20.0); // 100%
        sc.insert("final".to_string(), 50.0); // 50%
        let (_, _, pct, grade) = compute_total(&c, &sc);
        assert!((pct - 75.0).abs() < 0.01, "pct={pct}");
        assert_eq!(grade, "A");
    }

    #[test]
    fn hakuna_alama_huwa_daraja_dash() {
        let c = cols(&[("t1", 100.0, 1.0)]);
        let sc = HashMap::new();
        let (_, _, pct, grade) = compute_total(&c, &sc);
        assert_eq!(pct, -1.0);
        assert_eq!(grade, "-");
    }

    #[test]
    fn csv_cell_huzuia_formula() {
        assert_eq!(csv_cell("Juma"), "Juma");
        assert_eq!(csv_cell("=1+1"), "\"'=1+1\"");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("a\"b"), "\"a\"\"b\"");
    }

    #[tokio::test]
    async fn alama_husasishwa_si_kurudia() {
        let p = db().await;
        let cid = uid();
        sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight,position,created_at)
                     VALUES (?1,'lv','s','t1','T1',100,1,1,?2)")
            .bind(&cid).bind(now()).execute(&p).await.unwrap();
        for sc in [40.0, 70.0] {
            sqlx::query(
                "INSERT INTO ep_marks (id,column_id,student_id,score,updated_at) VALUES (?1,?2,'st1',?3,?4)
                 ON CONFLICT(column_id,student_id) DO UPDATE SET score=excluded.score",
            ).bind(uid()).bind(&cid).bind(sc).bind(now()).execute(&p).await.unwrap();
        }
        let (sc,): (f64,) = sqlx::query_as("SELECT score FROM ep_marks WHERE column_id=?1 AND student_id='st1'")
            .bind(&cid).fetch_one(&p).await.unwrap();
        assert_eq!(sc, 70.0);
    }

    #[tokio::test]
    async fn nguzo_hupata_nafasi_inayofuata() {
        let p = db().await;
        for expect in 1..=3 {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(MAX(position),0)+1 FROM ep_marks_columns WHERE level_id='lv' AND subject_id='s'",
            ).fetch_one(&p).await.unwrap();
            assert_eq!(n, expect);
            sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,position,created_at)
                         VALUES (?1,'lv','s','t1','T1',?2,?3)")
                .bind(uid()).bind(n).bind(now()).execute(&p).await.unwrap();
        }
    }
}
