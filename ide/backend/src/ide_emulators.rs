//! ide_emulators.rs — Website / Mobile / iOS / Desktop / IoT / Gaming / Rust / R / API

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ide_api::ide_workspace_root;

fn sessions() -> &'static Mutex<HashMap<String, Value>> {
    static S: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn new_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("emu-{n}")
}

fn safe_rel(rel: &str) -> Option<PathBuf> {
    let root = ide_workspace_root();
    let rel = rel.trim_start_matches('/');
    if rel.contains("..") {
        return None;
    }
    let full = root.join(rel);
    if !full.starts_with(&root) {
        return None;
    }
    Some(full)
}

pub async fn ide_emulate(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let kind = body["kind"].as_str().unwrap_or("website").to_lowercase();
    dispatch(s, kind, body).await
}

pub async fn ide_emulate_kind(
    State(s): State<crate::AppState>,
    Path(kind): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    dispatch(s, kind.to_lowercase(), body).await
}

async fn dispatch(s: crate::AppState, kind: String, body: Value) -> Json<Value> {
    let path = body["path"].as_str().unwrap_or("").to_string();
    let entry = body["entry"].as_str().unwrap_or("").to_string();
    let code = body["code"].as_str().unwrap_or("").to_string();

    let result = match kind.as_str() {
        "website" | "web" => emulate_website(&path, &code).await,
        "mobile" | "android" => {
            let device = body["device"].as_str().unwrap_or("pixel-8");
            emulate_device(device, &path, &code, body["orientation"].as_str()).await
        }
        "ios" | "iphone" | "ipad" => {
            let device = body["device"].as_str().unwrap_or("iphone-16");
            emulate_device(device, &path, &code, body["orientation"].as_str()).await
        }
        "desktop" | "tauri" | "native" => emulate_desktop(&s, &path, &entry, &body).await,
        "iot" | "sensor" => emulate_iot(&code).await,
        "gaming" | "game" | "3d" => emulate_gaming(&path, &code).await,
        "api" | "backend" => emulate_api(&s, &path, &entry).await,
        "rust" => emulate_rust(&s, &path, &entry, &body).await,
        "r" | "stats" => emulate_r(&code, &path).await,
        _ => json!({ "ok": false, "error": format!("kind isiyojulikana: {kind}") }),
    };

    // Mradi KAMILI (kamilisha mfu + jijaribu) — kama `project:true` au `complete:true`.
    let want_project = body["project"].as_bool().unwrap_or(false) || body["complete"].as_bool().unwrap_or(false);
    let mut session = result;
    if want_project && crate::forge_projects::is_project(&kind) {
        let proj_name = if path.trim().is_empty() { "app" } else { path.trim() };
        let gen = crate::ide_pipeline::generate_and_test(&kind, proj_name).await;
        if let Some(obj) = session.as_object_mut() {
            obj.insert("project".into(), gen);
        }
    }

    let id = new_id();
    if let Some(obj) = session.as_object_mut() {
        obj.insert("session_id".into(), json!(id));
        obj.insert("kind".into(), json!(kind));
    }
    if let Ok(mut map) = sessions().lock() {
        map.insert(id, session.clone());
        if map.len() > 40 {
            let keys: Vec<_> = map.keys().cloned().collect();
            for k in keys.into_iter().take(map.len().saturating_sub(40)) {
                map.remove(&k);
            }
        }
    }
    Json(session)
}

pub async fn ide_emulate_sessions() -> Json<Value> {
    let list: Vec<Value> = sessions()
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    Json(json!({ "ok": true, "sessions": list }))
}

async fn emulate_website(path: &str, code: &str) -> Value {
    let html = if !code.is_empty() {
        code.to_string()
    } else if !path.is_empty() {
        match safe_rel(path).and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(c) => c,
            None => return json!({ "ok": false, "error": "faili HTML haipatikani" }),
        }
    } else {
        r#"<!DOCTYPE html><html><body style="font-family:system-ui;background:#0d1117;color:#e6edf3;padding:24px"><h1>Web Preview</h1></body></html>"#.into()
    };
    json!({ "ok": true, "kind": "website", "preview": "iframe", "html": html })
}

