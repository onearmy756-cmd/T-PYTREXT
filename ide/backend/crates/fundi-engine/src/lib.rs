//! fundi-engine — MTECHOS injini YETU ya inference.
//!
//! Injini ya pure Rust inayopakia modeli za **GGUF** moja kwa moja (bila Python,
//! CUDA, Ollama, au vLLM ya nje). Inaunga mkono Llama / Qwen2 / Mistral
//! (RMSNorm + RoPE + GQA + SwiGLU) na quantization zote kuu za llama.cpp.
//!
//! ```no_run
//! use fundi_engine::{Engine, SamplingParams};
//! let mut e = Engine::load(std::path::Path::new("model.gguf"), 4096).unwrap();
//! let p = SamplingParams::default();
//! let jibu = e.chat("Wewe ni msaidizi.", "Habari?", 128, &p);
//! println!("{jibu}");
//! ```

pub mod engine;
pub mod gguf;
pub mod model;
pub mod quant;
pub mod sampler;
pub mod snn;
pub mod tensor;
pub mod tokenizer;

// ── Safu za kibinadamu + hisabati (CALCULUS/MATRIX) + BF16 fidelity ──────────
pub mod calculus;
pub mod cortex;
pub mod matrix;
pub mod precision;

pub use engine::{inspect, Engine, GenEvent};
pub use gguf::{GgmlType, GgufFile};
pub use model::{Model, ModelConfig};
pub use sampler::{SamplingParams, Rng};
pub use snn::{SnnConfig, SpikeStats};
pub use tokenizer::Tokenizer;
