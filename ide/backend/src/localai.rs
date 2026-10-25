//! localai — AI ya NDANI (bila API yoyote): Whisper ya local, TTS ya local, ffmpeg.
//!
//! Kanuni: sauti haitoki kwenye mashine hii. Whisper hukimbia kama binary ya
//! `whisper.cpp`; TTS kwa `espeak-ng`; ubadilishaji wa umbizo kwa `ffmpeg`.
//! Hakuna key, hakuna cloud, hakuna gharama kwa dakika.
//!
//! Ugunduzi wa zana ni kwa mpangilio huu:
//!   1. env (WHISPER_BIN / WHISPER_MODEL / FFMPEG_BIN / ESPEAK_BIN)
//!   2. njia za kawaida za usakinishaji
//!   3. PATH
//! Hii huruhusu dev (build ya ndani) na production (apt / image) bila mabadiliko.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/localai/status", get(status_handler))
        .route("/api/localai/transcribe", post(transcribe_handler))
        .route("/api/localai/tts", post(tts_handler))
}

async fn status_handler() -> Json<Value> {
    let mut v = status();
    v["ok"] = json!(true);
    v["module"] = json!("localai");
    Json(v)
}

#[derive(serde::Deserialize, Default)]
struct TranscribeQuery {
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    title: Option<String>,
    /// Hifadhi kwenye DB (scribe jobs) — default `true`.
    #[serde(default)]
    save: Option<bool>,
}

/// Sauti (bytes mbichi) → maandishi, KWA WHISPER YA NDANI. Body ni bytes za
/// umbizo lolote (webm/ogg/mp3/wav). Query: `?language=sw&model=small`.
async fn transcribe_handler(
    State(s): State<AppState>,
    Query(q): Query<TranscribeQuery>,
    body: axum::body::Bytes,
) -> Response {
    if body.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "hakuna sauti iliyotumwa (body tupu)"})),
        )
            .into_response();
    }
    if !whisper_available() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "ok": false,
                "error": "Whisper ya ndani haipatikani. Sakinisha whisper.cpp + modeli ya ggml-*.bin, \
                          au weka WHISPER_BIN na WHISPER_MODEL.",
                "hint": status()["whisper"],
            })),
        )
            .into_response();
    }

    let lang = q.language.clone().unwrap_or_else(|| "sw".into());
    let audio = body.to_vec();
    let model = q.model.clone();

    let result = tokio::task::spawn_blocking(move || transcribe(&audio, &lang, model.as_deref())).await;

    let tr = match result {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"ok": false, "error": e})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": format!("kazi imeshindwa: {e}")})),
            )
                .into_response()
        }
    };

    let save = q.save.unwrap_or(true);
    let mut id = String::new();
    if save {
        id = uuid::Uuid::new_v4().to_string();
        let title = q
            .title
            .clone()
            .unwrap_or_else(|| format!("Scribe (sauti ya ndani) — {}", chrono::Local::now().to_rfc3339()));
        let _ = sqlx::query(
            "INSERT INTO edu_scribe (id, title, source_type, language, transcript, analysis, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
        )
        .bind(&id)
        .bind(&title)
        .bind("audio")
        .bind(&tr.language)
        .bind(&tr.text)
        .bind(json!({}).to_string())
        .bind(chrono::Local::now().to_rfc3339())
        .execute(&s.db)
        .await;
    }

    Json(json!({
        "ok": true,
        "engine": "whisper.cpp (ndani)",
        "text": tr.text,
        "language": tr.language,
        "model": tr.model,
        "seconds": tr.duration_s,
        "id": id,
        "privacy": "sauti haikutoka kwenye mashine hii",
    }))
    .into_response()
}

#[derive(serde::Deserialize)]
struct TtsIn {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    speed: Option<u32>,
}

/// TTS ya ndani: maandishi → WAV bytes (espeak-ng).
async fn tts_handler(Json(b): Json<TtsIn>) -> Response {
    let text = b.text.trim().to_string();
    if text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "text inahitajika"})),
        )
            .into_response();
    }
    let lang = b.language.clone().unwrap_or_else(|| "sw".into());
    let speed = b.speed.unwrap_or(150);
    match tokio::task::spawn_blocking(move || speak_wav(&text, &lang, speed)).await {
        Ok(Ok(wav)) => (
            StatusCode::OK,
            [("content-type", "audio/wav")],
            wav,
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": e, "hint": "sakinisha espeak-ng"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": format!("kazi imeshindwa: {e}")})),
        )
            .into_response(),
    }
}

