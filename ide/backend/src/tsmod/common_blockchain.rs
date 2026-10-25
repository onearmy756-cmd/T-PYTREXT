//! common_blockchain.rs — msingi wa SEHEMU 19 (Rust Blockchain, modules 468-500).
//!
//! Kina blockchain HALISI: vitalu vyenye merkle root, uthibitishaji wa mnyororo,
//! consensus (PoS + PBFT quorum), VM ndogo ya kuamua (deterministic, Wasm-style)
//! kwa smart contracts, multi-signature (Lamport OTS, hash-based/post-quantum),
//! na leja ya tokeni.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// SHA-256 → hex.
pub fn sha_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash ya node mbili za merkle.
fn hash_pair(a: &str, b: &str) -> String {
    sha_hex(format!("{a}{b}").as_bytes())
}

/// Merkle root kutoka hashi za miamala.
pub fn merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return sha_hex(b"empty");
    }
    let mut level: Vec<String> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i < level.len() {
            let a = &level[i];
            let b = if i + 1 < level.len() { &level[i + 1] } else { &level[i] };
            next.push(hash_pair(a, b));
            i += 2;
        }
        level = next;
    }
    level[0].clone()
}

/// Muamala.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tx {
    pub from: String,
    pub to: String,
    pub amount: i64,
    pub data: String,
    pub nonce: u64,
}

impl Tx {
    pub fn hash(&self) -> String {
        sha_hex(format!("{}|{}|{}|{}|{}", self.from, self.to, self.amount, self.data, self.nonce).as_bytes())
    }
}

/// Kitalu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub prev_hash: String,
    pub timestamp: i64,
    pub proposer: String,
    pub txs: Vec<Tx>,
    pub merkle: String,
    pub hash: String,
}

impl Block {
    fn compute_hash(index: u64, prev: &str, ts: i64, proposer: &str, merkle: &str) -> String {
        sha_hex(format!("{index}|{prev}|{ts}|{proposer}|{merkle}").as_bytes())
    }
}

/// Mnyororo wa vitalu.
#[derive(Debug, Clone, Default)]
pub struct Chain {
    pub blocks: Vec<Block>,
    pub validators: HashMap<String, u64>, // validator → stake
}

impl Chain {
    /// Genesis block.
    pub fn new(genesis_proposer: &str) -> Self {
        let ts = 1_700_000_000;
        let merkle = merkle_root(&[]);
        let hash = Block::compute_hash(0, "0", ts, genesis_proposer, &merkle);
        Chain {
            blocks: vec![Block { index: 0, prev_hash: "0".into(), timestamp: ts, proposer: genesis_proposer.into(), txs: vec![], merkle, hash }],
            validators: HashMap::new(),
        }
    }

    pub fn height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn tip(&self) -> &Block {
        self.blocks.last().unwrap()
    }

    pub fn add_validator(&mut self, id: &str, stake: u64) {
        self.validators.insert(id.to_string(), stake);
    }

    /// Ongeza kitalu kipya (inathibitisha prev hash + merkle).
    pub fn add_block(&mut self, proposer: &str, txs: Vec<Tx>, timestamp: i64) -> Result<&Block, String> {
        let prev = self.tip();
        let leaves: Vec<String> = txs.iter().map(|t| t.hash()).collect();
        let merkle = merkle_root(&leaves);
        let index = prev.index + 1;
        let hash = Block::compute_hash(index, &prev.hash, timestamp, proposer, &merkle);
        self.blocks.push(Block { index, prev_hash: prev.hash.clone(), timestamp, proposer: proposer.into(), txs, merkle, hash });
        Ok(self.blocks.last().unwrap())
    }

    /// Thibitisha mnyororo wote.
    pub fn validate(&self) -> bool {
        for (i, b) in self.blocks.iter().enumerate() {
            let leaves: Vec<String> = b.txs.iter().map(|t| t.hash()).collect();
            if merkle_root(&leaves) != b.merkle {
                return false;
            }
            let expect = Block::compute_hash(b.index, &b.prev_hash, b.timestamp, &b.proposer, &b.merkle);
            if expect != b.hash {
                return false;
            }
            if i > 0 && b.prev_hash != self.blocks[i - 1].hash {
                return false;
            }
        }
        true
    }

