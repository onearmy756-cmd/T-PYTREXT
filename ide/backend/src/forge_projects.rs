//! forge_projects.rs — MIUNDO KAMILI ya web/mobile/iot/gaming (inayojijaribu).
//!
//! Tofauti na `forge_generators` (scaffold), hapa tunazalisha MIUNDO KAMILI
//! yenye majaribio yake (self-test): website (static site), mobile (PWA),
//! iot (Rust telemetry), gaming (JS game logic). Kila mradi una faili zote
//! na testi zinazoendeshwa kwa `node --test` au `cargo test`.

use crate::forge_domain::{to_snake, Domain};

/// Je, kind hii ni mradi kamili unaojijaribu?
pub fn is_project(kind: &str) -> bool {
    matches!(kind, "website" | "mobile" | "iot" | "gaming")
}

/// Amri ya kujaribu mradi (self-test).
pub fn test_command(kind: &str) -> &'static str {
    match kind {
        "iot" => "cargo test",
        _ => "node --test",
    }
}

/// Zalisha mradi kamili (files, note). `None` kama kind haijulikani.
pub fn generate_project(kind: &str, domain: &Domain) -> Option<(Vec<(String, String)>, String)> {
    let name = project_name(domain);
    match kind {
        "website" => Some(gen_website(&name)),
        "mobile" => Some(gen_mobile(&name)),
        "iot" => Some(gen_iot(&name)),
        "gaming" => Some(gen_gaming(&name)),
        _ => None,
    }
}

fn project_name(domain: &Domain) -> String {
    let n = to_snake(&domain.project);
    if n.is_empty() || n == "mradi" {
        "app".to_string()
    } else {
        n
    }
}

fn pkg_json(name: &str, extra: &str) -> String {
    format!(
        r##"{{
  "name": "{name}",
  "version": "1.0.0",
  "private": true,
  "type": "module",
  "scripts": {{
    "test": "node --test",
    "start": "npx --yes serve ."{extra}
  }}
}}
"##
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// WEBSITE — static site kamili (HTML + CSS + JS) + node --test
// ─────────────────────────────────────────────────────────────────────────────

fn gen_website(name: &str) -> (Vec<(String, String)>, String) {
    let files = vec![
        ("index.html".to_string(), website_html(name)),
        ("styles.css".to_string(), website_css()),
        ("app.js".to_string(), website_js()),
        ("site.test.js".to_string(), website_test()),
        ("package.json".to_string(), pkg_json(name, "")),
        ("README.md".to_string(), website_readme(name)),
    ];
    (files, "website (static site + node --test) production".to_string())
}

fn website_html(name: &str) -> String {
    r##"<!DOCTYPE html>
<html lang="sw">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>__PROJECT__</title>
  <meta name="description" content="__PROJECT__ — tovuti kamili yenye majaribio" />
  <link rel="stylesheet" href="styles.css" />
</head>
<body>
  <header class="site-header">
    <nav aria-label="main">
      <a class="brand" href="#hero">__PROJECT__</a>
      <ul class="menu">
        <li><a href="#features">Vipengele</a></li>
        <li><a href="#contact">Wasiliana</a></li>
      </ul>
    </nav>
  </header>

  <main>
    <section id="hero" class="hero">
      <h1>__PROJECT__</h1>
      <p>Tovuti kamili, yenye majaribio ya moja kwa moja.</p>
      <button id="cta" type="button">Anza</button>
    </section>

    <section id="features" class="features">
      <h2>Vipengele</h2>
      <ul id="feature-list"></ul>
    </section>

    <section id="contact" class="contact">
      <h2>Wasiliana</h2>
      <form id="contact-form">
        <label for="email">Barua pepe</label>
        <input id="email" name="email" type="email" required placeholder="jina@mfano.co" />
        <label for="message">Ujumbe</label>
        <textarea id="message" name="message" required rows="4"></textarea>
        <button type="submit">Tuma</button>
      </form>
      <p id="form-status" role="status"></p>
    </section>
  </main>

  <footer class="site-footer">
    <p>&copy; __PROJECT__</p>
  </footer>

  <script src="app.js" defer></script>
</body>
</html>
"##
    .replace("__PROJECT__", name)
}

