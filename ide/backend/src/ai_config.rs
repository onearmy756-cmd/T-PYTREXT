//! ai_config.rs — LLM ENGINE (SEHEMU 1.1 + maelezo ya mmiliki):
//!
//! "Qwen 2.5 VL inafanya kazi hata OFFLINE — kompyuta zote zimeunganishwa kwenye
//!  server kuu." — LLM (Qwen 2.5 3B VL) inaishi kwenye SERVER KUU YA LAN
//!  (docker: ollama :11434 / llamacpp :8081). Offline (bila internet) inafanya
//!  kazi kikamilifu — mtandao wa ndani unaotosha. Online ni kwa cloud AI pekee.
//!
//! Endpoints (env-configurable, defaults ni LAN server):
//!   FUNDI_LLM_URL    (default http://127.0.0.1:11434 — Ollama ndani ya server kuu)
//!   FUNDI_LLM_MODEL  (default qwen2.5vl:3b)
//!
//! CLOUD AI (H23): mteja akitaka cloud — anaweka custom URL ya
//! OpenAI-compatible (OpenAI/Groq/DeepSeek/Mistral/…) + API key kwenye env
//!   FUNDI_AI_KEY     (Bearer token kwa cloud APIs; niHIHIFADHIWI kwenye DB)
//! akitumia UI ya 🧠 AI (aiSave/aiLoad); ollama/llamacpp za LAN hazihitaji key.
//!
//! KANUNI: LLM HAIHESABU kamwe — inaeleza, inatafsiri, inajibu maswali.

use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize)]
pub struct LlmConfig {
    pub url: String,
    pub model: String,
    pub offline_capable: bool, // daima true — server kuu ya LAN
}

/// CUSTOM MODEL / API (maelezo ya mmiliki: "weka sehemu ya kuweka custom model,
/// local model au API"): mteja anaweza kuweka endpoint/model yake — inahifadhiwa
/// kwenye DB (settings table) na INATUMIKA mara moja kwenye chat/translate.
/// Hakuna AI key inayohifadhiwa kwenye DB (inabaki kwenye env ya server).
pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS ai_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            url TEXT NOT NULL,
            model TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CustomAi {
    pub url: String,
    pub model: String,
}

/// Weka custom model/API (OpenAI-compatible / Ollama / llama.cpp / vLLM — URL yoyote).
pub async fn set_custom(db: &SqlitePool, c: &CustomAi) -> Result<(), String> {
    let url = c.url.trim().trim_end_matches('/').to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL lazima ianze na http:// au https://".into());
    }
    if c.model.trim().is_empty() {
        return Err("jina la model ni lazima".into());
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO ai_settings (id, url, model, updated_at) VALUES (1,?,?,?)
         ON CONFLICT(id) DO UPDATE SET url=excluded.url, model=excluded.model, updated_at=excluded.updated_at",
    )
    .bind(&url)
    .bind(c.model.trim())
    .bind(&now)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    // RUNTIME: weka env ili injini zisizo na DB (autocomplete, agentic loop)
    // zitumie model/URL ILE ILE iliyochaguliwa (badala ya default isiyopo).
    std::env::set_var("FUNDI_CLOUD_URL", &url);
    std::env::set_var("FUNDI_CLOUD_MODEL", c.model.trim());
    Ok(())
}

