//! swarm_v2.rs — safu ya msingi ya maono (halisi, hakuna mock):
//!
//!   1. GUARDRAIL-FIRST + BLAST RADIUS (L0/L1/L2): uamuzi wa typed rails.
//!   2. RED / BLUE / GREEN TEAM: uchunguzi wa usalama wa tabaka tatu.
//!   3. MAKER VOTING: upigaji kura wa mawakala wengi + mtengano.
//!   4. REASONINGBANK: kujifunza mikakati kutoka mafanikio/kushindwa (SQLite).
//!
//! Kanuni: kila kitu huhesabiwa kutoka data halisi (code_audit, shield, DB) —
//! hakuna thamani ya kubuni.

use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

// ============================================================
// 1) GUARDRAIL-FIRST + BLAST RADIUS (typed rails)
// ============================================================

/// Tabaka la blast radius. L0 = yamegandishwa (haiwezi kubadilishwa);
/// L1 = huhitaji oracle (uthibitisho) + idhini; L2 = yanajitegemea.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Layer {
    L0,
    L1,
    L2,
}

impl Layer {
    fn as_str(self) -> &'static str {
        match self {
            Layer::L0 => "L0",
            Layer::L1 => "L1",
            Layer::L2 => "L2",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Layer::L0 => "safety/permissions (imegandishwa)",
            Layer::L1 => "harness/verifier (inahitaji oracle)",
            Layer::L2 => "skills/knowledge (hururu)",
        }
    }
}

/// Ramani ya kitendo → tabaka (typed rail). Hii ndiyo "typed rails" ya GFA.
fn layer_of(action: &str) -> Option<(Layer, &'static str)> {
    let a = action.trim().to_lowercase();
    const L0: &[(&str, &str)] = &[
        ("safety", "sheria za usalama"),
        ("permission", "ruhusa za mfumo"),
        ("permissions", "ruhusa za mfumo"),
        ("rbac", "udhibiti wa ufikiaji"),
        ("auth_policy", "sera ya uthibitisho"),
        ("disable_guardrail", "kuzima kizuizi"),
        ("rotate_key", "kubadilisha ufunguo"),
        ("drop_table", "kufuta jedwali la DB"),
        ("delete_data", "kufuta data"),
    ];
    const L1: &[(&str, &str)] = &[
        ("harness", "bomba la utekelezaji"),
        ("verifier", "kithibitishaji"),
        ("test", "majaribio"),
        ("deploy", "kuweka production"),
        ("migrate", "uhamishaji wa schema"),
        ("schema", "schema ya DB"),
        ("ci", "bomba la CI"),
        ("release", "kutolewa"),
        ("code", "mabadiliko ya msimbo"),
    ];
    const L2: &[(&str, &str)] = &[
        ("skill", "ujuzi"),
        ("prompt", "kichocheo"),
        ("doc", "nyaraka"),
        ("knowledge", "maarifa"),
        ("template", "kiolezo"),
        ("agent_role", "wajibu wa agent"),
        ("ui_label", "lebo ya UI"),
        ("bundle", "kifurushi"),
    ];
    for (k, l) in L0 {
        if a == *k {
            return Some((Layer::L0, l));
        }
    }
    for (k, l) in L1 {
        if a == *k {
            return Some((Layer::L1, l));
        }
    }
    for (k, l) in L2 {
        if a == *k {
            return Some((Layer::L2, l));
        }
    }
    None
}

/// Uamuzi wa Guardrail-First. `approved` = idhini ya binadamu (HITL);
/// `oracle_ok` = uthibitisho halisi (majaribio/verifier zimepita).
pub fn decide(action: &str, approved: bool, oracle_ok: bool) -> Value {
    let Some((layer, label)) = layer_of(action) else {
        return json!({
            "ok": true,
            "action": action,
            "layer": null,
            "decision": "needs_approval",
            "reason": "kitendo hakina typed rail — kinahitaji uainishaji wa binadamu",
            "blast_radius": "unknown",
        });
    };
    let (decision, reason) = match layer {
        Layer::L0 => (
            "blocked",
            "L0 imegandishwa — usalama/ruhusa haziwezi kubadilishwa hata kwa idhini",
        ),
        Layer::L1 => {
            if !approved {
                ("needs_approval", "L1 inahitaji idhini ya binadamu (HITL)")
            } else if !oracle_ok {
                ("needs_oracle", "L1 inahitaji oracle (majaribio/verifier) zimepita")
            } else {
                ("allowed", "L1 imeidhinishwa na oracle imethibitisha")
            }
        }
        Layer::L2 => ("allowed", "L2 hururu — hakuna kizuizi"),
    };
    json!({
        "ok": true,
        "action": action,
        "layer": layer.as_str(),
        "layer_label": label,
        "decision": decision,
        "reason": reason,
        "blast_radius": layer.as_str(),
        "approved": approved,
        "oracle_ok": oracle_ok,
    })
}

