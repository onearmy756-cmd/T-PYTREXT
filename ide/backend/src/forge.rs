//! forge.rs — MTECHOS FORGE: injini ya kuzalisha MRADI MZIMA kwa kasi.
//!
//! Mtiririko (chanzo kimoja — hakuna mantiki iliyorudiwa):
//!   spec (Kiswahili) → BLUEPRINT (LLM au template deterministic) →
//!   CODER WOLF (faili NYINGI kwa PARALLEL) → LINTER HALISI
//!   (`code_rules::lint`) → FIX LOOP (juu `MAX_FIX_ROUNDS`) → ANDIKA →
//!   USALAMA (`code_audit::analyze_content`) → ENGINEER (`ide_engineer::run`).
//!
//! Kanuni za repo:
//!   - Hakuna linter/LLM/writer mpya: tunaita `build_pipeline` (extract_code,
//!     fix_prompt), `code_rules::lint`, `code_audit`, `ide_engineer`, `chat::ask`,
//!     `ide_api::safe_path`. Chandulizo kimoja kwa kila kazi.
//!   - Kasi = faili nyingi zinazalishwa kwa PARALLEL (buffer_unordered).
//!   - Usalama: kila faili hupita lint + audit; path zote zinalindwa (`safe_path`).

use axum::extract::State;
use axum::Json;
use futures::stream::{self, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;

/// Juu ya mizunguko ya kujisahihisha kwa kila faili (inalingana na build_pipeline).
pub const MAX_FIX_ROUNDS: usize = 3;

/// Juu ya mizunguko ya kurekebisha makosa HALISI ya build (backend agent).
pub const BUILD_FIX_ROUNDS: usize = 2;

/// Idadi ya faili zinazozalishwa kwa wakati mmoja (kasi ya "coder wolf").
pub const CONCURRENCY: usize = 6;

/// Faili moja kwenye mpango wa mradi.
#[derive(Debug, Clone)]
pub struct FilePlan {
    pub path: String,
    pub purpose: String,
    pub lang: String,
    /// MKATABA wa interface: signatures, majina ya madaraja, IDs za DOM,
    /// ruwaza za JSON, n.k. Faili nyingine HULAZIMIKA kuutumia ili muunganisho
    /// ufanye kazi (index.html ↔ app.js, model ↔ screen). Tupu = hakuna.
    pub contract: String,
}

/// Mpango kamili wa mradi (blueprint).
#[derive(Debug, Clone)]
pub struct Blueprint {
    pub project: String,
    pub kind: String,
    pub lang: String,
    pub files: Vec<FilePlan>,
}

// ---------------------------------------------------------------------------
// Vadu safi (pure functions — zote zina tests)
// ---------------------------------------------------------------------------

/// Lugha kutoka extension ya path (default "text").
pub fn lang_from_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" => "javascript",
        "go" => "go",
        "html" | "htm" => "html",
        "java" => "java",
        "php" => "php",
        "rb" => "ruby",
        "ex" | "exs" => "elixir",
        "r" | "R" => "r",
        "css" => "css",
        "sql" => "sql",
        "sh" | "bash" => "shell",
        "json" => "json",
        "md" => "markdown",
        _ => "text",
    }
}

/// Aina ya mradi kutoka spec (deterministic).
pub fn detect_kind(spec: &str) -> &'static str {
    let s = spec.to_lowercase();
    if s.contains("api") || s.contains("rest") || s.contains("backend") || s.contains("server") {
        "api"
    } else if s.contains("cli") || s.contains("command line") || s.contains("terminal") {
        "cli"
    } else if s.contains("mobile") || s.contains("simu") || s.contains("android") || s.contains("ios")
    {
        "mobile"
    } else if s.contains("game") || s.contains("mchezo") || s.contains("3d") {
        "game"
    } else if s.contains("dashboard") || s.contains("data") || s.contains("analytics")
        || s.contains("report") || s.contains("takwimu")
    {
        "data"
    } else {
        "web"
    }
}

/// Lugha kuu kutoka spec (deterministic; mwisho hutegemea kind).
pub fn detect_lang(spec: &str, kind: &str) -> &'static str {
    let s = spec.to_lowercase();
    // Lugha mpya za production (nyongeza — hazibadilishi tabia za awali).
    if s.contains("elixir") || s.contains("phoenix") {
        return "elixir";
    }
    if s.contains("r analytics") || s.contains("r scripts") || s.contains("uchanganuzi wa r") {
        return "r";
    }
    if s.contains("candle") {
        return "rust";
    }
    if s.contains("rust") {
        "rust"
    } else if s.contains("typescript") {
        "typescript"
    } else if s.contains("javascript") || s.contains("node") {
        "javascript"
    } else if s.contains("golang") || s.contains(" go ") {
        "go"
    } else {
        match kind {
            "api" | "cli" | "data" => "rust",
            _ => "html",
        }
    }
}

/// Jina la mradi linalofaa filesystem (a-z, 0-9, -, _ pekee).
pub fn sanitize_name(spec: &str) -> String {
    let mut out = String::new();
    for ch in spec.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
        if out.len() >= 32 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "mradi".to_string()
    } else {
        trimmed
    }
}

/// Toa JSON kutoka jibu la LLM (huvumilia maelezo/nyongeza kabla/baada).
pub fn extract_json(reply: &str) -> Option<Value> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str(&reply[start..=end]).ok()
}

/// Soma blueprint kutoka jibu la LLM (JSON). Path hatari zinapuuzwa.
pub fn parse_blueprint(reply: &str) -> Option<Blueprint> {
    let v = extract_json(reply)?;
    let files_arr = v.get("files")?.as_array()?;
    let mut files: Vec<FilePlan> = Vec::new();
    for f in files_arr {
        let path = f
            .get("path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if path.is_empty() || path.contains("..") || path.starts_with('/') {
            continue;
        }
        let purpose = f
            .get("purpose")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let lang = f
            .get("lang")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| lang_from_path(&path).to_string());
        let contract = f
            .get("contract")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        files.push(FilePlan { path, purpose, lang, contract });
    }
    if files.is_empty() {
        return None;
    }
    Some(Blueprint {
        project: v
            .get("project")
            .and_then(|x| x.as_str())
            .unwrap_or("mradi")
            .to_string(),
        kind: v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("web")
            .to_string(),
        lang: v
            .get("lang")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        files,
    })
}

/// Prompt ya BLUEPRINT: muundo wa mradi (JSON pekee).
pub fn blueprint_prompt(spec: &str) -> String {
    format!(
        "Wewe ni MBINU (architect) wa MTECHOS FORGE. Tengeneza mpango wa mradi kwa JSON PEKEE \
         (bila maelezo, bila ```).\nMuundo:\n\
         {{\"project\":\"jina-fupi\",\"kind\":\"web|api|cli|mobile|game|data\",\
         \"lang\":\"html|rust|javascript|typescript|go|elixir\",\
         \"files\":[{{\"path\":\"rel/path.ext\",\"purpose\":\"kazi\",\"lang\":\"lugha\",\
         \"contract\":\"interface/signatures/IDs/modeli\"}}]}}\n\
         Weka faili 2-8 zinazotosha mradi KAMILI kufanya kazi. Path ziwe za jamaa (bila '..').\n\
         MUHIMU: `contract` ni mkataba wa interface ambao faili NYINGINE zitatekeleza. Andika \
         hasa: (a) majina ya madaraja/modeli na sehemu zake ZOTE (kwa mobile/data: constructor \
         inahitaji nini); (b) signatures za kazi zinazoitwa kutoka faili nyingine; (c) IDs za \
         DOM na functions za kiolesura (web/game); (d) ruwaza za JSON. Faili moja isitengeneze \
         logic ambayo faili nyingine pia ina (hakuna kurudia).\n\
         Mradi: {spec}"
    )
}

