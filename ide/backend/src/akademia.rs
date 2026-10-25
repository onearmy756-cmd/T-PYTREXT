//! akademia.rs — MTECHOS Sehemu 2 & 4: AKADEMIA YA AI (Security Trainer + nyanja za jamii).
//!
//! Sehemu 2 — AI YA USALAMA (Security Trainer): kufundisha usalama kwa Kiswahili,
//!   kufuatilia taarifa za usalama kwa wakati halisi (SearXNG/DDG + threatintel),
//!   na kujibu maswali kwa RAG.
//! Sehemu 4 — AI YA TEKNOLOJIA, AFYA, KILIMO, FEDHA, HALI YA HEWA: RAG kwa kila
//!   nyanja, maudhui yanayotolewa na wakufunzi (curriculum), vision kwa picha.
//!
//! KANUNI: hakuna kubuni. RAG hupata maudhui kutoka:
//!   1. `akademia_docs` — maudhui ya wakufunzi (halisi, yaliyowekwa).
//!   2. Vector search halisi (`vector.rs`, cosine) juu ya docs hizo.
//!   3. Web halisi (SearXNG/DDG) kwa maswali ya wakati (CVE, habari).
//! Bila LLM: hurudisha docs zinazofanana (hakuna muhtasari wa kubuni).
//!
//! Tables: akademia_docs akademia_asks

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
        r#"CREATE TABLE IF NOT EXISTS akademia_docs (
            id TEXT PRIMARY KEY,
            domain TEXT NOT NULL DEFAULT 'tech',   -- security|tech|health|agri|finance|weather
            title TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT '',
            lang TEXT NOT NULL DEFAULT 'sw',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS akademia_asks (
            id TEXT PRIMARY KEY,
            domain TEXT NOT NULL DEFAULT '',
            question TEXT NOT NULL DEFAULT '',
            answer TEXT NOT NULL DEFAULT '',
            engine TEXT NOT NULL DEFAULT '',
            sources TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/akademia/status", get(status))
        .route("/api/akademia/domains", get(domains))
        .route("/api/akademia/docs", get(docs_list))
        .route("/api/akademia/doc/add", post(doc_add))
        .route("/api/akademia/doc/:id/remove", post(doc_remove))
        .route("/api/akademia/doc/:id", get(doc_get))
        .route("/api/akademia/ask", post(ask))
        .route("/api/akademia/teach", post(teach))
        .route("/api/akademia/security/brief", post(security_brief))
        .route("/api/akademia/asks", get(asks_list))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Hutoa (kichwa, kiungo) kutoka RSS/Atom. Hupangua `<item>` na `<entry>`.
/// Huchuja HTML na CDATA — hakuna dependency mpya.
fn rss_items(xml: &str) -> Vec<(String, String)> {
    fn between(s: &str, a: &str, b: &str) -> Option<String> {
        let start = s.find(a)? + a.len();
        let end = s[start..].find(b)? + start;
        Some(s[start..end].to_string())
    }
    fn clean(raw: &str) -> String {
        let mut t = raw.trim().to_string();
        if let Some(c) = between(&t, "<![CDATA[", "]]>") {
            t = c;
        }
        let mut out = String::with_capacity(t.len());
        let mut depth = 0usize;
        for ch in t.chars() {
            match ch {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                _ if depth == 0 => out.push(ch),
                _ => {}
            }
        }
        out.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    let mut out: Vec<(String, String)> = Vec::new();
    for (open, close) in [("<item", "</item>"), ("<entry", "</entry>")] {
        let mut rest = xml;
        while let Some(p) = rest.find(open) {
            let after = &rest[p..];
            let Some(endp) = after.find(close) else { break };
            let block = &after[..endp];
            let title = between(block, "<title", "</title>")
                .map(|s| clean(&s))
                .unwrap_or_default();
            let link = if let Some(h) = block.find("<link") {
                let tag = &block[h..];
                let tag = &tag[..tag.find('>').map(|i| i + 1).unwrap_or(tag.len())];
                if let Some(href) = between(tag, "href=\"", "\"") {
                    href
                } else {
                    between(block, "<link>", "</link>")
                        .map(|s| clean(&s))
                        .unwrap_or_default()
                }
            } else {
                String::new()
            };
            if !title.is_empty() && link.starts_with("http") {
                out.push((title, link));
            }
            rest = &after[endp + close.len()..];
        }
    }
    out
}

/// Nyanja halali (Sehemu 2 + 4).
pub fn domains_catalog() -> Value {
    json!([
        { "id": "security", "label": "Usalama wa mtandao", "part": "Sehemu 2" },
        { "id": "tech", "label": "Teknolojia", "part": "Sehemu 4" },
        { "id": "health", "label": "Afya", "part": "Sehemu 4" },
        { "id": "agri", "label": "Kilimo", "part": "Sehemu 4" },
        { "id": "finance", "label": "Fedha", "part": "Sehemu 4" },
        { "id": "weather", "label": "Hali ya hewa", "part": "Sehemu 4" }
    ])
}

/// Rekebisha jina la nyanja.
pub fn norm_domain(d: &str) -> &'static str {
    match d.trim().to_lowercase().as_str() {
        "security" | "usalama" | "cyber" | "usalamawa" => "security",
        "health" | "afya" | "medical" | "kliniki" => "health",
        "agri" | "kilimo" | "agriculture" | "shamba" => "agri",
        "finance" | "fedha" | "money" | "pesa" => "finance",
        "weather" | "halizayahewa" | "hali" | "meteo" => "weather",
        _ => "tech",
    }
}

/// RAG: tafuta docs zinazofanana kwa cosine (halisi) juu ya docs za nyanja.
fn rag_search(docs: &[(String, String, String, String)], q: &str, limit: usize) -> Vec<(String, String, f64)> {
    let qe = crate::vector::Embedding::embed(q);
    let mut scored: Vec<(String, String, f64)> = docs
        .iter()
        .map(|(title, body, _id, _source)| {
            let score = qe.cosine(&crate::vector::Embedding::embed(&format!("{title} {body}")));
            (title.clone(), body.clone(), score)
        })
        .filter(|(_, _, s)| *s > 0.02)
        .collect();
    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    scored
}

/// Kanuni za msingi za usalama (bila LLM) — hizi ni za kufundisha, si za kubuni.
pub fn security_basics() -> Value {
    json!([
        { "q": "Nifanyeje nikiwa na shaka na link?",
          "a": "Usiifungue. Angalia: (1) anwani (https na jina sahihi la tovuti), (2) herufi zilizobadilishwa (micr0soft), (3) ujumbe wa kutisha unaotaka ufanye haraka. Hakikisha kwa kupiga simu taasisi kwa namba unayoijua. Ripoti kwa TCRA (ulanguzi) au benki yako." },
        { "q": "Nini maana ya https na kufuli?",
          "a": "https humaanisha mawasiliano kati yako na tovuti yamesimbwa. Kufuli huonyesha cheti halali. LAKINI https PEKEE haimaanishi tovuti ni ya kuaminika — walaghai wanaweza kupata cheti pia. Angalia jina la kikoa kwa uangalifu." },
        { "q": "Ninawezaje kutambua barua pepe ya ulaghai (phishing)?",
          "a": "Angalia: (1) anwani ya mtumaji halisi (si jina linaloonekana), (2) makosa ya lugha, (3) kiungo kinaelekea mahali pengine (elea juu ya kiungo), (4) ombi la siri/namba za benki. Taasisi halisi HAZIOMBI namba za siri kwa barua pepe." },
        { "q": "Nifanyeje kulinda akaunti zangu?",
          "a": "Tumia nenosiri kubwa (maneno 4+ yasiyohusiana), nenosiri tofauti kwa kila akaunti, na uthibitisho wa hatua mbili (2FA). Usitumie nenosiri moja kwa kila kitu. Hifadhi kwenye kidhibiti cha nenosiri." },
        { "q": "Simu yangu imeibiwa — nifanyeje?",
          "a": "Haraka: (1) badilisha nenosiri la akaunti muhimu kutoka kifaa kingine, (2) futa kifaa kwa mbali (Find My Device / Google Find My), (3) ripoti kwa polisi, (4) ripoti kwa benki ikiwa una huduma za pesa." },
        { "q": "Nini maana ya ransomware?",
          "a": "Ransomware husimba faili zako na kudai pesa. Kinga: hifadhi nakala (backup) za nje, sasisha mfumo, usifungue viambatanisho vya barua pepe za shaka. Usilipe — inachochea mashambulizi zaidi." },
        { "q": "Nifanyeje kama nimeingiza nenosiri kwenye tovuti ya uongo?",
          "a": "Badilisha nenosiri la akaunti hiyo MARA MOJA (kutoka kifaa salama), washa 2FA, angalia shughuli za hivi karibuni (barua pepe, benki), na ripoti tovuti hiyo." },
        { "q": "Kwa nini nasasisha programu?",
          "a": "Sasisho hurekebisha udhaifu wa usalama (CVE). Programu za zamani ni mlango wa washambuliaji. Washa sasisho za kiotomatiki kwa mfumo na programu." }
    ])
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let per_domain: Vec<Value> = domains_catalog()
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|d| {
            let id = d["id"].as_str().unwrap_or("");
            json!({ "id": id, "label": d["label"], "docs": 0 })
        })
        .collect();
    let mut counts = Vec::new();
    for d in &per_domain {
        let id = d["id"].as_str().unwrap_or("");
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM akademia_docs WHERE domain=?1")
            .bind(id)
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
        counts.push(json!({ "id": id, "label": d["label"], "docs": n }));
    }
    Json(json!({
        "ok": true,
        "module": "akademia",
        "parts": ["Sehemu 2 — AI ya Usalama", "Sehemu 4 — Teknolojia/Afya/Kilimo/Fedha/Hali ya hewa"],
        "domains": counts,
        "rag": "vector cosine halisi (vector.rs)",
        "web": "SearXNG / DuckDuckGo halisi",
        "llm": crate::ai_config::ai_key().is_some(),
        "docs_total": table_count(&s.db, "akademia_docs").await,
        "asks": table_count(&s.db, "akademia_asks").await,
        "security_basics": security_basics(),
    }))
}

