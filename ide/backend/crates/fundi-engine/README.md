# fundi-engine

Injini YETU ya inference — **pure Rust**, GGUF moja kwa moja (bila Python/vLLM).

## Tabaka

| Faili | Kazi |
|-------|------|
| `gguf.rs` | Msomaji/andishi wa GGUF v3 (metadata + tensors) |
| `quant.rs` | Dequantization (F32/F16/BF16/Q4_0…Q8K) |
| `tensor.rs` | matvec, RMSNorm, softmax, RoPE, SiLU/GeLU (AVX2+FMA) |
| `model.rs` | Transformer forward (Llama/Qwen2/Mistral) + **spiking forward** |
| `snn.rs` | **Spiking Neural Network** (LIF, Shiftmax, addition-only, STDP) |
| `sampler.rs` | Sampling (temperature/top-p/top-k/repeat) |
| `tokenizer.rs` | Tokenizer (SPM/BPE, chat templates) |
| `engine.rs` | Orchestrator: chat + streaming (SSE) + **chat_spiking** |
| `cortex.rs` | **CORTEX** (ubongo): minicolumns, lateral inhibition (k-WTA), recurrent excitation, homeostasis, predictive coding |
| `matrix.rs` | **MATRIX**: matmul, matvec, LU, solve (Gaussian+pivoting), determinant, Frobenius norm |
| `calculus.rs` | **CALCULUS**: dual-number autodiff, Simpson/Romberg integration, cumulative trapezoid |
| `precision.rs` | **BF16 FIDELITY**: round-trip (RNE), stochastic rounding, Kahan/Neumaier, block scaling |

## CORTEX (safu ya kibinadamu)

CORTEX hujenga **juu ya SNN** (haiundi modeli mpya) na kuongeza tabia za cortex:
minicolumns, lateral inhibition (k-WTA → sparse coding), recurrent excitation
(working memory), homeostasis (kizingiti cha adaptive), na predictive coding
(residual/error signal). Hii hutoa "ubongo" wa sparse coding kwenye vifaa vidogo.

## CALCULUS + MATRIX

`calculus.rs` na `matrix.rs` ni safu za hisabati zinazotumiwa na CORTEX na
matumizi ya kiinjeni: autodiff (forward-mode), integration (Simpson/Romberg),
matmul/LU/solve. Zote ni pure Rust — hakuna BLAS.

## BF16 FIDELITY

`precision.rs` huhifadhi ubora wa faili za bf16: round-trip (RNE), stochastic
rounding (hupunguza bias), na **compensated summation** (Kahan/Neumaier) kwa dot
products ndefu (n_in hadi 17408 kwa Qwen3.8) — hivyo kusoma/kuzalisha hakupoteza
ubora zaidi ya ufafanuzi wa bf16 wenyewe.


## SNN (Spiking Neural Network)

Safu ya SNN iko juu ya injini ile ile ya transformer — inatumia **weights HALISI**
za GGUF, si modeli mpya. Hii ni "speed + ubora" kwa matumizi ya nishati NDOGO:

- **LIF neuron** (Leaky Integrate-and-Fire): integrate → leak → fire → reset.
- **Shiftmax** badala ya softmax: hakuna `exp` (huokoa nishati).
- **Addition-only matvec** (A2OS2A): spikes (0/1) → AC badala ya MAC.
- **k-WTA** (k-winners-take-all): inadhibiti sparsity (na nishati) kwa uhakika.
- **Bipolar attention** + **spiking pooling attention**.
- **Surrogate gradient** (arctan / fast-sigmoid) kwa mafunzo (BPTT).
- **Encodings**: rate (Bernoulli), TTFS (time-to-first-spike).
- **STDP** (Spike-Timing-Dependent Plasticity).
- **Energy accounting**: AC/MAC → pJ (Horowitz, ISSCC 2014).

### Matumizi

```rust
use fundi_engine::{Engine, SamplingParams, SnnConfig};

let mut eng = Engine::load(std::path::Path::new("model.gguf"), 2048)?;
let scfg = SnnConfig { t_steps: 4, sparsity_target: 0.9, ..Default::default() };
let (text, stats) = eng.chat_spiking("", "habari", 128, &SamplingParams::default(), &scfg);
println!("{text}\nsparsity={} savings={}x", stats.sparsity(), stats.savings_factor());
```

### API (HTTP)

- `GET  /api/engine/snn` — maelezo ya SNN + features.
- `POST /api/engine/snn/prefill {prompt, t_steps?, sparsity?}` — telemetry ya logits.
- `POST /api/engine/snn/chat {prompt, max_tokens?, sparsity?}` — kuzalisha kwa SNN.

## Kanuni

Hakuna `.unwrap()`/`panic!`/`..` kwenye code ya runtime; hucheshimu `Result`.
Modeli haipo → `ok:false` yenye ujumbe wazi (hakuna mock).
