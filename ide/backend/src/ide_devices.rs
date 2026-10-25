//! ide_devices.rs — MAZINGIRA HALISI ya kifaa: Android + iPhone/iPad.
//!
//! Inatoa:
//!   catalog  — orodha ya vifaa vyenye specs HALISI (resolution, dpi, notch, nav)
//!   frame    — HTML ya kifaa kamili (status bar, notch/island/punch-hole, nav bar,
//!              home screen) — inaonekana kama simu halisi
//!   launch   — kuzindua emulator HALISI: Android (`emulator -avd`) au
//!              iOS Simulator (`xcrun simctl boot`) — hukagua tooling kwanza
//!
//! KANUNI: hakuna uwongo — `launch` hurudisha status halisi ya tooling; kama
//! emulator/simulator haipo, inasema wazi (haitengenezi matokeo ya uongo).

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub struct Device {
    pub id: &'static str,
    pub label: &'static str,
    pub os: &'static str, // android | ios
    pub os_version: &'static str,
    pub chip: &'static str,
    pub w: u32, // native px
    pub h: u32,
    pub dpi: u32,
    /// none | notch | island | punch-center | punch-left | teardrop
    pub notch: &'static str,
    /// gesture | three-button | home-indicator | home-button
    pub nav: &'static str,
}

pub const DEVICES: &[Device] = &[
    // ---------------- ANDROID ----------------
    Device { id: "pixel-7", label: "Pixel 7", os: "android", os_version: "Android 13", chip: "Tensor G2", w: 1080, h: 2400, dpi: 416, notch: "punch-center", nav: "gesture" },
    Device { id: "pixel-8", label: "Pixel 8", os: "android", os_version: "Android 14", chip: "Tensor G3", w: 1080, h: 2400, dpi: 428, notch: "punch-center", nav: "gesture" },
    Device { id: "pixel-9", label: "Pixel 9", os: "android", os_version: "Android 15", chip: "Tensor G4", w: 1080, h: 2424, dpi: 422, notch: "punch-center", nav: "gesture" },
    Device { id: "galaxy-s23", label: "Galaxy S23", os: "android", os_version: "Android 13", chip: "Snapdragon 8 Gen 2", w: 1080, h: 2340, dpi: 425, notch: "punch-center", nav: "gesture" },
    Device { id: "galaxy-s24", label: "Galaxy S24", os: "android", os_version: "Android 14", chip: "Exynos 2400", w: 1080, h: 2340, dpi: 416, notch: "punch-center", nav: "gesture" },
    Device { id: "oneplus-12", label: "OnePlus 12", os: "android", os_version: "Android 14", chip: "Snapdragon 8 Gen 3", w: 1264, h: 2780, dpi: 450, notch: "punch-left", nav: "gesture" },
    Device { id: "xiaomi-14", label: "Xiaomi 14", os: "android", os_version: "Android 14", chip: "Snapdragon 8 Gen 3", w: 1200, h: 2670, dpi: 460, notch: "punch-center", nav: "gesture" },
    // ---------------- iOS ----------------
    Device { id: "iphone-se", label: "iPhone SE", os: "ios", os_version: "iOS 17", chip: "A15 Bionic", w: 750, h: 1334, dpi: 326, notch: "none", nav: "home-button" },
    Device { id: "iphone-13", label: "iPhone 13", os: "ios", os_version: "iOS 17", chip: "A15 Bionic", w: 1170, h: 2532, dpi: 460, notch: "notch", nav: "home-indicator" },
    Device { id: "iphone-14", label: "iPhone 14", os: "ios", os_version: "iOS 17", chip: "A15 Bionic", w: 1170, h: 2532, dpi: 460, notch: "notch", nav: "home-indicator" },
    Device { id: "iphone-15", label: "iPhone 15", os: "ios", os_version: "iOS 17", chip: "A16 Bionic", w: 1179, h: 2556, dpi: 460, notch: "island", nav: "home-indicator" },
    Device { id: "iphone-16", label: "iPhone 16", os: "ios", os_version: "iOS 18", chip: "A18", w: 1179, h: 2556, dpi: 460, notch: "island", nav: "home-indicator" },
    Device { id: "iphone-16-pro-max", label: "iPhone 16 Pro Max", os: "ios", os_version: "iOS 18", chip: "A18 Pro", w: 1320, h: 2868, dpi: 460, notch: "island", nav: "home-indicator" },
    Device { id: "ipad-pro-11", label: "iPad Pro 11", os: "ios", os_version: "iPadOS 17", chip: "M2", w: 1668, h: 2388, dpi: 264, notch: "none", nav: "home-indicator" },
];

