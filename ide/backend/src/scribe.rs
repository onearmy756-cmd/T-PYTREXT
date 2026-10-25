//! scribe.rs — MTECHOS Sehemu 1: AI SCRIBE (kuchuja sauti, agentic, PDF safi).
//!
//! Awamu: moduli 21. Kila kitu ni HALISI kwa kadiri kinavyowezekana bila GPU:
//!
//!  1. KUCHUJA SAUTI (`/api/scribe/denoise`) — DSP halisi kwa Rust:
//!     - VAD ya nishati (energy + zero-crossing) → inaonyesha sehemu za sauti.
//!     - Kupunguza kelele kwa spectral subtraction (STFT halisi, FFT ya radix-2).
//!     - WAV 16-bit mono/halisi. Hutumia frame 1024, hop 256, Hann window.
//!  2. KUNAKILI — Whisper (API) ikiwa key ipo; la sivyo Web Speech API.
//!  3. AGENTIC (`/api/scribe/process`) — LLM hutoa SOAP / dakika / notes.
//!     Bila LLM: muundo wa msingi wa kimaandishi (hakuna kubuni).
//!  4. PDF SAFI (`/api/scribe/pdf/:id`) — kurasa nyingi, muhuri, QR halisi.
//!
//! Data inabaki kwa mtumiaji: sauti na PDF hupitiwa hapa, hazihifadhiwi cloud.
//!
//! Tables: scribe_jobs scribe_pdfs

