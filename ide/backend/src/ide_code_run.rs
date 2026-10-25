//! ide_code_run.rs — UTEKELEZAJI HALISI wa vipande vya code (sandbox ya muda).
//!
//! Awali `/api/code/run` ilielekea `fundi-server :8787` ambao haukimbii kwenye
//! dev, hivyo zana `code_execute` ilishindwa KILA MARA ("error sending request").
//! Modeli ilitegemea zana hiyo kwa mzunguko wa "andika → endesha → rekebisha",
//! hivyo uwezo wake wa kujithibitisha ulikatika.
//!
//! Hii hutekeleza kwa NDANI: code huandikwa kwenye saraka ya muda, kisha
//! interpreter huendeshwa kwa arg-array (HAKUNA shell injection), yenye timeout.

use serde_json::{json, Value};
use std::time::Duration;

const RUN_TIMEOUT: Duration = Duration::from_secs(30);

/// Lugha zinazotumika → amri ya kuendesha faili la code. `None` = haijulikani.
fn runner(lang: &str) -> Option<&'static str> {
    match lang {
        "rust" | "rs" => Some("rustc"),
        "node" | "js" | "javascript" => Some("node"),
        "ts" | "typescript" => Some("npx --yes tsx"),
        "bash" | "sh" | "shell" => Some("bash"),
        "go" | "golang" => Some("go run"),
        "elixir" | "ex" => Some("elixir"),
        "python" | "python3" | "py" => Some("python3"),
        "r" | "rscript" => Some("Rscript"),
        _ => None,
    }
}

fn file_name(lang: &str) -> &'static str {
    match lang {
        "rust" | "rs" => "main.rs",
        "node" | "js" | "javascript" => "main.js",
        "ts" | "typescript" => "main.ts",
        "bash" | "sh" | "shell" => "main.sh",
        "go" | "golang" => "main.go",
        "elixir" | "ex" => "main.exs",
        "python" | "python3" | "py" => "main.py",
        "r" | "rscript" => "main.R",
        _ => "main.txt",
    }
}

fn clip(s: &[u8]) -> String {
    String::from_utf8_lossy(s).chars().take(8000).collect()
}

