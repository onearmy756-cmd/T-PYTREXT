//! tokenizer.rs — TOKENIZER kutoka metadata ya GGUF (bila faili za nje).
//!
//! GGUF huhifadhi: `tokenizer.ggml.tokens`, `tokenizer.ggml.scores`,
//! `tokenizer.ggml.token_type`, `tokenizer.ggml.merges`, na `model` (aina).
//! Tunaunga mkono:
//!   - **SPM** (Llama 2, SmolLM, TinyLlama, Mistral): `▁` (U+2581) ni nafasi;
//!     tokeni huchaguliwa kwa score (kama llama.cpp).
//!   - **BPE** (GPT-2, Llama 3, Qwen2): merges kwa rank; byte-level.

use crate::gguf::GgufFile;
use std::collections::HashMap;

pub const TT_NORMAL: i64 = 1;
pub const TT_UNKNOWN: i64 = 2;
pub const TT_CONTROL: i64 = 3;
pub const TT_USER_DEFINED: i64 = 4;
pub const TT_BYTE: i64 = 6;

/// Alama ya nafasi ya SPM (U+2581 LOWER ONE EIGHTH BLOCK).
const SPM_SPACE: char = '\u{2581}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokModel {
    Spm,
    Bpe,
}

pub struct Tokenizer {
    pub model: TokModel,
    pub token_to_id: HashMap<Vec<u8>, u32>,
    pub id_to_token: Vec<Vec<u8>>,
    pub scores: Vec<f32>,
    pub types: Vec<i64>,
    pub merges: HashMap<(Vec<u8>, Vec<u8>), usize>,
    pub bos_id: Option<u32>,
    pub eos_id: Option<u32>,
    pub unk_id: u32,
    pub add_bos: bool,
    pub add_space_prefix: bool,
    pub special: HashMap<String, u32>,
}

impl Tokenizer {
    pub fn from_gguf(g: &GgufFile) -> Result<Self, String> {
        let toks = g
            .meta_array("tokenizer.ggml.tokens")
            .ok_or("GGUF haina tokenizer.ggml.tokens")?;
        let n = toks.len();
        let mut id_to_token = Vec::with_capacity(n);
        let mut token_to_id = HashMap::with_capacity(n);
        for (i, t) in toks.iter().enumerate() {
            let bytes = t.as_str().ok_or("tokeni si string")?.as_bytes().to_vec();
            id_to_token.push(bytes.clone());
            token_to_id.entry(bytes).or_insert(i as u32);
        }

        let scores = g
            .meta_array("tokenizer.ggml.scores")
            .map(|a| a.iter().filter_map(|v| v.as_f32()).collect::<Vec<f32>>())
            .unwrap_or_else(|| vec![0f32; n]);
        let types = g
            .meta_array("tokenizer.ggml.token_type")
            .map(|a| a.iter().filter_map(|v| v.as_i64()).collect::<Vec<i64>>())
            .unwrap_or_else(|| vec![TT_NORMAL; n]);

        let mut merges = HashMap::new();
        if let Some(m) = g.meta_array("tokenizer.ggml.merges") {
            for (rank, pair) in m.iter().enumerate() {
                if let Some(s) = pair.as_str() {
                    if let Some((a, b)) = s.split_once(' ') {
                        merges.insert((a.as_bytes().to_vec(), b.as_bytes().to_vec()), rank);
                    }
                }
            }
        }

        let model = match g.meta_str("tokenizer.ggml.model") {
            Some("gpt2") => TokModel::Bpe,
            Some("llama") => TokModel::Spm,
            _ => {
                if merges.is_empty() {
                    TokModel::Spm
                } else {
                    TokModel::Bpe
                }
            }
        };

        let bos_id = g.meta_u64("tokenizer.ggml.bos_token_id").map(|v| v as u32);
        let eos_id = g.meta_u64("tokenizer.ggml.eos_token_id").map(|v| v as u32);
        let unk_id = g
            .meta_u64("tokenizer.ggml.unknown_token_id")
            .map(|v| v as u32)
            .or_else(|| {
                types
                    .iter()
                    .position(|&t| t == TT_UNKNOWN)
                    .map(|i| i as u32)
            })
            .unwrap_or(0);
        let add_bos = g
            .meta_bool("tokenizer.ggml.add_bos_token")
            .unwrap_or(bos_id.is_some());
        let add_space_prefix = g
            .meta_bool("tokenizer.ggml.add_space_prefix")
            .unwrap_or(true);

        let mut special = HashMap::new();
        for name in [
            "<|im_start|>",
            "<|im_end|>",
            "<|begin_of_text|>",
            "<|end_of_text|>",
            "<|eot_id|>",
            "<s>",
            "</s>",
            "[INST]",
            "[/INST]",
        ] {
            if let Some(&id) = token_to_id.get(name.as_bytes()) {
                special.insert(name.to_string(), id);
            }
        }

        Ok(Tokenizer {
            model,
            token_to_id,
            id_to_token,
            scores,
            types,
            merges,
            bos_id,
            eos_id,
            unk_id,
            add_bos,
            add_space_prefix,
            special,
        })
    }

