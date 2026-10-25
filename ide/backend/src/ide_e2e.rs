//! ide_e2e.rs — E2E HALISI (Playwright) kwa browser na app.
//!
//! Hutoa:
//!   status  — kama Node + Playwright zipo
//!   spec    — hutengeneza Playwright test (smoke: status, title, console errors)
//!   run     — huendesha `npx playwright test` HALISI na kurudisha JSON report
//!
//! Kama Playwright haipo, hurudisha `available:false` + amri ya kuweka.

use axum::Json;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

use crate::ide_api::ide_workspace_root;

async fn has(bin: &str) -> bool {
    let out = tokio::process::Command::new("bash").arg("-lc").arg(format!("command -v {bin}")).output().await;
    matches!(out, Ok(o) if o.status.success())
}

async fn node() -> bool { has("node").await || has("npx").await }

/// Playwright ipo? (browser binaries + package). Huhakikisha run itafanya kazi.
async fn playwright_ready() -> bool {
    let out = tokio::process::Command::new("bash")
        .arg("-lc")
        .arg("npx --yes playwright --version")
        .output()
        .await;
    matches!(out, Ok(o) if o.status.success())
}

/// GET /api/ide/e2e/status
pub async fn status() -> Json<Value> {
    let n = node().await;
    let pw = if n { playwright_ready().await } else { false };
    Json(json!({
        "ok": true,
        "node": n,
        "playwright": pw,
        "ready": n && pw,
        "install": "npm i -D @playwright/test && npx playwright install --with-deps chromium",
        "note": if n && pw { "Node + Playwright vipo — E2E inaweza kuendeshwa." }
            else if n { "Node ipo lakini Playwright haipo — sakinisha kwa amri ya install." }
            else { "Node haipo — sakinisha Node.js kwanza." },
    }))
}

fn spec_for(name: &str, url: &str) -> String {
    format!(
        r#"// Auto-generated na MBILINYITECH IDE — E2E smoke test ({name})
// Endesha: npx playwright test {name}.spec.js
const {{ test, expect }} = require('@playwright/test');

test('{name}: smoke', async ({{ page }}) => {{
  const errors = [];
  page.on('console', m => {{ if (m.type() === 'error') errors.push(m.text()); }});
  page.on('pageerror', e => errors.push(String(e)));

  const resp = await page.goto('{url}', {{ waitUntil: 'load', timeout: 30000 }});
  expect(resp, 'response').toBeTruthy();
  expect(resp.status(), 'status < 400').toBeLessThan(400);

  const title = await page.title();
  expect(title.length).toBeGreaterThan(0);

  // UI ipo hai
  expect(await page.locator('body').count()).toBeGreaterThan(0);

  // hakuna console error
  expect(errors, 'console errors: ' + errors.join(' | ')).toHaveLength(0);
}});
"#
    )
}

/// POST /api/ide/e2e/spec { name, url } → huandika e2e/<name>.spec.js
pub async fn spec(Json(body): Json<Value>) -> Json<Value> {
    let name = body["name"].as_str().unwrap_or("smoke");
    let url = body["url"].as_str().unwrap_or("http://127.0.0.1:8080/");
    let content = spec_for(name, url);
    let dir = ide_workspace_root().join("e2e");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join(format!("{name}.spec.js"));
    let written = std::fs::write(&file, &content).is_ok();
    Json(json!({
        "ok": written,
        "file": file.display().to_string(),
        "spec": content,
        "url": url,
    }))
}