/// Prompt ya faili MOJA: kodi kamili ndani ya fenced block moja.
/// `siblings` = faili NYINGINE za mradi (path + madhumuni) ili faili hizi
/// zilingane (mfano index.html na app.js zitumie IDs zile zile) — faili
/// huzalishwa kwa PARALLEL, hivyo muktadha huu ni muhimu kwa muunganisho.
pub fn file_prompt(spec: &str, file: &FilePlan, siblings: &[FilePlan]) -> String {
    let others: String = siblings
        .iter()
        .filter(|f| f.path != file.path)
        .map(|f| {
            if f.contract.is_empty() {
                format!("  - {} : {}", f.path, f.purpose)
            } else {
                format!("  - {} : {} — MKATABA: {}", f.path, f.purpose, f.contract)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let others = if others.is_empty() {
        "  (hakuna)".to_string()
    } else {
        others
    };
    let own = if file.contract.is_empty() {
        String::new()
    } else {
        format!("\nMKATABA wa faili HII (faili nyingine zinautegemea — tekeleza KAMILI):\n  {}\n", file.contract)
    };
    format!(
        "Andika KODI HALISI (bila maelezo) ya faili `{path}` kwa mradi: {spec}\n\
         Madhumuni ya faili: {purpose}. Lugha: {lang}.{own}\
         Faili NYINGINE za mradi huu pamoja na MIKATABA yao (HAZIPASWI kugongana na hizi — \
         tumia majina/IDs/signatures/modeli ZILE ZILE zilizotajwa hapa ili muunganisho ufanye kazi):\n{others}\n\
         USIRUDIE logic iliyo kwenye faili nyingine (mfano: mantiki ya mchezo iwe faili MOJA tu).\n\
         Rejesha fenced block MOJA tu (```{lang} ... ```) yenye kodi KAMILI inayofanya kazi.\n\
         Kanuni za usalama: hakuna hard-coded API keys/password; hakuna eval/innerHTML (JS/Py); \
         Rust isitumie .unwrap() (tumia match/if let).",
        path = file.path,
        purpose = file.purpose,
        lang = file.lang,
        spec = spec,
        others = others,
        own = own,
    )
}

/// Blueprint ya KUHIFADHI (deterministic, hakuna LLM) — hutumika kama fallback.
pub fn template_blueprint(spec: &str) -> Blueprint {
    let kind = detect_kind(spec);
    let lang = detect_lang(spec, kind);
    let files = if lang == "elixir" {
        vec![
            FilePlan {
                path: "mix.exs".into(),
                purpose: "usanidi wa mradi wa Elixir (Mix)".into(),
                lang: "elixir".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "lib/mradi.ex".into(),
                purpose: "moduli kuu ya Elixir".into(),
                lang: "elixir".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "test/mradi_test.exs".into(),
                purpose: "testi za ExUnit".into(),
                lang: "elixir".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "README.md".into(),
                purpose: "maelezo ya matumizi".into(),
                lang: "markdown".into(),
                contract: String::new(),
            },
        ]
    } else if lang == "r" {
        vec![
            FilePlan {
                path: "main.R".into(),
                purpose: "programu kuu ya R".into(),
                lang: "r".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "R/analysis.R".into(),
                purpose: "kazi za uchanganuzi (base R)".into(),
                lang: "r".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "DESCRIPTION".into(),
                purpose: "metadata ya mradi wa R".into(),
                lang: "text".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "README.md".into(),
                purpose: "maelezo ya matumizi".into(),
                lang: "markdown".into(),
                contract: String::new(),
            },
        ]
    } else {
        match kind {
        "api" | "cli" => vec![
            FilePlan {
                path: "src/main.rs".into(),
                purpose: "programu kuu (endpoint/CLI)".into(),
                lang: "rust".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "Cargo.toml".into(),
                purpose: "usanidi wa crate".into(),
                lang: "text".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "README.md".into(),
                purpose: "maelezo ya matumizi".into(),
                lang: "markdown".into(),
                contract: String::new(),
            },
        ],
        "data" => vec![
            FilePlan {
                path: "src/main.rs".into(),
                purpose: "uchambuzi wa data + ripoti".into(),
                lang: "rust".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "Cargo.toml".into(),
                purpose: "usanidi wa crate".into(),
                lang: "text".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "README.md".into(),
                purpose: "maelezo ya matumizi".into(),
                lang: "markdown".into(),
                contract: String::new(),
            },
        ],
        "mobile" | "game" => vec![
            FilePlan {
                path: "index.html".into(),
                purpose: "kiolesura cha skrini".into(),
                lang: "html".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "app.js".into(),
                purpose: "mantiki ya kiolesura".into(),
                lang: "javascript".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "styles.css".into(),
                purpose: "muonekano".into(),
                lang: "css".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "README.md".into(),
                purpose: "maelezo ya matumizi".into(),
                lang: "markdown".into(),
                contract: String::new(),
            },
        ],
        _ => vec![
            FilePlan {
                path: "index.html".into(),
                purpose: "ukurasa wa mwanzo".into(),
                lang: "html".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "app.js".into(),
                purpose: "mantiki ya ukurasa".into(),
                lang: "javascript".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "styles.css".into(),
                purpose: "muonekano".into(),
                lang: "css".into(),
                contract: String::new(),
            },
            FilePlan {
                path: "README.md".into(),
                purpose: "maelezo ya matumizi".into(),
                lang: "markdown".into(),
                contract: String::new(),
            },
        ],
        }
    };
    Blueprint {
        project: sanitize_name(spec),
        kind: kind.to_string(),
        lang: lang.to_string(),
        files,
    }
}

/// Kodi ya kuhifadhi ya faili (deterministic) — hutumika kama fallback ya coder.
pub fn template_code(file: &FilePlan, spec: &str) -> String {
    match file.path.as_str() {
        "index.html" => format!(
            "<!DOCTYPE html>\n<html lang=\"sw\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{spec}</title>\n\
<link rel=\"stylesheet\" href=\"styles.css\">\n</head>\n<body>\n<header>\n<h1>{spec}</h1>\n\
<button id=\"run\">Anza</button>\n<p id=\"out\"></p>\n</header>\n<script src=\"app.js\"></script>\n\
</body>\n</html>\n"
        ),
        "app.js" => "document.addEventListener('DOMContentLoaded', function () {\n  \
var btn = document.getElementById('run');\n  var out = document.getElementById('out');\n  \
if (btn) {\n    btn.addEventListener('click', function () {\n      if (out) {\n        \
out.textContent = 'Habari ' + new Date().toISOString();\n      }\n    });\n  }\n});\n"
            .to_string(),
        "styles.css" => "body { font-family: system-ui, sans-serif; margin: 0; padding: 32px; background: #0d1117; color: #e6edf3; }\nheader { max-width: 720px; margin: 0 auto; }\nbutton { background: #58a6ff; color: #fff; border: 0; border-radius: 8px; padding: 10px 18px; cursor: pointer; }\n".to_string(),
        "src/main.rs" => "//! Programu kuu — mradi: ".to_string()
            + spec
            + "\nuse std::io::{Read, Write};\nuse std::net::TcpListener;\n\nfn main() {\n    let addr = \"127.0.0.1:8080\";\n    let listener = match TcpListener::bind(addr) {\n        Ok(l) => l,\n        Err(e) => {\n            eprintln!(\"bind imeshindikana: {e}\");\n            return;\n        }\n    };\n    println!(\"Inasikiliza {addr}\");\n    for stream in listener.incoming() {\n        match stream {\n            Ok(mut s) => {\n                let mut buf = [0u8; 1024];\n                let _ = s.read(&mut buf);\n                let body = \"{\\\"ok\\\":true}\";\n                let resp = format!(\n                    \"HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\nContent-Length: {}\\r\\n\\r\\n{}\",\n                    body.len(),\n                    body\n                );\n                let _ = s.write_all(resp.as_bytes());\n            }\n            Err(_) => continue,\n        }\n    }\n}\n",
        "Cargo.toml" => "[package]\nname = \"mradi\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n".to_string(),
        "mix.exs" => "defmodule Mradi.MixProject do\n  use Mix.Project\n\n  def project do\n    [\n      app: :mradi,\n      version: \"0.1.0\",\n      elixir: \"~> 1.15\",\n      start_permanent: Mix.env() == :prod,\n      deps: deps()\n    ]\n  end\n\n  def application do\n    [extra_applications: [:logger]]\n  end\n\n  defp deps, do: []\nend\n".to_string(),
        "lib/mradi.ex" => "defmodule Mradi do\n  @moduledoc \"Moduli kuu ya mradi wa Elixir.\"\n\n  @spec jumla([number()]) :: number()\n  def jumla(values), do: Enum.sum(values)\n\n  @spec wastani([number()]) :: float()\n  def wastani([]), do: 0.0\n  def wastani(values), do: jumla(values) / length(values)\nend\n".to_string(),
        "test/mradi_test.exs" => "defmodule MradiTest do\n  use ExUnit.Case, async: true\n\n  test \"jumla inahesabu\" do\n    assert Mradi.jumla([1, 2, 3]) == 6\n  end\n\n  test \"wastani wa tupu ni sifuri\" do\n    assert Mradi.wastani([]) == 0.0\n  end\nend\n".to_string(),
        "main.R" => "# Programu kuu — mradi: ".to_string()
            + spec
            + "\nsource(\"R/analysis.R\")\n\nvalues <- c(12, 7, 9, 15, 3, 22, 8)\nprint(summarize(values))\n",
        "R/analysis.R" => "# Kazi za uchanganuzi (base R — hakuna eval/system/setwd).\nsummarize <- function(values) {\n  if (length(values) == 0) {\n    return(list(count = 0, mean = 0, min = 0, max = 0))\n  }\n  list(\n    count = length(values),\n    mean = mean(values),\n    min = min(values),\n    max = max(values)\n  )\n}\n".to_string(),
        "DESCRIPTION" => "Package: mradi\nType: Project\nTitle: ".to_string()
            + spec
            + "\nVersion: 0.1.0\n",
        _ => format!("# {}\n\n{}\n", file.path, file.purpose),
    }
}

// ---------------------------------------------------------------------------
// Mtiririko (SSE)
// ---------------------------------------------------------------------------

use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;

async fn emit(tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>, ev: Value) {
    let _ = tx.send(Ok(Event::default().data(ev.to_string()))).await;
}

/// Faili "msingi": modeli/manifest/ruwaza ambazo faili nyingine zote
/// zinazitegemea. Hizi huzalishwa KWANZA (wimbi la kwanza) ili faili
/// zinazotegemea zipate kodi HALISI ya modeli (sio kubahatisha).
fn is_foundation(path: &str) -> bool {
    let p = path.to_lowercase();
    const MANIFESTS: [&str; 7] = [
        "pubspec.yaml", "package.json", "cargo.toml", "go.mod",
        "requirements.txt", "pyproject.toml", "index.html",
    ];
    let name = p.rsplit('/').next().unwrap_or(&p);
    if MANIFESTS.contains(&name) {
        return true;
    }
    // Faili za modeli/data (zinazotegemewa na skrini/UI).
    // Kumbuka: "interface"/"type" ni pana mno (zinanasa api_interface, types.dart
    // zisizokuwa modeli) — zinapaswa kuzalishwa kwenye wimbi la pili.
    p.contains("model")
        || p.contains("schema")
        || p.contains("entit")
}

/// MANIFEST INJECTION: hakikisha manifest inayohitajika ipo. LLM mara nyingi
/// huipanga lakini huisahau; bila manifest mradi hauwezi kuendeshwa (Flutter:
/// pubspec.yaml; Node: package.json; Rust: Cargo.toml). Hii ni HALISI — inaandika
/// faili kwenye diski.
fn inject_manifest(bp: &mut Blueprint, dir: &std::path::Path) -> Option<String> {
    let name = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let has = |bp: &Blueprint, n: &str| bp.files.iter().any(|f| name(&f.path) == n);

    let kind = bp.kind.as_str();
    let lang = bp.lang.to_lowercase();

    // (jina, yaliyomo, lugha) kulingana na aina ya mradi.
    let manifest: Option<(&str, String, &str)> = if kind == "mobile" || lang.contains("dart") || lang.contains("flutter") {
        if has(bp, "pubspec.yaml") { None } else { Some(("pubspec.yaml", pubspec_content(&bp.project), "text")) }
    } else if lang.contains("javascript") || lang.contains("typescript") || kind == "web" || kind == "game" {
        if has(bp, "package.json") { None } else { Some(("package.json", package_json_content(&bp.project), "json")) }
    } else if lang.contains("rust") {
        if has(bp, "Cargo.toml") { None } else { Some(("Cargo.toml", cargo_toml_content(&bp.project), "text")) }
    } else {
        None
    };

    let (mname, content, mlang) = manifest?;
    let full = dir.join(mname);
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&full, &content).ok()?;
    bp.files.push(FilePlan {
        path: mname.to_string(),
        purpose: "manifest (mradi uendeshwe)".into(),
        lang: mlang.to_string(),
        contract: String::new(),
    });
    Some(mname.to_string())
}

fn pubspec_content(name: &str) -> String {
    format!(
        "name: {}\ndescription: Mradi wa Flutter\npublish_to: 'none'\nversion: 1.0.0+1\n\n\
         environment:\n  sdk: '>=3.0.0 <4.0.0'\n\n\
         dependencies:\n  flutter:\n    sdk: flutter\n  http: ^1.1.0\n  provider: ^6.0.5\n  \
         cupertino_icons: ^1.0.6\n\n\
         dev_dependencies:\n  flutter_test:\n    sdk: flutter\n  flutter_lints: ^3.0.0\n\n\
         flutter:\n  uses-material-design: true\n",
        name = name.replace('-', "_")
    )
}

fn package_json_content(name: &str) -> String {
    format!(
        "{{\n  \"name\": \"{}\",\n  \"version\": \"1.0.0\",\n  \"private\": true,\n  \
         \"scripts\": {{ \"start\": \"node index.js\" }}\n}}\n",
        name
    )
}

fn cargo_toml_content(name: &str) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        name.replace('-', "_")
    )
}