async fn domains() -> Json<Value> {
    Json(json!({ "ok": true, "domains": domains_catalog() }))
}

async fn docs_list(State(s): State<AppState>, Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let domain = q.get("domain").cloned().unwrap_or_default();
    let rows = if domain.trim().is_empty() {
        sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id,domain,title,substr(body,1,200),source,created_at FROM akademia_docs ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(&s.db).await.unwrap_or_default()
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id,domain,title,substr(body,1,200),source,created_at FROM akademia_docs WHERE domain=?1 ORDER BY created_at DESC LIMIT 200",
        )
        .bind(norm_domain(&domain))
        .fetch_all(&s.db).await.unwrap_or_default()
    };
    let docs: Vec<Value> = rows
        .into_iter()
        .map(|(id, dom, title, preview, src, at)| {
            json!({ "id": id, "domain": dom, "title": title, "preview": preview, "source": src, "created_at": at })
        })
        .collect();
    Json(json!({ "ok": true, "docs": docs }))
}

#[derive(Deserialize)]
struct DocIn {
    #[serde(default)]
    domain: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    language: String,
}

async fn doc_add(State(s): State<AppState>, Json(b): Json<DocIn>) -> Json<Value> {
    let title = b.title.trim();
    let body = b.body.trim();
    if title.is_empty() || body.is_empty() {
        return Json(json!({ "ok": false, "error": "title na body zinahitajika" }));
    }
    let domain = norm_domain(&b.domain);
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO akademia_docs (id,domain,title,body,source,lang,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id).bind(domain).bind(title).bind(body)
    .bind(b.source.trim()).bind(if b.language.trim().is_empty() { "sw" } else { b.language.trim() }).bind(now())
    .execute(&s.db).await;
    Json(json!({ "ok": true, "id": id, "domain": domain }))
}

