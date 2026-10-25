//! Real code execution backend: compile/run scripts via tokio::process::Command.
//!
//! MTECHOS hukimbiza Rust/Go/Elixir/Python/R/JS/TS. Kila lugha huandikwa kwenye
//! faili la muda lenye kiendelezi sahihi, kisha huendeshwa kwa toolchain yake.

use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::tools::code_execution::CodeExecutionBackend;
use fundi_core::ToolError;

/// Lugha zinazotumika pamoja na kiendelezi cha faili la muda.
const LANGS: &[(&str, &str)] = &[
    ("rust", "rs"),
    ("go", "go"),
    ("elixir", "exs"),
    ("python", "py"),
    ("r", "R"),
    ("javascript", "js"),
    ("typescript", "ts"),
    ("bash", "sh"),
];

fn canonical_lang(language: &str) -> Option<&'static str> {
    match language.trim().to_ascii_lowercase().as_str() {
        "rust" | "rs" => Some("rust"),
        "go" | "golang" => Some("go"),
        "elixir" | "ex" | "exs" => Some("elixir"),
        "python" | "py" | "python3" => Some("python"),
        "r" | "rscript" => Some("r"),
        "javascript" | "js" | "node" => Some("javascript"),
        "typescript" | "ts" => Some("typescript"),
        "bash" | "sh" => Some("bash"),
        _ => None,
    }
}

fn extension(lang: &str) -> &'static str {
    LANGS
        .iter()
        .find(|(name, _)| *name == lang)
        .map(|(_, ext)| *ext)
        .unwrap_or("txt")
}

/// Code execution backend using local toolchains.
pub struct LocalCodeExecutionBackend {
    default_timeout_secs: u64,
}

impl LocalCodeExecutionBackend {
    pub fn new(default_timeout_secs: u64) -> Self {
        Self {
            default_timeout_secs,
        }
    }
}

impl Default for LocalCodeExecutionBackend {
    fn default() -> Self {
        Self::new(30)
    }
}

#[async_trait]
impl CodeExecutionBackend for LocalCodeExecutionBackend {
    async fn execute(
        &self,
        code: &str,
        language: Option<&str>,
        timeout: Option<u64>,
    ) -> Result<String, ToolError> {
        let requested = language.unwrap_or("rust");
        let lang = canonical_lang(requested).ok_or_else(|| {
            ToolError::InvalidParams(format!(
                "Unsupported language: {}. Supported: rust, go, elixir, python, r, javascript, typescript, bash",
                requested
            ))
        })?;
        let timeout_secs = timeout.unwrap_or(self.default_timeout_secs);

        let workdir = std::env::temp_dir().join(format!("fundi_exec_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&workdir)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create work dir: {}", e)))?;
        let src = workdir.join(format!("main.{}", extension(lang)));
        tokio::fs::write(&src, code)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write temp file: {}", e)))?;

        let script = shell_command(lang, &src, &workdir);
        // `bash -lc` hupakia profile ya login, ambayo mara nyingi HUWEKA UPYA
        // PATH na kupoteza saraka za toolchain (rustup, GOPATH). Kwa hivyo
        // tunaweka PATH mwenyewe NDANI ya script, baada ya profile kupakiwa.
        let script = format!("export PATH={}; {script}", toolchain_path());
        let mut cmd = TokioCommand::new("bash");
        cmd.arg("-lc").arg(&script);
        // cwd: workspace ya IDE (ili code/scripts zione faili halisi za mradi)
        // ikipatikana; la sivyo sandbox ya muda.
        match workspace_cwd() {
            Some(root) => cmd.current_dir(root),
            None => cmd.current_dir(&workdir),
        };
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
            let child = cmd
                .spawn()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to spawn bash: {}", e)))?;
            let output = child.wait_with_output().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to wait for process: {}", e))
            })?;
            Ok::<_, ToolError>((
                output.status.code().unwrap_or(-1),
                output.stdout,
                output.stderr,
            ))
        })
        .await;

        let _ = tokio::fs::remove_dir_all(&workdir).await;

        match result {
            Ok(Ok((exit_code, stdout, stderr))) => Ok(json!({
                "exit_code": exit_code,
                "stdout": String::from_utf8_lossy(&stdout).to_string(),
                "stderr": String::from_utf8_lossy(&stderr).to_string(),
                "language": lang,
            })
            .to_string()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ToolError::Timeout(format!(
                "Code execution timed out after {}s",
                timeout_secs
            ))),
        }
    }
}