    pub fn vocab_size(&self) -> usize {
        self.id_to_token.len()
    }

    pub fn is_eos(&self, id: u32) -> bool {
        self.eos_id == Some(id)
    }

    /// Encode maandishi → ids (hutambua special tokens kama token MOJA).
    pub fn encode(&self, text: &str, add_bos: bool) -> Vec<u32> {
        let mut ids = Vec::new();
        if add_bos && self.add_bos {
            if let Some(b) = self.bos_id {
                ids.push(b);
            }
        }
        for (is_special, part) in self.split_specials(text) {
            if is_special {
                ids.push(self.token_to_id[part.as_bytes()]);
                continue;
            }
            match self.model {
                TokModel::Spm => ids.extend(self.encode_spm(&part)),
                TokModel::Bpe => ids.extend(self.encode_bpe(&part)),
            }
        }
        ids
    }

    /// Gawanya maandishi: (ni_special, kipande). Longest-match kwa special tokens.
    fn split_specials(&self, text: &str) -> Vec<(bool, String)> {
        let mut names: Vec<&String> = self.special.keys().collect();
        names.sort_by_key(|n| std::cmp::Reverse(n.len()));
        let mut out = Vec::new();
        let mut buf = String::new();
        let mut i = 0;
        let bytes = text.as_bytes();
        'outer: while i < text.len() {
            if text.is_char_boundary(i) {
                for name in &names {
                    if bytes[i..].starts_with(name.as_bytes()) {
                        if !buf.is_empty() {
                            out.push((false, std::mem::take(&mut buf)));
                        }
                        out.push((true, name.to_string()));
                        i += name.len();
                        continue 'outer;
                    }
                }
            }
            let ch = text[i..].chars().next().unwrap();
            buf.push(ch);
            i += ch.len_utf8();
        }
        if !buf.is_empty() {
            out.push((false, buf));
        }
        out
    }

    // ----------------------------------------------------------------- SPM
    fn encode_spm(&self, text: &str) -> Vec<u32> {
        let mut out = Vec::new();
        // SPM: nafasi zote → ▁, kisha gawanya kwa ▁ (nafuata llama.cpp).
        let prepared: String = if self.add_space_prefix {
            format!("{SPM_SPACE}{}", text.replace(' ', &SPM_SPACE.to_string()))
        } else {
            text.replace(' ', &SPM_SPACE.to_string())
        };
        for piece in prepared.split(SPM_SPACE) {
            if piece.is_empty() {
                continue;
            }
            let with_space = format!("{SPM_SPACE}{piece}");
            out.extend(self.spm_tokenize(&with_space));
        }
        // Kama add_space_prefix, piece ya kwanza ilianza na ▁ (kawaida ni <s>).
        out
    }

    /// SPM merge: chagua pair yenye score KUBWA (kama llama.cpp).
    fn spm_tokenize(&self, piece: &str) -> Vec<u32> {
        if piece.is_empty() {
            return vec![];
        }
        if let Some(&id) = self.token_to_id.get(piece.as_bytes()) {
            return vec![id];
        }
        let chars: Vec<String> = piece.chars().map(|c| c.to_string()).collect();
        let mut parts: Vec<String> = chars;
        loop {
            let mut best: Option<(usize, f32, u32)> = None;
            for i in 0..parts.len().saturating_sub(1) {
                let cand = format!("{}{}", parts[i], parts[i + 1]);
                if let Some(&id) = self.token_to_id.get(cand.as_bytes()) {
                    let score = self.scores.get(id as usize).copied().unwrap_or(0.0);
                    match best {
                        Some((_, bs, _)) if bs >= score => {}
                        _ => best = Some((i, score, id)),
                    }
                }
            }
            let Some((i, _, _)) = best else { break };
            let merged = format!("{}{}", parts[i], parts[i + 1]);
            parts.splice(i..i + 2, [merged]);
        }
        let mut ids = Vec::new();
        for p in parts {
            if let Some(&id) = self.token_to_id.get(p.as_bytes()) {
                ids.push(id);
            } else {
                // Fallback: byte tokens <0xXX>
                for b in p.as_bytes() {
                    let bt = format!("<0x{:02X}>", b).into_bytes();
                    ids.push(*self.token_to_id.get(&bt).unwrap_or(&self.unk_id));
                }
            }
        }
        ids
    }

    // ----------------------------------------------------------------- BPE
    fn encode_bpe(&self, text: &str) -> Vec<u32> {
        let mut out = Vec::new();
        for piece in pretokenize(text) {
            out.extend(self.bpe(&piece));
        }
        out
    }

    /// BPE ya byte-level (GPT-2/Qwen/Llama-3): bytes → chars za unicode
    /// (`bytes_to_unicode`), merges kwa rank, kisha lookup kwa UTF-8 ya chars.
    fn bpe(&self, piece: &[u8]) -> Vec<u32> {
        if piece.is_empty() {
            return vec![];
        }
        let mapped: Vec<String> = piece
            .iter()
            .map(|&b| BYTE_TO_CHAR[b as usize].to_string())
            .collect();
        // Token moja kama ipo.
        let joined: String = mapped.concat();
        if let Some(&id) = self.token_to_id.get(joined.as_bytes()) {
            return vec![id];
        }
        let mut parts = mapped;
        loop {
            let mut best: Option<(usize, usize)> = None;
            for i in 0..parts.len().saturating_sub(1) {
                let key = (parts[i].as_bytes().to_vec(), parts[i + 1].as_bytes().to_vec());
                if let Some(&rank) = self.merges.get(&key) {
                    match best {
                        Some((_, br)) if br <= rank => {}
                        _ => best = Some((i, rank)),
                    }
                }
            }
            let Some((i, _)) = best else { break };
            let merged = format!("{}{}", parts[i], parts[i + 1]);
            parts.splice(i..i + 2, [merged]);
        }
        let mut ids = Vec::new();
        for p in parts {
            if let Some(&id) = self.token_to_id.get(p.as_bytes()) {
                ids.push(id);
            } else {
                // Fallback: chars moja moja (byte tokens).
                for c in p.chars() {
                    let b = CHAR_TO_BYTE[&c];
                    let bt = format!("<0x{:02X}>", b).into_bytes();
                    if let Some(&id) = self.token_to_id.get(&bt) {
                        ids.push(id);
                    } else {
                        ids.push(self.unk_id);
                    }
                }
            }
        }
        if ids.is_empty() {
            ids.push(self.unk_id);
        }
        ids
    }

    // ----------------------------------------------------------------- decode
    pub fn token_bytes(&self, id: u32) -> &[u8] {
        self.id_to_token
            .get(id as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Decode id MOJA → maandishi (kwa streaming). Hugeuza ▁ → nafasi (SPM) au
    /// byte-level chars → bytes (BPE).
    pub fn decode_stream_piece(&self, id: u32) -> String {
        if self.types.get(id as usize) == Some(&TT_CONTROL) {
            return String::new();
        }
        let raw = self.token_bytes(id);
        let s = String::from_utf8_lossy(raw).to_string();
        match self.model {
            TokModel::Spm => s.replace(SPM_SPACE, " "),
            TokModel::Bpe => unmap_byte_chars(&s),
        }
    }

    /// Decode ids → maandishi.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut out = String::new();
        for &id in ids {
            out.push_str(&self.decode_stream_piece(id));
        }
        out
    }

    /// Tengeneza prompt kwa mtindo wa chat (ChatML / Llama-3 / fallback).
    pub fn chat_prompt(&self, system: &str, user: &str, template: &str) -> String {
        match template {
            "chatml" => {
                let mut s = String::new();
                if !system.is_empty() {
                    s.push_str(&format!("<|im_start|>system\n{system}<|im_end|>\n"));
                }
                s.push_str(&format!(
                    "<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
                ));
                s
            }
            "llama3" => {
                let mut s = String::from("<|begin_of_text|>");
                if !system.is_empty() {
                    s.push_str(&format!(
                        "<|start_header_id|>system<|end_header_id|>\n\n{system}<|eot_id|>"
                    ));
                }
                s.push_str(&format!(
                    "<|start_header_id|>user<|end_header_id|>\n\n{user}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
                ));
                s
            }
            _ => {
                if system.is_empty() {
                    user.to_string()
                } else {
                    format!("{system}\n\n{user}")
                }
            }
        }
    }

    /// Template inayofaa (kwa kuzingatia special tokens zilizopo).
    pub fn detect_template(&self) -> &'static str {
        if self.special.contains_key("<|im_start|>") {
            "chatml"
        } else if self.special.contains_key("<|begin_of_text|>") {
            "llama3"
        } else {
            "plain"
        }
    }
}