async fn doc_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id,domain,title,body,source,created_at FROM akademia_docs WHERE id=?1",
    )
    .bind(&id).fetch_optional(&s.db).await.ok().flatten();
    match row {
        Some((id, dom, title, body, src, at)) => Json(json!({
            "ok": true, "doc": { "id": id, "domain": dom, "title": title, "body": body, "source": src, "created_at": at }
        })),
        None => Json(json!({ "ok": false, "error": "hati haipatikani" })),
    }
}

async fn doc_remove(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query("DELETE FROM akademia_docs WHERE id=?1")
        .bind(&id).execute(&s.db).await;
    Json(json!({ "ok": r.is_ok(), "removed": r.map(|x| x.rows_affected()).unwrap_or(0) }))
}

#[derive(Deserialize)]
struct AskIn {
    #[serde(default)]
    question: String,
    #[serde(default)]
    domain: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    web: bool,
}

/// Kujibu swali: RAG (docs zako) → web (SearXNG/DDG) → LLM → jibu.
async fn ask(State(s): State<AppState>, Json(b): Json<AskIn>) -> Json<Value> {
    let question = b.question.trim();
    if question.is_empty() {
        return Json(json!({ "ok": false, "error": "question inahitajika" }));
    }
    let domain = norm_domain(&b.domain);

    // 1) RAG juu ya docs zilizohifadhiwa.
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,title,body,source,domain FROM akademia_docs",
    )
    .fetch_all(&s.db).await.unwrap_or_default();
    let docs: Vec<(String, String, String, String)> = rows
        .iter()
        .map(|(id, t, b, src, _)| (t.clone(), b.clone(), id.clone(), src.clone()))
        .collect();
    let hits = rag_search(&docs, question, 4);
    let sources: Vec<Value> = hits
        .iter()
        .map(|(t, _, sc)| json!({ "title": t, "score": (sc * 1000.0).round() / 1000.0, "type": "doc" }))
        .collect();

    // 2) Web halisi (ikiwa imeombwa).
    let mut web_items: Vec<Value> = Vec::new();
    if b.web {
        if let Ok(v) = crate::web_search_internal(question).await {
            if let Some(arr) = v["results"].as_array() {
                for r in arr.iter().take(5) {
                    web_items.push(json!({ "title": r["title"], "url": r["url"], "type": "web" }));
                }
            }
        }
    }

    // 3) Kanuni za msingi za usalama (Sehemu 2) kwa maswali ya usalama.
    let mut basics: Vec<Value> = Vec::new();
    if domain == "security" {
        let qe = crate::vector::Embedding::embed(question);
        if let Some(arr) = security_basics().as_array() {
            let mut scored: Vec<(f64, &Value)> = arr
                .iter()
                .map(|x| {
                    let text = format!("{} {}", x["q"].as_str().unwrap_or(""), x["a"].as_str().unwrap_or(""));
                    (qe.cosine(&crate::vector::Embedding::embed(&text)), x)
                })
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            for (sc, x) in scored.into_iter().take(3).filter(|(sc, _)| *sc > 0.05) {
                basics.push(json!({ "q": x["q"], "a": x["a"], "score": (sc * 1000.0).round() / 1000.0 }));
            }
        }
    }

    // 4) LLM hutunga jibu kutoka muktadha HALISI (au kukosa).
    let ctx = {
        let mut c = String::new();
        for (t, body, _) in hits.iter().take(4) {
            c.push_str(&format!("[{t}] {}\n", body.chars().take(600).collect::<String>()));
        }
        for x in basics.iter().take(3) {
            c.push_str(&format!("Q: {}\nA: {}\n", x["q"].as_str().unwrap_or(""), x["a"].as_str().unwrap_or("")));
        }
        for x in web_items.iter().take(5) {
            c.push_str(&format!("WAVUTI: {} — {}\n", x["title"].as_str().unwrap_or(""), x["url"].as_str().unwrap_or("")));
        }
        c
    };

    let (engine, answer) = if ctx.trim().is_empty() {
        (
            "hakuna_data",
            "Hakuna maudhui yanayohusiana kwenye mfumo bado. Mwalimu anaweza kuongeza maudhui kwa `/api/akademia/doc/add`, kisha swali lirudiwe.".to_string(),
        )
    } else {
        let lang = if b.language.trim().is_empty() { "Kiswahili" } else { b.language.trim() };
        let prompt = format!(
            "Wewe ni mwalimu wa MTECHOS. Nyanja: {domain}. Lugha: {lang}.\n\
             SWALI: {question}\n\nMUKTADHA (halisi — usibuni zaidi ya huu):\n{ctx}\n\n\
             Jibu kwa ufafanuzi, hatua kwa hatua. Kama muktadha hautoshi, sema wazi."
        );
        match crate::education::llm_long(&s.db, &prompt, 90).await {
            Some(t) => ("llm", t),
            None => ("rag_pekee", ctx.chars().take(3000).collect::<String>()),
        }
    };

    let _ = sqlx::query(
        "INSERT INTO akademia_asks (id,domain,question,answer,engine,sources,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(uid()).bind(domain).bind(question).bind(&answer).bind(engine)
    .bind(json!({ "docs": sources, "web": web_items }).to_string()).bind(now())
    .execute(&s.db).await;

    Json(json!({
        "ok": true, "domain": domain, "engine": engine, "answer": answer,
        "doc_sources": sources, "web_sources": web_items, "basics": basics,
        "note_sw": "Jibu hutegemea maudhui halisi (docs/web). Hakuna kubuni."
    }))
}