/// cwd ya workspace kutoka env (`TERMINAL_CWD` > `MBILINYI_IDE_ROOT`) kama ipo.
fn workspace_cwd() -> Option<std::path::PathBuf> {
    for key in ["TERMINAL_CWD", "MBILINYI_IDE_ROOT"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                return Some(std::path::PathBuf::from(v));
            }
        }
    }
    None
}

/// PATH inayojumuisha saraka za kawaida za toolchain. `bash -lc` hupakia
/// profile ambayo huweza kuweka PATH nyembamba; hii huhakikisha `rustc`,
/// `go`, `elixir` na `node` zinapatikana hata kwenye mazingira ya huduma.
pub fn toolchain_path() -> String {
    let mut dirs: Vec<String> = Vec::new();
    for d in [
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/local/go/bin",
        "/usr/lib/go/bin",
    ] {
        dirs.push(d.to_string());
    }
    // Rustup/cargo (mtumiaji wa sasa, kisha HOME ya kawaida).
    if let Ok(home) = std::env::var("CARGO_HOME") {
        dirs.push(format!("{home}/bin"));
    }
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{home}/.cargo/bin"));
    }
    dirs.push("/workspace/.cargo/bin".to_string());
    // PATH iliyopo inabaki mwisho (haipotei, lakini haitawali).
    if let Ok(existing) = std::env::var("PATH") {
        dirs.push(existing);
    }
    dirs.join(":")
}