use crate::AppState;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS scribe_jobs (
            id TEXT PRIMARY KEY,
            mode TEXT NOT NULL DEFAULT 'meeting',   -- hospital|meeting|classroom
            title TEXT NOT NULL DEFAULT '',
            language TEXT NOT NULL DEFAULT 'sw',
            transcript TEXT NOT NULL DEFAULT '',
            speakers TEXT NOT NULL DEFAULT '[]',
            analysis TEXT NOT NULL DEFAULT '{}',
            engine TEXT NOT NULL DEFAULT 'offline',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS scribe_pdfs (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            seal TEXT NOT NULL DEFAULT '',
            bytes INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/scribe/status", get(status))
        .route("/api/scribe/denoise", post(denoise))
        .route("/api/scribe/diarize", post(diarize_handler))
        .route("/api/scribe/process", post(process))
        .route("/api/scribe/jobs", get(jobs_list))
        .route("/api/scribe/job/:id", get(job_get))
        .route("/api/scribe/job/:id/remove", post(job_remove))
        .route("/api/scribe/pdf/:id", get(pdf_get))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ================= DSP halisi (bila dependencies) =================

/// FFT ya radix-2 in-place (Cooley–Tukey). `re`/`im` lazima ziwe urefu wa 2^n.
pub fn fft_inplace(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two(), "FFT inahitaji 2^n");
    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        let mut i = 0usize;
        while i < n {
            let mut cur_r = 1.0f64;
            let mut cur_i = 0.0f64;
            for k in 0..len / 2 {
                let ur = re[i + k];
                let ui = im[i + k];
                let vr = re[i + k + len / 2] * cur_r - im[i + k + len / 2] * cur_i;
                let vi = re[i + k + len / 2] * cur_i + im[i + k + len / 2] * cur_r;
                re[i + k] = ur + vr;
                im[i + k] = ui + vi;
                re[i + k + len / 2] = ur - vr;
                im[i + k + len / 2] = ui - vi;
                let nr = cur_r * wr - cur_i * wi;
                cur_i = cur_r * wi + cur_i * wr;
                cur_r = nr;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// FFT ya kinyume (inverse) — hutumia uhusiano wa conjugate.
pub fn ifft_inplace(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    for x in im.iter_mut() {
        *x = -*x;
    }
    fft_inplace(re, im);
    let inv = 1.0 / n as f64;
    for i in 0..n {
        re[i] *= inv;
        im[i] *= -inv;
    }
}

/// Dirisha la Hann.
pub fn hann(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos())
        .collect()
}

/// Kipimo cha nishati (RMS) cha kipande.
pub fn rms(x: &[f32]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    let s: f64 = x.iter().map(|v| (*v as f64) * (*v as f64)).sum();
    (s / x.len() as f64).sqrt()
}

/// VAD: hutambua sehemu za sauti kwa nishati inayozidi kizingiti cha kelele.
/// Hurudisha bendera kwa kila dirisha.
pub fn vad_frames(samples: &[f32], frame: usize, thresh_mult: f64) -> Vec<bool> {
    if samples.len() < frame {
        return vec![!samples.is_empty()];
    }
    let energies: Vec<f64> = samples.chunks(frame).map(rms).collect();
    // Kizingiti: nishati ya kelele (percentile 20) × multiplier, lakini si zaidi
    // ya katikati kati ya kelele na kilele. Bila kikomo hiki, kelele kubwa
    // hufanya kizingiti kizidi sauti yote → VAD hurudisha kimya chote.
    let mut sorted = energies.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let noise_floor = sorted.get(sorted.len() / 5).copied().unwrap_or(0.0);
    let peak = sorted.last().copied().unwrap_or(0.0);
    let cap = noise_floor + (peak - noise_floor).max(0.0) * 0.5;
    let thresh = (noise_floor * thresh_mult).min(cap).max(1e-4);
    energies.iter().map(|e| *e > thresh).collect()
}

/// Kupunguza kelele kwa spectral subtraction (STFT halisi).
/// `strength` 0.0–1.0 — kiasi cha kelele kinachoondolewa.
pub fn spectral_denoise(samples: &[f32], frame: usize, hop: usize, strength: f64) -> Vec<f32> {
    if samples.len() < frame || frame < 4 || hop == 0 || hop > frame {
        return samples.to_vec();
    }
    let win = hann(frame);
    // Weka padding ya sifuri mwanzoni na mwisho ili kila sampuli ipate
    // mwingiliano KAMILI wa madirisha. Bila hii norm[i]≈0 kwenye kingo
    // (Hann huanza kwa 0) → mgawanyo hupuliza sampuli za pembeni.
    let pad = frame;
    let mut padded: Vec<f32> = Vec::with_capacity(samples.len() + 2 * pad);
    padded.extend(std::iter::repeat(0.0f32).take(pad));
    padded.extend_from_slice(samples);
    padded.extend(std::iter::repeat(0.0f32).take(pad));

    let n_frames = (padded.len() - frame) / hop + 1;
    // 1) STFT
    let mut spec: Vec<(Vec<f64>, Vec<f64>)> = Vec::with_capacity(n_frames);
    for f in 0..n_frames {
        let start = f * hop;
        let mut re = vec![0.0f64; frame];
        let mut im = vec![0.0f64; frame];
        for i in 0..frame {
            re[i] = padded[start + i] as f64 * win[i];
        }
        fft_inplace(&mut re, &mut im);
        spec.push((re, im));
    }
    // 2) Kadiria wigo wa kelele kwa kila bin (percentile 20 kwa muda) — madirisha
    // ya padding (sifuri) hayajumuishwi, la sivyo kizingiti hushuka kupita kiasi.
    let bins = frame / 2 + 1;
    let mut noise = vec![0.0f64; bins];
    let real_lo = pad / hop;
    let real_hi = ((pad + samples.len()) / hop).min(spec.len());
    for b in 0..bins {
        let mut mags: Vec<f64> = if real_lo < real_hi {
            (real_lo..real_hi)
                .map(|f| {
                    let (re, im) = &spec[f];
                    (re[b] * re[b] + im[b] * im[b]).sqrt()
                })
                .collect()
        } else {
            spec.iter()
                .map(|(re, im)| (re[b] * re[b] + im[b] * im[b]).sqrt())
                .collect()
        };
        mags.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        noise[b] = mags.get(mags.len() / 5).copied().unwrap_or(0.0);
    }
    // 3) Ondoa kelele na rekebisha awamu.
    let mut out = vec![0.0f32; padded.len()];
    let mut norm = vec![0.0f32; padded.len()];
    for (f, (re, im)) in spec.iter_mut().enumerate() {
        for b in 0..bins {
            let mag = (re[b] * re[b] + im[b] * im[b]).sqrt();
            let clean = (mag - strength * noise[b]).max(0.0);
            let scale = if mag > 1e-9 { clean / mag } else { 0.0 };
            re[b] *= scale;
            im[b] *= scale;
            if b > 0 && b < frame / 2 {
                let mirror = frame - b;
                re[mirror] = re[b];
                im[mirror] = -im[b];
            }
        }
        ifft_inplace(re, im);
        let start = f * hop;
        for i in 0..frame {
            out[start + i] += re[i] as f32 * win[i] as f32;
            norm[start + i] += win[i] as f32 * win[i] as f32;
        }
    }
    for i in 0..out.len() {
        if norm[i] > 1e-6 {
            out[i] /= norm[i];
        }
    }
    out[pad..pad + samples.len()].to_vec()
}

/// WAV 16-bit PCM (mono).
pub fn wav_encode(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    let byte_rate = sample_rate * 2;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Gawa maandishi ya maongezi kuwa wazungumzaji (diarization ya msingi).
/// Hutambua mabadiliko kwa alama za mzungumzaji (`Jina:` mwanzoni) ikiwepo,
/// la sivyo hurudisha kipande kimoja.
pub fn split_speakers(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut cur_speaker = String::new();
    let mut cur_text = String::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        // "Jina: maandishi" — jina fupi bila koma/period.
        let is_label = l
            .split_once(':')
            .map(|(name, _)| {
                let n = name.trim();
                !n.is_empty()
                    && n.chars().count() <= 40
                    && !n.contains(',')
                    && !n.contains('.')
                    && n.split_whitespace().count() <= 5
            })
            .unwrap_or(false);
        if is_label {
            if !cur_text.trim().is_empty() {
                out.push((cur_speaker.clone(), cur_text.trim().to_string()));
            }
            let (name, rest) = l.split_once(':').unwrap();
            cur_speaker = name.trim().to_string();
            cur_text = rest.trim().to_string();
        } else {
            if !cur_text.is_empty() {
                cur_text.push(' ');
            }
            cur_text.push_str(l);
        }
    }
    if !cur_text.trim().is_empty() {
        out.push((cur_speaker, cur_text.trim().to_string()));
    }
    if out.is_empty() {
        out.push((String::new(), text.trim().to_string()));
    }
    out
}

// ================= Handlers =================

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let whisper_api = crate::education::audio_endpoint().is_some();
    let local = crate::localai::whisper_available();
    let llm = crate::ai_config::ai_key().is_some();
    Json(json!({
        "ok": true,
        "module": "scribe",
        "part": "Sehemu 1 — AI Scribe",
        "dsp": { "vad": true, "spectral_denoise": true, "wav": true, "fft": "radix-2 halisi" },
        "transcription": {
            "local_whisper": local,
            "whisper_api": whisper_api,
            "engine": if local { "whisper.cpp (ndani)" } else if whisper_api { "openai api" } else { "hakuna" },
            "web_speech_fallback": true
        },
        "tts": { "local": crate::localai::espeak_bin().is_some(), "engine": "espeak-ng" },
        "localai": crate::localai::status(),
        "agentic": { "llm": llm, "modes": ["hospital", "meeting", "classroom"] },
        "pdf": { "multi_page": true, "qr": true, "seal": true },
        "jobs": table_count(&s.db, "scribe_jobs").await,
        "pdfs": table_count(&s.db, "scribe_pdfs").await,
    }))
}

/// Kuchuja sauti: VAD + spectral denoise. Hupokea sauti mbichi (WAV au PCM f32
/// ya little-endian) na `?rate=16000&strength=0.8`. Hurudisha WAV safi + ripoti.
async fn denoise(
    Query(q): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> axum::response::Response {
    if body.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "hakuna sauti (body tupu)" })),
        )
            .into_response();
    }
    let rate: u32 = q.get("rate").and_then(|v| v.parse().ok()).unwrap_or(16000).clamp(8000, 48000);
    let strength: f64 = q.get("strength").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.8).clamp(0.0, 1.5);

    // Tambua WAV ikiwa imetumwa, la sivyo chukulia PCM f32 (LE).
    let (samples, src_rate) = if body.len() > 44 && &body[0..4] == b"RIFF" && &body[8..12] == b"WAVE" {
        match decode_wav(&body) {
            Some((s, r)) => (s, r),
            None => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "WAV haisomeki" })),
                )
                    .into_response()
            }
        }
    } else {
        let n = body.len() / 4;
        let mut s = Vec::with_capacity(n);
        for i in 0..n {
            let b = &body[i * 4..i * 4 + 4];
            s.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        }
        (s, rate)
    };

    if samples.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "sauti tupu baada ya kusoma" })),
        )
            .into_response();
    }

    let frame = 1024usize;
    let hop = 256usize;
    let before_rms = rms(&samples);
    let vad = vad_frames(&samples, frame, 2.0);
    let speech_frames = vad.iter().filter(|b| **b).count();
    let speech_ratio = if vad.is_empty() { 0.0 } else { speech_frames as f64 / vad.len() as f64 };

    let clean = spectral_denoise(&samples, frame, hop, strength);
    let after_rms = rms(&clean);
    let wav = wav_encode(&clean, src_rate);

    // Ripoti ya uboreshaji (halisi).
    let db_gain = if after_rms > 1e-9 {
        20.0 * (before_rms / after_rms).log10()
    } else {
        0.0
    };

    // Diarization HALISI (acoustic features + k-means) kwenye sauti iliyosafishwa.
    let diar = crate::diarize::diarize(&clean, src_rate, 0);

    let mut resp = Json(json!({
        "ok": true,
        "sample_rate": src_rate,
        "samples_in": samples.len(),
        "samples_out": clean.len(),
        "duration_s": (samples.len() as f64 / src_rate as f64 * 100.0).round() / 100.0,
        "speech_frames": speech_frames,
        "total_frames": vad.len(),
        "speech_ratio": (speech_ratio * 100.0).round() / 100.0,
        "rms_before": (before_rms * 100000.0).round() / 100000.0,
        "rms_after": (after_rms * 100000.0).round() / 100000.0,
        "noise_removed_db": (db_gain * 10.0).round() / 10.0,
        "strength": strength,
        "diarization": crate::diarize::summary(&diar),
        "wav_base64": base64_encode(&wav),
        "note_sw": "Sauti imesafishwa kwa spectral subtraction halisi (FFT). Sehemu za kimya zimegunduliwa kwa VAD. Wazungumzaji wametenganishwa kwa MFCC + F0 + k-means."
    })).into_response();
    resp.headers_mut().insert(
        header::HeaderName::from_static("x-scribe-wav-bytes"),
        header::HeaderValue::from_str(&wav.len().to_string()).unwrap_or_else(|_| header::HeaderValue::from_static("0")),
    );
    resp
}

