//! nyi_ft.rs — FINE-TUNE ya 2NYI: kikusanya dataset kutoka UHALISIA.
//!
//! KANUNI (grounding): dataset hii hujengwa kutoka rekodi HALISI zilizopo:
//!   - `data/lancedb/brain_memories.lance/*.jsonl` — pairs za (tatizo, suluhisho)
//!     zilizothibitishwa (record_lesson / brain::remember).
//!
//! HAIbuni mifano. Kama hakuna data, hurudisha 0 pairs na kueleza wazi kwamba
//! fine-tune halisi inahitaji data ya kutosha (si dhana).
//!
//! Matokeo: JSONL ya ChatML (`messages: [system, user, assistant]`) — muundo
//! unaotumiwa na Unsloth/TRL/LLaMA-Factory kwa LoRA ya Qwen2.5.

use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

/// Saraka ya fine-tune (data/finetune).
pub fn ft_dir() -> PathBuf {
    ft_dir_in(&crate::data_io::data_dir())
}

fn ft_dir_in(data: &std::path::Path) -> PathBuf {
    let d = data.join("finetune");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Soma pairs kutoka kumbukumbu ya vector (brain_memories shards).
fn from_vector(data: &std::path::Path) -> Vec<(String, String, f64)> {
    let store = crate::vector::LanceStore::open(&data.display().to_string());
    store
        .list_records()
        .into_iter()
        .filter(|r| !r.problem.trim().is_empty() && !r.solution.trim().is_empty())
        .map(|r| (r.problem, r.solution, r.confidence))
        .collect()
}

/// Unda dataset ya ChatML JSONL. Hurudisha (njia, idadi ya pairs, chanzo).
pub fn build_dataset() -> Result<Value, String> {
    build_dataset_in(&crate::data_io::data_dir())
}

/// Kama `build_dataset`, lakini kwa data dir iliyotolewa (kwa tests — bila env).
pub fn build_dataset_in(data: &std::path::Path) -> Result<Value, String> {
    let vec_pairs = from_vector(data);

    let system = crate::nyi::block();
    let system = if system.trim().is_empty() {
        crate::nyi::directive().to_string()
    } else {
        system
    };

    let mut lines: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (q, a, conf) in vec_pairs.iter() {
        let key = format!("{q}\u{1}{a}");
        if !seen.insert(key) {
            continue;
        }
        let row = json!({
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": q },
                { "role": "assistant", "content": a },
            ],
            "meta": { "confidence": conf, "source": "2nyi-grounding" }
        });
        lines.push(row.to_string());
    }

    let out = ft_dir_in(data).join("2nyi_dataset.jsonl");
    let mut f = std::fs::File::create(&out).map_err(|e| e.to_string())?;
    for l in &lines {
        writeln!(f, "{l}").map_err(|e| e.to_string())?;
    }

    Ok(json!({
        "ok": true,
        "path": out.display().to_string(),
        "pairs": lines.len(),
        "from_vector": vec_pairs.len(),
        "note": if lines.is_empty() {
            "HAKUNA pairs — kumbukumbu ni tupu. Fine-tune halisi inahitaji data ya kutosha; \
             endelea kutumia 2NYI (kila kazi hurekodi somo) kisha jenga tena."
        } else if lines.len() < 50 {
            "Pairs chache sana kwa LoRA yenye maana (inashauriwa 500+). Tumia dataset kama \
             mwanzo wa kuendeleza, na endelea kukusanya."
        } else {
            "Dataset tayari kwa LoRA (Unsloth/TRL)."
        }.to_string(),
    }))
}

/// Script ya LoRA (llama.cpp `llama-finetune`, C++/CUDA) — huandikwa kama faili,
/// haiendeshwi hapa. Inahitaji GPU (CUDA); inaendeshwa kwenye server yenye GPU.
pub fn write_lora_script() -> Result<PathBuf, String> {
    let p = ft_dir().join("train_2nyi_lora.sh");
    std::fs::write(&p, LORA_SCRIPT).map_err(|e| e.to_string())?;
    Ok(p)
}