    pub fn find_tx(&self, hash: &str) -> Option<(u64, usize)> {
        for b in &self.blocks {
            for (j, t) in b.txs.iter().enumerate() {
                if t.hash() == hash {
                    return Some((b.index, j));
                }
            }
        }
        None
    }
}

// ---------- Consensus ----------

/// PoS: chagua proposer kwa uzito wa stake (deterministic kwa seed).
pub fn pos_select(stakes: &HashMap<String, u64>, seed: u64) -> Option<String> {
    let total: u64 = stakes.values().sum();
    if total == 0 {
        return None;
    }
    let target = seed % total;
    let mut acc = 0u64;
    let mut keys: Vec<&String> = stakes.keys().collect();
    keys.sort();
    for k in keys {
        acc += stakes[k];
        if target < acc {
            return Some(k.clone());
        }
    }
    None
}

/// PBFT: 2/3 quorum (f = (n-1)/3).
pub fn pbft_quorum(votes: usize, total: usize) -> bool {
    if total == 0 {
        return false;
    }
    votes * 3 >= total * 2 + 1
}

/// Idadi ya hitilafu zinazovumiliwa.
pub fn byzantine_tolerance(total: usize) -> usize {
    total.saturating_sub(1) / 3
}

// ---------- VM ndogo (deterministic, Wasm-style) ----------

/// Opcodes za VM.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Push(i64),
    Add,
    Sub,
    Mul,
    Lt,
    Gt,
    Eq,
    Store(String),
    Load(String),
    Transfer(String, String),
    JumpIfZero(usize),
    Halt,
}

/// Matokeo ya utekelezaji.
#[derive(Debug, Clone)]
pub struct VmOutcome {
    pub stack_top: i64,
    pub storage: HashMap<String, i64>,
    pub transfers: Vec<(String, String, i64)>,
    pub gas_used: u64,
    pub halted: bool,
}

/// Tekeleza program (deterministic; gas inakaguliwa).
pub fn vm_execute(program: &[Op], gas_limit: u64) -> Result<VmOutcome, String> {
    let mut stack: Vec<i64> = Vec::new();
    let mut storage: HashMap<String, i64> = HashMap::new();
    let mut transfers: Vec<(String, String, i64)> = Vec::new();
    let mut gas = 0u64;
    let mut pc = 0usize;
    let mut halted = false;
    while pc < program.len() {
        if gas >= gas_limit {
            return Err("gas imeisha".into());
        }
        gas += 1;
        match &program[pc] {
            Op::Push(v) => stack.push(*v),
            Op::Add => bin(&mut stack, |a, b| a + b)?,
            Op::Sub => bin(&mut stack, |a, b| a - b)?,
            Op::Mul => bin(&mut stack, |a, b| a * b)?,
            Op::Lt => bin(&mut stack, |a, b| (a < b) as i64)?,
            Op::Gt => bin(&mut stack, |a, b| (a > b) as i64)?,
            Op::Eq => bin(&mut stack, |a, b| (a == b) as i64)?,
            Op::Store(k) => {
                let v = stack.pop().ok_or("stack tupu")?;
                storage.insert(k.clone(), v);
            }
            Op::Load(k) => stack.push(*storage.get(k).unwrap_or(&0)),
            Op::Transfer(from, to) => {
                let amt = stack.pop().ok_or("stack tupu")?;
                transfers.push((from.clone(), to.clone(), amt));
            }
            Op::JumpIfZero(t) => {
                let v = stack.pop().ok_or("stack tupu")?;
                if v == 0 {
                    pc = *t;
                    continue;
                }
            }
            Op::Halt => {
                halted = true;
                break;
            }
        }
        pc += 1;
    }
    Ok(VmOutcome { stack_top: stack.last().copied().unwrap_or(0), storage, transfers, gas_used: gas, halted })
}

fn bin(stack: &mut Vec<i64>, f: impl Fn(i64, i64) -> i64) -> Result<(), String> {
    let b = stack.pop().ok_or("stack tupu")?;
    let a = stack.pop().ok_or("stack tupu")?;
    stack.push(f(a, b));
    Ok(())
}

// ---------- Multi-signature (Lamport OTS) ----------