/// Prompt ya faili MSINGI: modeli/data lazima iwe KAMILI kwa sababu faili
/// nyingine zinaitegemea (kila sehemu itumike; usiondoe kitu).
fn foundation_prompt(spec: &str, file: &FilePlan) -> String {
    format!(
        "Andika KODI HALISI (bila maelezo) ya faili MSINGI `{path}` kwa mradi: {spec}\n\
         Madhumuni: {purpose}. Lugha: {lang}.\n\
         Hii ni faili ya MSINGI — faili nyingine ZOTE zinaitegemea. Kwa hivyo:\n\
         - Weka modeli/madaraja YOTE yanayohitajika (kila sehemu itumike).\n\
         - Kwa kila modeli weka: constructor yenye sehemu zote, fromJson/toJson (kama lugha ina JSON).\n\
         - Usiondoe sehemu yoyote — faili nyingine zitaitumia.\n\
         Rejesha fenced block MOJA tu (```{lang} ... ```).\n\
         Kanuni: hakuna hard-coded keys; Rust isitumie .unwrap().",
        path = file.path, purpose = file.purpose, lang = file.lang, spec = spec
    )
}


/// Prompt ya KUREKEBISHA makosa HALISI ya build: rudisha output ya compiler kwa
/// coder, omba faili zilizorekebishwa KAMILI (fenced blocks + path).
fn build_fix_prompt(errors: &str, files: &[FilePlan]) -> String {
    let paths: String = files
        .iter()
        .map(|f| format!("  - {}", f.path))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "KODI HAICOMPILE. Hizi ni path za faili za mradi:\n{paths}\n\n\
         MAKOSA HALISI ya compiler:\n{errors}\n\n\
         Rekebisha makosa. Rejesha faili ZOTE zilizobadilika KAMILI. Kwa KILA faili andika \
         mstari `FILE: <path>` KABLA ya fenced block (mfano `FILE: Cargo.toml` kisha ```...```). \
         Usionyeshe faili zisizobadilika. Usiondoe kazi yoyote — weka kodi KAMILI ya kila faili. \
         Usiongeze `path:`/maelezo ndani ya faili za manifest (Cargo.toml, package.json, pubspec.yaml)."
    )
}