/// Maelekezo ya kuendesha fine-tune (kwa mtumiaji/LLM).
pub fn instructions() -> Value {
    json!({
        "ok": true,
        "gpu_required": true,
        "note": "Fine-tune (LoRA) HAIWEZI kuendeshwa kwenye sandbox hii (hakuna GPU). \
                 Inaendeshwa kwenye mashine yenye NVIDIA GPU (>=16GB VRAM kwa 3B QLoRA).",
        "steps": [
            "1) Jenga dataset: POST /api/nyi/finetune/build -> data/finetune/2nyi_dataset.jsonl",
            "2) Andika script: POST /api/nyi/finetune/script -> data/finetune/train_2nyi_lora.sh",
            "3) Kwenye GPU: jenga llama.cpp (CUDA): cmake -B build -DGGML_CUDA=ON && cmake --build build --target llama-finetune",
            "4) bash data/finetune/train_2nyi_lora.sh",
            "5) Hamisha adapter (LoRA) kwenye server kuu; unganisha kwenye GGUF ya 2NYI \
                kisha ihudumie kwa llama.cpp/Ollama au injini yetu ya Rust.",
            "6) Weka model: POST /api/ai/key au UI ya AI -> model = 2nyi-qwen2.5-3b-vl-lora"
        ],
        "base_model": "Qwen2.5-VL-3B-Instruct (au Qwen2.5-Coder-3B kwa code pekee) kwa GGUF",
        "data_needed": "500+ pairs zenye ubora; kwa sasa dataset hukusanya kutoka brain_memories"
    })
}

const LORA_SCRIPT: &str = r##"#!/usr/bin/env bash
# train_2nyi_lora.sh — LoRA ya 2NYI (Qwen2.5 3B) kwa llama.cpp `llama-finetune`.
# INAHITAJI GPU (CUDA). Endesha kwenye mashine yenye NVIDIA GPU, si kwenye CPU sandbox.
# HAKUNA Python: llama.cpp ni C++/CUDA; jq hutumika kugeuza ChatML JSONL -> text.
#
#   cmake -B build -DGGML_CUDA=ON && cmake --build build --target llama-finetune
#   bash train_2nyi_lora.sh
#
# Dataset: data/finetune/2nyi_dataset.jsonl (ChatML: system/user/assistant)
set -euo pipefail

DATA="data/finetune/2nyi_dataset.jsonl"
OUT="data/finetune/2nyi-lora.gguf"
BASE="${BASE_MODEL:-data/models/qwen2.5-vl-3b-instruct.gguf}"
TXT="data/finetune/2nyi_dataset.txt"
LLAMA_FINETUNE="${LLAMA_FINETUNE:-build/bin/llama-finetune}"

if [ ! -f "$DATA" ]; then
  echo "Dataset haipo: $DATA — endesha POST /api/nyi/finetune/build kwanza." >&2
  exit 1
fi

# ChatML JSONL -> text ya llama.cpp (kila mazungumzo kama kipande kimoja)
jq -r '.messages[] | (.role + ": " + .content)' "$DATA" > "$TXT"
PAIRS=$(jq -s 'length' "$DATA")
echo "pairs: $PAIRS"
if [ "$PAIRS" -lt 50 ]; then
  echo "ONYO: pairs < 50 — LoRA haitakuwa na maana. Kusanya data zaidi kwanza."
fi

"$LLAMA_FINETUNE" \
  --model-base "$BASE" \
  --train-data "$TXT" \
  --lora-out "$OUT" \
  --lora-r 16 --lora-alpha 16 \
  --ctx 4096 --threads 8 --batch 2 --grad-accum 4 \
  --adamw --lr 2e-4 --epochs 2 --seed 42

echo "LoRA imehifadhiwa: $OUT"
echo "Hatua inayofuata: unganisha na base model (llama-export-lora) kisha badilisha kwenda GGUF ya 2NYI."
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_hujengwa_bila_data_na_kurudisha_0() {
        let tmp = std::env::temp_dir().join(format!("nyi_ft_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let r = build_dataset_in(&tmp).expect("build_dataset haipaswi kufeli");
        assert_eq!(r["ok"], json!(true));
        assert_eq!(r["pairs"], json!(0));
        assert!(r["note"].as_str().unwrap().contains("HAKUNA pairs"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lora_script_huandikwa_na_ina_llamacpp() {
        let tmp = std::env::temp_dir().join(format!("nyi_ft3_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        std::env::set_var("FUNDI_DATA", &tmp);
        let p = write_lora_script().expect("script");
        let body = std::fs::read_to_string(&p).unwrap();
        assert!(body.contains("llama-finetune"), "script lazima itumie llama.cpp");
        assert!(body.contains("--lora-out"));
        std::env::remove_var("FUNDI_DATA");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn instructions_zinaonyesha_gpu_inahitajika() {
        let i = instructions();
        assert_eq!(i["gpu_required"], json!(true));
        assert!(i["steps"].as_array().unwrap().len() >= 5);
    }
}
