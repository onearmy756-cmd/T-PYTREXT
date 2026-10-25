//! e2e.rs — UTHIBITISHO WA MWISHO: modeli KAMILI ndogo (Llama) inajengwa kama
//! GGUF halisi, inapakiwa, na forward pass NZIMA inaendeshwa — bila kupakua
//! modeli yoyote kutoka mtandao.

use fundi_engine::gguf::{write_test_gguf, GgmlType, MetaValue};
use fundi_engine::{Engine, SamplingParams};

/// Deterministic pseudo-random f32 generator (xorshift) kwa weights.
struct Rng(u64);
impl Rng {
    fn f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 / (1u32 << 24) as f32 - 0.5) * 0.1
    }
}

fn f32_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Tengeneza GGUF kamili ya Llama (n_layer=2) kwa majaribio.
fn build_model_gguf() -> Vec<u8> {
    let n_layer = 2usize;
    let n_embd = 16usize;
    let n_head = 2usize;
    let n_kv = 1usize;
    let head_dim = n_embd / n_head; // 8
    let n_ff = 32usize;
    let vocab = 32usize;

    // Vocab: tokeni za herufi moja (byte tokens) + specials — rahisi kwa test.
    let mut tokens: Vec<MetaValue> = Vec::new();
    let mut scores: Vec<MetaValue> = Vec::new();
    let mut types: Vec<MetaValue> = Vec::new();
    for i in 0..vocab {
        tokens.push(MetaValue::Str(format!("<0x{:02X}>", i)));
        scores.push(MetaValue::F64(-(i as f64)));
        types.push(MetaValue::I64(1));
    }

    let mut rng = Rng(0x1234_5678_9abc_def0);
    let mut tensors: Vec<(&str, Vec<u64>, GgmlType, Vec<u8>)> = Vec::new();

    // Embeddings + output.
    let emb: Vec<f32> = (0..n_embd * vocab).map(|_| rng.f32()).collect();
    tensors.push((
        "token_embd.weight",
        vec![n_embd as u64, vocab as u64],
        GgmlType::F32,
        f32_bytes(&emb),
    ));
    let out: Vec<f32> = (0..n_embd * vocab).map(|_| rng.f32()).collect();
    tensors.push((
        "output.weight",
        vec![n_embd as u64, vocab as u64],
        GgmlType::F32,
        f32_bytes(&out),
    ));
    let on: Vec<f32> = vec![1.0; n_embd];
    tensors.push((
        "output_norm.weight",
        vec![n_embd as u64],
        GgmlType::F32,
        f32_bytes(&on),
    ));

    for l in 0..n_layer {
        let mut mk = |dims: Vec<u64>| -> (Vec<u64>, Vec<u8>) {
            let n: usize = dims.iter().product::<u64>() as usize;
            (dims, f32_bytes(&(0..n).map(|_| rng.f32()).collect::<Vec<f32>>()))
        };
        let names: [(&str, Vec<u64>); 7] = [
            ("attn_norm.weight", vec![n_embd as u64]),
            ("attn_q.weight", vec![n_embd as u64, (n_head * head_dim) as u64]),
            ("attn_k.weight", vec![n_embd as u64, (n_kv * head_dim) as u64]),
            ("attn_v.weight", vec![n_embd as u64, (n_kv * head_dim) as u64]),
            ("attn_output.weight", vec![(n_head * head_dim) as u64, n_embd as u64]),
            ("ffn_norm.weight", vec![n_embd as u64]),
            ("ffn_gate.weight", vec![n_embd as u64, n_ff as u64]),
        ];
        let mut owned: Vec<(String, Vec<u64>, Vec<u8>)> = Vec::new();
        for (suffix, dims) in names {
            let (d, b) = mk(dims);
            owned.push((format!("blk.{l}.{suffix}"), d, b));
        }
        // attn_norm/ffn_norm ziwe 1.0 (norm weights kawaida ~1)
        for name in [format!("blk.{l}.attn_norm.weight"), format!("blk.{l}.ffn_norm.weight")] {
            if let Some(e) = owned.iter_mut().find(|(n, _, _)| *n == name) {
                e.2 = f32_bytes(&vec![1.0f32; n_embd]);
            }
        }
        let (d, b) = mk(vec![n_embd as u64, n_ff as u64]);
        owned.push((format!("blk.{l}.ffn_up.weight"), d, b));
        let (d, b) = mk(vec![n_ff as u64, n_embd as u64]);
        owned.push((format!("blk.{l}.ffn_down.weight"), d, b));

        // Hifadhi kwa maisha ya function — tunatumia Box::leak kwa urahisi wa test.
        for (name, dims, bytes) in owned {
            let leaked: &'static str = Box::leak(name.into_boxed_str());
            tensors.push((leaked, dims, GgmlType::F32, bytes));
        }
    }

    let meta: Vec<(&str, MetaValue)> = vec![
        ("general.architecture", MetaValue::Str("llama".into())),
        ("general.name", MetaValue::Str("test-tiny-llama".into())),
        ("llama.block_count", MetaValue::U64(n_layer as u64)),
        ("llama.embedding_length", MetaValue::U64(n_embd as u64)),
        ("llama.attention.head_count", MetaValue::U64(n_head as u64)),
        ("llama.attention.head_count_kv", MetaValue::U64(n_kv as u64)),
        ("llama.feed_forward_length", MetaValue::U64(n_ff as u64)),
        (
            "llama.attention.layer_norm_rms_epsilon",
            MetaValue::F64(1e-5),
        ),
        ("llama.rope.freq_base", MetaValue::F64(10000.0)),
        ("tokenizer.ggml.tokens", MetaValue::Array(tokens)),
        ("tokenizer.ggml.scores", MetaValue::Array(scores)),
        ("tokenizer.ggml.token_type", MetaValue::Array(types)),
        ("tokenizer.ggml.bos_token_id", MetaValue::U64(1)),
        ("tokenizer.ggml.eos_token_id", MetaValue::U64(2)),
    ];

    write_test_gguf(&meta, &tensors)
}