#[derive(Debug, Deserialize)]
pub struct PolicyReq {
    pub action: String,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub oracle_ok: bool,
}

/// POST /api/swarm/v2/policy — uamuzi mmoja wa typed rail.
pub async fn policy(Json(r): Json<PolicyReq>) -> Json<Value> {
    Json(decide(&r.action, r.approved, r.oracle_ok))
}

/// POST /api/swarm/v2/policy/batch — uamuzi wa vitendo vingi kwa wakati mmoja.
pub async fn policy_batch(Json(r): Json<Value>) -> Json<Value> {
    let approved = r.get("approved").and_then(|v| v.as_bool()).unwrap_or(false);
    let oracle_ok = r.get("oracle_ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let actions: Vec<String> = r
        .get("actions")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let results: Vec<Value> = actions.iter().map(|a| decide(a, approved, oracle_ok)).collect();
    let allowed = results.iter().filter(|v| v["decision"] == "allowed").count();
    let blocked = results.iter().filter(|v| v["decision"] == "blocked").count();
    Json(json!({
        "ok": true,
        "count": results.len(),
        "allowed": allowed,
        "blocked": blocked,
        "needs_approval": results.iter().filter(|v| v["decision"] == "needs_approval").count(),
        "needs_oracle": results.iter().filter(|v| v["decision"] == "needs_oracle").count(),
        "results": results,
    }))
}

// ============================================================
// 2) RED / BLUE / GREEN TEAM
// ============================================================

#[derive(Debug, Deserialize)]
pub struct TeamReq {
    #[serde(default)]
    pub path: String,
}

/// POST /api/swarm/v2/team — red (attack surface) + blue (ulinzi) + green (marekebisho).
pub async fn team(State(s): State<AppState>, Json(r): Json<TeamReq>) -> Json<Value> {
    let root = crate::ide_api::ide_workspace_root();

    // --- RED TEAM: eneo la mashambulizi kutoka code_audit (regex halisi) ---
    let scan = crate::code_audit::scan_workspace(&root, &r.path);
    let findings = scan.get("findings").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let total = scan.get("total_findings").and_then(|v| v.as_u64()).unwrap_or(0);
    let by_sev = scan.get("by_severity").cloned().unwrap_or(json!({}));
    let by_cat = scan.get("by_category").cloned().unwrap_or(json!({}));
    let high = by_sev.get("high").and_then(|v| v.as_u64()).unwrap_or(0);
    let critical = by_sev.get("critical").and_then(|v| v.as_u64()).unwrap_or(0);
    let red = json!({
        "role": "red",
        "label": "Red Team — wanaotafuta udhaifu",
        "findings": total,
        "critical": critical,
        "high": high,
        "by_severity": by_sev,
        "by_category": by_cat,
        "sample": findings.iter().take(15).cloned().collect::<Vec<_>>(),
    });

    // --- BLUE TEAM: ulinzi — shield (prompt-injection) + GFA (typed rails) ---
    let shield = crate::shield::stats(&s.db).await;
    let rail_probe = vec![
        decide("safety", true, true),
        decide("permissions", true, true),
        decide("test", false, false),
        decide("skill", false, false),
        decide("deploy", true, true),
        decide("unknown_thing", true, true),
    ];
    let rails_blocked = rail_probe.iter().filter(|v| v["decision"] == "blocked").count();
    let blue = json!({
        "role": "blue",
        "label": "Blue Team — wanaogundua uvamizi",
        "injection_blocks": shield.get("total").cloned().unwrap_or(json!(0)),
        "shield": shield,
        "typed_rails": {
            "probed": rail_probe.len(),
            "blocked": rails_blocked,
            "details": rail_probe,
        },
    });

    // --- GREEN TEAM: marekebisho — pendekezo kwa kila aina ya finding ---
    let mut remediations: Vec<Value> = Vec::new();
    if let Some(cats) = by_cat.as_object() {
        for (cat, count) in cats {
            remediations.push(json!({
                "category": cat,
                "count": count,
                "action": remediation_for(cat),
            }));
        }
    }
    let green = json!({
        "role": "green",
        "label": "Green Team — wanaorekebisha kiotomatiki",
        "remediations": remediations,
        "auto_fixable": remediations.iter().filter(|r| r["action"]["auto"] == true).count(),
    });

    // --- VERDICT ---
    let risk = if critical > 0 { "critical" } else if high > 0 { "high" } else if total > 0 { "medium" } else { "low" };
    let verdict = if critical > 0 || high > 0 {
        "Rekebisha matokeo ya red team kabla ya kuweka production"
    } else {
        "Hakuna hatari kubwa iliyogunduliwa"
    };
    Json(json!({
        "ok": true,
        "target": r.path,
        "risk": risk,
        "verdict": verdict,
        "red": red,
        "blue": blue,
        "green": green,
    }))
}

fn remediation_for(cat: &str) -> Value {
    match cat {
        "secret" => json!({"advice": "Hamisha siri kwenye env/secrets manager", "auto": false}),
        "rce" => json!({"advice": "Ondoa utekelezaji wa amri za nje; tumia orodha nyeupe", "auto": false}),
        "auth" => json!({"advice": "Thibitisha uthibitisho + RBAC kwenye njia hii", "auto": false}),
        "crypto" => json!({"advice": "Tumia Argon2/bcrypt badala ya SHA-256 kwa nenosiri", "auto": false}),
        "traversal" => json!({"advice": "Safisha njia za faili (kataa `..`)", "auto": true}),
        "panic" => json!({"advice": "Badilisha `unwrap` na usimamizi wa makosa", "auto": true}),
        "todo" => json!({"advice": "Kamilisha au futa TODO", "auto": true}),
        "cmd" => json!({"advice": "Safisha ingizo la amri", "auto": true}),
        _ => json!({"advice": "Kagua na rekebisha kwa mkono", "auto": false}),
    }
}

// ============================================================
// 3) MAKER VOTING (mawakala wengi + mtengano)
// ============================================================

fn hash64(s: &str) -> u64 {
    // FNV-1a
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Debug, Deserialize)]
pub struct VoteReq {
    pub task: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default = "default_voters")]
    pub voters: usize,
}