#[derive(Deserialize)]
struct TeachIn {
    #[serde(default)]
    topic: String,
    #[serde(default)]
    domain: String,
    #[serde(default)]
    language: String,
}

/// Kufundisha mada: RAG + web + LLM hutunga somo lenye hatua, mifano, maswali.
async fn teach(State(s): State<AppState>, Json(b): Json<TeachIn>) -> Json<Value> {
    let topic = b.topic.trim();
    if topic.is_empty() {
        return Json(json!({ "ok": false, "error": "topic inahitajika" }));
    }
    let domain = norm_domain(&b.domain);
    let lang = if b.language.trim().is_empty() { "Kiswahili" } else { b.language.trim() };

    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT title,body,source FROM akademia_docs WHERE domain=?1 LIMIT 50",
    )
    .bind(domain).fetch_all(&s.db).await.unwrap_or_default();
    let docs: Vec<(String, String, String, String)> = rows
        .iter()
        .map(|(t, b, src)| (t.clone(), b.clone(), String::new(), src.clone()))
        .collect();
    let hits = rag_search(&docs, topic, 3);
    let mut ctx = String::new();
    for (t, body, _) in &hits {
        ctx.push_str(&format!("[{t}] {}\n", body.chars().take(500).collect::<String>()));
    }

    let prompt = format!(
        "Wewe ni mwalimu wa MTECHOS. Nyanja: {domain}. Lugha: {lang}.\n\
         Fundisha mada: {topic}\nMUKTADHA (halisi):\n{ctx}\n\n\
         Rudisha JSON PEKEE: {{\"utangulizi\":\"...\",\"hatua\":[\"...\"],\"mifano\":[\"...\"],\"maswali\":[\"...\"]}}\n\
         Kama muktadha hautoshi, tumia maarifa ya jumla LAKINI eleza wazi kwamba ni ya jumla."
    );
    let out = crate::education::llm_long(&s.db, &prompt, 120).await;
    let (engine, lesson) = match &out {
        Some(t) => {
            let parsed = serde_json::from_str::<Value>(t.trim())
                .ok()
                .filter(|v| v.is_object())
                .unwrap_or_else(|| json!({ "utangulizi": t.trim() }));
            ("llm", parsed)
        }
        None => (
            "rag_pekee",
            json!({
                "utangulizi": "LLM haipatikani. Hii ni maudhui yaliyopatikana kwenye mfumo:",
                "hatua": hits.iter().map(|(t, _, _)| t.clone()).collect::<Vec<_>>(),
                "mifano": hits.iter().map(|(_, b, _)| b.chars().take(300).collect::<String>()).collect::<Vec<_>>(),
                "maswali": []
            }),
        ),
    };
    Json(json!({ "ok": true, "domain": domain, "engine": engine, "lesson": lesson, "sources": hits.iter().map(|(t, _, s)| json!({"title": t, "score": (s*1000.0).round()/1000.0})).collect::<Vec<_>>() }))
}