fn write_tmp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fundi_engine_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn modeli_kamili_inapakia_na_inaendesha_forward() {
    let gguf = build_model_gguf();
    let p = write_tmp("tiny-llama.gguf", &gguf);

    // inspect: config inasomeka
    let info = fundi_engine::inspect(&p).unwrap();
    assert_eq!(info["arch"], "llama");
    assert_eq!(info["layers"], 2);
    assert_eq!(info["embd"], 16);
    assert_eq!(info["vocab"], 32);
    assert_eq!(info["heads"], 2);
    assert_eq!(info["heads_kv"], 1);

    // Pakia injini na endesha forward pass NZIMA.
    let mut eng = Engine::load(&p, 64).unwrap();
    let params = SamplingParams {
        temperature: 0.0, // greedy (deterministic)
        seed: 42,
        ..Default::default()
    };
    let out = eng.chat("", "hi", 8, &params);
    // Lazima itoe maandishi (inaweza kuwa byte tokens) na kutokuwa na panic.
    assert!(
        out.len() <= 8 * 8,
        "jibu ni refu mno: {} bytes",
        out.len()
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn forward_ni_deterministic_na_finite() {
    let gguf = build_model_gguf();
    let p = write_tmp("det.gguf", &gguf);
    let mut eng = Engine::load(&p, 32).unwrap();

    // Endesha prompt mbili tofauti; logits za mwisho ziwe finite.
    let ids = eng.tokenizer.encode("test", true);
    let logits = eng.model.prefill(&ids).unwrap();
    assert_eq!(logits.len(), 32);
    assert!(logits.iter().all(|v| v.is_finite()), "logits zina NaN/inf");

    // Determinism: prefill mara ya pili kwa cache mpya → matokeo yale yale.
    eng.model.cache.reset();
    let logits2 = eng.model.prefill(&ids).unwrap();
    for i in 0..logits.len() {
        assert!(
            (logits[i] - logits2[i]).abs() < 1e-4,
            "logits hazilingani kwenye {i}: {} vs {}",
            logits[i],
            logits2[i]
        );
    }
    let _ = std::fs::remove_file(&p);
}

#[test]
fn tokenizer_encode_decode_huzunguka() {
    let gguf = build_model_gguf();
    let p = write_tmp("tok.gguf", &gguf);
    let eng = Engine::load(&p, 16).unwrap();

    // Byte tokens: "A" (0x41 = 65) haipo (vocab=32) → fallback; lakini encode
    // haipaswi panic. Decode ya ids halali hurudisha maandishi ya tokeni hizo.
    let ids = eng.tokenizer.encode("hello", false);
    assert!(!ids.is_empty(), "encode imerudisha tupu");
    // Vocab ya test ina tokeni za maandishi "<0x00>".."<0x1F>" — decode huzirudisha.
    let s = eng.tokenizer.decode(&[0, 1, 2]);
    assert_eq!(s, "<0x00><0x01><0x02>");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn spiking_forward_inaendesha_na_sparsity_na_energy() {
    let gguf = build_model_gguf();
    let p = write_tmp("snn.gguf", &gguf);
    let mut eng = Engine::load(&p, 32).unwrap();

    // SNN config: sparsity ya juu → nishati ndogo.
    let scfg = fundi_engine::SnnConfig {
        t_steps: 4,
        sparsity_target: 0.9,
        ..Default::default()
    };

    let ids = eng.tokenizer.encode("test", true);
    let (logits, stats) = eng.model.prefill_spiking(&ids, &scfg).unwrap();
    assert_eq!(logits.len(), 32, "logits za vocab");
    assert!(logits.iter().all(|v| v.is_finite()), "logits zina NaN/inf");

    // SNN imetumia AC nyingi na MAC chache (addition-only).
    assert!(stats.acs > 0, "hakuna AC");
    assert!(stats.spikes > 0, "hakuna spikes");
    assert!(
        stats.sparsity() > 0.5,
        "sparsity ndogo mno: {}",
        stats.sparsity()
    );
    // Nishati ya SNN iwe ndogo kuliko dense (ubora: speed + nishati).
    assert!(
        stats.energy_snn_pj() < stats.energy_dense_pj(),
        "SNN haikuokoa nishati: snn={} dense={}",
        stats.energy_snn_pj(),
        stats.energy_dense_pj()
    );
    assert!(stats.savings_factor() > 1.0);

    // Determinism: spiking forward mara ya pili (cache mpya) = matokeo Yale Yale.
    eng.model.cache.reset();
    let (logits2, s2) = eng.model.prefill_spiking(&ids, &scfg).unwrap();
    for i in 0..logits.len() {
        assert!(
            (logits[i] - logits2[i]).abs() < 1e-4,
            "spiking logits hazilingani {i}: {} vs {}",
            logits[i],
            logits2[i]
        );
    }
    assert_eq!(stats.acs, s2.acs);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn spiking_logits_ni_finite_kwa_config_mbalimbali() {
    let gguf = build_model_gguf();
    let p = write_tmp("snn2.gguf", &gguf);
    let mut eng = Engine::load(&p, 16).unwrap();
    let ids = eng.tokenizer.encode("habari", true);
    for sparsity in [0.0f32, 0.5, 0.9, 0.99] {
        eng.model.cache.reset();
        let scfg = fundi_engine::SnnConfig {
            sparsity_target: sparsity,
            t_steps: 3,
            ..Default::default()
        };
        let (logits, stats) = eng.model.prefill_spiking(&ids, &scfg).unwrap();
        assert!(logits.iter().all(|v| v.is_finite()), "sparsity={sparsity}");
        assert!(stats.energy_snn_pj().is_finite());
    }
    let _ = std::fs::remove_file(&p);
}