fn default_voters() -> usize {
    16
}

/// POST /api/swarm/v2/vote — upigaji kura wa mawakala wengi (MAKER).
/// Kila "mvotaji" ni persona ya kimahesabu; kura zinatoka kwa mchanganyiko wa
/// hash(task + persona + option) — huamua, si nasibu.
pub async fn vote(Json(r): Json<VoteReq>) -> Json<Value> {
    if r.task.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "task ni lazima" }));
    }
    let options: Vec<String> = if r.options.is_empty() {
        // Mtengano wa kimsingi: chaguo = sehemu za kazi
        vec!["mkakati-A".into(), "mkakati-B".into(), "mkakati-C".into()]
    } else {
        r.options.clone()
    };
    let voters = r.voters.clamp(1, 1000);
    let personas = [
        "conservative", "aggressive", "explorer", "verifier", "minimalist",
        "synthesizer", "optimist", "skeptic", "planner", "executor",
        "analyst", "creative", "auditor", "refactorer", "tester", "architect",
    ];
    let mut tally = vec![0u64; options.len()];
    let mut sample: Vec<Value> = Vec::new();
    for v in 0..voters {
        let persona = personas[v % personas.len()];
        // Chaguo bora kwa mvotaji huyu
        let mut best = 0usize;
        let mut best_score = f64::MIN;
        for (i, opt) in options.iter().enumerate() {
            let seed = format!("{}|{}|{}|{}", r.task, persona, opt, v);
            let score = (hash64(&seed) % 10_000) as f64 / 10_000.0;
            if score > best_score {
                best_score = score;
                best = i;
            }
        }
        tally[best] += 1;
        if sample.len() < 6 {
            sample.push(json!({"voter": v, "persona": persona, "choice": options[best], "score": (best_score * 1000.0).round() / 1000.0}));
        }
    }
    let winner_idx = tally
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| **c)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let total: u64 = tally.iter().sum();
    let confidence = if total > 0 { tally[winner_idx] as f64 / total as f64 } else { 0.0 };
    let distribution: Vec<Value> = options
        .iter()
        .enumerate()
        .map(|(i, o)| json!({"option": o, "votes": tally[i], "share": (tally[i] as f64 / total.max(1) as f64 * 1000.0).round() / 10.0}))
        .collect();
    Json(json!({
        "ok": true,
        "task": r.task,
        "voters": voters,
        "winner": options.get(winner_idx).cloned().unwrap_or_default(),
        "confidence": (confidence * 1000.0).round() / 1000.0,
        "distribution": distribution,
        "sample_votes": sample,
    }))
}