/// Sehemu za kawaida za build ya whisper.cpp (dev + production).
const WHISPER_ROOTS: &[&str] = &[
    "/workspace/whisper.cpp",
    "/opt/whisper.cpp",
    "/usr/local/share/whisper.cpp",
    "/srv/whisper.cpp",
];

/// Modeli: kutoka ndogo (haraka) hadi kubwa (sahihi zaidi). Kila jina hupangwa
/// kwa `ggml-<jina>.bin` ndani ya `models/`.
pub const MODELS: &[(&str, &str, u64)] = &[
    ("tiny", "haraka sana, usahihi mdogo", 75),
    ("base", "haraka, usahihi wa wastani", 142),
    ("small", "salama kwa Kiswahili (default)", 466),
    ("medium", "sahihi zaidi, polepole", 1533),
    ("large-v3", "bora zaidi, mzito sana", 3095),
];

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.exists())
}

fn which(name: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} 2>/dev/null"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() {
        None
    } else {
        Some(PathBuf::from(p))
    }
}

fn whisper_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(r) = std::env::var("WHISPER_DIR") {
        let r = r.trim().to_string();
        if !r.is_empty() {
            roots.push(PathBuf::from(r));
        }
    }
    roots.extend(WHISPER_ROOTS.iter().map(PathBuf::from));
    roots
}