/// Changanua majibu ya fix: rudisha [(path, code)] kutoka fenced blocks.
/// Path inaweza kuwa (a) mstari `FILE: x` / `path: x` KABLA ya block, au
/// (b) mstari wa kwanza ndani ya block (`// path: x` / `# path: x`).
/// Kwa faili za manifest (Cargo.toml/JSON/YAML) hamna comment — hivyo (a) ni salama.
fn parse_fix(reply: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let lines: Vec<&str> = reply.lines().collect();
    let mut pending: Option<String> = None;
    let mut i = 0usize;
    while i < lines.len() {
        let t = lines[i].trim();
        if !t.starts_with("```") {
            if let Some(p) = path_marker(t) {
                pending = Some(p);
            }
            i += 1;
            continue;
        }
        // Block: kusanya mpaka fence ya kufunga.
        let mut j = i + 1;
        let mut body = String::new();
        while j < lines.len() && !lines[j].trim_start().starts_with("```") {
            body.push_str(lines[j]);
            body.push('\n');
            j += 1;
        }
        let mut path = pending.take();
        if path.is_none() {
            if let Some(first) = body.lines().next() {
                if let Some(p) = path_marker(first.trim()) {
                    path = Some(p);
                    body = body.splitn(2, '\n').nth(1).unwrap_or("").to_string();
                }
            }
        }
        if let Some(p) = path {
            if !body.trim().is_empty() {
                out.push((p, body));
            }
        }
        i = j + 1;
    }
    out
}

/// Tambua marker ya path kutoka mstari mmoja (a) nje ya block au (b) ndani ya block.
fn path_marker(line: &str) -> Option<String> {
    for pfx in [
        "// path:", "# path:", "path:", "FILE:", "file:", "// file:", "// filepath:",
        "# file:", "File:",
    ] {
        if let Some(rest) = line.strip_prefix(pfx) {
            let p = rest.trim().trim_matches('"').trim_matches('\'').trim();
            // Lazima ionekane kama path halisi (ina . au /) — si maneno ya kawaida.
            if !p.is_empty() && (p.contains('.') || p.contains('/')) {
                return Some(p.to_string());
            }
        }
    }
    None
}

async fn forge_file(
    brain: &crate::brain::Brain,
    spec: &str,
    project: &str,
    file: &FilePlan,
    siblings: &[FilePlan],
    foundation: bool,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
) -> Value {
    emit(
        tx,
        json!({ "type": "step", "stage": "code", "file": file.path }),
    )
    .await;

    let prompt = if foundation {
        foundation_prompt(spec, file)
    } else {
        file_prompt(spec, file, siblings)
    };
    // STREAMING: kodi inatiririka token kwa token (delta) — mtumiaji haoni kusubiri.
    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel::<crate::chat::ChatEvent>(256);
    let fname = file.path.clone();
    let tx_pump = tx.clone();
    let pump = tokio::spawn(async move {
        while let Some(ev) = delta_rx.recv().await {
            if ev.kind == "delta" {
                emit(
                    &tx_pump,
                    json!({ "type": "delta", "file": fname, "text": ev.text }),
                )
                .await;
            }
        }
    });
    let reply = crate::chat::code_ask_stream(brain, &prompt, &delta_tx).await;
    drop(delta_tx);
    let _ = pump.await;

    // Coder: kodi kutoka LLM; fallback ya kuhifadhi ikiwa LLM haikutoa block.
    let mut code = crate::build_pipeline::extract_code(&reply)
        .unwrap_or_else(|| template_code(file, spec));

    let mut ok = false;
    let mut rounds = 0usize;
    for r in 0..=MAX_FIX_ROUNDS {
        rounds = r;
        let report = crate::code_rules::lint(&code, &file.path);
        emit(
            tx,
            json!({ "type": "lint", "file": file.path, "round": r, "report": report }),
        )
        .await;
        if report["ok"].as_bool().unwrap_or(false) {
            ok = true;
            break;
        }
        if r == MAX_FIX_ROUNDS {
            break;
        }
        let findings: Vec<Value> = report["findings"].as_array().cloned().unwrap_or_default();
        emit(
            tx,
            json!({ "type": "step", "stage": "fix", "file": file.path, "round": r + 1 }),
        )
        .await;
        let fp = crate::build_pipeline::fix_prompt(&file.path, &code, &findings);
        let reply = crate::chat::code_ask(brain, &fp).await;
        match crate::build_pipeline::extract_code(&reply) {
            Some(c) => code = c,
            None => break,
        }
    }

    // ANDIKA (path inalindwa kwa safe_path).
    let rel = format!("{}/{}", project, file.path.trim_start_matches('/'));
    let written = match crate::ide_api::safe_path(&rel) {
        Some(full) => {
            if let Some(parent) = full.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&full, &code).is_ok()
        }
        None => false,
    };

    // USALAMA
    let audit = crate::code_audit::analyze_content(&file.lang, &code);
    let critical = audit
        .iter()
        .filter(|f| f["severity"] == json!("critical"))
        .count();

    emit(
        tx,
        json!({
            "type": "file", "path": file.path, "ok": ok, "rounds": rounds,
            "bytes": code.len(), "written": written, "critical": critical,
        }),
    )
    .await;

    json!({
        "path": file.path, "ok": ok, "rounds": rounds, "bytes": code.len(),
        "written": written, "critical": critical, "audit": audit,
    })
}