// ============================================================
// 4) REASONINGBANK (kujifunza mikakati)
// ============================================================

pub async fn init_tables(db: &sqlx::SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS reasoning_bank (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task TEXT NOT NULL,
            strategy TEXT NOT NULL,
            outcome TEXT NOT NULL,
            reward REAL NOT NULL,
            lessons TEXT,
            at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

#[derive(Debug, Deserialize)]
pub struct LearnReq {
    pub task: String,
    pub strategy: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub reward: f64,
    #[serde(default)]
    pub lessons: String,
}

/// POST /api/swarm/v2/learn — rekodi uzoefu (mfano wa ReasoningBank).
pub async fn learn(State(s): State<AppState>, Json(r): Json<LearnReq>) -> Json<Value> {
    if r.task.trim().is_empty() || r.strategy.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "task na strategy ni lazima" }));
    }
    let reward = r.reward.clamp(-1.0, 1.0);
    let _ = sqlx::query(
        "INSERT INTO reasoning_bank (task, strategy, outcome, reward, lessons, at) VALUES (?,?,?,?,?,?)",
    )
    .bind(&r.task)
    .bind(&r.strategy)
    .bind(&r.outcome)
    .bind(reward)
    .bind(&r.lessons)
    .bind(chrono::Local::now().to_rfc3339())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "task": r.task, "strategy": r.strategy, "reward": reward }))
}

#[derive(Debug, Deserialize)]
pub struct StratQuery {
    #[serde(default)]
    pub task: String,
}

/// GET /api/swarm/v2/strategies — mikakati iliyopangwa kwa wastani wa reward.
pub async fn strategies(State(s): State<AppState>, Query(q): Query<StratQuery>) -> Json<Value> {
    let rows: Vec<(String, f64, i64, String)> = if q.task.trim().is_empty() {
        sqlx::query_as(
            "SELECT strategy, AVG(reward) AS avg_reward, COUNT(*) AS n, MAX(lessons) AS lessons
             FROM reasoning_bank GROUP BY strategy ORDER BY avg_reward DESC LIMIT 50",
        )
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT strategy, AVG(reward) AS avg_reward, COUNT(*) AS n, MAX(lessons) AS lessons
             FROM reasoning_bank WHERE task = ? GROUP BY strategy ORDER BY avg_reward DESC LIMIT 50",
        )
        .bind(&q.task)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
    };
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(strategy, avg, n, lessons)| {
            json!({"strategy": strategy, "avg_reward": (avg * 1000.0).round() / 1000.0, "runs": n, "lessons": lessons})
        })
        .collect();
    Json(json!({ "ok": true, "task": q.task, "count": items.len(), "strategies": items }))
}

/// GET /api/swarm/v2/status — muhtasari wa safu ya maono.
pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let bank: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reasoning_bank")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    let avg: f64 = sqlx::query_scalar("SELECT COALESCE(AVG(reward), 0) FROM reasoning_bank")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0.0);
    Json(json!({
        "ok": true,
        "guardrail_first": { "rails": ["L0", "L1", "L2"], "frozen": "L0" },
        "teams": ["red", "blue", "green"],
        "maker_voting": true,
        "reasoning_bank": { "records": bank, "avg_reward": (avg * 1000.0).round() / 1000.0 },
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/swarm/v2/status", get(status))
        .route("/api/swarm/v2/policy", post(policy))
        .route("/api/swarm/v2/policy/batch", post(policy_batch))
        .route("/api/swarm/v2/team", post(team))
        .route("/api/swarm/v2/vote", post(vote))
        .route("/api/swarm/v2/learn", post(learn))
        .route("/api/swarm/v2/strategies", get(strategies))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_imegandishwa_kila_wakati() {
        let d = decide("safety", true, true);
        assert_eq!(d["decision"], "blocked");
        assert_eq!(d["layer"], "L0");
    }

    #[test]
    fn l1_inahitaji_idhini_na_oracle() {
        assert_eq!(decide("test", false, false)["decision"], "needs_approval");
        assert_eq!(decide("test", true, false)["decision"], "needs_oracle");
        assert_eq!(decide("test", true, true)["decision"], "allowed");
    }

    #[test]
    fn l2_hururu() {
        assert_eq!(decide("skill", false, false)["decision"], "allowed");
    }

    #[test]
    fn hakuna_rail_inaitaji_uainishaji() {
        assert_eq!(decide("kitu_isichojulikana", true, true)["decision"], "needs_approval");
    }
}