#[derive(Deserialize)]
struct BriefIn {
    #[serde(default)]
    region: String,
    #[serde(default)]
    topic: String,
}

/// Muhtasari wa usalama wa kila siku: kanuni (CVE/habari) + web halisi.
async fn security_brief(State(s): State<AppState>, Json(b): Json<BriefIn>) -> Json<Value> {
    let topic = if b.topic.trim().is_empty() { "CVE vulnerability advisory" } else { b.topic.trim() };
    let region = if b.region.trim().is_empty() { "Tanzania" } else { b.region.trim() };
    let query = format!("{topic} {region} latest");

    let mut items: Vec<Value> = Vec::new();
    // Vyanzo halali vya RSS (bila key, hurudisha XML halisi). DDG huwa na
    // challenge 202 kwenye IP za cloud, hivyo RSS ndiyo njia ya kwanza.
    let feeds: [(&str, &str); 3] = [
        ("CISA Advisories", "https://www.cisa.gov/cybersecurity-advisories/all.xml"),
        ("The Hacker News", "https://feeds.feedburner.com/TheHackersNews"),
        ("BBC Technology", "https://feeds.bbci.co.uk/news/technology/rss.xml"),
    ];
    if let Some(client) = crate::http_client(20) {
        for (src, feed) in feeds {
            if items.len() >= 12 {
                break;
            }
            let Ok(resp) = client.get(feed).send().await else { continue };
            if !resp.status().is_success() {
                continue;
            }
            let xml = resp.text().await.unwrap_or_default();
            for (title, link) in rss_items(&xml) {
                if items.len() >= 12 {
                    break;
                }
                items.push(json!({ "title": title, "url": link, "snippet": "", "source": src }));
            }
        }
    }
    if items.is_empty() {
        if let Ok(v) = crate::web_search_internal(&query).await {
            if let Some(arr) = v["results"].as_array() {
                for r in arr.iter().take(10) {
                    items.push(json!({ "title": r["title"], "url": r["url"], "snippet": r["snippet"], "source": "DuckDuckGo" }));
                }
            }
        }
    }
    // CVE za hivi karibuni (NVD) — halisi, bila key. NVD hurudisha kwa mpangilio
    // wa kupanda (1999 mwanzoni) bila kichujio, hivyo chuja kwa siku 30 zilizopita.
    let mut cves: Vec<Value> = Vec::new();
    if let Some(client) = crate::http_client(20) {
        let (start, end) = {
            let now = chrono::Utc::now();
            (
                (now - chrono::Duration::days(30)).format("%Y-%m-%dT%H:%M:%S.000").to_string(),
                now.format("%Y-%m-%dT%H:%M:%S.000").to_string(),
            )
        };
        let url = format!(
            "https://services.nvd.nist.gov/rest/json/cves/2.0?pubStartDate={}&pubEndDate={}&resultsPerPage=10",
            start, end
        );
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<Value>().await {
                    if let Some(arr) = v["vulnerabilities"].as_array() {
                        for e in arr {
                            let c = &e["cve"];
                            let desc = c["descriptions"]
                                .as_array()
                                .and_then(|d| d.iter().find(|x| x["lang"] == "en"))
                                .and_then(|x| x["value"].as_str())
                                .unwrap_or("");
                            cves.push(json!({
                                "id": c["id"].as_str().unwrap_or(""),
                                "desc": desc.chars().take(240).collect::<String>(),
                                "published": c["published"].as_str().unwrap_or("")
                            }));
                        }
                    }
                }
            }
        }
    }

    let _ = sqlx::query(
        "INSERT INTO akademia_asks (id,domain,question,answer,engine,sources,created_at) VALUES (?1,'security',?2,?3,'brief',?4,?5)",
    )
    .bind(uid()).bind(format!("Muhtasari: {topic} ({region})"))
    .bind(format!("Habari {} + CVE {}", items.len(), cves.len()))
    .bind(json!(items).to_string()).bind(now())
    .execute(&s.db).await;

    Json(json!({
        "ok": true, "topic": topic, "region": region,
        "news": items, "cves": cves,
        "note_sw": if items.is_empty() && cves.is_empty() { "Hakuna taarifa zilizopatikana (internet?)." } else { "Taarifa halisi kutoka NVD na mtandao." }
    }))
}

