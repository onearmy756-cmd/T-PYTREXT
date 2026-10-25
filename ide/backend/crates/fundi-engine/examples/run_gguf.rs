//! run_gguf.rs — endesha faili ya GGUF HALISI kwa injini YETU (bila vLLM/Python).
//!
//! Matumizi:
//!   cargo run -p fundi-engine --example run_gguf -- <model.gguf> [prompt] [max_tokens]

use fundi_engine::{Engine, GenEvent, SamplingParams};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "model.gguf".to_string());
    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "Once upon a time".to_string());
    let max_tokens: usize = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);

    let t0 = std::time::Instant::now();
    let mut eng = match Engine::load(std::path::Path::new(&path), 512) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("kosa la kupakia: {e}");
            std::process::exit(1);
        }
    };
    let info = fundi_engine::inspect(std::path::Path::new(&path)).unwrap_or_default();
    println!("── fundi-engine ─────────────────────────────");
    println!("modeli : {}", info["name"]);
    println!("arch   : {}", info["arch"]);
    println!(
        "layers : {} · embd {} · heads {} · kv {}",
        info["layers"], info["embd"], info["heads"], info["heads_kv"]
    );
    println!("vocab  : {} · params {}", info["vocab"], info["params"]);
    println!("quant  : {}", info["quant_mix"]);
    println!(
        "ukubwa : {} MB · kupakia {} ms",
        info["size_mb"],
        t0.elapsed().as_millis()
    );
    println!("prompt : {prompt:?}");
    println!("─────────────────────────────────────────────");

    let params = SamplingParams {
        temperature: 0.8,
        top_k: 40,
        top_p: 0.95,
        repeat_penalty: 1.1,
        seed: 1234,
        ..Default::default()
    };
    let template = if eng.tokenizer.special.contains_key("<|im_start|>") {
        "chatml"
    } else if eng.tokenizer.special.contains_key("<|begin_of_text|>") {
        "llama3"
    } else {
        "plain"
    };
    let full = eng.tokenizer.chat_prompt("", &prompt, template);
    let ids = eng.tokenizer.encode(&full, true);
    println!("tokeni : {}", ids.len());

    eng.generate(&ids, max_tokens, &params, &[], |ev| match ev {
        GenEvent::Token { text, .. } => {
            print!("{text}");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        GenEvent::Done {
            total,
            elapsed_ms,
            tps_avg,
            reason,
        } => {
            println!("\n─────────────────────────────────────────────");
            println!("tokeni : {total} · {elapsed_ms} ms · {tps_avg} tok/s · {reason}");
        }
        GenEvent::Error { message } => eprintln!("kosa: {message}"),
        GenEvent::Start { .. } => {}
    });
}