/// MAZINGIRA HALISI ya kifaa (Android/iPhone) — hujenga chrome kamili
/// (status bar, notch/island/punch, nav) + specs halisi. Hutumia `ide_devices`.
async fn emulate_device(device: &str, path: &str, code: &str, orientation: Option<&str>) -> Value {
    let body = json!({
        "device": device,
        "path": path,
        "code": code,
        "orientation": orientation.unwrap_or("portrait"),
    });
    let res = crate::ide_devices::device_frame(Json(body)).await;
    res.0
}

async fn emulate_desktop(s: &crate::AppState, path: &str, entry: &str, body: &Value) -> Value {
    let root = ide_workspace_root();
    let workdir = if !path.is_empty() {
        safe_rel(path)
            .map(|p| if p.is_file() { p.parent().unwrap_or(&p).to_path_buf() } else { p })
            .unwrap_or(root.clone())
    } else {
        root.clone()
    };
    let cargo = workdir.join("Cargo.toml");
    let cmd = if cargo.exists() {
        let bin = body["bin"].as_str().unwrap_or("");
        if bin.is_empty() { "cargo run --release 2>&1".into() } else { format!("cargo run --release --bin {bin} 2>&1") }
    } else if !entry.is_empty() {
        format!("./{entry} 2>&1")
    } else {
        return json!({ "ok": false, "error": "Hakuna Cargo.toml wala entry", "workdir": workdir.display().to_string() });
    };
    run_shell(s, &cmd, &workdir.display().to_string()).await
}

