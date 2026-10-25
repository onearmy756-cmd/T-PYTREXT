//! mtechos-dependabot — uamuzi wa checks za PR
//! (Rust — hakuna Python; jq kwa CI).
//!
//! Husoma hali ya checks kutoka `/tmp/pr.json` (matokeo ya
//! `gh pr view --json statusCheckRollup,mergeable,mergeStateStatus`) na
//! kuchapisha `<STATE>|<detail>` ambapo STATE ni PASS, FAIL, PENDING, au EMPTY.
//!
//! Matumizi: mtechos-dependabot [pr.json]

use std::process::ExitCode;

use serde_json::Value;

const IGNORE: [&str; 2] = ["auto-merge", "dependabot auto-merge"];

fn main() -> ExitCode {
    let path = std::env::args().nth(1).unwrap_or_else(|| "/tmp/pr.json".to_string());
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => {
            // Fail-open: hakuna data bado → bado tunasubiri.
            println!("PENDING|hakuna data bado");
            return ExitCode::SUCCESS;
        }
    };
    let data: Value = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(_) => {
            println!("PENDING|hakuna data bado");
            return ExitCode::SUCCESS;
        }
    };

    let mut fail: Vec<String> = Vec::new();
    let mut pend: Vec<String> = Vec::new();
    let mut ok: Vec<String> = Vec::new();

    if let Some(rollup) = data["statusCheckRollup"].as_array() {
        for c in rollup {
            let name = c["name"]
                .as_str()
                .or_else(|| c["context"].as_str())
                .unwrap_or("(bila jina)")
                .to_string();
            if IGNORE.iter().any(|t| name.to_ascii_lowercase().contains(t)) {
                continue;
            }
            if c.get("status").is_some() {
                // CheckRun
                let st = c["status"].as_str().unwrap_or("");
                let cc = c["conclusion"].as_str().unwrap_or("");
                if st != "COMPLETED" {
                    pend.push(name);
                } else if matches!(cc, "SUCCESS" | "NEUTRAL" | "SKIPPED") {
                    ok.push(name);
                } else {
                    fail.push(name);
                }
            } else {
                // StatusContext ya zamani
                match c["state"].as_str().unwrap_or("") {
                    "SUCCESS" => ok.push(name),
                    "PENDING" | "EXPECTED" => pend.push(name),
                    _ => fail.push(name),
                }
            }
        }
    }

    let line = if !fail.is_empty() {
        format!("FAIL|{}", fail.join(", "))
    } else if !pend.is_empty() {
        format!("PENDING|{}", pend.join(", "))
    } else if !ok.is_empty() {
        format!("PASS|{}", ok.join(", "))
    } else {
        "EMPTY|hakuna checks".to_string()
    };
    println!("{line}");
    ExitCode::SUCCESS
}
