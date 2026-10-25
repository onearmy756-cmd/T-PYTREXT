//! shadow.rs — SHADOW COMPILER (kodi → UI HALISI, haraka upande wa Rust).
//!
//! Mwanafunzi/mwalimu anaandika Monaco; kodi inapasuliwa kuwa UI halisi:
//!   - HTML: passthrough (raw HTML iko iframe sandboxed kwenye browser)
//!   - CSS: fungwa ndani ya demo page (rangi/baada zinaona live)
//!   - JS: fungwa ndani ya harness yenye console-capture + onerror display
//!
//! Njia MBILI (zote HALISI, hakuna uongo):
//!   - HARAKA (browser-local, ≤10ms): `static/ide-shadow.js` ina wrap inline —
//!     hakuna mtandao; latensi inapimwa performance.now().
//!   - PARITY (Rust, endpoint hii): `POST /api/ide/shadow {path, content}`
//!     — server inafunga kwa Rust (kwa wateja ambao wanataka server-side,
//!     au Tauri desktop). Matokeo ni JSON {ok, kind, html}.
//! Kanuni: hakuna execution ya kodi server-side (iframe sandbox = browser);
//! security: sandbox="allow-scripts" pekee (no same-origin kwa JS wrapper).

use axum::Json;
use serde_json::{json, Value};

/// Aina ya UI kutoka extension.
pub fn kind_of(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "html" | "htm" => "html",
        "css" => "css",
        "js" | "jsx" | "mjs" | "cjs" => "js",
        _ => "other",
    }
}

/// HTML passthrough — kodi ileile ni UI ileile.
pub fn html_doc(content: &str) -> Value {
    json!({ "ok": true, "kind": "html", "html": content })
}

/// CSS → demo page HALISI (rangi/layout zinaonekana live).
pub fn css_doc(content: &str) -> Value {
    let html = format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>{content}</style></head>
<body><div class="box head" data-demo="heading">Heading 1</div>
<div class="box para" data-demo="paragraph">Para 1 — maandishi mfano</div>
<button class="btn" data-demo="button">Button</button>
<a class="link" data-demo="link">Link</a>
<ul><li class="item" data-demo="list">Item 1</li><li class="item">Item 2</li></ul></body></html>"#
    );
    json!({ "ok": true, "kind": "css", "html": html })
}

/// JS → harness yenye console-capture + error display (sandboxed iframe).
pub fn js_doc(content: &str) -> Value {
    let esc = content.replace("</script>", "<\\/script>");
    let html = format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>body{{font-family:system-ui;background:#0d1117;color:#e6edf3;padding:12px}}#console{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:8px;font:12px monospace;max-height:220px;overflow:auto}}.err{{color:#f85149}}</style></head>
<body><h3 style="margin:0 0 8px">JS Harness (sandbox)</h3><div id="deck"></div><pre id="console"></pre>
<script>
var out=document.getElementById('console');
function log(s,cls){{var d=document.createElement('div');if(cls)d.className=cls;d.textContent=s;out.appendChild(d);}}
['log','info','warn','error'].forEach(function(k){{var o=console[k];console[k]=function(){{log('[console.'+k+'] '+Array.prototype.map.call(arguments,String).join(' '),k==='error'?'err':'');o.apply(console,arguments);}};}});
window.onerror=function(m,s,l,c,e){{log('[onerror] '+m+' @'+l+':'+c,'err');}};
try {{
(function(){esc}
)();
}} catch(e) {{ log('[catch] '+e.message,'err'); }}
log('shadow: kodi imetekelezwa sandbox. ✓');
</script></body></html>"#
    );
    json!({ "ok": true, "kind": "js", "html": html })
}

/// Lugha isiyotafsirika live (e.g. rust/py) — jibu la uongo HAPANA.
pub fn other_doc(path: &str) -> Value {
    json!({
        "ok": false,
        "kind": "other",
        "error": format!("shadow: '{path}' si lugha ya UI live — tumia Emulator (kind=rust/r) au Terminal"),
    })
}

/// SHADOW COMPILER kuu: path + kodi → UI live doc.
pub fn shadow_doc(path: &str, content: &str) -> Value {
    match kind_of(path) {
        "html" => html_doc(content),
        "css" => css_doc(content),
        "js" => js_doc(content),
        _ => other_doc(path),
    }
}

// ---------------------------------------------------------------------------
// HTTP endpoint — POST /api/ide/shadow {path, content}
// Parity ya HARAKA (browser-local): server inafunga kwa Rust — matokeo JSON
// {ok, kind, html}; browser inaweka iframe sandboxed. Hakuna execution ya
// kodi kwa server (usalama: sandbox iko upande wa browser tu).
// ---------------------------------------------------------------------------

pub async fn api(Json(r): Json<Value>) -> Json<Value> {
    let path = r["path"].as_str().unwrap_or("").trim().to_string();
    let content = r["content"].as_str().unwrap_or("");
    Json(shadow_doc(&path, content))
}

// ---------------------------------------------------------------------------
// Tests — mantiki safi (deterministic)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_passthrough_kodi_ileile() {
        let code = "<h1>Halo</h1>";
        let v = html_doc(code);
        assert_eq!(v["kind"], "html");
        assert_eq!(v["html"], code);
    }

    #[test]
    fn css_harness_inaufunga_maudhui() {
        let v = css_doc("body { background: #123; }");
        let html = v["html"].as_str().unwrap();
        assert!(html.contains("body { background: #123; }"));
        assert!(html.contains("<style>"));
        assert!(html.contains("data-demo"));
    }

    #[test]
    fn js_harness_inalinda_script_close() {
        // "</script>" ndani ya kodi ya mtumii inaweza kuvunja harness —
        // lazima iwe escaped (script injection guard).
        let v = js_doc("var s = \"</script>\";");
        let html = v["html"].as_str().unwrap();
        assert!(html.contains("<\\/script>"), "script-close lazima iwe escaped");
        assert!(html.contains("[console."), "console-capture iko");
        assert!(html.contains("onerror"), "error display iko");
    }

    #[test]
    fn kind_of_lugha_mawa() {
        assert_eq!(kind_of("ui.html"), "html");
        assert_eq!(kind_of("style.CSS"), "css");
        assert_eq!(kind_of("app.js"), "js");
        assert_eq!(kind_of("main.rs"), "other");
    }

    #[test]
    fn shadow_dispatcher_lugha_isingara() {
        let v = shadow_doc("main.rs", "fn main() {}");
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap().contains("Emulator"));
        let h = shadow_doc("page.html", "<b>x</b>");
        assert_eq!(h["kind"], "html");
    }
}