/// Jedwali la GPT-2 `bytes_to_unicode`: byte (0..256) → char ya unicode.
/// Hii ndiyo inayofanya BPE ya byte-level (Qwen/GPT-2/Llama-3) ifanye kazi.
pub static BYTE_TO_CHAR: std::sync::LazyLock<[char; 256]> = std::sync::LazyLock::new(|| {
    let mut table = ['\0'; 256];
    let mut n = 0u32;
    for b in 0u16..256 {
        let b = b as u8;
        let printable = (33..=126).contains(&b) || (161..=172).contains(&b) || (174..=255).contains(&b);
        if printable {
            table[b as usize] = char::from_u32(b as u32).unwrap();
        } else {
            table[b as usize] = char::from_u32(256 + n).unwrap();
            n += 1;
        }
    }
    table
});

/// Ramani ya kinyume (char → byte) kwa decode/fallback.
pub static CHAR_TO_BYTE: std::sync::LazyLock<HashMap<char, u8>> = std::sync::LazyLock::new(|| {
    let mut m = HashMap::new();
    for (b, &c) in BYTE_TO_CHAR.iter().enumerate() {
        m.insert(c, b as u8);
    }
    m
});

/// Rudisha maandishi kwa kugeuza byte-level chars → bytes halisi (Ġ → nafasi).
pub fn unmap_byte_chars(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    for c in s.chars() {
        match CHAR_TO_BYTE.get(&c) {
            Some(&b) => bytes.push(b),
            None => bytes.extend_from_slice(c.to_string().as_bytes()),
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

/// Pre-tokenization ya GPT-2 (byte-level) kwa BPE.
pub fn pretokenize(text: &str) -> Vec<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            cur.push(b);
            i += 1;
        } else if b.is_ascii_alphanumeric() || b == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let mut seg = cur;
            seg.extend_from_slice(&bytes[start..i]);
            cur = seg;
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push(vec![b]);
            i += 1;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretokenize_hugawanya_sawa() {
        let p = pretokenize("hello world!");
        assert_eq!(p, vec![b"hello".to_vec(), b" world".to_vec(), b"!".to_vec()]);
    }

    #[test]
    fn decode_huruka_control_na_hubadilisha_spm() {
        let tok = Tokenizer {
            model: TokModel::Spm,
            token_to_id: HashMap::new(),
            id_to_token: vec!["\u{2581}hi".as_bytes().to_vec(), b"<eos>".to_vec(), b"!".to_vec()],
            scores: vec![0.0; 3],
            types: vec![TT_NORMAL, TT_CONTROL, TT_NORMAL],
            merges: HashMap::new(),
            bos_id: None,
            eos_id: Some(1),
            unk_id: 0,
            add_bos: false,
            add_space_prefix: true,
            special: HashMap::new(),
        };
        assert_eq!(tok.decode(&[0, 1, 2]), " hi!");
        assert!(tok.is_eos(1));
    }

    #[test]
    fn spm_tokenize_huchagua_score_kubwa() {
        // vocab: "a", "b", "ab"(score 5), "▁ab"(score 1)
        let mut t2i = HashMap::new();
        let i2t = vec![
            b"a".to_vec(),
            b"b".to_vec(),
            b"ab".to_vec(),
            "\u{2581}ab".as_bytes().to_vec(),
        ];
        for (i, t) in i2t.iter().enumerate() {
            t2i.insert(t.clone(), i as u32);
        }
        let tok = Tokenizer {
            model: TokModel::Spm,
            token_to_id: t2i,
            id_to_token: i2t,
            scores: vec![0.0, 0.0, 5.0, 1.0],
            types: vec![TT_NORMAL; 4],
            merges: HashMap::new(),
            bos_id: None,
            eos_id: None,
            unk_id: 0,
            add_bos: false,
            add_space_prefix: false,
            special: HashMap::new(),
        };
        // "ab" → tokeni 3 (▁ab) kwa sababu ya nafasi ya mwanzo
        let ids = tok.encode("ab", false);
        assert_eq!(ids, vec![3]);
    }
}
