//! gguf.rs — SOMA FAILI ZA GGUF (moja kwa moja, bila Python).
//!
//! GGUF ni fomati ya llama.cpp: kichwa (magic "GGUF" + toleo + hesabu), jozi za
//! metadata (key → thamani yenye aina), orodha ya tensor (jina, dimensheni, aina,
//! offset), kisha padding hadi alignment (default 32) na data ya tensor.
//!
//! Hii ni parser KAMILI ya toleo la 3 (na inakubali 2). Data hutolewa kwa
//! `memmap2` — hakuna kunakili GB kadhaa kwenye RAM; tensor ni (offset, len)
//! ndani ya mmap.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Aina za thamani za metadata za GGUF (spec v3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaType {
    Uint8 = 0,
    Int8 = 1,
    Uint16 = 2,
    Int16 = 3,
    Uint32 = 4,
    Int32 = 5,
    Float32 = 6,
    Bool = 7,
    String = 8,
    Array = 9,
    Uint64 = 10,
    Int64 = 11,
    Float64 = 12,
}

impl MetaType {
    fn from_u32(v: u32) -> Result<Self, String> {
        Ok(match v {
            0 => MetaType::Uint8,
            1 => MetaType::Int8,
            2 => MetaType::Uint16,
            3 => MetaType::Int16,
            4 => MetaType::Uint32,
            5 => MetaType::Int32,
            6 => MetaType::Float32,
            7 => MetaType::Bool,
            8 => MetaType::String,
            9 => MetaType::Array,
            10 => MetaType::Uint64,
            11 => MetaType::Int64,
            12 => MetaType::Float64,
            other => return Err(format!("MetaType haijulikani: {other}")),
        })
    }
}

/// Thamani ya metadata (zote zinazotumika kwenye llama/qwen GGUF).
#[derive(Debug, Clone, PartialEq)]
pub enum MetaValue {
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(String),
    Array(Vec<MetaValue>),
}

impl MetaValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            MetaValue::U64(v) => Some(*v),
            MetaValue::I64(v) => Some(*v as u64),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            MetaValue::U64(v) => Some(*v as i64),
            MetaValue::I64(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MetaValue::F64(v) => Some(*v),
            MetaValue::U64(v) => Some(*v as f64),
            MetaValue::I64(v) => Some(*v as f64),
            _ => None,
        }
    }
    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|v| v as f32)
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MetaValue::Bool(v) => Some(*v),
            MetaValue::U64(v) => Some(*v != 0),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MetaValue::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[MetaValue]> {
        match self {
            MetaValue::Array(a) => Some(a),
            _ => None,
        }
    }
}

/// Aina za tensor za GGML (quantization). Tunaweka IDs za msingi zinazotumika
/// kwenye modeli za GGUF; nyingine zinaripotiwa kama hazijaungwa mkono.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgmlType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2K = 10,
    Q3K = 11,
    Q4K = 12,
    Q5K = 13,
    Q6K = 14,
    Q8K = 15,
    Bf16 = 30,
}

impl GgmlType {
    pub fn from_u32(v: u32) -> Option<Self> {
        Some(match v {
            0 => GgmlType::F32,
            1 => GgmlType::F16,
            2 => GgmlType::Q4_0,
            3 => GgmlType::Q4_1,
            6 => GgmlType::Q5_0,
            7 => GgmlType::Q5_1,
            8 => GgmlType::Q8_0,
            9 => GgmlType::Q8_1,
            10 => GgmlType::Q2K,
            11 => GgmlType::Q3K,
            12 => GgmlType::Q4K,
            13 => GgmlType::Q5K,
            14 => GgmlType::Q6K,
            15 => GgmlType::Q8K,
            30 => GgmlType::Bf16,
            _ => return None,
        })
    }

    /// Ukubwa wa block (vipengele kwa block). Quantized zote za K ni 256.
    pub fn block_size(self) -> usize {
        match self {
            GgmlType::F32 | GgmlType::F16 | GgmlType::Bf16 => 1,
            GgmlType::Q4_0 | GgmlType::Q4_1 | GgmlType::Q5_0 | GgmlType::Q5_1
            | GgmlType::Q8_0 | GgmlType::Q8_1 => 32,
            GgmlType::Q2K | GgmlType::Q3K | GgmlType::Q4K | GgmlType::Q5K
            | GgmlType::Q6K | GgmlType::Q8K => 256,
        }
    }