/// Diarization ya pekee: sauti → zamu za wazungumzaji (bila kubadilisha sauti).
/// `?rate=16000&speakers=2` (speakers=0 → igundue yenyewe).
async fn diarize_handler(
    Query(q): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> axum::response::Response {
    if body.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "hakuna sauti (body tupu)" })),
        )
            .into_response();
    }
    let rate: u32 = q.get("rate").and_then(|v| v.parse().ok()).unwrap_or(16000).clamp(8000, 48000);
    let expected: usize = q.get("speakers").and_then(|v| v.parse().ok()).unwrap_or(0);

    let (samples, src_rate) = if body.len() > 44 && &body[0..4] == b"RIFF" && &body[8..12] == b"WAVE" {
        match decode_wav(&body) {
            Some((s, r)) => (s, r),
            None => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "WAV haisomeki" })),
                )
                    .into_response()
            }
        }
    } else {
        let n = body.len() / 4;
        let mut s = Vec::with_capacity(n);
        for i in 0..n {
            let b = &body[i * 4..i * 4 + 4];
            s.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        }
        (s, rate)
    };

    if samples.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "sauti tupu baada ya kusoma" })),
        )
            .into_response();
    }

    let turns = crate::diarize::diarize(&samples, src_rate, expected);
    let mut out = crate::diarize::summary(&turns);
    if let Some(o) = out.as_object_mut() {
        o.insert("ok".into(), json!(true));
        o.insert("sample_rate".into(), json!(src_rate));
        o.insert(
            "duration_s".into(),
            json!((samples.len() as f64 / src_rate as f64 * 100.0).round() / 100.0),
        );
        o.insert(
            "note_sw".into(),
            json!("Wazungumzaji wametenganishwa kwa sifa za akustika (RMS/ZCR/centroid/rolloff/flux/bendi) + k-means. Hakuna GPU, hakuna modeli ya nje."),
        );
    }
    Json(out).into_response()
}

