//! nyi_media.rs — MEDIA ya 2NYI: chati, picha, video — vyote kwa data HALISI.
//!
//! Hukusanya kwenye sehemu MOJA:
//!  - **chati/visualization** (`nyi_viz`) — SVG halisi, offline.
//!  - **picha** — image_gen (fal.ai/gpt-image) + vision (kuelewa picha).
//!  - **video** — ffmpeg kama ipo (inachunguzwa kwa `which`); vinginevyo hurudisha
//!    maelekezo wazi BILA kubuni matokeo (KANUNI ya grounding ya 2NYI).
//!
//! KANUNI: hakuna "inafanya kazi" bila uthibitisho — zana isiyopo hurudisha
//! `available: false` pamoja na jinsi ya kuisakinisha.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

/// Saraka ya media ya mradi (data/media) — inaundwa kama haipo.
pub fn media_dir() -> PathBuf {
    let d = crate::data_io::data_dir().join("media");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Je, ffmpeg ipo kwenye PATH? (uthibitisho halisi, si dhana)
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Hali ya media: zana zipi zipo HALISI (kwa UI na kwa LLM).
pub fn status() -> Value {
    json!({
        "chart": { "available": true, "kinds": ["line","bar","pie","scatter","sparkline"],
                   "engine": "nyi_viz (pure Rust SVG, offline, hakuna dependency)" },
        "image": { "available": true, "engine": "image_gen (fal.ai / gpt-image) + vision",
                   "note": "Picha hutengenezwa kwa URL; vision huchunguza picha kutoka URL." },
        "video": { "available": ffmpeg_available(), "engine": "ffmpeg",
                   "note": if ffmpeg_available() {
                       "ffmpeg ipo — video inawezekana (mp4/gif kutoka picha au frames)."
                   } else {
                       "ffmpeg HAIPO kwenye mashine hii. Sakinisha kwa: apt-get install -y ffmpeg. \
                        Bila ffmpeg, video HAITENGENEZWI — hatubuni tokeo."
                   }.to_string() },
        "media_dir": media_dir().display().to_string(),
    })
}

/// Chati kutoka data halisi → SVG + data-URL (tayari kwa chat/IDE).
pub fn chart(kind: &str, title: &str, data: &Value) -> Value {
    let pts = crate::nyi_viz::parse_points(data);
    let svg = crate::nyi_viz::chart_svg(kind, title, &pts);
    json!({
        "ok": true,
        "kind": kind,
        "title": title,
        "points": pts.len(),
        "svg": svg,
        "data_url": crate::nyi_viz::data_url(&svg),
        "note": "SVG halisi kutoka data uliyotoa — hakuna data ya kubuni.",
    })
}

/// Tengeneza video kutoka fremu (picha) kwa ffmpeg. Hurudisha njia ya faili
/// au kosa la wazi. HAIbuni tokeo.
///
/// `frames`: njia za picha (png/jpg) kwa mfuatano; `fps`: fremu kwa sekunde.
pub fn video_from_frames(frames: &[String], fps: u32) -> Result<PathBuf, String> {
    if frames.is_empty() {
        return Err("hakuna fremu (frames) zilizotolewa".into());
    }
    if !ffmpeg_available() {
        return Err(
            "ffmpeg haipo — video haitengenezwi. Sakinisha: apt-get install -y ffmpeg".into(),
        );
    }
    for f in frames {
        if !std::path::Path::new(f).is_file() {
            return Err(format!("fremu haipo: {f}"));
        }
    }
    let out = media_dir().join(format!("video_{}.mp4", chrono::Utc::now().timestamp()));
    let list = media_dir().join("frames.txt");
    let body: String = frames
        .iter()
        .map(|f| format!("file '{}'\n", f.replace('\'', "'\\''")))
        .collect();
    std::fs::write(&list, body).map_err(|e| e.to_string())?;
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            &list.display().to_string(),
            "-r",
            &fps.to_string(),
            "-pix_fmt",
            "yuv420p",
            &out.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&list);
    if !status.success() || !out.is_file() {
        return Err("ffmpeg imeshindwa kutengeneza video (hakuna faili lililotolewa)".into());
    }
    Ok(out)
}

/// Unda GIF kutoka picha (ffmpeg) — kwa onyesho la haraka.
pub fn gif_from_frames(frames: &[String], fps: u32) -> Result<PathBuf, String> {
    if !ffmpeg_available() {
        return Err("ffmpeg haipo — GIF haitengenezwi. Sakinisha: apt-get install -y ffmpeg".into());
    }
    let mp4 = video_from_frames(frames, fps)?;
    let gif = media_dir().join(format!("anim_{}.gif", chrono::Utc::now().timestamp()));
    let palette = media_dir().join("palette.png");
    let ok = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &mp4.display().to_string(),
            "-vf",
            &format!("fps={fps},scale=640:-1:flags=lanczos,palettegen"),
            &palette.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return Err("ffmpeg imeshindwa kutengeneza palette ya GIF".into());
    }
    let ok2 = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &mp4.display().to_string(),
            "-i",
            &palette.display().to_string(),
            "-lavfi",
            &format!("fps={fps},scale=640:-1:flags=lanczos[x];[x][1:v]paletteuse"),
            &gif.display().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&palette);
    if !ok2 || !gif.is_file() {
        return Err("ffmpeg imeshindwa kutengeneza GIF".into());
    }
    Ok(gif)
}