    /// Ukubwa wa block kwa bytes (ikijumuisha scales/zeros).
    pub fn type_size(self) -> usize {
        match self {
            GgmlType::F32 => 4,
            GgmlType::F16 => 2,
            GgmlType::Bf16 => 2,
            // f16 scale + 32 × 4-bit
            GgmlType::Q4_0 => 2 + 16,
            // f16 scale + f16 min + 32 × 4-bit
            GgmlType::Q4_1 => 2 + 2 + 16,
            // f16 scale + 4 bytes qh + 16 bytes ql
            GgmlType::Q5_0 => 2 + 4 + 16,
            GgmlType::Q5_1 => 2 + 2 + 4 + 16,
            // f16 scale + 32 × i8
            GgmlType::Q8_0 => 2 + 32,
            // f16 scale + f16 sum + 32 × i8
            GgmlType::Q8_1 => 2 + 2 + 32,
            // K-quants: super-block 256 vipengele (kufuata llama.cpp block_* structs)
            // block_q2_K: scales[16] + qs[64] + d(2) + dmin(2) = 84
            GgmlType::Q2K => 16 + 64 + 2 + 2,
            // block_q3_K: hmask[32] + qs[64] + scales[12] + d(2) = 110
            GgmlType::Q3K => 32 + 64 + 12 + 2,
            // block_q4_K: d(2) + dmin(2) + scales[12] + qs[128] = 144
            GgmlType::Q4K => 2 + 2 + 12 + 128,
            // block_q5_K: d(2) + dmin(2) + scales[12] + qh[32] + ql[128] = 176
            GgmlType::Q5K => 2 + 2 + 12 + 32 + 128,
            // block_q6_K: ql[128] + qh[64] + scales[16] + d(2) = 210
            GgmlType::Q6K => 128 + 64 + 16 + 2,
            // block_q8_K: d(f32=4) + qs[256] + bsums[16×i16=32] = 292
            GgmlType::Q8K => 4 + 256 + 32,
        }
    }

    /// Ukubwa wa bytes kwa `n` vipengele (n LAZIMA igawike kwa block_size).
    pub fn n_bytes_per(self, n: usize) -> usize {
        n / self.block_size() * self.type_size()
    }

    /// Jina la kuonyesha kwa UI.
    pub fn label(self) -> &'static str {
        match self {
            GgmlType::F32 => "F32",
            GgmlType::F16 => "F16",
            GgmlType::Bf16 => "BF16",
            GgmlType::Q4_0 => "Q4_0",
            GgmlType::Q4_1 => "Q4_1",
            GgmlType::Q5_0 => "Q5_0",
            GgmlType::Q5_1 => "Q5_1",
            GgmlType::Q8_0 => "Q8_0",
            GgmlType::Q8_1 => "Q8_1",
            GgmlType::Q2K => "Q2_K",
            GgmlType::Q3K => "Q3_K",
            GgmlType::Q4K => "Q4_K",
            GgmlType::Q5K => "Q5_K",
            GgmlType::Q6K => "Q6_K",
            GgmlType::Q8K => "Q8_K",
        }
    }
}

/// Maelezo ya tensor moja (data yake iko kwenye mmap).
#[derive(Debug, Clone)]
pub struct TensorInfo {
    pub name: String,
    pub dims: Vec<u64>,
    pub ty: GgmlType,
    /// Offset kutoka mwanzo wa eneo la data (baada ya padding ya alignment).
    pub offset: u64,
}

impl TensorInfo {
    pub fn n_elements(&self) -> u64 {
        self.dims.iter().product()
    }
    pub fn n_bytes(&self) -> u64 {
        let n = self.n_elements() as usize;
        let bs = self.ty.block_size();
        (n / bs) as u64 * self.ty.type_size() as u64
    }
}

/// Faili ya GGUF iliyopakiwa (metadata + tensor index + mmap).
pub struct GgufFile {
    path: PathBuf,
    pub version: u32,
    pub metadata: HashMap<String, MetaValue>,
    pub tensors: Vec<TensorInfo>,
    index: HashMap<String, usize>,
    pub alignment: usize,
    data_offset: usize,
    map: memmap2::Mmap,
}