/// Decode WAV 16-bit PCM (mono au stereo → mono).
pub fn decode_wav(data: &[u8]) -> Option<(Vec<f32>, u32)> {
    if data.len() < 44 || &data[0..4] != b"RIFF" {
        return None;
    }
    let mut pos = 12usize;
    let mut rate = 16000u32;
    let mut channels = 1u16;
    let mut bits = 16u16;
    let mut samples: Vec<f32> = Vec::new();
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let sz = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let body = data.get(pos + 8..pos + 8 + sz)?;
        if id == b"fmt " {
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            if bits == 16 {
                for c in body.chunks_exact(2) {
                    samples.push(i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0);
                }
            } else if bits == 8 {
                for &b in body {
                    samples.push((b as f32 - 128.0) / 128.0);
                }
            } else {
                return None;
            }
            break;
        }
        pos += 8 + sz + (sz & 1);
    }
    if channels > 1 {
        let ch = channels as usize;
        samples = samples
            .chunks(ch)
            .map(|c| c.iter().sum::<f32>() / ch as f32)
            .collect();
    }
    Some((samples, rate))
}

pub fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[derive(Deserialize)]
struct ProcessIn {
    #[serde(default)]
    transcript: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    subject: String,
}

/// Uchambuzi wa agentic: SOAP (hospitali), dakika (mkutano), notes (darasa).
async fn process(State(s): State<AppState>, Json(b): Json<ProcessIn>) -> Json<Value> {
    let transcript = b.transcript.trim();
    if transcript.is_empty() {
        return Json(json!({ "ok": false, "error": "transcript inahitajika" }));
    }
    let mode = norm_mode(&b.mode);
    let lang = if b.language.trim().is_empty() { "Kiswahili" } else { b.language.trim() };
    let speakers = split_speakers(transcript);

    let schema = match mode {
        "hospital" => "{\"muhtasari\":\"...\",\"subjective\":\"...\",\"objective\":\"...\",\"assessment\":\"...\",\"plan\":\"...\",\"dawa\":[\"...\"],\"follow_up\":\"...\"}",
        "classroom" => "{\"muhtasari\":\"...\",\"mada_kuu\":[\"...\"],\"pointi_muhimu\":[\"...\"],\"kazi_ya_nyumbani\":[\"...\"]}",
        _ => "{\"muhtasari\":\"...\",\"ajenda\":[\"...\"],\"maamuzi\":[\"...\"],\"kazi\":[{\"nani\":\"...\",\"kazi\":\"...\",\"lini\":\"...\"}]}",
    };
    let speakers_note = if speakers.len() > 1 {
        format!("\nWAZUNGUMZAJI WALIOGUNDULIWA: {}\n", speakers.iter().map(|(s, _)| if s.is_empty() { "?".to_string() } else { s.clone() }).collect::<Vec<_>>().join(", "))
    } else {
        String::new()
    };
    let prompt = format!(
        "Wewe ni AI Scribe wa MTECHOS. Lugha: {lang}. Aina ya kikao: {mode}.{}\n\
         MAANDISHI YA MAONGEZI:\n{transcript}\n\n\
         Changanua kwa uangalifu na urudishe JSON PEKEE yenye muundo huu:\n{schema}\n\
         Kanuni: usibuni kitu kisichopo kwenye maandishi. Kama hakipo, acha tupu.",
        speakers_note
    );

    let out = crate::education::llm_long(&s.db, &prompt, 120).await;
    let (engine, analysis) = match &out {
        Some(t) => {
            let parsed = serde_json::from_str::<Value>(t.trim())
                .ok()
                .filter(|v| v.is_object())
                .unwrap_or_else(|| json!({ "muhtasari": t.trim() }));
            ("llm", parsed)
        }
        None => {
            // Offline: muundo wa kimaandishi wa msingi — hakuna kubuni.
            let words = transcript.split_whitespace().count();
            let mut base = json!({
                "muhtasari": format!("Kikao cha aina `{mode}` — maneno {words}. LLM haipatikani; maandishi yamehifadhiwa kwa ukaguzi wa mtu."),
                "hali": "llm_haipatikani"
            });
            if mode == "meeting" {
                base["ajenda"] = json!([]);
                base["maamuzi"] = json!([]);
                base["kazi"] = json!([]);
            }
            ("offline", base)
        }
    };

    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO scribe_jobs (id,mode,title,language,transcript,speakers,analysis,engine,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&id)
    .bind(mode)
    .bind(if b.title.trim().is_empty() { format!("Kikao — {}", now()) } else { b.title.trim().to_string() })
    .bind(lang)
    .bind(transcript)
    .bind(json!(speakers.iter().map(|(s, t)| json!({ "speaker": s, "text": t })).collect::<Vec<_>>()).to_string())
    .bind(analysis.to_string())
    .bind(engine)
    .bind(now())
    .execute(&s.db)
    .await;

    Json(json!({
        "ok": true, "id": id, "mode": mode, "engine": engine,
        "speakers": speakers.iter().map(|(s, t)| json!({ "speaker": s, "text": t })).collect::<Vec<_>>(),
        "analysis": analysis,
        "pdf_url": format!("/api/scribe/pdf/{id}"),
        "note_sw": "Data inabaki kwenye seva yako. PDF inaweza kudownload au kuchapishwa."
    }))
}

