//! forge_tools.rs — FORGE ZANA HALISI kupitia backend agent (fundi-server :8787).
//!
//! FORGE ya codegen (`chat::code_ask`) HAITUMII tools kwa makusudi (ni njia safi
//! ya kuzalisha faili). Lakini UTHIBITISHO wa mradi unahitaji kutenda kwa kweli:
//! kucompile, kuendesha code, kukagua faili. Moduli hii inaunganisha FORGE na
//! tools 70 za backend (`terminal`, `execute_code`) ili uthibitisho uwe HALISI
//! (sio madai) — hata kwenye mazingira ambapo `Command` ya ndani haiwezi.
//!
//! Chombo kimoja: kila kazi inaita `invoke_tool_raw` (lango moja la registry).

use serde_json::{json, Value};

/// Toa maandishi halisi kutoka jibu la tool (`{"output": "{\"result\":\"...\"}"}`).
pub fn tool_output(v: &Value) -> String {
    if let Some(s) = v["output"].as_str() {
        if let Ok(inner) = serde_json::from_str::<Value>(s) {
            for key in ["result", "stdout", "output", "text"] {
                if let Some(r) = inner[key].as_str() {
                    return r.to_string();
                }
            }
            return inner.to_string();
        }
        return s.to_string();
    }
    v.to_string()
}

/// Endesha amri ya shell HALISI kwa backend tool `terminal`.
pub async fn run_terminal(
    agent: &crate::fundi_agent_client::FundiAgentClient,
    command: &str,
) -> Value {
    let body = json!({ "tool": "terminal", "params": { "command": command } });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => json!({
            "ok": v["ok"].as_bool().unwrap_or(false),
            "output": tool_output(&v),
        }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Endesha CODE HALISI kwa backend tool `execute_code` (rust/node/elixir).
pub async fn run_code(
    agent: &crate::fundi_agent_client::FundiAgentClient,
    code: &str,
    language: &str,
) -> Value {
    let body = json!({
        "tool": "execute_code",
        "params": { "code": code, "language": language },
    });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => json!({
            "ok": v["ok"].as_bool().unwrap_or(false),
            "output": tool_output(&v),
        }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// UTHIBITISHO kwa backend agent: endesha build/analyze kwenye project halisi.
/// Hii ni njia BORA pale backend ipo — inaendesha kwenye mazingira ya backend
/// (tools halisi) badala ya `Command` ya ndani ya process.
pub async fn verify_project(
    agent: &crate::fundi_agent_client::FundiAgentClient,
    dir: &str,
    lang: &str,
) -> Value {
    let langs: Vec<&str> = lang.split('|').map(|s| s.trim()).collect();
    let has = |x: &str| langs.iter().any(|l| *l == x);

    // (amri, jina la zana) — hii ni uthibitisho halisi wa kucompile.
    let (cmd, tool): (String, &str) = if has("rust") {
        ("cargo build --quiet".into(), "cargo")
    } else if has("dart") {
        ("dart analyze --no-fatal-warnings".into(), "dart")
    } else if has("go") {
        ("go build ./...".into(), "go")
    } else if has("elixir") {
        ("mix compile --warnings-as-errors".into(), "mix")
    } else if has("r") {
        ("Rscript -e 'invisible(lapply(list.files(\".\", pattern=\"[.]R$\", recursive=TRUE, full.names=TRUE), parse))'".into(), "Rscript")
    } else if has("javascript") || has("typescript") {
        ("npm install --silent".into(), "npm")
    } else {
        return json!({
            "attempted": false, "ok": null, "lang": lang, "via": "backend",
            "note": "Hakuna build inayojaribiwa (lugha ya kutafsiri)."
        });
    };

    // Ruka ikiwa manifest haipo (kama verify ya ndani).
    let manifest_ok = run_terminal(
        agent,
        &format!(
            "cd {} 2>/dev/null && ls Cargo.toml pubspec.yaml go.mod mix.exs DESCRIPTION package.json 2>/dev/null | head -1",
            shell_escape(dir)
        ),
    )
    .await;
    if manifest_ok["output"].as_str().unwrap_or("").trim().is_empty() {
        return json!({
            "attempted": false, "ok": null, "lang": lang, "via": "backend",
            "note": "Hakuna manifest ya build kwenye mradi."
        });
    }

    let r = run_terminal(
        agent,
        &format!("cd {} && {} 2>&1 | tail -40", shell_escape(dir), cmd),
    )
    .await;
    let out = r["output"].as_str().unwrap_or("").to_string();
    let missing = out.contains("command not found") || out.contains("No such file or directory");
    json!({
        "attempted": true, "ok": r["ok"].as_bool(), "lang": lang, "via": "backend",
        "command": cmd, "tool": tool, "output": out.chars().take(2000).collect::<String>(),
        "note": if missing {
            format!("Zana `{tool}` haipo kwenye mazingira ya backend.")
        } else if r["ok"].as_bool().unwrap_or(false) {
            "Build imefanikiwa (backend).".to_string()
        } else {
            "Build imeshindikana (backend) — angalia output.".to_string()
        }
    })
}

/// Zuia uingizaji wa shell (dir inaweza kuwa na nafasi/quote).
fn shell_escape(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || "/._-".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_output_inatoa_result() {
        let v = json!({ "output": "{\"result\":\"hello\\nworld\"}" });
        assert_eq!(tool_output(&v), "hello\nworld");
    }

    #[test]
    fn tool_output_inashughulikia_stdout_na_plain() {
        assert_eq!(tool_output(&json!({ "output": "{\"stdout\":\"abc\"}" })), "abc");
        assert_eq!(tool_output(&json!({ "output": "raw text" })), "raw text");
    }

    #[test]
    fn shell_escape_inalinda_nafasi() {
        assert_eq!(shell_escape("/home/x/proj"), "/home/x/proj");
        assert_eq!(shell_escape("/a b/c"), "'/a b/c'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}