impl GgufFile {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("GGUF haifunguki {}: {e}", path.display()))?;
        let map = unsafe { memmap2::Mmap::map(&file) }
            .map_err(|e| format!("mmap imefeli: {e}"))?;
        let mut r = Reader::new(&map);
        let magic = r.take(4)?;
        if magic != b"GGUF" {
            return Err("si faili ya GGUF (magic si 'GGUF')".into());
        }
        let version = r.u32()?;
        if !(2..=3).contains(&version) {
            return Err(format!("GGUF toleo {version} halijaungwa mkono (2 au 3)"));
        }
        let tensor_count = r.u64()? as usize;
        let kv_count = r.u64()? as usize;

        let mut metadata = HashMap::with_capacity(kv_count);
        for _ in 0..kv_count {
            let key = r.string()?;
            let ty = MetaType::from_u32(r.u32()?)?;
            let val = read_meta(&mut r, ty)?;
            metadata.insert(key, val);
        }

        let mut tensors = Vec::with_capacity(tensor_count);
        for _ in 0..tensor_count {
            let name = r.string()?;
            let n_dims = r.u32()? as usize;
            if n_dims > 4 {
                return Err(format!("tensor '{name}' ina dims {n_dims} (> 4)"));
            }
            let mut dims = Vec::with_capacity(n_dims);
            for _ in 0..n_dims {
                dims.push(r.u64()?);
            }
            let raw_ty = r.u32()?;
            let ty = GgmlType::from_u32(raw_ty)
                .ok_or_else(|| format!("tensor '{name}' ina aina {raw_ty} isiyojulikana"))?;
            let offset = r.u64()?;
            tensors.push(TensorInfo {
                name,
                dims,
                ty,
                offset,
            });
        }

        let alignment = metadata
            .get("general.alignment")
            .and_then(|v| v.as_u64())
            .unwrap_or(32) as usize;
        let pos = r.pos();
        let data_offset = (pos + alignment - 1) / alignment * alignment;

        let index = tensors
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.clone(), i))
            .collect();

        Ok(GgufFile {
            path: path.to_path_buf(),
            version,
            metadata,
            tensors,
            index,
            alignment,
            data_offset,
            map,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Data GHAMBI ya tensor (ndani ya mmap — zero-copy).
    pub fn tensor_bytes(&self, name: &str) -> Result<&[u8], String> {
        let i = *self
            .index
            .get(name)
            .ok_or_else(|| format!("tensor '{name}' haipo"))?;
        let t = &self.tensors[i];
        let start = self.data_offset + t.offset as usize;
        let end = start + t.n_bytes() as usize;
        if end > self.map.len() {
            return Err(format!("tensor '{name}' inaenda nje ya faili"));
        }
        Ok(&self.map[start..end])
    }

    pub fn tensor(&self, name: &str) -> Option<&TensorInfo> {
        self.index.get(name).map(|i| &self.tensors[*i])
    }

    pub fn meta_str(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|v| v.as_str())
    }
    pub fn meta_u64(&self, key: &str) -> Option<u64> {
        self.metadata.get(key).and_then(|v| v.as_u64())
    }
    pub fn meta_f32(&self, key: &str) -> Option<f32> {
        self.metadata.get(key).and_then(|v| v.as_f32())
    }
    pub fn meta_array(&self, key: &str) -> Option<&[MetaValue]> {
        self.metadata.get(key).and_then(|v| v.as_array())
    }
    pub fn meta_bool(&self, key: &str) -> Option<bool> {
        self.metadata.get(key).and_then(|v| v.as_bool())
    }
}

fn read_meta(r: &mut Reader, ty: MetaType) -> Result<MetaValue, String> {
    Ok(match ty {
        MetaType::Uint8 => MetaValue::U64(r.u8()? as u64),
        MetaType::Int8 => MetaValue::I64(r.u8()? as i8 as i64),
        MetaType::Uint16 => MetaValue::U64(r.u16()? as u64),
        MetaType::Int16 => MetaValue::I64(r.u16()? as i16 as i64),
        MetaType::Uint32 => MetaValue::U64(r.u32()? as u64),
        MetaType::Int32 => MetaValue::I64(r.u32()? as i32 as i64),
        MetaType::Uint64 => MetaValue::U64(r.u64()?),
        MetaType::Int64 => MetaValue::I64(r.u64()? as i64),
        MetaType::Float32 => MetaValue::F64(r.f32()? as f64),
        MetaType::Float64 => MetaValue::F64(r.f64()?),
        MetaType::Bool => MetaValue::Bool(r.u8()? != 0),
        MetaType::String => MetaValue::Str(r.string()?),
        MetaType::Array => {
            let elem_ty = MetaType::from_u32(r.u32()?)?;
            let n = r.u64()? as usize;
            // Ulinzi: array kubwa mno kwenye metadata = faili mbovu.
            if n > 16_000_000 {
                return Err(format!("array ya metadata ni kubwa mno: {n}"));
            }
            let mut out = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                out.push(read_meta(r, elem_ty)?);
            }
            MetaValue::Array(out)
        }
    })
}

/// Msomaji mdogo wa little-endian juu ya slice.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }
    fn pos(&self) -> usize {
        self.pos
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or("overflow ya GGUF")?;
        if end > self.buf.len() {
            return Err("GGUF imeisha ghafla (truncated)".into());
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.u64()? as usize;
        if n > 1_000_000 {
            return Err(format!("string ya GGUF ni ndefu mno: {n}"));
        }
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("string si UTF-8: {e}"))
    }
}