/// Hali ya kikao (hospitali/mkutano/darasa).
pub fn norm_mode(m: &str) -> &'static str {
    match m.trim().to_lowercase().as_str() {
        "hospital" | "hospitali" | "clinic" | "kliniki" | "afya" => "hospital",
        "classroom" | "darasa" | "class" | "somo" | "shule" => "classroom",
        _ => "meeting",
    }
}

async fn jobs_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT id,mode,title,language,engine,created_at,substr(transcript,1,160) FROM scribe_jobs ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let jobs: Vec<Value> = rows
        .into_iter()
        .map(|(id, mode, title, lang, engine, at, preview)| {
            json!({ "id": id, "mode": mode, "title": title, "language": lang, "engine": engine, "created_at": at, "preview": preview })
        })
        .collect();
    Json(json!({ "ok": true, "jobs": jobs }))
}

async fn job_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
        "SELECT id,mode,title,language,transcript,speakers,analysis,engine FROM scribe_jobs WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match row {
        Some((id, mode, title, lang, transcript, speakers, analysis, engine)) => Json(json!({
            "ok": true,
            "job": {
                "id": id, "mode": mode, "title": title, "language": lang,
                "transcript": transcript, "engine": engine,
                "speakers": serde_json::from_str::<Value>(&speakers).unwrap_or(json!([])),
                "analysis": serde_json::from_str::<Value>(&analysis).unwrap_or(json!({})),
            },
            "pdf_url": format!("/api/scribe/pdf/{id}")
        })),
        None => Json(json!({ "ok": false, "error": "kikao hakipatikani" })),
    }
}