/// Hutekeleza kipande cha code na kurudisha `{ok, language, exit, stdout, stderr, ms}`.
/// `cwd` (si lazima) huiweka utekelezaji ndani ya workspace ili faili zipatikane.
pub async fn run_code(code: &str, language: &str, cwd: Option<&str>) -> Value {
    let lang = language.trim().to_ascii_lowercase();
    let lang = if lang.is_empty() { "rust".to_string() } else { lang };

    let Some(cmd) = runner(&lang) else {
        return json!({
            "ok": false,
            "error": format!("lugha '{lang}' haitumiki — tumia: rust, node, ts, bash, go, elixir, python, r"),
        });
    };

    // Sandbox ya muda — hutolewa baada ya utekelezaji (haigusi workspace).
    let dir = std::env::temp_dir().join(format!("mtech-run-{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir_all(&dir).is_err() {
        return json!({ "ok": false, "error": "imeshindwa kutengeneza sandbox" });
    }
    // CWD ya utekelezaji: workspace ikiombwa (faili zake zinapatikana kwa
    // amri kama `node --test src/x.test.js`), la sivyo sandbox ya muda.
    // Code yenyewe hukaa KWENYE sandbox — workspace haichafuki.
    let workdir = match cwd.map(str::trim).filter(|s| !s.is_empty()) {
        Some(c) => {
            let p = std::path::PathBuf::from(c);
            if !p.is_dir() {
                let _ = std::fs::remove_dir_all(&dir);
                return json!({ "ok": false, "error": format!("cwd haipo: {c}") });
            }
            p
        }
        None => dir.clone(),
    };
    let src = dir.join(file_name(&lang));
    if std::fs::write(&src, code).is_err() {
        let _ = std::fs::remove_dir_all(&dir);
        return json!({ "ok": false, "error": "imeshindwa kuandika code" });
    }

    let started = std::time::Instant::now();
    // PATH ya zana za mtumiaji (huduma hii huendeshwa bila PATH ya mtumiaji).
    let path_prefix = crate::ide_api::tool_path_prefix();
    let shell_cmd = |inner: String| format!("{path_prefix}{inner}");
    // Rust: kwanza compile, kisha endesha binary.
    let result = if lang == "rust" || lang == "rs" {
        let compile = tokio::time::timeout(
            RUN_TIMEOUT,
            tokio::process::Command::new("bash")
                .arg("-lc")
                .arg(shell_cmd(format!(
                    "{} \"{}\" -o \"{}\" --edition 2021",
                    cmd,
                    src.display(),
                    dir.join("prog").display()
                )))
                .current_dir(&workdir)
                .output(),
        )
        .await;
        match compile {
            Err(_) => json!({ "ok": false, "error": "timeout wakati wa kucompile", "exit": null }),
            Ok(Err(e)) => json!({ "ok": false, "error": format!("{e}"), "exit": null }),
            Ok(Ok(o)) if !o.status.success() => json!({
                "ok": false, "stage": "compile", "exit": o.status.code(),
                "stdout": clip(&o.stdout), "stderr": clip(&o.stderr),
            }),
            Ok(Ok(_)) => {
                let exe = dir.join("prog");
                match tokio::time::timeout(
                    RUN_TIMEOUT,
                    tokio::process::Command::new(&exe).current_dir(&workdir).output(),
                )
                .await
                {
                    Err(_) => json!({ "ok": false, "error": "timeout wakati wa kuendesha" }),
                    Ok(Err(e)) => json!({ "ok": false, "error": format!("{e}") }),
                    Ok(Ok(o)) => json!({
                        "ok": o.status.success(), "stage": "run", "exit": o.status.code(),
                        "stdout": clip(&o.stdout), "stderr": clip(&o.stderr),
                    }),
                }
            }
        }
    } else {
        let inner = format!("{} \"{}\"", cmd, src.display());
        match tokio::time::timeout(
            RUN_TIMEOUT,
            tokio::process::Command::new("bash")
                .arg("-lc")
                .arg(shell_cmd(inner))
                .current_dir(&workdir)
                .output(),
        )
        .await
        {
            Err(_) => json!({ "ok": false, "error": "timeout (30s)" }),
            Ok(Err(e)) => json!({ "ok": false, "error": format!("{e}") }),
            Ok(Ok(o)) => json!({
                "ok": o.status.success(), "exit": o.status.code(),
                "stdout": clip(&o.stdout), "stderr": clip(&o.stderr),
            }),
        }
    };

    let ms = started.elapsed().as_millis() as u64;
    let _ = std::fs::remove_dir_all(&dir);
    let mut out = result;
    out["language"] = json!(lang);
    out["ms"] = json!(ms);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hutekeleza_bash_halisi() {
        let r = run_code("echo mtech-sandbox-ok", "bash", None).await;
        assert_eq!(r["ok"], json!(true), "matokeo: {r}");
        assert!(r["stdout"].as_str().unwrap_or("").contains("mtech-sandbox-ok"));
    }

    #[tokio::test]
    async fn hukataa_lugha_haijulikani() {
        let r = run_code("x", "cobol", None).await;
        assert_eq!(r["ok"], json!(false));
        assert!(r["error"].as_str().unwrap_or("").contains("haitumiki"));
    }

    #[tokio::test]
    async fn hurudisha_exit_isiyofanikiwa() {
        let r = run_code("exit 3", "bash", None).await;
        assert_eq!(r["ok"], json!(false));
        assert_eq!(r["exit"], json!(3));
    }

    #[tokio::test]
    async fn cwd_huruhusu_faili_za_workspace() {
        // Faili ya workspace inaonekana wakati cwd imewekwa.
        let dir = std::env::temp_dir().join(format!("mtech-cwd-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("data.txt"), "workspace-inaonekana").unwrap();
        let r = run_code("cat data.txt", "bash", Some(dir.to_str().unwrap())).await;
        assert_eq!(r["ok"], json!(true), "matokeo: {r}");
        assert!(r["stdout"].as_str().unwrap_or("").contains("workspace-inaonekana"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