/// Sahihi ya Lamport (reuse ya m328): (public, signature).
pub fn lamport_sign_msg(sk: &super::m328_future_proof_architecture::LamportSecret, msg: &[u8]) -> Vec<[u8; 32]> {
    super::m328_future_proof_architecture::lamport_sign(sk, msg)
}

pub fn lamport_verify_msg(pk: &super::m328_future_proof_architecture::LamportPublic, msg: &[u8], sig: &[[u8; 32]]) -> bool {
    super::m328_future_proof_architecture::lamport_verify(pk, msg, sig)
}

/// Thibitisha m-of-n multisig (kila signer ana ufunguo wake).
pub fn multisig_verify(
    signers: &[&super::m328_future_proof_architecture::LamportPublic],
    msg: &[u8],
    sigs: &[Vec<[u8; 32]>],
    threshold: usize,
) -> bool {
    let mut ok = 0;
    for (i, sig) in sigs.iter().enumerate() {
        if i < signers.len() && lamport_verify_msg(signers[i], msg, sig) {
            ok += 1;
        }
    }
    ok >= threshold
}

// ---------- Token ledger ----------

/// Leja ya tokeni.
#[derive(Debug, Clone, Default)]
pub struct TokenLedger {
    pub balances: HashMap<String, i64>,
}

impl TokenLedger {
    pub fn mint(&mut self, who: &str, amount: i64) {
        *self.balances.entry(who.to_string()).or_insert(0) += amount;
    }

    pub fn balance(&self, who: &str) -> i64 {
        *self.balances.get(who).unwrap_or(&0)
    }

    pub fn transfer(&mut self, from: &str, to: &str, amount: i64) -> Result<(), String> {
        if amount <= 0 {
            return Err("kiasi lazima kiwe > 0".into());
        }
        let bal = self.balance(from);
        if bal < amount {
            return Err(format!("salio la {from} haitoshi ({bal} < {amount})"));
        }
        *self.balances.get_mut(from).unwrap() -= amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;
        Ok(())
    }
}

/// Tengeneza muamala (helper).
pub fn tx(from: &str, to: &str, amount: i64, data: &str, nonce: u64) -> Tx {
    Tx { from: from.into(), to: to.into(), amount, data: data.into(), nonce }
}

/// Rekodi kwenye blockchain na kurudisha hash ya kitalu.
pub fn anchor(chain: &mut Chain, proposer: &str, entries: &[(&str, &str)], ts: i64) -> Result<String, String> {
    let txs: Vec<Tx> = entries.iter().enumerate().map(|(i, (k, v))| tx("anchor", k, 0, v, i as u64)).collect();
    let b = chain.add_block(proposer, txs, ts)?;
    Ok(b.hash.clone())
}

/// Kigezo cha JSON kwa block.
pub fn block_json(b: &Block) -> Value {
    json!({ "index": b.index, "hash": b.hash, "prev_hash": b.prev_hash, "merkle": b.merkle, "proposer": b.proposer, "txs": b.txs.len(), "timestamp": b.timestamp })
}

// ---------- Uendelevu (SQLite) ----------

/// Unda jedwali la blockchain (mnyororo mmoja kwa jina).
pub async fn init_chain_tables(db: &sqlx::SqlitePool) {
    let _ = sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS ts_chain (
            chain TEXT NOT NULL,
            idx INTEGER NOT NULL,
            prev_hash TEXT NOT NULL,
            hash TEXT NOT NULL,
            merkle TEXT NOT NULL,
            proposer TEXT NOT NULL,
            ts INTEGER NOT NULL,
            txs TEXT NOT NULL,
            PRIMARY KEY (chain, idx)
        )"#,
    )
    .execute(db)
    .await;
}

/// Pakia mnyororo kutoka DB (au genesis mpya).
pub async fn load_chain(db: &sqlx::SqlitePool, name: &str, genesis_proposer: &str) -> Chain {
    let rows: Vec<(i64, String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT idx, prev_hash, hash, merkle, proposer, ts, txs FROM ts_chain WHERE chain=?1 ORDER BY idx ASC",
    )
    .bind(name)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return Chain::new(genesis_proposer);
    }
    let blocks = rows
        .iter()
        .map(|(idx, prev, hash, merkle, proposer, ts, txs)| {
            let txs: Vec<Tx> = serde_json::from_str(txs).unwrap_or_default();
            Block { index: *idx as u64, prev_hash: prev.clone(), timestamp: *ts, proposer: proposer.clone(), txs, merkle: merkle.clone(), hash: hash.clone() }
        })
        .collect();
    Chain { blocks, validators: HashMap::new() }
}