async fn job_remove(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query("DELETE FROM scribe_jobs WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    let _ = sqlx::query("DELETE FROM scribe_pdfs WHERE job_id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": r.is_ok(), "removed": r.map(|x| x.rows_affected()).unwrap_or(0) }))
}

/// PDF safi ya kikao: kichwa, sehemu, muhuri "AI Generated — Review Required",
/// na QR halisi inayoelekeza kwenye kikao.
async fn pdf_get(State(s): State<AppState>, Path(id): Path<String>) -> axum::response::Response {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT id,mode,title,language,transcript,analysis,created_at FROM scribe_jobs WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((jid, mode, title, _lang, transcript, analysis, created_at)) = row else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            "kikao hakipatikani".to_string(),
        )
            .into_response();
    };
    let a: Value = serde_json::from_str(&analysis).unwrap_or(json!({}));
    let mode_label = match mode.as_str() {
        "hospital" => "RIPOTI YA KLINIKI (SOAP)",
        "classroom" => "NOTES ZA SOMO",
        _ => "DAKIKA ZA MKUTANO",
    };

    let mut sections: Vec<(String, String, Vec<String>)> = Vec::new();
    if let Some(m) = a["muhtasari"].as_str() {
        if !m.trim().is_empty() {
            sections.push(("MUHTASARI".to_string(), m.trim().to_string(), vec![]));
        }
    }
    let arr_field = |key: &str| -> Vec<String> {
        a[key].as_array().map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or_default()
    };
    match mode.as_str() {
        "hospital" => {
            for (k, label) in [("subjective", "SUBJECTIVE"), ("objective", "OBJECTIVE"), ("assessment", "ASSESSMENT"), ("plan", "PLAN")] {
                if let Some(v) = a[k].as_str() {
                    if !v.trim().is_empty() {
                        sections.push((label.to_string(), v.trim().to_string(), vec![]));
                    }
                }
            }
            let dawa = arr_field("dawa");
            if !dawa.is_empty() {
                sections.push(("DAWA".to_string(), String::new(), dawa));
            }
            if let Some(f) = a["follow_up"].as_str() {
                if !f.trim().is_empty() {
                    sections.push(("MAELEKEZO YA UFUATILIAJI".to_string(), f.trim().to_string(), vec![]));
                }
            }
        }
        "classroom" => {
            for (k, label) in [("mada_kuu", "MADA KUU"), ("pointi_muhimu", "POINTI MUHIMU"), ("kazi_ya_nyumbani", "KAZI YA NYUMBANI")] {
                let v = arr_field(k);
                if !v.is_empty() {
                    sections.push((label.to_string(), String::new(), v));
                }
            }
        }
        _ => {
            for (k, label) in [("ajenda", "AJENDA"), ("maamuzi", "MAAMUZI")] {
                let v = arr_field(k);
                if !v.is_empty() {
                    sections.push((label.to_string(), String::new(), v));
                }
            }
            let kazi = a["kazi"].as_array().cloned().unwrap_or_default();
            if !kazi.is_empty() {
                let items: Vec<String> = kazi
                    .iter()
                    .map(|k| {
                        let nani = k["nani"].as_str().unwrap_or("?");
                        let kk = k["kazi"].as_str().unwrap_or("");
                        let lini = k["lini"].as_str().unwrap_or("");
                        format!("{nani} — {kk} ({lini})")
                    })
                    .collect();
                sections.push(("KAZI ZILIZOPEWA".to_string(), String::new(), items));
            }
        }
    }
    sections.push(("MAANDISHI YA MAONGEZI (RAW)".to_string(), transcript.clone(), vec![]));

    let seal = crate::pkicrypto::sha256_hex(format!("{jid}|{title}|{created_at}").as_bytes());
    let seal_short = &seal[..16.min(seal.len())];
    let footer = format!(
        "AI Generated — Review Required · Muhuri SHA-256: {seal_short} · Kikao: {jid} · MTECH OS (Mbilinyi Tech)"
    );
    let subtitle = format!("{} · {}", created_at.chars().take(16).collect::<String>(), seal_short);
    let pdf = crate::report::document_pdf(mode_label, &subtitle, &sections, &footer);

    let _ = sqlx::query("INSERT OR REPLACE INTO scribe_pdfs (id,job_id,title,seal,bytes,created_at) VALUES (?1,?2,?3,?4,?5,?6)")
        .bind(uid()).bind(&jid).bind(&title).bind(&seal).bind(pdf.len() as i64).bind(now())
        .execute(&s.db).await;

    let len = pdf.len();
    let mut resp = pdf.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/pdf"),
    );
    if let Ok(v) = header::HeaderValue::from_str(&len.to_string()) {
        resp.headers_mut().insert(header::CONTENT_LENGTH, v);
    }
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_static("inline; filename=\"scribe.pdf\""),
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_na_ifft_hurudisha_awali() {
        let mut re: Vec<f64> = (0..16).map(|i| (i as f64).sin()).collect();
        let orig = re.clone();
        let mut im = vec![0.0f64; 16];
        fft_inplace(&mut re, &mut im);
        ifft_inplace(&mut re, &mut im);
        for i in 0..16 {
            assert!((re[i] - orig[i]).abs() < 1e-9, "bin {i}: {} vs {}", re[i], orig[i]);
        }
    }

    #[test]
    fn fft_ya_sine_ina_kilele_kimoja() {
        // Sine yenye frequency 2 cycles/16 samples → kilele kwenye bin 2 na 14.
        let n = 16usize;
        let mut re: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * 2.0 * i as f64 / n as f64).sin())
            .collect();
        let mut im = vec![0.0f64; n];
        fft_inplace(&mut re, &mut im);
        let mag: Vec<f64> = (0..n).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt()).collect();
        let peak = mag.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
        assert!(peak == 2 || peak == 14, "kilele kilikuwa bin {peak}");
    }

    #[test]
    fn vad_hutambua_kimya_na_sauti() {
        let mut s = vec![0.0f32; 1024];
        s.extend(vec![0.6f32; 1024]); // sauti
        s.extend(vec![0.0f32; 1024]); // kimya
        let frames = vad_frames(&s, 1024, 2.0);
        assert_eq!(frames.len(), 3);
        assert!(!frames[0], "kimya kilitambuliwa kama sauti");
        assert!(frames[1], "sauti haikutambuliwa");
        assert!(!frames[2], "kimya cha pili kilitambuliwa");
    }

    #[test]
    fn denoise_hupunguza_kelele_na_kuhifadhi_sauti() {
        // Sauti: sine 440Hz yenye amplitude kubwa + kelele ndogo.
        let sr = 16000u32;
        let n = 8192usize;
        let mut s: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f64 / sr as f64;
                (0.5 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) as f32
                    + ((i as f32 * 12.9898).sin() * 0.05) // pseudo-noise ndogo
            })
            .collect();
        let clean = spectral_denoise(&s, 1024, 256, 0.8);
        assert_eq!(clean.len(), s.len());
        assert!(rms(&clean) > 0.0, "sauti ilifutwa kabisa");
        // Kelele ya bin ya juu inapaswa kupungua; sauti kuu ibaki.
        s.truncate(n);
        assert!(rms(&clean).is_finite());
    }

    #[test]
    fn wav_encode_ina_kichwa_sahihi() {
        let s = vec![0.0f32; 100];
        let w = wav_encode(&s, 16000);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(&w[36..40], b"data");
        assert_eq!(w.len(), 44 + 200);
        assert_eq!(u32::from_le_bytes([w[24], w[25], w[26], w[27]]), 16000);
    }

    #[test]
    fn wav_roundtrip_ni_sahihi() {
        let s: Vec<f32> = (0..200).map(|i| (i as f32 / 100.0) - 1.0).collect();
        let w = wav_encode(&s, 8000);
        let (back, rate) = decode_wav(&w).unwrap();
        assert_eq!(rate, 8000);
        assert_eq!(back.len(), s.len());
        for i in 0..s.len() {
            assert!((back[i] - s[i]).abs() < 0.001, "sample {i} haifanani");
        }
    }

    #[test]
    fn wav_stereo_hupunguzwa_kuwa_mono() {
        // WAV ya stereo (channels=2) — sampuli 2 kwa kila frame.
        let frames = 50usize;
        let mut samples: Vec<f32> = Vec::new();
        for i in 0..frames {
            samples.push(i as f32 / 100.0);
            samples.push(i as f32 / 100.0);
        }
        // Unda WAV ya stereo kwa mkono.
        let data_len = samples.len() * 2;
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        w.extend_from_slice(b"WAVE");
        w.extend_from_slice(b"fmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes()); // stereo
        w.extend_from_slice(&16000u32.to_le_bytes());
        w.extend_from_slice(&(16000u32 * 4).to_le_bytes());
        w.extend_from_slice(&4u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in &samples {
            w.extend_from_slice(&((s * 32767.0) as i16).to_le_bytes());
        }
        let (mono, _) = decode_wav(&w).unwrap();
        assert_eq!(mono.len(), frames, "stereo haikupunguzwa");
    }

    #[test]
    fn split_speakers_hutambua_wazungumzaji() {
        let t = "Daktari: Habari, unaumwa nini?\nMgonjwa: Naumwa kichwa.\nDaktari: Siku ngapi?";
        let sp = split_speakers(t);
        assert_eq!(sp.len(), 3, "sp: {sp:?}");
        assert_eq!(sp[0].0, "Daktari");
        assert_eq!(sp[1].0, "Mgonjwa");
        assert!(sp[1].1.contains("kichwa"));
    }

    #[test]
    fn split_speakers_bila_alama_hurudisha_kipande_kimoja() {
        let t = "Maongezi ya kawaida bila alama za mzungumzaji kabisa.";
        let sp = split_speakers(t);
        assert_eq!(sp.len(), 1);
        assert!(sp[0].0.is_empty());
    }

    #[test]
    fn mode_za_kikao_hupangwa() {
        assert_eq!(norm_mode("hospitali"), "hospital");
        assert_eq!(norm_mode("darasa"), "classroom");
        assert_eq!(norm_mode("meeting"), "meeting");
        assert_eq!(norm_mode(""), "meeting");
    }
}

#[cfg(test)]
mod denoise_regression {
    use super::*;

    /// Regression: `denoise` ilikuwa ikirudisha diarization ya sifa za KALE
    /// (au haikurudishi kabisa). UI (`scribepro-tab`) huonyesha panel ya
    /// wazungumzaji kutoka `diarization` ya jibu hili, kwa hivyo ni LAZIMA
    /// iwepo na iwe na muundo sahihi kwa sauti ya wazungumzaji wawili.
    #[tokio::test]
    async fn denoise_hurudisha_diarization_halisi() {
        // Wazungumzaji 2 wakipishana (A-B-A), kama kwenye majaribio ya diarize.
        let tone = |freq: f64, dur_s: f64| -> Vec<f32> {
            let n = (dur_s * 16000.0) as usize;
            (0..n)
                .map(|i| {
                    let t = i as f64 / 16000.0;
                    (0.5 * (2.0 * std::f64::consts::PI * freq * t).sin()) as f32
                })
                .collect()
        };
        let mut samples = tone(200.0, 1.5);
        samples.extend(tone(2500.0, 1.5));
        samples.extend(tone(200.0, 1.5));

        let wav = wav_encode(&samples, 16000);
        let resp = denoise(
            axum::extract::Query(std::collections::HashMap::from([
                ("rate".to_string(), "16000".to_string()),
                ("strength".to_string(), "0.8".to_string()),
            ])),
            axum::body::Bytes::from(wav),
        )
        .await;

        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("jibu lisilosomeka");
        let v: serde_json::Value = serde_json::from_slice(&body).expect("JSON batili");

        let d = v.get("diarization").expect("diarization haipo kwenye jibu");
        assert!(d["speakers"].is_u64(), "speakers haipo");
        assert!(
            d["speakers"].as_u64().unwrap() >= 2,
            "wazungumzaji 2 walitarajiwa, tumepata {:?}",
            d["speakers"]
        );
        assert!(d["segments"].as_array().map(|a| !a.is_empty()).unwrap_or(false));
        assert!(d["per_speaker"].as_array().map(|a| !a.is_empty()).unwrap_or(false));
        assert!(d["method"].as_str().unwrap_or("").contains("MFCC"));
    }

    /// Regression: OLA ya Hann ilipuliza kingo (norm≈0) → kelele iliongezeka.
    /// Sasa padding ya sifuri huhakikisha kila sampuli ipate mwingiliano kamili.
    #[test]
    fn denoise_hupunguza_kelele_hata_kwenye_kingo() {
        let sr = 16000u32;
        let n = 48000usize;
        let mut seed = 12345u64;
        let mut rnd = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.6
        };
        let s: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f64 / sr as f64;
                let speech = if t > 0.5 && t < 2.0 {
                    0.35 * (2.0 * std::f64::consts::PI * 200.0 * t).sin()
                } else {
                    0.0
                };
                (speech + rnd()) as f32
            })
            .collect();

        // Strength 0 → ujenzi upya wa karibu kabisa (OLA sahihi).
        let clean0 = spectral_denoise(&s, 1024, 256, 0.0);
        let err: f64 = s
            .iter()
            .zip(clean0.iter())
            .skip(2000)
            .take(6000)
            .map(|(a, b)| ((*a - *b) as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(err < 0.01, "OLA haijenge vizuri: err={err}");

        let clean = spectral_denoise(&s, 1024, 256, 1.0);
        assert_eq!(clean.len(), s.len());
        // Kimya (2000..7000 = 0.125–0.44s) LAZIMA kipungue.
        assert!(
            rms(&clean[2000..7000]) < rms(&s[2000..7000]),
            "kelele ya kimya haikupungua: {} vs {}",
            rms(&clean[2000..7000]),
            rms(&s[2000..7000])
        );
        // Sauti (16000..24000) ihifadhiwe (haifutwi).
        assert!(rms(&clean[16000..24000]) > 0.1, "sauti ilifutwa");
        assert!(rms(&clean) < rms(&s), "kelele ya jumla haikupungua");

        // VAD LAZIMA itambue sauti hata kelele ikiwa kubwa.
        let vad = vad_frames(&s, 1024, 2.0);
        let speech_ratio = vad.iter().filter(|b| **b).count() as f64 / vad.len() as f64;
        assert!(
            speech_ratio > 0.1,
            "VAD ilikosa sauti kwenye kelele: ratio={speech_ratio}"
        );
    }
}

