//! studio.rs — ukurasa mmoja unaoonyesha UI zote mbili za MTECHOS.
//!
//! MTECHOS IDE (Canvas ya `/ide`, origin hii) na OpenHands (Agent Canvas,
//! container yenye origin yake) zinaendesha kama wasimamizi wawili huru.
//! Ukurasa huu huziunganisha kwenye skrini moja kwa `iframe` mbili ili
//! mtumiaji aone zote bila kubadilisha kichupo cha browser.
//!
//! Kwa nini iframe: huduma zote mbili hazitoi `X-Frame-Options` wala CSP
//! `frame-ancestors`, hivyo zinaweza kupachikwa. OpenHands huweka session key
//! yake kwenye `localStorage` ya origin YAKE, hivyo iframe inafanya kazi kama
//! ukurasa wake mwenyewe — hakuna proxy ya WebSocket inayohitajika.
//!
//! Base ya OpenHands hupatuliwa kwa `OPENHANDS_URL`; ikiwa haipo, hubainishwa
//! kutoka host ya sasa (`work-1`→`work-2`, porta `12000`→`12001`) upande wa
//! browser, ili sandbox na localhost zote zifanye kazi.

use axum::response::Html;
use axum::routing::get;
use axum::Router;

use crate::AppState;

/// URL ya OpenHands kama ilivyowekwa kwenye mazingira (server-side).
pub fn openhands_url() -> Option<String> {
    std::env::var("OPENHANDS_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
}

/// `/studio` — skrini moja yenye UI zote mbili.
async fn studio() -> Html<String> {
    Html(page(openhands_url().as_deref()))
}

/// HTML ya ukurasa. `openhands` ikiwa `Some`, hutumika kama base MOJA KWA MOJA;
/// vinginevyo `null` na browser hubainisha host yake.
fn page(openhands: Option<&str>) -> String {
    let oh_literal = match openhands {
        Some(u) => serde_json::Value::String(u.to_string()).to_string(),
        None => "null".to_string(),
    };
    TEMPLATE.replace("__OPENHANDS_URL__", &oh_literal)
}

/// Routes za studio (merge kwenye router kuu).
pub fn router() -> Router<AppState> {
    Router::new().route("/studio", get(studio))
}

const TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="sw">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>MTECHOS · Studio — UI zote mbili</title>
<style>
  :root {
    --bg: #130A07; --panel: #1E0F0A; --line: #472415;
    --fg: #FFF3EA; --muted: #B85C2C; --accent: #FF6B2C; --accent-fg: #0A0503;
  }
  * { box-sizing: border-box; }
  html, body { height: 100%; }
  body {
    margin: 0; background: var(--bg); color: var(--fg);
    font: 14px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif;
    display: flex; flex-direction: column;
  }
  header {
    display: flex; align-items: center; gap: 12px;
    padding: 8px 14px; background: var(--panel);
    border-bottom: 1px solid var(--line); flex: none;
  }
  .brand { font-weight: 700; letter-spacing: .3px; }
  .brand span { color: var(--accent); }
  nav { display: flex; gap: 6px; margin-left: 8px; }
  nav button {
    background: transparent; color: var(--fg); border: 1px solid var(--line);
    border-radius: 6px; padding: 5px 12px; cursor: pointer; font: inherit;
  }
  nav button:hover { border-color: var(--accent); }
  nav button[aria-pressed="true"] {
    background: var(--accent); color: var(--accent-fg); border-color: var(--accent);
  }
  .spacer { flex: 1; }
  .hint { color: var(--muted); font-size: 12px; }
  main { flex: 1; display: flex; min-height: 0; gap: 1px; background: var(--line); }
  section { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg); }
  section > .cap {
    flex: none; padding: 4px 10px; font-size: 12px; color: var(--muted);
    background: var(--panel); border-bottom: 1px solid var(--line);
  }
  iframe { flex: 1; width: 100%; border: 0; background: var(--bg); }
  /* Hali za kuonyesha: moja pekee au zote mbili. */
  body[data-view="mt"] #pane-oh,
  body[data-view="oh"] #pane-mt { display: none; }
</style>
</head>
<body data-view="both">
  <header>
    <div class="brand">MTECHOS <span>·</span> Studio</div>
    <nav>
      <button type="button" data-view="mt">MTECHOS IDE</button>
      <button type="button" data-view="oh">OpenHands</button>
      <button type="button" data-view="both" aria-pressed="true">Zote mbili</button>
    </nav>
    <div class="spacer"></div>
    <span class="hint" id="oh-hint"></span>
  </header>
  <main>
    <section id="pane-mt">
      <div class="cap">MTECHOS IDE — /ide/elimu</div>
      <iframe id="frame-mt" src="/ide/elimu" title="MTECHOS IDE"></iframe>
    </section>
    <section id="pane-oh">
      <div class="cap">OpenHands — Agent Canvas</div>
      <iframe id="frame-oh" title="OpenHands"></iframe>
    </section>
  </main>
  <script>
    const CONFIGURED = __OPENHANDS_URL__;

    // Base ya OpenHands: env ikiwa imewekwa; vinginevyo kanuni ya host ya sasa.
    function openhandsBase() {
      if (CONFIGURED) return CONFIGURED.replace(/\/+$/, "") + "/canvas/";
      const { protocol, hostname, port } = location;
      if (hostname.includes("work-1")) {
        return protocol + "//" + hostname.replace("work-1", "work-2") + "/canvas/";
      }
      const p = port === "12000" ? "12001" : (port || "12001");
      return protocol + "//" + hostname + ":" + p + "/canvas/";
    }

    const base = openhandsBase();
    document.getElementById("frame-oh").src = base;
    document.getElementById("oh-hint").textContent = base;

    // Vitendo vya kuonyesha.
    const buttons = [...document.querySelectorAll("nav button")];
    function setView(view) {
      document.body.dataset.view = view;
      for (const b of buttons) {
        b.setAttribute("aria-pressed", String(b.dataset.view === view));
      }
    }
    for (const b of buttons) {
      b.addEventListener("click", () => setView(b.dataset.view));
    }
    // `?view=mt|oh|both` huchagua hali mara moja (deep-link).
    const wanted = new URLSearchParams(location.search).get("view");
    if (["mt", "oh", "both"].includes(wanted)) setView(wanted);
  </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ukurasa_una_ui_zote_mbili() {
        let html = page(None);
        assert!(html.contains(r#"src="/ide/elimu""#), "iframe ya MTECHOS ipo");
        assert!(html.contains(r#"id="frame-oh""#), "iframe ya OpenHands ipo");
        assert!(html.contains("window") || html.contains("location"), "JS ipo");
    }

    #[test]
    fn openhands_url_imewekwa_kama_literal() {
        let html = page(Some("https://mfano.test"));
        assert!(html.contains(r#""https://mfano.test""#));
        assert!(!html.contains("__OPENHANDS_URL__"));
    }

    #[test]
    fn openhands_url_isipowekwa_ni_null() {
        let html = page(None);
        assert!(html.contains("const CONFIGURED = null;"));
    }
}