pub fn find(id: &str) -> Option<&'static Device> {
    DEVICES.iter().find(|d| d.id == id)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub async fn catalog() -> Json<Value> {
    let list: Vec<Value> = DEVICES
        .iter()
        .map(|d| {
            json!({
                "id": d.id, "label": d.label, "os": d.os, "os_version": d.os_version,
                "chip": d.chip, "resolution": format!("{}×{}", d.w, d.h), "dpi": d.dpi,
                "notch": d.notch, "nav": d.nav,
            })
        })
        .collect();
    Json(json!({
        "ok": true,
        "android": DEVICES.iter().filter(|d| d.os == "android").count(),
        "ios": DEVICES.iter().filter(|d| d.os == "ios").count(),
        "devices": list,
    }))
}

/// Home screen ya Android (halisi: clock widget, app grid, dock, gesture bar).
fn android_home() -> String {
    let apps = [
        ("Phone", "#34a853", "📞"), ("Messages", "#4285f4", "💬"),
        ("Chrome", "#ea4335", "🌐"), ("Camera", "#5f6368", "📷"),
        ("Photos", "#fbbc05", "🖼️"), ("Maps", "#34a853", "🗺️"),
        ("Play Store", "#4285f4", "▶️"), ("Settings", "#5f6368", "⚙️"),
    ];
    let grid: String = apps
        .iter()
        .map(|(n, c, e)| {
            format!(
                r#"<div class="app"><div class="ico" style="background:{c}">{e}</div><span>{n}</span></div>"#
            )
        })
        .collect();
    let dock = [("Phone", "#34a853", "📞"), ("Chrome", "#ea4335", "🌐"), ("Messages", "#4285f4", "💬"), ("Camera", "#5f6368", "📷")]
        .iter()
        .map(|(n, c, e)| format!(r#"<div class="app"><div class="ico" style="background:{c}">{e}</div></div>"#))
        .collect::<String>();
    format!(
        r#"<div class="home android-home">
  <div class="clock"><b>9:41</b><span>Jumatatu, 12 Oktoba</span></div>
  <div class="grid">{grid}</div>
  <div class="dock">{dock}</div>
</div>"#
    )
}

/// Home screen ya iOS (halisi: widget, app grid, dock, page dots).
fn ios_home() -> String {
    let apps = [
        ("FaceTime", "#34c759", "📹"), ("Calendar", "#ff3b30", "📅"),
        ("Photos", "#ffcc00", "🖼️"), ("Camera", "#8e8e93", "📷"),
        ("Maps", "#34c759", "🗺️"), ("Weather", "#007aff", "☁️"),
        ("Clock", "#1c1c1e", "🕐"), ("Settings", "#8e8e93", "⚙️"),
    ];
    let grid: String = apps
        .iter()
        .map(|(n, c, e)| format!(r#"<div class="app"><div class="ico" style="background:{c}">{e}</div><span>{n}</span></div>"#))
        .collect();
    let dock = [("Phone", "#34c759", "📞"), ("Safari", "#007aff", "🧭"), ("Messages", "#34c759", "💬"), ("Music", "#ff2d55", "🎵")]
        .iter()
        .map(|(n, c, e)| format!(r#"<div class="app"><div class="ico" style="background:{c}">{e}</div></div>"#))
        .collect::<String>();
    format!(
        r#"<div class="home ios-home">
  <div class="grid">{grid}</div>
  <div class="dots"><i class="on"></i><i></i></div>
  <div class="dock">{dock}</div>
</div>"#
    )
}

fn status_bar(os: &str, notch: &str) -> String {
    if os == "ios" {
        let pad = if notch == "island" { 62 } else if notch == "notch" { 34 } else { 8 };
        format!(
            r#"<div class="statusbar ios" style="padding-top:{pad}px"><span class="t">9:41</span><span class="r">📶 5G <span class="batt">100%</span></span></div>"#
        )
    } else {
        format!(
            r#"<div class="statusbar android"><span class="t">9:41</span><span class="r">5G 📶 <span class="batt">100</span></span></div>"#
        )
    }
}

fn notch_html(notch: &str) -> String {
    match notch {
        "island" => r#"<div class="island"></div>"#.into(),
        "notch" => r#"<div class="notch"></div>"#.into(),
        "punch-center" => r#"<div class="punch center"></div>"#.into(),
        "punch-left" => r#"<div class="punch left"></div>"#.into(),
        "teardrop" => r#"<div class="punch teardrop"></div>"#.into(),
        _ => String::new(),
    }
}

fn nav_html(nav: &str, os: &str) -> String {
    match nav {
        "gesture" => r#"<div class="navbar android"><div class="gesture"></div></div>"#.into(),
        "three-button" => r#"<div class="navbar android"><div class="tb"><i>◁</i><i>◯</i><i>□</i></div></div>"#.into(),
        "home-indicator" => r#"<div class="navbar ios"><div class="indicator"></div></div>"#.into(),
        _ => {
            let _ = os;
            String::new()
        }
    }
}

/// CSS ya kifaa — plain string (bila format! braces) kisha replace tokens.
const DEVICE_CSS: &str = r#"
*{box-sizing:border-box}
body{margin:0;min-height:100vh;background:radial-gradient(circle at 50% 0%,#1b2233,#0a0b10 60%);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;font-family:-apple-system,system-ui,Roboto,sans-serif;padding:18px}
.dev{position:relative;display:flex;flex-direction:column;align-items:center}
.frame{position:relative;background:linear-gradient(150deg,#2a2d34,#101216);border-radius:__CORNER__px;padding:__BEZEL_T__px __BEZEL_X__px __BEZEL_B__px;box-shadow:0 30px 90px #000c,0 0 0 2px #000 inset}
.frame::after{content:"";position:absolute;inset:-3px;border-radius:__CORNER__px;border:1px solid #ffffff14;pointer-events:none}
.screen{position:relative;width:__W__px;height:__H__px;border-radius:__SR__px;overflow:hidden;background:#000;display:flex;flex-direction:column}
.wall{position:absolute;inset:0;background:linear-gradient(160deg,__WALL1__,__WALL2__)}
.statusbar{position:relative;z-index:4;display:flex;justify-content:space-between;align-items:center;height:__SBH__px;padding:0 16px;font-size:12px;color:#fff;font-weight:600}
.statusbar.ios .t{margin-left:6px}
.batt{opacity:.85}
.island{position:absolute;top:9px;left:50%;transform:translateX(-50%);width:112px;height:32px;background:#000;border-radius:20px;z-index:6;box-shadow:0 0 0 1px #ffffff10}
.island::after{content:"";position:absolute;right:8px;top:50%;transform:translateY(-50%);width:9px;height:9px;border-radius:50%;background:#0b1a2a;box-shadow:0 0 3px #1e6fff88}
.notch{position:absolute;top:0;left:50%;transform:translateX(-50%);width:190px;height:26px;background:#000;border-radius:0 0 16px 16px;z-index:6}
.punch{position:absolute;width:11px;height:11px;border-radius:50%;background:#000;z-index:6;box-shadow:0 0 0 1px #ffffff12}
.punch.center{top:9px;left:50%;transform:translateX(-50%)}
.punch.left{top:10px;left:14px}
.punch.teardrop{top:8px;left:50%;transform:translateX(-50%);border-radius:0 0 9px 9px;width:16px;height:9px}
.content{position:relative;z-index:3;flex:1;overflow:auto;-webkit-overflow-scrolling:touch}
.content iframe{width:100%;height:100%;border:0;background:#fff}
.navbar{position:relative;z-index:4;display:flex;align-items:center;justify-content:center;height:__NAVH__px}
.navbar.android .gesture{width:112px;height:5px;border-radius:3px;background:#ffffffcc}
.navbar.ios .indicator{width:128px;height:5px;border-radius:3px;background:#fff}
.navbar.android .tb{display:flex;gap:38px;color:#fff;font-size:14px;opacity:.9}
.home{position:absolute;inset:0;display:flex;flex-direction:column;padding:10px 18px 6px}
.android-home{padding-top:14px}
.clock{color:#fff;text-align:left;margin:6px 4px 14px}
.clock b{display:block;font-size:42px;font-weight:200;line-height:1}
.clock span{font-size:12px;opacity:.85}
.grid{display:grid;grid-template-columns:repeat(4,1fr);gap:16px 10px}
.ios-home .grid{grid-template-columns:repeat(4,1fr);gap:20px 12px;margin-top:8px}
.app{display:flex;flex-direction:column;align-items:center;gap:5px;color:#fff;font-size:10px;text-shadow:0 1px 2px #0008}
.ico{width:46px;height:46px;border-radius:13px;display:flex;align-items:center;justify-content:center;font-size:22px;box-shadow:0 4px 10px #0006}
.dots{display:flex;gap:5px;justify-content:center;margin:auto 0 6px}
.dots i{width:6px;height:6px;border-radius:50%;background:#ffffff66}
.dots i.on{background:#fff}
.dock{margin-top:auto;display:flex;justify-content:space-around;background:#ffffff1f;backdrop-filter:blur(14px);border-radius:22px;padding:10px 8px}
.dock .ico{width:46px;height:46px}
.home-btn{position:absolute;bottom:7px;left:50%;transform:translateX(-50%);width:34px;height:34px;border-radius:50%;border:2px solid #333;background:#0a0a0a}
.earpiece{position:absolute;top:14px;left:50%;transform:translateX(-50%);width:56px;height:6px;border-radius:3px;background:#0a0a0a}
.side{position:absolute;width:3px;border-radius:2px;background:#2b2f36}
.side.power{right:-3px;top:26%;height:64px}
.side.vol{left:-3px;top:20%;height:44px}
.caption{color:#8b98ab;font-size:12px;text-align:center;line-height:1.5}
.caption b{color:#dbe4f0}
.caption .spec{color:#6b7a8d;font:11px ui-monospace,monospace}
"#;

fn build_frame(d: &Device, app_html: &str, landscape: bool) -> String {
    let (nw, nh) = if landscape { (d.h, d.w) } else { (d.w, d.h) };
    // Display size: fit within bounds, preserve aspect.
    let max_h = if d.id.starts_with("ipad") { 820.0 } else { 720.0 };
    let max_w = 420.0;
    let ar = nw as f64 / nh as f64;
    let mut dh = max_h;
    let mut dw = dh * ar;
    if dw > max_w {
        dw = max_w;
        dh = dw / ar;
    }
    let w = dw.round() as u32;
    let h = dh.round() as u32;

    let is_ios = d.os == "ios";
    let (corner, sr, bezel_t, bezel_x, bezel_b) = if d.nav == "home-button" {
        (44, 4, 34, 12, 44) // iPhone SE: bezels + home button
    } else if d.id.starts_with("ipad") {
        (30, 16, 16, 16, 16)
    } else if is_ios {
        (52, 40, 12, 12, 12)
    } else {
        (40, 30, 10, 10, 10)
    };
    let sbh = if is_ios { 44 } else { 30 };
    let navh = if d.nav == "home-button" { 0 } else if is_ios { 22 } else { 24 };
    let (wall1, wall2) = if is_ios {
        ("#1e3a5f", "#0b1522")
    } else {
        ("#12324a", "#0a1a12")
    };

    let css = DEVICE_CSS
        .replace("__CORNER__", &corner.to_string())
        .replace("__BEZEL_T__", &bezel_t.to_string())
        .replace("__BEZEL_X__", &bezel_x.to_string())
        .replace("__BEZEL_B__", &bezel_b.to_string())
        .replace("__W__", &w.to_string())
        .replace("__H__", &h.to_string())
        .replace("__SR__", &sr.to_string())
        .replace("__SBH__", &sbh.to_string())
        .replace("__NAVH__", &navh.to_string())
        .replace("__WALL1__", wall1)
        .replace("__WALL2__", wall2);

    let home_btn = if d.nav == "home-button" { r#"<div class="home-btn"></div>"# } else { "" };
    let earpiece = if d.nav == "home-button" { r#"<div class="earpiece"></div>"# } else { "" };
    let caption = format!(
        r#"<div class="caption"><b>{label}</b> · {os} {osv}<br/><span class="spec">{chip} · {w}×{h} · {dpi}dpi{orient}</span></div>"#,
        label = esc(d.label),
        os = if is_ios { "iOS" } else { "Android" },
        osv = d.os_version.split_whitespace().last().unwrap_or(""),
        chip = esc(d.chip),
        w = nw,
        h = nh,
        dpi = d.dpi,
        orient = if landscape { " · landscape" } else { "" },
    );

    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>{label}</title><style>{css}</style></head>
<body><div class="dev">
  <div class="frame">
    <div class="screen">
      <div class="wall"></div>
      {notch}
      {status}
      <div class="content">{app}</div>
      {nav}
    </div>
    {earpiece}
    {home_btn}
    <div class="side power"></div><div class="side vol"></div>
  </div>
  {caption}
</div></body></html>"#,
        label = esc(d.label),
        css = css,
        notch = notch_html(d.notch),
        status = status_bar(d.os, d.notch),
        app = app_html,
        nav = nav_html(d.nav, d.os),
        earpiece = earpiece,
        home_btn = home_btn,
        caption = caption,
    )
}

/// POST /api/ide/device/frame — HTML ya kifaa kamili.
pub async fn device_frame(Json(body): Json<Value>) -> Json<Value> {
    let id = body["device"].as_str().unwrap_or("iphone-16");
    let landscape = body["orientation"].as_str() == Some("landscape");
    let Some(d) = find(id) else {
        return Json(json!({ "ok": false, "error": format!("kifaa hakijulikani: {id}") }));
    };

    let app = if let Some(code) = body["code"].as_str().filter(|c| !c.is_empty()) {
        format!(r#"<iframe sandbox="allow-scripts allow-same-origin allow-forms" srcdoc="{c}"></iframe>"#, c = esc(code))
    } else if let Some(path) = body["path"].as_str().filter(|p| !p.is_empty()) {
        let rel = path.trim_start_matches('/');
        if rel.contains("..") {
            return Json(json!({ "ok": false, "error": "path nje ya workspace" }));
        }
        let full = crate::ide_api::ide_workspace_root().join(rel);
        match std::fs::read_to_string(&full) {
            Ok(c) => format!(r#"<iframe sandbox="allow-scripts allow-same-origin allow-forms" srcdoc="{c}"></iframe>"#, c = esc(&c)),
            Err(e) => return Json(json!({ "ok": false, "error": format!("faili haipatikani: {e}") })),
        }
    } else if d.os == "ios" {
        ios_home()
    } else {
        android_home()
    };

    let html = build_frame(d, &app, landscape);
    Json(json!({
        "ok": true,
        "kind": d.os,
        "preview": "iframe",
        "html": html,
        "device": d.label,
        "device_id": d.id,
        "os_version": d.os_version,
        "resolution": format!("{}×{}", if landscape { d.h } else { d.w }, if landscape { d.w } else { d.h }),
        "orientation": if landscape { "landscape" } else { "portrait" },
    }))
}

async fn run_capture(cmd: &str) -> Value {
    let out = tokio::process::Command::new("bash")
        .arg("-lc")
        .arg(cmd)
        .output()
        .await;
    match out {
        Ok(o) => json!({
            "ok": o.status.success(),
            "stdout": String::from_utf8_lossy(&o.stdout).trim().to_string(),
            "stderr": String::from_utf8_lossy(&o.stderr).trim().to_string(),
        }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

async fn has(bin: &str) -> bool {
    let out = tokio::process::Command::new("bash")
        .arg("-lc")
        .arg(format!("command -v {bin}"))
        .output()
        .await;
    matches!(out, Ok(o) if o.status.success())
}

/// POST /api/ide/device/launch — kuzindua emulator/simulator HALISI.
/// Hurudisha status halisi ya tooling; haiandishi matokeo ya uongo.
pub async fn device_launch(State(_s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let os = body["os"].as_str().unwrap_or("android").to_lowercase();
    let avd = body["avd"].as_str().unwrap_or("");
    let udid = body["udid"].as_str().unwrap_or("");

    if os == "android" {
        let has_emulator = has("emulator").await;
        let has_adb = has("adb").await;
        let avds = if has_emulator {
            run_capture("emulator -list-avds 2>/dev/null").await
        } else {
            json!({ "ok": false, "note": "emulator haipo" })
        };
        let devices = if has_adb {
            run_capture("adb devices 2>/dev/null").await
        } else {
            json!({ "ok": false, "note": "adb haipo" })
        };
        let launch = if !avd.is_empty() && has_emulator {
            run_capture(&format!(
                "nohup emulator -avd {avd} -no-snapshot-save >/tmp/mtechos-emu.log 2>&1 & echo launched"
            ))
            .await
        } else {
            json!({ "ok": false, "note": "weka 'avd' + sakinisha Android SDK emulator" })
        };
        return Json(json!({
            "ok": has_emulator,
            "platform": "android",
            "tooling": { "emulator": has_emulator, "adb": has_adb },
            "avds": avds,
            "connected": devices,
            "launch": launch,
            "install_hint": "sdkmanager --install 'emulator' 'system-images;android-34;google_apis;x86_64' && avdmanager create avd -n mtechos -k 'system-images;android-34;google_apis;x86_64'",
        }));
    }

    // iOS Simulator
    let has_xcrun = has("xcrun").await;
    let sims = if has_xcrun {
        run_capture("xcrun simctl list devices available 2>/dev/null | head -40").await
    } else {
        json!({ "ok": false, "note": "xcrun haipo (macOS/Xcode pekee)" })
    };
    let launch = if !udid.is_empty() && has_xcrun {
        run_capture(&format!("xcrun simctl boot {udid} 2>&1; open -a Simulator 2>/dev/null; echo done")).await
    } else {
        json!({ "ok": false, "note": "weka 'udid' + sakinisha Xcode" })
    };
    Json(json!({
        "ok": has_xcrun,
        "platform": "ios",
        "tooling": { "xcrun": has_xcrun },
        "simulators": sims,
        "launch": launch,
        "install_hint": "Xcode → Settings → Platforms → iOS Simulator (macOS pekee)",
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/device/catalog", get(catalog))
        .route("/api/ide/device/frame", post(device_frame))
        .route("/api/ide/device/launch", post(device_launch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_both_platforms() {
        assert!(DEVICES.iter().any(|d| d.os == "android"));
        assert!(DEVICES.iter().any(|d| d.os == "ios"));
        assert!(DEVICES.len() >= 12);
    }

    #[test]
    fn ids_unique() {
        let mut ids: Vec<&str> = DEVICES.iter().map(|d| d.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn frame_contains_device_specs() {
        let d = find("iphone-16").unwrap();
        let html = build_frame(d, &ios_home(), false);
        assert!(html.contains("iPhone 16"));
        assert!(html.contains("island"));
        assert!(html.contains("1179"));
    }

    #[test]
    fn android_frame_has_punch_hole() {
        let d = find("pixel-8").unwrap();
        let html = build_frame(d, &android_home(), false);
        assert!(html.contains("punch center"));
        assert!(html.contains("Android"));
    }

    #[test]
    fn se_has_home_button() {
        let d = find("iphone-se").unwrap();
        let html = build_frame(d, &ios_home(), false);
        assert!(html.contains("home-btn"));
    }

    #[test]
    fn landscape_swaps() {
        let d = find("pixel-8").unwrap();
        let html = build_frame(d, &android_home(), true);
        assert!(html.contains("2400×1080"));
    }
}