/// Chambua video kwa ffprobe (metadata halisi) kama ffmpeg ipo.
pub fn probe_video(path: &str) -> Value {
    if !std::path::Path::new(path).is_file() {
        return json!({ "ok": false, "error": format!("faili haipo: {path}") });
    }
    if !ffmpeg_available() {
        return json!({ "ok": false, "error": "ffmpeg/ffprobe haipo kwenye mashine hii" });
    }
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path,
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
            json!({ "ok": true, "path": path, "probe": v })
        }
        Ok(o) => json!({ "ok": false, "error": String::from_utf8_lossy(&o.stderr).to_string() }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Sehemu ya chat inayoweza kuonyesha media (kwa UI): aina ya block.
/// Hii hufafanua jinsi jibu la LLM linapaswa kupangwa ili UI ionyeshe picha/chati/video.
pub fn chat_block(kind: &str, payload: &Value) -> Value {
    json!({
        "type": kind,
        "payload": payload,
        "render": match kind {
            "chart" => "svg (inline <img src=data-url> au innerHTML ya SVG salama)",
            "image" => "img (URL)",
            "video" => "video (mp4/gif URL au faili la media_dir)",
            _ => "text",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_ina_chart_na_image_zinazopatikana() {
        let s = status();
        assert_eq!(s["chart"]["available"], json!(true));
        assert_eq!(s["image"]["available"], json!(true));
        // video hutegemea ffmpeg — thamani lazima iwe ya kweli (bool), si kubuniwa.
        assert!(s["video"]["available"].is_boolean());
    }

    #[test]
    fn chart_huzalisha_svg_na_data_url() {
        let d = json!([{"label": "a", "value": 1}, {"label": "b", "value": 4}]);
        let c = chart("bar", "Pembejeo", &d);
        assert_eq!(c["ok"], json!(true));
        assert!(c["svg"].as_str().unwrap().contains("<svg"));
        assert!(c["data_url"].as_str().unwrap().starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn video_bila_fremu_hurudisha_kosa_la_wazi() {
        let e = video_from_frames(&[], 12).unwrap_err();
        assert!(e.contains("fremu"));
    }

    #[test]
    fn probe_faili_lisilopo_hurudisha_ok_false() {
        let r = probe_video("/nonexistent/nope.mp4");
        assert_eq!(r["ok"], json!(false));
    }

    #[test]
    fn chat_block_ina_render_inaolingana() {
        assert_eq!(chat_block("chart", &json!({}))["render"], json!("svg (inline <img src=data-url> au innerHTML ya SVG salama)"));
        assert_eq!(chat_block("video", &json!({}))["render"], json!("video (mp4/gif URL au faili la media_dir)"));
    }

    /// End-to-end HALISI: tengeneza fremu kwa ffmpeg, kisha video (mp4) + GIF.
    /// Hukimbia tu kama ffmpeg ipo — vinginevyo huruka (hakuna madai ya uongo).
    #[test]
    fn video_na_gif_halisi_kwa_ffmpeg() {
        if !ffmpeg_available() {
            eprintln!("ffmpeg haipo — test ya video imerukwa (si kosa)");
            return;
        }
        let tmp = std::env::temp_dir().join(format!("nyi_vid_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let mut frames = Vec::new();
        for i in 0..4 {
            let f = tmp.join(format!("f{i}.png"));
            let ok = Command::new("ffmpeg")
                .args([
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    &format!("color=c=red:s=64x64:d=1"),
                    "-frames:v",
                    "1",
                    &f.display().to_string(),
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "ffmpeg imeshindwa kutengeneza fremu");
            frames.push(f.display().to_string());
        }
        let mp4 = video_from_frames(&frames, 8).expect("video ya mp4");
        assert!(mp4.is_file(), "mp4 lazima iwepo");
        assert!(std::fs::metadata(&mp4).unwrap().len() > 0, "mp4 tupu");
        let probe = probe_video(&mp4.display().to_string());
        assert_eq!(probe["ok"], json!(true), "ffprobe: {probe}");
        let gif = gif_from_frames(&frames, 8).expect("gif");
        assert!(gif.is_file(), "gif lazima iwepo");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_file(&mp4);
        let _ = std::fs::remove_file(&gif);
    }
}