fn website_css() -> String {
    r##":root {
  --bg: #0d1117;
  --fg: #e6edf3;
  --accent: #58a6ff;
  --muted: #8b949e;
}

* { box-sizing: border-box; }

body {
  margin: 0;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--fg);
  line-height: 1.6;
}

.site-header {
  position: sticky;
  top: 0;
  backdrop-filter: blur(8px);
  border-bottom: 1px solid #1f242c;
}

nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  max-width: 960px;
  margin: 0 auto;
  padding: 16px 24px;
}

.brand { color: var(--accent); font-weight: 700; text-decoration: none; }

.menu { display: flex; gap: 20px; list-style: none; margin: 0; padding: 0; }

.menu a { color: var(--fg); text-decoration: none; }

main { max-width: 960px; margin: 0 auto; padding: 32px 24px; }

.hero h1 { font-size: clamp(2rem, 6vw, 3.5rem); margin: 0 0 8px; }

button {
  background: var(--accent);
  color: #fff;
  border: 0;
  border-radius: 8px;
  padding: 12px 20px;
  font-size: 1rem;
  cursor: pointer;
}

form { display: grid; gap: 8px; max-width: 480px; }

input, textarea {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  color: var(--fg);
  padding: 10px;
}

.site-footer { border-top: 1px solid #1f242c; text-align: center; color: var(--muted); }

@media (max-width: 640px) {
  nav { flex-direction: column; gap: 12px; }
  .menu { gap: 12px; }
}
"##
    .to_string()
}

fn website_js() -> String {
    r##"// app.js — mwingiliano wa tovuti (bila framework).
const features = ["Kasi", "Usalama", "Ufikivu"];

function renderFeatures() {
  const list = document.getElementById("feature-list");
  if (!list) return;
  for (const f of features) {
    const li = document.createElement("li");
    li.textContent = f;
    list.appendChild(li);
  }
}

function wireForm() {
  const form = document.getElementById("contact-form");
  const status = document.getElementById("form-status");
  if (!form) return;
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    if (status) status.textContent = "Asante, tumepokea ujumbe.";
    form.reset();
  });
}

function wireCta() {
  const cta = document.getElementById("cta");
  const contact = document.getElementById("contact");
  if (!cta || !contact) return;
  cta.addEventListener("click", () => {
    contact.scrollIntoView({ behavior: "smooth" });
  });
}

renderFeatures();
wireForm();
wireCta();
"##
    .to_string()
}

fn website_test() -> String {
    r##"import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const dir = import.meta.dirname;
const read = (name) => readFileSync(join(dir, name), "utf8");

test("index.html ina muundo wa msingi", () => {
  const html = read("index.html");
  assert.match(html, /<!DOCTYPE html>/i);
  assert.match(html, /<title>/i);
  assert.match(html, /<nav/i);
  assert.match(html, /<main/i);
  assert.match(html, /<footer/i);
  assert.match(html, /styles\.css/);
  assert.match(html, /app\.js/);
});

test("styles.css ina media query", () => {
  const css = read("styles.css");
  assert.match(css, /@media/);
});

test("app.js ina handlers salama", () => {
  const js = read("app.js");
  assert.match(js, /addEventListener/);
  assert.match(js, /textContent/);
});
"##
    .to_string()
}