/// Hifadhi kitalu kwenye DB.
pub async fn persist_block(db: &sqlx::SqlitePool, name: &str, b: &Block) -> Result<(), String> {
    let txs = serde_json::to_string(&b.txs).map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT OR REPLACE INTO ts_chain (chain, idx, prev_hash, hash, merkle, proposer, ts, txs) VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(name).bind(b.index as i64).bind(&b.prev_hash).bind(&b.hash).bind(&b.merkle)
    .bind(&b.proposer).bind(b.timestamp).bind(&txs)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Hesabu ya vitalu kwenye mnyororo wa DB.
pub async fn chain_count(db: &sqlx::SqlitePool, name: &str) -> i64 {
    let row: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM ts_chain WHERE chain=?1")
        .bind(name)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    row.map(|(c,)| c).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_inathibitika() {
        let mut c = Chain::new("genesis");
        c.add_block("v1", vec![tx("a", "b", 10, "x", 1)], 1_700_000_001).unwrap();
        c.add_block("v2", vec![tx("b", "c", 5, "y", 2)], 1_700_000_002).unwrap();
        assert_eq!(c.height(), 3);
        assert!(c.validate());
    }

    #[test]
    fn chain_imebadilishwa_inakataliwa() {
        let mut c = Chain::new("genesis");
        c.add_block("v1", vec![tx("a", "b", 10, "x", 1)], 1_700_000_001).unwrap();
        c.blocks[1].txs[0].amount = 999;
        assert!(!c.validate());
    }

    #[test]
    fn pos_na_pbft() {
        let mut s = HashMap::new();
        s.insert("a".into(), 60);
        s.insert("b".into(), 40);
        assert_eq!(pos_select(&s, 10).as_deref(), Some("a"));
        assert_eq!(pos_select(&s, 70).as_deref(), Some("b"));
        assert!(pbft_quorum(4, 5));
        assert!(!pbft_quorum(2, 5));
        assert_eq!(byzantine_tolerance(7), 2);
    }

    #[test]
    fn vm_inafanya_kazi() {
        // if 3 > 2 { store "ok" = 1 } transfer(a,b,3)
        let prog = vec![
            Op::Push(3), Op::Push(2), Op::Gt,
            Op::JumpIfZero(7),
            Op::Push(1), Op::Store("ok".into()),
            Op::Push(3), Op::Transfer("a".into(), "b".into()),
            Op::Halt,
        ];
        let out = vm_execute(&prog, 100).unwrap();
        assert_eq!(out.storage.get("ok"), Some(&1));
        assert_eq!(out.transfers, vec![("a".into(), "b".into(), 3)]);
        assert!(out.halted);
    }

    #[test]
    fn token_ledger() {
        let mut l = TokenLedger::default();
        l.mint("a", 100);
        assert!(l.transfer("a", "b", 30).is_ok());
        assert_eq!(l.balance("b"), 30);
        assert!(l.transfer("a", "c", 1000).is_err());
    }

    #[test]
    fn multisig_3_of_5() {
        use super::super::m328_future_proof_architecture::{lamport_keypair, lamport_sign, lamport_verify};
        let keys: Vec<_> = (0..5).map(|_| lamport_keypair()).collect();
        let msg = b"release escrow";
        let sigs: Vec<_> = keys.iter().take(3).map(|(sk, _)| lamport_sign(sk, msg)).collect();
        let pub_refs: Vec<&_> = keys.iter().map(|(_, p)| p).collect();
        assert!(multisig_verify(&pub_refs, msg, &sigs, 3));
        assert!(!multisig_verify(&pub_refs, msg, &sigs, 4));
        // Sahihi bandia inakataliwa.
        let wrong = lamport_sign(&keys[0].0, b"tofauti");
        assert!(!lamport_verify(&keys[0].1, msg, &wrong));
    }
}