/// Amri ya bash ya kucompile/kukimbia faili kwa lugha husika.
fn shell_command(lang: &str, src: &std::path::Path, workdir: &std::path::Path) -> String {
    let s = src.display();
    let out = workdir.join("out.bin");
    match lang {
        "rust" => format!("rustc -O {s} -o {} && {}", out.display(), out.display()),
        "go" => format!("go run {s}"),
        "elixir" => format!("elixir {s}"),
        "python" => format!("python3 {s}"),
        "r" => format!("Rscript {s}"),
        "javascript" => format!("node {s}"),
        "typescript" => format!("npx --yes ts-node {s}"),
        "bash" => format!("bash {s}"),
        _ => format!("cat {s}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lugha_zote_za_msingi_zinatambulika() {
        for (name, _) in LANGS {
            assert_eq!(canonical_lang(name), Some(*name), "lugha {name} haitambuliki");
        }
        // Majina mbadala (aliases) hukubaliwa.
        assert_eq!(canonical_lang("rs"), Some("rust"));
        assert_eq!(canonical_lang("golang"), Some("go"));
        assert_eq!(canonical_lang("ex"), Some("elixir"));
        assert_eq!(canonical_lang("r"), Some("r"));
        assert_eq!(canonical_lang("Rscript"), Some("r"));
        assert_eq!(canonical_lang("node"), Some("javascript"));
        assert_eq!(canonical_lang("sh"), Some("bash"));
        // Herufi kubwa hazizuii.
        assert_eq!(canonical_lang("RUST"), Some("rust"));
        // Python ni lugha inayotumika (mtumiaji aliomba).
        assert_eq!(canonical_lang("python"), Some("python"));
        assert_eq!(canonical_lang("py"), Some("python"));
        // Lugha isiyojulikana hukataliwa (haipiti kama bash).
        assert_eq!(canonical_lang("cobol"), None);
        assert_eq!(canonical_lang(""), None);
    }

    #[test]
    fn viendelezi_vya_faili_ni_sahihi() {
        assert_eq!(extension("rust"), "rs");
        assert_eq!(extension("go"), "go");
        assert_eq!(extension("elixir"), "exs");
        assert_eq!(extension("python"), "py");
        assert_eq!(extension("r"), "R");
        assert_eq!(extension("javascript"), "js");
        assert_eq!(extension("typescript"), "ts");
        assert_eq!(extension("bash"), "sh");
        // Lugha isiyojulikana hurudi "txt" (haijifichi kama lugha halali).
        assert_eq!(extension("cobol"), "txt");
    }

    #[test]
    fn amri_ya_kila_lugha_ina_toolchain_sahihi() {
        let src = std::path::Path::new("/tmp/x/main.rs");
        let wd = std::path::Path::new("/tmp/x");
        assert!(shell_command("rust", src, wd).starts_with("rustc"));
        assert!(shell_command("go", src, wd).starts_with("go run"));
        assert!(shell_command("elixir", src, wd).starts_with("elixir"));
        assert!(shell_command("python", src, wd).starts_with("python3"));
        assert!(shell_command("r", src, wd).starts_with("Rscript"));
        assert!(shell_command("javascript", src, wd).starts_with("node"));
        assert!(shell_command("typescript", src, wd).starts_with("npx"));
        assert!(shell_command("bash", src, wd).starts_with("bash"));
        // Amri haina mabadiliko ya nje ya workdir (hakuna `rm`/`sudo`).
        for lang in ["rust", "go", "elixir", "python", "r", "javascript", "typescript", "bash"] {
            let cmd = shell_command(lang, src, wd);
            assert!(!cmd.contains("sudo"), "amri ya {lang} ina sudo");
            assert!(!cmd.contains("rm -rf"), "amri ya {lang} ina rm -rf");
        }
    }

    /// Endesha lugha halisi kupitia backend. Huruka (si kufeli) ikiwa toolchain
    /// haijasakinishwa — ili CI isivunjike kwenye mazingira yasiyo nayo.
    async fn run(lang: &str, code: &str) -> Option<String> {
        let be = LocalCodeExecutionBackend::new(20);
        let out = be.execute(code, Some(lang), Some(20)).await.ok()?;
        // `bash -lc` hupakia profile ambayo huweza kuweka PATH mpya; ikiwa
        // toolchain haipo, bash hurudisha 127 ("command not found"). Hiyo ni
        // hali ya mazingira, si kosa la backend — iruke.
        if out.contains("\"exit_code\":127") || out.contains("command not found") {
            return None;
        }
        Some(out)
    }

    #[tokio::test]
    async fn lugha_zote_hukimbia_code_halisi() {
        let cases = [
            ("rust", "fn main(){println!(\"MTECHOS-RUST\");}", "MTECHOS-RUST"),
            ("go", "package main\nimport \"fmt\"\nfunc main(){fmt.Println(\"MTECHOS-GO\")}", "MTECHOS-GO"),
            ("elixir", "IO.puts(\"MTECHOS-ELIXIR\")", "MTECHOS-ELIXIR"),
            ("python", "print('MTECHOS-PY')", "MTECHOS-PY"),
            ("r", "cat(\"MTECHOS-R\\n\")", "MTECHOS-R"),
            ("javascript", "console.log('MTECHOS-JS')", "MTECHOS-JS"),
            ("bash", "echo MTECHOS-BASH", "MTECHOS-BASH"),
        ];
        let mut ran = 0;
        for (lang, code, expect) in cases {
            let Some(out) = run(lang, code).await else {
                eprintln!("{lang}: toolchain haipo — imerukwa");
                continue;
            };
            assert!(
                out.contains(expect),
                "{lang} haikutoa {expect}; output: {out}"
            );
            assert!(out.contains("\"exit_code\":0"), "{lang} exit_code si 0: {out}");
            ran += 1;
        }
        assert!(ran >= 4, "lugha chache ziliendeshwa: {ran}");
    }

    #[tokio::test]
    async fn lugha_isiyojulikana_hukataliwa() {
        let be = LocalCodeExecutionBackend::new(5);
        let err = be.execute("DISPLAY 'X'.", Some("cobol"), Some(5)).await;
        assert!(err.is_err(), "cobol haipaswi kukubaliwa");
    }
}