/// Ukaguzi wa muundo wa mradi: faili zote za HTML zinazorejelewa (`<script src>`,
/// `<link href>`) LAZIMA ziwepo kwenye orodha ya faili zilizopangwa. Hii inagundua
/// blueprint inayobuni faili za kubuni (mfano index.html inaita `snake.js` lakini
/// blueprint haikuipanga) — dosari halisi iliyoonekana kwenye mchezo wa snake.
fn verify_links(files: &[FilePlan]) -> Value {
    use std::collections::HashSet;
    let planned: HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    let mut missing: Vec<String> = Vec::new();
    for f in files {
        if f.lang != "html" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f.path) else { continue };
        for attr in ["src=\"", "href=\""] {
            let mut rest = text.as_str();
            while let Some(i) = rest.find(attr) {
                rest = &rest[i + attr.len()..];
                let Some(end) = rest.find('"') else { break };
                let url = &rest[..end];
                rest = &rest[end..];
                // Ruka URL za nje, absolute, na anchors.
                if url.starts_with("http") || url.starts_with("//") || url.starts_with('#') || url.is_empty() {
                    continue;
                }
                let rel = url.trim_start_matches("./").split('?').next().unwrap_or(url);
                if !planned.contains(rel) && !std::path::Path::new(rel).exists() {
                    missing.push(format!("{} → {}", f.path, rel));
                }
            }
        }
    }
    json!({
        "ok": missing.is_empty(),
        "missing": missing,
        "note": if missing.is_empty() { "Viungo vyote vipo." } else { "Faili zinazorejelewa hazipo (blueprint ilibuni faili)." }
    })
}

/// UTHIBITISHO HALISI wa kucompile: endesha build ya lugha na urudishe
/// matokeo ya kweli (sio madai). Hii inafanya FORGE isidai "ok" kwa kodi
/// isiyocompile — kigezo kikuu cha uwezo wa mradi mzima.
fn verify_project(dir: &std::path::Path, lang: &str) -> Value {
    use std::process::Command;
    // Lugha inaweza kuwa mchanganyiko ("html|css|javascript|rust").
    let langs: Vec<&str> = lang.split('|').map(|s| s.trim()).collect();
    let has = |x: &str| langs.iter().any(|l| *l == x);
    let (cmd, args): (&str, Vec<&str>) = if has("rust") && dir.join("Cargo.toml").exists() {
        ("cargo", vec!["build", "--quiet"])
    } else if has("go") && dir.join("go.mod").exists() {
        ("go", vec!["build", "./..."])
    } else if has("dart") && dir.join("pubspec.yaml").exists() {
        // Flutter/Dart: `dart analyze` ni uthibitisho bora (hakuna device inahitajika).
        ("dart", vec!["analyze", "--no-fatal-warnings"])
    } else if has("javascript") && dir.join("package.json").exists() {
        ("npm", vec!["install", "--silent"])
    } else {
        return json!({
            "attempted": false, "ok": null, "lang": lang,
            "note": "Hakuna build iliyojaribiwa (lugha ya kutafsiri au manifest haipo)."
        });
    };

    let out = Command::new(cmd)
        .args(&args)
        .current_dir(dir)
        .output();
    match out {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let errors: Vec<String> = stderr
                .lines()
                .filter(|l| l.starts_with("error") || l.contains("error["))
                .take(8)
                .map(|l| l.trim().to_string())
                .collect();
            json!({
                "attempted": true, "command": format!("{} {}", cmd, args.join(" ")),
                "ok": o.status.success(),
                "errors": errors,
                "note": if o.status.success() { "Build imefanikiwa." } else { "Build imeshindikana — angalia errors." }
            })
        }
        Err(e) => {
            let missing = e.kind() == std::io::ErrorKind::NotFound;
            json!({
                "attempted": true, "ok": null, "lang": lang,
                "note": if missing {
                    format!("Zana `{cmd}` haipo kwenye mfumo — build haikujaribiwa (sio kosa la kodi).")
                } else {
                    format!("Build haikuweza kuendeshwa: {e}")
                }
            })
        }
    }
}