/// Tengeneza faili NDOGO ya GGUF kwa majaribio (unit tests — bila kupakua modeli).
pub fn write_test_gguf(
    metadata: &[(&str, MetaValue)],
    tensors: &[(&str, Vec<u64>, GgmlType, Vec<u8>)],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GGUF");
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&(tensors.len() as u64).to_le_bytes());
    out.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
    for (k, v) in metadata {
        write_str(&mut out, k);
        write_meta(&mut out, v);
    }
    // Tensor infos: tunahesabu offset kwa mfululizo (bila alignment kati yao —
    // spec hairuhusu padding kati ya tensors, padding ni mwanzo pekee).
    let mut off = 0u64;
    for (name, dims, ty, data) in tensors {
        write_str(&mut out, name);
        out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
        for d in dims {
            out.extend_from_slice(&d.to_le_bytes());
        }
        out.extend_from_slice(&(*ty as u32).to_le_bytes());
        out.extend_from_slice(&off.to_le_bytes());
        off += data.len() as u64;
    }
    // Padding hadi alignment 32.
    while out.len() % 32 != 0 {
        out.push(0);
    }
    for (_, _, _, data) in tensors {
        out.extend_from_slice(data);
    }
    out
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u64).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn write_meta(out: &mut Vec<u8>, v: &MetaValue) {
    out.extend_from_slice(&meta_tag(v).to_le_bytes());
    write_meta_raw(out, v);
}

fn meta_tag(v: &MetaValue) -> u32 {
    match v {
        MetaValue::U64(_) => 10,
        MetaValue::I64(_) => 11,
        MetaValue::F64(_) => 12,
        MetaValue::Bool(_) => 7,
        MetaValue::Str(_) => 8,
        MetaValue::Array(_) => 9,
    }
}

/// Andika thamani BILA tag (kwa vipengele vya array).
fn write_meta_raw(out: &mut Vec<u8>, v: &MetaValue) {
    match v {
        MetaValue::U64(x) => out.extend_from_slice(&x.to_le_bytes()),
        MetaValue::I64(x) => out.extend_from_slice(&x.to_le_bytes()),
        MetaValue::F64(x) => out.extend_from_slice(&x.to_le_bytes()),
        MetaValue::Bool(b) => out.push(if *b { 1 } else { 0 }),
        MetaValue::Str(s) => write_str(out, s),
        MetaValue::Array(items) => {
            let ty = items.first().map(meta_tag).unwrap_or(10);
            out.extend_from_slice(&ty.to_le_bytes());
            out.extend_from_slice(&(items.len() as u64).to_le_bytes());
            for it in items {
                write_meta_raw(out, it);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Vec<u8> {
        // tensor 1: "a.weight" F32 [4]  = [1,2,3,4]
        let a: Vec<u8> = [1f32, 2.0, 3.0, 4.0]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        write_test_gguf(
            &[
                ("general.architecture", MetaValue::Str("llama".into())),
                ("general.alignment", MetaValue::U64(32)),
                ("llama.block_count", MetaValue::U64(1)),
                ("test.floats", MetaValue::F64(3.5)),
                (
                    "test.arr",
                    MetaValue::Array(vec![MetaValue::U64(1), MetaValue::U64(2)]),
                ),
            ],
            &[("a.weight", vec![4], GgmlType::F32, a)],
        )
    }

    #[test]
    fn somo_gguf_ndogo() {
        let bytes = tiny();
        let dir = std::env::temp_dir().join(format!("fundi_engine_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("tiny.gguf");
        std::fs::write(&p, &bytes).unwrap();
        let g = GgufFile::open(&p).unwrap();
        assert_eq!(g.version, 3);
        assert_eq!(g.meta_str("general.architecture"), Some("llama"));
        assert_eq!(g.meta_u64("llama.block_count"), Some(1));
        assert_eq!(g.meta_f32("test.floats"), Some(3.5));
        assert_eq!(g.meta_array("test.arr").unwrap().len(), 2);
        let t = g.tensor("a.weight").unwrap();
        assert_eq!(t.dims, vec![4]);
        assert_eq!(t.ty, GgmlType::F32);
        assert_eq!(t.n_bytes(), 16);
        let raw = g.tensor_bytes("a.weight").unwrap();
        let vals: Vec<f32> = raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(vals, vec![1.0, 2.0, 3.0, 4.0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gguf_mbovu_inakataliwa() {
        let dir = std::env::temp_dir().join(format!("fundi_engine_bad_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("bad.gguf");
        std::fs::write(&p, b"NOPE____").unwrap();
        let e = GgufFile::open(&p).err().unwrap();
        assert!(e.contains("GGUF"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