fn website_readme(name: &str) -> String {
    format!(
        r##"# {name} — tovuti kamili

Static site (HTML + CSS + JS), hakuna framework.

## Kukimbia

```bash
npm start        # huduma ya ndani (npx serve)
npm test         # node --test
```

## Faili

- `index.html` — muundo (header, main, footer)
- `styles.css` — mitindo + media queries
- `app.js` — mwingiliano (features, form, cta)
- `site.test.js` — majaribio
"##
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// MOBILE — PWA (installable) + node --test
// ─────────────────────────────────────────────────────────────────────────────

fn gen_mobile(name: &str) -> (Vec<(String, String)>, String) {
    let files = vec![
        ("index.html".to_string(), mobile_html(name)),
        ("styles.css".to_string(), mobile_css()),
        ("app.js".to_string(), mobile_js()),
        ("manifest.webmanifest".to_string(), mobile_manifest(name)),
        ("service-worker.js".to_string(), mobile_sw(name)),
        ("icon.svg".to_string(), mobile_icon(name)),
        ("pwa.test.js".to_string(), mobile_test()),
        ("package.json".to_string(), pkg_json(name, "")),
        ("README.md".to_string(), mobile_readme(name)),
    ];
    (files, "mobile (PWA + service worker + node --test) production".to_string())
}

fn mobile_html(name: &str) -> String {
    r##"<!DOCTYPE html>
<html lang="sw">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <meta name="theme-color" content="#0d1117" />
  <title>__PROJECT__</title>
  <link rel="manifest" href="manifest.webmanifest" />
  <link rel="stylesheet" href="styles.css" />
</head>
<body>
  <header class="app-bar">
    <h1>__PROJECT__</h1>
    <button id="theme" type="button" aria-label="Badilisha mandhari">Aa</button>
  </header>
  <main>
    <section class="card">
      <h2>Karibu</h2>
      <p>Programu ya simu (PWA) inayofanya kazi nje ya mtandao.</p>
    </section>
    <section class="card">
      <h2>Vipengele</h2>
      <ul id="features"></ul>
    </section>
  </main>
  <nav class="tab-bar">
    <a href="#home" class="active">Nyumbani</a>
    <a href="#settings">Mipangilio</a>
  </nav>
  <script src="app.js" defer></script>
</body>
</html>
"##
    .replace("__PROJECT__", name)
}

fn mobile_css() -> String {
    r##":root {
  --bg: #0d1117;
  --card: #161b22;
  --fg: #e6edf3;
  --accent: #007aff;
}

* { box-sizing: border-box; -webkit-tap-highlight-color: transparent; }

body {
  margin: 0;
  font-family: -apple-system, system-ui, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--fg);
  padding-bottom: 64px;
}

.app-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  padding-top: calc(14px + env(safe-area-inset-top));
  border-bottom: 1px solid #1f242c;
}

.app-bar h1 { font-size: 1.1rem; margin: 0; }

.card {
  background: var(--card);
  border-radius: 14px;
  padding: 16px;
  margin: 12px;
}

.tab-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  display: flex;
  justify-content: space-around;
  padding: 12px 0 calc(12px + env(safe-area-inset-bottom));
  background: #0b0f14;
  border-top: 1px solid #1f242c;
}

.tab-bar a { color: #8b949e; text-decoration: none; }
.tab-bar a.active { color: var(--accent); }

button {
  background: var(--accent);
  color: #fff;
  border: 0;
  border-radius: 10px;
  padding: 10px 14px;
}

@media (min-width: 700px) {
  main { max-width: 640px; margin: 0 auto; }
}
"##
    .to_string()
}

fn mobile_js() -> String {
    r##"// app.js — PWA: features, mandhari, na usajili wa service worker.
const features = ["Nje ya mtandao", "Kifungo cha nyumbani", "Arifa"];

function renderFeatures() {
  const list = document.getElementById("features");
  if (!list) return;
  for (const f of features) {
    const li = document.createElement("li");
    li.textContent = f;
    list.appendChild(li);
  }
}

function wireTheme() {
  const btn = document.getElementById("theme");
  if (!btn) return;
  btn.addEventListener("click", () => {
    document.body.classList.toggle("light");
  });
}

function registerServiceWorker() {
  if (!("serviceWorker" in navigator)) return;
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("service-worker.js").catch(() => {});
  });
}