/// Toa (attempted, ok, maandishi ya makosa) kutoka matokeo ya uthibitisho —
/// inashughulikia umbo la backend (`output` string) na la ndani (`errors` array).
fn build_errors(backend: &Value, local: &Value) -> (bool, bool, String) {
    if backend["attempted"].as_bool().unwrap_or(false) {
        let ok = backend["ok"].as_bool().unwrap_or(false);
        let errs = backend["output"].as_str().unwrap_or_default().to_string();
        return (true, ok, errs);
    }
    let ok = local["ok"].as_bool().unwrap_or(false);
    let errs = local["errors"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    (local["attempted"].as_bool().unwrap_or(false), ok, errs)
}


/// POST /api/forge/plan — blueprint PEKEE (hakuna kuzalisha kodi).
pub async fn plan(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let spec = body["spec"].as_str().unwrap_or("").trim().to_string();
    if spec.is_empty() {
        return Json(json!({ "ok": false, "error": "spec ni lazima" }));
    }
    let bp = parse_blueprint(&crate::chat::code_ask(&s.brain, &blueprint_prompt(&spec)).await);
    let (bp, source) = match bp {
        Some(b) => (b, "llm"),
        None => (template_blueprint(&spec), "template"),
    };
    Json(json!({
        "ok": true,
        "source": source,
        "project": bp.project,
        "kind": bp.kind,
        "lang": bp.lang,
        "files": bp.files.iter().map(|f| json!({
            "path": f.path, "purpose": f.purpose, "lang": f.lang
        })).collect::<Vec<_>>(),
    }))
}

/// POST /api/forge/run — zalisha MRADI MZIMA (SSE: blueprint → faili → done).
pub async fn run(
    State(s): State<crate::AppState>,
    Json(r): Json<Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let spec = r["spec"].as_str().unwrap_or("").trim().to_string();
    if spec.is_empty() {
        return Json(json!({ "ok": false, "error": "spec ni lazima" })).into_response();
    }
    let want_engineer = r["engineer"].as_bool().unwrap_or(true);
    // Generator deterministic (kasi + "milioni ya faili"): `{generator:"modules",count:N}`.
    let gen_kind = r["generator"].as_str().map(|s| s.to_string());
    let gen_count = r["count"].as_u64().unwrap_or(0) as usize;
    // Production (domain-driven): default true kwa kinds zinazozalisha kodi kamili.
    // `{production:false}` hulazimisha generator ya zamani (scaffold).
    let gen_production = r["production"].as_bool().unwrap_or(true);

    let brain = Arc::clone(&s.brain);
    let agent = s.agent.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(128);
    tokio::spawn(async move {
        let t0 = std::time::Instant::now();

        // 0) GENERATOR DETERMINISTIC (hakuna LLM) — kwa miradi mikubwa ya faili nyingi.
        let gen = gen_kind
            .as_deref()
            .filter(|k| !k.is_empty())
            .or_else(|| crate::forge_generators::detect(&spec));
        if let Some(kind) = gen {
            let project = sanitize_name(&spec);
            let count = if gen_count > 0 { gen_count } else { 64 };
            let is_prod = gen_production && crate::forge_prod::is_production(kind);
            let is_3d = crate::forge_rust3d::is_rust3d(kind);
            let is_flutter = gen_production && crate::forge_mobile::is_flutter(kind);
            let is_proj = gen_production && crate::forge_projects::is_project(kind);
            emit(&tx, json!({ "type": "step", "stage": "generate", "kind": kind, "count": count, "production": is_prod || is_3d || is_flutter || is_proj })).await;
            let (files, note, mode) = if is_3d {
                let domain = crate::forge_domain::parse_domain(&spec);
                let (f, n) = crate::forge_rust3d::generate_rust3d(&domain);
                (f, n, "production")
            } else if is_flutter {
                let domain = crate::forge_domain::parse_domain(&spec);
                let (f, n) = crate::forge_mobile::generate_flutter(&domain);
                (f, n, "production")
            } else if is_proj {
                let domain = crate::forge_domain::parse_domain(&spec);
                match crate::forge_projects::generate_project(kind, &domain) {
                    Some((f, n)) => (f, n, "production"),
                    None => {
                        let (f, n) = crate::forge_generators::generate(kind, &project, count);
                        (f, n, "generator")
                    }
                }
            } else if is_prod {
                let domain = crate::forge_domain::parse_domain(&spec);
                match crate::forge_prod::generate_production(kind, &domain) {
                    Some((f, n)) => (f, n, "production"),
                    None => {
                        let (f, n) = crate::forge_generators::generate(kind, &project, count);
                        (f, n, "generator")
                    }
                }
            } else {
                let (f, n) = crate::forge_generators::generate(kind, &project, count);
                (f, n, "generator")
            };
            let dir = crate::ide_api::safe_path(&project);
            let mut written = 0usize;
            if let Some(d) = &dir {
                for (path, content) in &files {
                    let full = d.join(path);
                    if let Some(parent) = full.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::write(&full, content).is_ok() {
                        written += 1;
                    }
                }
            }
            let elapsed = t0.elapsed().as_millis() as u64;
            // LANGI LA USALAMA: hata kizazi cha deterministic hupimwa.
            let gate = crate::sec_gate::audit_files(&files);
            emit(&tx, json!({
                "type": "security_gate",
                "verdict": gate["verdict"], "grade": gate["grade"],
                "findings": gate["total_findings"], "by_severity": gate["by_severity"],
            })).await;
            emit(
                &tx,
                json!({
                    "type": "done", "ok": true, "mode": mode,
                    "project": project, "kind": kind, "source": mode,
                    "files_total": files.len(), "files_written": written, "note": note,
                    "elapsed_ms": elapsed,
                    "security_gate": gate,
                    "release_ok": gate["verdict"] == json!("pass"),
                    "rate_per_sec": if elapsed > 0 { files.len() as u64 * 1000 / elapsed } else { 0 },
                }),
            )
            .await;
            return;
        }

        // 1) BLUEPRINT
        emit(&tx, json!({ "type": "step", "stage": "blueprint" })).await;
        let reply = crate::chat::code_ask(&brain, &blueprint_prompt(&spec)).await;
        let (mut bp, source) = match parse_blueprint(&reply) {
            Some(b) => (b, "llm"),
            None => (template_blueprint(&spec), "template"),
        };
        let project = sanitize_name(&bp.project);

        // MANIFEST INJECTION: hakikisha manifest ipo (inaandikwa sasa).
        let mut injected: Option<String> = None;
        if let Some(dir) = crate::ide_api::safe_path(&project) {
            let _ = std::fs::create_dir_all(&dir);
            injected = inject_manifest(&mut bp, &dir);
        }
        emit(
            &tx,
            json!({
                "type": "blueprint", "source": source, "project": project,
                "kind": bp.kind, "lang": bp.lang, "injected_manifest": injected,
                "files": bp.files.iter().map(|f| json!({
                    "path": f.path, "purpose": f.purpose, "lang": f.lang, "contract": f.contract
                })).collect::<Vec<_>>(),
            }),
        )
        .await;

        // 2) CODER WOLF — MAWIMBI MAWILI:
        //    (a) FAILI MSINGI (modeli/manifest) kwanza, kwa Mfululizo — ili kodi
        //        yao HALISI iwe kwenye siblings za wimbi la pili (muunganisho).
        //    (b) Faili nyingine kwa PARALLEL (kasi).
        let (foundation, rest): (Vec<FilePlan>, Vec<FilePlan>) = bp
            .files
            .iter()
            .cloned()
            .partition(|f| is_foundation(&f.path));
        emit(
            &tx,
            json!({ "type": "step", "stage": "foundation", "count": foundation.len() }),
        )
        .await;

        // Siblings zinaundwa hatua kwa hatua: kila faili ya msingi inaongeza
        // kodi yake HALISI (truncated) kwenye muktadha wa wale wanaofuata.
        let mut siblings: Vec<FilePlan> = rest.clone();
        let mut results: Vec<Value> = Vec::new();
        for f in &foundation {
            let r = forge_file(&brain, &spec, &project, f, &siblings, true, &tx).await;
            let written = crate::ide_api::safe_path(&project)
                .map(|d| d.join(&f.path))
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            let mut with_code = f.clone();
            with_code.contract = format!(
                "{}\n--- KODI HALISI (sehemu) ---\n{}",
                f.contract,
                written.chars().take(1800).collect::<String>()
            );
            siblings.push(with_code);
            results.push(r);
        }

        // (b) Wimbi la pili: faili zilizosalia kwa parallel (zinaona modeli halisi).
        let rest_results: Vec<Value> = stream::iter(rest)
            .map(|file| {
                let brain = Arc::clone(&brain);
                let spec = spec.clone();
                let project = project.clone();
                let siblings = siblings.clone();
                let tx = tx.clone();
                async move { forge_file(&brain, &spec, &project, &file, &siblings, false, &tx).await }
            })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;
        results.extend(rest_results);

        let total = results.len();
        let clean = results
            .iter()
            .filter(|x| x["ok"].as_bool().unwrap_or(false))
            .count();
        let critical: usize = results
            .iter()
            .map(|x| x["critical"].as_u64().unwrap_or(0) as usize)
            .sum();

        // 3) ENGINEER (testi + build + LSP + DAP + E2E + kasi) — hiari
        let eng = if want_engineer {
            emit(&tx, json!({ "type": "step", "stage": "engineer" })).await;
            let e = crate::ide_engineer::run(Json(json!({
                "path": project,
                "tests": true,
                "lint": true,
            })))
            .await
            .0;
            emit(
                &tx,
                json!({ "type": "engineer", "score": e["score"], "tests": e["tests"] }),
            )
            .await;
            e
        } else {
            Value::Null
        };

        // 3b) UTHIBITISHO HALISI wa build (honest — sio madai)
        emit(&tx, json!({ "type": "step", "stage": "verify" })).await;
        let proj_dir = crate::ide_api::safe_path(&project);
        let links = proj_dir
            .as_ref()
            .map(|d| {
                // Kagua HTML iliyoandikwa (paths za rel kuhusiana na dir ya mradi).
                let written: Vec<FilePlan> = bp
                    .files
                    .iter()
                    .map(|f| FilePlan {
                        path: d.join(&f.path).to_string_lossy().to_string(),
                        purpose: f.purpose.clone(),
                        lang: f.lang.clone(),
                        contract: f.contract.clone(),
                    })
                    .collect();
                verify_links(&written)
            })
            .unwrap_or_else(|| json!({ "ok": null, "note": "Project dir haipatikani." }));
        let verify = proj_dir
            .as_ref()
            .map(|dir| verify_project(dir, &bp.lang))
            .unwrap_or_else(|| json!({ "attempted": false, "ok": null, "note": "Project dir haipatikani." }));

        // 3c) UTHIBITISHO KWA BACKEND AGENT (tools HALISI) — pale backend ipo.
        // Inaendesha build kwenye mazingira ya backend (terminal tool) — hii ni
        // njia ya kweli pale `Command` ya ndani haiwezi kuendesha zana.
        let mut backend_verify = match (&agent, &proj_dir) {
            (Some(a), Some(d)) => {
                crate::forge_tools::verify_project(a, &d.to_string_lossy(), &bp.lang).await
            }
            _ => json!({ "attempted": false, "ok": null, "note": "Backend agent haipo." }),
        };

        // 3d) FIX LOOP kwa makosa HALISI ya build: rudisha output kwa coder, andika
        //     marekebisho, kisha jaribu build tena (hadi BUILD_FIX_ROUNDS).
        //     Makosa yanachukuliwa kutoka backend (kama ipo), la sivyo ya ndani.
        if want_engineer {
            for round in 1..=BUILD_FIX_ROUNDS {
                let (attempted, ok, errs) = build_errors(&backend_verify, &verify);
                if !attempted || ok || errs.trim().is_empty() {
                    break;
                }
                emit(&tx, json!({ "type": "step", "stage": "build_fix", "round": round })).await;
                let fix = crate::chat::code_ask(&brain, &build_fix_prompt(&errs, &bp.files)).await;
                let patches = parse_fix(&fix);
                let mut applied = 0usize;
                if let Some(d) = &proj_dir {
                    for (path, code) in &patches {
                        if let Some(full) = crate::ide_api::safe_path(&format!("{}/{}", project, path)) {
                            if full.starts_with(d) && std::fs::write(&full, code).is_ok() {
                                applied += 1;
                            }
                        }
                    }
                }
                emit(&tx, json!({ "type": "build_fix_applied", "round": round, "files": applied })).await;
                if applied == 0 {
                    break;
                }
                // Re-verify: backend ikiwa ipo, la sivyo ya ndani.
                if let (Some(a), Some(d)) = (&agent, &proj_dir) {
                    backend_verify =
                        crate::forge_tools::verify_project(a, &d.to_string_lossy(), &bp.lang).await;
                } else if let Some(d) = &proj_dir {
                    let _ = &verify;
                    backend_verify = verify_project(d, &bp.lang);
                }
            }
        }

        emit(&tx, json!({
            "type": "verify", "ok": verify["ok"], "links_ok": links["ok"],
            "backend_ok": backend_verify["ok"], "note": verify["note"],
        })).await;

        // 3e) LANGI LA USALAMA (sec_gate): kila mradi uliozalishwa LAZIMA
        //     upimwe usalama kabla ya kutolewa. Hii hufunga mzunguko:
        //     kagua → rekebisha → kagua tena → amua.
        emit(&tx, json!({ "type": "step", "stage": "security_gate" })).await;
        let gate = crate::sec_gate::gate_workspace(&project);
        emit(&tx, json!({
            "type": "security_gate",
            "verdict": gate["verdict"], "release_ok": gate["release_ok"],
            "grade": gate["audit"]["grade"], "findings": gate["audit"]["total_findings"],
            "by_severity": gate["audit"]["by_severity"],
        })).await;

        // 4) DONE
        emit(
            &tx,
            json!({
                "type": "done", "ok": true,
                "project": project, "kind": bp.kind, "lang": bp.lang, "source": source,
                "files_total": total, "files_clean": clean, "critical_findings": critical,
                "engineer_score": eng["score"],
                "security_gate": gate,
                "release_ok": gate["release_ok"],
                "build": verify,
                "backend_build": backend_verify,
                "links": links,
                "elapsed_ms": t0.elapsed().as_millis() as u64,
                "results": results,
            }),
        )
        .await;
    });

    Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// GET /api/forge/capabilities
pub async fn capabilities() -> Json<Value> {
    Json(json!({
        "ok": true,
        "engine": "MTECHOS FORGE",
        "pipeline": ["blueprint", "code(parallel)", "lint", "fix-loop", "write", "security", "engineer", "verify(build)"],
        "concurrency": CONCURRENCY,
        "max_fix_rounds": MAX_FIX_ROUNDS,
        "kinds": ["web", "api", "cli", "mobile", "game", "data"],
        "endpoints": ["POST /api/forge/plan", "POST /api/forge/run", "GET /api/forge/capabilities"],
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/forge/plan", post(plan))
        .route("/api/forge/run", post(run))
        .route("/api/forge/capabilities", get(capabilities))
}

// ---------------------------------------------------------------------------
// Tests — hesabu halisi (deterministic), hakuna mocks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Linter husoma `data/code_rules/languages.json` kutoka `FUNDI_DATA`.
    /// Cargo test huendesha kutoka `server/agent`, hivyo hapa tunaelekeza kwenye
    /// data ya repo (`<manifest>/../../data`) ili kujaribu NJIA HALISI ya lint.
    fn ensure_rules_env() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            if root.join("code_rules").join("languages.json").exists() {
                std::env::set_var("FUNDI_DATA", root.to_string_lossy().to_string());
            }
        });
    }

    #[test]
    fn lang_from_path_inatambua() {
        assert_eq!(lang_from_path("src/main.rs"), "rust");
        assert_eq!(lang_from_path("app.js"), "javascript");
        assert_eq!(lang_from_path("README.md"), "markdown");
        assert_eq!(lang_from_path("hakuna-extension"), "text");
    }

    #[test]
    fn detect_kind_ni_deterministic() {
        assert_eq!(detect_kind("tengeneza REST API ya malipo"), "api");
        assert_eq!(detect_kind("CLI ya kusimamia mafaili"), "cli");
        assert_eq!(detect_kind("mobile app ya simu"), "mobile");
        assert_eq!(detect_kind("mchezo wa 3D"), "game");
        assert_eq!(detect_kind("dashboard ya takwimu"), "data");
        assert_eq!(detect_kind("tovuti ya kuuza"), "web");
    }

    #[test]
    fn detect_lang_hutii_kind_na_spec() {
        assert_eq!(detect_lang("api ya malipo", "api"), "rust");
        assert_eq!(detect_lang("tovuti", "web"), "html");
        assert_eq!(detect_lang("dashboard", "data"), "rust");
    }

    #[test]
    fn detect_lang_inaunga_lugha_mpya() {
        assert_eq!(detect_lang("api ya elixir phoenix", "api"), "elixir");
        assert_eq!(detect_lang("r analytics ya mauzo", "data"), "r");
        assert_eq!(detect_lang("modeli ya candle", "api"), "rust");
    }

    #[test]
    fn lang_from_path_inaunga_elixir_na_r() {
        assert_eq!(lang_from_path("lib/app.ex"), "elixir");
        assert_eq!(lang_from_path("test/app_test.exs"), "elixir");
        assert_eq!(lang_from_path("R/analysis.R"), "r");
        assert_eq!(lang_from_path("main.R"), "r");
    }

    #[test]
    fn template_blueprint_elixir_na_r_ni_sahihi() {
        let bp = template_blueprint("api ya elixir phoenix");
        assert_eq!(bp.lang, "elixir");
        assert!(bp.files.iter().any(|f| f.path == "mix.exs"));
        assert!(bp.files.iter().any(|f| f.path == "lib/mradi.ex"));

        let bp = template_blueprint("r analytics ya mauzo");
        assert_eq!(bp.lang, "r");
        assert!(bp.files.iter().any(|f| f.path == "main.R"));
        assert!(bp.files.iter().any(|f| f.path == "R/analysis.R"));
    }

    #[test]
    fn template_code_mpya_inapita_lint() {
        ensure_rules_env();
        for spec in ["api ya elixir phoenix", "r analytics ya mauzo"] {
            let bp = template_blueprint(spec);
            for f in &bp.files {
                let code = template_code(f, spec);
                let report = crate::code_rules::lint(&code, &f.path);
                assert!(
                    report["ok"].as_bool().unwrap_or(false),
                    "faili {} ({spec}) haikupita lint: {}",
                    f.path,
                    report["findings"]
                );
            }
        }
    }

    #[test]
    fn sanitize_name_ni_salama() {
        assert_eq!(sanitize_name("Tovuti ya Kuuza Vitu!"), "tovuti-ya-kuuza-vitu");
        assert_eq!(sanitize_name("   "), "mradi");
        assert_eq!(sanitize_name("a/b\\c"), "abc");
    }

    #[test]
    fn extract_json_huvumilia_maelezo() {
        let r = "Hapa ni mpango wangu:\n{\"project\":\"x\",\"files\":[]}\nasante";
        let v = extract_json(r).expect("json");
        assert_eq!(v["project"], "x");
        assert!(extract_json("hakuna json").is_none());
    }

    #[test]
    fn parse_blueprint_hupuuza_path_hatari() {
        let r = r#"{"project":"bank","kind":"api","lang":"rust","files":[
            {"path":"src/main.rs","purpose":"endpoint","lang":"rust"},
            {"path":"../evil.rs","purpose":"x"},
            {"path":"/etc/passwd","purpose":"x"},
            {"path":"src/lib.rs"}
        ]}"#;
        let bp = parse_blueprint(r).expect("blueprint");
        assert_eq!(bp.project, "bank");
        assert_eq!(bp.files.len(), 2);
        assert_eq!(bp.files[0].path, "src/main.rs");
        assert_eq!(bp.files[1].lang, "rust"); // kutoka extension
    }

    #[test]
    fn parse_blueprint_bila_files_ni_None() {
        assert!(parse_blueprint("{\"project\":\"x\",\"files\":[]}").is_none());
        assert!(parse_blueprint("hakuna").is_none());
    }

    #[test]
    fn template_blueprint_inatoa_faili_kwa_kila_kind() {
        for spec in ["tovuti", "api ya malipo", "cli", "mobile app", "mchezo 3d", "dashboard"] {
            let bp = template_blueprint(spec);
            assert!(!bp.files.is_empty(), "kind {spec} haina faili");
            assert!(!bp.project.is_empty());
        }
    }

    #[test]
    fn template_code_inapita_lint_kwa_html_js() {
        ensure_rules_env();
        // Kodi ya kuhifadhi LAZIMA ipite linter (hakuna kosa la syntax/usalaa).
        let bp = template_blueprint("tovuti ya kuuza");
        for f in &bp.files {
            let code = template_code(f, "tovuti ya kuuza");
            let report = crate::code_rules::lint(&code, &f.path);
            assert!(
                report["ok"].as_bool().unwrap_or(false),
                "faili {} haikupita lint: {}",
                f.path,
                report["findings"]
            );
        }
    }

    #[test]
    fn template_code_inapita_lint_kwa_rust() {
        ensure_rules_env();
        for spec in ["api ya malipo", "dashboard ya takwimu"] {
            let bp = template_blueprint(spec);
            for f in &bp.files {
                let code = template_code(f, spec);
                let report = crate::code_rules::lint(&code, &f.path);
                assert!(
                    report["ok"].as_bool().unwrap_or(false),
                    "faili {} ({spec}) haikupita lint: {}",
                    f.path,
                    report["findings"]
                );
            }
        }
    }

    #[test]
    fn file_prompt_inajumuisha_path_na_spec() {
        let f = FilePlan {
            path: "src/auth.rs".into(),
            purpose: "uthibitisho".into(),
            lang: "rust".into(),
            contract: String::new(),
        };
        let p = file_prompt("app ya benki", &f, &[]);
        assert!(p.contains("src/auth.rs"));
        assert!(p.contains("app ya benki"));
        assert!(p.contains("uthibitisho"));
    }

    #[test]
    fn file_prompt_inataja_faili_wenzake() {
        let f = FilePlan {
            path: "app.js".into(),
            purpose: "logiki".into(),
            lang: "javascript".into(),
            contract: String::new(),
        };
        let sib = FilePlan {
            path: "index.html".into(),
            purpose: "muundo".into(),
            lang: "html".into(),
            contract: String::new(),
        };
        let p = file_prompt("dashibodi", &f, &[f.clone(), sib]);
        assert!(p.contains("index.html"));
        assert!(p.contains("muundo"));
        assert!(!p.contains("  - app.js :"), "faili yenyewe isitajwe kama sibling");
    }

    #[test]
    fn file_prompt_inaweka_mikataba_ya_wenzake_na_yenyewe() {
        let f = FilePlan {
            path: "lib/models/restaurant.dart".into(),
            purpose: "modeli".into(),
            lang: "dart".into(),
            contract: "class Restaurant { id, name, address, rating, menuItems }".into(),
        };
        let screen = FilePlan {
            path: "lib/screens/home_screen.dart".into(),
            purpose: "skrini".into(),
            lang: "dart".into(),
            contract: "HomeScreen inatumia Restaurant(...)".into(),
        };
        let p = file_prompt("app ya chakula", &f, &[f.clone(), screen]);
        assert!(p.contains("MKATABA wa faili HII"), "mkataba wa faili yenyewe uwepo");
        assert!(p.contains("menuItems"), "sehemu za modeli ziwepo");
        assert!(p.contains("MKATABA: HomeScreen"), "mkataba wa sibling uwepo");
        assert!(p.contains("USIRUDIE logic"), "kanuni ya kutokurudia iwepo");
    }

    #[test]
    fn parse_blueprint_inasoma_contract() {
        let json = r#"{"project":"p","kind":"mobile","lang":"dart",
            "files":[{"path":"a.dart","purpose":"x","lang":"dart","contract":"class A{}"}]}"#;
        let bp = parse_blueprint(json).expect("blueprint");
        assert_eq!(bp.files[0].contract, "class A{}");
    }

    #[test]
    fn parse_fix_inatoa_path_na_kodi() {
        let reply = "Hapa:\n```rust\n// path: src/main.rs\nfn main() {}\n```\nna\n```rust\n// path: src/lib.rs\npub fn a() {}\n```";
        let patches = parse_fix(reply);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].0, "src/main.rs");
        assert!(patches[0].1.contains("fn main()"));
        assert_eq!(patches[1].0, "src/lib.rs");
    }

    #[test]
    fn parse_fix_hupuuza_block_bila_path() {
        let reply = "```rust\nfn x() {}\n```";
        assert!(parse_fix(reply).is_empty());
    }

    #[test]
    fn parse_fix_inasoma_path_nje_ya_block() {
        let reply = "FILE: src/main.rs\n```rust\nfn main() {}\n```\n\nFILE: Cargo.toml\n```toml\n[package]\nname = \"x\"\n```";
        let patches = parse_fix(reply);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].0, "src/main.rs");
        assert_eq!(patches[1].0, "Cargo.toml");
        assert!(patches[1].1.contains("[package]"));
        assert!(!patches[1].1.contains("path:"), "manifest isiwe na marker ndani");
    }

    #[test]
    fn blueprint_prompt_inaomba_json() {
        let p = blueprint_prompt("tovuti");
        assert!(p.contains("JSON"));
        assert!(p.contains("files"));
        assert!(p.contains("tovuti"));
        assert!(p.contains("contract"), "blueprint iombe mikataba");
    }

    #[test]
    fn verify_links_inagundua_faili_iliyobuniwa() {
        let dir = std::env::temp_dir().join(format!("forge-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let html = dir.join("index.html");
        std::fs::write(&html, "<html><script src=\"ghost.js\"></script></html>").unwrap();
        let files = vec![FilePlan {
            path: html.to_string_lossy().to_string(),
            purpose: "ukurasa".into(),
            lang: "html".into(),
            contract: String::new(),
        }];
        let r = verify_links(&files);
        assert_eq!(r["ok"], json!(false));
        assert!(r["missing"][0].as_str().unwrap().contains("ghost.js"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrency_na_rounds_ni_positive() {
        assert!(CONCURRENCY >= 1);
        assert_eq!(MAX_FIX_ROUNDS, 3);
    }
}