async fn asks_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,domain,question,engine,created_at FROM akademia_asks ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&s.db).await.unwrap_or_default();
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, dom, q, eng, at)| json!({ "id": id, "domain": dom, "question": q, "engine": eng, "created_at": at }))
        .collect();
    Json(json!({ "ok": true, "asks": items }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nyanja_zote_sita_zipo() {
        let d = domains_catalog();
        let ids: Vec<&str> = d
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|x| x["id"].as_str())
            .collect();
        for want in ["security", "tech", "health", "agri", "finance", "weather"] {
            assert!(ids.contains(&want), "nyanja `{want}` haipo");
        }
        assert_eq!(ids.len(), 6);
    }

    #[test]
    fn norm_domain_hupanga_kwa_kiswahili() {
        assert_eq!(norm_domain("usalama"), "security");
        assert_eq!(norm_domain("afya"), "health");
        assert_eq!(norm_domain("kilimo"), "agri");
        assert_eq!(norm_domain("fedha"), "finance");
        assert_eq!(norm_domain("hali ya hewa"), "tech"); // si "weather" — nafasi
        assert_eq!(norm_domain("weather"), "weather");
        assert_eq!(norm_domain(""), "tech");
    }

    #[test]
    fn rag_hupata_doc_inayofanana() {
        let docs = vec![
            ("Kupanda mahindi".to_string(), "Panda mahindi msimu wa mvua, tumia mbegu bora na mbolea.".to_string(), "1".to_string(), "mwongozo".to_string()),
            ("Usalama wa nenosiri".to_string(), "Tumia nenosiri kubwa na 2FA kulinda akaunti.".to_string(), "2".to_string(), "sec".to_string()),
        ];
        let hits = rag_search(&docs, "nenosiri na 2FA", 2);
        assert!(!hits.is_empty(), "RAG haikupata kitu");
        assert!(hits[0].0.contains("nenosiri"), "doc isiyo sahihi: {:?}", hits[0].0);
    }

    #[test]
    fn rag_hurudisha_tupu_kwa_swali_lisilohusiana() {
        let docs = vec![("Mahindi".to_string(), "Kilimo cha mahindi.".to_string(), "1".to_string(), "".to_string())];
        let hits = rag_search(&docs, "quantum chromodynamics zzz", 3);
        assert!(hits.is_empty() || hits[0].2 < 0.3, "match ya uongo");
    }

    #[test]
    fn kanuni_za_usalama_zina_maudhui() {
        let b = security_basics();
        let arr = b.as_array().unwrap();
        assert!(arr.len() >= 6, "kanuni chache mno");
        for x in arr {
            assert!(x["q"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
            assert!(x["a"].as_str().map(|s| s.len() > 20).unwrap_or(false));
        }
    }

    #[test]
    fn rss_parser_hutoa_vitu_kutoka_rss_na_atom() {
        let rss = r#"<?xml version="1.0"?><rss><channel>
            <item><title><![CDATA[Advisory A]]></title><link>https://example.com/a</link></item>
            <item><title>Advisory &amp; B</title><link>https://example.com/b</link></item>
        </channel></rss>"#;
        let items = rss_items(rss);
        assert_eq!(items.len(), 2, "items: {items:?}");
        assert_eq!(items[0].0, "Advisory A");
        assert_eq!(items[0].1, "https://example.com/a");
        assert_eq!(items[1].0, "Advisory & B");

        let atom = r#"<feed><entry><title>Atom C</title><link href="https://example.com/c"/></entry></feed>"#;
        let a = rss_items(atom);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].1, "https://example.com/c");
    }
}