/// POST /api/ide/e2e/run { name, url?, install? }
pub async fn run(Json(body): Json<Value>) -> Json<Value> {
    if !node().await {
        return Json(json!({ "ok": false, "error": "Node/npx haipo", "install": "sakinisha Node.js" }));
    }
    let name = body["name"].as_str().unwrap_or("smoke").to_string();
    let url = body["url"].as_str().unwrap_or("http://127.0.0.1:8080/").to_string();
    let do_install = body["install"].as_bool().unwrap_or(false);
    let root = ide_workspace_root();

    // Hakikisha spec ipo
    let spec_file = root.join("e2e").join(format!("{name}.spec.js"));
    if !spec_file.exists() {
        let _ = spec(Json(json!({ "name": name, "url": url }))).await;
    }

    // package.json ya eneo la E2E — ili `require('@playwright/test')` itatuliwe
    // ndani ya e2e/ (npx -p pekee haitoshi: require hutafuta node_modules ya hapa).
    let e2e_dir = root.join("e2e");
    let pkg = e2e_dir.join("package.json");
    if !pkg.exists() {
        let _ = std::fs::write(
            &pkg,
            r#"{
  "name": "mtechos-e2e",
  "private": true,
  "version": "1.0.0",
  "devDependencies": { "@playwright/test": "^1.48.0" }
}
"#,
        );
    }
    // config inayosoma e2e/
    let cfg = e2e_dir.join("playwright.config.js");
    if !cfg.exists() {
        let _ = std::fs::write(&cfg, r#"module.exports = {
  testDir: __dirname,
  reporter: [['json', { outputFile: 'report.json' }], ['list']],
  timeout: 60000,
  use: { headless: true, ignoreHTTPSErrors: true },
};
"#);
    }

    let started = Instant::now();
    // Futa report ya ZAMANI kabla ya run — la sivyo ripoti ya kizamani
    // husababisha `ok:true` ya uongo (hata Playwright ikikosa).
    let report_path = e2e_dir.join("report.json");
    let _ = std::fs::remove_file(&report_path);

    // Sakinisha ndani ya e2e/ kama haipo (au kwa ombi la `install`), kisha endesha.
    let need_local = !e2e_dir.join("node_modules/@playwright/test").exists();
    let install_step = if need_local || do_install {
        "npm i --no-audit --no-fund >/dev/null 2>&1 && (npx playwright install chromium >/dev/null 2>&1 || true) && "
    } else {
        ""
    };
    let cmd = format!(
        "cd {} && {}npx playwright test {}.spec.js --config playwright.config.js 2>&1 | tail -c 6000",
        e2e_dir.display(),
        install_step,
        name
    );
    let out = tokio::time::timeout(
        Duration::from_secs(900),
        tokio::process::Command::new("bash").arg("-lc").arg(&cmd).output(),
    )
    .await;
    let ms = started.elapsed().as_millis() as u64;
    let output = match out {
        Err(_) => return Json(json!({ "ok": false, "error": "timeout", "ms": ms, "cmd": cmd })),
        Ok(Err(e)) => return Json(json!({ "ok": false, "error": e.to_string(), "ms": ms })),
        Ok(Ok(o)) => format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
    };

    // Soma report HALISI iliyoandikwa na run hii.
    let report = std::fs::read_to_string(&report_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());

    let Some(report) = report else {
        // Hakuna report → Playwright hakukimbia (haipo / config error).
        // HATURUDII ok:true ya uongo.
        return Json(json!({
            "ok": false,
            "error": "Playwright hakutoa report — kama haipo: npm i -D @playwright/test && npx playwright install --with-deps chromium",
            "spec": spec_file.display().to_string(),
            "url": url,
            "ms": ms,
            "report": null,
            "output": output.chars().take(6000).collect::<String>(),
        }));
    };

    let passed = report["stats"]["expected"].as_u64().unwrap_or(0);
    let failed = report["stats"]["unexpected"].as_u64().unwrap_or(0);
    let flaky = report["stats"]["flaky"].as_u64().unwrap_or(0);

    Json(json!({
        "ok": failed == 0 && (passed > 0 || flaky > 0),
        "spec": spec_file.display().to_string(),
        "url": url,
        "ms": ms,
        "passed": passed,
        "failed": failed,
        "flaky": flaky,
        "report": report,
        "output": output.chars().take(6000).collect::<String>(),
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/e2e/status", get(status))
        .route("/api/ide/e2e/spec", post(spec))
        .route("/api/ide/e2e/run", post(run))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_contains_url_and_assertions() {
        let s = spec_for("login", "http://localhost:3000/");
        assert!(s.contains("http://localhost:3000/"));
        assert!(s.contains("expect"));
        assert!(s.contains("console errors"));
    }
}