/// `whisper-cli` (jina jipya) au `main` (jina la zamani) ndani ya build/bin.
pub fn whisper_bin() -> Option<PathBuf> {
    if let Some(p) = env_path("WHISPER_BIN") {
        return Some(p);
    }
    for root in whisper_roots() {
        for cand in ["build/bin/whisper-cli", "build/bin/main", "bin/whisper-cli", "whisper-cli"] {
            let p = root.join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    which("whisper-cli")
}

/// Modeli ya ggml. `WHISPER_MODEL` inaweza kuwa njia kamili AU jina la modeli
/// (mfano `small`) — jina hupangwa kwenye `models/ggml-<jina>.bin`.
pub fn whisper_model() -> Option<PathBuf> {
    if let Some(p) = env_path("WHISPER_MODEL") {
        return Some(p);
    }
    if let Ok(name) = std::env::var("WHISPER_MODEL_NAME") {
        let name = name.trim().to_string();
        if !name.is_empty() {
            if let Some(p) = model_by_name(&name) {
                return Some(p);
            }
        }
    }
    // Default: small → medium → base → tiny (kubwa zaidi inayopatikana).
    for name in ["small", "medium", "base", "tiny", "large-v3"] {
        if let Some(p) = model_by_name(name) {
            return Some(p);
        }
    }
    None
}

pub fn model_by_name(name: &str) -> Option<PathBuf> {
    let file = format!("ggml-{name}.bin");
    for root in whisper_roots() {
        let p = root.join("models").join(&file);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Modeli zote zilizopo kwenye diski (kwa UI ya kuchagua).
pub fn available_models() -> Vec<Value> {
    MODELS
        .iter()
        .map(|(name, desc, mb)| {
            json!({
                "name": name,
                "description": desc,
                "size_mb": mb,
                "installed": model_by_name(name).is_some(),
            })
        })
        .collect()
}

pub fn ffmpeg_bin() -> Option<PathBuf> {
    env_path("FFMPEG_BIN").or_else(|| which("ffmpeg"))
}

pub fn espeak_bin() -> Option<PathBuf> {
    env_path("ESPEAK_BIN")
        .or_else(|| which("espeak-ng"))
        .or_else(|| which("espeak"))
}

pub fn whisper_available() -> bool {
    whisper_bin().is_some() && whisper_model().is_some()
}

/// `LD_LIBRARY_PATH` inahitajika ikiwa whisper.cpp haijasakinishwa mfumo mzima:
/// `libwhisper.so`/`libggml.so` ziko kando ya binary.
fn lib_dir(bin: &Path) -> Option<String> {
    let dir = bin.parent()?;
    let has_lib = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with("libwhisper"));
    if has_lib {
        let mut p = dir.to_path_buf();
        if let Some(prev) = std::env::var_os("LD_LIBRARY_PATH") {
            let prev = prev.to_string_lossy().to_string();
            if !prev.is_empty() {
                p = PathBuf::from(format!("{}:{}", dir.display(), prev));
            }
        }
        Some(p.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Ubadilishaji wa umbizo: umbizo lolote (webm/ogg/mp3/m4a/wav) → 16 kHz mono
/// PCM WAV — umbizo analopenda Whisper.
pub fn to_wav16k(input: &Path, output: &Path) -> Result<(), String> {
    let ffmpeg = ffmpeg_bin().ok_or("ffmpeg haipatikani (inahitajika kwa ubadilishaji wa sauti)")?;
    let out = Command::new(ffmpeg)
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", "-f", "wav"])
        .arg(output)
        .output()
        .map_err(|e| format!("ffmpeg haikimbii: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg imeshindwa: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Matokeo ya transcription ya ndani.
pub struct Transcript {
    pub text: String,
    pub language: String,
    pub model: String,
    pub duration_s: f64,
}

/// Transcription ya NDANI kwa whisper.cpp. `audio` ni bytes za umbizo lolote.
/// `lang` ni msimbo wa ISO-639-1 (`sw`, `en`, …) au `auto` kwa kugundua.
pub fn transcribe(audio: &[u8], lang: &str, model_name: Option<&str>) -> Result<Transcript, String> {
    if audio.is_empty() {
        return Err("sauti tupu".into());
    }
    let bin = whisper_bin().ok_or("whisper-cli haipatikani")?;
    let model = match model_name.map(str::trim).filter(|m| !m.is_empty()) {
        Some(name) => {
            model_by_name(name).ok_or_else(|| format!("modeli ya whisper '{name}' haipo kwenye diski"))?
        }
        None => whisper_model().ok_or("hakuna modeli ya whisper (ggml-*.bin)")?,
    };

    let stamp = uuid::Uuid::new_v4().simple().to_string();
    let work = std::env::temp_dir().join(format!("mtechos_stt_{stamp}"));
    std::fs::create_dir_all(&work).map_err(|e| format!("temp dir: {e}"))?;
    let raw = work.join("in.audio");
    let wav = work.join("in16k.wav");
    let out_prefix = work.join("out");

    let cleanup = |work: &Path| {
        let _ = std::fs::remove_dir_all(work);
    };

    if let Err(e) = std::fs::write(&raw, audio) {
        cleanup(&work);
        return Err(format!("kuandika sauti: {e}"));
    }

    // Kama tayari ni WAV ya 16 kHz mono, ffmpeg bado ni salama — lakini ffmpeg
    // inaweza kutokuwepo; hivyo tujaribu kubadilisha, na kama ffmpeg haipo na
    // input ni WAV, tumia moja kwa moja.
    let wav_for_whisper = match to_wav16k(&raw, &wav) {
        Ok(()) => wav.clone(),
        Err(e) => {
            let is_wav = audio.len() > 12 && &audio[0..4] == b"RIFF" && &audio[8..12] == b"WAVE";
            if is_wav {
                raw.clone()
            } else {
                cleanup(&work);
                return Err(e);
            }
        }
    };

    let lang_code = match lang.trim().to_ascii_lowercase().as_str() {
        "" | "auto" => "auto".to_string(),
        l => l.to_string(),
    };

    let mut cmd = Command::new(&bin);
    cmd.arg("-m")
        .arg(&model)
        .arg("-f")
        .arg(&wav_for_whisper)
        .arg("-l")
        .arg(&lang_code)
        .args(["-nt", "-otxt", "-of"])
        .arg(&out_prefix);
    if let Some(ld) = lib_dir(&bin) {
        cmd.env("LD_LIBRARY_PATH", ld);
    }

    let started = std::time::Instant::now();
    let out = cmd
        .output()
        .map_err(|e| format!("whisper-cli haikimbii: {e}"))?;
    let elapsed = started.elapsed().as_secs_f64();

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        cleanup(&work);
        return Err(format!("whisper-cli imeshindwa: {err}"));
    }

    // whisper.cpp huandika `<prefix>.txt` kwa `-otxt`; kama haipo, tumia stdout.
    let txt_file = out_prefix.with_extension("txt");
    let text = match std::fs::read_to_string(&txt_file) {
        Ok(t) => t,
        Err(_) => String::from_utf8_lossy(&out.stdout).to_string(),
    };
    cleanup(&work);

    let text = clean_transcript(&text);
    if text.is_empty() {
        return Err("whisper haikutoa maandishi (sauti kimya au haieleweki)".into());
    }

    Ok(Transcript {
        text,
        language: lang_code,
        model: model
            .file_stem()
            .map(|s| s.to_string_lossy().trim_start_matches("ggml-").to_string())
            .unwrap_or_else(|| "?".into()),
        duration_s: elapsed,
    })
}

/// Huondoa alama za muda na mistari tupu anayorudisha whisper.cpp.
fn clean_transcript(raw: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in raw.lines() {
        let mut l = line.trim().to_string();
        if l.is_empty() {
            continue;
        }
        // `[00:00:00.000 --> 00:00:05.000]` au `[00:00.000]`
        if l.starts_with('[') {
            if let Some(idx) = l.find(']') {
                l = l[idx + 1..].trim().to_string();
            }
        }
        if l.is_empty() || l == "[BLANK_AUDIO]" || l == "(silence)" {
            continue;
        }
        lines.push(l);
    }
    lines.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// TTS ya NDANI kwa espeak-ng. Hurudisha WAV bytes.
pub fn speak_wav(text: &str, lang: &str, speed: u32) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("maandishi tupu".into());
    }
    let bin = espeak_bin().ok_or("espeak-ng haipatikani")?;
    let stamp = uuid::Uuid::new_v4().simple().to_string();
    let out = std::env::temp_dir().join(format!("mtechos_tts_{stamp}.wav"));

    let lang_lc = lang.trim().to_ascii_lowercase();
    let voice = match lang_lc.as_str() {
        "" | "sw" | "swa" | "kiswahili" => "sw",
        "en" | "eng" | "english" => "en",
        "fr" => "fr",
        "ar" => "ar",
        other => other,
    };

    let out_res = Command::new(&bin)
        .args(["-v", voice, "-s", &speed.clamp(80, 400).to_string(), "-w"])
        .arg(&out)
        .arg(text)
        .output()
        .map_err(|e| format!("espeak-ng haikimbii: {e}"))?;
    if !out_res.status.success() {
        let _ = std::fs::remove_file(&out);
        return Err(format!(
            "espeak-ng imeshindwa: {}",
            String::from_utf8_lossy(&out_res.stderr).trim()
        ));
    }
    let bytes = std::fs::read(&out).map_err(|e| format!("kusoma WAV: {e}"))?;
    let _ = std::fs::remove_file(&out);
    if bytes.is_empty() {
        return Err("espeak-ng imetoa faili tupu".into());
    }
    Ok(bytes)
}

/// Hali ya zana zote za ndani (kwa UI na /status).
pub fn status() -> Value {
    let wbin = whisper_bin();
    let wmodel = whisper_model();
    json!({
        "whisper": {
            "available": wbin.is_some() && wmodel.is_some(),
            "binary": wbin.map(|p| p.display().to_string()),
            "model": wmodel.as_ref().map(|p| p.display().to_string()),
            "model_name": wmodel.as_ref()
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().trim_start_matches("ggml-").to_string()),
            "models": available_models(),
        },
        "tts": {
            "available": espeak_bin().is_some(),
            "engine": "espeak-ng",
            "binary": espeak_bin().map(|p| p.display().to_string()),
            "swahili_voice": espeak_bin().is_some(),
        },
        "ffmpeg": {
            "available": ffmpeg_bin().is_some(),
            "binary": ffmpeg_bin().map(|p| p.display().to_string()),
        },
        "privacy": "kila kitu hukimbia kwenye mashine hii — hakuna data inayotoka",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_transcript_huondoa_alama_za_muda() {
        let raw = "[00:00:00.000 --> 00:00:05.000]  Habari yako\n\n[00:00:05.000 --> 00:00:09.000]  Nzuri sana\n";
        assert_eq!(clean_transcript(raw), "Habari yako Nzuri sana");
    }

    #[test]
    fn clean_transcript_huondoa_kimya() {
        assert_eq!(clean_transcript("[BLANK_AUDIO]\n\n"), "");
        assert_eq!(clean_transcript("   \n  \n"), "");
    }

    #[test]
    fn clean_transcript_huunganisha_mistari() {
        assert_eq!(clean_transcript("mstari  wa\npili"), "mstari wa pili");
    }

    #[test]
    fn moduli_zinajulikana() {
        let names: Vec<&str> = MODELS.iter().map(|(n, _, _)| *n).collect();
        assert!(names.contains(&"small"));
        assert!(names.contains(&"medium"));
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn model_by_name_haipatikani_hurudisha_none() {
        assert!(model_by_name("model-isiyokuwepo-kabisa").is_none());
    }

    #[test]
    fn status_ina_sehemu_zote() {
        let s = status();
        assert!(s["whisper"].is_object());
        assert!(s["tts"].is_object());
        assert!(s["ffmpeg"].is_object());
        assert!(s["whisper"]["models"].is_array());
        assert_eq!(s["whisper"]["models"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn speak_wav_hukataa_maandishi_tupu() {
        assert!(speak_wav("   ", "sw", 150).is_err());
    }

    #[test]
    fn transcribe_hukataa_sauti_tupu() {
        assert!(transcribe(&[], "sw", None).is_err());
    }
}