renderFeatures();
wireTheme();
registerServiceWorker();
"##
    .to_string()
}

fn mobile_manifest(name: &str) -> String {
    format!(
        r##"{{
  "name": "{name}",
  "short_name": "{name}",
  "start_url": "./index.html",
  "scope": "./",
  "display": "standalone",
  "background_color": "#0d1117",
  "theme_color": "#0d1117",
  "icons": [
    {{ "src": "icon.svg", "sizes": "any", "type": "image/svg+xml", "purpose": "any" }}
  ]
}}
"##
    )
}

fn mobile_sw(name: &str) -> String {
    format!(
        r##"// service-worker.js — cache ya PWA kwa matumizi nje ya mtandao.
const CACHE = "{name}-v1";
const ASSETS = ["./", "./index.html", "./styles.css", "./app.js", "./manifest.webmanifest"];

self.addEventListener("install", (event) => {{
  event.waitUntil(caches.open(CACHE).then((c) => c.addAll(ASSETS)));
  self.skipWaiting();
}});

self.addEventListener("activate", (event) => {{
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))
    )
  );
  self.clients.claim();
}});

self.addEventListener("fetch", (event) => {{
  event.respondWith(
    caches.match(event.request).then((hit) => hit || fetch(event.request))
  );
}});
"##
    )
}

fn mobile_icon(name: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
  <rect width="128" height="128" rx="24" fill="#0d1117" />
  <circle cx="64" cy="64" r="34" fill="none" stroke="#007aff" stroke-width="8" />
  <text x="64" y="74" font-family="system-ui" font-size="34" fill="#e6edf3" text-anchor="middle">{initial}</text>
  <title>{name}</title>
</svg>
"##,
        initial = name.chars().next().unwrap_or('A').to_uppercase(),
    )
}