async fn emulate_iot(code: &str) -> Value {
    let r_code = if !code.is_empty() { code.to_string() } else {
        r#"set.seed(42); n<-48; time<-seq_len(n); temp<-22+3*sin(time/6)+rnorm(n,0,0.4);
cat(sprintf("IoT n=%d temp mean=%.2f\n",n,mean(temp)))"#.into()
    };
    let r_result = run_rscript(&r_code).await;
    // Series halisi: deterministic (seed), inaonyesha temp + humidity + anomaly.
    let mut series = Vec::new();
    let mut s: u64 = 42 | 1;
    let mut next = || {
        s ^= s >> 12; s ^= s << 25; s ^= s >> 27;
        ((s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let anomaly_at = 17usize;
    for i in 0..48 {
        let phase = (i as f64) / 6.0;
        let mut temp = 22.0 + 3.0 * phase.sin() + (next() - 0.5) * 0.8;
        let hum = (55.0 + 8.0 * (phase / 2.0).cos() + (next() - 0.5) * 2.4).clamp(0.0, 100.0);
        let anomaly = i == anomaly_at;
        if anomaly { temp += 9.0; }
        series.push(json!({
            "t": i,
            "temp": (temp * 100.0).round() / 100.0,
            "humidity": (hum * 100.0).round() / 100.0,
            "anomaly": anomaly,
        }));
    }
    let anomalies = series.iter().filter(|p| p["anomaly"].as_bool().unwrap_or(false)).count();
    json!({
        "ok": true, "kind": "iot", "preview": "iot-dashboard",
        "series": series, "anomalies": anomalies, "r": r_result,
        "lab": "/api/ide/lab/telemetry"
    })
}

async fn emulate_gaming(path: &str, code: &str) -> Value {
    let custom = if !code.is_empty() {
        code.to_string()
    } else if !path.is_empty() {
        safe_rel(path).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default()
    } else {
        String::new()
    };
    // Kama mtumiaji ameandika HTML/Three.js yake, itumike. La sivyo, scene HALISI
    // ya Three.js (CDN) yenye mwanga, mzunguko, na HUD — inayojaribu 3D moja kwa moja.
    let html = if custom.contains("<html") || custom.contains("three") {
        custom
    } else {
        default_game_scene()
    };
    json!({ "ok": true, "kind": "gaming", "preview": "iframe", "html": html, "engine": "three.js" })
}

fn default_game_scene() -> String {
    r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<style>html,body{margin:0;height:100%;background:#05070d;overflow:hidden;font-family:ui-monospace,monospace}
#hud{position:fixed;top:10px;left:12px;color:#9fb2c8;font-size:12px;z-index:9;text-shadow:0 1px 2px #000}
#hud b{color:#58a6ff}#hint{position:fixed;bottom:10px;left:12px;color:#6b7a8d;font-size:11px;z-index:9}</style>
<script type="importmap">{"imports":{"three":"https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.module.js"}}</script>
</head><body>
<div id="hud">🎮 MTECHOS 3D · <b id="fps">--</b> fps · <b id="objs">0</b> objects</div>
<div id="hint">Drag = zungusha · Wheel = zoom</div>
<script type="module">
import * as THREE from 'three';
const renderer = new THREE.WebGLRenderer({antialias:true});
renderer.setSize(innerWidth, innerHeight); renderer.setPixelRatio(Math.min(devicePixelRatio,2));
document.body.appendChild(renderer.domElement);
const scene = new THREE.Scene(); scene.fog = new THREE.Fog(0x05070d, 12, 40);
const camera = new THREE.PerspectiveCamera(60, innerWidth/innerHeight, 0.1, 200);
camera.position.set(0, 5, 12);
scene.add(new THREE.AmbientLight(0x8899bb, 0.6));
const key = new THREE.DirectionalLight(0xffffff, 1.4); key.position.set(6,10,7); scene.add(key);
const rim = new THREE.PointLight(0x58a6ff, 2.2, 40); rim.position.set(-8,4,-6); scene.add(rim);
const grid = new THREE.GridHelper(40, 40, 0x1f6feb, 0x12304d); scene.add(grid);
const objs = [];
const palette = [0x58a6ff, 0x3fb950, 0xd29922, 0xf85149, 0xbc8cff, 0x39c5cf];
for (let i=0;i<12;i++){
  const geo = i%3===0 ? new THREE.TorusKnotGeometry(0.7,0.24,120,16)
            : i%3===1 ? new THREE.IcosahedronGeometry(0.85,0)
            : new THREE.BoxGeometry(1.1,1.1,1.1);
  const mat = new THREE.MeshStandardMaterial({color:palette[i%palette.length], metalness:0.4, roughness:0.25, emissive:palette[i%palette.length], emissiveIntensity:0.12});
  const m = new THREE.Mesh(geo, mat);
  const a = (i/12)*Math.PI*2, r = 5 + (i%2)*1.6;
  m.position.set(Math.cos(a)*r, 1.6 + (i%3)*1.1, Math.sin(a)*r);
  m.userData.spin = 0.005 + (i%5)*0.002; m.userData.bob = a;
  scene.add(m); objs.push(m);
}
document.getElementById('objs').textContent = objs.length;
let drag=false, px=0, py=0, theta=0, phi=0.3, radius=14;
renderer.domElement.addEventListener('pointerdown',e=>{drag=true;px=e.clientX;py=e.clientY;});
addEventListener('pointerup',()=>drag=false);
addEventListener('pointermove',e=>{if(!drag)return;theta-=(e.clientX-px)*0.005;phi=Math.max(0.05,Math.min(1.4,phi-(e.clientY-py)*0.005));px=e.clientX;py=e.clientY;});
renderer.domElement.addEventListener('wheel',e=>{radius=Math.max(6,Math.min(28,radius+e.deltaY*0.01));e.preventDefault();},{passive:false});
let frames=0, last=performance.now(), t0=last;
function loop(){
  const now=performance.now(); const t=(now-t0)/1000;
  objs.forEach((m,i)=>{ m.rotation.x+=m.userData.spin; m.rotation.y+=m.userData.spin*1.3;
    m.position.y = 1.6 + (i%3)*1.1 + Math.sin(t*1.2 + m.userData.bob)*0.35; });
  camera.position.set(Math.sin(theta)*radius, 4+phi*4, Math.cos(theta)*radius);
  camera.lookAt(0,1.5,0);
  renderer.render(scene,camera);
  frames++; if(now-last>500){document.getElementById('fps').textContent=Math.round(frames*1000/(now-last));frames=0;last=now;}
  requestAnimationFrame(loop);
}
addEventListener('resize',()=>{camera.aspect=innerWidth/innerHeight;camera.updateProjectionMatrix();renderer.setSize(innerWidth,innerHeight);});
loop();
</script></body></html>"#.to_string()
}

async fn emulate_api(s: &crate::AppState, path: &str, entry: &str) -> Value {
    let root = ide_workspace_root();
    let workdir = safe_rel(path).unwrap_or(root);
    let cmd = if workdir.join("Cargo.toml").exists() {
        "cargo check 2>&1".into()
    } else if !entry.is_empty() {
        format!("curl -sS -m 5 {entry} 2>&1 || true")
    } else {
        "echo API".into()
    };
    run_shell(s, &cmd, &workdir.display().to_string()).await
}

async fn emulate_rust(s: &crate::AppState, path: &str, entry: &str, body: &Value) -> Value {
    let root = ide_workspace_root();
    let workdir = safe_rel(path).unwrap_or(root);
    let mode = body["mode"].as_str().unwrap_or("check");
    let cmd = match mode {
        "run" if !entry.is_empty() => format!("cargo run --release --bin {entry} 2>&1"),
        "run" => "cargo run --release 2>&1".into(),
        "test" => "cargo test 2>&1".into(),
        "build" => "cargo build --release 2>&1".into(),
        _ => "cargo check 2>&1".into(),
    };
    run_shell(s, &cmd, &workdir.display().to_string()).await
}

async fn emulate_r(code: &str, data_path: &str) -> Value {
    let code = if code.is_empty() { r#"cat("R OK\n")"#.to_string() } else { code.to_string() };
    let mut r_result = run_rscript(&code).await;
    if !data_path.is_empty() { r_result["data_path"] = json!(data_path); }
    json!({ "ok": true, "kind": "r", "preview": "console", "r": r_result })
}

async fn run_shell(s: &crate::AppState, cmd: &str, workdir: &str) -> Value {
    let Some(agent) = s.agent.as_ref() else { return run_process(cmd, workdir).await; };
    let mut params = json!({ "command": cmd, "timeout": 120000 });
    if !workdir.is_empty() { params["workdir"] = json!(workdir); }
    match agent.invoke_tool_raw(json!({ "tool": "terminal", "params": params })).await {
        Ok(v) => json!({ "ok": true, "preview": "console", "command": cmd, "workdir": workdir, "output": v["output"].as_str().unwrap_or(""), "tool_result": v }),
        Err(_) => run_process(cmd, workdir).await,
    }
}

async fn run_process(cmd: &str, workdir: &str) -> Value {
    let mut c = tokio::process::Command::new("bash");
    c.arg("-lc").arg(cmd);
    if !workdir.is_empty() { c.current_dir(workdir); }
    match tokio::time::timeout(std::time::Duration::from_secs(120), c.output()).await {
        Err(_) => json!({ "ok": false, "error": "timeout", "command": cmd }),
        Ok(Err(e)) => json!({ "ok": false, "error": e.to_string() }),
        Ok(Ok(out)) => json!({
            "ok": out.status.success(), "preview": "console", "command": cmd, "workdir": workdir,
            "stdout": String::from_utf8_lossy(&out.stdout), "stderr": String::from_utf8_lossy(&out.stderr),
            "output": format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
        }),
    }
}

async fn run_rscript(code: &str) -> Value {
    // Chanzo kimoja: injini moja ya R (r_engine) — hakuna Rscript ya pili.
    crate::r_engine::run_req("custom", &json!({ "r_code": code })).await
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/emulate", post(ide_emulate))
        .route("/api/ide/emulate/:kind", post(ide_emulate_kind))
        .route("/api/ide/emulate/sessions", get(ide_emulate_sessions))
}