/// Ondoa custom config — rejea default ya server kuu ya LAN.
pub async fn clear_custom(db: &SqlitePool) -> Result<(), String> {
    sqlx::query("DELETE FROM ai_settings WHERE id = 1")
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn default_config() -> LlmConfig {
    LlmConfig {
        url: std::env::var("FUNDI_LLM_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
        model: std::env::var("FUNDI_LLM_MODEL").unwrap_or_else(|_| "qwen2.5vl:3b".into()),
        offline_capable: true,
    }
}

/// Timeout ya wito mmoja wa LLM (sekunde). CPU inference ni ya polepole
/// (model 3B ~10-15 tok/s), kwa hivyo default ni ndefu. Cloud/LAN yenye GPU
/// inaweza kupunguza kwa `FUNDI_LLM_TIMEOUT_S`.
pub fn llm_timeout_secs() -> u64 {
    std::env::var("FUNDI_LLM_TIMEOUT_S")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(900)
}

/// Idadi ya juu ya tokens kwa jibu MOJA la LLM (`num_predict`/`max_tokens`).
/// Kwa CPU, jibu kubwa mno huzidi timeout — weka `FUNDI_LLM_MAX_TOKENS` ndogo
/// kama unataka hatua fupi zaidi.
pub fn llm_max_tokens() -> u64 {
    std::env::var("FUNDI_LLM_MAX_TOKENS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(4096)
}

/// Dirisha la muktadha (`num_ctx`) kwa Ollama. Default ya Ollama (4096) ni
/// ndogo kwa prompt yenye skills ZOTE — inaweza kukatwa. Kwa CPU, 8192 ni
/// usawa mzuri wa kumbukumbu/kasi; ongeza kwa `FUNDI_LLM_NUM_CTX`.
pub fn llm_num_ctx() -> u64 {
    std::env::var("FUNDI_LLM_NUM_CTX")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(8192)
}

/// Prompt COMPACT kwa modeli ndogo za local (Ollama CPU). Modeli 3B hupotea
/// kwenye prompt yenye skills ZOTE (maelfu ya tokens) na hurudia schema badala
/// ya kuandika code. Compact hupunguza blocks za skills/memory na kuweka
/// mfano (few-shot) karibu na mwisho. Default: `true` kwa local, `false` kwa
/// cloud. Weka `FUNDI_LLM_COMPACT=0/1` kulazimisha.
pub fn llm_compact() -> bool {
    if let Ok(v) = std::env::var("FUNDI_LLM_COMPACT") {
        let v = v.trim().to_lowercase();
        return !(v == "0" || v == "false" || v == "no" || v.is_empty());
    }
    // Local Ollama/llama.cpp → compact (CPU ni dhaifu).
    matches!(style_for_url(&llm_url()), "generate") || llm_url().contains(":8081")
}

/// URL ya LLM iliyosanidiwa (env au default).
pub fn llm_url() -> String {
    std::env::var("FUNDI_LLM_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into())
}

/// API key ya CLOUD (env pekee — hakuna key inahifadhiwa kwenye DB).
/// Hukubali `FUNDI_AI_KEY` na, kama haipo, `OPENAI_API_KEY` (entrypoint ya
/// container huweka hii) ili cloud isikose key kwenye deployment.
pub fn ai_key() -> Option<String> {
    for var in ["FUNDI_AI_KEY", "OPENAI_API_KEY"] {
        if let Ok(s) = std::env::var(var) {
            let s = s.trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

/// Njia ya ombi: Ollama/llama.cpp LAN zinatumia `/api/generate` (prompt);
/// OpenAI-compatible (cloud au vLLM) zinatumia `/chat/completions` (messages).
pub fn style_for_url(url: &str) -> &'static str {
    if url.contains("11434") && (url.contains("localhost") || url.contains("127.0.0.1")) {
        "generate"
    } else if url.to_ascii_lowercase().contains("llamacpp") || url.contains(":8081") {
        // llama.cpp (llama-server) ina OpenAI-compatible API: /v1/chat/completions
        "openai"
    } else {
        "openai"
    }
}

/// Load: custom (DB) inapita env — mteja ameweka model/API yake → inatumika.
pub async fn load_with_db(db: &SqlitePool) -> LlmConfig {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT url, model FROM ai_settings WHERE id = 1")
            .fetch_optional(db)
            .await
            .unwrap_or(None);
    match row {
        Some((url, model)) => LlmConfig {
            url,
            model,
            offline_capable: true,
        },
        None => default_config(),
    }
}

/// Compatibility: load() bila DB (env default).
pub fn load() -> LlmConfig {
    default_config()
}

/// Mwanzoni mwa server: kama `ai_settings` ina model iliyochaguliwa, weka
/// `FUNDI_CLOUD_MODEL`/`FUNDI_CLOUD_URL` kwenye env — ili autocomplete na
/// injini nyingine zisizo na DB zitumie model ILE ILE (SI default ya kizamani).
pub async fn sync_env_from_db(db: &SqlitePool) {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT url, model FROM ai_settings WHERE id = 1")
            .fetch_optional(db)
            .await
            .unwrap_or(None);
    if let Some((url, model)) = row {
        if !url.trim().is_empty() && !model.trim().is_empty() {
            std::env::set_var("FUNDI_CLOUD_URL", url.trim());
            std::env::set_var("FUNDI_CLOUD_MODEL", model.trim());
        }
    }
}

/// Njia MOJA ya kuita LLM kwa prompt moja (HALISI):
/// inachagua style sahihi (`/api/generate` kwa Ollama/LAN, `/v1/chat/completions`
/// kwa cloud/vLLM) NA inatuma `Authorization: Bearer FUNDI_AI_KEY` ikiwa imewekwa.
/// Hii ni njia ile ile inayotumiwa na `test_current` — hivyo kile kinachopimwa
/// kwenye UI ni kile kinachotumika kwenye chat/translate halisi.
pub async fn call_llm(cfg: &LlmConfig, prompt: &str) -> Option<String> {
    let base = cfg.url.trim_end_matches('/').to_string();
    let style = style_for_url(&base);
    let (url, body) = if style == "openai" {
        (
            format!("{}/chat/completions", openai_base(&base)),
            serde_json::json!({
                "model": cfg.model,
                "messages": [{ "role": "user", "content": prompt }],
                "stream": false
            }),
        )
    } else {
        (
            format!("{base}/api/generate"),
            serde_json::json!({ "model": cfg.model, "prompt": prompt, "stream": false }),
        )
    };
    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60));
    if let Some(k) = ai_key() {
        req = req.bearer_auth(k);
    }
    let resp = req.send().await.ok()?;
    let v: serde_json::Value = resp.json().await.ok()?;
    let out = v["response"]
        .as_str()
        .map(String::from)
        .or_else(|| v["choices"][0]["message"]["content"].as_str().map(String::from))
        .unwrap_or_default();

    let out = out.trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Eleza picha kwa LLM yenye uwezo wa kuona (multimodal). Hurudisha `(text, ok)`.
/// Hutumika kama njia ya akiba pale Magma server haipo: modeli ya LAN
/// (default `qwen2.5vl:3b`) inaona picha kupitia API ile ile ya LLM.
pub async fn describe_image(
    cfg: &LlmConfig,
    image_base64: &str,
    prompt: &str,
) -> (String, bool) {
    let base = cfg.url.trim_end_matches('/').to_string();
    // `data:` prefix ni ya lazima kwa OpenAI-compatible; Ollama /api/generate
    // hupokea base64 mbichi.
    let raw = image_base64
        .split_once("base64,")
        .map(|(_, b)| b)
        .unwrap_or(image_base64);
    let style = style_for_url(&base);
    let (url, body) = if style == "openai" {
        (
            format!("{}/chat/completions", openai_base(&base)),
            serde_json::json!({
                "model": cfg.model,
                "messages": [{
                    "role": "user",
                    "content": [
                        { "type": "text", "text": prompt },
                        { "type": "image_url",
                          "image_url": { "url": format!("data:image/png;base64,{raw}") } }
                    ]
                }],
                "stream": false
            }),
        )
    } else {
        (
            format!("{base}/api/generate"),
            serde_json::json!({
                "model": cfg.model,
                "prompt": prompt,
                "images": [raw],
                "stream": false
            }),
        )
    };
    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(llm_timeout_secs()));
    if let Some(k) = ai_key() {
        req = req.bearer_auth(k);
    }
    match req.send().await {
        Ok(r) if r.status().is_success() => {
            let v: serde_json::Value = r.json().await.unwrap_or(serde_json::Value::Null);
            let out = v["response"]
                .as_str()
                .map(String::from)
                .or_else(|| v["choices"][0]["message"]["content"].as_str().map(String::from))
                .unwrap_or_default()
                .trim()
                .to_string();
            if out.is_empty() {
                ("Modeli haikutoa maelezo ya picha.".into(), false)
            } else {
                (out, true)
            }
        }
        Ok(r) => (format!("LLM ya kuona HTTP {}", r.status()), false),
        Err(e) => (format!("LLM ya kuona haipatikani: {e}"), false),
    }
}

/// Uliza LLM (HALISI — inaitwa na chat.rs/translate):
/// inafanya kazi OFFLINE kama LLM ipo kwenye server kuu ya LAN (docker service),
/// na ONLINE kwa cloud (Ollama Cloud / OpenAI-compatible) ukiwa na FUNDI_AI_KEY.
pub async fn ask_db(db: &SqlitePool, question: &str, context: &str) -> Option<String> {
    let cfg = load_with_db(db).await;
    let prompt = format!(
        "Wewe ni fundi wa kompyuta wa MTECH OS (Licensed by Mbilinyi Tech). Jibu kwa ufupi na fasaha.\n\nMuktadha (suluhisho za agents wenzake):\n{context}\n\nSwali: {question}\n\nJibu:"
    );
    call_llm(&cfg, &prompt).await
}

/// AI TRANSLATION (maelezo ya mmiliki): "AI ndiyo itatafsiri lugha husika
/// automatically" — inatafsiri text kwa lugha iliyochaguliwa (sw/en/…).
pub async fn translate_db(db: &SqlitePool, text: &str, target_lang: &str) -> Option<String> {
    let cfg = load_with_db(db).await;
    let prompt = format!(
        "Translate the following text to language code '{target_lang}' (sw = Kiswahili, en = English). Reply with the translation ONLY.\n\nText: {text}"
    );
    call_llm(&cfg, &prompt).await
}

/// Msingi wa OpenAI-compatible: hakikisha `/v1` yupo mwishoni (llama-server,
/// vLLM, Ollama /v1, OpenAI, Groq...) ili `/chat/completions` ipate njia sahihi.
pub fn openai_base(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.ends_with("/v1") {
        base.to_string()
    } else {
        format!("{base}/v1")
    }
}

// ---------------------------------------------------------------------------
// API key (hifadhi kwenye FAILI)
// ---------------------------------------------------------------------------
/// AHIFADHI API key (Ollama Cloud / OpenAI-compatible) kwenye FAILI — SIO DB
/// (kanuni: DB haiwezi kubeba key). Env `FUNDI_AI_KEY` inawekwa mara moja.
pub fn save_key(raw: &str) -> serde_json::Value {
    let k = raw.trim();
    if k.is_empty() {
        return serde_json::json!({ "ok": false, "error": "API key ni lazima" });
    }
    if k.len() > 512 || !k.chars().all(|c| c.is_ascii_graphic()) {
        return serde_json::json!({ "ok": false, "error": "key si sahihi (herufi/nambari maalum tu, chini ya herufi 512)" });
    }
    let p = key_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&p, k) {
        Ok(_) => {
            std::env::set_var("FUNDI_AI_KEY", k);
            serde_json::json!({ "ok": true, "file": p.display().to_string(), "note": "key haihifadhiwi DB — faili + env pekee" })
        }
        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Pakia API key kutoka faili mwanzoni mwa server (kama env haijawekwa).
pub fn load_key_file() {
    if ai_key().is_some() {
        return;
    }
    if let Ok(s) = std::fs::read_to_string(key_path()) {
        let s2 = s.trim().to_string();
        if !s2.is_empty() {
            std::env::set_var("FUNDI_AI_KEY", s2);
        }
    }
}

fn key_path() -> std::path::PathBuf {
    let dir = std::env::var("FUNDI_DATA").unwrap_or_else(|_| "data".into());
    std::path::PathBuf::from(dir).join("ai_key")
}

/// MODEL ZOTE za Ollama Cloud kwa key hii (free tier ikiwa ni pamoja):
/// GET https://ollama.com/api/tags + Bearer FUNDI_AI_KEY.
/// Hurudisha kila model ikiwa na `free` flag kutoka data/ai/models.json (chanzo
/// kimoja cha ukweli) — UI inaonyesha wazi bure vs PRO ($20/mwezi).
pub async fn cloud_models() -> serde_json::Value {
    let Some(k) = ai_key() else {
        return serde_json::json!({ "ok": false, "error": "Hakuna API key — weka Ollama Cloud API key kwanza (Hifadhi Key)" });
    };
    let client = reqwest::Client::new();
    match client
        .get("https://ollama.com/api/tags")
        .bearer_auth(k)
        .timeout(std::time::Duration::from_secs(25))
        .send()
        .await
    {
        Err(e) => serde_json::json!({ "ok": false, "error": format!("mtandao: {e}") }),
        Ok(resp) if !resp.status().is_success() => {
            let st = resp.status();
            serde_json::json!({ "ok": false, "error": format!("Ollama Cloud ilitoa {st} — angalia API key") })
        }
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Err(e) => serde_json::json!({ "ok": false, "error": format!("JSON: {e}") }),
            Ok(v) => {
                let mut models: Vec<String> = v["models"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                models.sort();
                let catalog = catalog_free_map();
                let details: Vec<serde_json::Value> = models
                    .iter()
                    .map(|m| {
                        let free = catalog
                            .iter()
                            .find(|(id, _)| id == m)
                            .map(|(_, f)| *f)
                            .unwrap_or(false);
                        serde_json::json!({ "name": m, "free": free })
                    })
                    .collect();
                let free_models: Vec<String> = details
                    .iter()
                    .filter(|d| d["free"] == true)
                    .filter_map(|d| d["name"].as_str().map(String::from))
                    .collect();
                let pro_models: Vec<String> = details
                    .iter()
                    .filter(|d| d["free"] != true)
                    .filter_map(|d| d["name"].as_str().map(String::from))
                    .collect();
                serde_json::json!({
                    "ok": true,
                    "models": models,
                    "details": details,
                    "free_models": free_models,
                    "pro_models": pro_models,
                    "count": models.len(),
                    "free_count": free_models.len(),
                    "note": "free=true → inafanya kazi kwa key ya bure; free=false → Ollama Cloud PRO ($20/mwezi)"
                })
            }
        },
    }
}

/// Map ya (model_id → free) kutoka data/ai/models.json. Default false kwa
/// model isiyojulikana (tunadhani PRO hadi ithibitishwe) — hakuna uongo.
fn catalog_free_map() -> Vec<(String, bool)> {
    let c = crate::data_io::load_models_json();
    c["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    m["id"]
                        .as_str()
                        .map(|id| (id.to_string(), m["free"].as_bool().unwrap_or(false)))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// MAJARIBIO HALISI: swali dogo kwenye LLM iliyosetwa → reply + muda (ms).
/// Inatumia njia ile ile ya chat (generate vs openai + bearer ikiwaipo).
pub async fn test_current(db: &SqlitePool) -> serde_json::Value {
    let cfg = load_with_db(db).await;
    let base = cfg.url.trim_end_matches('/').to_string();
    let style = style_for_url(&base);
    let (url, body) = if style == "openai" {
        (
            format!("{}/chat/completions", openai_base(&base)),
            serde_json::json!({ "model": cfg.model, "messages": [{"role": "user", "content": "Jibu NDIYO tu."}], "stream": false }),
        )
    } else {
        (
            format!("{base}/api/generate"),
            serde_json::json!({ "model": cfg.model, "prompt": "Jibu NDIYO tu.", "stream": false }),
        )
    };
    let started = std::time::Instant::now();
    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30));
    if let Some(k) = ai_key() {
        req = req.bearer_auth(k);
    }
    match req.send().await {
        Err(e) => {
            serde_json::json!({ "ok": false, "error": format!("{e}"), "url": url, "model": cfg.model })
        }
        Ok(resp) => {
            let st = resp.status();
            match resp.json::<serde_json::Value>().await {
                Err(e) => {
                    serde_json::json!({ "ok": false, "error": format!("jibu si JSON ({st}): {e}"), "url": url, "model": cfg.model })
                }
                Ok(v) => {
                    let reply = v["response"]
                        .as_str()
                        .map(String::from)
                        .or_else(|| {
                            v["choices"][0]["message"]["content"]
                                .as_str()
                                .map(String::from)
                        })
                        .unwrap_or_default();
                    let ms = started.elapsed().as_millis() as u64;
                    if reply.is_empty() {
                        serde_json::json!({ "ok": false, "error": format!("jibu tupu ({st})"), "url": url, "model": cfg.model })
                    } else {
                        serde_json::json!({ "ok": true, "reply": reply, "ms": ms, "url": url, "model": cfg.model, "style": style })
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_free_map_inaonyesha_bure_na_pro() {
        let m = catalog_free_map();
        if m.is_empty() {
            return; // models.json haipo kwenye mazingira haya
        }
        let free = m.iter().find(|(id, _)| id == "gpt-oss:20b").map(|(_, f)| *f);
        let pro = m.iter().find(|(id, _)| id == "kimi-k2.7-code").map(|(_, f)| *f);
        assert_eq!(free, Some(true), "gpt-oss:20b ni bure");
        assert_eq!(pro, Some(false), "kimi-k2.7-code ni PRO");
        // Model isiyojulikana → false (hakuna uongo).
        assert!(m.iter().all(|(_, f)| !*f) || m.iter().any(|(_, f)| *f));
    }

    #[test]
    fn config_defaults_ni_lan_server() {
        let cfg = load();
        assert!(
            cfg.url.contains("127.0.0.1")
                || cfg.url.contains("localhost")
                || cfg.url.starts_with("http")
        );
        assert!(
            cfg.offline_capable,
            "LLM ya server kuu inafanya kazi offline"
        );
    }

    #[tokio::test]
    async fn custom_model_inatumika_na_inaondolewa() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&db).await;
        // default kwanza
        let cfg = load_with_db(&db).await;
        assert!(cfg.url.contains("127.0.0.1"));
        // weka custom (mteja ameweka API yake/model yake)
        set_custom(
            &db,
            &CustomAi {
                url: "http://192.168.1.50:1234/".into(),
                model: "mymodel-7b".into(),
            },
        )
        .await
        .unwrap();
        let cfg = load_with_db(&db).await;
        assert_eq!(
            cfg.url, "http://192.168.1.50:1234",
            "trailing slash inaondolewa"
        );
        assert_eq!(cfg.model, "mymodel-7b");
        // URL mbaya inakataliwa
        assert!(set_custom(
            &db,
            &CustomAi {
                url: "ftp://bad".into(),
                model: "x".into()
            }
        )
        .await
        .is_err());
        assert!(set_custom(
            &db,
            &CustomAi {
                url: "http://ok".into(),
                model: "  ".into()
            }
        )
        .await
        .is_err());
        // clear → rejea default
        clear_custom(&db).await.unwrap();
        let cfg = load_with_db(&db).await;
        assert!(cfg.url.contains("127.0.0.1"));
    }

    #[test]
    fn style_urls_na_key_hakuna_panic() {
        // Ollama/llama.cpp za LAN → /api/generate (prompt style, hakuna bearer)
        assert_eq!(style_for_url("http://127.0.0.1:11434"), "generate");
        assert_eq!(style_for_url("http://ollama-internal:11434"), "openai");
        assert_eq!(style_for_url("http://127.0.0.1:8081"), "openai");
        // Ollama CLOUD (ollama.com) ni OpenAI-compatible: /v1/chat/completions
        assert_eq!(style_for_url("https://ollama.com/v1"), "openai");
        // Cloud/OpenAI-compatible → /chat/completions + Bearer FUNDI_AI_KEY
        assert_eq!(style_for_url("https://api.openai.com/v1"), "openai");
        assert_eq!(style_for_url("https://api.groq.com/openai/v1"), "openai");
        // key: hakuna env sandbox → None (hakuna panic)
        let k = ai_key();
        assert!(k.is_none() || !k.unwrap().trim().is_empty());
        // defaults za LAN (ollama :11434 / model qwen2.5vl:3b)
        let d = default_config();
        assert!(d.url.contains(":11434"));
        assert_eq!(d.model, "qwen2.5vl:3b");
    }

    #[test]
    fn llm_knobs_na_compact_defaults() {
        // Knobs za CPU inference zina defaults zinazofaa local (bila env).
        if std::env::var("FUNDI_LLM_TIMEOUT_S").is_err() {
            assert!(llm_timeout_secs() >= 300, "timeout ndefu kwa CPU");
        }
        if std::env::var("FUNDI_LLM_MAX_TOKENS").is_err() {
            assert!(llm_max_tokens() >= 512 && llm_max_tokens() <= 8192);
        }
        if std::env::var("FUNDI_LLM_NUM_CTX").is_err() {
            assert!(llm_num_ctx() >= 4096);
        }
        // Compact: URL ya local ollama → true (modeli ndogo haipotei kwenye skills).
        if std::env::var("FUNDI_LLM_COMPACT").is_err() && std::env::var("FUNDI_LLM_URL").is_err()
        {
            assert!(llm_compact(), "ollama local inatumia prompt compact");
        }
    }

    #[test]
    fn openai_base_hakikisha_v1() {
        assert_eq!(
            openai_base("http://127.0.0.1:8081"),
            "http://127.0.0.1:8081/v1"
        );
        assert_eq!(
            openai_base("https://api.openai.com/v1"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            openai_base("http://127.0.0.1:8081/v1/"),
            "http://127.0.0.1:8081/v1"
        );
        // llama.cpp (llama-server) sasa ni openai-style: /v1/chat/completions
        assert_eq!(style_for_url("http://127.0.0.1:8081/v1"), "openai");
    }

    #[test]
    fn save_key_inakataa_bila_madhara() {
        // Hizi zinarudisha error PEKEE — hakuna faili linaloandikwa hapa.
        assert_eq!(save_key("")["ok"], false);
        assert_eq!(save_key("   ")["ok"], false);
        assert_eq!(save_key("key mpya na nafasi")["ok"], false); // nafasi si graphic
        assert_eq!(save_key(&"k".repeat(600))["ok"], false); // mrefu mno
    }

    #[test]
    fn vision_haijengi_bila_server() {
        // Server haipo → describe_image HURUDISHA ok=false (hakuna kubuni),
        // si kusimama (panic) wala kuning'inia.
        let cfg = LlmConfig {
            url: "http://127.0.0.1:1".into(),
            model: "qwen2.5vl:3b".into(),
            offline_capable: true,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (msg, ok) = rt.block_on(describe_image(&cfg, "AAAA", "eleza picha"));
        assert!(!ok, "bila server ok lazima iwe false");
        assert!(!msg.is_empty(), "lazima kuwe na ujumbe wa kosa");
    }
}