fn mobile_test() -> String {
    r##"import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const dir = import.meta.dirname;
const read = (name) => readFileSync(join(dir, name), "utf8");

test("manifest ni JSON halali yenye fields", () => {
  const manifest = JSON.parse(read("manifest.webmanifest"));
  assert.equal(typeof manifest.name, "string");
  assert.equal(manifest.display, "standalone");
  assert.ok(Array.isArray(manifest.icons));
  assert.ok(manifest.icons.length > 0);
});

test("index.html ina viewport na manifest link", () => {
  const html = read("index.html");
  assert.match(html, /viewport/);
  assert.match(html, /manifest\.webmanifest/);
});

test("service-worker ina install/activate/fetch", () => {
  const sw = read("service-worker.js");
  assert.match(sw, /addEventListener\("install"/);
  assert.match(sw, /addEventListener\("activate"/);
  assert.match(sw, /addEventListener\("fetch"/);
});
"##
    .to_string()
}

fn mobile_readme(name: &str) -> String {
    format!(
        r##"# {name} — programu ya simu (PWA)

Progressive Web App inayoweza kusakinishwa na kufanya kazi nje ya mtandao.

## Kukimbia

```bash
npm start        # huduma ya ndani
npm test         # node --test
```

## Faili

- `index.html` — UI ya simu (app bar, cards, tab bar)
- `manifest.webmanifest` — metadata ya usakinishaji
- `service-worker.js` — cache ya nje ya mtandao
- `app.js` — mwingiliano + usajili wa SW
- `pwa.test.js` — majaribio
"##
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// IOT — Rust telemetry (deterministic) + cargo test
// ─────────────────────────────────────────────────────────────────────────────

fn gen_iot(name: &str) -> (Vec<(String, String)>, String) {
    let files = vec![
        ("Cargo.toml".to_string(), iot_cargo(name)),
        ("src/telemetry.rs".to_string(), iot_telemetry()),
        ("src/analytics.rs".to_string(), iot_analytics()),
        ("src/main.rs".to_string(), iot_main()),
        ("README.md".to_string(), iot_readme(name)),
    ];
    (
        files,
        "iot (telemetry simulator + analytics + cargo test) production".to_string(),
    )
}

fn iot_cargo(name: &str) -> String {
    format!(
        r##"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"
"##
    )
}

fn iot_telemetry() -> String {
    r##"//! Simulator ya sensor ya IoT (deterministic) + ugunduzi wa hitilafu.

/// Kipimo kimoja cha sensor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub t: usize,
    pub temperature: f64,
    pub humidity: f64,
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Zalisha mfululizo wa joto + unyevu (deterministic kwa seed).
pub fn simulate(points: usize, seed: u32) -> Vec<Reading> {
    let mut state = (seed | 1) & 0x7FFF_FFFF;
    let mut next = || {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7FFF_FFFF;
        state as f64 / 0x7FFF_FFFFu32 as f64
    };
    let mut out = Vec::with_capacity(points);
    for i in 0..points {
        let noise = next() - 0.5;
        let phase = i as f64 / 6.0;
        let temp = 22.0 + 3.0 * phase.sin() + noise * 0.8;
        let hum = (55.0 + 8.0 * (phase / 2.0).cos() + noise * 2.4).clamp(0.0, 100.0);
        out.push(Reading {
            t: i,
            temperature: round2(temp),
            humidity: round2(hum),
        });
    }
    out
}

/// Weka hitilafu moja ya joto kwenye index `at`.
pub fn inject_anomaly(series: &[Reading], at: usize, delta: f64) -> Vec<Reading> {
    series
        .iter()
        .map(|r| {
            if r.t == at {
                Reading {
                    t: r.t,
                    temperature: round2(r.temperature + delta),
                    humidity: r.humidity,
                }
            } else {
                *r
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_deterministic() {
        assert_eq!(simulate(48, 42), simulate(48, 42));
        assert_eq!(simulate(48, 42).len(), 48);
    }

    #[test]
    fn simulate_seed_tofauti() {
        assert_ne!(simulate(12, 1), simulate(12, 2));
    }

    #[test]
    fn unyevu_ndani_ya_mipaka() {
        for r in simulate(48, 7) {
            assert!((0.0..=100.0).contains(&r.humidity));
        }
    }

    #[test]
    fn inject_anomaly_inaongeza_joto() {
        let s = simulate(48, 42);
        let a = inject_anomaly(&s, 17, 9.0);
        assert!(a[17].temperature > s[17].temperature);
    }
}
"##
    .to_string()
}

fn iot_analytics() -> String {
    r##"//! Uchambuzi wa mfululizo wa IoT: wastani, mkengeuko, hitilafu.

use crate::telemetry::Reading;

/// Muhtasari wa takwimu kwa mfululizo wa namba.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

/// Muhtasari wa takwimu kwa mfululizo wa namba.
pub fn summarize(values: &[f64]) -> Summary {
    if values.is_empty() {
        return Summary {
            count: 0,
            mean: 0.0,
            std: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Summary {
        count: values.len(),
        mean: round4(mean),
        std: round4(var.sqrt()),
        min: round4(min),
        max: round4(max),
    }
}

/// Rejesha `t` za vipimo vilivyo mbali na wastani kwa z-mkengeuko >= z.
pub fn detect_anomalies(series: &[Reading], z: f64) -> Vec<usize> {
    let temps: Vec<f64> = series.iter().map(|r| r.temperature).collect();
    if temps.len() < 2 {
        return Vec::new();
    }
    let m = temps.iter().sum::<f64>() / temps.len() as f64;
    let var = temps.iter().map(|v| (v - m).powi(2)).sum::<f64>() / temps.len() as f64;
    let s = var.sqrt();
    if s == 0.0 {
        return Vec::new();
    }
    series
        .iter()
        .filter(|r| (r.temperature - m).abs() / s >= z)
        .map(|r| r.t)
        .collect()
}

/// Takwimu za joto pekee.
pub fn temperature_stats(series: &[Reading]) -> Summary {
    let temps: Vec<f64> = series.iter().map(|r| r.temperature).collect();
    summarize(&temps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{inject_anomaly, simulate};

    #[test]
    fn detect_anomalies_inapata_hitilafu() {
        let s = inject_anomaly(&simulate(48, 42), 17, 9.0);
        assert!(detect_anomalies(&s, 3.0).contains(&17));
    }

    #[test]
    fn summarize_sahihi() {
        let out = summarize(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(out.count, 4);
        assert_eq!(out.mean, 2.5);
        assert_eq!(out.min, 1.0);
        assert_eq!(out.max, 4.0);
    }

    #[test]
    fn summarize_tupu() {
        let out = summarize(&[]);
        assert_eq!(out.count, 0);
        assert_eq!(out.mean, 0.0);
    }

    #[test]
    fn temperature_stats_ina_fields() {
        let out = temperature_stats(&simulate(48, 42));
        assert_eq!(out.count, 48);
        assert!(out.std >= 0.0);
    }
}
"##
    .to_string()
}

fn iot_main() -> String {
    r##"//! Demo: simulator + uchambuzi wa telemetry.

mod analytics;
mod telemetry;

use analytics::{detect_anomalies, temperature_stats};
use telemetry::{inject_anomaly, simulate};

fn main() {
    let series = inject_anomaly(&simulate(48, 42), 17, 9.0);
    let stats = temperature_stats(&series);
    println!("vipimo: {}", stats.count);
    println!("wastani: {:.2}", stats.mean);
    println!("mkengeuko: {:.2}", stats.std);
    println!("hitilafu: {:?}", detect_anomalies(&series, 3.0));
}
"##
    .to_string()
}

fn iot_readme(name: &str) -> String {
    format!(
        r##"# {name} — IoT telemetry

Simulator ya sensor (deterministic) + uchambuzi wa takwimu + ugunduzi wa hitilafu.

## Kukimbia

```bash
cargo run
cargo test
```

## Faili

- `src/telemetry.rs` — simulate() + inject_anomaly()
- `src/analytics.rs` — summarize() + detect_anomalies()
- `src/main.rs` — demo
"##
    )
}



// ─────────────────────────────────────────────────────────────────────────────
// GAMING — JS game (pure logic + canvas) + node --test
// ─────────────────────────────────────────────────────────────────────────────

fn gen_gaming(name: &str) -> (Vec<(String, String)>, String) {
    let files = vec![
        ("index.html".to_string(), gaming_html(name)),
        ("styles.css".to_string(), gaming_css()),
        ("logic.js".to_string(), gaming_logic()),
        ("main.js".to_string(), gaming_main()),
        ("game.test.js".to_string(), gaming_test()),
        ("package.json".to_string(), pkg_json(name, "")),
        ("README.md".to_string(), gaming_readme(name)),
    ];
    (files, "gaming (pure game logic + canvas + node --test) production".to_string())
}

fn gaming_html(name: &str) -> String {
    r##"<!DOCTYPE html>
<html lang="sw">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>__PROJECT__</title>
  <link rel="stylesheet" href="styles.css" />
</head>
<body>
  <div id="hud">
    <span>Alama: <b id="score">0</b></span>
    <span>Muda: <b id="time">0</b>s</span>
  </div>
  <canvas id="game" width="720" height="480"></canvas>
  <p id="hint">Tumia mishale / WASD kusogeza. Kukusanya alama.</p>
  <script type="module" src="main.js"></script>
</body>
</html>
"##
    .replace("__PROJECT__", name)
}

fn gaming_css() -> String {
    r##"html, body { margin: 0; height: 100%; background: #05070d; overflow: hidden; }

body {
  font-family: ui-monospace, monospace;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #9fb2c8;
}

#hud {
  display: flex;
  gap: 24px;
  font-size: 14px;
  margin-bottom: 10px;
}

#hud b { color: #58a6ff; }

canvas {
  background: #0a0e16;
  border: 1px solid #1f6feb;
  border-radius: 10px;
  max-width: 96vw;
  touch-action: none;
}

#hint { color: #6b7a8d; font-size: 12px; }
"##
    .to_string()
}

fn gaming_logic() -> String {
    r##"// logic.js — mantiki safi ya mchezo (haina DOM; inajaribiwa kwa node --test).
export function clamp(value, lo, hi) {
  return Math.min(hi, Math.max(lo, value));
}

export class Game {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.player = { x: width / 2, y: height / 2, r: 12 };
    this.target = { x: width * 0.75, y: height * 0.5, r: 10 };
    this.score = 0;
    this.seed = 1;
  }

  movePlayer(dx, dy) {
    this.player.x = clamp(this.player.x + dx, this.player.r, this.width - this.player.r);
    this.player.y = clamp(this.player.y + dy, this.player.r, this.height - this.player.r);
    return this.player;
  }

  nextSeed() {
    this.seed = (this.seed * 1103515245 + 12345) & 0x7fffffff;
    return this.seed;
  }

  spawnTarget() {
    const s = this.nextSeed();
    const maxX = Math.max(1, Math.floor(this.width - 40));
    const maxY = Math.max(1, Math.floor(this.height - 40));
    this.target.x = 20 + (s % maxX);
    this.target.y = 20 + ((s >> 8) % maxY);
    return this.target;
  }

  collides() {
    const dx = this.player.x - this.target.x;
    const dy = this.player.y - this.target.y;
    return Math.hypot(dx, dy) <= this.player.r + this.target.r;
  }

  collect() {
    if (this.collides()) {
      this.score += 1;
      this.spawnTarget();
      return true;
    }
    return false;
  }
}
"##
    .to_string()
}

fn gaming_main() -> String {
    r##"// main.js — render loop (canvas) inayotumia mantiki ya logic.js.
import { Game } from "./logic.js";

const canvas = document.getElementById("game");
const ctx = canvas.getContext("2d");
const scoreEl = document.getElementById("score");
const timeEl = document.getElementById("time");

const game = new Game(canvas.width, canvas.height);
const keys = new Set();
const started = performance.now();

addEventListener("keydown", (e) => keys.add(e.key.toLowerCase()));
addEventListener("keyup", (e) => keys.delete(e.key.toLowerCase()));

function step() {
  const speed = 5;
  let dx = 0;
  let dy = 0;
  if (keys.has("arrowleft") || keys.has("a")) dx -= speed;
  if (keys.has("arrowright") || keys.has("d")) dx += speed;
  if (keys.has("arrowup") || keys.has("w")) dy -= speed;
  if (keys.has("arrowdown") || keys.has("s")) dy += speed;
  if (dx !== 0 || dy !== 0) game.movePlayer(dx, dy);
  game.collect();

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = "#58a6ff";
  ctx.beginPath();
  ctx.arc(game.player.x, game.player.y, game.player.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#d29922";
  ctx.beginPath();
  ctx.arc(game.target.x, game.target.y, game.target.r, 0, Math.PI * 2);
  ctx.fill();

  if (scoreEl) scoreEl.textContent = String(game.score);
  if (timeEl) timeEl.textContent = String(Math.floor((performance.now() - started) / 1000));
  requestAnimationFrame(step);
}

step();
"##
    .to_string()
}

fn gaming_test() -> String {
    r##"import { test } from "node:test";
import assert from "node:assert/strict";
import { Game, clamp } from "./logic.js";

test("clamp inaweka mipaka", () => {
  assert.equal(clamp(5, 0, 10), 5);
  assert.equal(clamp(-1, 0, 10), 0);
  assert.equal(clamp(99, 0, 10), 10);
});

test("movePlayer haitoki nje ya uwanja", () => {
  const g = new Game(100, 100);
  g.movePlayer(-9999, -9999);
  assert.equal(g.player.x, g.player.r);
  assert.equal(g.player.y, g.player.r);
});

test("collect inaongeza alama", () => {
  const g = new Game(100, 100);
  g.target = { x: g.player.x, y: g.player.y, r: 10 };
  assert.equal(g.collect(), true);
  assert.equal(g.score, 1);
});

test("spawnTarget ni deterministic", () => {
  const a = new Game(100, 100);
  const b = new Game(100, 100);
  assert.deepEqual(a.spawnTarget(), b.spawnTarget());
});
"##
    .to_string()
}

fn gaming_readme(name: &str) -> String {
    format!(
        r##"# {name} — mchezo (canvas)

Mchezo wa 2D: mantiki safi (`logic.js`) + render loop (`main.js`).

## Kukimbia

```bash
npm start        # huduma ya ndani
npm test         # node --test
```

## Faili

- `logic.js` — mantiki safi (Game, clamp) inayojaribiwa
- `main.js` — render loop (canvas)
- `game.test.js` — majaribio
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge_domain::parse_domain;

    fn write_project(files: &[(String, String)], dir: &std::path::Path) {
        for (path, content) in files {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("mkdir");
            }
            std::fs::write(&full, content).expect("write");
        }
    }

    #[test]
    fn kila_kind_inazalisha_nyaraka_zake() {
        let domain = parse_domain("mradi wa mfano");
        let expect: &[(&str, &[&str])] = &[
            ("website", &["index.html", "styles.css", "app.js", "site.test.js", "package.json"]),
            ("mobile", &["index.html", "manifest.webmanifest", "service-worker.js", "pwa.test.js"]),
            ("iot", &["Cargo.toml", "src/telemetry.rs", "src/analytics.rs", "src/main.rs"]),
            ("gaming", &["index.html", "logic.js", "main.js", "game.test.js", "package.json"]),
        ];
        for (kind, wants) in expect {
            let (files, note) = generate_project(kind, &domain).expect("generator");
            assert!(note.contains("production"), "{kind}: note");
            let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
            for w in *wants {
                assert!(paths.contains(w), "{kind}: haipo {w}");
            }
        }
    }

    #[test]
    fn project_hupita_linter_ya_repo() {
        let domain = parse_domain("mradi wa mfano");
        let mut violations = 0;
        for kind in ["website", "mobile", "iot", "gaming"] {
            let (files, _) = generate_project(kind, &domain).expect("generator");
            for (p, c) in &files {
                let lang = crate::code_audit::lang_for(p);
                let v = crate::code_audit::analyze_content(lang, c);
                if !v.is_empty() {
                    violations += v.len();
                    println!("VIOLATION kind={kind} path={p} lang={lang} -> {v:?}");
                }
            }
        }
        assert_eq!(violations, 0, "miundo inavunja kanuni za repo");
    }

    /// Uthibitisho HALISI: kila mradi unapita self-test yake.
    #[test]
    #[ignore = "inahitaji node/cargo; endesha kwa --ignored"]
    fn project_unapita_self_test() {
        let domain = parse_domain("mradi wa mfano");
        for kind in ["website", "mobile", "gaming", "iot"] {
            let (files, _) = generate_project(kind, &domain).expect("generator");
            let dir = std::env::temp_dir().join(format!("forge_proj_{}_{}", kind, std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            write_project(&files, &dir);
            let cmd = test_command(kind);
            let out = std::process::Command::new("bash")
                .arg("-lc")
                .arg(cmd)
                .current_dir(&dir)
                .output()
                .expect("bash haipo");
            println!(
                "== {kind} ({cmd}) ==\n{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(out.status.success(), "{kind}: self-test imeshindwa");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
